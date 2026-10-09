mod memory;
use bevy::{
    asset::{
        AssetEvent, AssetId, AsyncReadExt, AsyncSeekExt, RenderAssetUsages,
        io::{AssetReaderError, AssetSourceId},
    },
    camera::{primitives::Aabb, visibility::VisibleEntities},
    gltf::GltfMaterialExtras,
    image::{CompressedImageFormats, ImageFormat, ImageSampler, ImageType},
    light::{Cascades, DirectionalLightShadowMap, NotShadowCaster, PointLightShadowMap},
    math::Vec3A,
    prelude::*,
    render::{
        Render, RenderApp, RenderSystems, erased_render_asset::prepare_erased_assets,
        extract_resource::ExtractResourcePlugin, render_asset::prepare_assets,
        renderer::RenderAdapterInfo, texture::GpuImage,
    },
    tasks::{IoTaskPool, Task, block_on, poll_once},
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BinaryHeap, HashMap, HashSet},
    io::SeekFrom,
    path::Path,
    sync::Arc,
    time::Duration,
};

const INDEX_PATH: &str = "texture-streaming.json";
const MAX_IN_FLIGHT: usize = 4;
const MAX_PENDING_REQUESTS: usize = 8;
const BUFFERED_READ_BYTES: u64 = 64 * 1024 * 1024;
const UPLOAD_BYTES_PER_FRAME: u64 = 8 * 1024 * 1024;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum DemandClass {
    Foreground,
    Camera,
    Indirect,
    Prefetch,
    Idle,
}
const SERVICE: [DemandClass; 6] = [
    DemandClass::Foreground,
    DemandClass::Camera,
    DemandClass::Foreground,
    DemandClass::Camera,
    DemandClass::Indirect,
    DemandClass::Prefetch,
];
struct Candidate {
    id: AssetId<Image>,
    class: DemandClass,
    since: f64,
    bytes: u64,
    promotion: bool,
    uri: String,
    coverage: f32,
    gpu_bytes: u64,
}
fn fair_next(candidates: &mut Vec<Candidate>, cursor: &mut usize) -> Option<Candidate> {
    for offset in 0..SERVICE.len() {
        let index = (*cursor + offset) % SERVICE.len();
        if let Some(position) = candidates
            .iter()
            .position(|c| c.promotion && c.class == SERVICE[index])
        {
            *cursor = (index + 1) % SERVICE.len();
            return Some(candidates.remove(position));
        }
    }
    (!candidates.is_empty()).then(|| candidates.remove(0))
}
fn order_candidates(candidates: &mut [Candidate]) {
    candidates.sort_unstable_by(|a, b| {
        b.promotion
            .cmp(&a.promotion)
            .then_with(|| a.class.cmp(&b.class))
            .then_with(|| a.since.floor().total_cmp(&b.since.floor()))
            .then_with(|| b.coverage.total_cmp(&a.coverage))
            .then_with(|| a.since.total_cmp(&b.since))
            .then_with(|| a.uri.cmp(&b.uri))
    });
}

#[derive(Deserialize)]
struct Index {
    version: u32,
    textures: Vec<TextureEntry>,
    #[serde(default)]
    meshes: HashMap<String, Vec<SurfaceRegion>>,
    #[serde(default)]
    density_policy: Option<DensityPolicy>,
}

#[derive(Deserialize)]
struct DensityPolicy {
    target: f32,
    pending_textures: Vec<String>,
    results: Vec<DensityResult>,
}

#[derive(Deserialize)]
struct DensityResult {
    exception: bool,
}

#[derive(Deserialize)]
struct SurfaceRegion {
    center: [f32; 3],
    half_extents: [f32; 3],
    textures: HashMap<String, f32>,
}

struct BoundRegion {
    center: Vec3,
    half_extents: Vec3,
    textures: Vec<(AssetId<Image>, f32)>,
}

#[derive(Clone, Deserialize)]
struct TextureEntry {
    uri: String,
    full_uri: String,
    width: u32,
    height: u32,
    bootstrap_mip: usize,
    header_bytes: usize,
    bytes: u64,
    color_space: String,
    texels_per_meter: f32,
    mips: Vec<Mip>,
}

#[derive(Clone, Deserialize)]
struct Mip {
    level: usize,
    width: u32,
    height: u32,
    offset: u64,
    bytes: u64,
}

impl TextureEntry {
    fn validate(&self) -> Result<(), String> {
        for uri in [&self.uri, &self.full_uri] {
            let path = Path::new(uri);
            if path.is_absolute()
                || uri.contains('\\')
                || uri.contains(':')
                || path
                    .components()
                    .any(|c| matches!(c, std::path::Component::ParentDir))
                || path.extension().is_none_or(|extension| extension != "dds")
            {
                return Err(format!("Invalid streaming dependency: {uri}"));
            }
        }
        if self.width == 0
            || self.height == 0
            || !matches!(self.header_bytes, 128 | 148)
            || self.mips.is_empty()
            || self.bootstrap_mip >= self.mips.len()
            || self.width > 16384
            || self.height > 16384
            || self.mips.len() != (32 - self.width.max(self.height).leading_zeros()) as usize
            || !self.texels_per_meter.is_finite()
            || self.texels_per_meter < 0.
            || !matches!(self.color_space.as_str(), "linear" | "srgb")
        {
            return Err(format!("Invalid streaming descriptor: {}", self.uri));
        }
        let mut cursor = self.header_bytes as u64;
        for (level, mip) in self.mips.iter().enumerate() {
            if mip.level != level
                || mip.width != (self.width >> level).max(1)
                || mip.height != (self.height >> level).max(1)
                || mip.offset != cursor
                || mip.bytes == 0
            {
                return Err(format!("Invalid streaming mip chain: {}", self.uri));
            }
            cursor = cursor
                .checked_add(mip.bytes)
                .ok_or("Streaming range overflow")?;
        }
        if cursor != self.bytes {
            return Err(format!("Streaming byte range does not cover {}", self.uri));
        }
        Ok(())
    }

    fn resident_bytes(&self, mip: usize) -> u64 {
        self.bytes - self.mips[mip].offset
    }

    fn gpu_bytes(&self, mip: usize) -> u64 {
        self.resident_bytes(mip).div_ceil(65536) * 65536 + 65536
    }
}

struct Request {
    mip: usize,
    task: Task<Result<Image, String>>,
    ready: Option<Image>,
}

struct Resident {
    _handle: Handle<Image>,
    entry: TextureEntry,
    mip: usize,
    wanted: usize,
    last_demand: f64,
    last_change: f64,
    sampler: ImageSampler,
    request: Option<Request>,
    failed: bool,
    foreground: bool,
    class: DemandClass,
    queued_since: Option<f64>,
    screen_coverage: f32,
    requested: usize,
    stabilized: usize,
    coarser_since: Option<f64>,
}

impl Resident {
    fn requires_base_mip(&self) -> bool {
        matches!(&self.sampler, ImageSampler::Descriptor(sampler) if sampler.lod_max_clamp == 0.)
    }
}

#[derive(Resource)]
pub(crate) struct TextureStreams {
    index_task: Option<Task<Result<Option<Index>, String>>>,
    entries: HashMap<String, TextureEntry>,
    residents: HashMap<AssetId<Image>, Resident>,
    meshes: HashMap<String, Arc<Vec<SurfaceRegion>>>,
    bound_regions: HashMap<AssetId<Mesh>, Arc<Vec<BoundRegion>>>,
    timer: Timer,
    previous_camera: Option<(Vec3, f64)>,
    enabled: bool,
    evaluated_demand: bool,
    failure: Option<String>,
    read_cursor: usize,
    admission_reservation: Option<(AssetId<Image>, usize)>,
    upload_cursor: usize,
    upload_reservation: Option<(AssetId<Image>, usize)>,
    budget: u64,
    bootstrap_reserve: u64,
    last_image_count: usize,
}

impl Default for TextureStreams {
    fn default() -> Self {
        Self {
            index_task: None,
            entries: HashMap::new(),
            residents: HashMap::new(),
            meshes: HashMap::new(),
            bound_regions: HashMap::new(),
            timer: Timer::new(Duration::from_millis(100), TimerMode::Repeating),
            previous_camera: None,
            enabled: false,
            evaluated_demand: false,
            failure: None,
            read_cursor: 0,
            admission_reservation: None,
            upload_cursor: 0,
            upload_reservation: None,
            budget: 0,
            bootstrap_reserve: 0,
            last_image_count: 0,
        }
    }
}

impl TextureStreams {
    fn refresh_bootstrap_reservation(&self, stats: &mut TextureStreamingStats) -> u64 {
        let registered: u64 = self
            .residents
            .values()
            .map(|resident| resident.entry.gpu_bytes(resident.entry.bootstrap_mip))
            .sum();
        let reserved = self.bootstrap_reserve.saturating_sub(registered);
        stats.indexed_bootstrap_gpu_bytes = self.bootstrap_reserve;
        stats.registered_bootstrap_gpu_bytes = registered;
        stats.bootstrap_reserved_bytes = reserved;
        reserved
    }

    pub(crate) fn residency(&self, id: AssetId<Image>) -> Option<serde_json::Value> {
        let resident = self.residents.get(&id)?;
        Some(serde_json::json!({
            "id":format!("{id:?}"), "uri":resident.entry.uri,
            "full_extent":[resident.entry.width,resident.entry.height],
            "bootstrap_mip":resident.entry.bootstrap_mip,
            "resident_mip":resident.mip,"wanted_mip":resident.wanted,
            "pending_mip":resident.request.as_ref().map(|r|r.mip),
            "foreground":resident.foreground,
            "screen_coverage":resident.screen_coverage,
            "requested_mip":resident.requested,
            "stabilized_mip":resident.stabilized,
            "budget_limited":resident.wanted > resident.requested,
        }))
    }
    fn fail(&mut self, error: String) {
        self.failure = Some(error);
        self.enabled = false;
        for resident in self.residents.values_mut() {
            resident.request = None;
        }
    }

    pub(crate) fn failure(&self) -> Option<&str> {
        self.failure.as_deref()
    }

    pub(crate) fn ready(&self) -> bool {
        self.index_task.is_none()
            && self.failure.is_none()
            && (!self.enabled
                || (self.evaluated_demand && self.residents.values().all(|r| r.mip <= r.wanted)))
    }
}

#[derive(Resource, Default, Serialize)]
pub(crate) struct TextureStreamingStats {
    pub managed_textures: usize,
    pub resident_mip_bytes: u64,
    pub resident_gpu_bytes: u64,
    pub accepted_gpu_bytes: u64,
    pub wanted_mip_bytes: u64,
    pub queued_reads: usize,
    pub uploaded_bytes: u64,
    pub promotions: u64,
    pub demotions: u64,
    pub pending_detail: usize,
    pub pending_mip_changes: usize,
    pub errors: u32,
    pub shadow_requests: usize,
    pub anisotropic_textures: usize,
    pub resident_base_textures: usize,
    pub material_refreshes: u64,
    pub standard_refreshes: u64,
    pub opaque_refreshes: u64,
    pub diagnostic_refreshes: u64,
    pub reads_by_class: [u64; 4],
    pub uploads_by_class: [u64; 4],
    pub max_read_wait_seconds: [f64; 4],
    pub buffered_read_bytes: u64,
    pub peak_buffered_read_bytes: u64,
    pub active_workers: usize,
    pub peak_active_workers: usize,
    pub cancelled_reads: u64,
    pub pending_by_class: [usize; 4],
    pub admission_drains: u64,
    pub upload_deferrals: u64,
    pub density_bound_meshes: usize,
    pub evaluated_surface_regions: usize,
    pub density_target: f32,
    pub density_capped_textures: usize,
    pub density_pending_textures: usize,
    pub adapter: String,
    pub capacity_source: String,
    pub detected_vram_bytes: u64,
    pub residency_budget_bytes: u64,
    pub indexed_bootstrap_gpu_bytes: u64,
    pub registered_bootstrap_gpu_bytes: u64,
    pub bootstrap_reserved_bytes: u64,
    pub retiring_gpu_bytes: u64,
    pub replacement_reserved_bytes: u64,
    pub accounted_gpu_bytes: u64,
    pub peak_accounted_gpu_bytes: u64,
    pub naturally_requested_bytes: u64,
    pub stabilized_requested_bytes: u64,
    pub budget_limited_textures: usize,
    pub screen_coverage_sum: f32,
    pub gpu_budget_drains: u64,
}
impl TextureStreamingStats {
    pub(crate) fn settled(&self) -> bool {
        self.pending_mip_changes == 0
            && self.queued_reads == 0
            && self.active_workers == 0
            && self.buffered_read_bytes == 0
            && self.replacement_reserved_bytes == 0
            && self.retiring_gpu_bytes == 0
    }
}

#[derive(Component)]
pub(crate) struct TextureUse {
    pub(crate) textures: Vec<AssetId<Image>>,
    coverage: Vec<AssetId<Image>>,
    regions: Arc<Vec<BoundRegion>>,
}

#[derive(Component)]
pub(crate) struct ForegroundTexture;

pub(super) struct TextureStreamingPlugin;
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) struct TextureStreamingSet;
impl Plugin for TextureStreamingPlugin {
    fn build(&self, app: &mut App) {
        let memory = memory::MemoryLedger::default();
        app.insert_resource(memory.clone());
        app.init_resource::<TextureStreams>()
            .init_resource::<memory::Replacements>()
            .add_plugins(ExtractResourcePlugin::<memory::Replacements>::default())
            .init_resource::<TextureStreamingStats>()
            .add_systems(Startup, start)
            .add_systems(
                PostUpdate,
                (receive_index, register_images, upload, report_failure)
                    .chain()
                    .in_set(TextureStreamingSet)
                    .after(bevy::transform::TransformSystems::Propagate)
                    .before(bevy::asset::AssetEventSystems),
            )
            .add_systems(
                PostUpdate,
                (bind_uses, demand, shadow_demand, allocate_detail, schedule)
                    .chain()
                    .after(TextureStreamingSet)
                    .after(bevy::camera::visibility::VisibilitySystems::CheckVisibility)
                    .after(bevy::light::SimulationLightSystems::UpdateLightFrusta),
            );
        if let Some(render) = app.get_sub_app_mut(RenderApp) {
            render
                .insert_resource(memory)
                .init_resource::<memory::RetiringTextures>()
                .init_resource::<memory::MaterialImages>()
                .init_resource::<memory::MatchedReplacements>()
                .add_systems(
                    Render,
                    (
                        memory::invalidate_material_bindings,
                        memory::capture_replaced,
                    )
                        .chain()
                        .in_set(RenderSystems::PrepareAssets)
                        .before(prepare_assets::<GpuImage>)
                        .before(prepare_erased_assets::<MeshMaterial3d<StandardMaterial>>)
                        .before(
                            prepare_erased_assets::<MeshMaterial3d<super::opaque::OpaqueMaterial>>,
                        )
                        .before(
                            prepare_erased_assets::<MeshMaterial3d<super::pbr::DiagnosticMaterial>>,
                        ),
                )
                .add_systems(
                    Render,
                    memory::release_completed.in_set(RenderSystems::Cleanup),
                );
            render.add_systems(
                Render,
                uploaded_images_ready
                    .in_set(RenderSystems::PrepareAssets)
                    .after(prepare_assets::<GpuImage>)
                    .before(prepare_erased_assets::<MeshMaterial3d<StandardMaterial>>)
                    .before(prepare_erased_assets::<MeshMaterial3d<super::opaque::OpaqueMaterial>>)
                    .before(
                        prepare_erased_assets::<MeshMaterial3d<super::pbr::DiagnosticMaterial>>,
                    ),
            );
        }
    }
}

fn uploaded_images_ready() {}

fn report_failure(
    streams: Res<TextureStreams>,
    state: Res<State<crate::GameState>>,
    mut failure: ResMut<crate::assets::LoadFailure>,
    mut next: ResMut<NextState<crate::GameState>>,
) {
    if **state != crate::GameState::LoadError {
        if let Some(error) = streams.failure() {
            failure.0 = format!("Texture streaming failed: {error}");
            next.set(crate::GameState::LoadError);
        }
    }
}

fn start(
    server: Res<AssetServer>,
    adapter: Res<RenderAdapterInfo>,
    mut streams: ResMut<TextureStreams>,
    mut stats: ResMut<TextureStreamingStats>,
) {
    let capacity = match memory::detect(&adapter) {
        Ok(capacity) => capacity,
        Err(error) => {
            stats.errors += 1;
            streams.fail(error.to_string());
            return;
        }
    };
    stats.adapter = adapter.name.clone();
    stats.capacity_source = capacity.source.into();
    stats.detected_vram_bytes = capacity.bytes;
    streams.budget = capacity.bytes / 4 * 3;
    stats.residency_budget_bytes = streams.budget;
    info!(
        "Texture VRAM: {} dedicated bytes; {} byte hard cap on {}",
        capacity.bytes, streams.budget, adapter.name
    );
    let server = server.clone();
    streams.index_task = Some(IoTaskPool::get().spawn(async move {
        let source = server
            .get_source(AssetSourceId::Default)
            .map_err(|e| e.to_string())?;
        let mut reader = match source.reader().read(Path::new(INDEX_PATH)).await {
            Ok(reader) => reader,
            Err(AssetReaderError::NotFound(_)) => return Ok(None),
            Err(error) => return Err(error.to_string()),
        };
        let mut bytes = Vec::new();
        reader
            .read_to_end(&mut bytes)
            .await
            .map_err(|e| e.to_string())?;
        let index: Index = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
        if !matches!(index.version, 1 | 2) {
            return Err("Unsupported texture streaming index".into());
        }
        for entry in &index.textures {
            entry.validate()?;
        }
        let texture_uris: HashSet<_> = index
            .textures
            .iter()
            .map(|entry| entry.uri.as_str())
            .collect();
        for (mesh, regions) in &index.meshes {
            if !mesh.contains("#Mesh") || !mesh.contains("/Primitive") {
                return Err(format!("Invalid surface density mesh: {mesh}"));
            }
            for region in regions {
                if region.center.iter().any(|v| !v.is_finite())
                    || region
                        .half_extents
                        .iter()
                        .any(|v| !v.is_finite() || *v < 0.)
                    || region.textures.iter().any(|(uri, density)| {
                        !density.is_finite()
                            || *density < 0.
                            || !texture_uris.contains(uri.as_str())
                    })
                {
                    return Err(format!("Invalid surface density region: {mesh}"));
                }
            }
        }
        Ok(Some(index))
    }));
}

fn receive_index(mut streams: ResMut<TextureStreams>, mut stats: ResMut<TextureStreamingStats>) {
    let Some(task) = streams.index_task.as_mut() else {
        return;
    };
    let Some(result) = block_on(poll_once(task)) else {
        return;
    };
    streams.index_task = None;
    match result {
        Ok(Some(index)) => {
            if let Some(policy) = &index.density_policy {
                stats.density_target = policy.target;
                stats.density_capped_textures = policy
                    .results
                    .iter()
                    .filter(|result| result.exception)
                    .count();
                stats.density_pending_textures = policy.pending_textures.len();
                info!(
                    "Prepared density target: {} texels/m, {} capped exceptions, {} pending maps",
                    stats.density_target,
                    stats.density_capped_textures,
                    stats.density_pending_textures
                );
                if stats.density_pending_textures > 0 {
                    warn!(
                        "This is a partial texture qualification pack; the density migration is incomplete"
                    );
                }
            }
            streams.meshes = index
                .meshes
                .into_iter()
                .map(|(key, regions)| (key, Arc::new(regions)))
                .collect();
            for entry in index.textures {
                if streams.entries.insert(entry.uri.clone(), entry).is_some() {
                    error!("Duplicate streamed texture URI");
                    stats.errors += 1;
                    streams.entries.clear();
                    streams.fail("Duplicate streamed texture URI".into());
                    return;
                }
            }
            streams.enabled = !streams.entries.is_empty();
            streams.bootstrap_reserve = streams
                .entries
                .values()
                .map(|entry| entry.gpu_bytes(entry.bootstrap_mip))
                .sum();
            let minimum_replacement = streams
                .entries
                .values()
                .map(|entry| entry.gpu_bytes(entry.mips.len() - 1))
                .max()
                .unwrap_or(0);
            if streams.bootstrap_reserve + minimum_replacement > streams.budget {
                stats.errors += 1;
                streams.fail(
                    "Required native texture bootstraps and minimum replacement exceed the detected VRAM budget".into(),
                );
                return;
            }
            info!(
                "Texture streaming: {} indexed full mip chains",
                streams.entries.len()
            );
        }
        Ok(None) => {}
        Err(error) => {
            error!("Texture streaming index: {error}");
            stats.errors += 1;
            streams.fail(error);
        }
    }
}

fn register_images(
    server: Res<AssetServer>,
    mut events: MessageReader<AssetEvent<Image>>,
    mut streams: ResMut<TextureStreams>,
    mut images: ResMut<Assets<Image>>,
    mut stats: ResMut<TextureStreamingStats>,
) {
    let changed = events
        .read()
        .filter_map(|event| match event {
            AssetEvent::Added { id } | AssetEvent::Modified { id } => Some(*id),
            _ => None,
        })
        .collect::<Vec<_>>();
    if !streams.enabled {
        return;
    }
    let new_images = streams.residents.is_empty() || streams.last_image_count != images.len();
    streams.last_image_count = images.len();
    let ids = if new_images {
        images.ids().collect::<Vec<_>>()
    } else {
        changed
    };
    for id in ids {
        if streams.residents.contains_key(&id) {
            continue;
        }
        let Some(path) = server.get_path(id) else {
            continue;
        };
        let Some(entry) = streams
            .entries
            .get(&path.path().to_string_lossy().into_owned())
            .cloned()
        else {
            continue;
        };
        let Some(mut image) = images.get_mut(id) else {
            continue;
        };
        let bootstrap = &entry.mips[entry.bootstrap_mip];
        let descriptor = &image.texture_descriptor;
        if descriptor.size.width != bootstrap.width
            || descriptor.size.height != bootstrap.height
            || descriptor.size.depth_or_array_layers != 1
            || descriptor.mip_level_count as usize != entry.mips.len() - entry.bootstrap_mip
            || descriptor.format.is_srgb() != (entry.color_space == "srgb")
        {
            stats.errors += 1;
            streams.fail(format!(
                "Bootstrap image differs from streaming index: {}",
                entry.uri
            ));
            return;
        }
        let Some(handle) = server.get_id_handle(id) else {
            continue;
        };
        image.asset_usage = RenderAssetUsages::RENDER_WORLD;
        image.texture_descriptor.dimension = bevy::render::render_resource::TextureDimension::D2;
        let bootstrap_mip = entry.bootstrap_mip;
        streams.residents.insert(
            id,
            Resident {
                _handle: handle,
                mip: entry.bootstrap_mip,
                wanted: entry.bootstrap_mip,
                entry,
                last_demand: 0.,
                last_change: 0.,
                sampler: image.sampler.clone(),
                request: None,
                failed: false,
                foreground: false,
                class: DemandClass::Idle,
                queued_since: None,
                screen_coverage: 0.,
                requested: bootstrap_mip,
                stabilized: bootstrap_mip,
                coarser_since: None,
            },
        );
    }
    streams.refresh_bootstrap_reservation(&mut stats);
}

fn bind_uses(
    mut commands: Commands,
    mut streams: ResMut<TextureStreams>,
    server: Res<AssetServer>,
    mut stats: ResMut<TextureStreamingStats>,
    unbound: Query<(Entity, &GltfMaterialExtras, &Mesh3d), Without<TextureUse>>,
) {
    if !streams.enabled {
        return;
    }
    for (entity, extras, mesh) in &unbound {
        let Ok(value) = serde_json::from_str::<serde_json::Value>(&extras.value) else {
            continue;
        };
        let Some(channels) = value["surface"]["channels"].as_array() else {
            continue;
        };
        let mut ids = Vec::new();
        let mut coverage = Vec::new();
        let masked = value["surface"]["presentation"]["alpha"].as_str() == Some("mask");
        let mut pending = false;
        for channel in channels {
            let Some(uri) = channel["uri"].as_str() else {
                continue;
            };
            if !streams.entries.contains_key(uri) {
                continue;
            }
            if let Some(handle) = server.get_handle::<Image>(uri.to_owned()) {
                ids.push(handle.id());
                if masked && matches!(channel["role"].as_str(), Some("color" | "control")) {
                    coverage.push(handle.id());
                }
            } else {
                pending = true;
            }
        }
        if !pending {
            ids.sort_unstable();
            ids.dedup();
            coverage.sort_unstable();
            coverage.dedup();
            let regions = if let Some(regions) = streams.bound_regions.get(&mesh.0.id()) {
                regions.clone()
            } else {
                let path = server.get_path(mesh.0.id()).map(|path| path.to_string());
                let regions = path.as_ref().and_then(|path| streams.meshes.get(path));
                let bound = Arc::new(regions.map_or_else(Vec::new, |regions| {
                    regions
                        .iter()
                        .map(|region| BoundRegion {
                            center: Vec3::from_array(region.center),
                            half_extents: Vec3::from_array(region.half_extents),
                            textures: region
                                .textures
                                .iter()
                                .filter_map(|(uri, density)| {
                                    server
                                        .get_handle::<Image>(uri.clone())
                                        .map(|handle| (handle.id(), *density))
                                })
                                .collect(),
                        })
                        .collect()
                }));
                streams.bound_regions.insert(mesh.0.id(), bound.clone());
                bound
            };
            commands.entity(entity).insert(TextureUse {
                textures: ids,
                coverage,
                regions,
            });
        }
    }
    stats.density_bound_meshes = streams
        .bound_regions
        .values()
        .filter(|regions| !regions.is_empty())
        .count();
}

fn demand(
    time: Res<Time<Real>>,
    mut streams: ResMut<TextureStreams>,
    state: Res<State<crate::GameState>>,
    definition: Option<Res<crate::assets::GameDefinition>>,
    mut stats: ResMut<TextureStreamingStats>,
    cameras: Query<
        (&GlobalTransform, &Projection, &Camera, &VisibleEntities),
        With<super::GameCamera>,
    >,
    overlays: Query<
        (&GlobalTransform, &Projection, &Camera),
        With<crate::presentation::ViewModelCamera>,
    >,
    geometry: Query<(
        Entity,
        &TextureUse,
        &GlobalTransform,
        Option<&Aabb>,
        Option<&InheritedVisibility>,
        Has<ForegroundTexture>,
        Has<bevy::mesh::skinning::SkinnedMesh>,
        Has<super::ActorSurface>,
    )>,
) {
    if !streams.enabled || !streams.timer.tick(time.delta()).just_finished() {
        return;
    }
    let _span = info_span!("texture_streaming_demand").entered();
    let Ok((camera_pose, projection, camera, visible_entities)) = cameras.single() else {
        return;
    };
    let Projection::Perspective(projection) = projection else {
        return;
    };
    let Some(viewport) = camera.physical_viewport_size() else {
        return;
    };
    let position = if **state == crate::GameState::Loading {
        let Some(definition) = definition else { return };
        Vec3::from_array(definition.spawn) + Vec3::Y * definition.tuning.eye_height
    } else {
        camera_pose.translation()
    };
    let now = time.elapsed_secs_f64();
    let velocity = streams.previous_camera.map_or(Vec3::ZERO, |(last, stamp)| {
        ((position - last) / (now - stamp).max(0.001) as f32).clamp_length_max(30.)
    });
    streams.previous_camera = Some((position, now));
    let predicted = position + velocity * 0.75;
    let focal = viewport.y as f32 / (2. * (projection.fov * 0.5).tan());
    let overlay = overlays
        .single()
        .ok()
        .and_then(|(pose, projection, camera)| {
            if let Projection::Perspective(projection) = projection {
                Some((pose, projection, camera.physical_viewport_size()?))
            } else {
                None
            }
        });
    let camera_meshes: HashSet<_> = visible_entities
        .get(std::any::TypeId::of::<Mesh3d>())
        .iter()
        .copied()
        .collect();
    for resident in streams.residents.values_mut() {
        resident.requested = resident.entry.mips.len() - 1;
        resident.foreground = false;
        resident.class = DemandClass::Idle;
        resident.screen_coverage = 0.;
    }
    let mut bound_geometry = false;
    let mut evaluated_regions = 0;
    for (entity, usage, pose, bounds, inherited, foreground, skinned, actor) in &geometry {
        bound_geometry |= !usage.textures.is_empty();
        if **state != crate::GameState::Loading
            && inherited.is_some_and(|visibility| !visibility.get())
        {
            continue;
        }
        let class = if foreground {
            DemandClass::Foreground
        } else if camera_meshes.contains(&entity) || **state == crate::GameState::Loading {
            DemandClass::Camera
        } else {
            DemandClass::Prefetch
        };
        if foreground {
            let center = bounds.map_or(Vec3::ZERO, |aabb| aabb.center.into());
            let half = bounds.map_or(Vec3::ONE, |aabb| aabb.half_extents.into());
            let (view_pose, view_projection, _) =
                overlay.unwrap_or((camera_pose, projection, viewport));
            let coverage = screen_fraction(pose, center, half, view_pose, view_projection);
            evaluated_regions += usage.regions.len();
            for id in &usage.textures {
                if let Some(resident) = streams.residents.get_mut(id) {
                    resident.requested = 0;
                    resident.foreground = true;
                    resident.class = DemandClass::Foreground;
                    resident.last_demand = now;
                    resident.screen_coverage = (resident.screen_coverage + coverage).min(1.);
                }
            }
            continue;
        }
        if actor {
            let center = bounds.map_or(Vec3::ZERO, |aabb| aabb.center.into());
            let half = bounds.map_or(Vec3::ONE, |aabb| aabb.half_extents.into());
            let distance = surface_distance(pose, center, half, position)
                .min(surface_distance(pose, center, half, predicted))
                .max(0.05);
            if class == DemandClass::Prefetch && distance > 15. {
                continue;
            }
            let visible = class == DemandClass::Camera;
            let close = visible && distance <= 6.;
            let coverage = if visible {
                screen_fraction(pose, center, half, camera_pose, projection)
            } else {
                0.
            };
            let diameter = (2. * half.max_element() * transform_scale(pose)).max(0.1);
            for id in &usage.textures {
                let Some(resident) = streams.residents.get_mut(id) else {
                    continue;
                };
                let extent = resident.entry.width.max(resident.entry.height) as f32;
                let mip = if close || resident.requires_base_mip() {
                    0
                } else {
                    (extent * distance / (focal * diameter * 2.))
                        .log2()
                        .floor()
                        .max(0.) as usize
                }
                .min(resident.entry.mips.len() - 1);
                resident.requested = resident.requested.min(mip);
                resident.foreground |= close;
                resident.class = resident.class.min(if close {
                    DemandClass::Foreground
                } else {
                    class
                });
                resident.last_demand = now;
                resident.screen_coverage = (resident.screen_coverage + coverage).min(1.);
            }
            continue;
        }
        if skinned && !usage.regions.is_empty() {
            let center = bounds.map_or(Vec3::ZERO, |aabb| aabb.center.into());
            let half = bounds.map_or(Vec3::ONE, |aabb| aabb.half_extents.into());
            let distance = surface_distance(pose, center, half, position)
                .min(surface_distance(pose, center, half, predicted))
                .max(0.05);
            if class == DemandClass::Prefetch && distance > 15. {
                continue;
            }
            let coverage = if class == DemandClass::Camera {
                screen_fraction(pose, center, half, camera_pose, projection)
            } else {
                0.
            };
            let scale = transform_scale(pose);
            let mut densities: HashMap<AssetId<Image>, f32> = HashMap::new();
            for region in usage.regions.iter() {
                for (id, density) in &region.textures {
                    densities
                        .entry(*id)
                        .and_modify(|previous| *previous = previous.min(*density))
                        .or_insert(*density);
                }
            }
            evaluated_regions += usage.regions.len();
            for (id, density) in densities {
                let Some(resident) = streams.residents.get_mut(&id) else {
                    continue;
                };
                let density = density / scale;
                let mip = if resident.requires_base_mip() || density <= 0. {
                    0
                } else {
                    (density * distance / (focal * 2.)).log2().floor().max(0.) as usize
                }
                .min(resident.entry.mips.len() - 1);
                resident.requested = resident.requested.min(mip);
                resident.class = resident.class.min(class);
                resident.last_demand = now;
                resident.screen_coverage = (resident.screen_coverage + coverage).min(1.);
            }
            continue;
        }
        if !usage.regions.is_empty() {
            let scale = transform_scale(pose);
            for region in usage.regions.iter() {
                let distance = surface_distance(pose, region.center, region.half_extents, position)
                    .min(surface_distance(
                        pose,
                        region.center,
                        region.half_extents,
                        predicted,
                    ))
                    .max(0.05);
                let (view_pose, view_projection, _) = if foreground {
                    overlay.unwrap_or((camera_pose, projection, viewport))
                } else {
                    (camera_pose, projection, viewport)
                };
                let coverage = if foreground || class == DemandClass::Camera {
                    screen_fraction(
                        pose,
                        region.center,
                        region.half_extents,
                        view_pose,
                        view_projection,
                    )
                } else {
                    0.
                };
                if !foreground && !camera_meshes.contains(&entity) && distance > 15. {
                    continue;
                }
                evaluated_regions += 1;
                for (id, local_density) in &region.textures {
                    let Some(resident) = streams.residents.get_mut(id) else {
                        continue;
                    };
                    let density = local_density / scale;
                    let mip = if foreground || resident.requires_base_mip() || density <= 0. {
                        0
                    } else {
                        (density * distance / (focal * 2.)).log2().floor().max(0.) as usize
                    }
                    .min(resident.entry.mips.len() - 1);
                    resident.requested = resident.requested.min(mip);
                    resident.foreground |= foreground;
                    resident.class = resident.class.min(class);
                    resident.last_demand = now;
                    resident.screen_coverage = (resident.screen_coverage + coverage).min(1.);
                }
            }
            continue;
        }
        let center = bounds.map_or(Vec3::ZERO, |aabb| aabb.center.into());
        let half_extents = bounds.map_or(Vec3::ONE, |aabb| aabb.half_extents.into());
        let distance = surface_distance(pose, center, half_extents, position)
            .min(surface_distance(pose, center, half_extents, predicted))
            .max(0.05);
        let (view_pose, view_projection, _) = if foreground {
            overlay.unwrap_or((camera_pose, projection, viewport))
        } else {
            (camera_pose, projection, viewport)
        };
        let coverage = if foreground || class == DemandClass::Camera {
            screen_fraction(pose, center, half_extents, view_pose, view_projection)
        } else {
            0.
        };
        if !foreground && !camera_meshes.contains(&entity) && distance > 15. {
            continue;
        }
        let pixels_per_meter = focal / distance * 2.;
        for id in &usage.textures {
            let Some(resident) = streams.residents.get_mut(id) else {
                continue;
            };
            let density = resident.entry.texels_per_meter;
            let no_mips = resident.requires_base_mip();
            let mip = if foreground || no_mips || density <= 0. || distance < 1. {
                0
            } else {
                (density / pixels_per_meter).log2().floor().max(0.) as usize
            }
            .min(resident.entry.mips.len() - 1);
            resident.requested = resident.requested.min(mip);
            resident.foreground |= foreground;
            let class = if foreground {
                DemandClass::Foreground
            } else if camera_meshes.contains(&entity) || **state == crate::GameState::Loading {
                DemandClass::Camera
            } else {
                DemandClass::Prefetch
            };
            resident.class = resident.class.min(class);
            resident.last_demand = now;
            resident.screen_coverage = (resident.screen_coverage + coverage).min(1.);
        }
    }
    streams.evaluated_demand = bound_geometry || streams.entries.is_empty();
    stats.evaluated_surface_regions = evaluated_regions;
}

fn transform_scale(pose: &GlobalTransform) -> f32 {
    let matrix = pose.affine().matrix3.abs();
    let rows = matrix * Vec3A::ONE;
    let columns = Vec3A::new(
        matrix.x_axis.element_sum(),
        matrix.y_axis.element_sum(),
        matrix.z_axis.element_sum(),
    );
    (rows.max_element() * columns.max_element())
        .sqrt()
        .max(0.00001)
}

fn surface_distance(pose: &GlobalTransform, center: Vec3, half: Vec3, camera: Vec3) -> f32 {
    let inverse = pose.affine().inverse();
    let local = Vec3::from(inverse.transform_point3a(camera.into()));
    let outside = ((local - center).abs() - half).max(Vec3::ZERO).length();
    let matrix = inverse.matrix3.abs();
    let rows = matrix * Vec3A::ONE;
    let columns = Vec3A::new(
        matrix.x_axis.element_sum(),
        matrix.y_axis.element_sum(),
        matrix.z_axis.element_sum(),
    );
    outside
        / (rows.max_element() * columns.max_element())
            .sqrt()
            .max(0.00001)
}

fn screen_fraction(
    pose: &GlobalTransform,
    center: Vec3,
    half: Vec3,
    camera: &GlobalTransform,
    projection: &PerspectiveProjection,
) -> f32 {
    let inverse = camera.affine().inverse();
    let tangent = (projection.fov * 0.5).tan();
    let mut low = Vec2::ONE;
    let mut high = -Vec2::ONE;
    let mut points = Vec::with_capacity(8);
    for x in [-1., 1.] {
        for y in [-1., 1.] {
            for z in [-1., 1.] {
                let world = pose.transform_point(center + half * Vec3::new(x, y, z));
                let point = inverse.transform_point3a(world.into());
                points.push(Vec3::from(point));
            }
        }
    }
    let mut clipped: Vec<_> = points
        .iter()
        .copied()
        .filter(|point| point.z <= -projection.near)
        .collect();
    for index in 0..8 {
        for axis in [1, 2, 4] {
            let other = index ^ axis;
            if other <= index {
                continue;
            }
            let (a, b) = (points[index], points[other]);
            if (a.z < -projection.near) != (b.z < -projection.near) {
                clipped.push(a.lerp(b, (-projection.near - a.z) / (b.z - a.z)));
            }
        }
    }
    if clipped.is_empty() {
        return 0.;
    }
    for point in clipped {
        let ndc = Vec2::new(
            point.x / (-point.z * tangent * projection.aspect_ratio),
            point.y / (-point.z * tangent),
        )
        .clamp(-Vec2::ONE, Vec2::ONE);
        low = low.min(ndc);
        high = high.max(ndc);
    }
    ((high - low).max(Vec2::ZERO).element_product() * 0.25).clamp(0., 1.)
}

fn shadow_demand(
    time: Res<Time<Real>>,
    mut streams: ResMut<TextureStreams>,
    mut stats: ResMut<TextureStreamingStats>,
    geometry: Query<
        (
            &TextureUse,
            &GlobalTransform,
            Option<&Aabb>,
            Option<&InheritedVisibility>,
            Has<bevy::mesh::skinning::SkinnedMesh>,
        ),
        Without<NotShadowCaster>,
    >,
    directional: Query<(&DirectionalLight, &Cascades)>,
    points: Query<(&PointLight, &GlobalTransform, Option<&ViewVisibility>)>,
    spots: Query<(&SpotLight, &GlobalTransform, Option<&ViewVisibility>)>,
    point_map: Res<PointLightShadowMap>,
    directional_map: Res<DirectionalLightShadowMap>,
) {
    if !streams.enabled {
        return;
    }
    let _span = info_span!("texture_streaming_shadows").entered();
    let now = time.elapsed_secs_f64();
    let mut requested = 0;
    for (usage, pose, bounds, inherited, skinned) in &geometry {
        if usage.coverage.is_empty() || inherited.is_some_and(|visibility| !visibility.get()) {
            continue;
        }
        let matrix = pose.affine().matrix3.abs();
        let scale = transform_scale(pose);
        let posed_regions = skinned && !usage.regions.is_empty();
        let fallback = if posed_regions {
            let mut densities: HashMap<AssetId<Image>, f32> = HashMap::new();
            for region in usage.regions.iter() {
                for (id, density) in &region.textures {
                    densities
                        .entry(*id)
                        .and_modify(|previous| *previous = previous.min(*density))
                        .or_insert(*density);
                }
            }
            densities
                .into_iter()
                .map(|(id, density)| (id, density / scale))
                .collect::<Vec<_>>()
        } else if usage.regions.is_empty() {
            usage
                .coverage
                .iter()
                .filter_map(|id| {
                    streams
                        .residents
                        .get(id)
                        .map(|resident| (*id, resident.entry.texels_per_meter))
                })
                .collect::<Vec<_>>()
        } else {
            Vec::new()
        };
        for region in usage
            .regions
            .iter()
            .filter(|_| !posed_regions)
            .map(Some)
            .chain((usage.regions.is_empty() || posed_regions).then_some(None))
        {
            let center = region.map_or_else(
                || {
                    bounds.map_or(pose.translation(), |aabb| {
                        pose.transform_point(aabb.center.into())
                    })
                },
                |region| pose.transform_point(region.center),
            );
            let radius = region.map_or_else(
                || {
                    bounds.map_or(1., |aabb| {
                        (pose.affine().matrix3.abs() * aabb.half_extents).length()
                    })
                },
                |region| (matrix * Vec3A::from(region.half_extents)).length(),
            );
            let mut pixels_per_meter: f32 = 0.;
            for (light, cascades) in &directional {
                if !light.shadow_maps_enabled {
                    continue;
                }
                for cascade in cascades.cascades.values().flatten() {
                    let clip = cascade.clip_from_world.project_point3(center);
                    let rx = cascade.clip_from_world.row(0).truncate().length() * radius;
                    let ry = cascade.clip_from_world.row(1).truncate().length() * radius;
                    if clip.x.abs() <= 1. + rx && clip.y.abs() <= 1. + ry {
                        pixels_per_meter =
                            pixels_per_meter.max(2. / cascade.texel_size.max(0.00001));
                    }
                }
            }
            for (light, pose, visible) in &points {
                if !light.shadow_maps_enabled || visible.is_some_and(|v| !v.get()) {
                    continue;
                }
                let distance = ((pose.translation() - center).length() - radius).max(0.05);
                if distance < light.range {
                    pixels_per_meter = pixels_per_meter.max(point_map.size as f32 / distance);
                }
            }
            for (light, pose, visible) in &spots {
                if !light.shadow_maps_enabled || visible.is_some_and(|v| !v.get()) {
                    continue;
                }
                let distance = ((pose.translation() - center).length() - radius).max(0.05);
                if distance < light.range {
                    pixels_per_meter = pixels_per_meter.max(
                        directional_map.size as f32
                            / (distance * light.outer_angle.tan()).max(0.05),
                    );
                }
            }
            if pixels_per_meter <= 0. {
                continue;
            }
            let textures = region.map_or(fallback.as_slice(), |region| region.textures.as_slice());
            for (id, local_density) in textures {
                if !usage.coverage.contains(id) {
                    continue;
                }
                let Some(resident) = streams.residents.get_mut(id) else {
                    continue;
                };
                let density = if region.is_some() {
                    local_density / scale
                } else {
                    *local_density
                };
                let no_mips = resident.requires_base_mip();
                let mip = if no_mips || density <= 0. {
                    0
                } else {
                    (density / pixels_per_meter).log2().floor().max(0.) as usize
                }
                .min(resident.entry.mips.len() - 1);
                resident.requested = resident.requested.min(mip);
                resident.class = resident.class.min(DemandClass::Indirect);
                resident.last_demand = now;
                requested += 1;
            }
        }
    }
    stats.shadow_requests = requested;
}

struct DetailStep {
    id: AssetId<Image>,
    mip: usize,
    bytes: u64,
    score: f64,
    uri: String,
}
impl PartialEq for DetailStep {
    fn eq(&self, other: &Self) -> bool {
        self.score == other.score && self.uri == other.uri && self.mip == other.mip
    }
}
impl Eq for DetailStep {}
impl PartialOrd for DetailStep {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for DetailStep {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.score
            .total_cmp(&other.score)
            .then_with(|| other.uri.cmp(&self.uri))
            .then_with(|| self.mip.cmp(&other.mip))
    }
}
fn detail_step(id: AssetId<Image>, resident: &Resident, mip: usize, previous: usize) -> DetailStep {
    let coverage = (resident.screen_coverage * 64.).ceil() / 64.;
    let priority = match resident.class {
        DemandClass::Foreground => 1024. * coverage.max(0.02),
        DemandClass::Camera => 16. * coverage.max(0.001),
        DemandClass::Indirect => 2.,
        DemandClass::Prefetch => 0.5,
        DemandClass::Idle => 0.01,
    };
    let bytes = resident.entry.gpu_bytes(mip) - resident.entry.gpu_bytes(mip + 1);
    let benefit = 2_f64.powi(-((mip - resident.stabilized) as i32));
    let retained = if mip >= previous { 1.05 } else { 1. };
    DetailStep {
        id,
        mip,
        bytes,
        score: f64::from(priority) * benefit * retained / bytes.max(1) as f64,
        uri: resident.entry.uri.clone(),
    }
}

fn allocate_detail(
    time: Res<Time<Real>>,
    mut streams: ResMut<TextureStreams>,
    mut stats: ResMut<TextureStreamingStats>,
) {
    if !streams.enabled {
        return;
    }
    let unregistered = streams.refresh_bootstrap_reservation(&mut stats);
    let limit = streams.budget.saturating_sub(unregistered);
    stats.naturally_requested_bytes = streams
        .residents
        .values()
        .map(|resident| resident.entry.resident_bytes(resident.requested))
        .sum();
    stats.screen_coverage_sum = streams
        .residents
        .values()
        .map(|resident| resident.screen_coverage)
        .sum();
    let mut selected: u64 = 0;
    let mut largest_replacement: u64 = 0;
    let mut improvements = BinaryHeap::new();
    let previous: HashMap<_, _> = streams
        .residents
        .iter()
        .map(|(id, resident)| (*id, resident.wanted))
        .collect();
    for (id, resident) in streams.residents.iter_mut() {
        if resident.requested <= resident.stabilized {
            resident.stabilized = resident.requested;
            resident.coarser_since = None;
        } else {
            let since = resident
                .coarser_since
                .get_or_insert(time.elapsed_secs_f64());
            if time.elapsed_secs_f64() - *since >= 4. {
                resident.stabilized = resident.requested;
                resident.coarser_since = None;
            }
        }
        resident.wanted = resident.entry.mips.len() - 1;
        selected += resident.entry.gpu_bytes(resident.wanted);
        largest_replacement = largest_replacement.max(resident.entry.gpu_bytes(resident.wanted));
        if resident.wanted > resident.stabilized {
            improvements.push(detail_step(
                *id,
                resident,
                resident.wanted - 1,
                previous[id],
            ));
        }
    }
    while let Some(step) = improvements.pop() {
        let replacement =
            largest_replacement.max(streams.residents[&step.id].entry.gpu_bytes(step.mip));
        if selected + step.bytes + replacement > limit {
            continue;
        }
        selected += step.bytes;
        largest_replacement = replacement;
        let resident = streams
            .residents
            .get_mut(&step.id)
            .expect("Allocated texture resident");
        resident.wanted = step.mip;
        if step.mip > resident.stabilized {
            improvements.push(detail_step(
                step.id,
                resident,
                step.mip - 1,
                previous[&step.id],
            ));
        }
    }
    stats.budget_limited_textures = streams
        .residents
        .values()
        .filter(|resident| resident.wanted > resident.requested)
        .count();
    stats.accepted_gpu_bytes = selected;
    stats.stabilized_requested_bytes = streams
        .residents
        .values()
        .map(|resident| resident.entry.resident_bytes(resident.stabilized))
        .sum();
}

fn upload(
    time: Res<Time<Real>>,
    mut streams: ResMut<TextureStreams>,
    mut stats: ResMut<TextureStreamingStats>,
    mut images: ResMut<Assets<Image>>,
    mut standard: ResMut<Assets<StandardMaterial>>,
    mut opaque: ResMut<Assets<super::opaque::OpaqueMaterial>>,
    mut diagnostic: ResMut<Assets<super::pbr::DiagnosticMaterial>>,
    memory: Res<memory::MemoryLedger>,
    mut replacements: ResMut<memory::Replacements>,
) {
    if !streams.enabled {
        return;
    }
    let _span = info_span!("texture_streaming_upload").entered();
    let bootstrap_reserved = streams.refresh_bootstrap_reservation(&mut stats);
    let mut frame_bytes = 0;
    let mut failure = None;
    let mut replaced = HashSet::new();
    let active: Vec<_> = streams
        .residents
        .iter()
        .filter(|(_, resident)| resident.request.is_some())
        .map(|(id, _)| *id)
        .collect();
    for id in active {
        let resident = streams
            .residents
            .get_mut(&id)
            .expect("Active texture request");
        let Some(request) = resident.request.as_mut() else {
            continue;
        };
        if (request.mip > resident.wanted && request.mip >= resident.mip)
            || request.mip < resident.wanted
        {
            resident.request = None;
            stats.cancelled_reads += 1;
            continue;
        }
        if request.ready.is_none() {
            let Some(result) = block_on(poll_once(&mut request.task)) else {
                continue;
            };
            match result {
                Ok(image) => request.ready = Some(image),
                Err(error) => {
                    error!("Streaming {}: {error}", resident.entry.uri);
                    failure = Some(format!("{}: {error}", resident.entry.uri));
                    stats.errors += 1;
                    resident.failed = true;
                    resident.request = None;
                    continue;
                }
            }
        }
    }
    let now = time.elapsed_secs_f64();
    let mut ready: Vec<_> = streams
        .residents
        .iter()
        .filter_map(|(id, resident)| {
            let request = resident.request.as_ref()?;
            request.ready.as_ref()?;
            Some(Candidate {
                id: *id,
                class: resident.class,
                since: resident.queued_since.unwrap_or(now),
                bytes: resident.entry.resident_bytes(request.mip),
                promotion: request.mip < resident.mip,
                uri: resident.entry.uri.clone(),
                coverage: resident.screen_coverage,
                gpu_bytes: resident.entry.gpu_bytes(request.mip),
            })
        })
        .collect();
    order_candidates(&mut ready);
    loop {
        let cursor = streams.upload_cursor;
        let reserved = streams
            .upload_reservation
            .take()
            .and_then(|(id, next_cursor)| {
                let position = ready.iter().position(|candidate| candidate.id == id)?;
                streams.upload_cursor = next_cursor;
                Some(ready.remove(position))
            });
        let Some(candidate) =
            reserved.or_else(|| fair_next(&mut ready, &mut streams.upload_cursor))
        else {
            break;
        };
        if frame_bytes != 0 && frame_bytes + candidate.bytes > UPLOAD_BYTES_PER_FRAME {
            streams.upload_reservation = Some((candidate.id, streams.upload_cursor));
            streams.upload_cursor = cursor;
            stats.upload_deferrals += 1;
            break;
        }
        let id = candidate.id;
        let resident = streams
            .residents
            .get_mut(&id)
            .expect("Ready texture resident");
        let request = resident.request.as_mut().expect("Ready texture request");
        let bytes = candidate.bytes;
        let old_bytes = resident.entry.gpu_bytes(resident.mip);
        let image = request.ready.take().expect("Ready streaming image");
        let descriptor = image.texture_descriptor.clone();
        if images.insert(id, image).is_err() {
            resident.failed = true;
            stats.errors += 1;
            failure = Some(format!(
                "Image asset retired during streaming: {}",
                resident.entry.uri
            ));
        } else {
            memory.replace(old_bytes);
            replacements.record(id, old_bytes, descriptor);
            stats.promotions += u64::from(request.mip < resident.mip);
            stats.demotions += u64::from(request.mip > resident.mip);
            stats.uploaded_bytes += bytes;
            resident.mip = request.mip;
            resident.last_change = time.elapsed_secs_f64();
            frame_bytes += bytes;
            replaced.insert(id.untyped());
            if candidate.class != DemandClass::Idle {
                stats.uploads_by_class[candidate.class as usize] += 1;
            }
        }
        resident.request = None;
    }
    if !replaced.is_empty() {
        let standard_count = refresh_materials(&mut standard, &replaced, dependencies_changed);
        let opaque_count = refresh_materials(&mut opaque, &replaced, |material, replaced| {
            dependencies_changed(&material.base, replaced)
                || dependencies_changed(&material.extension, replaced)
        });
        let diagnostic_count =
            refresh_materials(&mut diagnostic, &replaced, |material, replaced| {
                dependencies_changed(&material.base, replaced)
            });
        stats.standard_refreshes += standard_count;
        stats.opaque_refreshes += opaque_count;
        stats.diagnostic_refreshes += diagnostic_count;
        stats.material_refreshes += standard_count + opaque_count + diagnostic_count;
    }
    if let Some(error) = failure {
        streams.fail(error);
    }
    stats.managed_textures = streams.residents.len();
    stats.resident_mip_bytes = streams
        .residents
        .values()
        .map(|r| r.entry.resident_bytes(r.mip))
        .sum();
    stats.wanted_mip_bytes = streams
        .residents
        .values()
        .map(|r| r.entry.resident_bytes(r.wanted))
        .sum();
    stats.queued_reads = streams
        .residents
        .values()
        .filter(|r| r.request.is_some())
        .count();
    stats.pending_detail = streams
        .residents
        .values()
        .filter(|r| r.wanted < r.mip)
        .count();
    stats.anisotropic_textures = streams
        .residents
        .values()
        .filter(|r| matches!(&r.sampler, ImageSampler::Descriptor(s) if s.anisotropy_clamp > 1))
        .count();
    stats.resident_base_textures = streams.residents.values().filter(|r| r.mip == 0).count();
    stats.retiring_gpu_bytes = memory.retiring_bytes();
    stats.resident_gpu_bytes = streams
        .residents
        .values()
        .map(|resident| resident.entry.gpu_bytes(resident.mip))
        .sum();
    stats.replacement_reserved_bytes = streams
        .residents
        .values()
        .filter_map(|resident| {
            resident
                .request
                .as_ref()
                .map(|request| resident.entry.gpu_bytes(request.mip))
        })
        .sum();
    stats.accounted_gpu_bytes = stats.resident_gpu_bytes
        + stats.retiring_gpu_bytes
        + stats.replacement_reserved_bytes
        + bootstrap_reserved;
    stats.peak_accounted_gpu_bytes = stats
        .peak_accounted_gpu_bytes
        .max(stats.accounted_gpu_bytes);
    if stats.accounted_gpu_bytes > streams.budget {
        stats.errors += 1;
        streams.fail("Texture replacement exceeded its reserved VRAM allocation".into());
    }
}

fn refresh_materials<M: Material>(
    materials: &mut Assets<M>,
    replaced: &HashSet<bevy::asset::UntypedAssetId>,
    uses_replaced: impl Fn(&M, &HashSet<bevy::asset::UntypedAssetId>) -> bool,
) -> u64 {
    let affected: Vec<_> = materials
        .iter()
        .filter_map(|(id, material)| uses_replaced(material, replaced).then_some(id))
        .collect();
    for id in &affected {
        if let Some(material) = materials.get_mut(*id) {
            let _ = material.into_inner();
        }
    }
    affected.len() as u64
}

fn dependencies_changed<A: Asset>(
    asset: &A,
    replaced: &HashSet<bevy::asset::UntypedAssetId>,
) -> bool {
    let mut changed = false;
    asset.visit_dependencies(&mut |dependency| changed |= replaced.contains(&dependency));
    changed
}

fn schedule(
    server: Res<AssetServer>,
    time: Res<Time<Real>>,
    mut streams: ResMut<TextureStreams>,
    mut stats: ResMut<TextureStreamingStats>,
    memory: Res<memory::MemoryLedger>,
) {
    if !streams.enabled {
        return;
    }
    let _span = info_span!("texture_streaming_schedule").entered();
    let bootstrap_reserved = streams.refresh_bootstrap_reservation(&mut stats);
    let now = time.elapsed_secs_f64();
    let mut in_flight = 0;
    let mut pending = 0;
    let mut buffered = 0;
    let mut gpu_reserved = 0;
    stats.pending_by_class = [0; 4];
    for resident in streams.residents.values_mut() {
        if resident.wanted < resident.mip {
            resident.queued_since.get_or_insert(now);
            if resident.class != DemandClass::Idle {
                stats.pending_by_class[resident.class as usize] += 1;
            }
        } else if resident.request.is_none() {
            resident.queued_since = None;
        }
        if let Some(request) = &resident.request {
            pending += 1;
            in_flight += usize::from(request.ready.is_none());
            buffered += resident.entry.resident_bytes(request.mip);
            gpu_reserved += resident.entry.gpu_bytes(request.mip);
        }
    }
    let pool: u64 = streams
        .residents
        .values()
        .map(|r| r.entry.gpu_bytes(r.mip))
        .sum();
    let mut candidates = streams
        .residents
        .iter()
        .filter(|(_, resident)| {
            if resident.failed || resident.request.is_some() || resident.mip == resident.wanted {
                return false;
            }
            if resident.wanted < resident.mip {
                return true;
            }
            stats.budget_limited_textures > 0 || now - resident.last_change >= 2.
        })
        .map(|(id, resident)| Candidate {
            id: *id,
            class: if resident.wanted < resident.mip {
                resident.class
            } else {
                DemandClass::Idle
            },
            since: resident.queued_since.unwrap_or(resident.last_change),
            bytes: resident.entry.resident_bytes(resident.wanted),
            promotion: resident.wanted < resident.mip,
            uri: resident.entry.uri.clone(),
            coverage: resident.screen_coverage,
            gpu_bytes: resident.entry.gpu_bytes(resident.wanted),
        })
        .collect::<Vec<_>>();
    order_candidates(&mut candidates);
    while in_flight < MAX_IN_FLIGHT && pending < MAX_PENDING_REQUESTS {
        let cursor = streams.read_cursor;
        let reserved = streams
            .admission_reservation
            .take()
            .and_then(|(id, next_cursor)| {
                let position = candidates.iter().position(|candidate| candidate.id == id)?;
                streams.read_cursor = next_cursor;
                Some(candidates.remove(position))
            });
        let Some(mut candidate) =
            reserved.or_else(|| fair_next(&mut candidates, &mut streams.read_cursor))
        else {
            break;
        };
        let accounted = pool + memory.retiring_bytes() + gpu_reserved + bootstrap_reserved;
        if accounted + candidate.gpu_bytes > streams.budget {
            streams.admission_reservation = Some((candidate.id, streams.read_cursor));
            streams.read_cursor = cursor;
            stats.gpu_budget_drains += 1;
            if let Some(position) = candidates
                .iter()
                .position(|c| !c.promotion && accounted + c.gpu_bytes <= streams.budget)
            {
                candidate = candidates.remove(position);
            } else {
                break;
            }
        }
        if buffered != 0 && buffered + candidate.bytes > BUFFERED_READ_BYTES {
            streams.admission_reservation = Some((candidate.id, streams.read_cursor));
            streams.read_cursor = cursor;
            stats.admission_drains += 1;
            break;
        }
        let id = candidate.id;
        let resident = streams.residents.get_mut(&id).expect("Candidate resident");
        let mip = resident.wanted;
        let entry = resident.entry.clone();
        let sampler = resident.sampler.clone();
        let server = server.clone();
        resident.request = Some(Request {
            mip,
            ready: None,
            task: IoTaskPool::get()
                .spawn(async move { read_tail(server, entry, mip, sampler).await }),
        });
        in_flight += 1;
        pending += 1;
        buffered += candidate.bytes;
        gpu_reserved += candidate.gpu_bytes;
        if candidate.class != DemandClass::Idle {
            let index = candidate.class as usize;
            stats.reads_by_class[index] += 1;
            stats.max_read_wait_seconds[index] =
                stats.max_read_wait_seconds[index].max(now - candidate.since);
        }
    }
    stats.active_workers = in_flight;
    stats.queued_reads = pending;
    stats.pending_detail = streams
        .residents
        .values()
        .filter(|r| r.wanted < r.mip)
        .count();
    stats.pending_mip_changes = streams
        .residents
        .values()
        .filter(|r| r.wanted != r.mip)
        .count();
    stats.peak_active_workers = stats.peak_active_workers.max(in_flight);
    stats.buffered_read_bytes = buffered;
    stats.peak_buffered_read_bytes = stats.peak_buffered_read_bytes.max(buffered);
    stats.retiring_gpu_bytes = memory.retiring_bytes();
    stats.replacement_reserved_bytes = gpu_reserved;
    stats.accounted_gpu_bytes = pool + stats.retiring_gpu_bytes + gpu_reserved + bootstrap_reserved;
    stats.peak_accounted_gpu_bytes = stats
        .peak_accounted_gpu_bytes
        .max(stats.accounted_gpu_bytes);
}

async fn read_tail(
    server: AssetServer,
    entry: TextureEntry,
    mip: usize,
    sampler: ImageSampler,
) -> Result<Image, String> {
    let source = server
        .get_source(AssetSourceId::Default)
        .map_err(|e| e.to_string())?;
    let mut reader = source
        .reader()
        .read(Path::new(&entry.full_uri))
        .await
        .map_err(|e| e.to_string())?;
    let mut header = vec![0; entry.header_bytes];
    reader
        .read_exact(&mut header)
        .await
        .map_err(|e| e.to_string())?;
    if &header[..4] != b"DDS " {
        return Err("Invalid streaming DDS magic".into());
    }
    let start = &entry.mips[mip];
    match reader.seekable() {
        Ok(seekable) => {
            seekable
                .seek(SeekFrom::Start(start.offset))
                .await
                .map_err(|e| e.to_string())?;
        }
        Err(_) => {
            let mut remaining = start.offset - entry.header_bytes as u64;
            let mut scratch = [0u8; 8192];
            while remaining > 0 {
                let count = remaining.min(scratch.len() as u64) as usize;
                reader
                    .read_exact(&mut scratch[..count])
                    .await
                    .map_err(|e| e.to_string())?;
                remaining -= count as u64;
            }
        }
    }
    header[12..16].copy_from_slice(&start.height.to_le_bytes());
    header[16..20].copy_from_slice(&start.width.to_le_bytes());
    header[28..32].copy_from_slice(&((entry.mips.len() - mip) as u32).to_le_bytes());
    let flags = u32::from_le_bytes(header[8..12].try_into().expect("DDS flags"));
    let pitch = if flags & 0x80000 != 0 {
        start.bytes
    } else {
        start.bytes / start.height as u64
    };
    header[20..24].copy_from_slice(&(pitch as u32).to_le_bytes());
    let count = usize::try_from(entry.resident_bytes(mip)).map_err(|e| e.to_string())?;
    header.resize(entry.header_bytes + count, 0);
    reader
        .read_exact(&mut header[entry.header_bytes..])
        .await
        .map_err(|e| e.to_string())?;
    let mut image = Image::from_buffer(
        &header,
        ImageType::Format(ImageFormat::Dds),
        CompressedImageFormats::all(),
        entry.color_space == "srgb",
        sampler,
        RenderAssetUsages::RENDER_WORLD,
    )
    .map_err(|e| e.to_string())?;
    image.texture_descriptor.dimension = bevy::render::render_resource::TextureDimension::D2;
    Ok(image)
}

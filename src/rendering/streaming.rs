//! Range-streamed standard DDS mip tails, using Bevy assets and task pools.
//! Image IDs stay stable while Bevy replaces/reclaims differently sized GPU
//! textures. Visible demand takes precedence over the idle residency budget.
use bevy::{
    asset::{
        AssetEvent, AssetId, AsyncReadExt, AsyncSeekExt, RenderAssetUsages,
        io::{AssetReaderError, AssetSourceId},
    },
    camera::primitives::Aabb,
    gltf::GltfMaterialExtras,
    image::{CompressedImageFormats, ImageFormat, ImageSampler, ImageType},
    light::{Cascades, DirectionalLightShadowMap, NotShadowCaster, PointLightShadowMap},
    math::Vec3A,
    prelude::*,
    tasks::{IoTaskPool, Task, block_on, poll_once},
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    io::SeekFrom,
    path::Path,
    sync::Arc,
    time::Duration,
};

const INDEX_PATH: &str = "texture-streaming.json";
const MAX_IN_FLIGHT: usize = 2;
const UPLOAD_BYTES_PER_FRAME: u64 = 8 * 1024 * 1024;
const IDLE_POOL_BYTES: u64 = 2 * 1024 * 1024 * 1024;

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
}

struct Request {
    mip: usize,
    task: Task<Result<Image, String>>,
    ready: Option<Image>,
}

struct Resident {
    // This handle owns the bootstrap throughout streaming and scene replacement.
    _handle: Handle<Image>,
    entry: TextureEntry,
    mip: usize,
    wanted: usize,
    last_demand: f64,
    last_change: f64,
    sampler: ImageSampler,
    request: Option<Request>,
    failed: bool,
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
        }
    }
}

impl TextureStreams {
    fn fail(&mut self, error: String) {
        self.failure = Some(error);
        self.enabled = false;
        for resident in self.residents.values_mut() {
            resident.request = None;
        }
    }

    pub(crate) fn mip_descriptor(&self, id: AssetId<Image>) -> Option<TextureMipDescriptor> {
        let resident = self.residents.get(&id)?;
        Some(TextureMipDescriptor {
            full_extent: UVec2::new(resident.entry.width, resident.entry.height),
        })
    }

    /// Convert a pass footprint diameter in meters to a full-chain mip.
    /// Use the narrower axis of an anisotropic footprint conservatively.
    /// GI and reflection producers share the same disk-backed Image assets.
    pub(crate) fn mip_for_footprint(
        &self,
        id: AssetId<Image>,
        footprint_meters: f32,
    ) -> Option<usize> {
        let resident = self.residents.get(&id)?;
        let density = resident.entry.texels_per_meter;
        let mip = if resident.requires_base_mip()
            || density <= 0.
            || !footprint_meters.is_finite()
            || footprint_meters <= 0.
        {
            0
        } else {
            (density * footprint_meters).log2().floor().max(0.) as usize
        };
        Some(mip.min(resident.entry.mips.len() - 1))
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

#[derive(Clone, Copy)]
pub(crate) struct TextureMipDescriptor {
    pub full_extent: UVec2,
}

#[derive(Resource, Default, Serialize)]
pub(crate) struct TextureStreamingStats {
    pub managed_textures: usize,
    pub resident_mip_bytes: u64,
    pub wanted_mip_bytes: u64,
    pub queued_reads: usize,
    pub uploaded_bytes: u64,
    pub promotions: u64,
    pub demotions: u64,
    pub pending_detail: usize,
    pub errors: u32,
    pub lighting_requests: usize,
    pub shadow_requests: usize,
    pub anisotropic_textures: usize,
    pub resident_base_textures: usize,
    pub density_bound_meshes: usize,
    pub evaluated_surface_regions: usize,
    pub density_target: f32,
    pub density_capped_textures: usize,
    pub density_pending_textures: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum TextureConsumer {
    StaticGi,
    Reflection,
    Lighting,
}

struct PassDemand {
    mip: usize,
    valid_until: f64,
}

/// Pass producers replace their current requests after each completed batch.
/// A GPU feedback readback can request full-chain mips using these stable IDs.
#[derive(Resource, Default)]
pub(crate) struct TextureStreamingRequests {
    requests: HashMap<(AssetId<Image>, TextureConsumer), PassDemand>,
}

impl TextureStreamingRequests {
    /// `valid_until` uses Bevy's `Time<Real>::elapsed_secs_f64()` clock so
    /// residency can retire stale pass requests while gameplay is paused.
    pub(crate) fn request(
        &mut self,
        id: AssetId<Image>,
        consumer: TextureConsumer,
        mip: usize,
        valid_until: f64,
    ) {
        if valid_until.is_finite() {
            self.requests
                .insert((id, consumer), PassDemand { mip, valid_until });
        }
    }

    pub(crate) fn clear(&mut self, consumer: TextureConsumer) {
        self.requests.retain(|(_, pass), _| *pass != consumer);
    }
}

#[derive(Component)]
struct TextureUse {
    textures: Vec<AssetId<Image>>,
    coverage: Vec<AssetId<Image>>,
    regions: Arc<Vec<BoundRegion>>,
}

pub(super) struct TextureStreamingPlugin;
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) struct TextureStreamingSet;
impl Plugin for TextureStreamingPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TextureStreams>()
            .init_resource::<TextureStreamingStats>()
            .init_resource::<TextureStreamingRequests>()
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
                // Bevy computes visibility after asset events. Publish finished
                // images above first, then queue next-frame reads from this
                // frame's camera and shadow footprints without mutating assets.
                (bind_uses, demand, shadow_demand, pass_demand, schedule)
                    .chain()
                    .after(TextureStreamingSet)
                    .after(bevy::camera::visibility::VisibilitySystems::CheckVisibility)
                    .after(bevy::light::SimulationLightSystems::UpdateLightFrusta),
            );
    }
}

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

fn start(server: Res<AssetServer>, mut streams: ResMut<TextureStreams>) {
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
    // Drain events even while the optional index is loading; catch up once on
    // receipt rather than repeatedly scanning the asset collection each frame.
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
    let initial = streams.residents.is_empty();
    let ids = if initial {
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
        // Bevy retains the descriptor/ID after moving pixel bytes to render.
        // MAIN_WORLD would instead clone and retain a complete CPU mip chain.
        image.asset_usage = RenderAssetUsages::RENDER_WORLD;
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
            },
        );
    }
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
    cameras: Query<(&GlobalTransform, &Projection, &Camera), With<super::GameCamera>>,
    geometry: Query<(
        &TextureUse,
        &GlobalTransform,
        Option<&Aabb>,
        Option<&ViewVisibility>,
    )>,
) {
    if !streams.enabled || !streams.timer.tick(time.delta()).just_finished() {
        return;
    }
    let _span = info_span!("texture_streaming_demand").entered();
    let Ok((camera_pose, projection, camera)) = cameras.single() else {
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
    // No material gets an unconditional authored-resolution residency floor.
    // Each consumer requests its real footprint, including offscreen GI hits.
    for resident in streams.residents.values_mut() {
        resident.wanted = resident.entry.mips.len() - 1;
    }
    let mut bound_geometry = false;
    let mut evaluated_regions = 0;
    for (usage, pose, bounds, visible) in &geometry {
        bound_geometry |= !usage.textures.is_empty();
        if !usage.regions.is_empty() {
            // sqrt(||A||1 * ||A||inf) bounds the largest singular scale,
            // including shear; rotation can conservatively increase demand.
            let matrix = pose.affine().matrix3.abs();
            let rows = matrix * Vec3A::ONE;
            let columns = Vec3A::new(
                matrix.x_axis.element_sum(),
                matrix.y_axis.element_sum(),
                matrix.z_axis.element_sum(),
            );
            let scale = (rows.max_element() * columns.max_element())
                .sqrt()
                .max(0.00001);
            for region in usage.regions.iter() {
                let center = pose.transform_point(region.center);
                let radius = (matrix * Vec3A::from(region.half_extents)).length();
                let distance = ((center - position)
                    .length()
                    .min((center - predicted).length())
                    - radius
                    - 2.)
                    .max(0.05);
                if visible.is_some_and(|v| !v.get()) && distance > 15. {
                    continue;
                }
                evaluated_regions += 1;
                for (id, local_density) in &region.textures {
                    let Some(resident) = streams.residents.get_mut(id) else {
                        continue;
                    };
                    let density = local_density / scale;
                    let mip = if resident.requires_base_mip() || density <= 0. {
                        0
                    } else {
                        (density * distance / (focal * 2.)).log2().floor().max(0.) as usize
                    }
                    .min(resident.entry.mips.len() - 1);
                    resident.wanted = resident.wanted.min(mip);
                    resident.last_demand = now;
                }
            }
            continue;
        }
        let center = bounds.map_or(pose.translation(), |aabb| {
            pose.transform_point(aabb.center.into())
        });
        let radius = bounds.map_or(1., |aabb| {
            // Transform the complete bounds, including nonuniform/sheared
            // ancestors, before estimating the nearest possible surface.
            (pose.affine().matrix3.abs() * aabb.half_extents).length()
        });
        let distance = ((center - position)
            .length()
            .min((center - predicted).length())
            - radius
            - 2.)
            .max(0.05);
        // Offscreen consumers add their own footprints below. Movement-prefetch
        // keeps nearby surfaces ready before they enter the camera view.
        if visible.is_some_and(|v| !v.get()) && distance > 15. {
            continue;
        }
        let pixels_per_meter = focal / distance * 2.;
        for id in &usage.textures {
            let Some(resident) = streams.residents.get_mut(id) else {
                continue;
            };
            let density = resident.entry.texels_per_meter;
            let no_mips = resident.requires_base_mip();
            let mip = if no_mips || density <= 0. || distance < 1. {
                0
            } else {
                (density / pixels_per_meter).log2().floor().max(0.) as usize
            }
            .min(resident.entry.mips.len() - 1);
            resident.wanted = resident.wanted.min(mip);
            resident.last_demand = now;
        }
    }
    streams.evaluated_demand = bound_geometry || streams.entries.is_empty();
    stats.evaluated_surface_regions = evaluated_regions;
}

fn pass_demand(
    time: Res<Time<Real>>,
    mut streams: ResMut<TextureStreams>,
    mut requests: ResMut<TextureStreamingRequests>,
    mut stats: ResMut<TextureStreamingStats>,
) {
    let now = time.elapsed_secs_f64();
    requests
        .requests
        .retain(|_, request| request.valid_until >= now);
    stats.lighting_requests = requests.requests.len();
    for ((id, _), request) in &requests.requests {
        if let Some(resident) = streams.residents.get_mut(id) {
            let mip = if resident.requires_base_mip() {
                0
            } else {
                request.mip
            };
            resident.wanted = resident.wanted.min(mip.min(resident.entry.mips.len() - 1));
            resident.last_demand = now;
        }
    }
}

fn shadow_demand(
    time: Res<Time<Real>>,
    mut streams: ResMut<TextureStreams>,
    mut stats: ResMut<TextureStreamingStats>,
    geometry: Query<(&TextureUse, &GlobalTransform, Option<&Aabb>), Without<NotShadowCaster>>,
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
    for (usage, pose, bounds) in &geometry {
        if usage.coverage.is_empty() {
            continue;
        }
        let matrix = pose.affine().matrix3.abs();
        let rows = matrix * Vec3A::ONE;
        let columns = Vec3A::new(
            matrix.x_axis.element_sum(),
            matrix.y_axis.element_sum(),
            matrix.z_axis.element_sum(),
        );
        let scale = (rows.max_element() * columns.max_element())
            .sqrt()
            .max(0.00001);
        let fallback = if usage.regions.is_empty() {
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
            .map(Some)
            .chain(usage.regions.is_empty().then_some(None))
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
                resident.wanted = resident.wanted.min(mip);
                resident.last_demand = now;
                requested += 1;
            }
        }
    }
    stats.shadow_requests = requested;
}

fn upload(
    time: Res<Time<Real>>,
    mut streams: ResMut<TextureStreams>,
    mut stats: ResMut<TextureStreamingStats>,
    mut images: ResMut<Assets<Image>>,
) {
    if !streams.enabled {
        return;
    }
    let _span = info_span!("texture_streaming_upload").entered();
    let mut frame_bytes = 0;
    let mut failure = None;
    for (id, resident) in &mut streams.residents {
        let Some(request) = resident.request.as_mut() else {
            continue;
        };
        // A stale demotion must not discard detail newly required by any pass.
        // Cancel a stale promotion only when current residency already suffices.
        if (request.mip > resident.wanted && request.mip >= resident.mip)
            || (request.mip < resident.wanted && resident.mip <= resident.wanted)
        {
            resident.request = None;
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
        let bytes = resident.entry.resident_bytes(request.mip);
        // A large single promotion is allowed; otherwise it would starve.
        if frame_bytes > 0 && frame_bytes + bytes > UPLOAD_BYTES_PER_FRAME {
            continue;
        }
        let image = request.ready.take().expect("Ready streaming image");
        if images.insert(*id, image).is_err() {
            resident.failed = true;
            stats.errors += 1;
            failure = Some(format!(
                "Image asset retired during streaming: {}",
                resident.entry.uri
            ));
        } else {
            stats.promotions += u64::from(request.mip < resident.mip);
            stats.demotions += u64::from(request.mip > resident.mip);
            stats.uploaded_bytes += bytes;
            resident.mip = request.mip;
            resident.last_change = time.elapsed_secs_f64();
            frame_bytes += bytes;
        }
        resident.request = None;
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
}

fn schedule(server: Res<AssetServer>, time: Res<Time<Real>>, mut streams: ResMut<TextureStreams>) {
    if !streams.enabled {
        return;
    }
    let _span = info_span!("texture_streaming_schedule").entered();
    let now = time.elapsed_secs_f64();
    let in_flight = streams
        .residents
        .values()
        .filter(|r| r.request.is_some())
        .count();
    let pool: u64 = streams
        .residents
        .values()
        .map(|r| r.entry.resident_bytes(r.mip))
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
            now - resident.last_change >= 2.
                && (pool > IDLE_POOL_BYTES || now - resident.last_demand >= 5.)
        })
        .map(|(id, resident)| (*id, resident.wanted < resident.mip, resident.last_demand))
        .collect::<Vec<_>>();
    candidates.sort_unstable_by(|a, b| b.1.cmp(&a.1).then_with(|| b.2.total_cmp(&a.2)));
    for (id, _, _) in candidates
        .into_iter()
        .take(MAX_IN_FLIGHT.saturating_sub(in_flight))
    {
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
    }
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
            // Virtual readers without seek support retain bounded scratch RAM.
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
    Image::from_buffer(
        &header,
        ImageType::Format(ImageFormat::Dds),
        CompressedImageFormats::all(),
        entry.color_space == "srgb",
        sampler,
        RenderAssetUsages::RENDER_WORLD,
    )
    .map_err(|e| e.to_string())
}

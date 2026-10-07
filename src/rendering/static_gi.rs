//! Runtime irradiance fields: Solari owns hardware acceleration and scene bindings.
//! One shared world-aligned field rolls through its probes once per second.
use super::opaque::OpaqueMaterial;
use bevy::{
    asset::{AssetId, RenderAssetUsages, uuid_handle},
    ecs::schedule::{ConditionWithAccess, ScheduleCleanupPolicy},
    image::{ImageSampler, ImageSamplerDescriptor},
    light::{IrradianceVolume, LightProbe},
    mesh::{Indices, PrimitiveTopology},
    prelude::*,
    render::{
        Extract, ExtractSchedule, Render, RenderApp, RenderStartup,
        diagnostic::RecordDiagnostics,
        extract_component::{ExtractComponent, ExtractComponentPlugin},
        extract_resource::{ExtractResource, ExtractResourcePlugin},
        render_asset::RenderAssets,
        render_resource::{binding_types::*, *},
        renderer::{RenderContext, RenderDevice, RenderGraph, RenderGraphSystems, RenderQueue},
        texture::GpuImage,
    },
    shader::{Shader, ShaderImport},
    solari::scene::{RaytracingMesh3d, RaytracingSceneBindings, RaytracingScenePlugin},
};
use std::{
    collections::{HashMap, HashSet, VecDeque},
    hash::{DefaultHasher, Hash, Hasher},
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
};

pub(super) const GRID: UVec3 = UVec3::new(32, 16, 32);
const SPACING: [f32; 3] = [2., 8., 32.];
const COMPUTE: Handle<Shader> = uuid_handle!("cc86667c-7dfa-4eb8-9cf4-dc212c5cff70");
const SAMPLING: Handle<Shader> = uuid_handle!("bc979314-ad7f-465f-908d-de01ca5cac16");
const APPLY: Handle<Shader> = uuid_handle!("cd91d828-5d17-4531-a011-cd1d865dc5f6");
// 8x8 irradiance, packed 16x16 distance moments, and probe metadata.
pub(super) const PROBE_TILE: UVec2 = UVec2::new(16, 13);
const PROBE_TEXELS: u32 = PROBE_TILE.x * PROBE_TILE.y;
const MAX_BATCH: u32 = 2048;

#[derive(Component)]
struct Traced;
#[derive(Component, Clone, ExtractComponent)]
pub(super) struct LocalReflection;
#[derive(Resource, Default)]
struct TraceMeshes(HashMap<(AssetId<Mesh>, u32), Handle<Mesh>>);
#[derive(Resource, Default, Clone)]
pub(crate) struct StaticGiStats(Arc<AtomicU64>, Arc<AtomicU64>, Arc<SceneRefreshStats>);

#[derive(Default)]
struct SceneRefreshStats {
    geometry: AtomicU64,
    lighting: AtomicU64,
    materials: AtomicU64,
    bindings: AtomicU64,
    compaction: AtomicU64,
    cleanup: AtomicU64,
}

impl StaticGiStats {
    pub(crate) fn updates(&self) -> u64 {
        self.0.load(Ordering::Relaxed)
    }
    pub(crate) fn reflections(&self) -> u64 {
        self.1.load(Ordering::Relaxed)
    }
    pub(crate) fn scene_refreshes(&self) -> serde_json::Value {
        let load = |value: &AtomicU64| value.load(Ordering::Relaxed);
        serde_json::json!({
            "geometry_changes": load(&self.2.geometry),
            "lighting_changes": load(&self.2.lighting),
            "material_changes": load(&self.2.materials),
            "binding_rebuilds": load(&self.2.bindings),
            "settled_compactions": load(&self.2.compaction),
            "settled_texture_cleanup": load(&self.2.cleanup),
        })
    }
}

#[derive(Default)]
struct InputSnapshots {
    lamps: Vec<(Entity, Lamp)>,
    suns: Vec<(Entity, Vec3, Vec3, f32, f32)>,
    previous_suns: Vec<(Entity, Vec3, Vec3, f32, f32)>,
    geometry: HashMap<Entity, (AssetId<Mesh>, AssetId<StandardMaterial>, Mat4)>,
}

#[derive(Clone)]
pub(super) struct Cascade {
    entity: Entity,
    pub image: Handle<Image>,
    pub center: Vec3,
    pub spacing: f32,
    probes: Vec<IVec4>,
}
#[derive(Clone, Copy, PartialEq, ShaderType)]
struct Lamp {
    position_range: Vec4,
    radiance_radius: Vec4,
}
#[derive(Default)]
struct LampGrid {
    origin: Vec3,
    dimensions: UVec3,
    // Two words per cell (absolute start, count), followed by lamp indices.
    // Empty dimensions select the complete lamp list instead of clipping it.
    entries: Vec<u32>,
}
impl LampGrid {
    const CELL_SIZE: f32 = 16.;

    fn build(lamps: &[Lamp]) -> Self {
        if lamps.is_empty() {
            return Self {
                entries: vec![0],
                ..default()
            };
        }
        let mut low = Vec3::splat(f32::INFINITY);
        let mut high = Vec3::splat(f32::NEG_INFINITY);
        for lamp in lamps {
            let position = lamp.position_range.truncate();
            let radius = lamp.position_range.w.max(0.);
            if !position.is_finite() || !radius.is_finite() {
                return Self {
                    entries: vec![0],
                    ..default()
                };
            }
            low = low.min(position - Vec3::splat(radius));
            high = high.max(position + Vec3::splat(radius));
        }
        let origin = (low / Self::CELL_SIZE).floor() * Self::CELL_SIZE;
        let extent = ((high - origin) / Self::CELL_SIZE).floor();
        if !extent.is_finite() || extent.cmpgt(Vec3::splat(1_000_000.)).any() {
            return Self {
                entries: vec![0],
                ..default()
            };
        }
        let dimensions = extent.as_uvec3() + UVec3::ONE;
        let cells = u64::from(dimensions.x) * u64::from(dimensions.y) * u64::from(dimensions.z);
        // Bound preparation memory even for future widely separated lights.
        // The shader falls back to the complete list for this case and for
        // positions outside the grid; no light can disappear at its boundary.
        if cells > 1_000_000 {
            return Self {
                entries: vec![0],
                ..default()
            };
        }
        let mut lists = vec![Vec::new(); cells as usize];
        let mut references = 0usize;
        for (index, lamp) in lamps.iter().enumerate() {
            let position = lamp.position_range.truncate();
            let radius = lamp.position_range.w.max(0.);
            let first = ((position - Vec3::splat(radius) - origin) / Self::CELL_SIZE)
                .floor()
                .as_uvec3()
                .min(dimensions - UVec3::ONE);
            let last = ((position + Vec3::splat(radius) - origin) / Self::CELL_SIZE)
                .floor()
                .as_uvec3()
                .min(dimensions - UVec3::ONE);
            for z in first.z..=last.z {
                for y in first.y..=last.y {
                    for x in first.x..=last.x {
                        let cell_low = origin + UVec3::new(x, y, z).as_vec3() * Self::CELL_SIZE;
                        let nearest =
                            position.clamp(cell_low, cell_low + Vec3::splat(Self::CELL_SIZE));
                        if position.distance_squared(nearest) <= radius * radius {
                            references += 1;
                            if references > 1_000_000 {
                                return Self {
                                    entries: vec![0],
                                    ..default()
                                };
                            }
                            let cell = x + dimensions.x * (y + dimensions.y * z);
                            lists[cell as usize].push(index as u32);
                        }
                    }
                }
            }
        }
        let mut entries = vec![0; cells as usize * 2];
        for (cell, indices) in lists.into_iter().enumerate() {
            entries[cell * 2] = entries.len() as u32;
            entries[cell * 2 + 1] = indices.len() as u32;
            entries.extend(indices);
        }
        Self {
            origin,
            dimensions,
            entries,
        }
    }
}
#[derive(Resource, Clone, Default, ExtractResource)]
pub(super) struct GiInputs {
    pub cascades: Vec<Cascade>,
    lamps: Vec<Lamp>,
    generation: u32,
    scene_generation: u32,
    material_generation: u32,
    lighting_generation: u32,
    lamp_grid: Arc<LampGrid>,
    native_meshes: Arc<HashSet<AssetId<Mesh>>>,
    native_textures: Arc<HashSet<AssetId<Image>>>,
    retained_native_textures: Arc<HashSet<AssetId<Image>>>,
    reflection: Option<Handle<Image>>,
    reflection_entity: Option<Entity>,
    reflection_position: Vec3,
}
#[derive(Default)]
struct ProbeWork {
    origin: Option<IVec3>,
    priority: VecDeque<IVec4>,
    cursor: u32,
    credit: f32,
    receivers: HashSet<IVec3>,
}
#[derive(Resource, Default)]
struct Work(Vec<ProbeWork>);
#[derive(Clone, Copy, ShaderType)]
struct Constants {
    center_spacing: Vec4,
    grid_generation: UVec4,
    batch_lamps: UVec4,
    reflection_position: Vec4,
    lamp_grid_origin: Vec4,
    lamp_grid_extent: UVec4,
}
#[derive(Resource)]
struct GiPipeline {
    layout: BindGroupLayoutDescriptor,
    pipeline: CachedComputePipelineId,
    reflection_pipeline: CachedComputePipelineId,
    apply_layout: BindGroupLayoutDescriptor,
    apply_pipeline: CachedComputePipelineId,
    sampler: Sampler,
}
#[derive(Resource, Default)]
struct GiOutputs {
    updates: Option<Buffer>,
    generation: u32,
    lamps: StorageBuffer<Vec<Lamp>>,
    lamp_grid: StorageBuffer<Vec<u32>>,
    lighting_generation: u32,
    reflection_view: Option<(TextureId, TextureView)>,
}

pub(super) struct StaticGiPlugin;
impl Plugin for StaticGiPlugin {
    fn build(&self, app: &mut App) {
        let stats = StaticGiStats::default();
        app.add_plugins((
            RaytracingScenePlugin,
            super::ray_surface::RaySurfacePlugin,
            ExtractResourcePlugin::<GiInputs>::default(),
            ExtractComponentPlugin::<LocalReflection>::default(),
        ))
        .insert_resource(stats.clone())
        .init_resource::<GiInputs>()
        .init_resource::<TraceMeshes>()
        .init_resource::<super::ray_skin::Sources>()
        .init_resource::<super::ray_skin::Revision>()
        .init_resource::<super::ray_skin::RayContributorStats>()
        .init_resource::<Work>()
        .add_systems(PostStartup, setup)
        .add_systems(
            Update,
            (shaders, proxies.after(super::opaque::convert)).chain(),
        );
        app.add_systems(
            PostUpdate,
            (super::ray_skin::deform, super::ray_skin::visibility)
                .chain()
                .after(bevy::transform::TransformSystems::Propagate)
                .after(bevy::camera::visibility::VisibilitySystems::VisibilityPropagate)
                .before(bevy::asset::AssetEventSystems),
        );
        app.add_systems(
            PostUpdate,
            inputs
                .after(super::ray_skin::visibility)
                .after(bevy::asset::AssetEventSystems),
        );
        if let Some(render) = app.get_sub_app_mut(RenderApp) {
            render
                .insert_resource(stats)
                .init_resource::<GiOutputs>()
                .add_systems(RenderStartup, pipeline.after(super::ray_surface::layout));
        }
        // Native Solari installs its private systems in finish. Finalize after
        // that child plugin, before pipelined rendering takes RenderApp in cleanup.
        app.add_plugins(StaticGiSchedulePlugin);
    }
}
struct StaticGiSchedulePlugin;
impl Plugin for StaticGiSchedulePlugin {
    fn build(&self, _: &mut App) {}
    fn finish(&self, app: &mut App) {
        let render = app
            .get_sub_app_mut(RenderApp)
            .expect("Native render app before cleanup");
        // Native Solari owns extraction, TLAS and descriptor creation. Static
        // fields need fresh scene data only on their update, not every frame.
        // Pin the native systems by their diagnostic names because their modules
        // are private in Bevy 0.19.1; attach ordinary native run conditions.
        render.edit_schedule(ExtractSchedule, |schedule| {
            let keys: Vec<_> = schedule
                .graph()
                .systems
                .iter()
                .filter_map(|(key, system, _)| {
                    let name = format!("{}", system.name());
                    (name.contains("bevy_solari::scene::extract::extract_raytracing_scene")
                        || name.contains(
                            "extract_resource<bevy_solari::scene::extract::StandardMaterialAssets",
                        ))
                    .then_some((key, name.contains("extract_resource<")))
                })
                .collect();
            assert_eq!(keys.len(), 2, "Pinned Solari extraction contract");
            for (key, materials) in keys {
                let conditions = schedule
                    .graph_mut()
                    .systems
                    .get_conditions_mut(key)
                    .expect("Native conditions");
                if materials {
                    conditions.push(ConditionWithAccess::new(Box::new(IntoSystem::into_system(
                        extract_materials_due,
                    ))));
                } else {
                    conditions.push(ConditionWithAccess::new(Box::new(IntoSystem::into_system(
                        extract_due,
                    ))));
                }
            }
        });
        render.edit_schedule(Render, |schedule| {
            let key = schedule
                .graph()
                .systems
                .iter()
                .find_map(|(key, system, _)| {
                    format!("{}", system.name())
                        .contains("bevy_solari::scene::binder::prepare_raytracing_scene_bindings")
                        .then_some(key)
                })
                .expect("Pinned Solari binding contract");
            schedule
                .graph_mut()
                .systems
                .get_conditions_mut(key)
                .expect("Native conditions")
                .push(ConditionWithAccess::new(Box::new(IntoSystem::into_system(
                    prepare_due,
                ))));
        });
        // Attach native conditions before removal initializes the schedule.
        // Graph-side edits after initialization would be overwritten by the
        // executable's retained conditions on its next rebuild.
        use bevy::pbr::generate::{downsampling_system, filtering_system};
        render
            .world_mut()
            .schedule_scope(Render, |world, schedule| {
                let downsampling = schedule
                    .remove_systems_in_set(
                        downsampling_system,
                        world,
                        ScheduleCleanupPolicy::RemoveSystemsOnly,
                    )
                    .expect("Native environment downsampling contract");
                let filtering = schedule
                    .remove_systems_in_set(
                        filtering_system,
                        world,
                        ScheduleCleanupPolicy::RemoveSystemsOnly,
                    )
                    .expect("Native environment filtering contract");
                assert_eq!(
                    (downsampling, filtering),
                    (1, 1),
                    "One native environment pipeline"
                );
            });
        // RenderContext and diagnostic spans belong to the native root graph.
        // Keep current-frame production and native filters in submission order.
        render.add_systems(
            RenderGraph,
            (
                update,
                downsampling_system,
                filtering_system,
                super::sky_lighting::restore,
            )
                .chain()
                .after(RenderGraphSystems::Begin)
                .before(RenderGraphSystems::Render),
        );
    }
}
fn extract_due(inputs: Extract<Res<GiInputs>>, mut previous: Local<u32>) -> bool {
    let due = inputs.scene_generation != *previous;
    *previous = inputs.scene_generation;
    due
}
fn extract_materials_due(inputs: Extract<Res<GiInputs>>, mut previous: Local<u32>) -> bool {
    let due = inputs.material_generation != *previous;
    *previous = inputs.material_generation;
    due
}
#[derive(Default)]
struct NativeSceneRefresh {
    scene: (u32, usize, u64),
    retained_views: u64,
    compaction: u8,
    cleanup: u8,
}

fn prepare_due(
    inputs: Res<GiInputs>,
    meshes: Res<RenderAssets<bevy::render::mesh::RenderMesh>>,
    images: Res<RenderAssets<GpuImage>>,
    stats: Res<StaticGiStats>,
    mut previous: Local<NativeSceneRefresh>,
) -> bool {
    let views = |textures: &HashSet<AssetId<Image>>| {
        textures.iter().fold(0u64, |sum, id| {
            let mut hash = DefaultHasher::new();
            id.hash(&mut hash);
            images
                .get(*id)
                .map(|image| image.texture_view.id())
                .hash(&mut hash);
            sum.wrapping_add(hash.finish())
        })
    };
    let image_views = views(&inputs.native_textures);
    // Solari also binds unused raster templates. Their old views need eventual
    // release, but changing those unused descriptors cannot change a ray hit.
    // Debounce cleanup until promotions settle instead of rebuilding a TLAS
    // for every streamed image. Referenced native maps still refresh immediately.
    let retained_views = views(&inputs.retained_native_textures);
    let retained_changed = retained_views != previous.retained_views;
    if retained_changed {
        previous.retained_views = retained_views;
        previous.cleanup = 16;
    }
    let ready_meshes = inputs
        .native_meshes
        .iter()
        .filter(|id| meshes.get(**id).is_some())
        .count();
    let current = (inputs.scene_generation, ready_meshes, image_views);
    if current != previous.scene {
        previous.scene = current;
        previous.compaction = 16;
        previous.cleanup = 0;
        stats.2.bindings.fetch_add(1, Ordering::Relaxed);
        return true;
    }
    // A final settled rebuild lets the native TLAS adopt asynchronously
    // compacted BLAS. Idle scenes then retain their bindings indefinitely.
    if previous.compaction > 0 {
        previous.compaction -= 1;
        if previous.compaction == 0 {
            previous.cleanup = 0;
            stats.2.compaction.fetch_add(1, Ordering::Relaxed);
            stats.2.bindings.fetch_add(1, Ordering::Relaxed);
            return true;
        }
    }
    if previous.cleanup > 0 && !retained_changed {
        previous.cleanup -= 1;
        if previous.cleanup == 0 {
            stats.2.cleanup.fetch_add(1, Ordering::Relaxed);
            stats.2.bindings.fetch_add(1, Ordering::Relaxed);
            return true;
        }
    }
    false
}

fn setup(mut commands: Commands, mut images: ResMut<Assets<Image>>, mut inputs: ResMut<GiInputs>) {
    let mut reflection = Image::new_fill(
        Extent3d {
            width: 256,
            height: 256,
            depth_or_array_layers: 6,
        },
        TextureDimension::D2,
        &[0; 8],
        TextureFormat::Rgba16Float,
        RenderAssetUsages::RENDER_WORLD,
    );
    reflection.texture_descriptor.usage =
        TextureUsages::TEXTURE_BINDING | TextureUsages::STORAGE_BINDING;
    reflection.texture_view_descriptor = Some(TextureViewDescriptor {
        dimension: Some(TextureViewDimension::Cube),
        ..default()
    });
    let reflection = images.add(reflection);
    let entity = commands
        .spawn((
            bevy::light::GeneratedEnvironmentMapLight {
                environment_map: reflection.clone(),
                intensity: 1.,
                ..default()
            },
            LightProbe {
                falloff: Vec3::splat(0.125),
            },
            LocalReflection,
            Transform::from_scale(Vec3::new(32., 16., 32.)),
            bevy::camera::visibility::RenderLayers::from_layers(&[0, 1]),
            Name::new("Scene-derived local reflection probe"),
        ))
        .id();
    inputs.reflection = Some(reflection);
    inputs.reflection_entity = Some(entity);
    for spacing in SPACING {
        let mut image = Image::new_fill(
            Extent3d {
                width: GRID.x * PROBE_TILE.x,
                height: GRID.y * PROBE_TILE.y,
                depth_or_array_layers: GRID.z,
            },
            TextureDimension::D3,
            &[0; 8],
            TextureFormat::Rgba16Float,
            RenderAssetUsages::RENDER_WORLD,
        );
        image.texture_descriptor.usage = TextureUsages::TEXTURE_BINDING
            | TextureUsages::STORAGE_BINDING
            | TextureUsages::COPY_SRC
            | TextureUsages::COPY_DST;
        image.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor::linear());
        let image = images.add(image);
        let entity = commands
            .spawn((
                Transform::from_scale(GRID.as_vec3() * spacing),
                bevy::camera::visibility::RenderLayers::from_layers(&[0, 1]),
                Name::new(format!("Runtime GI {spacing} m probe cascade")),
            ))
            .id();
        inputs.cascades.push(Cascade {
            entity,
            image,
            center: Vec3::ZERO,
            spacing,
            probes: Vec::new(),
        });
    }
}

fn shaders(mut shaders: ResMut<Assets<Shader>>, mut done: Local<bool>) {
    if *done {
        return;
    }
    let Some(id) = shaders.iter().find_map(|(id, shader)| {
        (shader.import_path == ShaderImport::Custom("bevy_pbr::irradiance_volume".into()))
            .then_some(id)
    }) else {
        return;
    };
    shaders
        .insert(
            SAMPLING.id(),
            Shader::from_wgsl(include_str!("gi_sampling.wgsl"), "origins::gi_sampling"),
        )
        .expect("GI shader UUID");
    shaders
        .insert(
            COMPUTE.id(),
            Shader::from_wgsl(include_str!("static_gi.wgsl"), "origins::static_gi"),
        )
        .expect("GI shader UUID");
    shaders
        .insert(
            APPLY.id(),
            Shader::from_wgsl(include_str!("gi_apply.wgsl"), "origins::gi_apply"),
        )
        .expect("GI apply shader UUID");
    // All Bevy PBR materials use the same visibility-aware native volume lookup.
    // Keep one sampling implementation, but address native binding-array
    // textures inside the lookup, avoiding opaque binding-array resources in
    // the function interface used by the Vulkan shader compiler.
    let helper = include_str!("gi_sampling.wgsl").split_once('\n').expect("GI import declaration").1
        .replace("field: texture_3d<f32>", "texture_index: i32")
        .replace("fn sample_gi_field(", "fn sample_volume(")
        .replace("gi_read(field,", "gi_read(texture_index,")
        .replace("gi_direction_sample(field,", "gi_direction_sample(texture_index,")
        .replace("    let grid =", "#ifdef MULTIPLE_LIGHT_PROBES_IN_ARRAY\n    let field = irradiance_volumes[texture_index];\n#else\n    let field = irradiance_volume;\n#endif\n    let grid =");
    let volume = include_str!("gi_volume.wgsl").replace("// SAMPLING_HELPER", &helper);
    shaders
        .insert(id, Shader::from_wgsl(volume, "origins::gi_volume"))
        .expect("Engine volume shader");
    *done = true;
}

fn proxies(
    mut commands: Commands,
    meshes_query: Query<
        (
            Entity,
            &Mesh3d,
            Option<&MeshMaterial3d<OpaqueMaterial>>,
            Option<&MeshMaterial3d<StandardMaterial>>,
            &InheritedVisibility,
            Option<&bevy::mesh::skinning::SkinnedMesh>,
        ),
        (
            With<super::pbr::PbrReady>,
            Without<Traced>,
            Without<crate::assets::WorldCollider>,
        ),
    >,
    parents: Query<&ChildOf>,
    roots: Query<(), With<crate::assets::MapRoot>>,
    dynamic: Query<(), Or<(With<crate::enemies::Zombie>, With<super::DynamicSurface>)>>,
    first_person: Query<(), With<crate::presentation::ViewModelGroup>>,
    mut skin_sources: ResMut<super::ray_skin::Sources>,
    opaque: Res<Assets<OpaqueMaterial>>,
    images: Res<Assets<Image>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut cache: ResMut<TraceMeshes>,
    material_caches: (
        Local<HashMap<AssetId<StandardMaterial>, Handle<StandardMaterial>>>,
        Local<HashMap<AssetId<OpaqueMaterial>, Handle<StandardMaterial>>>,
    ),
    ray_data: (
        ResMut<super::ray_surface::Registry>,
        ResMut<super::ray_surface::Data>,
    ),
) {
    let (mut material_cache, mut opaque_cache) = material_caches;
    let (mut registry, mut data) = ray_data;
    for (entity, mesh, surface, standard, visibility, skin) in &meshes_query {
        if !visibility.get() {
            continue;
        }
        let mut parent = Some(entity);
        let mut static_world = false;
        let mut movable = false;
        let mut viewmodel = false;
        while let Some(owner) = parent {
            movable |= dynamic.contains(owner);
            viewmodel |= first_person.contains(owner);
            static_world |= roots.contains(owner);
            parent = parents.get(owner).ok().map(ChildOf::parent);
        }
        if (!static_world && !movable) || viewmodel {
            continue;
        }
        let Some(source) = meshes.get(&mesh.0) else {
            continue;
        };
        let material = if let Some(surface) = surface {
            let Some(surface) = opaque.get(&surface.0) else {
                continue;
            };
            surface.extension.trace_material(&surface.base)
        } else if let Some(standard) = standard {
            let Some(material) = materials.get(&standard.0) else {
                continue;
            };
            material.clone()
        } else {
            continue;
        };
        // Solari's opaque BLAS cannot represent transparent ray intersections.
        if !matches!(material.alpha_mode, AlphaMode::Opaque | AlphaMode::Mask(_)) {
            commands.entity(entity).insert(Traced);
            continue;
        }
        let controls = surface
            .and_then(|s| opaque.get(&s.0))
            .is_some_and(|s| s.extension.ray_parameters().layers.x > 1)
            || (standard.is_some() && source.attribute(Mesh::ATTRIBUTE_COLOR).is_some());
        let vertex_offset = registry.vertices(source, mesh.0.id(), &mut data, controls);
        let skin_source = if skin.is_some() {
            if let std::collections::hash_map::Entry::Vacant(entry) =
                skin_sources.0.entry(mesh.0.id())
            {
                let Some(source) = super::ray_skin::Source::read(source) else {
                    continue;
                };
                entry.insert(Arc::new(source));
            }
            skin_sources.0.get(&mesh.0.id()).cloned()
        } else {
            None
        };
        let trace_mesh = if let Some(source) = &skin_source {
            let mut trace = source.mesh();
            super::ray_surface::encode_vertices(&mut trace, vertex_offset);
            meshes.add(trace)
        } else if let Some(handle) = cache.0.get(&(mesh.0.id(), vertex_offset)) {
            handle.clone()
        } else {
            let mut trace = Mesh::new(
                PrimitiveTopology::TriangleList,
                RenderAssetUsages::RENDER_WORLD,
            );
            let attributes = [
                Mesh::ATTRIBUTE_POSITION,
                Mesh::ATTRIBUTE_NORMAL,
                Mesh::ATTRIBUTE_UV_0,
                Mesh::ATTRIBUTE_TANGENT,
            ];
            if attributes
                .iter()
                .any(|attribute| source.try_attribute(*attribute).is_err())
            {
                continue;
            }
            for attribute in attributes {
                let values = source
                    .attribute(attribute)
                    .expect("Checked Solari vertex layout");
                trace.insert_attribute(attribute, values.clone());
            }
            let Some(indices) = source.indices() else {
                continue;
            };
            trace.insert_indices(Indices::U32(indices.iter().map(|i| i as u32).collect()));
            trace.enable_raytracing = true;
            super::ray_surface::encode_vertices(&mut trace, vertex_offset);
            let handle = meshes.add(trace);
            cache.0.insert((mesh.0.id(), vertex_offset), handle.clone());
            handle
        };
        // Material ownership is shared across the repeated static instances.
        let trace_material = if let Some(standard) = standard {
            material_cache
                .entry(standard.0.id())
                .or_insert_with(|| {
                    let id = registry.material(&material, None, &images, &mut data);
                    // Companion metadata owns the complete authored material.
                    // Native carrier maps would retain duplicate views and
                    // unnecessarily tie TLAS refreshes to their residency.
                    materials.add(StandardMaterial {
                        reflectance: -(id as f32 + 1.),
                        emissive: material.emissive,
                        ..default()
                    })
                })
                .clone()
        } else {
            // Extended material IDs have a separate namespace; cache them below.
            let key = surface.expect("Extended material").0.id();
            opaque_cache
                .entry(key)
                .or_insert_with(|| {
                    let original = opaque.get(key).expect("Loaded extended material");
                    let id = registry.material(
                        &original.base,
                        Some(&original.extension),
                        &images,
                        &mut data,
                    );
                    materials.add(StandardMaterial {
                        reflectance: -(id as f32 + 1.),
                        emissive: material.emissive,
                        ..default()
                    })
                })
                .clone()
        };
        let mut proxy = commands.spawn((
            RaytracingMesh3d(trace_mesh.clone()),
            MeshMaterial3d(trace_material.clone()),
            ChildOf(entity),
            super::ray_skin::Provider {
                owner: entity,
                mesh: trace_mesh.clone(),
            },
            super::ray_surface::Usage {
                owner: entity,
                textures: data.textures_for(
                    (-materials
                        .get(&trace_material)
                        .expect("Ray material")
                        .reflectance
                        - 1.) as usize,
                ),
            },
            Name::new("Shared lighting ray geometry"),
        ));
        if let Some(source) = skin_source {
            proxy.insert(super::ray_skin::Skin {
                owner: entity,
                source,
                mesh: trace_mesh,
                previous: Vec::new(),
                vertex_offset,
            });
        }
        commands.entity(entity).insert(Traced);
    }
}

fn cell(index: u32) -> IVec3 {
    IVec3::new(
        (index % GRID.x) as i32,
        ((index / GRID.x) % GRID.y) as i32,
        (index / (GRID.x * GRID.y)) as i32,
    )
}

fn queue_region(work: &mut ProbeWork, origin: IVec3, previous: Option<IVec3>, reset: bool) {
    let extent = GRID.as_ivec3();
    let mut cells = Vec::new();
    for index in 0..GRID.element_product() {
        let position = origin + cell(index);
        if reset
            || previous.is_none_or(|old| {
                !(position.cmpge(old).all() && position.cmplt(old + extent).all())
            })
        {
            cells.push(position.extend(i32::from(reset)));
        }
    }
    let center = origin + extent / 2;
    cells.sort_unstable_by_key(|p| (p.truncate() - center).abs().element_sum());
    work.priority.extend(cells);
}

fn inputs(
    time: Res<Time<Real>>,
    state: Res<State<crate::GameState>>,
    cameras: Query<&GlobalTransform, With<super::GameCamera>>,
    lights: Query<(Entity, &GlobalTransform, &PointLight)>,
    receivers: Query<
        (&GlobalTransform, Option<&bevy::camera::primitives::Aabb>),
        With<super::DynamicSurface>,
    >,
    suns: Query<(
        Entity,
        &DirectionalLight,
        &GlobalTransform,
        Option<&bevy::light::SunDisk>,
    )>,
    changed_geometry: Query<
        (
            Entity,
            &RaytracingMesh3d,
            &MeshMaterial3d<StandardMaterial>,
            &GlobalTransform,
        ),
        Or<(
            Changed<GlobalTransform>,
            Changed<RaytracingMesh3d>,
            Changed<MeshMaterial3d<StandardMaterial>>,
        )>,
    >,
    mut removed_geometry: RemovedComponents<RaytracingMesh3d>,
    material_state: (
        MessageReader<AssetEvent<StandardMaterial>>,
        Res<Assets<StandardMaterial>>,
        Query<(&RaytracingMesh3d, &MeshMaterial3d<StandardMaterial>)>,
    ),
    mut work: ResMut<Work>,
    mut inputs: ResMut<GiInputs>,
    stats: Res<StaticGiStats>,
    mut volumes: Query<&mut Transform>,
    mut commands: Commands,
    locals: (Local<bool>, Local<u64>, Local<InputSnapshots>),
    deformation: Res<super::ray_skin::Revision>,
) {
    let (mut material_events, materials, ray_instances) = material_state;
    let (mut attached, mut previous_skin, mut snapshots) = locals;
    let skin_changed = *previous_skin != deformation.0;
    *previous_skin = deformation.0;
    if !*attached && stats.updates() > 0 {
        for cascade in &inputs.cascades {
            commands.entity(cascade.entity).insert((
                LightProbe {
                    falloff: Vec3::splat(0.15),
                },
                IrradianceVolume {
                    voxels: cascade.image.clone(),
                    intensity: 1.,
                    ..default()
                },
            ));
        }
        *attached = true;
    }
    let material_change = material_events
        .read()
        .filter(|event| !matches!(event, AssetEvent::LoadedWithDependencies { .. }))
        .count()
        != 0;
    let mut removed = false;
    for entity in removed_geometry.read() {
        snapshots.geometry.remove(&entity);
        removed = true;
    }
    if **state == crate::GameState::Loading || inputs.cascades.is_empty() {
        return;
    }
    let Ok(camera) = cameras.single() else {
        return;
    };
    inputs.reflection_position = camera.translation();
    if let Some(entity) = inputs.reflection_entity
        && let Ok(mut transform) = volumes.get_mut(entity)
    {
        transform.translation = camera.translation();
    }
    // Native shadow caching moves light entities between archetypes. Query order
    // is therefore not a light revision. Retain scratch storage and compare
    // stable entity order and the numeric fields consumed by ray lighting.
    snapshots.lamps.clear();
    snapshots
        .lamps
        .extend(lights.iter().map(|(entity, pose, light)| {
            (
                entity,
                Lamp {
                    position_range: pose.translation().extend(light.range),
                    radiance_radius: (light.color.to_linear().to_vec3() * light.intensity
                        / std::f32::consts::TAU
                        / 2.)
                        .extend(light.radius),
                },
            )
        }));
    snapshots
        .lamps
        .sort_unstable_by_key(|(entity, _)| entity.to_bits());
    let lamps_changed = snapshots.lamps.len() != inputs.lamps.len()
        || snapshots
            .lamps
            .iter()
            .map(|(_, lamp)| lamp)
            .ne(inputs.lamps.iter());
    snapshots.suns.clear();
    snapshots
        .suns
        .extend(suns.iter().map(|(entity, light, pose, disk)| {
            (
                entity,
                *pose.back(),
                light.color.to_linear().to_vec3(),
                light.illuminance,
                disk.map_or(bevy::light::SunDisk::EARTH.angular_size, |disk| {
                    disk.angular_size
                }),
            )
        }));
    snapshots
        .suns
        .sort_unstable_by_key(|(entity, ..)| entity.to_bits());
    let sun_changed = snapshots.suns != snapshots.previous_suns;
    if sun_changed {
        let InputSnapshots {
            suns,
            previous_suns,
            ..
        } = &mut *snapshots;
        previous_suns.clone_from(suns);
    }
    let lighting_changed = lamps_changed || sun_changed;
    let mut geometry_changed = removed || skin_changed;
    // Animation/transform propagation can mark an unchanged matrix as Changed.
    // Only actual current-frame geometry, pose or material changes need a TLAS.
    for (entity, mesh, material, pose) in &changed_geometry {
        let current = (mesh.0.id(), material.0.id(), pose.to_matrix());
        if snapshots.geometry.get(&entity) != Some(&current) {
            snapshots.geometry.insert(entity, current);
            geometry_changed = true;
        }
    }
    if geometry_changed {
        stats.2.geometry.fetch_add(1, Ordering::Relaxed);
    }
    if lighting_changed {
        stats.2.lighting.fetch_add(1, Ordering::Relaxed);
    }
    if material_change {
        stats.2.materials.fetch_add(1, Ordering::Relaxed);
    }
    let material_refresh = inputs.material_generation == 0 || material_change;
    if material_refresh {
        inputs.material_generation = inputs.material_generation.wrapping_add(1);
        // Bevy 0.19.1's native binder inspects these four channels on every
        // StandardMaterial, including retained raster templates. Other GPU
        // images (UI, layered companions, generated atlases) cannot change its
        // descriptors and must not force a fresh native TLAS.
        inputs.retained_native_textures = Arc::new(
            materials
                .iter()
                .flat_map(|(_, material)| {
                    [
                        &material.base_color_texture,
                        &material.normal_map_texture,
                        &material.emissive_texture,
                        &material.metallic_roughness_texture,
                    ]
                })
                .flatten()
                .map(Handle::id)
                .collect(),
        );
    }
    if material_refresh || geometry_changed {
        inputs.native_meshes =
            Arc::new(ray_instances.iter().map(|(mesh, _)| mesh.0.id()).collect());
        let ids: HashSet<_> = ray_instances
            .iter()
            .map(|(_, material)| material.0.id())
            .collect();
        inputs.native_textures = Arc::new(
            ids.into_iter()
                .filter_map(|id| materials.get(id))
                .flat_map(|material| {
                    [
                        &material.base_color_texture,
                        &material.normal_map_texture,
                        &material.emissive_texture,
                        &material.metallic_roughness_texture,
                    ]
                })
                .flatten()
                .map(Handle::id)
                .collect(),
        );
    }
    // Companion image bindings track resident views directly. A timed rebuild
    // of the entire native scene is unnecessary when its content is unchanged.
    if inputs.scene_generation == 0 || sun_changed || geometry_changed || material_change {
        inputs.scene_generation = inputs.scene_generation.wrapping_add(1);
    }
    if inputs.lighting_generation == 0 || lamps_changed {
        inputs.lamps.clear();
        inputs
            .lamps
            .extend(snapshots.lamps.iter().map(|(_, lamp)| *lamp));
        inputs.lamp_grid = Arc::new(LampGrid::build(&inputs.lamps));
        inputs.lighting_generation = inputs.lighting_generation.wrapping_add(1);
    }
    work.0
        .resize_with(inputs.cascades.len(), ProbeWork::default);
    for (cascade, schedule) in inputs.cascades.iter_mut().zip(&mut work.0) {
        cascade.center = (camera.translation() / cascade.spacing).round() * cascade.spacing;
        let origin = (cascade.center / cascade.spacing).as_ivec3() - GRID.as_ivec3() / 2;
        if schedule.origin != Some(origin) || lighting_changed || material_change {
            let reset = lighting_changed || material_change;
            if reset
                || schedule
                    .origin
                    .is_none_or(|old| (origin - old).abs().cmpge(GRID.as_ivec3()).any())
            {
                schedule.priority.clear();
            }
            let old_origin = schedule.origin;
            queue_region(schedule, origin, old_origin, reset);
            schedule.origin = Some(origin);
        }
        if let Ok(mut transform) = volumes.get_mut(cascade.entity) {
            transform.translation = cascade.center;
        }
        // Classification may sleep probes in empty space. A moving receiver
        // still needs its interpolation corners, including the overlay rig.
        // Wake newly requested cells and retain their world-space history.
        let mut receiver_cells = HashSet::new();
        for (pose, bounds) in &receivers {
            let center = bounds.map_or(pose.translation(), |b| {
                pose.affine().transform_point3a(b.center).into()
            });
            let radius = bounds.map_or(1., |b| {
                (pose.affine().matrix3.abs() * b.half_extents).length()
            });
            let low = ((center - Vec3::splat(radius)) / cascade.spacing - Vec3::splat(0.5))
                .floor()
                .as_ivec3()
                .max(origin);
            let high = ((center + Vec3::splat(radius)) / cascade.spacing - Vec3::splat(0.5))
                .floor()
                .as_ivec3()
                .saturating_add(IVec3::ONE)
                .min(origin + GRID.as_ivec3() - IVec3::ONE);
            for z in low.z..=high.z {
                for y in low.y..=high.y {
                    for x in low.x..=high.x {
                        receiver_cells.insert(IVec3::new(x, y, z));
                    }
                }
            }
        }
        let mut newly_required: Vec<_> = receiver_cells
            .difference(&schedule.receivers)
            .copied()
            .collect();
        newly_required
            .sort_unstable_by_key(|p| (*p - origin - GRID.as_ivec3() / 2).abs().element_sum());
        for cell in newly_required.into_iter().rev() {
            schedule.priority.push_front(cell.extend(4));
        }
        schedule.receivers = receiver_cells;
        // Rolling one-second sweep, independent of image reconstruction and FPS.
        schedule.credit = (schedule.credit
            + time.delta_secs().min(1.) * GRID.element_product() as f32)
            .min(MAX_BATCH as f32);
        let count = schedule.credit.floor() as u32;
        schedule.credit -= count as f32;
        cascade.probes.clear();
        for _ in 0..count {
            let priority = loop {
                let Some(p) = schedule.priority.pop_front() else {
                    break None;
                };
                if p.truncate().cmpge(origin).all()
                    && p.truncate().cmplt(origin + GRID.as_ivec3()).all()
                {
                    break Some(p);
                }
            };
            let mut probe = priority.unwrap_or_else(|| {
                let p = (origin + cell(schedule.cursor)).extend(0);
                schedule.cursor = (schedule.cursor + 1) % GRID.element_product();
                p
            });
            if schedule.receivers.contains(&probe.truncate()) {
                probe.w |= 4;
            }
            // Distinct workgroups may never write the same probe tile.
            if !cascade
                .probes
                .iter()
                .any(|p| p.truncate() == probe.truncate())
            {
                cascade.probes.push(probe);
            }
        }
    }
    inputs.generation = inputs.generation.wrapping_add(1);
}

fn pipeline(
    mut commands: Commands,
    scene: Res<RaytracingSceneBindings>,
    device: Res<RenderDevice>,
    cache: Res<PipelineCache>,
    surfaces: Res<super::ray_surface::Layout>,
) {
    let layout = BindGroupLayoutDescriptor::new(
        "ddgi_trace_and_blend",
        &BindGroupLayoutEntries::sequential(
            ShaderStages::COMPUTE,
            (
                uniform_buffer::<Constants>(false),
                texture_3d(TextureSampleType::Float { filterable: true }),
                storage_buffer_sized(false, None),
                sampler(SamplerBindingType::Filtering),
                texture_cube(TextureSampleType::Float { filterable: true }),
                storage_buffer_read_only::<Vec<Lamp>>(false),
                storage_buffer_read_only::<Vec<IVec4>>(false),
                texture_storage_2d_array(
                    TextureFormat::Rgba16Float,
                    StorageTextureAccess::WriteOnly,
                ),
                storage_buffer_read_only::<Vec<u32>>(false),
            ),
        ),
    );
    let apply_layout = BindGroupLayoutDescriptor::new(
        "ddgi_apply",
        &BindGroupLayoutEntries::sequential(
            ShaderStages::COMPUTE,
            (
                uniform_buffer::<Constants>(false),
                storage_buffer_read_only_sized(false, None),
                texture_storage_3d(TextureFormat::Rgba16Float, StorageTextureAccess::WriteOnly),
                storage_buffer_read_only::<Vec<IVec4>>(false),
            ),
        ),
    );
    let pipeline = cache.queue_compute_pipeline(ComputePipelineDescriptor {
        label: Some("DDGI hardware trace and world-space accumulation".into()),
        layout: vec![
            scene.bind_group_layout.clone(),
            layout.clone(),
            surfaces.0.clone(),
        ],
        shader: COMPUTE,
        entry_point: Some("main".into()),
        ..default()
    });
    let reflection_pipeline = cache.queue_compute_pipeline(ComputePipelineDescriptor {
        label: Some("Local scene reflection rays".into()),
        layout: vec![
            scene.bind_group_layout.clone(),
            layout.clone(),
            surfaces.0.clone(),
        ],
        shader: COMPUTE,
        entry_point: Some("reflection".into()),
        ..default()
    });
    let apply_pipeline = cache.queue_compute_pipeline(ComputePipelineDescriptor {
        label: Some("DDGI persistent atlas writes".into()),
        layout: vec![apply_layout.clone()],
        shader: APPLY,
        ..default()
    });
    commands.insert_resource(GiPipeline {
        layout,
        pipeline,
        reflection_pipeline,
        apply_layout,
        apply_pipeline,
        sampler: device.create_sampler(&SamplerDescriptor {
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            mipmap_filter: MipmapFilterMode::Linear,
            ..default()
        }),
    });
}

fn update(
    inputs: Res<GiInputs>,
    pipeline: Res<GiPipeline>,
    scene: Res<RaytracingSceneBindings>,
    cache: Res<PipelineCache>,
    images: Res<RenderAssets<GpuImage>>,
    environment: Query<&super::sky_lighting::SkyEnvironment>,
    device: Res<RenderDevice>,
    queue: Res<RenderQueue>,
    mut outputs: ResMut<GiOutputs>,
    stats: Res<StaticGiStats>,
    surfaces: Res<super::ray_surface::Prepared>,
    mut ctx: RenderContext,
) {
    if inputs.generation == outputs.generation || inputs.cascades.is_empty() {
        return;
    }
    let (Some(compute), Some(apply), Some(scene_group), Some(surface_group)) = (
        cache.get_compute_pipeline(pipeline.pipeline),
        cache.get_compute_pipeline(pipeline.apply_pipeline),
        scene.bind_group.as_ref(),
        surfaces.group.as_ref(),
    ) else {
        return;
    };
    let Ok(environment) = environment.single() else {
        return;
    };
    let Some(sky) = images.get(&environment.0) else {
        return;
    };
    let Some(reflection) = inputs
        .reflection
        .as_ref()
        .and_then(|image| images.get(image))
    else {
        return;
    };
    if outputs
        .reflection_view
        .as_ref()
        .is_none_or(|(id, _)| *id != reflection.texture.id())
    {
        outputs.reflection_view = Some((
            reflection.texture.id(),
            reflection.texture.create_view(&TextureViewDescriptor {
                dimension: Some(TextureViewDimension::D2Array),
                ..default()
            }),
        ));
    }
    if inputs
        .cascades
        .iter()
        .any(|c| images.get(&c.image).is_none())
    {
        return;
    }
    if inputs
        .cascades
        .iter()
        .all(|cascade| cascade.probes.is_empty())
        && cache
            .get_compute_pipeline(pipeline.reflection_pipeline)
            .is_none()
    {
        return;
    }
    super::ray_surface::reset_feedback(&surfaces, ctx.command_encoder());
    if outputs.updates.is_none() {
        outputs.updates = Some(device.create_buffer(&BufferDescriptor {
            label: Some("DDGI bounded probe update scratch"),
            size: u64::from(MAX_BATCH * PROBE_TEXELS) * 16,
            usage: BufferUsages::STORAGE,
            mapped_at_creation: false,
        }));
    }
    if outputs.lighting_generation != inputs.lighting_generation {
        let lamps = if inputs.lamps.is_empty() {
            vec![Lamp {
                position_range: Vec4::ZERO,
                radiance_radius: Vec4::ZERO,
            }]
        } else {
            inputs.lamps.clone()
        };
        outputs.lamps.set(lamps);
        outputs.lamps.write_buffer(&device, &queue);
        outputs.lamp_grid.set(inputs.lamp_grid.entries.clone());
        outputs.lamp_grid.write_buffer(&device, &queue);
        outputs.lighting_generation = inputs.lighting_generation;
    }
    let reflection_view = &outputs
        .reflection_view
        .as_ref()
        .expect("Reflection array view")
        .1;
    let scratch = outputs.updates.as_ref().expect("DDGI update buffer");
    let diagnostics = ctx.diagnostic_recorder();
    let diagnostics = diagnostics.as_deref();
    let mut recorded = false;
    for cascade in &inputs.cascades {
        if cascade.probes.is_empty() {
            continue;
        }
        let image = images.get(&cascade.image).expect("Checked DDGI texture");
        let mut constants = UniformBuffer::from(Constants {
            center_spacing: cascade.center.extend(cascade.spacing),
            grid_generation: GRID.extend(inputs.generation),
            batch_lamps: UVec4::new(cascade.probes.len() as u32, inputs.lamps.len() as u32, 0, 0),
            reflection_position: inputs.reflection_position.extend(0.),
            lamp_grid_origin: inputs.lamp_grid.origin.extend(LampGrid::CELL_SIZE),
            lamp_grid_extent: inputs.lamp_grid.dimensions.extend(0),
        });
        constants.write_buffer(&device, &queue);
        let mut probes = StorageBuffer::from(cascade.probes.clone());
        probes.write_buffer(&device, &queue);
        let trace_group = device.create_bind_group(
            "ddgi_trace_and_blend",
            &cache.get_bind_group_layout(&pipeline.layout),
            &BindGroupEntries::sequential((
                constants.binding().expect("DDGI constants"),
                &image.texture_view,
                scratch.as_entire_binding(),
                &pipeline.sampler,
                &sky.texture_view,
                outputs.lamps.binding().expect("DDGI lamps"),
                probes.binding().expect("DDGI probes"),
                reflection_view,
                outputs.lamp_grid.binding().expect("DDGI lamp grid"),
            )),
        );
        let apply_group = device.create_bind_group(
            "ddgi_apply",
            &cache.get_bind_group_layout(&pipeline.apply_layout),
            &BindGroupEntries::sequential((
                constants.binding().expect("DDGI constants"),
                scratch.as_entire_binding(),
                &image.texture_view,
                probes.binding().expect("DDGI probes"),
            )),
        );
        {
            let mut pass = ctx
                .command_encoder()
                .begin_compute_pass(&ComputePassDescriptor {
                    label: Some("ddgi_trace_blend"),
                    ..default()
                });
            let span = diagnostics.pass_span(&mut pass, "ddgi_trace_blend");
            pass.set_pipeline(compute);
            pass.set_bind_group(0, scene_group, &[]);
            pass.set_bind_group(1, &trace_group, &[]);
            pass.set_bind_group(2, surface_group, &[]);
            pass.dispatch_workgroups(cascade.probes.len() as u32, 1, 1);
            span.end(&mut pass);
        }
        // Separate passes avoid aliasing sampled/storage resources and keep
        // recursive bounce reads on a consistent field during each trace batch.
        {
            let mut pass = ctx
                .command_encoder()
                .begin_compute_pass(&ComputePassDescriptor {
                    label: Some("ddgi_apply"),
                    ..default()
                });
            let span = diagnostics.pass_span(&mut pass, "ddgi_apply");
            pass.set_pipeline(apply);
            pass.set_bind_group(0, &apply_group, &[]);
            pass.dispatch_workgroups(cascade.probes.len() as u32, 1, 1);
            span.end(&mut pass);
        }
        recorded = true;
    }
    if let Some(reflect) = cache.get_compute_pipeline(pipeline.reflection_pipeline) {
        stats.1.fetch_add(1, Ordering::Relaxed);
        let cascade = &inputs.cascades[0];
        let mut constants = UniformBuffer::from(Constants {
            center_spacing: cascade.center.extend(cascade.spacing),
            grid_generation: GRID.extend(inputs.generation),
            batch_lamps: UVec4::new(0, inputs.lamps.len() as u32, 0, 0),
            reflection_position: inputs.reflection_position.extend(0.),
            lamp_grid_origin: inputs.lamp_grid.origin.extend(LampGrid::CELL_SIZE),
            lamp_grid_extent: inputs.lamp_grid.dimensions.extend(0),
        });
        constants.write_buffer(&device, &queue);
        let mut probes = StorageBuffer::from(vec![IVec4::ZERO]);
        probes.write_buffer(&device, &queue);
        let group = device.create_bind_group(
            "Local scene reflection inputs",
            &cache.get_bind_group_layout(&pipeline.layout),
            &BindGroupEntries::sequential((
                constants.binding().expect("Reflection constants"),
                &images.get(&cascade.image).expect("GI image").texture_view,
                scratch.as_entire_binding(),
                &pipeline.sampler,
                &sky.texture_view,
                outputs.lamps.binding().expect("Reflection lamps"),
                probes.binding().expect("Probe placeholder"),
                reflection_view,
                outputs.lamp_grid.binding().expect("Reflection lamp grid"),
            )),
        );
        let mut pass = ctx
            .command_encoder()
            .begin_compute_pass(&ComputePassDescriptor {
                label: Some("local_scene_reflections"),
                ..default()
            });
        let span = diagnostics.pass_span(&mut pass, "local_scene_reflections");
        pass.set_pipeline(reflect);
        pass.set_bind_group(0, scene_group, &[]);
        pass.set_bind_group(1, &group, &[]);
        pass.set_bind_group(2, surface_group, &[]);
        pass.dispatch_workgroups(32, 32, 6);
        span.end(&mut pass);
    }
    outputs.generation = inputs.generation;
    if recorded && stats.0.fetch_add(1, Ordering::Relaxed) == 0 {
        info!("Recorded first rolling hardware DDGI probe batch");
    }
}

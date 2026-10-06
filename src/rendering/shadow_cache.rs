//! Retain Bevy-owned shadow maps while their projection, light and casters are unchanged.
use super::opaque::OpaqueMaterial;
use crate::assets::WorldCollider;
use bevy::{
    camera::primitives::{Aabb, CubemapFrusta, Sphere},
    camera::visibility::{
        CubemapVisibleEntities, RenderLayers, SetViewVisibility, VisibilitySystems,
    },
    core_pipeline::schedule::{Core3d, RootNonCameraView},
    ecs::{
        change_detection::Tick,
        schedule::{ScheduleLabel, SingleThreadedExecutor},
    },
    light::{NotShadowCaster, SimulationLightSystems},
    pbr::{
        ExtractedDirectionalLight, ExtractedPointLight, LightEntity, Shadow, ShadowView,
        ViewShadowBindings,
    },
    prelude::*,
    render::{
        Render, RenderApp, RenderStartup, RenderSystems,
        batching::gpu_preprocessing::{GpuPreprocessingMode, GpuPreprocessingSupport},
        extract_resource::{ExtractResource, ExtractResourcePlugin},
        render_phase::{BinnedRenderPhase, ViewBinnedRenderPhases},
        render_resource::{CachedPipelineState, PipelineCache, TextureId},
        sync_world::MainEntity,
        view::{ExtractedView, RetainedViewEntity},
    },
};
use std::{
    collections::{HashMap, HashSet},
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
};

#[derive(Resource, Default, Clone)]
pub(crate) struct ShadowCacheStats {
    rendered: Arc<AtomicU64>,
    reused: Arc<AtomicU64>,
    allocations: Arc<AtomicU64>,
    caster_changes: Arc<AtomicU64>,
    visibility_reused: Arc<AtomicU64>,
    directional_rendered: Arc<AtomicU64>,
    directional_reused: Arc<AtomicU64>,
}

impl ShadowCacheStats {
    pub(crate) fn report(&self) -> serde_json::Value {
        serde_json::json!({
            "rendered_directional_cascades": self.directional_rendered.load(Ordering::Relaxed),
            "reused_directional_cascades": self.directional_reused.load(Ordering::Relaxed),
            "rendered_cube_faces": self.rendered.load(Ordering::Relaxed),
            "reused_cube_faces": self.reused.load(Ordering::Relaxed),
            "texture_allocation_changes": self.allocations.load(Ordering::Relaxed),
            "changed_caster_bounds": self.caster_changes.load(Ordering::Relaxed),
            "reused_light_visibility_checks": self.visibility_reused.load(Ordering::Relaxed),
        })
    }
}

#[derive(Resource)]
struct CacheEnabled(bool);

#[derive(Resource, Default, Clone)]
struct CompletedLights(Arc<Mutex<HashMap<Entity, (u64, u64)>>>);

#[derive(Resource, Default)]
struct SuspendedVisibility(Vec<(Entity, CubemapVisibleEntities, Tick)>);

#[derive(Resource, Default, Clone, ExtractResource)]
struct ShadowRevisions {
    epoch: u64,
    casters: u64,
    lights: HashMap<Entity, u64>,
    faces: HashMap<Entity, [u64; 6]>,
}

#[derive(Clone, Copy)]
struct Bounds {
    center: Vec3,
    radius: f32,
}

#[derive(Resource, Default)]
struct Casters(HashMap<Entity, Bounds>);

#[derive(Clone, PartialEq)]
struct Stamp {
    owner: Entity,
    revision: u64,
    epoch: u64,
    world_from_view: Mat4,
    clip_from_view: Mat4,
    clip_from_world: Option<Mat4>,
    viewport: UVec4,
    range: f32,
    bias: Vec2,
    layers: RenderLayers,
}

#[derive(Resource, Default)]
struct CachedShadows {
    texture: Option<(TextureId, TextureId)>,
    // Cache texture-slot ownership, not just light identities: camera culling
    // can remap a cube-array slot to a different light.
    slots: HashMap<String, Stamp>,
    pending: Vec<(String, Stamp)>,
    suspended_phases: HashMap<RetainedViewEntity, BinnedRenderPhase<Shadow>>,
    suspended_views: HashMap<Entity, ExtractedView>,
}

pub(super) struct ShadowCachePlugin;
impl Plugin for ShadowCachePlugin {
    fn build(&self, app: &mut App) {
        let enabled = !app
            .world()
            .resource::<crate::assets::LaunchOptions>()
            .uncached_shadows;
        let stats = ShadowCacheStats::default();
        let completed = CompletedLights::default();
        app.insert_resource(stats.clone())
            .insert_resource(completed.clone())
            .insert_resource(CacheEnabled(enabled))
            .init_resource::<SuspendedVisibility>();
        app.init_resource::<ShadowRevisions>()
            .init_resource::<Casters>()
            .add_plugins(ExtractResourcePlugin::<ShadowRevisions>::default())
            .add_systems(
                PostUpdate,
                changes
                    .after(SimulationLightSystems::UpdateLightFrusta)
                    .after(bevy::transform::TransformSystems::Propagate)
                    .after(VisibilitySystems::VisibilityPropagate)
                    .after(VisibilitySystems::CalculateBounds)
                    .before(suspend_visibility),
            )
            .add_systems(
                PostUpdate,
                suspend_visibility
                    .after(SimulationLightSystems::UpdateLightFrusta)
                    .before(bevy::light::check_point_light_mesh_visibility),
            )
            .add_systems(
                PostUpdate,
                restore_visibility
                    .after(bevy::light::check_point_light_mesh_visibility)
                    .before(VisibilitySystems::MarkNewlyHiddenEntitiesInvisible),
            )
            .add_systems(Last, asset_changes.after(bevy::asset::AssetEventSystems));
        if let Some(render) = app.get_sub_app_mut(RenderApp) {
            render
                .insert_resource(stats)
                .insert_resource(completed)
                .insert_resource(CacheEnabled(enabled))
                .add_systems(
                    RenderStartup,
                    cpu_preprocessing
                        .after(bevy::render::init_gpu_resource::<GpuPreprocessingSupport>)
                        .before(bevy::pbr::MeshPipelineSystems),
                )
                .init_resource::<CachedShadows>()
                .add_systems(
                    Render,
                    select
                        .after(RenderSystems::CreateViews)
                        .before(RenderSystems::Specialize),
                )
                .add_systems(Render, complete.after(RenderSystems::Render))
                .edit_schedule(Core3d, |schedule| {
                    // Shadow views have very short passes. Repeated worker dispatch
                    // dominated their schedule; other render preparation stays parallel.
                    schedule.set_executor(SingleThreadedExecutor::new());
                });
        }
    }
}

fn cpu_preprocessing(mut support: ResMut<GpuPreprocessingSupport>) {
    // CPU instance buffers require direct CPU phase batches.
    support.max_supported_mode = GpuPreprocessingMode::None;
}

fn changes(
    meshes: Query<
        (
            Entity,
            Ref<GlobalTransform>,
            Ref<Mesh3d>,
            Option<Ref<Aabb>>,
            Ref<InheritedVisibility>,
            Option<Ref<MeshMaterial3d<StandardMaterial>>>,
            Option<Ref<MeshMaterial3d<OpaqueMaterial>>>,
            Option<Ref<RenderLayers>>,
        ),
        (Without<NotShadowCaster>, Without<WorldCollider>),
    >,
    lights: Query<(Entity, &GlobalTransform, &PointLight, &CubemapFrusta)>,
    skinned: Query<(), With<bevy::mesh::skinning::SkinnedMesh>>,
    mut removed_meshes: RemovedComponents<Mesh3d>,
    mut removed_standard: RemovedComponents<MeshMaterial3d<StandardMaterial>>,
    mut removed_surface: RemovedComponents<MeshMaterial3d<OpaqueMaterial>>,
    mut removed_layers: RemovedComponents<RenderLayers>,
    mut removed_bounds: RemovedComponents<Aabb>,
    mut removed_visibility: RemovedComponents<InheritedVisibility>,
    mut removed_exclusions: RemovedComponents<NotShadowCaster>,
    excluded: Query<Entity, Added<NotShadowCaster>>,
    mut casters: ResMut<Casters>,
    mut revisions: ResMut<ShadowRevisions>,
    stats: Res<ShadowCacheStats>,
) {
    let mut dirty = Vec::new();
    // Bone animation can change a silhouette without changing its entity pose.
    // Current actors use rigid meshes; future skinned actors redraw conservatively.
    if !skinned.is_empty() {
        revisions.epoch = revisions.epoch.wrapping_add(1);
    }
    for entity in removed_meshes
        .read()
        .chain(excluded.iter())
        .chain(removed_visibility.read())
    {
        if let Some(bounds) = casters.0.remove(&entity) {
            dirty.push(bounds);
        }
    }
    // An exclusion removal adds a caster back even when its transform is static.
    let removed_exclusions: Vec<_> = removed_exclusions.read().collect();
    let layer_removals: Vec<_> = removed_layers.read().collect();
    if layer_removals.iter().any(|entity| lights.contains(*entity)) {
        revisions.epoch = revisions.epoch.wrapping_add(1);
    }
    let removed_inputs: HashSet<_> = removed_standard
        .read()
        .chain(removed_surface.read())
        .chain(removed_bounds.read())
        .chain(layer_removals)
        .collect();
    for (entity, transform, mesh, aabb, visibility, standard, surface, layers) in &meshes {
        if !transform.is_changed()
            && !mesh.is_changed()
            && !visibility.is_changed()
            && !aabb.as_ref().is_some_and(|a| a.is_changed())
            && !standard
                .as_ref()
                .is_some_and(|material| material.is_changed())
            && !surface
                .as_ref()
                .is_some_and(|material| material.is_changed())
            && !layers.as_ref().is_some_and(|layers| layers.is_changed())
            && !removed_exclusions.contains(&entity)
            && !removed_inputs.contains(&entity)
            && casters.0.contains_key(&entity)
        {
            continue;
        }
        if let Some(old) = casters.0.remove(&entity) {
            dirty.push(old);
        }
        if !visibility.get() {
            continue;
        }
        if let Some(aabb) = aabb {
            let affine = transform.affine();
            let x = affine.matrix3.x_axis * aabb.half_extents.x;
            let y = affine.matrix3.y_axis * aabb.half_extents.y;
            let z = affine.matrix3.z_axis * aabb.half_extents.z;
            let bounds = Bounds {
                center: affine.transform_point3(aabb.center.into()),
                // Opposite corners have equal radius. The four unique lengths
                // bound the transformed box exactly, including hierarchy shear.
                radius: [x + y + z, x + y - z, x - y + z, x - y - z]
                    .into_iter()
                    .map(|corner| corner.length_squared())
                    .fold(0., f32::max)
                    .sqrt(),
            };
            dirty.push(bounds);
            casters.0.insert(entity, bounds);
        } else {
            // Unknown extents must never leave a stale map.
            revisions.epoch = revisions.epoch.wrapping_add(1);
        }
    }
    if !dirty.is_empty() {
        revisions.casters = revisions.casters.wrapping_add(1);
    }
    for (entity, transform, light, frusta) in &lights {
        let revision = revisions.lights.entry(entity).or_default();
        if dirty.iter().any(|bounds| {
            bounds.center.distance_squared(transform.translation())
                <= (bounds.radius + light.range).powi(2)
        }) {
            *revision = revision.wrapping_add(1);
            let faces = revisions.faces.entry(entity).or_default();
            for (face, frustum) in frusta.frusta.iter().enumerate() {
                if dirty.iter().any(|bounds| {
                    frustum.intersects_sphere(
                        &Sphere {
                            center: bounds.center.into(),
                            radius: bounds.radius,
                        },
                        true,
                    )
                }) {
                    faces[face] = faces[face].wrapping_add(1);
                }
            }
        }
    }
    stats
        .caster_changes
        .fetch_add(dirty.len() as u64, Ordering::Relaxed);
    revisions
        .lights
        .retain(|entity, _| lights.contains(*entity));
    revisions.faces.retain(|entity, _| lights.contains(*entity));
}

fn changed_asset_id<A: Asset>(event: &AssetEvent<A>) -> Option<AssetId<A>> {
    match event {
        AssetEvent::Added { id } | AssetEvent::Modified { id } | AssetEvent::Removed { id } => {
            Some(*id)
        }
        AssetEvent::Unused { .. } | AssetEvent::LoadedWithDependencies { .. } => None,
    }
}
fn asset_changes(
    mut mesh_events: MessageReader<AssetEvent<Mesh>>,
    mut material_events: MessageReader<AssetEvent<StandardMaterial>>,
    mut surface_events: MessageReader<AssetEvent<OpaqueMaterial>>,
    mut shader_events: MessageReader<AssetEvent<Shader>>,
    mut image_events: MessageReader<AssetEvent<Image>>,
    server: Res<AssetServer>,
    mut revisions: ResMut<ShadowRevisions>,
    casters: Res<Casters>,
    meshes: Query<(
        &Mesh3d,
        Option<&MeshMaterial3d<StandardMaterial>>,
        Option<&MeshMaterial3d<OpaqueMaterial>>,
    )>,
    lights: Query<(Entity, &GlobalTransform, &PointLight, &CubemapFrusta)>,
) {
    // Asset messages are published in Last, before extraction. A material or
    // newly available mesh affects its users, not every lantern in the map.
    let mut geometry_modified = false;
    let mesh_ids: HashSet<_> = mesh_events
        .read()
        .filter_map(|event| {
            geometry_modified |= matches!(event, AssetEvent::Modified { .. });
            changed_asset_id(event)
        })
        .collect();
    let material_ids: HashSet<_> = material_events
        .read()
        .filter_map(changed_asset_id)
        .collect();
    let surface_ids: HashSet<_> = surface_events.read().filter_map(changed_asset_id).collect();
    let dirty: Vec<_> = if mesh_ids.is_empty() && material_ids.is_empty() && surface_ids.is_empty()
    {
        Vec::new()
    } else {
        casters
            .0
            .iter()
            .filter_map(|(entity, bounds)| {
                let Ok((mesh, standard, surface)) = meshes.get(*entity) else {
                    return None;
                };
                (mesh_ids.contains(&mesh.0.id())
                    || standard.is_some_and(|material| material_ids.contains(&material.0.id()))
                    || surface.is_some_and(|material| surface_ids.contains(&material.0.id())))
                .then_some(*bounds)
            })
            .collect()
    };
    if !dirty.is_empty() {
        revisions.casters = revisions.casters.wrapping_add(1);
        for (entity, pose, light, frusta) in &lights {
            if dirty.iter().any(|bounds| {
                bounds.center.distance_squared(pose.translation())
                    <= (bounds.radius + light.range).powi(2)
            }) {
                let revision = revisions.lights.entry(entity).or_default();
                *revision = revision.wrapping_add(1);
                let faces = revisions.faces.entry(entity).or_default();
                for (face, frustum) in frusta.frusta.iter().enumerate() {
                    if dirty.iter().any(|bounds| {
                        frustum.intersects_sphere(
                            &Sphere {
                                center: bounds.center.into(),
                                radius: bounds.radius,
                            },
                            true,
                        )
                    }) {
                        faces[face] = faces[face].wrapping_add(1);
                    }
                }
            }
        }
    }
    // Modified geometry can grow beyond its previous bounds. Shaders can change
    // silhouettes independently of any material or transform.
    if geometry_modified || shader_events.read().filter_map(changed_asset_id).count() > 0 {
        revisions.epoch = revisions.epoch.wrapping_add(1);
    }
    // Texture edits may change alpha-tested silhouettes. HUD font-atlas edits
    // have no source path and do not invalidate world shadow maps.
    let changed_images = image_events
        .read()
        .filter(|event| {
            let id = match event {
                AssetEvent::Added { id }
                | AssetEvent::Modified { id }
                | AssetEvent::Removed { id }
                | AssetEvent::Unused { id }
                | AssetEvent::LoadedWithDependencies { id } => *id,
            };
            server.get_path(id).is_some()
        })
        .count();
    if changed_images > 0 {
        revisions.epoch = revisions.epoch.wrapping_add(1);
    }
}

fn suspend_visibility(world: &mut World) {
    if !world.resource::<CacheEnabled>().0 {
        return;
    }
    let completed = world.resource::<CompletedLights>().clone();
    let Ok(completed) = completed.0.lock() else {
        return;
    };
    let mut query = world.query::<(
        Entity,
        Ref<PointLight>,
        Ref<GlobalTransform>,
        Ref<CubemapVisibleEntities>,
        Option<Ref<RenderLayers>>,
    )>();
    let revisions = world.resource::<ShadowRevisions>();
    let eligible: Vec<_> = query
        .iter(world)
        .filter_map(|(entity, light, pose, visible, layers)| {
            (!light.is_changed()
                && !pose.is_changed()
                && !layers.as_ref().is_some_and(|layers| layers.is_changed())
                && completed.get(&entity)
                    == Some(&(
                        revisions.epoch,
                        revisions.lights.get(&entity).copied().unwrap_or_default(),
                    )))
            .then_some((entity, visible.last_changed()))
        })
        .collect();
    drop(completed);
    world.resource_scope(|world, mut suspended: Mut<SuspendedVisibility>| {
        for (entity, tick) in eligible {
            if let Some(visible) = world.entity_mut(entity).take::<CubemapVisibleEntities>() {
                // The engine visibility query skips this unchanged light. Its
                // PointLight and shadow-enabled flag are never disabled.
                suspended.0.push((entity, visible, tick));
            }
        }
    });
}

fn restore_visibility(world: &mut World) {
    world.resource_scope(|world, mut suspended: Mut<SuspendedVisibility>| {
        let count = suspended.0.len();
        for (entity, visible, tick) in suspended.0.drain(..) {
            // Shadow casters outside the camera remain visible to Bevy so that
            // allocation changes or later dynamic occluders can redraw them.
            for caster in visible
                .iter()
                .flat_map(|face| face.entities.iter().copied())
            {
                if let Some(mut visibility) = world.get_mut::<ViewVisibility>(caster) {
                    visibility.set_visible();
                }
            }
            if let Ok(mut entity) = world.get_entity_mut(entity) {
                entity.insert(visible);
                if let Some(mut visible) = entity.get_mut::<CubemapVisibleEntities>() {
                    visible.set_last_changed(tick);
                }
            }
        }
        world
            .resource::<ShadowCacheStats>()
            .visibility_reused
            .fetch_add(count as u64, Ordering::Relaxed);
    });
}

fn pipelines_ready(cache: &PipelineCache) -> bool {
    cache
        .pipelines()
        .all(|pipeline| matches!(pipeline.state, CachedPipelineState::Ok(_)))
}

fn select(
    mut commands: Commands,
    views: Query<(Entity, &LightEntity, &ShadowView, Option<&ExtractedView>)>,
    lights: Query<(&MainEntity, &ExtractedPointLight, &RenderLayers)>,
    sun: Query<(&MainEntity, &ExtractedDirectionalLight, &RenderLayers)>,
    bindings: Query<&ViewShadowBindings>,
    revisions: Res<ShadowRevisions>,
    mut cache: ResMut<CachedShadows>,
    enabled: Res<CacheEnabled>,
    stats: Res<ShadowCacheStats>,
    mut phases: ResMut<ViewBinnedRenderPhases<Shadow>>,
) {
    cache.pending.clear();
    // This application has one camera. On allocation change every slot is dirty.
    let Ok(bindings) = bindings.single() else {
        return;
    };
    let texture = (
        bindings.point_light_depth_texture.id(),
        bindings.directional_light_depth_texture.id(),
    );
    if cache.texture != Some(texture) {
        cache.texture = Some(texture);
        cache.slots.clear();
        stats.allocations.fetch_add(1, Ordering::Relaxed);
    }
    for (entity, kind, shadow, live_view) in &views {
        let stored = cache.suspended_views.remove(&entity);
        let Some(view) = live_view.or(stored.as_ref()) else {
            continue;
        };
        let (owner, revision, range, bias, layers, directional) = match kind {
            LightEntity::Point {
                light_entity,
                face_index,
            } => {
                let Ok((owner, light, layers)) = lights.get(*light_entity) else {
                    continue;
                };
                (
                    owner.id(),
                    revisions
                        .faces
                        .get(&owner.id())
                        .map(|faces| faces[*face_index])
                        .unwrap_or_default(),
                    light.range,
                    Vec2::new(light.shadow_depth_bias, light.shadow_normal_bias),
                    layers.clone(),
                    false,
                )
            }
            LightEntity::Directional { light_entity, .. } => {
                let Ok((owner, light, layers)) = sun.get(*light_entity) else {
                    continue;
                };
                (
                    owner.id(),
                    revisions.casters,
                    0.,
                    Vec2::new(light.shadow_depth_bias, light.shadow_normal_bias),
                    layers.clone(),
                    true,
                )
            }
            LightEntity::Spot { .. } => continue,
        };
        let stamp = Stamp {
            owner,
            revision,
            epoch: revisions.epoch,
            world_from_view: view.world_from_view.to_matrix(),
            clip_from_view: view.clip_from_view,
            clip_from_world: view.clip_from_world,
            viewport: view.viewport,
            range,
            bias,
            layers,
        };
        // A completed map stays valid while an unrelated pipeline compiles.
        // Dirty maps become reusable only after completion confirms readiness.
        if enabled.0 && cache.slots.get(&shadow.pass_name) == Some(&stamp) {
            // Bevy still owns lighting, visibility, phase preparation and the map.
            // Only execution of an unchanged cube face is omitted.
            cache.suspended_views.insert(
                entity,
                ExtractedView {
                    retained_view_entity: view.retained_view_entity,
                    clip_from_view: view.clip_from_view,
                    world_from_view: view.world_from_view,
                    clip_from_world: view.clip_from_world,
                    target_format: view.target_format,
                    viewport: view.viewport,
                    color_grading: view.color_grading.clone(),
                    invert_culling: view.invert_culling,
                },
            );
            if live_view.is_some() {
                commands
                    .entity(entity)
                    .remove::<(RootNonCameraView, ExtractedView)>();
            }
            if let Some(phase) = phases.remove(&view.retained_view_entity) {
                cache
                    .suspended_phases
                    .entry(view.retained_view_entity)
                    .or_insert(phase);
            }
            if directional {
                &stats.directional_reused
            } else {
                &stats.reused
            }
            .fetch_add(1, Ordering::Relaxed);
        } else {
            let retained = view.retained_view_entity;
            if let Some(phase) = cache.suspended_phases.remove(&retained) {
                phases.insert(retained, phase);
                phases.prepare_for_new_frame(retained, GpuPreprocessingMode::None);
            }
            if live_view.is_none() {
                if let Some(view) = stored {
                    commands.entity(entity).insert(view);
                }
            }
            commands
                .entity(entity)
                .insert(RootNonCameraView(Core3d.intern()));
            cache.slots.remove(&shadow.pass_name);
            cache.pending.push((shadow.pass_name.clone(), stamp));
            if directional {
                &stats.directional_rendered
            } else {
                &stats.rendered
            }
            .fetch_add(1, Ordering::Relaxed);
        }
    }
    cache
        .suspended_views
        .retain(|entity, _| views.contains(*entity));
    let retained: HashSet<_> = views
        .iter()
        .filter_map(|(entity, _, _, view)| {
            view.or(cache.suspended_views.get(&entity))
                .map(|view| view.retained_view_entity)
        })
        .collect();
    cache
        .suspended_phases
        .retain(|view, _| retained.contains(view));
}

fn complete(
    pipelines: Res<PipelineCache>,
    revisions: Res<ShadowRevisions>,
    mut cache: ResMut<CachedShadows>,
    completed: Res<CompletedLights>,
) {
    if pipelines_ready(&pipelines) {
        let pending = std::mem::take(&mut cache.pending);
        if let Ok(mut completed) = completed.0.lock() {
            for (_, stamp) in &pending {
                completed.insert(
                    stamp.owner,
                    (
                        stamp.epoch,
                        revisions
                            .lights
                            .get(&stamp.owner)
                            .copied()
                            .unwrap_or_default(),
                    ),
                );
            }
        }
        cache.slots.extend(pending);
    }
}

//! Retain Bevy-owned shadow maps while their projection, light and casters are unchanged.
use super::{opaque::OpaqueMaterial, pbr::DiagnosticMaterial};
use crate::assets::WorldCollider;
use bevy::{
    camera::primitives::{Aabb, CubemapFrusta, Sphere},
    camera::visibility::{
        CubemapVisibleEntities, NoCpuCulling, RenderLayers, SetViewVisibility, VisibilitySystems,
    },
    core_pipeline::schedule::{Core3d, RootNonCameraView},
    ecs::{
        change_detection::Tick,
        schedule::{ConditionWithAccess, ScheduleLabel, SingleThreadedExecutor},
    },
    light::{NotShadowCaster, SimulationLightSystems},
    pbr::{
        BinUnpackingBindGroups, ExtractedDirectionalLight, ExtractedPointLight, LightEntity,
        MeshInputUniform, MeshUniform, PendingShadowQueues, Shadow, ShadowView,
        SpecializedShadowMaterialPipelineCache, SpecializedShadowMaterialViewPipelineCache,
        ViewLightEntities, ViewShadowBindings, early_gpu_preprocess, shared_shadow_pass,
    },
    prelude::*,
    render::{
        Render, RenderApp, RenderSystems,
        batching::gpu_preprocessing::{
            BatchedInstanceBuffers, GpuPreprocessingMode, GpuPreprocessingSupport,
            PreprocessWorkItemBuffers, SceneUnpackingBuffers, SceneUnpackingBuffersKey,
        },
        camera::{DirtySpecializations, ViewPendingQueues},
        extract_resource::{ExtractResource, ExtractResourcePlugin},
        occlusion_culling::OcclusionCulling,
        render_phase::{BinnedRenderPhase, ViewBinnedRenderPhases},
        render_resource::{CachedPipelineState, PipelineCache, TextureId},
        renderer::CurrentView,
        sync_world::MainEntity,
        view::{
            ExtractedView, NoIndirectDrawing, RenderExtractedShadowMapVisibleEntities,
            RenderShadowMapVisibleEntities, RetainedViewEntity,
            visibility::collect_visible_cpu_culled_entities,
        },
    },
};
use std::{
    any::TypeId,
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
    render_visibility_reused: Arc<AtomicU64>,
    directional_rendered: Arc<AtomicU64>,
    directional_reused: Arc<AtomicU64>,
    specialization_runs: Arc<AtomicU64>,
    specialization_skipped: Arc<AtomicU64>,
    specialization_protected_views: Arc<AtomicU64>,
    specialization_protected_entries: Arc<AtomicU64>,
    specialization_revived_entries: Arc<AtomicU64>,
    specialization_invalidated_entries: Arc<AtomicU64>,
    specialization_epoch_resets: Arc<AtomicU64>,
    indexed_skin_checks: Arc<AtomicU64>,
    fallback_skin_checks: Arc<AtomicU64>,
    invalid_skin_poses: Arc<AtomicU64>,
    raster_influence_masks: Arc<AtomicU64>,
    refreshed_raster_influence_masks: Arc<AtomicU64>,
    translucent_skin_checks: Arc<AtomicU64>,
    late_indirect_runs: Arc<AtomicU64>,
    late_indirect_skipped: Arc<AtomicU64>,
    late_indirect_fallbacks: Arc<AtomicU64>,
    direct_shadow_phase_resets: Arc<AtomicU64>,
    indirect_stage_runs: [Arc<AtomicU64>; 3],
    indirect_stage_skipped: [Arc<AtomicU64>; 3],
    indirect_stage_fallbacks: [Arc<AtomicU64>; 3],
    removed_directional_roots: Arc<AtomicU64>,
    restored_directional_children: Arc<AtomicU64>,
    direct_schedule_views: Arc<AtomicU64>,
    full_schedule_fallbacks: Arc<AtomicU64>,
    material_coverage_changes: Arc<AtomicU64>,
    material_shading_changes: Arc<AtomicU64>,
    retired_completed_lights: Arc<AtomicU64>,
    native_prepared_views: Arc<AtomicU64>,
    native_retained_views: Arc<AtomicU64>,
    native_attachment_allocations: Arc<AtomicU64>,
    native_attachment_reuses: Arc<AtomicU64>,
}

impl ShadowCacheStats {
    pub(crate) fn report(&self) -> serde_json::Value {
        let mut report = serde_json::json!({
            "rendered_directional_cascades": self.directional_rendered.load(Ordering::Relaxed),
            "direct_shadow_schedule_views": self.direct_schedule_views.load(Ordering::Relaxed),
            "full_shadow_schedule_fallback_views": self.full_schedule_fallbacks.load(Ordering::Relaxed),
            "reused_directional_cascades": self.directional_reused.load(Ordering::Relaxed),
            "rendered_cube_faces": self.rendered.load(Ordering::Relaxed),
            "reused_cube_faces": self.reused.load(Ordering::Relaxed),
            "texture_allocation_changes": self.allocations.load(Ordering::Relaxed),
            "changed_caster_bounds": self.caster_changes.load(Ordering::Relaxed),
            "reused_light_visibility_checks": self.visibility_reused.load(Ordering::Relaxed),
            "reused_render_light_visibility_lists": self.render_visibility_reused.load(Ordering::Relaxed),
            "native_specialization_runs": self.specialization_runs.load(Ordering::Relaxed),
            "native_specialization_skipped_frames": self.specialization_skipped.load(Ordering::Relaxed),
            "protected_specialization_views": self.specialization_protected_views.load(Ordering::Relaxed),
            "protected_specialization_entries": self.specialization_protected_entries.load(Ordering::Relaxed),
            "revived_specialization_entries": self.specialization_revived_entries.load(Ordering::Relaxed),
            "invalidated_specialization_entries": self.specialization_invalidated_entries.load(Ordering::Relaxed),
            "specialization_epoch_resets": self.specialization_epoch_resets.load(Ordering::Relaxed),
            "indexed_skin_pose_checks": self.indexed_skin_checks.load(Ordering::Relaxed),
            "full_joint_fallback_checks": self.fallback_skin_checks.load(Ordering::Relaxed),
            "invalid_skin_pose_checks": self.invalid_skin_poses.load(Ordering::Relaxed),
            "independent_raster_influence_masks": self.raster_influence_masks.load(Ordering::Relaxed),
            "refreshed_raster_influence_masks": self.refreshed_raster_influence_masks.load(Ordering::Relaxed),
            "translucent_indexed_skin_pose_checks": self.translucent_skin_checks.load(Ordering::Relaxed),
            "native_late_indirect_runs": self.late_indirect_runs.load(Ordering::Relaxed),
            "native_late_indirect_skipped_views": self.late_indirect_skipped.load(Ordering::Relaxed),
            "native_late_indirect_conservative_fallbacks": self.late_indirect_fallbacks.load(Ordering::Relaxed),
            "direct_shadow_phase_resets": self.direct_shadow_phase_resets.load(Ordering::Relaxed),
            "native_indirect_clear_runs": null,
            "native_indirect_clear_skipped_views": null,
            "native_indirect_clear_conservative_fallbacks": null,
            "native_indirect_clear_scope": "scene",
            "native_bin_unpack_runs": self.indirect_stage_runs[1].load(Ordering::Relaxed),
            "native_bin_unpack_skipped_views": self.indirect_stage_skipped[1].load(Ordering::Relaxed),
            "native_bin_unpack_conservative_fallbacks": self.indirect_stage_fallbacks[1].load(Ordering::Relaxed),
            "native_early_indirect_runs": self.indirect_stage_runs[2].load(Ordering::Relaxed),
            "native_early_indirect_skipped_views": self.indirect_stage_skipped[2].load(Ordering::Relaxed),
            "native_early_indirect_conservative_fallbacks": self.indirect_stage_fallbacks[2].load(Ordering::Relaxed),
            "removed_duplicate_directional_roots": self.removed_directional_roots.load(Ordering::Relaxed),
            "restored_native_directional_children": self.restored_directional_children.load(Ordering::Relaxed),
        });
        report["material_coverage_changes"] = self
            .material_coverage_changes
            .load(Ordering::Relaxed)
            .into();
        report["material_shading_changes_without_depth_invalidation"] =
            self.material_shading_changes.load(Ordering::Relaxed).into();
        report["retired_completed_light_records"] =
            self.retired_completed_lights.load(Ordering::Relaxed).into();
        report["native_prepared_local_shadow_views"] =
            self.native_prepared_views.load(Ordering::Relaxed).into();
        report["native_retained_local_shadow_views"] =
            self.native_retained_views.load(Ordering::Relaxed).into();
        report["native_shadow_attachment_allocations"] = self
            .native_attachment_allocations
            .load(Ordering::Relaxed)
            .into();
        report["native_shadow_attachment_reuses"] =
            self.native_attachment_reuses.load(Ordering::Relaxed).into();
        report
    }
}

#[derive(Resource)]
struct CacheEnabled(bool);

// Inserted only after the native phase has actually been configured. A
// NoIndirectDrawing tag alone doesn't establish the phase's private mode.
#[derive(Component)]
struct DirectShadowPhase(GpuPreprocessingMode);

/// A native direct shadow phase needs mesh preprocessing and one depth pass,
/// without invoking the full camera graph for every point/spot shadow face.
#[derive(ScheduleLabel, Debug, Clone, PartialEq, Eq, Hash)]
struct DirectShadowRender;

#[derive(Resource, Default, Clone)]
struct CompletedLights(Arc<Mutex<HashMap<Entity, (u64, u64)>>>);

#[derive(Resource, Default)]
struct SuspendedVisibility(Vec<(Entity, CubemapVisibleEntities, Tick)>);

#[derive(Clone, PartialEq)]
struct PointVisibilityStamp {
    owner: Entity,
    epoch: u64,
    revision: u64,
    world_from_light: Mat4,
    range: f32,
    near_z: f32,
    layers: RenderLayers,
    frusta: [[Vec4; 6]; 6],
    preprocessing: GpuPreprocessingMode,
}

#[derive(Resource, Default)]
struct CachedLightVisibility {
    stamps: HashMap<Entity, PointVisibilityStamp>,
    suspended: Vec<(Entity, RenderExtractedShadowMapVisibleEntities, Tick)>,
}

#[derive(Resource, Default, Clone, ExtractResource)]
#[extract_app(bevy::render::RenderApp)]
struct ShadowRevisions {
    epoch: u64,
    casters: u64,
    lights: HashMap<Entity, u64>,
    faces: HashMap<Entity, [u64; 6]>,
}

/// Inputs consumed by Bevy's native shadow prepass, rather than the forward
/// surface shader. Premultiplied alpha tests all four color components.
#[derive(PartialEq)]
struct MaterialCoverage {
    alpha_mode: AlphaMode,
    color: Vec4,
    texture: Option<AssetId<Image>>,
    uv_transform: bevy::math::Affine2,
    channel: bevy::mesh::UvChannel,
    cull_mode: Option<bevy::render::render_resource::Face>,
    depth_bias: i32,
}

impl MaterialCoverage {
    fn new(material: &StandardMaterial, extended: bool) -> Self {
        let discard = !matches!(material.alpha_mode, AlphaMode::Opaque | AlphaMode::Multiply);
        let texture = discard
            .then(|| material.base_color_texture.as_ref().map(Handle::id))
            .flatten();
        Self {
            alpha_mode: material.alpha_mode,
            color: if !discard {
                Vec4::ONE
            } else if matches!(material.alpha_mode, AlphaMode::Premultiplied) {
                material.base_color.to_linear().to_vec4()
            } else {
                Vec4::new(1.0, 1.0, 1.0, material.base_color.alpha())
            },
            uv_transform: if texture.is_some() {
                material.uv_transform
            } else {
                bevy::math::Affine2::IDENTITY
            },
            channel: if texture.is_some() {
                material.base_color_channel.clone()
            } else {
                bevy::mesh::UvChannel::Uv0
            },
            texture,
            // OpaqueSurface deliberately overrides native shadow culling. Its
            // layer/response textures and forward vertex shader are not used.
            cull_mode: if extended { None } else { material.cull_mode },
            depth_bias: material.depth_bias as i32,
        }
    }
}

#[derive(Resource, Default)]
struct ShadowMaterialCoverage {
    standard: HashMap<AssetId<StandardMaterial>, MaterialCoverage>,
    extended: HashMap<AssetId<OpaqueMaterial>, MaterialCoverage>,
    diagnostic: HashMap<AssetId<DiagnosticMaterial>, MaterialCoverage>,
}

fn changed_material_coverage<'a, A: Asset>(
    events: impl Iterator<Item = &'a AssetEvent<A>>,
    mut current: impl FnMut(AssetId<A>) -> Option<MaterialCoverage>,
    retained: &mut HashMap<AssetId<A>, MaterialCoverage>,
    stats: &ShadowCacheStats,
) -> HashSet<AssetId<A>> {
    events
        .filter_map(|event| {
            let id = changed_asset_id(event)?;
            let Some(coverage) =
                current(id).filter(|_| !matches!(event, AssetEvent::Removed { .. }))
            else {
                retained.remove(&id);
                return Some(id);
            };
            let unchanged = retained.get(&id) == Some(&coverage);
            retained.insert(id, coverage);
            if unchanged {
                stats
                    .material_shading_changes
                    .fetch_add(1, Ordering::Relaxed);
                None
            } else {
                stats
                    .material_coverage_changes
                    .fetch_add(1, Ordering::Relaxed);
                Some(id)
            }
        })
        .collect()
}

#[derive(Clone, Copy)]
struct Bounds {
    center: Vec3,
    radius: f32,
}

#[derive(Resource, Default)]
struct Casters(
    HashMap<Entity, Bounds>,
    HashMap<Entity, (Mat4, Option<(Vec3, Vec3)>)>,
    HashMap<Entity, Vec<Mat4>>,
    // Bind sources are immutable. Raster asset edits require refreshed indexed
    // influences; missing or invalid current data uses full-joint fallback.
    HashMap<AssetId<Mesh>, Option<Vec<usize>>>,
);

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
    preprocessing: GpuPreprocessingMode,
}

#[derive(Resource, Default)]
struct CachedShadows {
    texture: Option<(TextureId, TextureId)>,
    // Cache texture-slot ownership, not just light identities: camera culling
    // can remap a cube-array slot to a different light.
    slots: HashMap<(String, Entity), Stamp>,
    pending: Vec<((String, Entity), Stamp)>,
    suspended_phases: HashMap<RetainedViewEntity, BinnedRenderPhase<Shadow>>,
    suspended_views: HashMap<Entity, ExtractedView>,
    // The native phase or its suspended owned value carries this mode. Native
    // empty phases created while that value is parked are placeholders only.
    phase_modes: HashMap<RetainedViewEntity, GpuPreprocessingMode>,
    specialization_epoch: Option<u64>,
    // Native specialization expires caches for views with no ExtractedView.
    // Park its owned values only across that system, then return them to Bevy.
    parked_specializations: HashMap<RetainedViewEntity, ParkedSpecialization>,
}

struct ParkedSpecialization {
    pipelines: SpecializedShadowMaterialViewPipelineCache,
    pending: ViewPendingQueues,
}

pub(super) struct ShadowCachePlugin;
impl Plugin for ShadowCachePlugin {
    fn build(&self, app: &mut App) {
        let record_visibility = cfg!(debug_assertions)
            && app
                .world()
                .resource::<crate::assets::LaunchOptions>()
                .verify
                .is_some();
        app.init_resource::<bevy::light::ShadowVisibilityWorkCounts>();
        app.world_mut()
            .resource_mut::<bevy::light::ShadowVisibilityWorkCounts>()
            .enabled = record_visibility;
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
            .init_resource::<ShadowMaterialCoverage>()
            .add_plugins(ExtractResourcePlugin::<ShadowRevisions>::default())
            .add_systems(
                PostUpdate,
                changes
                    .after(crate::enemies::EnemyBounds)
                    .after(SimulationLightSystems::UpdateLightFrusta)
                    .after(bevy::transform::TransformSystems::Propagate)
                    .after(VisibilitySystems::VisibilityPropagate)
                    .after(VisibilitySystems::CalculateBounds)
                    .before(suspend_visibility),
            )
            .add_systems(
                PostUpdate,
                culling_changes.after(changes).before(suspend_visibility),
            )
            .add_systems(PostUpdate, retire_completed_lights.after(changes))
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
                .init_resource::<CachedShadows>()
                .init_resource::<CachedLightVisibility>()
                .add_systems(
                    Render,
                    suspend_render_visibility
                        .after(RenderSystems::ExtractCommands)
                        .before(collect_visible_cpu_culled_entities),
                )
                .add_systems(
                    Render,
                    restore_render_visibility
                        .after(collect_visible_cpu_culled_entities)
                        .before(RenderSystems::PrepareMeshes),
                )
                .add_systems(
                    Render,
                    select
                        .after(RenderSystems::CreateViews)
                        .before(RenderSystems::Specialize),
                )
                .add_systems(
                    Render,
                    restore_specializations
                        .after(RenderSystems::Specialize)
                        .before(RenderSystems::QueueMeshes),
                )
                .add_systems(Render, complete.after(RenderSystems::Render))
                .add_systems(
                    Render,
                    route_direct_shadow_roots
                        .after(RenderSystems::Prepare)
                        .before(RenderSystems::Render),
                )
                .edit_schedule(Core3d, |schedule| {
                    // Shadow views have very short passes. Repeated worker dispatch
                    // dominated their schedule; other render preparation stays parallel.
                    schedule.set_executor(SingleThreadedExecutor::new());
                });
        }
    }

    fn finish(&self, app: &mut App) {
        let Some(render) = app.get_sub_app_mut(RenderApp) else {
            return;
        };
        // This exclusive native system has no public function path. Pin the
        // locked engine's diagnostic name, as the Solari integration does.
        // PBR has finished installing it. Attach before the later Solari
        // finalizer initializes Render and pipelined cleanup takes RenderApp.
        render.edit_schedule(Render, |schedule| {
            let keys: Vec<_> = schedule
                .graph()
                .systems
                .iter()
                .filter_map(|(key, system, _)| {
                    format!("{}", system.name())
                        .contains("bevy_pbr::render::light::specialize_shadows")
                        .then_some(key)
                })
                .collect();
            assert_eq!(keys.len(), 1, "Pinned shadow specialization contract");
            schedule
                .graph_mut()
                .systems
                .get_conditions_mut(keys[0])
                .expect("Native shadow conditions")
                .push(ConditionWithAccess::new(Box::new(IntoSystem::into_system(
                    specialization_due,
                ))));
        });
        let gpu_meshes = matches!(
            render
                .world()
                .get_resource::<bevy::pbr::RenderMeshInstances>(),
            Some(bevy::pbr::RenderMeshInstances::GpuBuilding(_))
        );
        let mut direct_schedule = Schedule::new(DirectShadowRender);
        direct_schedule.set_executor(SingleThreadedExecutor::new());
        if gpu_meshes {
            // Direct batches still require current native MeshUniforms. Their
            // CPU work lists do not use bin unpacking or uniform allocation.
            direct_schedule
                .add_systems((early_gpu_preprocess, shared_shadow_pass::<false>).chain());
        } else {
            direct_schedule.add_systems(shared_shadow_pass::<false>);
        }
        render.add_schedule(direct_schedule);
        render.edit_schedule(Core3d, |schedule| {
            let keys: Vec<_> = schedule
                .graph()
                .systems
                .iter()
                .filter_map(|(key, system, _)| {
                    format!("{}", system.name())
                        .contains("bevy_pbr::render::gpu_preprocess::late_prepass_build_indirect_parameters")
                        .then_some(key)
                })
                .collect();
            // Native GPU preprocessing is absent on the explicit CPU path
            // and devices without compute support. Pin the installed system.
            assert_eq!(
                keys.len(),
                usize::from(gpu_meshes),
                "Pinned late indirect preprocessing contract"
            );
            if let Some(&key) = keys.first() {
                schedule
                    .graph_mut()
                    .systems
                    .get_conditions_mut(key)
                    .expect("Native late indirect conditions")
                    .push(ConditionWithAccess::new(Box::new(IntoSystem::into_system(
                        late_indirect_due,
                    ))));
            }
            // The RC clears scene unpacking metadata once in Render, rather
            // than clearing indirect metadata for every Core3d view.
            for (stage, name) in [
                (1, "unpack_bins"),
                (2, "early_prepass_build_indirect_parameters"),
            ] {
                let native_name = format!("bevy_pbr::render::gpu_preprocess::{name}");
                let keys: Vec<_> = schedule
                    .graph()
                    .systems
                    .iter()
                    .filter_map(|(key, system, _)| {
                        format!("{}", system.name())
                            .contains(&native_name)
                            .then_some(key)
                    })
                    .collect();
                assert_eq!(
                    keys.len(),
                    usize::from(gpu_meshes),
                    "Pinned native indirect stage contract: {name}"
                );
                if let Some(&key) = keys.first() {
                    let condition = match stage {
                        1 => ConditionWithAccess::new(Box::new(IntoSystem::into_system(
                            indirect_stage_due::<1>,
                        ))),
                        _ => ConditionWithAccess::new(Box::new(IntoSystem::into_system(
                            indirect_stage_due::<2>,
                        ))),
                    };
                    schedule
                        .graph_mut()
                        .systems
                        .get_conditions_mut(key)
                        .expect("Native indirect stage conditions")
                        .push(condition);
                }
            }
        });
    }
}

fn route_direct_shadow_roots(
    mut views: Query<(
        &LightEntity,
        &ExtractedView,
        Option<&DirectShadowPhase>,
        Has<NoIndirectDrawing>,
        Has<OcclusionCulling>,
        &mut RootNonCameraView,
    )>,
    buffers: Option<Res<BatchedInstanceBuffers<MeshUniform, MeshInputUniform>>>,
    unpacking: Option<Res<SceneUnpackingBuffers>>,
    stats: Res<ShadowCacheStats>,
) {
    let shadow_type = TypeId::of::<Shadow>();
    for (kind, view, phase, direct, occlusion, mut root) in &mut views {
        if !matches!(kind, LightEntity::Point { .. } | LightEntity::Spot { .. }) {
            continue;
        }
        let direct_work = buffers.as_ref().is_none_or(|buffers| {
            buffers
                .phase_instance_buffers
                .get(&shadow_type)
                .is_none_or(|phase| {
                    phase
                        .work_item_buffers
                        .get(&view.retained_view_entity)
                        .is_none_or(|work| matches!(work, PreprocessWorkItemBuffers::Direct(_)))
                })
        });
        let no_unpack_jobs = unpacking.as_ref().is_none_or(|unpacking| {
            unpacking
                .view_phase_buffers
                .get(&SceneUnpackingBuffersKey {
                    phase: shadow_type,
                    view: view.retained_view_entity,
                })
                .is_none_or(|jobs| {
                    jobs.indexed_unpacking_jobs.is_empty()
                        && jobs.non_indexed_unpacking_jobs.is_empty()
                })
        });
        let proven_direct = direct
            && !occlusion
            && phase.is_some_and(|phase| phase.0 != GpuPreprocessingMode::Culling)
            && direct_work
            && no_unpack_jobs;
        if proven_direct {
            root.0 = DirectShadowRender.intern();
            stats.direct_schedule_views.fetch_add(1, Ordering::Relaxed);
        } else {
            // New modes or unsupported native work stay on the complete graph.
            root.0 = Core3d.intern();
            stats
                .full_schedule_fallbacks
                .fetch_add(1, Ordering::Relaxed);
        }
    }
}

fn indirect_stage_due<const STAGE: usize>(
    current: Option<Res<CurrentView>>,
    views: Query<
        (
            Has<NoIndirectDrawing>,
            Option<&DirectShadowPhase>,
            Option<&ViewLightEntities>,
        ),
        With<ExtractedView>,
    >,
    lights: Query<&LightEntity>,
    stats: Res<ShadowCacheStats>,
) -> bool {
    let _span = info_span!("shadow_indirect_stage_due", stage = STAGE).entered();
    let mut fallback = false;
    let proven_direct = |direct: bool, phase: Option<&DirectShadowPhase>| {
        direct && phase.is_some_and(|phase| phase.0 != GpuPreprocessingMode::Culling)
    };
    let due = if let Some(current) = current
        && let Ok((direct, phase, children)) = views.get(current.0)
    {
        // These native passes operate on global indirect metadata. Direct
        // point/spot phases consume none of it, but still need their separate
        // early_gpu_preprocess pass to expand current mesh uniforms.
        !proven_direct(direct, phase)
            || children.is_some_and(|children| {
                children.lights.iter().any(|child| {
                    match lights.get(*child) {
                        Ok(LightEntity::Directional { .. }) => {}
                        Ok(_) => return false,
                        Err(_) => {
                            fallback = true;
                            return true;
                        }
                    }
                    match views.get(*child) {
                        Ok((direct, phase, _)) => !proven_direct(direct, phase),
                        Err(_) => {
                            fallback = true;
                            true
                        }
                    }
                })
            })
    } else {
        fallback = true;
        true
    };
    if fallback {
        stats.indirect_stage_fallbacks[STAGE].fetch_add(1, Ordering::Relaxed);
    }
    if due {
        &stats.indirect_stage_runs[STAGE]
    } else {
        &stats.indirect_stage_skipped[STAGE]
    }
    .fetch_add(1, Ordering::Relaxed);
    due
}

fn late_indirect_due(
    current: Option<Res<CurrentView>>,
    views: Query<
        (
            Has<OcclusionCulling>,
            Has<NoIndirectDrawing>,
            Has<bevy::pbr::SkipGpuPreprocess>,
            Option<&ViewLightEntities>,
        ),
        With<ExtractedView>,
    >,
    lights: Query<&LightEntity>,
    stats: Res<ShadowCacheStats>,
) -> bool {
    let _span = info_span!("shadow_late_indirect_due").entered();
    let mut fallback = false;
    let due = if let Some(current) = current
        && let Ok((occlusion, direct, skip, children)) = views.get(current.0)
    {
        // Native late preprocessing and shadow drawing both skip ordinary
        // point faces. Its late parameter builder has only a global query
        // condition, so an unrelated occlusion camera otherwise enables it.
        (occlusion && !direct && !skip)
            || children.is_some_and(|children| {
                children.lights.iter().any(|child| {
                    // Match native gather_shadow_cascades_for_view: point and
                    // spot faces execute their own Core3d schedules.
                    match lights.get(*child) {
                        Ok(LightEntity::Directional { .. }) => {}
                        Ok(_) => return false,
                        Err(_) => {
                            fallback = true;
                            return true;
                        }
                    }
                    match views.get(*child) {
                        Ok((occlusion, direct, skip, _)) => occlusion && !direct && !skip,
                        Err(_) => {
                            fallback = true;
                            true
                        }
                    }
                })
            })
    } else {
        // Missing current-view contract data must retain native work.
        fallback = true;
        true
    };
    if fallback {
        stats
            .late_indirect_fallbacks
            .fetch_add(1, Ordering::Relaxed);
    }
    if due {
        &stats.late_indirect_runs
    } else {
        &stats.late_indirect_skipped
    }
    .fetch_add(1, Ordering::Relaxed);
    due
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
    skinned: Query<
        (
            Entity,
            Ref<bevy::mesh::skinning::SkinnedMesh>,
            &Mesh3d,
            Option<&MeshMaterial3d<StandardMaterial>>,
            Option<&MeshMaterial3d<OpaqueMaterial>>,
        ),
        (
            With<bevy::mesh::skinning::SkinnedMesh>,
            Without<NotShadowCaster>,
        ),
    >,
    joints: Query<&GlobalTransform>,
    skin_assets: (
        Res<Assets<Mesh>>,
        Res<Assets<StandardMaterial>>,
        Res<Assets<OpaqueMaterial>>,
    ),
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
    // A static skin must not invalidate every lantern merely by existing.
    // Changed joint poses invalidate the affected caster's nearby shadow maps.
    let (mesh_assets, standard_assets, surface_assets) = skin_assets;
    for (entity, skin, mesh, standard, surface) in &skinned {
        let Casters(_, _, skin_poses, revised_influences) = &mut *casters;
        // A material primitive can index only the housing of a shared rig.
        // Match the original raster asset, never its deformed ray provider.
        if let std::collections::hash_map::Entry::Vacant(entry) =
            revised_influences.entry(mesh.0.id())
            && let Some(mesh) = mesh_assets.get(&mesh.0)
        {
            let used_joints = super::ray_skin::Source::indexed_joints(mesh);
            if used_joints.is_some() {
                stats.raster_influence_masks.fetch_add(1, Ordering::Relaxed);
            }
            // Retry absent/loading assets next frame. Invalid native data
            // stays conservative until an actual asset edit fixes it.
            entry.insert(used_joints);
        }
        let used_joints = revised_influences
            .get(&mesh.0.id())
            .and_then(Option::as_deref);
        if used_joints.is_some() {
            &stats.indexed_skin_checks
        } else {
            &stats.fallback_skin_checks
        }
        .fetch_add(1, Ordering::Relaxed);
        if used_joints.is_some()
            && standard
                .and_then(|material| standard_assets.get(&material.0))
                .map(|material| material.alpha_mode)
                .or_else(|| {
                    surface
                        .and_then(|material| surface_assets.get(&material.0))
                        .map(|material| material.base.alpha_mode)
                })
                .is_some_and(|mode| {
                    matches!(
                        mode,
                        AlphaMode::Blend
                            | AlphaMode::Premultiplied
                            | AlphaMode::Add
                            | AlphaMode::Multiply
                    )
                })
        {
            stats
                .translucent_skin_checks
                .fetch_add(1, Ordering::Relaxed);
        }
        let count = used_joints.map_or(skin.joints.len(), <[usize]>::len);
        let poses = skin_poses.entry(entity).or_default();
        let mut changed = skin.is_changed() || poses.len() != count;
        let mut valid = count > 0;
        for position in 0..count {
            let index = used_joints.map_or(position, |joints| joints[position]);
            let Some(joint) = skin.joints.get(index) else {
                valid = false;
                break;
            };
            let Ok(transform) = joints.get(*joint) else {
                valid = false;
                break;
            };
            let matrix = transform.to_matrix();
            if !matrix.is_finite() {
                valid = false;
                break;
            }
            if let Some(previous) = poses.get_mut(position) {
                changed |= *previous != matrix;
                *previous = matrix;
            } else {
                poses.push(matrix);
            }
        }
        if valid {
            poses.truncate(count);
        } else {
            // Missing poses are not proof of an unchanged surface. Clear the
            // partial snapshot and keep invalidating until it is complete.
            poses.clear();
            changed = true;
            stats.invalid_skin_poses.fetch_add(1, Ordering::Relaxed);
        }
        if changed && let Some(bounds) = casters.0.get(&entity) {
            dirty.push(*bounds);
        }
    }
    for entity in removed_meshes
        .read()
        .chain(excluded.iter())
        .chain(removed_visibility.read())
    {
        if let Some(bounds) = casters.0.remove(&entity) {
            dirty.push(bounds);
        }
        casters.1.remove(&entity);
        casters.2.remove(&entity);
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
        let pose = (
            transform.to_matrix(),
            aabb.as_ref()
                .map(|aabb| (aabb.center.into(), aabb.half_extents.into())),
        );
        let changed_pose = casters.1.get(&entity) != Some(&pose);
        casters.1.insert(entity, pose);
        if !changed_pose
            && !mesh.is_changed()
            && !visibility.is_changed()
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

fn retire_completed_lights(
    lights: Query<Entity, Or<(With<PointLight>, With<SpotLight>, With<DirectionalLight>)>>,
    completed: Res<CompletedLights>,
    stats: Res<ShadowCacheStats>,
) {
    // Completion is shared with the render world. Retire despawned owners even
    // while pipeline compilation prevents that world from publishing new maps.
    if let Ok(mut completed) = completed.0.lock() {
        let previous = completed.len();
        completed.retain(|entity, _| lights.contains(*entity));
        if completed.len() != previous {
            stats
                .retired_completed_lights
                .fetch_add((previous - completed.len()) as u64, Ordering::Relaxed);
        }
    }
}
fn asset_changes(
    mut mesh_events: MessageReader<AssetEvent<Mesh>>,
    mut material_events: MessageReader<AssetEvent<StandardMaterial>>,
    mut surface_events: MessageReader<AssetEvent<OpaqueMaterial>>,
    mut diagnostic_events: MessageReader<AssetEvent<DiagnosticMaterial>>,
    mut shader_events: MessageReader<AssetEvent<Shader>>,
    mut image_events: MessageReader<AssetEvent<Image>>,
    mut inverse_bind_events: MessageReader<
        AssetEvent<bevy::mesh::skinning::SkinnedMeshInverseBindposes>,
    >,
    material_assets: (
        Res<Assets<StandardMaterial>>,
        Res<Assets<OpaqueMaterial>>,
        Res<Assets<DiagnosticMaterial>>,
    ),
    mesh_assets: Res<Assets<Mesh>>,
    mut revisions: ResMut<ShadowRevisions>,
    mut casters: ResMut<Casters>,
    mut coverage: ResMut<ShadowMaterialCoverage>,
    meshes: Query<(
        &Mesh3d,
        Option<&MeshMaterial3d<StandardMaterial>>,
        Option<&MeshMaterial3d<OpaqueMaterial>>,
        Option<&MeshMaterial3d<DiagnosticMaterial>>,
    )>,
    lights: Query<(Entity, &GlobalTransform, &PointLight, &CubemapFrusta)>,
    stats: Res<ShadowCacheStats>,
) {
    let (materials, surfaces, diagnostics) = material_assets;
    // Asset messages are published in PostUpdate, before extraction. A material or
    // newly available mesh affects its users, not every lantern in the map.
    if inverse_bind_events.read().any(|event| {
        matches!(
            event,
            AssetEvent::Modified { .. } | AssetEvent::Removed { .. }
        )
    }) {
        // Joint transforms alone cannot reveal edited inverse bind matrices.
        // Invalidate immediately and rebuild the pose snapshots next frame.
        revisions.epoch = revisions.epoch.wrapping_add(1);
        casters.2.clear();
    }
    let mut modified_mesh_ids = HashSet::new();
    let mesh_ids: HashSet<_> = mesh_events
        .read()
        .filter_map(|event| {
            match event {
                AssetEvent::Modified { id } => {
                    modified_mesh_ids.insert(*id);
                    if casters.3.contains_key(id) {
                        // Surface preparation can edit attributes before the
                        // bind source is admitted in this same frame. Read the
                        // current raster asset instead of retiring that valid
                        // source, or trusting old weights after later edits.
                        let used_joints = mesh_assets
                            .get(*id)
                            .and_then(super::ray_skin::Source::indexed_joints);
                        if used_joints.is_some() {
                            stats
                                .refreshed_raster_influence_masks
                                .fetch_add(1, Ordering::Relaxed);
                        }
                        casters.3.insert(*id, used_joints);
                    }
                }
                AssetEvent::Removed { id } => {
                    if casters.3.contains_key(id) {
                        casters.3.insert(*id, None);
                    }
                }
                AssetEvent::Added { id } => {
                    if casters.3.contains_key(id) {
                        let used_joints = mesh_assets
                            .get(*id)
                            .and_then(super::ray_skin::Source::indexed_joints);
                        if used_joints.is_some() {
                            stats
                                .refreshed_raster_influence_masks
                                .fetch_add(1, Ordering::Relaxed);
                        }
                        casters.3.insert(*id, used_joints);
                    }
                }
                _ => {}
            }
            changed_asset_id(event)
        })
        .collect();
    let material_ids = changed_material_coverage(
        material_events.read(),
        |id| {
            materials
                .get(id)
                .map(|material| MaterialCoverage::new(material, false))
        },
        &mut coverage.standard,
        &stats,
    );
    let surface_ids = changed_material_coverage(
        surface_events.read(),
        |id| {
            surfaces
                .get(id)
                .map(|material| MaterialCoverage::new(&material.base, true))
        },
        &mut coverage.extended,
        &stats,
    );
    let diagnostic_ids = changed_material_coverage(
        diagnostic_events.read(),
        |id| {
            diagnostics
                .get(id)
                .map(|material| MaterialCoverage::new(&material.base, false))
        },
        &mut coverage.diagnostic,
        &stats,
    );
    let image_ids: HashSet<_> = image_events.read().filter_map(changed_asset_id).collect();
    let mut geometry_modified = false;
    let dirty: Vec<_> = if mesh_ids.is_empty()
        && material_ids.is_empty()
        && surface_ids.is_empty()
        && diagnostic_ids.is_empty()
        && image_ids.is_empty()
    {
        Vec::new()
    } else {
        casters
            .0
            .iter()
            .filter_map(|(entity, bounds)| {
                let Ok((mesh, standard, surface, diagnostic)) = meshes.get(*entity) else {
                    return None;
                };
                // Ray-only pose providers have no raster caster and cannot
                // invalidate any raster shadow. Growing raster meshes still
                // require conservative invalidation beyond their old bounds.
                geometry_modified |= modified_mesh_ids.contains(&mesh.0.id());
                let coverage_changed = standard
                    .and_then(|handle| materials.get(&handle.0))
                    .is_some_and(|material| {
                        !matches!(material.alpha_mode, AlphaMode::Opaque | AlphaMode::Multiply)
                            && material
                                .base_color_texture
                                .as_ref()
                                .is_some_and(|image| image_ids.contains(&image.id()))
                    })
                    || surface
                        .and_then(|handle| surfaces.get(&handle.0))
                        .is_some_and(|surface| {
                            !matches!(
                                surface.base.alpha_mode,
                                AlphaMode::Opaque | AlphaMode::Multiply
                            ) && surface
                                .base
                                .base_color_texture
                                .as_ref()
                                .is_some_and(|image| image_ids.contains(&image.id()))
                        })
                    || diagnostic
                        .and_then(|handle| diagnostics.get(&handle.0))
                        .is_some_and(|material| {
                            !matches!(
                                material.base.alpha_mode,
                                AlphaMode::Opaque | AlphaMode::Multiply
                            ) && material
                                .base
                                .base_color_texture
                                .as_ref()
                                .is_some_and(|image| image_ids.contains(&image.id()))
                        });
                (mesh_ids.contains(&mesh.0.id())
                    || standard.is_some_and(|material| material_ids.contains(&material.0.id()))
                    || surface.is_some_and(|material| surface_ids.contains(&material.0.id()))
                    || diagnostic.is_some_and(|material| diagnostic_ids.contains(&material.0.id()))
                    || coverage_changed)
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
    // Normal/response mip promotions and HUD glyphs cannot change silhouettes.
    // Coverage edits invalidate their actual users above, using the same light
    // intersection path as material edits rather than a map-wide epoch.
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

fn culling_changes(
    changed: Query<
        Entity,
        (
            Changed<NoCpuCulling>,
            With<Mesh3d>,
            Without<NotShadowCaster>,
        ),
    >,
    mut removed: RemovedComponents<NoCpuCulling>,
    casters: Res<Casters>,
    mut revisions: ResMut<ShadowRevisions>,
) {
    let removed_caster = removed.read().any(|entity| casters.0.contains_key(&entity));
    if removed_caster || !changed.is_empty() {
        // Changing CPU/GPU classification changes the native visibility tables
        // even when a caster's world-space silhouette is unchanged.
        revisions.epoch = revisions.epoch.wrapping_add(1);
    }
}

fn suspend_render_visibility(world: &mut World) {
    if !world.resource::<CacheEnabled>().0 {
        return;
    }
    let completed = world.resource::<CompletedLights>().clone();
    let Ok(completed) = completed.0.lock() else {
        return;
    };
    let mut query = world.query::<(
        Entity,
        &MainEntity,
        Option<&ExtractedPointLight>,
        Has<ExtractedDirectionalLight>,
        &RenderLayers,
        &CubemapFrusta,
        Ref<RenderExtractedShadowMapVisibleEntities>,
        &RenderShadowMapVisibleEntities,
    )>();
    world.resource_scope(|world, mut cache: Mut<CachedLightVisibility>| {
        let revisions = world.resource::<ShadowRevisions>();
        let preprocessing = world
            .resource::<GpuPreprocessingSupport>()
            .max_supported_mode;
        let mut live = HashSet::new();
        let mut eligible = Vec::new();
        for (entity, owner, light, directional, layers, frusta, extracted, retained) in
            query.iter(world)
        {
            // The main-world point-light inventory also excludes old cube
            // components left behind if an entity changes its light kind.
            if directional || !revisions.lights.contains_key(&owner.id()) {
                continue;
            }
            let complete_lists = retained.subviews.len() == 6
                && extracted.subviews.len() == 6
                && retained.subviews.iter().all(|(view, entities)| {
                    view.main_entity == *owner
                        && view.subview_index < 6
                        && entities.1.get::<Mesh3d>().is_some()
                        && extracted.subviews.contains_key(view)
                });
            let Some(light) = light else {
                // Bevy removes ExtractedPointLight outside the camera, but
                // keeps both visibility tables. Those dormant tables have no
                // shadow view and need no sorting/diffing. Re-admission must
                // always collect current native lists before using a shadow.
                cache.stamps.remove(&entity);
                if complete_lists {
                    eligible.push((entity, extracted.last_changed()));
                }
                continue;
            };
            // Spot lights share ExtractedPointLight, but their visibility is not
            // a six-face, camera-independent point-light frustum.
            if !light.shadow_maps_enabled || light.spot_light_angles.is_some() {
                continue;
            }
            live.insert(entity);
            let stamp = PointVisibilityStamp {
                owner: owner.id(),
                epoch: revisions.epoch,
                revision: revisions
                    .lights
                    .get(&owner.id())
                    .copied()
                    .unwrap_or_default(),
                world_from_light: light.transform.to_matrix(),
                range: light.range,
                near_z: light.shadow_map_near_z,
                layers: layers.clone(),
                frusta: std::array::from_fn(|face| {
                    std::array::from_fn(|plane| {
                        frusta.frusta[face].half_spaces[plane].normal_d().into()
                    })
                }),
                preprocessing,
            };
            let reusable = complete_lists
                && completed.get(&stamp.owner) == Some(&(stamp.epoch, stamp.revision))
                && cache.stamps.get(&entity) == Some(&stamp);
            cache.stamps.insert(entity, stamp);
            if reusable {
                eligible.push((entity, extracted.last_changed()));
            }
        }
        cache.stamps.retain(|entity, _| live.contains(entity));
        for (entity, tick) in eligible {
            if let Some(extracted) = world
                .entity_mut(entity)
                .take::<RenderExtractedShadowMapVisibleEntities>()
            {
                // Bevy preserves the retained lists when this optional input is
                // absent, while still clearing its per-frame added/removed lists.
                // Cameras, sun cascades, dirty lights and GPU-culled entities
                // continue through their normal native update paths.
                cache.suspended.push((entity, extracted, tick));
            }
        }
    });
}

fn restore_render_visibility(world: &mut World) {
    world.resource_scope(|world, mut cache: Mut<CachedLightVisibility>| {
        let count = cache.suspended.len();
        for (entity, extracted, tick) in cache.suspended.drain(..) {
            if let Ok(mut entity) = world.get_entity_mut(entity) {
                entity.insert(extracted);
                if let Some(mut extracted) =
                    entity.get_mut::<RenderExtractedShadowMapVisibleEntities>()
                {
                    extracted.set_last_changed(tick);
                }
            }
        }
        world
            .resource::<ShadowCacheStats>()
            .render_visibility_reused
            .fetch_add(count as u64, Ordering::Relaxed);
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
    views: Query<(
        Entity,
        &LightEntity,
        &ShadowView,
        Option<&ExtractedView>,
        Has<NoIndirectDrawing>,
        Has<RootNonCameraView>,
    )>,
    lights: Query<(&MainEntity, &ExtractedPointLight, &RenderLayers)>,
    sun: Query<(&MainEntity, &ExtractedDirectionalLight, &RenderLayers)>,
    bindings: Query<(&ViewShadowBindings, Option<&RenderLayers>)>,
    revisions: Res<ShadowRevisions>,
    mut cache: ResMut<CachedShadows>,
    enabled: Res<CacheEnabled>,
    stats: Res<ShadowCacheStats>,
    mut phases: ResMut<ViewBinnedRenderPhases<Shadow>>,
    support: Res<GpuPreprocessingSupport>,
    mut specialized: ResMut<SpecializedShadowMaterialPipelineCache>,
    mut pending_queues: ResMut<PendingShadowQueues>,
    mut dirty: ResMut<DirtySpecializations>,
    mut gpu_buffers: (
        Option<ResMut<BatchedInstanceBuffers<MeshUniform, MeshInputUniform>>>,
        Option<ResMut<SceneUnpackingBuffers>>,
        Option<ResMut<BinUnpackingBindGroups>>,
    ),
    (mut retained_views, light_meta): (
        Local<HashSet<RetainedViewEntity>>,
        Res<bevy::pbr::LightMeta>,
    ),
) {
    stats
        .native_prepared_views
        .store(light_meta.local_shadow_views_prepared, Ordering::Relaxed);
    stats
        .native_retained_views
        .store(light_meta.local_shadow_views_reused, Ordering::Relaxed);
    stats
        .native_attachment_allocations
        .store(light_meta.shadow_attachment_allocations, Ordering::Relaxed);
    stats
        .native_attachment_reuses
        .store(light_meta.shadow_attachment_reuses, Ordering::Relaxed);
    let _span = info_span!("shadow_cache_select").entered();
    cache.pending.clear();
    // First-person overlays have a separate camera and no world shadow casters.
    // Select the world view's allocation; an overlay must not disable caching.
    let Some((bindings, _)) = bindings
        .iter()
        .find(|(_, layers)| layers.is_none_or(|layers| layers.intersects(&RenderLayers::layer(0))))
    else {
        return;
    };
    let texture = (
        bindings.point_light_depth_texture.id(),
        bindings.directional_light_depth_texture.id(),
    );
    let allocation_changed = cache.texture != Some(texture);
    let invalidate_specializations =
        allocation_changed || cache.specialization_epoch != Some(revisions.epoch);
    cache.specialization_epoch = Some(revisions.epoch);
    if invalidate_specializations {
        stats
            .specialization_epoch_resets
            .fetch_add(1, Ordering::Relaxed);
    }
    if allocation_changed {
        cache.texture = Some(texture);
        cache.slots.clear();
        // A replacement allocation may follow render-device recovery. Native
        // preparation has already created fresh phases for the live views.
        cache.suspended_phases.clear();
        cache.phase_modes.clear();
        stats.allocations.fetch_add(1, Ordering::Relaxed);
    }
    retained_views.clear();
    for (entity, kind, shadow, live_view, no_indirect_drawing, is_root) in &views {
        // Native directional cascades belong to ViewLightEntities of their
        // parent camera. An auxiliary root would preprocess and draw the same
        // cascade again before the parent camera repeats that work.
        if is_root && matches!(kind, LightEntity::Directional { .. }) {
            commands.entity(entity).remove::<RootNonCameraView>();
            stats
                .removed_directional_roots
                .fetch_add(1, Ordering::Relaxed);
        }
        let stored = cache.suspended_views.remove(&entity);
        let Some(view) = live_view.or(stored.as_ref()) else {
            continue;
        };
        let retained = view.retained_view_entity;
        // Restoring ExtractedView is deferred. This query still sees None for
        // a revived view until commands apply, but the phase already belongs
        // to that live view. Preserve ownership through end-of-system cleanup.
        retained_views.insert(retained);
        let direct_shadow = matches!(kind, LightEntity::Point { .. } | LightEntity::Spot { .. });
        let mode = support.min(if direct_shadow || no_indirect_drawing {
            GpuPreprocessingMode::PreprocessingOnly
        } else {
            GpuPreprocessingMode::Culling
        });
        if direct_shadow {
            if !no_indirect_drawing {
                commands.entity(entity).insert(NoIndirectDrawing);
            }
            let owns_phase =
                phases.contains_key(&retained) || cache.suspended_phases.contains_key(&retained);
            if cache.phase_modes.get(&retained) != Some(&mode) || !owns_phase {
                // Occupied native preparation clears batches but retains its
                // original mode. Replace once before queueing to establish a
                // real direct phase, not merely a NoIndirectDrawing tag.
                phases.remove(&retained);
                cache.suspended_phases.remove(&retained);
                phases.prepare_for_new_frame(retained, mode);
                cache.phase_modes.insert(retained, mode);
                commands.entity(entity).insert(DirectShadowPhase(mode));
                dirty.views.insert(retained);
                if let Some(old) = specialized.remove(&retained) {
                    stats
                        .specialization_invalidated_entries
                        .fetch_add(old.len() as u64, Ordering::Relaxed);
                }
                // Work-item variants don't convert in place either. Purge
                // only this reset view/phase; native preparation rebuilds its
                // current offsets and bindings later in the same frame.
                if let Some(buffers) = gpu_buffers.0.as_deref_mut()
                    && let Some(phase_buffers) = buffers
                        .phase_instance_buffers
                        .get_mut(&TypeId::of::<Shadow>())
                {
                    phase_buffers.work_item_buffers.remove(&retained);
                }
                let key = SceneUnpackingBuffersKey {
                    phase: TypeId::of::<Shadow>(),
                    view: retained,
                };
                if let Some(buffers) = gpu_buffers.1.as_deref_mut() {
                    buffers.view_phase_buffers.remove(&key);
                }
                if let Some(bind_groups) = gpu_buffers.2.as_deref_mut() {
                    bind_groups.remove(&key);
                }
                stats
                    .direct_shadow_phase_resets
                    .fetch_add(1, Ordering::Relaxed);
            }
        }
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
            preprocessing: mode,
        };
        if invalidate_specializations {
            if let Some(old) = specialized.remove(&retained) {
                stats
                    .specialization_invalidated_entries
                    .fetch_add(old.len() as u64, Ordering::Relaxed);
            }
            // Queueing must revisit every entity too, including entries whose
            // old material/shader changed while the raster map was suspended.
            dirty.views.insert(retained);
        }
        // Native pass labels repeat across stacked-camera sun cascades.
        // Retain slot remapping checks and include the camera allocation owner.
        let slot = (
            shadow.pass_name.clone(),
            view.retained_view_entity
                .auxiliary_entity
                .map(|entity| entity.id())
                .unwrap_or(Entity::PLACEHOLDER),
        );
        // A completed map stays valid while an unrelated pipeline compiles.
        // Dirty maps become reusable only after completion confirms readiness.
        // Directional cascades consume camera-owned visibility and shared GPU
        // batching state. Suspending their views produced incomplete occlusion
        // even after requeueing every caster. Keep Bevy's current-frame cascade
        // passes; camera-independent lantern faces retain qualified reuse.
        if enabled.0 && !directional && cache.slots.get(&slot) == Some(&stamp) {
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
            // Native specialization treats missing ExtractedView as expiry.
            // Protect both its cached pipelines and pending material retries;
            // restore_specializations returns them after that cleanup.
            if specialized.contains_key(&retained) || pending_queues.contains_key(&retained) {
                cache.parked_specializations.insert(
                    retained,
                    ParkedSpecialization {
                        pipelines: specialized.remove(&retained).unwrap_or_default(),
                        pending: pending_queues.remove(&retained).unwrap_or_default(),
                    },
                );
            }
            if directional {
                &stats.directional_reused
            } else {
                &stats.reused
            }
            .fetch_add(1, Ordering::Relaxed);
        } else {
            if let Some(phase) = cache.suspended_phases.remove(&retained) {
                stats.specialization_revived_entries.fetch_add(
                    specialized.get(&retained).map_or(0, |view| view.len()) as u64,
                    Ordering::Relaxed,
                );
                phases.insert(retained, phase);
                // This is the owned configured phase. Discard the native
                // empty placeholder and retain its mode across revival.
                phases.prepare_for_new_frame(retained, mode);
            }
            if live_view.is_none()
                && let Some(view) = stored
            {
                commands.entity(entity).insert(view);
            }
            if directional {
                // Restored ExtractedView/phase is sufficient: native camera
                // preprocessing and per_view_shadow_pass visit this child.
                stats
                    .restored_directional_children
                    .fetch_add(1, Ordering::Relaxed);
            } else if !is_root {
                commands
                    .entity(entity)
                    .insert(RootNonCameraView(Core3d.intern()));
            }
            cache.slots.remove(&slot);
            cache.pending.push((slot, stamp));
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
    cache
        .suspended_phases
        .retain(|view, _| retained_views.contains(view));
    cache
        .phase_modes
        .retain(|view, _| retained_views.contains(view));
}

fn restore_specializations(
    mut cache: ResMut<CachedShadows>,
    mut specialized: ResMut<SpecializedShadowMaterialPipelineCache>,
    mut pending_queues: ResMut<PendingShadowQueues>,
    dirty: Res<DirtySpecializations>,
    phases: Res<ViewBinnedRenderPhases<Shadow>>,
    stats: Res<ShadowCacheStats>,
) {
    let _span = info_span!("shadow_specialization_cache_restore").entered();
    let CachedShadows {
        parked_specializations,
        suspended_phases,
        ..
    } = &mut *cache;
    // Retain the outer map's capacity across frames; only ownership of its
    // native per-view values moves, without cloning large specialization maps.
    for (view, mut parked) in parked_specializations.drain() {
        let Some(phase) = suspended_phases.get_mut(&view) else {
            // The owner/view expired; its native allocations expire with it.
            continue;
        };
        // Native preparation swaps these sets on revival. Keep all unfinished
        // dependencies in current_frame until then, rather than losing a retry
        // when a view spends multiple frames without specialization.
        parked
            .pending
            .current_frame
            .extend(std::mem::take(&mut parked.pending.prev_frame));
        if dirty.must_wipe_specializations_for_view(view) {
            for entity in parked.pipelines.keys().copied() {
                phase.remove(entity);
                parked
                    .pending
                    .current_frame
                    .insert((Entity::PLACEHOLDER, entity));
            }
            stats
                .specialization_invalidated_entries
                .fetch_add(parked.pipelines.len() as u64, Ordering::Relaxed);
            parked.pipelines.clear();
        } else {
            // Replay the same dirty renderable identities as native Bevy. Old
            // phase bins must not survive a material type/key or mesh change.
            // Pending retries make those entries specialize/queue on revival
            // even after the originating one-frame asset event has expired.
            for &entity in dirty.iter_to_despecialize() {
                if parked.pipelines.remove(&entity).is_some() {
                    phase.remove(entity);
                    if dirty.changed_renderables.contains(&entity) {
                        parked
                            .pending
                            .current_frame
                            .insert((Entity::PLACEHOLDER, entity));
                    }
                    stats
                        .specialization_invalidated_entries
                        .fetch_add(1, Ordering::Relaxed);
                }
            }
        }
        parked.pending.current_frame.retain(|(_, entity)| {
            !dirty.removed_renderables.contains(entity)
                || dirty.changed_renderables.contains(entity)
        });
        stats
            .specialization_protected_views
            .fetch_add(1, Ordering::Relaxed);
        stats
            .specialization_protected_entries
            .fetch_add(parked.pipelines.len() as u64, Ordering::Relaxed);
        specialized.insert(view, parked.pipelines);
        pending_queues.insert(view, parked.pending);
    }
    // Native specialization may be skipped below, so expire pending queues for
    // actual removed views here too. Suspended maps still own their phases.
    pending_queues
        .retain(|view, _| phases.contains_key(view) || cache.suspended_phases.contains_key(view));
}

fn specialization_due(
    enabled: Res<CacheEnabled>,
    dirty: Res<DirtySpecializations>,
    views: Query<(&LightEntity, &ExtractedView)>,
    visible: Query<&RenderShadowMapVisibleEntities>,
    phases: Res<ViewBinnedRenderPhases<Shadow>>,
    specialized: Res<SpecializedShadowMaterialPipelineCache>,
    pending: Res<PendingShadowQueues>,
    stats: Res<ShadowCacheStats>,
) -> bool {
    let _span = info_span!("shadow_specialization_due").entered();
    let due = !enabled.0
        || views.iter().any(|(light, view)| {
            let retained = view.retained_view_entity;
            if !phases.contains_key(&retained) {
                return false;
            }
            if dirty.must_wipe_specializations_for_view(retained) {
                return true;
            }
            let Some(pending) = pending.get(&retained) else {
                return true;
            };
            if !pending.current_frame.is_empty() || !pending.prev_frame.is_empty() {
                return true;
            }
            let (owner, visible_view) = match light {
                LightEntity::Directional { light_entity, .. } => (*light_entity, retained),
                LightEntity::Point {
                    light_entity,
                    face_index,
                } => (
                    *light_entity,
                    RetainedViewEntity {
                        auxiliary_entity: None,
                        subview_index: *face_index as u32,
                        ..retained
                    },
                ),
                LightEntity::Spot { light_entity } => (
                    *light_entity,
                    RetainedViewEntity {
                        auxiliary_entity: None,
                        subview_index: 0,
                        ..retained
                    },
                ),
            };
            let Ok(visible) = visible.get(owner) else {
                return true;
            };
            let Some(visible) = visible.subviews.get(&visible_view) else {
                return true;
            };
            let Some(meshes) = visible.1.get::<Mesh3d>() else {
                return false;
            };
            let Some(pipelines) = specialized.get(&retained) else {
                return meshes.iter_visible().next().is_some();
            };
            // Already-specialized entities can become visible again without
            // changing their pipeline. Native queue_shadows still runs and
            // adds/removes bins; skin uniforms and real shadow draws stay live.
            meshes
                .added_entities()
                .iter()
                .any(|(_, entity)| !pipelines.contains_key(entity))
                || dirty.iter_to_despecialize().any(|entity| {
                    pipelines.contains_key(entity)
                        || meshes.entity_pair_is_visible(Entity::PLACEHOLDER, *entity)
                })
        });
    if due {
        &stats.specialization_runs
    } else {
        &stats.specialization_skipped
    }
    .fetch_add(1, Ordering::Relaxed);
    due
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

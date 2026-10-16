//! Native rangefinder UI3D window 5 (005288f0), using its authored atlas.
use super::native_weapon_view::{NativeWeaponInputs, NativeWeaponViewOwner};
use avian3d::prelude::SpatialQuery;
use bevy::{
    asset::{RenderAssetUsages, embedded_asset, load_embedded_asset},
    camera::primitives::Frustum,
    core_pipeline::schedule::Core3d,
    image::ImageLoaderSettings,
    platform::collections::HashMap,
    prelude::*,
    render::{
        Extract, ExtractSchedule, Render, RenderApp, RenderSystems,
        render_asset::RenderAssets,
        render_resource::{
            binding_types::{sampler, storage_buffer_read_only, texture_2d},
            *,
        },
        renderer::{RenderContext, RenderDevice, RenderQueue},
        texture::GpuImage,
    },
    transform::TransformSystems,
};

const SIZE: u32 = 512;
const BACKGROUND: [[f32; 9]; 6] = [
    [230., 230., 53., 53., 0., 0., 53., 53., 0.],
    [8., 85., 70., 25., 0., 53., 70., 25., 0.],
    [8., 98., 35., 35., 106., 0., 35., 35., 0.],
    [472., 98., 35., 35., 106., 0., 35., 35., 270.],
    [8., 408., 35., 35., 106., 0., 35., 35., 90.],
    [472., 408., 35., 35., 106., 0., 35., 35., 180.],
];
const TARGET_BACKGROUND: [[f32; 9]; 10] = [
    [230., 230., 53., 53., 53., 0., 53., 53., 0.],
    [8., 85., 70., 25., 0., 78., 70., 25., 0.],
    [8., 98., 35., 35., 141., 0., 35., 35., 0.],
    [472., 98., 35., 35., 141., 0., 35., 35., 270.],
    [8., 408., 35., 35., 141., 0., 35., 35., 90.],
    [472., 408., 35., 35., 141., 0., 35., 35., 180.],
    [240., 85., 33., 33., 70., 53., 33., 33., 0.],
    [8., 240., 33., 33., 70., 53., 33., 33., 90.],
    [472., 240., 33., 33., 70., 53., 33., 33., 270.],
    [240., 425., 33., 33., 70., 53., 33., 33., 180.],
];

#[derive(Clone, Copy, PartialEq, ShaderType)]
struct RangefinderQuad {
    rectangle: Vec4,
    uv_origin_x: Vec4,
    uv_y: Vec4,
}
impl RangefinderQuad {
    fn source(r: [f32; 9]) -> Self {
        // Native rotates source UVs around their center, leaving pixel geometry fixed.
        let angle = r[8] * 0.017453292;
        let (sin, cos) = angle.sin_cos();
        let origin = Vec2::new(
            r[7] * 0.5 * sin + ((r[4] + r[6] * 0.5) - r[6] * 0.5 * cos),
            ((r[7] * 0.5 + r[5]) - r[6] * 0.5 * sin) - r[7] * 0.5 * cos,
        ) / Vec2::new(256., 128.);
        let x = Vec2::new(r[6] * cos, r[6] * sin) / Vec2::new(256., 128.);
        let y = Vec2::new(-r[7] * sin, r[7] * cos) / Vec2::new(256., 128.);
        Self {
            rectangle: Vec4::new(r[0], r[1], r[2], r[3]),
            uv_origin_x: Vec4::new(origin.x, origin.y, x.x, x.y),
            uv_y: y.extend(0.).extend(0.),
        }
    }
    fn diamond(center: Vec2) -> Self {
        // cg_rangeFinderDiamondSize=50; native normalized UV bounds.
        let origin = Vec2::new(0.4140625, 0.2734375);
        let extent = Vec2::new(0.67578125, 0.796875) - origin;
        Self {
            rectangle: Vec4::new(center.x - 25., center.y - 25., 50., 50.),
            uv_origin_x: Vec4::new(origin.x, origin.y, extent.x, 0.),
            uv_y: Vec4::new(0., extent.y, 0., 0.),
        }
    }
}

#[derive(Component)]
struct RangefinderHud(Entity);
#[derive(Resource, Default)]
struct RangefinderBones {
    heads: HashMap<Entity, Entity>,
}
#[derive(Default)]
struct TargetVisibility {
    visible: bool,
    next_check: i32,
}
#[derive(Resource, Default)]
struct RangefinderTargets {
    visible: HashMap<Entity, TargetVisibility>,
    latched_at: Option<i32>,
}

#[derive(Resource, Default)]
pub(crate) struct NativeRangefinderUi {
    pub owner: Option<Entity>,
    pub alternate: bool,
    target: Option<Handle<Image>>,
    atlas: Option<Handle<Image>>,
    quads: Vec<RangefinderQuad>,
    next_quads: Vec<RangefinderQuad>,
    revision: u64,
}

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum RangefinderUiSystems {
    Update,
    Prepare,
}
pub(crate) struct NativeRangefinderUiPlugin;
impl Plugin for NativeRangefinderUiPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<NativeRangefinderUi>()
            .init_resource::<RangefinderBones>()
            .init_resource::<RangefinderTargets>()
            .add_systems(
                PostUpdate,
                (attach_bones, update_ui)
                    .chain()
                    .after(TransformSystems::Propagate)
                    .in_set(RangefinderUiSystems::Update),
            );
        embedded_asset!(app, "native_rangefinder_ui.wesl");
        let shader = load_embedded_asset!(app, "native_rangefinder_ui.wesl");
        let Some(render) = app.get_sub_app_mut(RenderApp) else {
            return;
        };
        render
            .init_resource::<RangefinderRequest>()
            .insert_resource(RangefinderPipeline::new(shader))
            .init_resource::<SpecializedRenderPipelines<RangefinderPipeline>>()
            .add_systems(ExtractSchedule, extract_ui)
            .add_systems(
                Render,
                prepare_ui
                    .in_set(RenderSystems::PrepareBindGroups)
                    .in_set(RangefinderUiSystems::Prepare),
            )
            .add_systems(
                Core3d,
                render_ui
                    .before(super::frame::render_native_opaque)
                    .in_set(super::frame::NativeMainPass),
            );
    }
}

fn attach_bones(
    mut commands: Commands,
    mut bones: ResMut<RangefinderBones>,
    mut targets: ResMut<RangefinderTargets>,
    mut removed: RemovedComponents<crate::enemies::Zombie>,
    names: Query<(Entity, &Name), Added<Name>>,
    parents: Query<&ChildOf>,
    owners: Query<&NativeWeaponViewOwner>,
    rigs: Query<&crate::presentation::Rig>,
    actors: Query<(), With<crate::enemies::Zombie>>,
) {
    for entity in removed.read() {
        bones.heads.remove(&entity);
        targets.visible.remove(&entity);
    }
    for (entity, name) in &names {
        if name.as_str() == "tag_hud" {
            if let Some(owner) = parents.iter_ancestors(entity).find_map(|e| {
                owners
                    .get(e)
                    .map(|o| o.0)
                    .ok()
                    .or_else(|| rigs.get(e).ok().map(|rig| rig.owner))
            }) {
                commands.entity(entity).insert(RangefinderHud(owner));
            }
        } else if name.as_str() == "j_head" {
            if let Some(owner) = parents.iter_ancestors(entity).find(|e| actors.contains(*e)) {
                bones.heads.insert(owner, entity);
            }
        }
    }
}

fn project_to_hud(eye: Vec3, point: Vec3, inverse_hud: Mat4) -> Option<Vec2> {
    let eye = inverse_hud.transform_point3(eye);
    let point = inverse_hud.transform_point3(point);
    let denominator = eye.x - point.x;
    if denominator.abs() <= 0.0001 {
        return None;
    }
    let t = eye.x / denominator;
    if !(0. ..=1.).contains(&t) {
        return None;
    }
    let local = (point - eye) * t + eye;
    Some((Vec2::splat(0.5) - local.yz()) * 512.)
}

fn update_ui(
    mut ui: ResMut<NativeRangefinderUi>,
    mut targets: ResMut<RangefinderTargets>,
    bones: Res<RangefinderBones>,
    server: Res<AssetServer>,
    mut images: ResMut<Assets<Image>>,
    time: Res<Time<Virtual>>,
    clock: Res<crate::session::SessionClock>,
    mut session: ResMut<crate::gameplay::Session>,
    cameras: Query<(&GlobalTransform, &Frustum), With<super::GameCamera>>,
    inventories: Query<(Entity, &crate::weapons::WeaponInventory)>,
    huds: Query<(&RangefinderHud, &GlobalTransform, &InheritedVisibility)>,
    globals: Query<&GlobalTransform>,
    actors: Query<(
        Entity,
        &crate::enemies::Zombie,
        &Transform,
        &InheritedVisibility,
    )>,
    ray_targets: crate::hit_regions::RayTargets,
    weapon_inputs: Query<(&NativeWeaponViewOwner, &NativeWeaponInputs)>,
    spatial: SpatialQuery,
) {
    let active = inventories.iter().find(|(_, inventory)| {
        inventory.attachment() == "rangefinder" && inventory.active().is_some()
    });
    ui.owner = active.map(|(owner, _)| owner);
    ui.alternate = false;
    let Some((owner, inventory)) = active else {
        targets.latched_at = None;
        targets.visible.clear();
        return;
    };
    let Some((camera, frustum)) = cameras.iter().next() else {
        return;
    };
    let eye = camera.translation();
    let now = clock.milliseconds(&time);
    // Source CG[84434] latch is set only at full ADS; alternate art begins later
    // than the latch timestamp (0052b080 /00527c40).
    let locked = inventory.ads == 1.
        && crate::hit_regions::cast(
            &ray_targets,
            &spatial,
            owner,
            eye,
            camera.forward(),
            8192. * crate::assets::SOURCE_UNIT_METERS,
        )
        .is_some();
    if locked {
        let latched_at = *targets.latched_at.get_or_insert(now);
        ui.alternate = now > latched_at;
    } else {
        targets.latched_at = None;
    }
    let suppressed = weapon_inputs
        .iter()
        .any(|(input_owner, input)| input_owner.0 == owner && input.parameter0.y != 0.);
    if ui.target.is_none() {
        let mut target = Image::new_uninit(
            Extent3d {
                width: SIZE,
                height: SIZE,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            TextureFormat::Rgba8Unorm,
            RenderAssetUsages::RENDER_WORLD,
        );
        target.texture_descriptor.label = Some("T6 UI3D window5 512x512");
        target.texture_descriptor.usage =
            TextureUsages::TEXTURE_BINDING | TextureUsages::RENDER_ATTACHMENT;
        ui.target = Some(images.add(target));
        ui.atlas = Some(
            server
                .load_builder()
                .with_settings(|settings: &mut ImageLoaderSettings| {
                    settings.is_srgb = false;
                })
                .load("images/mtl_t6_attach_rfo_reticle.dds"),
        );
    }
    let mut quads = std::mem::take(&mut ui.next_quads);
    quads.clear();
    // Native condition accepts exactly .99 and greater.
    if inventory.ads >= 0.99 && !suppressed {
        quads.extend(
            if ui.alternate {
                TARGET_BACKGROUND.as_slice()
            } else {
                BACKGROUND.as_slice()
            }
            .iter()
            .copied()
            .map(RangefinderQuad::source),
        );
        if let Some((_, hud, _)) = huds
            .iter()
            .find(|(tag, _, visible)| tag.0 == owner && visible.get())
        {
            let inverse_hud = hud.to_matrix().inverse();
            targets.visible.retain(|entity, _| actors.contains(*entity));
            for (entity, zombie, pose, visible) in &actors {
                let Ok((_, health, _, _)) = ray_targets.get(entity) else {
                    continue;
                };
                if health.state() != crate::life::LifeState::ActorAlive || !visible.get() {
                    continue;
                }
                let visibility = targets.visible.entry(entity).or_default();
                // 004b4680 checks visible actors every frame; only hidden
                // actors wait for the stored retry timestamp.
                if visibility.visible || now > visibility.next_check {
                    let point = bones
                        .heads
                        .get(&entity)
                        .and_then(|e| globals.get(*e).ok())
                        .map_or(
                            pose.translation + Vec3::Y * (10. * crate::assets::SOURCE_UNIT_METERS),
                            GlobalTransform::translation,
                        );
                    let delta = point - eye;
                    // 004b3320 defaults: max10000in, frustum expand -30in,
                    // skip frustum test inside sqrt(100)in.
                    visibility.visible = camera.forward().dot(delta) >= 0.
                        && delta.length_squared()
                            <= (10000. * crate::assets::SOURCE_UNIT_METERS).powi(2)
                        && (delta.length_squared()
                            <= 100. * crate::assets::SOURCE_UNIT_METERS.powi(2)
                            || !frustum.half_spaces.iter().any(|plane| {
                                plane.normal().dot(point.into()) + plane.d()
                                    < -30. * crate::assets::SOURCE_UNIT_METERS
                            }))
                        && Dir3::new(delta).ok().is_some_and(|direction| {
                            crate::hit_regions::cast(
                                &ray_targets,
                                &spatial,
                                owner,
                                eye,
                                direction,
                                delta.length() + 0.001,
                            )
                            .is_some_and(|hit| hit.target == entity)
                        });
                    let (base, spread) = if visibility.visible {
                        (200, 101)
                    } else {
                        (100, 51)
                    };
                    visibility.next_check = now + (base + session.random() % spread) as i32;
                }
                if visibility.visible {
                    let point = pose.translation
                        + Vec3::Y
                            * (if zombie.crawler() { 11. } else { 45. }
                                * crate::assets::SOURCE_UNIT_METERS);
                    if let Some(center) = project_to_hud(eye, point, inverse_hud) {
                        quads.push(RangefinderQuad::diamond(center));
                    }
                }
            }
        }
    }
    if quads != ui.quads {
        ui.revision = ui.revision.wrapping_add(1);
        std::mem::swap(&mut ui.quads, &mut quads);
    }
    ui.next_quads = quads;
}

#[derive(Resource, Default)]
struct RangefinderRequest {
    target: Option<AssetId<Image>>,
    atlas: Option<AssetId<Image>>,
    quads: Vec<RangefinderQuad>,
    revision: u64,
}
fn extract_ui(mut request: ResMut<RangefinderRequest>, ui: Extract<Res<NativeRangefinderUi>>) {
    if request.target != ui.target.as_ref().map(Handle::id) || request.revision != ui.revision {
        request.target = ui.target.as_ref().map(Handle::id);
        request.atlas = ui.atlas.as_ref().map(Handle::id);
        request.quads.clone_from(&ui.quads);
        request.revision = ui.revision;
    }
}

#[derive(Resource)]
struct RangefinderPipeline {
    shader: Handle<Shader>,
    layout: BindGroupLayoutDescriptor,
}
impl RangefinderPipeline {
    fn new(shader: Handle<Shader>) -> Self {
        Self {
            shader,
            layout: BindGroupLayoutDescriptor::new(
                "T6 rangefinder atlas",
                &BindGroupLayoutEntries::sequential(
                    ShaderStages::VERTEX_FRAGMENT,
                    (
                        texture_2d(TextureSampleType::Float { filterable: true }),
                        sampler(SamplerBindingType::Filtering),
                        storage_buffer_read_only::<Vec<RangefinderQuad>>(false),
                    ),
                ),
            ),
        }
    }
}
impl SpecializedRenderPipeline for RangefinderPipeline {
    type Key = ();
    fn specialize(&self, _: ()) -> RenderPipelineDescriptor {
        let additive = BlendComponent {
            src_factor: BlendFactor::One,
            dst_factor: BlendFactor::One,
            operation: BlendOperation::Add,
        };
        RenderPipelineDescriptor {
            label: Some("T6 rangefinder UI3D additive quads".into()),
            layout: vec![self.layout.clone()],
            vertex: VertexState {
                shader: self.shader.clone(),
                entry_point: Some("vertex".into()),
                ..default()
            },
            fragment: Some(FragmentState {
                shader: self.shader.clone(),
                entry_point: Some("fragment".into()),
                targets: vec![Some(ColorTargetState {
                    format: TextureFormat::Rgba8Unorm,
                    blend: Some(BlendState {
                        color: additive,
                        alpha: additive,
                    }),
                    write_mask: ColorWrites::ALL,
                })],
                ..default()
            }),
            primitive: PrimitiveState {
                cull_mode: None,
                ..default()
            },
            ..default()
        }
    }
}
/// Actual generated UI3D target, consumed by the rangefinder lens. This is never
/// substituted with a constant texture when an attachment is active.
#[derive(Resource)]
pub(crate) struct PreparedRangefinderUi {
    pub view: TextureView,
    pub sampler: Sampler,
    quads: StorageBuffer<Vec<RangefinderQuad>>,
    group: BindGroup,
    atlas: TextureViewId,
    pipeline: CachedRenderPipelineId,
    revision: u64,
    rendered: Option<u64>,
    count: u32,
}
fn prepare_ui(
    mut commands: Commands,
    request: Res<RangefinderRequest>,
    images: Res<RenderAssets<GpuImage>>,
    device: Res<RenderDevice>,
    queue: Res<RenderQueue>,
    cache: Res<PipelineCache>,
    pipeline: Res<RangefinderPipeline>,
    mut variants: ResMut<SpecializedRenderPipelines<RangefinderPipeline>>,
    prepared: Option<ResMut<PreparedRangefinderUi>>,
) {
    let (Some(target), Some(atlas)) = (
        request.target.and_then(|id| images.get(id)),
        request.atlas.and_then(|id| images.get(id)),
    ) else {
        return;
    };
    if let Some(mut prepared) = prepared {
        if prepared.revision == request.revision && prepared.atlas == atlas.texture_view.id() {
            return;
        }
        let old_buffer = prepared.quads.buffer().map(Buffer::id);
        prepared.quads.get_mut().clone_from(&request.quads);
        if prepared.quads.get().is_empty() {
            prepared
                .quads
                .get_mut()
                .push(RangefinderQuad::source(BACKGROUND[0]));
        }
        prepared.quads.write_buffer(&device, &queue);
        if old_buffer != prepared.quads.buffer().map(Buffer::id)
            || prepared.atlas != atlas.texture_view.id()
        {
            prepared.group = device.create_bind_group(
                "T6 rangefinder atlas",
                &cache.get_bind_group_layout(&pipeline.layout),
                &BindGroupEntries::sequential((
                    &atlas.texture_view,
                    &prepared.sampler,
                    prepared.quads.binding().expect("rangefinder quad upload"),
                )),
            );
        }
        prepared.atlas = atlas.texture_view.id();
        prepared.revision = request.revision;
        prepared.count = request.quads.len() as u32;
        return;
    }
    let sampler = device.create_sampler(&SamplerDescriptor {
        label: Some("T6 rangefinder UI3D clamp linear"),
        address_mode_u: AddressMode::ClampToEdge,
        address_mode_v: AddressMode::ClampToEdge,
        address_mode_w: AddressMode::ClampToEdge,
        mag_filter: FilterMode::Linear,
        min_filter: FilterMode::Linear,
        lod_max_clamp: 0.,
        ..default()
    });
    let mut quads = StorageBuffer::from(if request.quads.is_empty() {
        vec![RangefinderQuad::source(BACKGROUND[0])]
    } else {
        request.quads.clone()
    });
    quads.write_buffer(&device, &queue);
    let group = device.create_bind_group(
        "T6 rangefinder atlas",
        &cache.get_bind_group_layout(&pipeline.layout),
        &BindGroupEntries::sequential((
            &atlas.texture_view,
            &sampler,
            quads.binding().expect("rangefinder quad upload"),
        )),
    );
    commands.insert_resource(PreparedRangefinderUi {
        view: target.texture_view.clone(),
        sampler,
        quads,
        group,
        atlas: atlas.texture_view.id(),
        pipeline: variants.specialize(&cache, &pipeline, ()),
        revision: request.revision,
        rendered: None,
        count: request.quads.len() as u32,
    });
}
fn render_ui(
    prepared: Option<ResMut<PreparedRangefinderUi>>,
    cache: Res<PipelineCache>,
    mut context: RenderContext,
) {
    let Some(mut prepared) = prepared else { return };
    if prepared.rendered == Some(prepared.revision) {
        return;
    }
    let Some(pipeline) = cache.get_render_pipeline(prepared.pipeline) else {
        return;
    };
    let attachments = [Some(RenderPassColorAttachment {
        view: &prepared.view,
        depth_slice: None,
        resolve_target: None,
        ops: Operations {
            load: LoadOp::Clear(LinearRgba::NONE.into()),
            store: StoreOp::Store,
        },
    })];
    let mut pass = context
        .command_encoder()
        .begin_render_pass(&RenderPassDescriptor {
            label: Some("T6 rangefinder UI3D window5"),
            color_attachments: &attachments,
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
    pass.set_pipeline(pipeline);
    pass.set_bind_group(0, &prepared.group, &[]);
    // The source finishes by drawing opaque black, then clearAlpha, over rows0..85.
    pass.set_scissor_rect(0, 85, SIZE, SIZE - 85);
    pass.draw(0..6, 0..prepared.count);
    drop(pass);
    prepared.rendered = Some(prepared.revision);
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn source_rangefinder_hud_plane_projection_and_rejection() {
        assert_eq!(
            project_to_hud(Vec3::X, -Vec3::X, Mat4::IDENTITY),
            Some(Vec2::splat(256.))
        );
        assert_eq!(project_to_hud(Vec3::X, Vec3::X * 2., Mat4::IDENTITY), None);
        assert_eq!(
            project_to_hud(Vec3::X, Vec3::Y, Mat4::IDENTITY),
            Some(Vec2::new(-256., 256.))
        );
        assert_eq!(project_to_hud(Vec3::ZERO, Vec3::Y, Mat4::IDENTITY), None);
    }
    #[test]
    fn authored_rangefinder_rotation_changes_uv_and_preserves_rectangle() {
        let q = RangefinderQuad::source(BACKGROUND[3]);
        assert_eq!(q.rectangle, Vec4::new(472., 98., 35., 35.));
        let last = q.uv_origin_x.xy() + q.uv_origin_x.zw() + q.uv_y.xy();
        assert!((last - Vec2::new(141. / 256., 0.)).length() < 0.00001);
        assert_eq!(BACKGROUND.len(), 6);
        assert_eq!(TARGET_BACKGROUND.len(), 10);
    }
}

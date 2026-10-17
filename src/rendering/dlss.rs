//! Adapt the source renderer's encoded color and native depth to Bevy's DLSS inputs.
use super::{frame::NativeRenderView, native::NativeWorldScene, native_effect::NativeFxFrame};
mod motion;
use bevy::{
    anti_alias::dlss::{Dlss, DlssBiasCurrentColor},
    asset::{embedded_asset, load_embedded_asset},
    camera::MainPassResolutionOverride,
    core_pipeline::{
        FullscreenShader,
        prepass::ViewPrepassTextures,
        schedule::{Core3d, Core3dSystems},
    },
    prelude::*,
    render::{
        Render, RenderApp, RenderSystems,
        camera::{MipBias, TemporalJitter},
        render_asset::RenderAssets,
        render_resource::*,
        renderer::{RenderContext, RenderDevice, RenderQueue, ViewQuery},
        texture::{CachedTexture, GpuImage, TextureCache},
        view::{ExtractedView, ViewDepthStencilTexture, ViewTarget},
    },
};
pub(super) use motion::NativeMotion;

pub(super) struct NativeDlssPlugin;
impl Plugin for NativeDlssPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(crate::GameState::StartingSession), reset_history);
        embedded_asset!(app, "dlss_inputs.wesl");
        embedded_asset!(app, "dlss_motion.wesl");
        let shader = load_embedded_asset!(app, "dlss_inputs.wesl");
        let motion_shader = load_embedded_asset!(app, "dlss_motion.wesl");
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };
        render_app
            .insert_resource(NativeMotion::new(motion_shader))
            .insert_resource(DlssInputPipeline {
                shader,
                pipeline: None,
                layout: None,
            })
            .add_systems(
                Render,
                (
                    jitter_native_scene
                        .after(RenderSystems::PrepareViews)
                        .before(RenderSystems::Queue),
                    prepare_inputs.in_set(RenderSystems::PrepareBindGroups),
                    motion::prepare.in_set(RenderSystems::PrepareBindGroups),
                ),
            )
            .configure_sets(
                Core3d,
                (
                    Core3dSystems::Prepass.run_if(stock_prepass),
                    Core3dSystems::MainPass.run_if(stock_prepass),
                ),
            )
            .add_systems(
                Core3d,
                (write_inputs, motion::render)
                    .chain()
                    .after(super::frame::NativeMainPass)
                    .before(Core3dSystems::EarlyPostProcess),
            );
    }
}

pub(crate) fn reset_history(world: &mut World) {
    let mut cameras = world.query_filtered::<&mut Dlss, With<super::GameCamera>>();
    for mut dlss in cameras.iter_mut(world) {
        dlss.reset = true;
    }
    let mut cameras = world.query_filtered::<&mut bevy::anti_alias::dlss::neural_rendering::DlssNeuralRendering, With<super::GameCamera>>();
    for mut nr in cameras.iter_mut(world) {
        nr.reset = true;
    }
}

pub(super) fn toggle_neural_rendering(
    keys: Res<ButtonInput<KeyCode>>,
    mut cameras: Query<
        &mut bevy::anti_alias::dlss::neural_rendering::DlssNeuralRendering,
        With<super::GameCamera>,
    >,
) {
    if !keys.just_pressed(KeyCode::F11) {
        return;
    }
    for mut nr in &mut cameras {
        nr.intensity = if nr.intensity == 0.0 { 1.0 } else { 0.0 };
        nr.reset = true;
        info!(
            "DLSS Neural Rendering {}",
            if nr.intensity == 0.0 {
                "disabled"
            } else {
                "enabled"
            }
        );
    }
}

// The native D32S8 main attachment cannot be copied by Bevy's D32 prepass.
// write_inputs supplies the DLSS prepass textures after the native main pass.
fn stock_prepass(view: ViewQuery<Has<NativeRenderView>>) -> bool {
    !view.into_inner()
}

fn jitter_native_scene(
    mut motion: ResMut<NativeMotion>,
    frame: Res<bevy::diagnostic::FrameCount>,
    mut views: Query<
        (
            &TemporalJitter,
            &MainPassResolutionOverride,
            &ExtractedView,
            &Dlss,
            &MipBias,
            &mut NativeWorldScene,
            Option<&mut NativeFxFrame>,
            Option<&mut super::native_water::WaterScene>,
        ),
        With<Dlss>,
    >,
) {
    for (jitter, size, view, dlss, mip_bias, mut scene, effects, water) in &mut views {
        scene.texture_mip_scale = mip_bias.0.exp2();
        let unjittered = *scene;
        let offset = jitter.offset * Vec2::new(2., -2.) / size.0.as_vec2();
        let w = scene.view_projection_rows[3];
        scene.view_projection_rows[0] -= w * offset.x;
        scene.view_projection_rows[1] -= w * offset.y;
        motion.begin(view, &unjittered, &scene, frame.0, dlss.reset);
        if let Some(mut effects) = effects {
            effects.scene = *scene;
        }
        if let Some(mut water) = water {
            water.world = *scene;
        }
    }
}

#[derive(Resource)]
struct DlssInputPipeline {
    shader: Handle<Shader>,
    pipeline: Option<CachedRenderPipelineId>,
    layout: Option<BindGroupLayoutDescriptor>,
}

#[derive(Clone, Copy, ShaderType)]
struct Reprojection {
    world_from_clip: Mat4,
    clip_from_world: Mat4,
    previous_clip_from_world: Mat4,
    render_size: Vec4,
    jitter_delta: Vec4,
}

#[derive(Component)]
struct DlssInputs {
    source_ids: [TextureViewId; 3],
    group: [BindGroup; 2],
    uniforms: UniformBuffer<Reprojection>,
    previous_clip_from_world: Mat4,
    size: UVec2,
    bias: CachedTexture,
    reactive: [CachedTexture; 2],
    index: usize,
    frame: u32,
    rendered: bool,
    jitter: Vec2,
}

fn prepare_inputs(
    mut commands: Commands,
    device: Res<RenderDevice>,
    queue: Res<RenderQueue>,
    cache: Res<PipelineCache>,
    fullscreen: Res<FullscreenShader>,
    images: Res<RenderAssets<GpuImage>>,
    mut textures: ResMut<TextureCache>,
    frame: Res<bevy::diagnostic::FrameCount>,
    mut pipeline: ResMut<DlssInputPipeline>,
    mut views: Query<
        (
            Entity,
            &ExtractedView,
            &ViewTarget,
            &super::frame::NativeSceneColor,
            &super::frame::NativeDepthImage,
            &ViewDepthStencilTexture,
            &MainPassResolutionOverride,
            &TemporalJitter,
            &Dlss,
            Option<&mut DlssInputs>,
        ),
        With<NativeRenderView>,
    >,
) {
    if views.is_empty() {
        return;
    }
    if pipeline.pipeline.is_none() {
        let layout = BindGroupLayoutDescriptor::new(
            "Origins DLSS inputs",
            &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        sample_type: TextureSampleType::Float { filterable: false },
                        view_dimension: TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 3,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        sample_type: TextureSampleType::Float { filterable: false },
                        view_dimension: TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 4,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        sample_type: TextureSampleType::Float { filterable: false },
                        view_dimension: TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        sample_type: TextureSampleType::Depth,
                        view_dimension: TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 2,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: Some(Reprojection::min_size()),
                    },
                    count: None,
                },
            ],
        );
        pipeline.pipeline = Some(cache.queue_render_pipeline(RenderPipelineDescriptor {
            label: Some("Origins DLSS color, depth and camera motion".into()),
            layout: vec![layout.clone()],
            vertex: fullscreen.to_vertex_state(),
            fragment: Some(FragmentState {
                shader: pipeline.shader.clone(),
                entry_point: Some("fragment".into()),
                targets: vec![
                    Some(ColorTargetState {
                        format: TextureFormat::Rgba16Float,
                        blend: None,
                        write_mask: ColorWrites::ALL,
                    }),
                    Some(ColorTargetState {
                        format: TextureFormat::Rg16Float,
                        blend: None,
                        write_mask: ColorWrites::ALL,
                    }),
                    Some(ColorTargetState {
                        format: TextureFormat::R8Unorm,
                        blend: None,
                        write_mask: ColorWrites::ALL,
                    }),
                    Some(ColorTargetState {
                        format: TextureFormat::R8Unorm,
                        blend: None,
                        write_mask: ColorWrites::ALL,
                    }),
                ],
                ..default()
            }),
            depth_stencil: Some(DepthStencilState {
                format: TextureFormat::Depth32Float,
                depth_write_enabled: Some(true),
                depth_compare: Some(CompareFunction::Always),
                stencil: default(),
                bias: default(),
            }),
            ..default()
        }));
        pipeline.layout = Some(layout);
    }
    for (entity, view, _target, color, native_depth, depth, size, jitter, dlss, previous) in
        &mut views
    {
        let Some(before) = images.get(native_depth.resolved_scene) else {
            continue;
        };
        let history_valid = previous
            .as_ref()
            .is_some_and(|p| p.rendered && p.frame == frame.0.wrapping_sub(1) && p.size == size.0)
            && !dlss.reset;
        let view_from_world = view.world_from_view.to_matrix().inverse();
        let clip_from_world = view.clip_from_view * view_from_world;
        let mut projection = view.clip_from_view;
        jitter.jitter_projection(&mut projection, size.0.as_vec2());
        let world_from_clip = (projection * view_from_world).inverse();
        let mut data = Reprojection {
            world_from_clip,
            clip_from_world,
            previous_clip_from_world: previous
                .as_ref()
                .filter(|_| history_valid)
                .map_or(clip_from_world, |previous| {
                    previous.previous_clip_from_world
                }),
            render_size: size
                .0
                .as_vec2()
                .extend(if history_valid { 1. } else { 0. })
                .extend(0.),
            jitter_delta: previous
                .as_ref()
                .filter(|_| history_valid)
                .map_or(Vec2::ZERO, |p| jitter.offset - p.jitter)
                .extend(0.)
                .extend(0.),
        };
        let depth = depth
            .attachment
            .depth_stencil_views()
            .depth_only_view()
            .expect("native depth aspect");
        let source = &color.image.default_view;
        let ids = [source.id(), depth.id(), before.texture_view.id()];
        if let Some(mut previous) = previous.filter(|previous| previous.source_ids == ids) {
            previous.uniforms.set(data);
            previous.uniforms.write_buffer(&device, &queue);
            previous.previous_clip_from_world = clip_from_world;
            previous.size = size.0;
            previous.index ^= 1;
            previous.frame = frame.0;
            previous.rendered = false;
            previous.jitter = jitter.offset;
            continue;
        }
        // New source attachments allocate new mask history, even at the same resolution.
        data.render_size.z = 0.;
        let mut uniforms = UniformBuffer::from(data);
        uniforms.write_buffer(&device, &queue);
        let descriptor = TextureDescriptor {
            label: Some("DLSS current-color bias"),
            size: Extent3d {
                width: color.size.x,
                height: color.size.y,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::R8Unorm,
            usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        };
        let bias = textures.get(&device, descriptor.clone());
        let reactive = std::array::from_fn(|_| textures.get(&device, descriptor.clone()));
        let group = std::array::from_fn(|index| {
            device.create_bind_group(
                "Origins DLSS source",
                &cache.get_bind_group_layout(pipeline.layout.as_ref().unwrap()),
                &BindGroupEntries::sequential((
                    source,
                    depth,
                    uniforms.binding().unwrap(),
                    &before.texture_view,
                    &reactive[index ^ 1].default_view,
                )),
            )
        });
        commands.entity(entity).insert((
            DlssBiasCurrentColor(bias.default_view.clone()),
            DlssInputs {
                source_ids: ids,
                group,
                uniforms,
                previous_clip_from_world: clip_from_world,
                size: size.0,
                bias,
                reactive,
                index: 0,
                frame: frame.0,
                rendered: false,
                jitter: jitter.offset,
            },
        ));
    }
}

fn write_inputs(
    view: ViewQuery<(&ViewTarget, &ViewPrepassTextures, &mut DlssInputs), With<Dlss>>,
    pipeline: Res<DlssInputPipeline>,
    cache: Res<PipelineCache>,
    mut ctx: RenderContext,
) {
    let (target, prepass, mut inputs) = view.into_inner();
    let Some(pipeline) = pipeline
        .pipeline
        .and_then(|id| cache.get_render_pipeline(id))
    else {
        return;
    };
    let (Some(depth), Some(motion)) = (&prepass.depth, &prepass.motion_vectors) else {
        return;
    };
    let output = target.post_process_write();
    let attachments = [
        Some(RenderPassColorAttachment {
            view: output.destination,
            depth_slice: None,
            resolve_target: None,
            ops: Operations {
                load: LoadOp::Clear(default()),
                store: StoreOp::Store,
            },
        }),
        Some(RenderPassColorAttachment {
            view: &motion.texture.default_view,
            depth_slice: None,
            resolve_target: None,
            ops: Operations {
                load: LoadOp::Clear(default()),
                store: StoreOp::Store,
            },
        }),
        Some(RenderPassColorAttachment {
            view: &inputs.bias.default_view,
            depth_slice: None,
            resolve_target: None,
            ops: Operations {
                load: LoadOp::Clear(default()),
                store: StoreOp::Store,
            },
        }),
        Some(RenderPassColorAttachment {
            view: &inputs.reactive[inputs.index].default_view,
            depth_slice: None,
            resolve_target: None,
            ops: Operations {
                load: LoadOp::Clear(default()),
                store: StoreOp::Store,
            },
        }),
    ];
    let mut pass = ctx
        .command_encoder()
        .begin_render_pass(&RenderPassDescriptor {
            label: Some("Origins native DLSS inputs"),
            color_attachments: &attachments,
            depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                view: depth.depth_stencil_views().depth_only_view().unwrap(),
                depth_ops: Some(Operations {
                    load: LoadOp::Clear(0.),
                    store: StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
    pass.set_viewport(0., 0., inputs.size.x as f32, inputs.size.y as f32, 0., 1.);
    pass.set_pipeline(pipeline);
    pass.set_bind_group(0, &inputs.group[inputs.index], &[]);
    pass.draw(0..3, 0..1);
    drop(pass);
    inputs.rendered = true;
}

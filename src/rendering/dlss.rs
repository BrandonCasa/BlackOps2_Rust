//! Adapt the source renderer's encoded color and native depth to Bevy's DLSS inputs.
use super::{frame::NativeRenderView, native::NativeWorldScene, native_effect::NativeFxFrame};
use bevy::{
    anti_alias::dlss::Dlss,
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
        camera::TemporalJitter,
        render_resource::*,
        renderer::{RenderContext, RenderDevice, RenderQueue, ViewQuery},
        view::{ExtractedView, ViewDepthStencilTexture, ViewTarget},
    },
};

pub(super) struct NativeDlssPlugin;
impl Plugin for NativeDlssPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "dlss_inputs.wesl");
        let shader = load_embedded_asset!(app, "dlss_inputs.wesl");
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };
        render_app
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
                write_inputs
                    .after(super::frame::NativeMainPass)
                    .before(Core3dSystems::EarlyPostProcess),
            );
    }
}

// The native D32S8 main attachment cannot be copied by Bevy's D32 prepass.
// write_inputs supplies the DLSS prepass textures after the native main pass.
fn stock_prepass(view: ViewQuery<Has<NativeRenderView>>) -> bool {
    !view.into_inner()
}

fn jitter_native_scene(
    mut views: Query<
        (
            &TemporalJitter,
            &MainPassResolutionOverride,
            &mut NativeWorldScene,
            Option<&mut NativeFxFrame>,
            Option<&mut super::native_water::WaterScene>,
        ),
        With<Dlss>,
    >,
) {
    for (jitter, size, mut scene, effects, water) in &mut views {
        let offset = jitter.offset * Vec2::new(2., -2.) / size.0.as_vec2();
        let w = scene.view_projection_rows[3];
        scene.view_projection_rows[0] -= w * offset.x;
        scene.view_projection_rows[1] -= w * offset.y;
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
}

#[derive(Component)]
struct DlssInputs {
    source_ids: [TextureViewId; 2],
    group: BindGroup,
    uniforms: UniformBuffer<Reprojection>,
    previous_clip_from_world: Mat4,
    size: UVec2,
}

fn prepare_inputs(
    mut commands: Commands,
    device: Res<RenderDevice>,
    queue: Res<RenderQueue>,
    cache: Res<PipelineCache>,
    fullscreen: Res<FullscreenShader>,
    mut pipeline: ResMut<DlssInputPipeline>,
    mut views: Query<
        (
            Entity,
            &ExtractedView,
            &ViewTarget,
            &super::frame::NativeSceneColor,
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
    for (entity, view, _target, color, depth, size, jitter, dlss, previous) in &mut views {
        let view_from_world = view.world_from_view.to_matrix().inverse();
        let clip_from_world = view.clip_from_view * view_from_world;
        let mut projection = view.clip_from_view;
        jitter.jitter_projection(&mut projection, size.0.as_vec2());
        let world_from_clip = (projection * view_from_world).inverse();
        let data = Reprojection {
            world_from_clip,
            clip_from_world,
            previous_clip_from_world: previous
                .as_ref()
                .filter(|previous| previous.size == size.0 && !dlss.reset)
                .map_or(clip_from_world, |previous| {
                    previous.previous_clip_from_world
                }),
            render_size: size.0.as_vec2().extend(0.).extend(0.),
        };
        let depth = depth
            .attachment
            .depth_stencil_views()
            .depth_only_view()
            .expect("native depth aspect");
        let source = &color.image.default_view;
        let ids = [source.id(), depth.id()];
        if let Some(mut previous) = previous.filter(|previous| previous.source_ids == ids) {
            previous.uniforms.set(data);
            previous.uniforms.write_buffer(&device, &queue);
            previous.previous_clip_from_world = clip_from_world;
            previous.size = size.0;
            continue;
        }
        let mut uniforms = UniformBuffer::from(data);
        uniforms.write_buffer(&device, &queue);
        let group = device.create_bind_group(
            "Origins DLSS source",
            &cache.get_bind_group_layout(pipeline.layout.as_ref().unwrap()),
            &BindGroupEntries::sequential((source, depth, uniforms.binding().unwrap())),
        );
        commands.entity(entity).insert(DlssInputs {
            source_ids: ids,
            group,
            uniforms,
            previous_clip_from_world: clip_from_world,
            size: size.0,
        });
    }
}

fn write_inputs(
    view: ViewQuery<(&ViewTarget, &ViewPrepassTextures, &DlssInputs), With<Dlss>>,
    pipeline: Res<DlssInputPipeline>,
    cache: Res<PipelineCache>,
    mut ctx: RenderContext,
) {
    let (target, prepass, inputs) = view.into_inner();
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
    pass.set_bind_group(0, &inputs.group, &[]);
    pass.draw(0..3, 0..1);
}

//! Depth submission for the original transform-only shadowcaster technique.
use super::{ShadowViewSettings, sun::SunShadowViews};
use crate::rendering::native::{NativeWorldGeometry, PreparedNativeGeometry};
use bevy::{
    core_pipeline::{Core3dSystems, core_3d::main_opaque_pass_3d, schedule::Core3d},
    ecs::query::QueryItem,
    mesh::VertexBufferLayout,
    prelude::*,
    render::{
        Render, RenderApp, RenderSystems,
        extract_component::{ExtractComponent, ExtractComponentPlugin},
        render_asset::RenderAssets,
        render_resource::{binding_types::uniform_buffer, *},
        renderer::{RenderContext, RenderDevice, RenderQueue, ViewQuery},
        sync_component::SyncComponent,
    },
};
use std::sync::Arc;

#[derive(Clone)]
pub(crate) struct DepthDraw {
    pub primary_offset: u64,
    pub first_index: u32,
    pub index_count: u32,
}

#[derive(Component, Clone)]
pub(crate) struct ShadowCasters {
    pub geometry: Handle<NativeWorldGeometry>,
    pub draws: Arc<[DepthDraw]>,
}

impl SyncComponent<RenderApp> for ShadowCasters {
    type Target = Self;
}
impl ExtractComponent<RenderApp> for ShadowCasters {
    type QueryData = (&'static Self, &'static InheritedVisibility);
    type QueryFilter = ();
    type Out = Self;
    fn extract_component((casters, visible): QueryItem<'_, '_, Self::QueryData>) -> Option<Self> {
        visible.get().then(|| casters.clone())
    }
}

#[derive(Clone, Copy, ShaderType)]
struct DepthMatrix {
    rows: [Vec4; 4],
}
impl From<Mat4> for DepthMatrix {
    fn from(matrix: Mat4) -> Self {
        Self {
            rows: std::array::from_fn(|i| matrix.row(i)),
        }
    }
}

#[derive(Resource)]
struct DepthPipeline {
    shader: Handle<Shader>,
    layout: BindGroupLayoutDescriptor,
    pipeline: Option<CachedRenderPipelineId>,
}

#[derive(Component)]
pub(crate) struct SunShadowAtlas {
    _texture: Texture,
    pub view: TextureView,
    size: u32,
    matrices: [UniformBuffer<DepthMatrix>; 2],
    groups: [BindGroup; 2],
    _identity: UniformBuffer<DepthMatrix>,
    object_group: BindGroup,
}

pub(super) fn install(app: &mut App, shader: Handle<Shader>) {
    app.add_plugins((
        ExtractComponentPlugin::<ShadowCasters>::default(),
        ExtractComponentPlugin::<ShadowViewSettings>::default(),
        ExtractComponentPlugin::<SunShadowViews>::default(),
    ));
    let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
        return;
    };
    render_app
        .insert_resource(DepthPipeline {
            shader,
            layout: BindGroupLayoutDescriptor::new(
                "native shadow matrix",
                &BindGroupLayoutEntries::single(
                    ShaderStages::VERTEX,
                    uniform_buffer::<DepthMatrix>(false),
                ),
            ),
            pipeline: None,
        })
        .add_systems(
            Render,
            prepare_atlas.in_set(RenderSystems::PrepareBindGroups),
        )
        .add_systems(
            Core3d,
            render_sun_depth
                .before(main_opaque_pass_3d)
                .in_set(Core3dSystems::MainPass),
        );
}

fn prepare_atlas(
    mut commands: Commands,
    device: Res<RenderDevice>,
    queue: Res<RenderQueue>,
    cache: Res<PipelineCache>,
    mut pipeline: ResMut<DepthPipeline>,
    mut cameras: Query<(Entity, Ref<SunShadowViews>, Option<&mut SunShadowAtlas>)>,
) {
    if pipeline.pipeline.is_none() {
        pipeline.pipeline = Some(cache.queue_render_pipeline(RenderPipelineDescriptor {
            label: Some("T6 transform-only shadow depth".into()),
            layout: vec![pipeline.layout.clone(), pipeline.layout.clone()],
            vertex: VertexState {
                shader: pipeline.shader.clone(),
                entry_point: Some("vertex".into()),
                buffers: vec![VertexBufferLayout {
                    array_stride: 36,
                    step_mode: VertexStepMode::Vertex,
                    attributes: vec![VertexAttribute {
                        format: VertexFormat::Float32x3,
                        offset: 0,
                        shader_location: 0,
                    }],
                }],
                ..default()
            },
            primitive: PrimitiveState {
                cull_mode: Some(Face::Back),
                ..default()
            },
            depth_stencil: Some(DepthStencilState {
                format: TextureFormat::Depth32Float,
                depth_write_enabled: Some(true),
                depth_compare: Some(CompareFunction::GreaterEqual),
                stencil: default(),
                // 0046baa0 negates sm_polygonOffsetBias/Scale for reverse Z.
                // Defaults:8192 (0041f2b0),2.0 (007595b0).
                bias: DepthBiasState {
                    constant: -8192,
                    slope_scale: -2.0,
                    clamp: 0.0,
                },
            }),
            fragment: None,
            ..default()
        }));
    }
    for (entity, views, atlas) in &mut cameras {
        if let Some(mut atlas) = atlas
            && atlas.size == views.size
        {
            if views.is_changed() {
                for (uniform, matrix) in atlas.matrices.iter_mut().zip(views.clip_from_world) {
                    uniform.set(matrix.into());
                    uniform.write_buffer(&device, &queue);
                }
            }
            continue;
        }
        let texture = device.create_texture(&TextureDescriptor {
            label: Some("T6 two-view sun shadow atlas"),
            size: Extent3d {
                width: views.size,
                height: views.size * 2,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::Depth32Float,
            usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let view = texture.create_view(&default());
        let upload = |matrix: Mat4| {
            let mut buffer = UniformBuffer::from(DepthMatrix::from(matrix));
            buffer.write_buffer(&device, &queue);
            buffer
        };
        let matrices = views.clip_from_world.map(upload);
        let layout = cache.get_bind_group_layout(&pipeline.layout);
        let bind = |uniform: &UniformBuffer<DepthMatrix>| {
            device.create_bind_group(
                "native shadow matrix",
                &layout,
                &BindGroupEntries::single(uniform.binding().expect("uploaded shadow matrix")),
            )
        };
        let groups = std::array::from_fn(|i| bind(&matrices[i]));
        let identity = upload(Mat4::IDENTITY);
        let object_group = bind(&identity);
        commands.entity(entity).insert(SunShadowAtlas {
            _texture: texture,
            view,
            size: views.size,
            matrices,
            groups,
            _identity: identity,
            object_group,
        });
    }
}

fn render_sun_depth(
    view: ViewQuery<(&SunShadowAtlas, &ShadowViewSettings)>,
    pipeline: Res<DepthPipeline>,
    cache: Res<PipelineCache>,
    casters: Query<&ShadowCasters>,
    geometries: Res<RenderAssets<PreparedNativeGeometry>>,
    mut ctx: RenderContext,
) {
    let (atlas, settings) = view.into_inner();
    let Some(pipeline) = pipeline
        .pipeline
        .and_then(|id| cache.get_render_pipeline(id))
    else {
        return;
    };
    let mut pass = ctx.begin_tracked_render_pass(RenderPassDescriptor {
        label: Some("T6 sun shadowcaster depth"),
        color_attachments: &[],
        depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
            view: &atlas.view,
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
    if !settings.sun_enabled {
        return;
    }
    pass.set_render_pipeline(pipeline);
    pass.set_bind_group(1, &atlas.object_group, &[]);
    for (index, group) in atlas.groups.iter().enumerate() {
        pass.set_viewport(
            0.,
            (index as u32 * atlas.size) as f32,
            atlas.size as f32,
            atlas.size as f32,
            0.,
            1.,
        );
        pass.set_bind_group(0, group, &[]);
        for caster in &casters {
            let Some(geometry) = geometries.get(caster.geometry.id()) else {
                continue;
            };
            pass.set_index_buffer(geometry.indices.slice(..), IndexFormat::Uint16);
            for draw in caster.draws.iter() {
                pass.set_vertex_buffer(0, geometry.primary.slice(draw.primary_offset..));
                pass.draw_indexed(
                    draw.first_index..draw.first_index + draw.index_count,
                    0,
                    0..1,
                );
            }
        }
    }
}

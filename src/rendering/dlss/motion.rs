//! Object velocities reuse the native renderer's visible, validated draw lists.
use super::DlssInputs;
use crate::rendering::{
    native::{NativeWorldObject, NativeWorldScene, NativeWorldState},
    skinning::PreparedSkin,
};
use bevy::{
    anti_alias::dlss::Dlss,
    core_pipeline::prepass::ViewPrepassTextures,
    mesh::VertexBufferLayout,
    platform::collections::HashMap,
    prelude::*,
    render::{
        render_resource::{binding_types::*, *},
        renderer::{RenderContext, RenderDevice, RenderQueue, ViewQuery},
        view::ExtractedView,
    },
};

#[derive(Clone, Copy, ShaderType)]
struct MotionObject {
    clip: Mat4,
    current: Mat4,
    previous: Mat4,
    reactive: Vec4,
}

struct ObjectHistory {
    frame: u32,
    geometry: (BufferId, u64),
    absolute: Mat4,
    clip: Mat4,
}

struct MotionDraw {
    vertices: Buffer,
    indices: Buffer,
    offset: u64,
    first: u32,
    count: u32,
    skin: Option<Entity>,
    uniform: u32,
    pipeline: CachedRenderPipelineId,
}

#[derive(Clone, Copy, Hash, PartialEq, Eq)]
struct MotionKey {
    stride: u64,
    cull: Option<Face>,
    front: FrontFace,
}

#[derive(Resource)]
pub(crate) struct NativeMotion {
    shader: Handle<Shader>,
    layout: BindGroupLayoutDescriptor,
    pipelines: HashMap<MotionKey, CachedRenderPipelineId>,
    histories: HashMap<Entity, ObjectHistory>,
    objects: DynamicUniformBuffer<MotionObject>,
    draws: Vec<MotionDraw>,
    group: Option<BindGroup>,
    active: bool,
    reset: bool,
    frame: u32,
    eye: Vec3,
    clip: Mat4,
    unjittered: Mat4,
}

fn matrix(rows: [Vec4; 4]) -> Mat4 {
    Mat4::from_cols(rows[0], rows[1], rows[2], rows[3]).transpose()
}

impl NativeMotion {
    pub(crate) fn new(shader: Handle<Shader>) -> Self {
        Self {
            shader,
            layout: BindGroupLayoutDescriptor::new(
                "native object motion",
                &BindGroupLayoutEntries::sequential(
                    ShaderStages::VERTEX_FRAGMENT,
                    (uniform_buffer::<MotionObject>(true), texture_depth_2d()),
                ),
            ),
            pipelines: default(),
            histories: default(),
            objects: default(),
            draws: default(),
            group: None,
            active: false,
            reset: true,
            frame: 0,
            eye: Vec3::ZERO,
            clip: Mat4::IDENTITY,
            unjittered: Mat4::IDENTITY,
        }
    }

    pub(crate) fn begin(
        &mut self,
        view: &ExtractedView,
        scene: &NativeWorldScene,
        jittered: &NativeWorldScene,
        frame: u32,
        reset: bool,
    ) {
        self.active = true;
        self.reset = reset;
        self.frame = frame;
        self.eye = crate::rendering::world::source_frame()
            .inverse()
            .transform_point3(view.world_from_view.translation());
        self.clip = matrix(jittered.view_projection_rows);
        self.unjittered = matrix(scene.view_projection_rows);
        self.draws.clear();
        self.objects.clear();
        self.histories
            .retain(|_, history| history.frame == frame.wrapping_sub(1));
    }

    // Called only after the existing native queues validate geometry/materials.
    pub(crate) fn queue(
        &mut self,
        cache: &PipelineCache,
        entity: Entity,
        object: &NativeWorldObject,
        vertices: &Buffer,
        indices: &Buffer,
        offset: u64,
        first: u32,
        count: u32,
        stride: u64,
        skin: Option<Entity>,
        state: NativeWorldState,
        reactive: bool,
    ) {
        if !self.active || !state.depth_write || state.color_write.is_empty() {
            return;
        }
        let local = matrix(object.world_rows);
        let absolute = Mat4::from_translation(self.eye) * local;
        let current = self.unjittered * local;
        let previous = self.histories.insert(
            entity,
            ObjectHistory {
                frame: self.frame,
                geometry: (vertices.id(), offset),
                absolute,
                clip: current,
            },
        );
        let previous = previous.filter(|previous| {
            !self.reset
                && previous.frame == self.frame.wrapping_sub(1)
                && previous.geometry == (vertices.id(), offset)
        });
        // Static BSP and props already have exact camera velocities from depth.
        if skin.is_none()
            && !reactive
            && previous
                .as_ref()
                .is_some_and(|p| p.absolute.abs_diff_eq(absolute, 0.001))
        {
            return;
        }
        if self.reset && skin.is_none() && !reactive {
            return;
        }
        let fresh = previous.is_none();
        let key = MotionKey {
            stride,
            cull: state.cull_mode,
            front: state.front_face,
        };
        let pipeline = *self.pipelines.entry(key).or_insert_with(|| {
            cache.queue_render_pipeline(RenderPipelineDescriptor {
                label: Some("native object motion and reactive coverage".into()),
                layout: vec![self.layout.clone()],
                vertex: VertexState {
                    shader: self.shader.clone(),
                    entry_point: Some("vertex".into()),
                    buffers: vec![
                        VertexBufferLayout {
                            array_stride: stride,
                            step_mode: VertexStepMode::Vertex,
                            attributes: vec![VertexAttribute {
                                format: VertexFormat::Float32x3,
                                offset: 0,
                                shader_location: 0,
                            }],
                        },
                        VertexBufferLayout {
                            array_stride: stride,
                            step_mode: VertexStepMode::Vertex,
                            attributes: vec![VertexAttribute {
                                format: VertexFormat::Float32x3,
                                offset: 0,
                                shader_location: 1,
                            }],
                        },
                    ],
                    ..default()
                },
                fragment: Some(FragmentState {
                    shader: self.shader.clone(),
                    entry_point: Some("fragment".into()),
                    targets: vec![
                        Some(ColorTargetState {
                            format: TextureFormat::Rg16Float,
                            blend: None,
                            write_mask: ColorWrites::ALL,
                        }),
                        Some(mask_target()),
                        Some(mask_target()),
                    ],
                    ..default()
                }),
                primitive: PrimitiveState {
                    cull_mode: key.cull,
                    front_face: key.front,
                    ..default()
                },
                ..default()
            })
        });
        let uniform = self.objects.push(&MotionObject {
            clip: self.clip * local,
            current,
            previous: previous.map_or(current, |previous| previous.clip),
            reactive: Vec4::new(if reactive || fresh { 1. } else { 0. }, 0., 0., 0.),
        });
        self.draws.push(MotionDraw {
            vertices: vertices.clone(),
            indices: indices.clone(),
            offset,
            first,
            count,
            skin,
            uniform,
            pipeline,
        });
    }
}

fn mask_target() -> ColorTargetState {
    let max = BlendComponent {
        src_factor: BlendFactor::One,
        dst_factor: BlendFactor::One,
        operation: BlendOperation::Max,
    };
    ColorTargetState {
        format: TextureFormat::R8Unorm,
        blend: Some(BlendState {
            color: max,
            alpha: max,
        }),
        write_mask: ColorWrites::ALL,
    }
}

pub(super) fn prepare(
    mut motion: ResMut<NativeMotion>,
    device: Res<RenderDevice>,
    queue: Res<RenderQueue>,
    cache: Res<PipelineCache>,
    views: Query<&ViewPrepassTextures, With<Dlss>>,
) {
    if motion.draws.is_empty() {
        return;
    }
    let Ok(prepass) = views.single() else {
        return;
    };
    let Some(depth) = &prepass.depth else {
        return;
    };
    motion.objects.write_buffer(&device, &queue);
    motion.group = Some(device.create_bind_group(
        "native object motion",
        &cache.get_bind_group_layout(&motion.layout),
        &BindGroupEntries::sequential((
            motion.objects.binding().unwrap(),
            &depth.texture.default_view,
        )),
    ));
}

pub(super) fn render(
    view: ViewQuery<(&ViewPrepassTextures, &DlssInputs), With<Dlss>>,
    motion: Res<NativeMotion>,
    skins: Query<&PreparedSkin>,
    cache: Res<PipelineCache>,
    mut ctx: RenderContext,
) {
    let (prepass, inputs) = view.into_inner();
    if !inputs.rendered {
        return;
    }
    let (Some(vectors), Some(group)) = (&prepass.motion_vectors, &motion.group) else {
        return;
    };
    if motion.draws.is_empty() {
        return;
    }
    let attachments = [
        &vectors.texture.default_view,
        &inputs.bias.default_view,
        &inputs.reactive[inputs.index].default_view,
    ]
    .map(|view| {
        Some(RenderPassColorAttachment {
            view,
            depth_slice: None,
            resolve_target: None,
            ops: Operations {
                load: LoadOp::Load,
                store: StoreOp::Store,
            },
        })
    });
    let mut pass = ctx
        .command_encoder()
        .begin_render_pass(&RenderPassDescriptor {
            label: Some("native moving surface velocities"),
            color_attachments: &attachments,
            ..default()
        });
    pass.set_viewport(0., 0., inputs.size.x as f32, inputs.size.y as f32, 0., 1.);
    for draw in &motion.draws {
        let Some(pipeline) = cache.get_render_pipeline(draw.pipeline) else {
            continue;
        };
        let previous = if let Some(entity) = draw.skin {
            let Ok(skin) = skins.get(entity) else {
                continue;
            };
            if !skin.ready {
                continue;
            }
            if skin.previous_pose_valid {
                skin.previous_vertices.as_ref().unwrap_or(&draw.vertices)
            } else {
                &draw.vertices
            }
        } else {
            &draw.vertices
        };
        pass.set_pipeline(pipeline);
        pass.set_bind_group(0, group, &[draw.uniform]);
        pass.set_vertex_buffer(0, *draw.vertices.slice(draw.offset..));
        pass.set_vertex_buffer(1, *previous.slice(draw.offset..));
        pass.set_index_buffer(*draw.indices.slice(..), IndexFormat::Uint16);
        pass.draw_indexed(draw.first..draw.first + draw.count, 0, 0..1);
    }
}

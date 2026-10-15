//! Render-world ownership of per-instance native skin outputs.
use super::animated_models::{AnimatedModel, AnimatedModelInstance, NativeSkinPalette};
use bevy::{
    asset::{RenderAssetUsages, load_embedded_asset},
    core_pipeline::schedule::camera_driver,
    ecs::system::{SystemParamItem, lifetimeless::SRes},
    prelude::*,
    render::{
        Extract, ExtractSchedule, Render, RenderApp, RenderStartup, RenderSystems,
        render_asset::{
            AssetExtractionError, PrepareAssetError, RenderAsset, RenderAssetPlugin, RenderAssets,
        },
        render_resource::{binding_types::*, *},
        renderer::{RenderContext, RenderDevice, RenderGraph, RenderQueue},
        sync_component::{SyncComponent, SyncComponentPlugin},
        sync_world::RenderEntity,
    },
};

#[derive(Clone, Copy, ShaderType)]
pub(crate) struct SkinRange {
    pub first_vertex: u32,
    pub vertex_count: u32,
    pub first_blend_u16: u32,
    pub influence_count: u32,
    pub rigid_bone: u32,
    pub matrix_offset: u32,
    pub output_vertex: u32,
    pub padding: u32,
}

pub(crate) struct NativeSkinningPlugin;
impl Plugin for NativeSkinningPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            RenderAssetPlugin::<PreparedAnimatedModel>::default(),
            SyncComponentPlugin::<AnimatedModelInstance>::default(),
        ));
        let shader = load_embedded_asset!(app, "native_skinning.wesl");
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };
        render_app
            .insert_resource(SkinShader(shader))
            .add_systems(RenderStartup, init_pipeline)
            .add_systems(ExtractSchedule, extract_instances)
            .add_systems(
                Render,
                prepare_instances.in_set(RenderSystems::PrepareBindGroups),
            )
            .add_systems(RenderGraph, skin.before(camera_driver));
    }
}

#[derive(Resource)]
struct SkinShader(Handle<Shader>);

#[derive(Resource)]
pub(crate) struct SkinningPipeline {
    layout: BindGroupLayoutDescriptor,
    pipeline: CachedComputePipelineId,
}

fn init_pipeline(mut commands: Commands, shader: Res<SkinShader>, cache: Res<PipelineCache>) {
    let layout = BindGroupLayoutDescriptor::new(
        "native skinning",
        &BindGroupLayoutEntries::sequential(
            ShaderStages::COMPUTE,
            (
                storage_buffer_read_only::<Vec<u32>>(false),
                storage_buffer_read_only::<Vec<u32>>(false),
                storage_buffer_read_only::<Vec<Mat4>>(false),
                storage_buffer::<Vec<u32>>(false),
                uniform_buffer::<SkinRange>(true),
            ),
        ),
    );
    let pipeline = cache.queue_compute_pipeline(ComputePipelineDescriptor {
        label: Some("native packed model skinning".into()),
        layout: vec![layout.clone()],
        shader: shader.0.clone(),
        entry_point: Some("skin".into()),
        ..default()
    });
    commands.insert_resource(SkinningPipeline { layout, pipeline });
}

/// Attach to the render entity of one animated model instance. The output and
/// palette live exactly as long as that entity; source buffers are shared assets.
#[derive(Component)]
pub(crate) struct PreparedSkin {
    pub vertices: Buffer,
    pub ready: bool,
    matrices: Buffer,
    matrix_count: usize,
    palette_bytes: Vec<u8>,
    group: BindGroup,
    dispatches: Vec<(u32, u32)>,
    geometry_key: (BufferId, usize),
}

impl PreparedSkin {
    pub(crate) fn matches(&self, model: &PreparedAnimatedModel, lod: usize) -> bool {
        self.geometry_key == (model.vertices.id(), lod)
    }

    pub(crate) fn new(
        device: &RenderDevice,
        queue: &RenderQueue,
        cache: &PipelineCache,
        pipeline: &SkinningPipeline,
        source: &Buffer,
        blends: &Buffer,
        ranges: &[SkinRange],
        matrix_count: usize,
        vertex_count: u32,
        lod: usize,
    ) -> Self {
        assert!(matrix_count > 0 && vertex_count > 0 && !ranges.is_empty());
        let matrices = device.create_buffer(&BufferDescriptor {
            label: Some("native instance bone palette"),
            size: matrix_count as u64 * 64,
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let vertices = device.create_buffer(&BufferDescriptor {
            label: Some("native instance skinned PACKED32"),
            size: u64::from(vertex_count) * 32,
            usage: BufferUsages::STORAGE | BufferUsages::VERTEX,
            mapped_at_creation: false,
        });
        let mut uniforms = DynamicUniformBuffer::default();
        let mut dispatches = Vec::with_capacity(ranges.len());
        for range in ranges {
            assert!(range.influence_count <= 4 && range.vertex_count > 0);
            assert!(
                u64::from(range.first_vertex) + u64::from(range.vertex_count) <= source.size() / 32
            );
            assert!(
                u64::from(range.output_vertex) + u64::from(range.vertex_count)
                    <= u64::from(vertex_count)
            );
            if range.influence_count == 0 {
                assert!(
                    u64::from(range.matrix_offset) + u64::from(range.rigid_bone)
                        < matrix_count as u64
                );
            } else {
                let end = u64::from(range.first_blend_u16)
                    + u64::from(range.vertex_count) * u64::from(2 * range.influence_count - 1);
                assert!(end * 2 <= blends.size());
            }
            dispatches.push((uniforms.push(range), range.vertex_count.div_ceil(64)));
        }
        uniforms.write_buffer(device, queue);
        let group = device.create_bind_group(
            "native instance skinning",
            &cache.get_bind_group_layout(&pipeline.layout),
            &BindGroupEntries::sequential((
                source.as_entire_binding(),
                blends.as_entire_binding(),
                matrices.as_entire_binding(),
                vertices.as_entire_binding(),
                uniforms.binding().unwrap(),
            )),
        );
        Self {
            geometry_key: (source.id(), lod),
            vertices,
            ready: false,
            matrices,
            matrix_count,
            palette_bytes: Vec::with_capacity(matrix_count * 64),
            group,
            dispatches,
        }
    }

    /// The animation extractor updates this only when the instance pose changes.
    pub(crate) fn set_palette(&mut self, matrices: &[Mat4], queue: &RenderQueue) {
        assert_eq!(matrices.len(), self.matrix_count);
        self.palette_bytes.clear();
        for matrix in matrices {
            assert!(
                matrix.is_finite(),
                "native skin bone palette must be finite"
            );
            for value in matrix.to_cols_array() {
                self.palette_bytes.extend_from_slice(&value.to_le_bytes());
            }
        }
        queue.write_buffer(&self.matrices, 0, &self.palette_bytes);
        self.ready = false;
    }
}

fn skin(
    mut context: RenderContext,
    cache: Res<PipelineCache>,
    pipeline: Res<SkinningPipeline>,
    mut instances: Query<(&ExtractedSkin, &mut PreparedSkin)>,
) {
    let Some(pipeline) = cache.get_compute_pipeline(pipeline.pipeline) else {
        return;
    };
    if instances.iter().all(|(input, instance)| {
        !input.visible || instance.ready || instance.palette_bytes.is_empty()
    }) {
        return;
    }
    let mut pass = context
        .command_encoder()
        .begin_compute_pass(&ComputePassDescriptor::default());
    pass.set_pipeline(pipeline);
    for (input, mut instance) in &mut instances {
        if !input.visible || instance.ready || instance.palette_bytes.is_empty() {
            continue;
        }
        for &(offset, workgroups) in &instance.dispatches {
            pass.set_bind_group(0, &instance.group, &[offset]);
            pass.dispatch_workgroups(workgroups, 1, 1);
        }
        instance.ready = true;
    }
}

pub(crate) struct SkinLod {
    pub ranges: Vec<SkinRange>,
    pub surface_offsets: Vec<u64>,
    pub vertex_count: u32,
}

pub(crate) struct PreparedAnimatedModel {
    pub vertices: Buffer,
    pub indices: Buffer,
    blends: Buffer,
    pub lods: Vec<SkinLod>,
    bones: usize,
}

impl RenderAsset for PreparedAnimatedModel {
    type SourceAsset = AnimatedModel;
    type Param = SRes<RenderDevice>;

    fn asset_usage(_: &AnimatedModel) -> RenderAssetUsages {
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD
    }

    fn take_gpu_data(
        source: &mut AnimatedModel,
        _: Option<&Self>,
    ) -> Result<AnimatedModel, AssetExtractionError> {
        if source.vertices.is_empty() {
            return Err(AssetExtractionError::AlreadyExtracted);
        }
        Ok(AnimatedModel {
            definition: source.definition.clone(),
            vertices: std::mem::take(&mut source.vertices),
            indices: std::mem::take(&mut source.indices),
            blends: std::mem::take(&mut source.blends),
            bind: Vec::new(),
            materials: source.materials.clone(),
        })
    }

    fn byte_len(source: &AnimatedModel) -> Option<usize> {
        Some(source.vertices.len() + source.indices.len() + source.blends.len())
    }

    fn prepare_asset(
        mut source: AnimatedModel,
        _: AssetId<AnimatedModel>,
        device: &mut SystemParamItem<Self::Param>,
        _: Option<&Self>,
    ) -> Result<Self, PrepareAssetError<AnimatedModel>> {
        let lods = source.definition.lods.iter().map(skin_lod).collect();
        // Storage bindings need at least one word; a rigid model has no blends.
        source
            .blends
            .resize(source.blends.len().max(4).next_multiple_of(4), 0);
        source
            .indices
            .resize(source.indices.len().next_multiple_of(4), 0);
        let upload = |label, contents: &[u8], usage| {
            device.create_buffer_with_data(&BufferInitDescriptor {
                label: Some(label),
                contents,
                usage,
            })
        };
        Ok(Self {
            vertices: upload(
                "native animated source PACKED32",
                &source.vertices,
                BufferUsages::STORAGE,
            ),
            indices: upload(
                "native animated source indices",
                &source.indices,
                BufferUsages::INDEX,
            ),
            blends: upload(
                "native animated source blends",
                &source.blends,
                BufferUsages::STORAGE,
            ),
            lods,
            bones: source.definition.bone_names.len(),
        })
    }
}

fn skin_lod(lod: &super::animated_models::ModelLod) -> SkinLod {
    let mut output = SkinLod {
        ranges: Vec::new(),
        surface_offsets: Vec::new(),
        vertex_count: 0,
    };
    for surface in &lod.surfaces {
        output
            .surface_offsets
            .push(u64::from(output.vertex_count) * 32);
        let start =
            u32::try_from(surface.vertex_byte_offset / 32).expect("native vertex offset fits u32");
        let mut vertex = 0;
        let mut blend =
            u32::try_from(surface.blend_byte_offset / 2).expect("native blend offset fits u32");
        for rigid in &surface.rigid_lists {
            if rigid.vert_count > 0 {
                output.ranges.push(SkinRange {
                    first_vertex: start + vertex,
                    vertex_count: rigid.vert_count,
                    first_blend_u16: 0,
                    influence_count: 0,
                    rigid_bone: rigid.bone_offset / 64,
                    matrix_offset: 0,
                    output_vertex: output.vertex_count + vertex,
                    padding: 0,
                });
            }
            vertex += rigid.vert_count;
        }
        for (i, &count) in surface.blend_vertex_counts.iter().enumerate() {
            let influences = i as u32 + 1;
            if count > 0 {
                output.ranges.push(SkinRange {
                    first_vertex: start + vertex,
                    vertex_count: count,
                    first_blend_u16: blend,
                    influence_count: influences,
                    rigid_bone: 0,
                    matrix_offset: 0,
                    output_vertex: output.vertex_count + vertex,
                    padding: 0,
                });
            }
            vertex += count;
            blend += count * (2 * influences - 1);
        }
        assert_eq!(
            vertex, surface.vertex_count,
            "native skin dispatch coverage"
        );
        output.vertex_count += vertex;
    }
    output
}

#[derive(Component)]
pub(crate) struct ExtractedSkin {
    pub model: AssetId<AnimatedModel>,
    pub lod: usize,
    visible: bool,
    palette: Vec<Mat4>,
}

impl SyncComponent<RenderApp> for AnimatedModelInstance {
    type Target = (ExtractedSkin, PreparedSkin);
}

fn extract_instances(
    mut commands: Commands,
    inputs: Extract<
        Query<(
            RenderEntity,
            Ref<AnimatedModelInstance>,
            Ref<NativeSkinPalette>,
            &InheritedVisibility,
        )>,
    >,
    mut instances: Query<&mut ExtractedSkin>,
) {
    for (entity, input, palette, visibility) in &inputs {
        if let Ok(mut extracted) = instances.get_mut(entity) {
            let became_visible = visibility.get() && !extracted.visible;
            if extracted.visible != visibility.get() {
                extracted.visible = visibility.get();
            }
            if input.is_changed() {
                extracted.model = input.model.id();
                extracted.lod = input.lod;
            }
            if visibility.get() && (palette.is_changed() || became_visible) {
                extracted.palette.clone_from(&palette.0);
            }
        } else {
            commands.entity(entity).insert(ExtractedSkin {
                model: input.model.id(),
                lod: input.lod,
                visible: visibility.get(),
                palette: palette.0.clone(),
            });
        }
    }
}

fn prepare_instances(
    mut commands: Commands,
    device: Res<RenderDevice>,
    queue: Res<RenderQueue>,
    cache: Res<PipelineCache>,
    pipeline: Res<SkinningPipeline>,
    models: Res<RenderAssets<PreparedAnimatedModel>>,
    mut instances: Query<(Entity, Ref<ExtractedSkin>, Option<&mut PreparedSkin>)>,
) {
    for (entity, input, prepared) in &mut instances {
        if !input.visible {
            continue;
        }
        let Some(model) = models.get(input.model) else {
            continue;
        };
        let lod = &model.lods[input.lod];
        if let Some(mut prepared) =
            prepared.filter(|p| p.geometry_key == (model.vertices.id(), input.lod))
        {
            if input.is_changed() {
                prepared.set_palette(&input.palette, &queue);
            }
        } else {
            let mut prepared = PreparedSkin::new(
                &device,
                &queue,
                &cache,
                &pipeline,
                &model.vertices,
                &model.blends,
                &lod.ranges,
                model.bones,
                lod.vertex_count,
                input.lod,
            );
            prepared.set_palette(&input.palette, &queue);
            commands.entity(entity).insert(prepared);
        }
    }
}

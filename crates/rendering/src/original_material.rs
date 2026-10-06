//! General external translated-shader bridge using Bevy Material ownership.
//! Original generated shader sources remain external. Bindings/state are explicit.
use bevy::{
    asset::Asset,
    ecs::system::{SystemParamItem, lifetimeless::SRes},
    image::ImageSamplerDescriptor,
    mesh::MeshVertexBufferLayoutRef,
    pbr::{Material, MaterialPipeline, MaterialPipelineKey, MaterialPlugin},
    prelude::*,
    render::{
        render_asset::RenderAssets, render_resource::*, renderer::RenderDevice, texture::GpuImage,
    },
    shader::Shader,
};
use std::{collections::BTreeSet, num::NonZeroU64, sync::Arc};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OriginalMaterialError {
    InvalidDescriptor,
    DuplicateBinding,
    MissingBinding,
    WrongResource,
    InvalidUniform,
    InvalidSampler,
    DeviceLimit,
    InvalidAttributes,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum BindingKind {
    Uniform {
        minimum_size: u64,
    },
    Texture {
        dimension: TextureViewDimension,
        sample_type: TextureSampleType,
    },
    Sampler {
        kind: SamplerBindingType,
    },
}
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct BindingSpec {
    pub binding: u32,
    pub visibility: ShaderStages,
    pub kind: BindingKind,
}
impl BindingSpec {
    pub fn layout_entry(&self) -> BindGroupLayoutEntry {
        BindGroupLayoutEntry {
            binding: self.binding,
            visibility: self.visibility,
            count: None,
            ty: match self.kind {
                BindingKind::Uniform { minimum_size } => BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: NonZeroU64::new(minimum_size),
                },
                BindingKind::Texture {
                    dimension,
                    sample_type,
                } => BindingType::Texture {
                    sample_type,
                    view_dimension: dimension,
                    multisampled: false,
                },
                BindingKind::Sampler { kind } => BindingType::Sampler(kind),
            },
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SourceAttribute {
    Position3,
    Position4,
    Color4,
    Uv0,
    Normal3,
    Normal4,
    Tangent3,
    Tangent4,
    Uv1,
    SecondaryTexcoord0,
    SecondaryTexcoord1,
    SecondaryNormalTransform0,
    SecondaryNormalTransform1,
}
impl SourceAttribute {
    pub fn components(self) -> u8 {
        match self {
            Self::Uv0 | Self::Uv1 => 2,
            Self::Position3 | Self::Normal3 | Self::Tangent3 => 3,
            _ => 4,
        }
    }
    pub fn attribute(self) -> bevy::mesh::MeshVertexAttribute {
        use bevy::mesh::MeshVertexAttribute;
        let (name, id, format) = match self {
            Self::Position3 => (
                "OriginalSourcePosition3",
                20_000_009,
                VertexFormat::Float32x3,
            ),
            Self::Position4 => (
                "OriginalSourcePosition4",
                20_000_001,
                VertexFormat::Float32x4,
            ),
            Self::Color4 => ("OriginalSourceColor4", 20_000_002, VertexFormat::Float32x4),
            Self::Uv0 => ("OriginalSourceUv0", 20_000_003, VertexFormat::Float32x2),
            Self::Normal3 => (
                "OriginalSourceNormalInput3",
                20_000_004,
                VertexFormat::Float32x3,
            ),
            Self::Normal4 => (
                "OriginalSourceNormalInput4",
                20_000_005,
                VertexFormat::Float32x4,
            ),
            Self::Tangent3 => (
                "OriginalSourceTangentInput3",
                20_000_006,
                VertexFormat::Float32x3,
            ),
            Self::Tangent4 => (
                "OriginalSourceTangentInput4",
                20_000_007,
                VertexFormat::Float32x4,
            ),
            Self::Uv1 => (
                "OriginalSourceLightmapUv",
                20_000_008,
                VertexFormat::Float32x2,
            ),
            Self::SecondaryTexcoord0 => (
                "OriginalSourceSecondaryTexcoord0",
                20_000_010,
                VertexFormat::Float32x4,
            ),
            Self::SecondaryTexcoord1 => (
                "OriginalSourceSecondaryTexcoord1",
                20_000_011,
                VertexFormat::Float32x4,
            ),
            Self::SecondaryNormalTransform0 => (
                "OriginalSourceSecondaryNormalTransform0",
                20_000_012,
                VertexFormat::Float32x4,
            ),
            Self::SecondaryNormalTransform1 => (
                "OriginalSourceSecondaryNormalTransform1",
                20_000_013,
                VertexFormat::Float32x4,
            ),
        };
        MeshVertexAttribute::new(name, id, format)
    }
}

/// Combines hash-admitted external shader declarations. Attribute routing must
/// come from the qualified source vertex declaration; register numbers alone
/// do not establish semantics.
pub fn translated_descriptor(
    vertex: &bo2_assets::shader_bank::ShaderDeclaration,
    fragment: &bo2_assets::shader_bank::ShaderDeclaration,
    vertex_shader: Handle<Shader>,
    fragment_shader: Handle<Shader>,
    attributes: Vec<VertexInput>,
    raster: OriginalRasterState,
) -> Result<OriginalMaterialDescriptor, OriginalMaterialError> {
    use bo2_assets::shader_bank::{ResourceKind, ShaderStage, TextureDimension, TextureSample};
    if vertex.stage != ShaderStage::Vertex
        || fragment.stage != ShaderStage::Fragment
        || attributes.len() != vertex.inputs.len()
        || vertex.inputs.iter().any(|input| {
            !attributes.iter().any(|a| {
                a.location == input.location && a.attribute.components() == input.components
            })
        })
    {
        return Err(OriginalMaterialError::InvalidAttributes);
    }
    let mut bindings: Vec<BindingSpec> = Vec::new();
    for (declaration, visibility) in [
        (vertex, ShaderStages::VERTEX),
        (fragment, ShaderStages::FRAGMENT),
    ] {
        for resource in &declaration.resources {
            let kind = match resource.kind {
                ResourceKind::Uniform { size } => BindingKind::Uniform {
                    minimum_size: u64::from(size),
                },
                ResourceKind::Texture { dimension, sample } => BindingKind::Texture {
                    dimension: match dimension {
                        TextureDimension::D2 => TextureViewDimension::D2,
                        TextureDimension::Cube => TextureViewDimension::Cube,
                    },
                    sample_type: match sample {
                        TextureSample::Float => TextureSampleType::Float { filterable: true },
                        TextureSample::Depth => TextureSampleType::Depth,
                    },
                },
                ResourceKind::Sampler { comparison } => BindingKind::Sampler {
                    kind: if comparison {
                        SamplerBindingType::Comparison
                    } else {
                        SamplerBindingType::Filtering
                    },
                },
            };
            if let Some(existing) = bindings.iter_mut().find(|s| s.binding == resource.binding) {
                if existing.kind != kind {
                    return Err(OriginalMaterialError::WrongResource);
                }
                existing.visibility |= visibility;
            } else {
                bindings.push(BindingSpec {
                    binding: resource.binding,
                    visibility,
                    kind,
                });
            }
        }
    }
    bindings.sort_by_key(|s| s.binding);
    let descriptor = OriginalMaterialDescriptor {
        replay: None,
        vertex_shader,
        fragment_shader,
        vertex_entry: vertex.entry_point.clone(),
        fragment_entry: fragment.entry_point.clone(),
        bindings,
        attributes,
        raster,
    };
    descriptor.validate()?;
    Ok(descriptor)
}
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct VertexInput {
    pub attribute: SourceAttribute,
    pub location: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct OriginalRasterState {
    pub front_face: FrontFace,
    pub cull_mode: Option<Face>,
    pub depth_write: bool,
    pub depth_compare: CompareFunction,
    pub blend: Option<BlendState>,
    pub color_writes: ColorWrites,
}
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct OriginalMaterialDescriptor {
    pub replay: Option<OriginalReplayState>,
    pub vertex_shader: Handle<Shader>,
    pub fragment_shader: Handle<Shader>,
    pub vertex_entry: String,
    pub fragment_entry: String,
    pub bindings: Vec<BindingSpec>,
    pub attributes: Vec<VertexInput>,
    pub raster: OriginalRasterState,
}
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct OriginalReplayState {
    pub depth_bias: i32,
    pub slope_bias_bits: u32,
    pub clamp_bits: u32,
}
impl OriginalMaterialDescriptor {
    pub fn validate(&self) -> Result<(), OriginalMaterialError> {
        use OriginalMaterialError as E;
        if self.replay.as_ref().is_some_and(|r| {
            !f32::from_bits(r.slope_bias_bits).is_finite()
                || !f32::from_bits(r.clamp_bits).is_finite()
        }) {
            return Err(E::InvalidDescriptor);
        }
        if !self.vertex_shader.is_strong()
            || !self.fragment_shader.is_strong()
            || self.bindings.len() > 64
            || self.attributes.is_empty()
            || self.attributes.len() > 16
            || [&self.vertex_entry, &self.fragment_entry].iter().any(|s| {
                s.is_empty()
                    || s.len() > 128
                    || !s.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
            })
        {
            return Err(E::InvalidDescriptor);
        }
        let mut seen = BTreeSet::new();
        for spec in &self.bindings {
            if !seen.insert(spec.binding) {
                return Err(E::DuplicateBinding);
            }
            if spec.binding > 255
                || spec.visibility.is_empty()
                || spec.visibility.intersects(ShaderStages::COMPUTE)
            {
                return Err(E::InvalidDescriptor);
            }
            if let BindingKind::Uniform { minimum_size } = spec.kind
                && (minimum_size == 0 || minimum_size > 65536)
            {
                return Err(E::InvalidUniform);
            }
        }
        let mut locations = BTreeSet::new();
        for input in &self.attributes {
            if input.location >= 32 || !locations.insert(input.location) {
                return Err(E::InvalidAttributes);
            }
        }
        Ok(())
    }
    pub fn layout(&self) -> BindGroupLayoutDescriptor {
        BindGroupLayoutDescriptor {
            label: "original translated material".into(),
            entries: self
                .bindings
                .iter()
                .map(BindingSpec::layout_entry)
                .collect(),
        }
    }
    pub fn validate_limits(&self, limits: &WgpuLimits) -> Result<(), OriginalMaterialError> {
        use OriginalMaterialError as E;
        self.validate()?;
        if self.bindings.len() > limits.max_bindings_per_bind_group as usize
            || self.attributes.len() > limits.max_vertex_attributes as usize
        {
            return Err(E::DeviceLimit);
        }
        for stage in [ShaderStages::VERTEX, ShaderStages::FRAGMENT] {
            let mut counts = [0u32; 3];
            for s in self
                .bindings
                .iter()
                .filter(|s| s.visibility.intersects(stage))
            {
                counts[match s.kind {
                    BindingKind::Uniform { .. } => 0,
                    BindingKind::Texture { .. } => 1,
                    BindingKind::Sampler { .. } => 2,
                }] += 1;
            }
            // Bevy's view/mesh layouts also consume resources. These local checks
            // reject impossible original signatures; final pipeline validation
            // verifies the complete engine + material layout.
            if counts[0] > limits.max_uniform_buffers_per_shader_stage
                || counts[1] > limits.max_sampled_textures_per_shader_stage
                || counts[2] > limits.max_samplers_per_shader_stage
            {
                return Err(E::DeviceLimit);
            }
        }
        Ok(())
    }
}
#[derive(Clone)]
pub enum OriginalResource {
    Uniform {
        binding: u32,
        bytes: Vec<u8>,
    },
    Image {
        binding: u32,
        image: Handle<Image>,
    },
    Sampler {
        binding: u32,
        descriptor: ImageSamplerDescriptor,
    },
}
impl OriginalResource {
    pub fn binding(&self) -> u32 {
        match self {
            Self::Uniform { binding, .. }
            | Self::Image { binding, .. }
            | Self::Sampler { binding, .. } => *binding,
        }
    }
}

#[derive(Asset, TypePath, Clone)]
pub struct OriginalMaterial {
    descriptor: Arc<OriginalMaterialDescriptor>,
    resources: Vec<OriginalResource>,
}
impl OriginalMaterial {
    pub fn new(
        descriptor: OriginalMaterialDescriptor,
        resources: Vec<OriginalResource>,
        limits: &WgpuLimits,
    ) -> Result<Self, OriginalMaterialError> {
        use OriginalMaterialError as E;
        descriptor.validate_limits(limits)?;
        if resources.len() != descriptor.bindings.len() {
            return Err(E::MissingBinding);
        }
        let mut seen = BTreeSet::new();
        for resource in &resources {
            if !seen.insert(resource.binding()) {
                return Err(E::DuplicateBinding);
            }
            let spec = descriptor
                .bindings
                .iter()
                .find(|s| s.binding == resource.binding())
                .ok_or(E::WrongResource)?;
            match (&spec.kind, resource) {
                (
                    BindingKind::Uniform { minimum_size },
                    OriginalResource::Uniform { bytes, .. },
                ) => {
                    if bytes.is_empty()
                        || !bytes.len().is_multiple_of(16)
                        || bytes.len() < *minimum_size as usize
                        || bytes.len() > limits.max_uniform_buffer_binding_size as usize
                    {
                        return Err(E::InvalidUniform);
                    }
                }
                (BindingKind::Texture { .. }, OriginalResource::Image { image, .. })
                    if image.is_strong() => {}
                (BindingKind::Sampler { kind }, OriginalResource::Sampler { descriptor, .. }) => {
                    if !descriptor.lod_min_clamp.is_finite()
                        || !descriptor.lod_max_clamp.is_finite()
                        || descriptor.lod_min_clamp < 0.0
                        || descriptor.lod_max_clamp < descriptor.lod_min_clamp
                        || descriptor.anisotropy_clamp == 0
                        || descriptor.anisotropy_clamp > 16
                        || !descriptor.anisotropy_clamp.is_power_of_two()
                        || (descriptor.anisotropy_clamp > 1
                            && (descriptor.min_filter != bevy::image::ImageFilterMode::Linear
                                || descriptor.mag_filter != bevy::image::ImageFilterMode::Linear
                                || descriptor.mipmap_filter
                                    != bevy::image::ImageFilterMode::Linear))
                        || (*kind == SamplerBindingType::Comparison) != descriptor.compare.is_some()
                        || (*kind == SamplerBindingType::NonFiltering
                            && (descriptor.min_filter != bevy::image::ImageFilterMode::Nearest
                                || descriptor.mag_filter != bevy::image::ImageFilterMode::Nearest
                                || descriptor.mipmap_filter
                                    != bevy::image::ImageFilterMode::Nearest))
                    {
                        return Err(E::InvalidSampler);
                    }
                }
                _ => return Err(E::WrongResource),
            }
        }
        Ok(Self {
            descriptor: Arc::new(descriptor),
            resources,
        })
    }
    pub fn descriptor(&self) -> &OriginalMaterialDescriptor {
        &self.descriptor
    }
    /// Call before insertion to reject incompatible images without a GPU
    /// validation panic. Missing images are errors; no engine fallback is used.
    pub fn validate_images(&self, images: &Assets<Image>) -> Result<(), OriginalMaterialError> {
        self.validate_images_with_features(images, WgpuFeatures::empty())
    }
    /// Validation must use enabled device features, not adapter capabilities.
    /// FLOAT32_FILTERABLE permits native linear R32_FLOAT sampled views.
    pub fn validate_images_with_features(
        &self,
        images: &Assets<Image>,
        features: WgpuFeatures,
    ) -> Result<(), OriginalMaterialError> {
        for resource in &self.resources {
            if let OriginalResource::Image { binding, image } = resource {
                let image = images
                    .get(image)
                    .ok_or(OriginalMaterialError::MissingBinding)?;
                let spec = self
                    .descriptor
                    .bindings
                    .iter()
                    .find(|s| s.binding == *binding)
                    .ok_or(OriginalMaterialError::WrongResource)?;
                let BindingKind::Texture {
                    dimension,
                    sample_type,
                } = spec.kind
                else {
                    return Err(OriginalMaterialError::WrongResource);
                };
                let actual_dimension = image
                    .texture_view_descriptor
                    .as_ref()
                    .and_then(|d| d.dimension)
                    .unwrap_or(match image.texture_descriptor.dimension {
                        TextureDimension::D1 => TextureViewDimension::D1,
                        TextureDimension::D2
                            if image.texture_descriptor.size.depth_or_array_layers > 1 =>
                        {
                            TextureViewDimension::D2Array
                        }
                        TextureDimension::D2 => TextureViewDimension::D2,
                        TextureDimension::D3 => TextureViewDimension::D3,
                    });
                let view_format = image
                    .texture_view_descriptor
                    .as_ref()
                    .and_then(|d| d.format)
                    .unwrap_or(image.texture_descriptor.format);
                let actual_sample = view_format.sample_type(None, Some(features));
                let compatible = match (actual_sample, sample_type) {
                    (
                        Some(TextureSampleType::Float {
                            filterable: available,
                        }),
                        TextureSampleType::Float {
                            filterable: required,
                        },
                    ) => !required || available,
                    (Some(actual), required) => actual == required,
                    _ => false,
                };
                if dimension != actual_dimension
                    || !compatible
                    || image.texture_descriptor.sample_count != 1
                {
                    return Err(OriginalMaterialError::WrongResource);
                }
            }
        }
        Ok(())
    }
    pub fn update_uniform(
        &mut self,
        binding: u32,
        bytes: Vec<u8>,
    ) -> Result<(), OriginalMaterialError> {
        let spec = self
            .descriptor
            .bindings
            .iter()
            .find(|s| s.binding == binding)
            .ok_or(OriginalMaterialError::MissingBinding)?;
        let BindingKind::Uniform { minimum_size } = spec.kind else {
            return Err(OriginalMaterialError::WrongResource);
        };
        if bytes.is_empty()
            || !bytes.len().is_multiple_of(16)
            || bytes.len() < minimum_size as usize
            || bytes.len() > 65536
        {
            return Err(OriginalMaterialError::InvalidUniform);
        }
        let resource = self
            .resources
            .iter_mut()
            .find(|r| r.binding() == binding)
            .ok_or(OriginalMaterialError::MissingBinding)?;
        *resource = OriginalResource::Uniform { binding, bytes };
        Ok(())
    }
}
impl AsBindGroup for OriginalMaterial {
    type Data = Arc<OriginalMaterialDescriptor>;
    type Param = SRes<RenderAssets<GpuImage>>;
    fn label() -> &'static str {
        "original translated material"
    }
    fn bind_group_data(&self) -> Self::Data {
        self.descriptor.clone()
    }
    fn bind_group_layout_entries(
        _device: &RenderDevice,
        _no_bindless: bool,
    ) -> Vec<BindGroupLayoutEntry> {
        Vec::new()
    }
    fn unprepared_bind_group(
        &self,
        _layout: &BindGroupLayout,
        _device: &RenderDevice,
        _param: &mut SystemParamItem<Self::Param>,
        _no_bindless: bool,
    ) -> Result<UnpreparedBindGroup, AsBindGroupError> {
        Err(AsBindGroupError::CreateBindGroupDirectly)
    }
    fn as_bind_group(
        &self,
        _layout: &BindGroupLayoutDescriptor,
        device: &RenderDevice,
        cache: &PipelineCache,
        param: &mut SystemParamItem<Self::Param>,
    ) -> Result<PreparedBindGroup, AsBindGroupError> {
        let layout = cache.get_bind_group_layout(&self.descriptor.layout());
        let mut owned = Vec::new();
        for resource in &self.resources {
            let binding = resource.binding();
            let spec = self
                .descriptor
                .bindings
                .iter()
                .find(|s| s.binding == binding)
                .ok_or(AsBindGroupError::RetryNextUpdate)?;
            let value = match resource {
                OriginalResource::Uniform { bytes, .. } => OwnedBindingResource::Buffer(
                    device.create_buffer_with_data(&BufferInitDescriptor {
                        label: Some("original constant buffer"),
                        contents: bytes,
                        usage: BufferUsages::UNIFORM,
                    }),
                ),
                OriginalResource::Image { image, .. } => {
                    let gpu = param
                        .get(image.id())
                        .ok_or(AsBindGroupError::RetryNextUpdate)?;
                    let BindingKind::Texture { dimension, .. } = spec.kind else {
                        return Err(AsBindGroupError::RetryNextUpdate);
                    };
                    OwnedBindingResource::TextureView(dimension, gpu.texture_view.clone())
                }
                OriginalResource::Sampler { descriptor, .. } => {
                    let BindingKind::Sampler { kind } = spec.kind else {
                        return Err(AsBindGroupError::RetryNextUpdate);
                    };
                    OwnedBindingResource::Sampler(
                        kind,
                        device.create_sampler(&descriptor.as_wgpu()),
                    )
                }
            };
            owned.push((binding, value));
        }
        let entries = owned
            .iter()
            .map(|(binding, resource)| BindGroupEntry {
                binding: *binding,
                resource: resource.get_binding(),
            })
            .collect::<Vec<_>>();
        let bind_group = device.create_bind_group(Self::label(), &layout, &entries);
        Ok(PreparedBindGroup {
            bindings: BindingResources(owned),
            bind_group,
        })
    }
}
impl Material for OriginalMaterial {
    fn enable_prepass() -> bool {
        false
    }
    fn enable_shadows() -> bool {
        false
    }
    fn alpha_mode(&self) -> AlphaMode {
        if self.descriptor.replay.is_some() || self.descriptor.raster.blend.is_some() {
            AlphaMode::Blend
        } else {
            AlphaMode::Opaque
        }
    }
    fn specialize(
        _pipeline: &MaterialPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        layout: &MeshVertexBufferLayoutRef,
        key: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        let spec = &key.bind_group_data;
        descriptor.set_layout(3, spec.layout());
        descriptor.vertex.shader = spec.vertex_shader.clone();
        descriptor.vertex.entry_point = Some(spec.vertex_entry.clone().into());
        descriptor.vertex.shader_defs.clear();
        let attributes = spec
            .attributes
            .iter()
            .map(|a| a.attribute.attribute().at_shader_location(a.location))
            .collect::<Vec<_>>();
        descriptor.vertex.buffers = vec![layout.0.get_layout(&attributes)?];
        if let Some(fragment) = &mut descriptor.fragment {
            fragment.shader = spec.fragment_shader.clone();
            fragment.entry_point = Some(spec.fragment_entry.clone().into());
            fragment.shader_defs.clear();
            for target in fragment.targets.iter_mut().flatten() {
                if spec.replay.is_some() {
                    target.format = TextureFormat::Rgba8Unorm;
                }
                target.blend = spec.raster.blend;
                target.write_mask = spec.raster.color_writes;
            }
        }
        descriptor.primitive.front_face = spec.raster.front_face;
        descriptor.primitive.cull_mode = spec.raster.cull_mode;
        if let Some(depth) = &mut descriptor.depth_stencil {
            depth.depth_write_enabled = Some(spec.raster.depth_write);
            depth.depth_compare = Some(spec.raster.depth_compare);
            if let Some(replay) = &spec.replay {
                depth.format = TextureFormat::Depth32Float;
                depth.bias = DepthBiasState {
                    constant: replay.depth_bias,
                    slope_scale: f32::from_bits(replay.slope_bias_bits),
                    clamp: f32::from_bits(replay.clamp_bits),
                };
            }
        }
        if spec.replay.is_some() {
            descriptor.multisample.count = 4;
        }
        Ok(())
    }
}
pub struct OriginalMaterialPlugin;
impl Plugin for OriginalMaterialPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(MaterialPlugin::<OriginalMaterial>::default());
    }
}

/// Mesh POSITION is canonical Bevy space for engine bounds/visibility. Explicit
/// source-input attributes feed the external native shader's original interface.
pub fn original_world_mesh(
    input: &bo2_assets::full_scene::FullSceneInput,
    surface: usize,
) -> Result<Mesh, OriginalMaterialError> {
    use OriginalMaterialError as E;
    let range = input
        .geometry
        .surfaces()
        .get(surface)
        .ok_or(E::InvalidAttributes)?
        .index_range();
    let attributes = input
        .corner_attributes
        .as_ref()
        .ok_or(E::InvalidAttributes)?;
    if range.is_empty()
        || range.len() > 1_000_000
        || input.corner_uv.len() != input.geometry.indices().len()
        || attributes.len() != input.geometry.indices().len()
    {
        return Err(E::InvalidAttributes);
    }
    let mut positions = Vec::new();
    let mut source = Vec::new();
    let mut colors = Vec::new();
    let mut uv0 = Vec::new();
    let mut uv1 = Vec::new();
    let mut normal3 = Vec::new();
    let mut normal4 = Vec::new();
    let mut tangent3 = Vec::new();
    let mut tangent4 = Vec::new();
    for corner in range {
        let a = attributes[corner];
        if !a.position_w.is_finite()
            || a.color
                .into_iter()
                .chain(a.normal_unorm)
                .chain(a.tangent_unorm)
                .chain(a.lightmap_uv)
                .any(|c| !c.is_finite() || !(0.0..=1.0).contains(&c))
            || input.corner_uv[corner].iter().any(|c| !c.is_finite())
        {
            return Err(E::InvalidAttributes);
        }
        let index = input.geometry.indices()[corner] as usize;
        let p = *input
            .geometry
            .positions()
            .get(index)
            .ok_or(E::InvalidAttributes)?;
        positions.push(crate::coordinates::source_to_bevy(p));
        source.push([p[0], p[1], p[2], a.position_w]);
        colors.push(a.color);
        uv0.push(input.corner_uv[corner]);
        uv1.push(a.lightmap_uv);
        normal3.push([a.normal_unorm[0], a.normal_unorm[1], a.normal_unorm[2]]);
        normal4.push(a.normal_unorm);
        tangent3.push([a.tangent_unorm[0], a.tangent_unorm[1], a.tangent_unorm[2]]);
        tangent4.push(a.tangent_unorm);
    }
    let mut mesh = Mesh::new(
        bevy::mesh::PrimitiveTopology::TriangleList,
        bevy::asset::RenderAssetUsages::MAIN_WORLD | bevy::asset::RenderAssetUsages::RENDER_WORLD,
    );
    let count = positions.len();
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(SourceAttribute::Position4.attribute(), source);
    mesh.insert_attribute(SourceAttribute::Color4.attribute(), colors);
    mesh.insert_attribute(SourceAttribute::Uv0.attribute(), uv0);
    mesh.insert_attribute(SourceAttribute::Uv1.attribute(), uv1);
    mesh.insert_attribute(SourceAttribute::Normal3.attribute(), normal3);
    mesh.insert_attribute(SourceAttribute::Normal4.attribute(), normal4);
    mesh.insert_attribute(SourceAttribute::Tangent3.attribute(), tangent3);
    mesh.insert_attribute(SourceAttribute::Tangent4.attribute(), tangent4);
    mesh.insert_indices(bevy::mesh::Indices::U32((0..count as u32).collect()));
    Ok(mesh)
}

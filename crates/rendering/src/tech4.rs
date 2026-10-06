//! Qualified TECH4 mathematical response on explicit float attributes and globals.
//! Original packed-data, technique dispatch and runtime global admission are separate.
use bevy::{
    asset::RenderAssetUsages,
    asset::embedded_asset,
    image::{ImageAddressMode, ImageFilterMode, ImageSampler, ImageSamplerDescriptor},
    mesh::{MeshVertexAttribute, MeshVertexBufferLayoutRef, PrimitiveTopology},
    pbr::{Material, MaterialPipeline, MaterialPipelineKey, MaterialPlugin},
    prelude::*,
    render::render_resource::{
        AsBindGroup, Extent3d, Face, FrontFace, RenderPipelineDescriptor, ShaderType,
        SpecializedMeshPipelineError, TextureDimension, TextureFormat, TextureViewDescriptor,
        TextureViewDimension, VertexFormat,
    },
    shader::ShaderRef,
};

pub const ATTRIBUTE_FOG: MeshVertexAttribute = MeshVertexAttribute::new(
    "Tech4FogColorTransmittance",
    1_004_001,
    VertexFormat::Float32x4,
);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tech4Error {
    InvalidGlobals,
    InvalidAttributes,
    MissingBinding,
    UnsupportedTransfer,
}

#[derive(Clone, Copy, Debug, ShaderType)]
pub struct Tech4Globals {
    pub sh0: Vec4,
    pub sh1: Vec4,
    pub sh2: Vec4,
    pub hdr_scale: f32,
}
impl Tech4Globals {
    pub fn validate(&self) -> Result<(), Tech4Error> {
        if !self.sh0.is_finite()
            || !self.sh1.is_finite()
            || !self.sh2.is_finite()
            || !self.hdr_scale.is_finite()
            || self.hdr_scale < 0.0
        {
            return Err(Tech4Error::InvalidGlobals);
        }
        Ok(())
    }
}

#[derive(Clone)]
pub struct Tech4Textures {
    pub base_color: Handle<Image>,
    pub normal: Handle<Image>,
    pub specular_gloss: Handle<Image>,
    pub lightmap: Handle<Image>,
    pub probe: Handle<Image>,
}

/// Explicit native vertex-fog runtime inputs; values remain in source basis.
#[derive(Clone, Copy, Debug, Default, ShaderType)]
pub struct Tech4Fog {
    pub color: Vec4,
    pub constants: Vec4,
    pub constants2: Vec4,
    pub sun_direction: Vec4,
    pub sun_color: Vec4,
    pub sun_parameters: Vec4,
    pub enabled: f32,
}
impl Tech4Fog {
    pub fn validate(&self) -> Result<(), Tech4Error> {
        if [
            self.color,
            self.constants,
            self.constants2,
            self.sun_direction,
            self.sun_color,
            self.sun_parameters,
        ]
        .iter()
        .any(|v| !v.is_finite())
            || !matches!(self.enabled, 0.0 | 1.0)
        {
            return Err(Tech4Error::InvalidGlobals);
        }
        Ok(())
    }
}
/// Explicit capture-only clip mapping for an isolated original draw comparison.
#[derive(Clone, Copy, Debug, Default, ShaderType)]
pub struct CapturedClip {
    pub view_projection: Mat4,
    pub camera_source: Vec3,
    pub enabled: f32,
    pub viewport_depth: Vec2,
}
impl CapturedClip {
    pub fn validate(&self) -> Result<(), Tech4Error> {
        if !self.view_projection.is_finite()
            || !self.camera_source.is_finite()
            || !matches!(self.enabled, 0.0 | 1.0)
            || !self.viewport_depth.is_finite()
            || self.viewport_depth.x < 0.0
            || self.viewport_depth.y > 1.0
            || self.viewport_depth.y < self.viewport_depth.x
        {
            return Err(Tech4Error::InvalidGlobals);
        }
        Ok(())
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Tech4MaterialKey {
    pub original_raster: bool,
}

#[derive(Asset, TypePath, AsBindGroup, Clone)]
#[bind_group_data(Tech4MaterialKey)]
pub struct Tech4Material {
    #[uniform(0)]
    pub globals: Tech4Globals,
    #[uniform(11)]
    pub fog: Tech4Fog,
    #[uniform(12)]
    pub captured_clip: CapturedClip,
    pub original_raster: bool,
    #[texture(1)]
    #[sampler(2)]
    pub base_color: Handle<Image>,
    #[texture(3)]
    #[sampler(4)]
    pub normal: Handle<Image>,
    #[texture(5)]
    #[sampler(6)]
    pub specular_gloss: Handle<Image>,
    #[texture(7)]
    #[sampler(8)]
    pub lightmap: Handle<Image>,
    #[texture(9, dimension = "cube")]
    #[sampler(10)]
    pub probe: Handle<Image>,
}
impl Tech4Material {
    pub fn new(globals: Tech4Globals, textures: Tech4Textures) -> Result<Self, Tech4Error> {
        globals.validate()?;
        if [
            &textures.base_color,
            &textures.normal,
            &textures.specular_gloss,
            &textures.lightmap,
            &textures.probe,
        ]
        .iter()
        .any(|h| !h.is_strong())
        {
            return Err(Tech4Error::MissingBinding);
        }
        Ok(Self {
            globals,
            fog: Tech4Fog::default(),
            captured_clip: CapturedClip::default(),
            original_raster: false,
            base_color: textures.base_color,
            normal: textures.normal,
            specular_gloss: textures.specular_gloss,
            lightmap: textures.lightmap,
            probe: textures.probe,
        })
    }
    /// Captured opaque TECH4 raster contract; caller supplies all native fog globals.
    /// This constructor is not a proof of surface/pass/attribute provenance.
    pub fn new_original(
        globals: Tech4Globals,
        textures: Tech4Textures,
        mut fog: Tech4Fog,
    ) -> Result<Self, Tech4Error> {
        fog.enabled = 1.0;
        fog.validate()?;
        let mut material = Self::new(globals, textures)?;
        material.fog = fog;
        material.original_raster = true;
        Ok(material)
    }
    pub fn with_captured_clip(mut self, mut clip: CapturedClip) -> Result<Self, Tech4Error> {
        if !self.original_raster {
            return Err(Tech4Error::UnsupportedTransfer);
        }
        clip.enabled = 1.0;
        clip.validate()?;
        self.captured_clip = clip;
        Ok(self)
    }
}
impl From<&Tech4Material> for Tech4MaterialKey {
    fn from(material: &Tech4Material) -> Self {
        Self {
            original_raster: material.original_raster,
        }
    }
}
impl Material for Tech4Material {
    fn vertex_shader() -> ShaderRef {
        "embedded://bo2_rendering/tech4.wgsl".into()
    }
    fn fragment_shader() -> ShaderRef {
        "embedded://bo2_rendering/tech4.wgsl".into()
    }
    fn enable_prepass() -> bool {
        false
    }
    fn enable_shadows() -> bool {
        false
    }
    fn specialize(
        _pipeline: &MaterialPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        layout: &MeshVertexBufferLayoutRef,
        key: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        descriptor.vertex.buffers = vec![layout.0.get_layout(&[
            Mesh::ATTRIBUTE_POSITION.at_shader_location(0),
            Mesh::ATTRIBUTE_NORMAL.at_shader_location(1),
            Mesh::ATTRIBUTE_UV_0.at_shader_location(2),
            Mesh::ATTRIBUTE_UV_1.at_shader_location(3),
            Mesh::ATTRIBUTE_TANGENT.at_shader_location(4),
            Mesh::ATTRIBUTE_COLOR.at_shader_location(5),
            ATTRIBUTE_FOG.at_shader_location(6),
        ])?];
        if key.bind_group_data.original_raster {
            descriptor.primitive.front_face = FrontFace::Cw;
            descriptor.primitive.cull_mode = Some(Face::Back);
        } else {
            descriptor.primitive.cull_mode = None;
        }
        Ok(())
    }
}

pub struct Tech4MaterialPlugin;
impl Plugin for Tech4MaterialPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "tech4.wgsl");
        app.add_plugins(MaterialPlugin::<Tech4Material>::default());
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Tech4Vertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub tangent: [f32; 3],
    pub handedness: f32,
    pub color: [f32; 4],
    pub uv: [f32; 2],
    pub lightmap_uv: [f32; 2],
    /// Explicit source vertex-stage output (RGB fog color, transmittance).
    pub fog_color_transmittance: [f32; 4],
}

/// Source-space triangle corners -> canonical Bevy Mesh; never guesses packing.
pub fn tech4_mesh(corners: &[Tech4Vertex]) -> Result<Mesh, Tech4Error> {
    if corners.is_empty() || !corners.len().is_multiple_of(3) || corners.len() > 1_000_000 {
        return Err(Tech4Error::InvalidAttributes);
    }
    for v in corners {
        if v.position
            .iter()
            .chain(v.normal.iter())
            .chain(v.tangent.iter())
            .chain(v.color.iter())
            .chain(v.uv.iter())
            .chain(v.lightmap_uv.iter())
            .chain(v.fog_color_transmittance.iter())
            .any(|f| !f.is_finite())
            || !matches!(v.handedness, -1.0 | 1.0)
            || Vec3::from_array(v.normal).length_squared() < 1e-12
            || Vec3::from_array(v.tangent).length_squared() < 1e-12
            || !(0.0..=1.0).contains(&v.fog_color_transmittance[3])
        {
            return Err(Tech4Error::InvalidAttributes);
        }
    }
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    );
    mesh.insert_attribute(
        Mesh::ATTRIBUTE_POSITION,
        corners
            .iter()
            .map(|v| crate::coordinates::source_to_bevy(v.position))
            .collect::<Vec<_>>(),
    );
    mesh.insert_attribute(
        Mesh::ATTRIBUTE_NORMAL,
        corners
            .iter()
            .map(|v| crate::coordinates::source_to_bevy(v.normal))
            .collect::<Vec<_>>(),
    );
    mesh.insert_attribute(
        Mesh::ATTRIBUTE_TANGENT,
        corners
            .iter()
            .map(|v| {
                let t = crate::coordinates::source_to_bevy(v.tangent);
                [t[0], t[1], t[2], v.handedness]
            })
            .collect::<Vec<_>>(),
    );
    mesh.insert_attribute(
        Mesh::ATTRIBUTE_UV_0,
        corners.iter().map(|v| v.uv).collect::<Vec<_>>(),
    );
    mesh.insert_attribute(
        Mesh::ATTRIBUTE_UV_1,
        corners.iter().map(|v| v.lightmap_uv).collect::<Vec<_>>(),
    );
    mesh.insert_attribute(
        Mesh::ATTRIBUTE_COLOR,
        corners.iter().map(|v| v.color).collect::<Vec<_>>(),
    );
    mesh.insert_attribute(
        ATTRIBUTE_FOG,
        corners
            .iter()
            .map(|v| v.fog_color_transmittance)
            .collect::<Vec<_>>(),
    );
    mesh.insert_indices(bevy::mesh::Indices::U32(
        (0..corners.len() as u32).collect(),
    ));
    Ok(mesh)
}

pub type Tech4Fixture = (Handle<Mesh>, Handle<Tech4Material>, Vec<Handle<Image>>);

/// Project-owned triangle, explicit synthetic globals, linear 2D images and cube.
/// Parent may spawn Mesh3d/MeshMaterial3d with the returned strong handles.
pub fn synthetic_fixture(world: &mut World) -> Result<Tech4Fixture, Tech4Error> {
    let corners =
        [[-1.0, -1.0, 0.0], [1.0, -1.0, 0.0], [0.0, 1.0, 0.0]].map(|position| Tech4Vertex {
            position,
            normal: [0.0, 0.0, 1.0],
            tangent: [1.0, 0.0, 0.0],
            handedness: 1.0,
            color: [1.0; 4],
            uv: [0.5; 2],
            lightmap_uv: [0.5; 2],
            fog_color_transmittance: [0.0, 0.0, 0.0, 1.0],
        });
    let mesh = tech4_mesh(&corners)?;
    let mut make = |height: u32, layers: u32, bytes: Vec<u8>| {
        let mut image = Image::new(
            Extent3d {
                width: 1,
                height,
                depth_or_array_layers: layers,
            },
            TextureDimension::D2,
            bytes,
            TextureFormat::Rgba8Unorm,
            RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
        );
        image.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
            address_mode_u: ImageAddressMode::ClampToEdge,
            address_mode_v: ImageAddressMode::ClampToEdge,
            address_mode_w: ImageAddressMode::ClampToEdge,
            mag_filter: ImageFilterMode::Nearest,
            min_filter: ImageFilterMode::Nearest,
            mipmap_filter: ImageFilterMode::Nearest,
            ..default()
        });
        if layers == 6 {
            image.texture_view_descriptor = Some(TextureViewDescriptor {
                dimension: Some(TextureViewDimension::Cube),
                ..default()
            });
        }
        world.resource_mut::<Assets<Image>>().add(image)
    };
    let base_color = make(1, 1, vec![128, 64, 32, 255]);
    let normal = make(1, 1, vec![128, 128, 0, 255]);
    let specular_gloss = make(1, 1, vec![0, 0, 0, 0]);
    let lightmap = make(
        3,
        1,
        vec![128, 128, 128, 255, 0, 0, 0, 255, 128, 0, 128, 255],
    );
    let probe = make(1, 6, [0, 0, 0, 255].repeat(6));
    let images = vec![
        base_color.clone(),
        normal.clone(),
        specular_gloss.clone(),
        lightmap.clone(),
        probe.clone(),
    ];
    let material = Tech4Material::new(
        Tech4Globals {
            sh0: Vec4::ONE,
            sh1: Vec4::ZERO,
            sh2: Vec4::ZERO,
            hdr_scale: 1.0,
        },
        Tech4Textures {
            base_color,
            normal,
            specular_gloss,
            lightmap,
            probe,
        },
    )?;
    Ok((
        world.resource_mut::<Assets<Mesh>>().add(mesh),
        world.resource_mut::<Assets<Tech4Material>>().add(material),
        images,
    ))
}

/// Source-basis interpolated vertex outputs, not packed original attributes.
#[derive(Clone, Copy, Debug)]
pub struct Tech4Fragment {
    pub normal: Vec3,
    pub tangent: Vec3,
    pub bitangent: Vec3,
    pub camera_relative: Vec3,
    pub color: Vec3,
    pub inverse_sh: Vec3,
    pub fog: Vec4,
}
#[derive(Clone, Copy, Debug)]
pub struct Tech4Samples {
    pub base: Vec3,
    pub normal: Vec2,
    pub specular_gloss: Vec4,
    pub lightmap: [Vec4; 3],
    pub probe: Vec4,
}

pub fn inverse_probe_sh(normal: Vec3, globals: Tech4Globals) -> Vec3 {
    let n = normal;
    let k = globals.sh2.dot(Vec4::new(
        n.z * n.x,
        n.y * n.z,
        n.x * n.y,
        n.x * n.x - n.y * n.y,
    )) + globals.sh1.dot(n.extend(1.0))
        + globals.sh0.w * n.z * n.z;
    (globals.sh0.truncate() * k).max(Vec3::splat(0.1)).recip()
}

/// Independent CPU response for explicit sampled values, before target transfer.
/// Texture filtering, vertex interpolation and sampler LOD are separate GPU work.
pub fn shade_samples(
    input: Tech4Fragment,
    samples: Tech4Samples,
    hdr_scale: f32,
) -> Result<Vec4, Tech4Error> {
    if !input.normal.is_finite()
        || !input.tangent.is_finite()
        || !input.bitangent.is_finite()
        || !input.camera_relative.is_finite()
        || input.camera_relative.length_squared() < 1e-12
        || !input.color.is_finite()
        || !input.inverse_sh.is_finite()
        || !input.fog.is_finite()
        || !(0.0..=1.0).contains(&input.fog.w)
        || !hdr_scale.is_finite()
        || hdr_scale < 0.0
        || !samples.base.is_finite()
        || !samples.normal.is_finite()
        || !samples.specular_gloss.is_finite()
        || !samples.probe.is_finite()
        || samples.lightmap.iter().any(|v| !v.is_finite())
    {
        return Err(Tech4Error::InvalidAttributes);
    }
    let xy = samples.normal * 4.015748 - Vec2::splat(2.015748);
    let mapped = input.normal + input.tangent * xy.x + input.bitangent * xy.y;
    if mapped.length_squared() < 1e-12 {
        return Err(Tech4Error::InvalidAttributes);
    }
    let mapped = mapped.normalize();
    let gloss = samples.specular_gloss.w;
    let e = 2.0_f32.powf(
        -9.28
            * mapped
                .dot(-input.camera_relative.normalize())
                .clamp(0.0, 1.0),
    );
    let f = 1.041667 * gloss * e.min(0.475 * gloss) + 0.018229 * gloss - 0.015625;
    let spec = samples.specular_gloss.truncate();
    let modulation =
        (spec * spec * (0.75 + 0.25 * gloss - f) + Vec3::splat(f)).clamp(Vec3::ZERO, Vec3::ONE);
    let decode = |v: Vec4| v.truncate() / (v.w + 1e-6);
    let ambient = decode(samples.lightmap[0]);
    let directional = decode(samples.lightmap[1]);
    let direction = samples.lightmap[2].truncate() * 2.0 - Vec3::ONE;
    let diffuse = ambient + directional * direction.dot(mapped).clamp(0.0, 1.0);
    let geometric = ambient + directional * direction.dot(input.normal).clamp(0.0, 1.0);
    let base = samples.base * input.color;
    let lit =
        base * base * diffuse + decode(samples.probe) * modulation * geometric * input.inverse_sh;
    let fogged = (input.fog.truncate() * (1.0 - input.fog.w) + lit * input.fog.w) * hdr_scale;
    if !fogged.is_finite() || fogged.min_element() < 0.0 {
        return Err(Tech4Error::InvalidAttributes);
    }
    Ok(Vec3::new(fogged.x.sqrt(), fogged.y.sqrt(), fogged.z.sqrt()).extend(1.0))
}

/// Decode the qualified D3D R10G10B10A2_UNORM float inputs using the TECH4 VS
/// wrap-remap. The fourth component is not consumed by this direction equation.
/// Normalization occurs separately in the vertex stage, matching the source pass.
pub fn decode_direction_input(input: [f32; 4]) -> Result<[f32; 3], Tech4Error> {
    if input
        .iter()
        .any(|c| !c.is_finite() || !(0.0..=1.0).contains(c))
    {
        return Err(Tech4Error::InvalidAttributes);
    }
    let direction =
        [input[0], input[1], input[2]].map(|c| if c < 0.5 { 2.0 * c } else { 2.0 * c - 2.0 });
    if Vec3::from_array(direction).length_squared() < 1e-12 {
        return Err(Tech4Error::InvalidAttributes);
    }
    Ok(direction)
}

/// Qualified source vertex-fog response, before interpolation and HDR/square root.
pub fn vertex_fog(camera_relative: Vec3, fog: Tech4Fog) -> Result<Vec4, Tech4Error> {
    fog.validate()?;
    if !camera_relative.is_finite() || camera_relative.length_squared() < 1e-12 {
        return Err(Tech4Error::InvalidAttributes);
    }
    let c = fog.constants;
    let height = c.w * camera_relative.z;
    let profile = height + c.x;
    let integrated = if profile < 0.0 {
        (profile.min(64.0) * std::f32::consts::LOG2_E).exp2()
    } else {
        profile + 1.0
    };
    let density = if height.abs() < 0.0001 {
        fog.constants2.x.clamp(0.0, 1.0)
    } else {
        (integrated - fog.constants2.x) / height
    };
    let transmission = (density * c.y * camera_relative.length() + c.z)
        .exp2()
        .min(1.0);
    let sun = (fog
        .sun_direction
        .truncate()
        .dot(camera_relative.normalize())
        * fog.sun_parameters.y
        + fog.sun_parameters.x)
        .clamp(0.0, 1.0);
    let color = fog.color + (fog.sun_color - fog.color) * sun;
    let output = color
        .truncate()
        .extend(1.0 - (1.0 - transmission) * color.w);
    if !output.is_finite() {
        return Err(Tech4Error::InvalidGlobals);
    }
    Ok(output)
}

/// Build one vertex from captured, qualified source input-layout channels.
pub fn original_vertex(
    position: [f32; 3],
    uv: [f32; 2],
    attributes: bo2_assets::full_scene::SourceCornerAttributes,
) -> Result<Tech4Vertex, Tech4Error> {
    Ok(Tech4Vertex {
        position,
        normal: decode_direction_input(attributes.normal_unorm)?,
        tangent: decode_direction_input(attributes.tangent_unorm)?,
        handedness: attributes.position_w,
        color: attributes.color,
        uv,
        lightmap_uv: attributes.lightmap_uv,
        // Ignored by original material's qualified runtime-fog branch.
        fog_color_transmittance: [0.0, 0.0, 0.0, 1.0],
    })
}

/// Captured TECH4 scene inputs, never inferred from material names or defaults.
pub struct Tech4SurfaceRuntime {
    pub fog: Tech4Fog,
    pub hdr_scale: f32,
    pub lightmap_sampler: ImageSamplerDescriptor,
    pub probe_sampler: ImageSamplerDescriptor,
}
pub struct PreparedTech4Surface {
    pub mesh: Mesh,
    /// base color, normal, specular/gloss, secondary lightmap, reflection cube.
    pub images: [Image; 5],
    pub globals: Tech4Globals,
    pub fog: Tech4Fog,
    pub source_surface: usize,
}

/// Admit the exact captured base-opaque TECH4 family. Other technique-set names
/// are unsupported until their shader equations and pass contracts are qualified.
/// Serialized static source attributes/SH plus explicit captured runtime inputs
/// are required; no default light/fog/HDR/sampler is installed.
pub fn prepare_captured_tech4_surface(
    input: &bo2_assets::full_scene::FullSceneInput,
    surface_index: usize,
    runtime: Tech4SurfaceRuntime,
    formats: bevy::image::CompressedImageFormats,
) -> Result<PreparedTech4Surface, bo2_assets::appearance::AppearanceError> {
    use bo2_assets::appearance::AppearanceError as E;
    let source = input.surfaces.get(surface_index).ok_or(E::Reference)?;
    let material = input.materials.get(source.material).ok_or(E::Reference)?;
    if surface_index != 506
        || material.source_technique_set != "wpc_lit_sm_r0c0n0s0_1zzj1138"
        || source.source_vertex_format != 0
    {
        return Err(E::Unsupported);
    }
    let attributes = input.corner_attributes.as_ref().ok_or(E::Attributes)?;
    if attributes.len() != input.geometry.indices().len()
        || input.corner_uv.len() != attributes.len()
    {
        return Err(E::Attributes);
    }
    let range = input
        .geometry
        .surfaces()
        .get(surface_index)
        .ok_or(E::Reference)?
        .index_range();
    let mut corners = Vec::new();
    corners
        .try_reserve_exact(range.len())
        .map_err(|_| E::Limit)?;
    for corner in range {
        let source_index = *input.geometry.indices().get(corner).ok_or(E::Reference)?;
        let position = *input
            .geometry
            .positions()
            .get(source_index as usize)
            .ok_or(E::Reference)?;
        corners.push(
            original_vertex(position, input.corner_uv[corner], attributes[corner])
                .map_err(|_| E::Attributes)?,
        );
    }
    let mesh = tech4_mesh(&corners).map_err(|_| E::Attributes)?;
    let lookup = |semantic| -> Result<&bo2_assets::full_scene::SceneBinding, E> {
        let mut bindings = material.bindings.iter().filter(|b| b.semantic == semantic);
        let first = bindings.next().ok_or(E::Reference)?;
        if bindings.next().is_some() {
            return Err(E::Unsupported);
        }
        Ok(first)
    };
    let decode = |index: usize, sampler: ImageSamplerDescriptor, cube: bool| -> Result<Image, E> {
        let source = input.images.get(index).ok_or(E::Reference)?;
        let meta = bo2_assets::full_scene::inspect_dds(&source.dds)?;
        if meta != source.metadata || meta.cube != cube {
            return Err(E::Unsupported);
        }
        let image = Image::from_buffer(
            &source.dds,
            bevy::image::ImageType::Extension("dds"),
            formats,
            false,
            ImageSampler::Descriptor(sampler),
            RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
        )
        .map_err(|_| E::Unsupported)?;
        if image.texture_descriptor.format.is_srgb() {
            return Err(E::Unsupported);
        }
        Ok(image)
    };
    let base = lookup(2)?;
    let normal = lookup(5)?;
    let specular = lookup(8)?;
    let lightmap = input
        .lightmaps
        .get(source.lightmap.ok_or(E::Reference)?)
        .ok_or(E::Reference)?;
    let probe = input
        .probes
        .get(source.probe.ok_or(E::Reference)?)
        .ok_or(E::Reference)?;
    let images = [
        decode(
            base.image,
            crate::appearance::source_sampler(base.sampler)?,
            false,
        )?,
        decode(
            normal.image,
            crate::appearance::source_sampler(normal.sampler)?,
            false,
        )?,
        decode(
            specular.image,
            crate::appearance::source_sampler(specular.sampler)?,
            false,
        )?,
        decode(
            lightmap.secondary.ok_or(E::Reference)?,
            runtime.lightmap_sampler,
            false,
        )?,
        decode(probe.image, runtime.probe_sampler, true)?,
    ];
    let sh = probe.sh;
    let globals = Tech4Globals {
        sh0: Vec4::from_array(sh[..4].try_into().map_err(|_| E::Attributes)?),
        sh1: Vec4::from_array(sh[4..8].try_into().map_err(|_| E::Attributes)?),
        sh2: Vec4::from_array(sh[8..12].try_into().map_err(|_| E::Attributes)?),
        hdr_scale: runtime.hdr_scale,
    };
    globals.validate().map_err(|_| E::Attributes)?;
    runtime.fog.validate().map_err(|_| E::Attributes)?;
    Ok(PreparedTech4Surface {
        mesh,
        images,
        globals,
        fog: runtime.fog,
        source_surface: surface_index,
    })
}

/// Independent CPU clip reference; preserve source reversed depth and explicit viewport range.
pub fn captured_vertex_clip(position: Vec3, clip: CapturedClip) -> Result<Vec4, Tech4Error> {
    clip.validate()?;
    if !position.is_finite() {
        return Err(Tech4Error::InvalidAttributes);
    }
    let mut output = clip.view_projection * (position - clip.camera_source).extend(1.0);
    output.z = clip.viewport_depth.x * output.w
        + (clip.viewport_depth.y - clip.viewport_depth.x) * output.z;
    if !output.is_finite() {
        return Err(Tech4Error::InvalidGlobals);
    }
    Ok(output)
}

//! Source-informed opaque surfaces within Bevy's standard material pipeline.
use super::{PendingLightmap, SurfaceReady, profile::VisualProfile};
use crate::assets::{LoadConfiguration, LoadFailure, WorldCollider};
use bevy::{
    asset::{RenderAssetUsages, uuid_handle},
    gltf::{GltfExtras, GltfMaterialName},
    image::{
        ImageAddressMode, ImageFilterMode, ImageLoaderSettings, ImageSampler,
        ImageSamplerDescriptor,
    },
    light::{EnvironmentMapLight, GeneratedEnvironmentMapLight},
    mesh::{MeshVertexAttribute, MeshVertexBufferLayoutRef, VertexAttributeValues},
    pbr::{
        ExtendedMaterial, Lightmap, MaterialExtension, MaterialExtensionKey,
        MaterialExtensionPipeline,
    },
    prelude::*,
    render::render_resource::{
        AsBindGroup, Extent3d, RenderPipelineDescriptor, ShaderType, SpecializedMeshPipelineError,
        TextureDimension, TextureFormat, TextureViewDescriptor, TextureViewDimension, VertexFormat,
    },
    render::storage::ShaderBuffer,
    shader::{Shader, ShaderImport, ShaderRef},
};
use serde_json::Value;
use std::collections::{HashMap, HashSet};

const SHADER: Handle<Shader> = uuid_handle!("d61788a5-711f-4eb1-a7f7-bd1b9558da81");
const VERTEX_SHADER: Handle<Shader> = uuid_handle!("e6ed1727-30d7-4c74-b53d-59cb68d0fa84");
const BAKED_LIGHTING_SHADER: Handle<Shader> = uuid_handle!("febbf3da-9e6f-43b4-96bb-14ec1cfeb564");
const BINDINGS_SHADER: Handle<Shader> = uuid_handle!("20c29018-2914-4277-b41a-a97c8ad9cf0e");

#[derive(Resource)]
pub(super) struct BakedLightingReady;

// Use the pinned engine's lighting functions, with their baked-light conditions
// independent of its per-entity Lightmap allocator. No lighting equations are copied.
pub(super) fn baked_lighting(
    mut commands: Commands,
    mut shaders: ResMut<Assets<Shader>>,
    ready: Option<Res<BakedLightingReady>>,
    mut failure: ResMut<LoadFailure>,
    mut next: ResMut<NextState<crate::GameState>>,
) {
    if ready.is_some() {
        return;
    }
    let Some(source) = shaders.iter().find_map(|(_, shader)| {
        (shader.import_path == ShaderImport::Custom("bevy_pbr::pbr_functions".into()))
            .then(|| shader.source.as_str().to_string())
    }) else {
        return;
    };
    if source.matches("#ifdef LIGHTMAP").count() != 5 {
        failure.0 = "Pinned Bevy baked-lighting adapter no longer matches the engine shader".into();
        next.set(crate::GameState::LoadError);
        error!("{}", failure.0);
        return;
    }
    let adapted = source
        .replacen(
            "#define_import_path bevy_pbr::pbr_functions",
            "#define_import_path origins::baked_lighting",
            1,
        )
        .replace("#ifdef LIGHTMAP", "#ifdef ORIGINS_BAKED_LIGHTING");
    shaders
        .insert(
            BAKED_LIGHTING_SHADER.id(),
            Shader::from_wgsl(adapted, "origins::baked_lighting"),
        )
        .expect("Shader UUID is valid");
    commands.insert_resource(BakedLightingReady);
}
pub(crate) const LAYER_UV: MeshVertexAttribute =
    MeshVertexAttribute::new("OriginsLayerUv", 10001, VertexFormat::Float32x4);
pub(crate) const LAYER3_UV: MeshVertexAttribute =
    MeshVertexAttribute::new("OriginsLayer3Uv", 10002, VertexFormat::Float32x2);
pub(crate) const ROTATION1: MeshVertexAttribute =
    MeshVertexAttribute::new("OriginsRotation1", 10003, VertexFormat::Float32x4);
pub(crate) const ROTATION2: MeshVertexAttribute =
    MeshVertexAttribute::new("OriginsRotation2", 10004, VertexFormat::Float32x4);
const LIGHTING_VERTEX: MeshVertexAttribute =
    MeshVertexAttribute::new("OriginsLightingVertex", 10005, VertexFormat::Float32);
pub(super) type OpaqueMaterial = ExtendedMaterial<StandardMaterial, OpaqueSurface>;

#[derive(Resource, Default, serde::Serialize)]
pub(crate) struct OpaqueSurfaceStatus {
    world: usize,
    static_models: usize,
    normal: usize,
    specular_gloss: usize,
    reflections: usize,
    directional_atlas: usize,
    instance_colors: usize,
    probe_sh: usize,
    tile_macro: usize,
    layered: usize,
    masked: usize,
    transparent: usize,
    water: usize,
    wettable: usize,
    pub pending: usize,
    shared_materials: usize,
}

impl OpaqueSurfaceStatus {
    pub(crate) fn ready(&self) -> bool {
        self.world > 0 && self.static_models > 0 && self.pending == 0
    }
}

// Preparation-only face images and per-corner source arrays are no longer
// needed once all instances have their materials and vertex attributes.
pub(super) fn release_preparation(library: &mut SurfaceLibrary) -> Vec<AssetId<Image>> {
    let mut images = Vec::new();
    for probe in &mut library.probes {
        probe.faces.clear();
        if let Some(reflection) = &probe.reflection {
            images.push(reflection.id());
        }
    }
    library.lighting_vertices.clear();
    library.lighting_vertices.shrink_to_fit();
    images.extend(library.static_colors.values().map(Handle::id));
    images.push(library.black_cube.id());
    for definition in library.definitions.values_mut() {
        definition.channels.clear();
    }
    library.static_colors.clear();
    library.lightmaps.clear();
    images
}

// x: normal slope scale; y: gloss-to-roughness exponent; z: F0 scale; w: tile repeats.
// modes: tile bake, first specular, second specular, baked lighting kind (0/atlas/SH/colors).
#[derive(Clone, Copy, Debug, Reflect, ShaderType)]
struct SurfaceParameters {
    response: Vec4,
    radiance: Vec4,
    modes: UVec4,
    lighting: Vec4,
    sh0: Vec4,
    sh1: Vec4,
    sh2: Vec4,
    layers: UVec4,
    reveal: Vec4,
    wetness: Vec4,
    water: Vec4,
    velocity: Vec4,
}

#[derive(Asset, AsBindGroup, Reflect, Debug, Clone)]
#[bind_group_data(OpaqueSurfaceKey)]
#[data(100, SurfaceParameters, binding_array(130))]
#[bindless(limit(4096), index_table(range(100..129), binding(129)))]
pub(super) struct OpaqueSurface {
    parameters: SurfaceParameters,
    #[texture(101)]
    #[sampler(102)]
    normal: Option<Handle<Image>>,
    #[texture(103)]
    #[sampler(104)]
    second_normal: Option<Handle<Image>>,
    #[texture(105)]
    #[sampler(106)]
    specular: Option<Handle<Image>>,
    #[texture(107)]
    #[sampler(108)]
    second_specular: Option<Handle<Image>>,
    #[texture(109)]
    #[sampler(110)]
    control: Option<Handle<Image>>,
    #[texture(111)]
    #[sampler(112)]
    packed_lightmap: Option<Handle<Image>>,
    #[texture(113, dimension = "cube")]
    #[sampler(114)]
    reflection: Handle<Image>,
    #[texture(115)]
    #[sampler(116)]
    color0: Option<Handle<Image>>,
    #[texture(117)]
    #[sampler(118)]
    color1: Option<Handle<Image>>,
    #[texture(119)]
    #[sampler(120)]
    color2: Option<Handle<Image>>,
    #[texture(121)]
    #[sampler(122)]
    color3: Option<Handle<Image>>,
    #[texture(123)]
    #[sampler(124)]
    third_normal: Option<Handle<Image>>,
    #[texture(125)]
    #[sampler(126)]
    third_specular: Option<Handle<Image>>,
    #[texture(127)]
    instance_lighting: Option<Handle<Image>>,
    #[storage(128, read_only, binding_array(132))]
    animation_clock: Handle<ShaderBuffer>,
}

impl From<&OpaqueSurface> for SurfaceParameters {
    fn from(surface: &OpaqueSurface) -> Self {
        surface.parameters
    }
}

#[derive(Clone, Copy, Hash, PartialEq, Eq)]
pub(super) struct OpaqueSurfaceKey {
    baked: bool,
}

impl From<&OpaqueSurface> for OpaqueSurfaceKey {
    fn from(surface: &OpaqueSurface) -> Self {
        Self {
            baked: surface.parameters.modes.w != 0,
        }
    }
}

impl MaterialExtension for OpaqueSurface {
    fn vertex_shader() -> ShaderRef {
        VERTEX_SHADER.into()
    }
    fn fragment_shader() -> ShaderRef {
        SHADER.into()
    }
    fn specialize(
        _pipeline: &MaterialExtensionPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        layout: &MeshVertexBufferLayoutRef,
        key: MaterialExtensionKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        // Bevy's shadow/depth prepass uses its own locations (including color at 7).
        // Only the forward adapter needs the additional authored layer attributes.
        if descriptor.vertex.shader != VERTEX_SHADER {
            return Ok(());
        }
        if let Some(fragment) = descriptor.fragment.as_mut() {
            fragment.shader_defs.push(bevy::shader::ShaderDefVal::Bool(
                "ORIGINS_BAKED_LIGHTING".into(),
                key.bind_group_data.baked,
            ));
        }
        let mut attributes = vec![
            Mesh::ATTRIBUTE_POSITION.at_shader_location(0),
            Mesh::ATTRIBUTE_NORMAL.at_shader_location(1),
            Mesh::ATTRIBUTE_UV_0.at_shader_location(2),
            Mesh::ATTRIBUTE_TANGENT.at_shader_location(4),
        ];
        // Late scenes can specialize against the previous uploaded layout while
        // conversion adds neutral attributes to their CPU mesh. Request only
        // actual GPU inputs; the shader supplies the same defaults in that gap.
        for (attribute, location, define) in [
            (Mesh::ATTRIBUTE_UV_1, 3, "ORIGINS_UV_B"),
            (Mesh::ATTRIBUTE_COLOR, 5, "ORIGINS_COLOR"),
            (LAYER_UV, 8, "ORIGINS_LAYER_UV"),
            (LAYER3_UV, 9, "ORIGINS_LAYER3_UV"),
            (ROTATION1, 10, "ORIGINS_ROTATION1"),
            (ROTATION2, 11, "ORIGINS_ROTATION2"),
            (LIGHTING_VERTEX, 12, "ORIGINS_LIGHTING_VERTEX"),
        ] {
            if layout.0.contains(attribute) {
                attributes.push(attribute.at_shader_location(location));
                descriptor.vertex.shader_defs.push(define.into());
            } else {
                debug!("Surface vertex input {define} uses its neutral default");
            }
        }
        // Keep Bevy's intermediate/output structs and our fragment interface
        // stable even when the corresponding vertex input uses a default.
        for (attribute, define) in [
            (Mesh::ATTRIBUTE_UV_1, "VERTEX_UVS_B"),
            (Mesh::ATTRIBUTE_COLOR, "VERTEX_COLORS"),
        ] {
            if !layout.0.contains(attribute) {
                descriptor.vertex.shader_defs.push(define.into());
                if let Some(fragment) = descriptor.fragment.as_mut() {
                    fragment.shader_defs.push(define.into());
                }
            }
        }
        descriptor.vertex.buffers = vec![layout.0.get_layout(&attributes)?];
        Ok(())
    }
}

#[derive(Component)]
pub(super) struct OpaqueReady;
#[derive(Component)]
pub(super) struct ProbeIndex(usize);

pub(super) struct SurfaceDefinition {
    family: String,
    tiles: f32,
    pub(super) channels: HashMap<(String, u64), Handle<Image>>,
    pub presentation: Value,
    layers: UVec4,
    reveal: Vec4,
}
struct ProbeDefinition {
    faces: Vec<Handle<Image>>,
    sh: [Vec4; 3],
    position: Vec3,
    entity: Option<Entity>,
    reflection: Option<Handle<Image>>,
}
#[derive(Resource)]
pub(super) struct SurfaceLibrary {
    pub definitions: HashMap<String, SurfaceDefinition>,
    probes: Vec<ProbeDefinition>,
    static_colors: HashMap<u64, Handle<Image>>,
    lighting_vertices: HashMap<usize, Vec<f32>>,
    // Providers outlive scene instances and their released CPU vertex copies.
    pub(super) prepared_meshes: HashSet<AssetId<Mesh>>,
    lightmaps: HashMap<String, Handle<Image>>,
    black_cube: Handle<Image>,
    // Identity + source node lighting metadata preserves assignment-specific materials.
    materials: HashMap<(AssetId<StandardMaterial>, String), Handle<OpaqueMaterial>>,
    animation_clock: Handle<ShaderBuffer>,
}

fn data_texture(server: &AssetServer, path: String, repeat: bool) -> Handle<Image> {
    server
        .load_builder()
        .with_settings(move |settings: &mut ImageLoaderSettings| {
            settings.is_srgb = false;
            let mut sampler = ImageSamplerDescriptor::linear();
            if repeat {
                sampler.address_mode_u = ImageAddressMode::Repeat;
                sampler.address_mode_v = ImageAddressMode::Repeat;
            }
            settings.sampler = ImageSampler::Descriptor(sampler);
        })
        .load(path)
}

pub(super) fn initialize(
    mut commands: Commands,
    config: Res<LoadConfiguration>,
    server: Res<AssetServer>,
    mut images: ResMut<Assets<Image>>,
    mut shaders: ResMut<Assets<Shader>>,
    mut buffers: ResMut<Assets<ShaderBuffer>>,
    mut failure: ResMut<LoadFailure>,
    mut next: ResMut<NextState<crate::GameState>>,
) {
    let Ok(pack) = &config.0 else { return };
    let result = (|| -> Result<SurfaceLibrary, String> {
        let document: Value = serde_json::from_slice(
            &std::fs::read(pack.pack_root.join("map.gltf")).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
        let textures = document["textures"]
            .as_array()
            .ok_or("Missing texture table")?;
        let source_images = document["images"].as_array().ok_or("Missing image table")?;
        let mut definitions = HashMap::new();
        for material in document["materials"]
            .as_array()
            .ok_or("Missing material table")?
        {
            let surface = &material["extras"]["surface"];
            let identity = surface["identity"]
                .as_str()
                .ok_or("Missing surface identity")?;
            let family = surface["family"]
                .as_str()
                .ok_or("Missing surface family")?
                .to_string();
            let mut channels = HashMap::new();
            for channel in surface["channels"]
                .as_array()
                .ok_or("Missing surface channels")?
            {
                let role = channel["role"].as_str().ok_or("Missing channel role")?;
                let layer = channel["layer"].as_u64().ok_or("Missing channel layer")?;
                let texture = channel["texture"]
                    .as_u64()
                    .ok_or("Missing channel texture")? as usize;
                let image = textures
                    .get(texture)
                    .and_then(|t| t["source"].as_u64())
                    .ok_or("Invalid texture reference")? as usize;
                let path = source_images
                    .get(image)
                    .and_then(|i| i["uri"].as_str())
                    .ok_or("Invalid image reference")?;
                // Color channel 2 in this family is a linear macro mask, never albedo.
                let handle = if role == "color" && !(family == "tile_blend" && layer == 2) {
                    server
                        .load_builder()
                        .with_settings(|settings: &mut ImageLoaderSettings| {
                            settings.is_srgb = true;
                            let mut sampler = ImageSamplerDescriptor::linear();
                            sampler.address_mode_u = ImageAddressMode::Repeat;
                            sampler.address_mode_v = ImageAddressMode::Repeat;
                            settings.sampler = ImageSampler::Descriptor(sampler);
                        })
                        .load(path.to_string())
                } else {
                    data_texture(&server, path.into(), true)
                };
                channels.insert((role.into(), layer), handle);
            }
            let tiles = surface["preparation"]["tiles"].as_f64().unwrap_or(1.) as f32;
            let operations = surface["layers"]["operations"].as_array();
            let operation = |i: usize| {
                operations
                    .and_then(|v| v.get(i))
                    .and_then(Value::as_str)
                    .map(|s| match s {
                        "blend" => 1,
                        "multiply" => 2,
                        "add" => 3,
                        _ => 0,
                    })
                    .unwrap_or(0)
            };
            let reveal = |i, j| {
                surface["layers"]["reveal"][i][j]
                    .as_f64()
                    .unwrap_or(if j == 0 { 0.25 } else { 1. }) as f32
            };
            definitions.insert(
                identity.into(),
                SurfaceDefinition {
                    family,
                    tiles,
                    channels,
                    presentation: surface["presentation"].clone(),
                    layers: UVec4::new(
                        operations.map_or(0, Vec::len) as u32,
                        operation(1),
                        operation(2),
                        operation(3),
                    ),
                    reveal: Vec4::new(reveal(0, 0), reveal(0, 1), reveal(1, 0), reveal(1, 1)),
                },
            );
        }
        let mut probes = Vec::new();
        for probe in document["extras"]["lighting"]["probes"]
            .as_array()
            .ok_or("Missing probes")?
        {
            let faces = probe["faces"]
                .as_array()
                .ok_or("Missing probe faces")?
                .iter()
                .map(|v| {
                    v.as_str()
                        .map(|p| data_texture(&server, p.into(), false))
                        .ok_or("Invalid probe face".to_string())
                })
                .collect::<Result<Vec<_>, _>>()?;
            let sh = probe["sh"].as_array().ok_or("Missing probe SH")?;
            if faces.len() != 6 || sh.len() != 12 {
                return Err("Invalid probe layout".into());
            }
            let mut coefficients = [0.; 12];
            for (out, value) in coefficients.iter_mut().zip(sh) {
                *out = value.as_f64().ok_or("Invalid SH coefficient")? as f32;
            }
            let position = probe["position"]
                .as_array()
                .ok_or("Missing probe position")?;
            probes.push(ProbeDefinition {
                faces,
                sh: [
                    Vec4::from_slice(&coefficients[0..4]),
                    Vec4::from_slice(&coefficients[4..8]),
                    Vec4::from_slice(&coefficients[8..12]),
                ],
                position: Vec3::new(
                    position[0].as_f64().ok_or("Invalid probe position")? as f32,
                    position[1].as_f64().ok_or("Invalid probe position")? as f32,
                    position[2].as_f64().ok_or("Invalid probe position")? as f32,
                ),
                entity: None,
                reflection: None,
            });
        }
        // Sample assignment-specific lighting in the vertex shader. Geometry remains
        // shared; the retained LOD indices address each instance's own color texture.
        let binary = std::fs::read(pack.pack_root.join("map.bin")).map_err(|e| e.to_string())?;
        let mut lighting_vertices = HashMap::new();
        for (index, mesh) in document["meshes"]
            .as_array()
            .ok_or("Missing meshes")?
            .iter()
            .enumerate()
        {
            let Some(accessor_index) = mesh["extras"]["lighting_vertices"].as_u64() else {
                continue;
            };
            let accessor = &document["accessors"][accessor_index as usize];
            let view = &document["bufferViews"][accessor["bufferView"]
                .as_u64()
                .ok_or("Invalid lighting vertex view")?
                as usize];
            let start = view["byteOffset"].as_u64().unwrap_or(0) as usize
                + accessor["byteOffset"].as_u64().unwrap_or(0) as usize;
            let count = accessor["count"]
                .as_u64()
                .ok_or("Invalid lighting vertex count")? as usize;
            if accessor["componentType"] != 5125 || accessor["type"] != "SCALAR" {
                return Err("Unsupported lighting vertex layout".into());
            }
            let bytes = binary
                .get(start..start + count * 4)
                .ok_or("Lighting indices outside buffer")?;
            lighting_vertices.insert(
                index,
                bytes
                    .chunks_exact(4)
                    .map(|v| u32::from_le_bytes(v.try_into().unwrap()) as f32)
                    .collect(),
            );
        }
        let mut static_colors = HashMap::new();
        for node in document["nodes"].as_array().ok_or("Missing nodes")? {
            if let Some(index) = node["extras"]["lighting"]["color_lods"][0].as_u64() {
                if static_colors.contains_key(&index) {
                    continue;
                }
                let accessor = &document["accessors"][index as usize];
                let view = &document["bufferViews"][accessor["bufferView"]
                    .as_u64()
                    .ok_or("Invalid color accessor")?
                    as usize];
                let start = view["byteOffset"].as_u64().unwrap_or(0) as usize
                    + accessor["byteOffset"].as_u64().unwrap_or(0) as usize;
                let count = accessor["count"].as_u64().ok_or("Invalid color count")? as usize;
                if count == 0 || accessor["componentType"] != 5126 || accessor["type"] != "VEC4" {
                    return Err("Unsupported static color layout".into());
                }
                let bytes = binary
                    .get(start..start + count * 16)
                    .ok_or("Color accessor outside buffer")?;
                let width = count.min(1024) as u32;
                let height = (count as u32).div_ceil(width);
                let mut image = Image::new_fill(
                    Extent3d {
                        width,
                        height,
                        depth_or_array_layers: 1,
                    },
                    TextureDimension::D2,
                    &[0; 8],
                    TextureFormat::Rgba16Float,
                    RenderAssetUsages::default(),
                );
                for (vertex, color) in bytes.chunks_exact(16).enumerate() {
                    image
                        .set_color_at(
                            vertex as u32 % width,
                            vertex as u32 / width,
                            Color::linear_rgba(
                                f32::from_le_bytes(color[0..4].try_into().unwrap()),
                                f32::from_le_bytes(color[4..8].try_into().unwrap()),
                                f32::from_le_bytes(color[8..12].try_into().unwrap()),
                                f32::from_le_bytes(color[12..16].try_into().unwrap()),
                            ),
                        )
                        .map_err(|e| e.to_string())?;
                }
                static_colors.insert(index, images.add(image));
            }
        }
        let mut cube = Image::new_fill(
            Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 6,
            },
            TextureDimension::D2,
            &[0, 0, 0, 255],
            TextureFormat::Rgba8Unorm,
            RenderAssetUsages::default(),
        );
        cube.texture_view_descriptor = Some(TextureViewDescriptor {
            dimension: Some(TextureViewDimension::Cube),
            ..default()
        });
        Ok(SurfaceLibrary {
            definitions,
            probes,
            static_colors,
            lighting_vertices,
            lightmaps: HashMap::new(),
            prepared_meshes: HashSet::new(),
            black_cube: images.add(cube),
            materials: HashMap::new(),
            animation_clock: buffers.add(ShaderBuffer::from(Vec4::ZERO)),
        })
    })();
    match result {
        Ok(library) => {
            shaders
                .insert(
                    BINDINGS_SHADER.id(),
                    Shader::from_wgsl(
                        include_str!("surface_bindings.wgsl"),
                        "origins::surface_bindings",
                    ),
                )
                .expect("Shader UUID is valid");
            shaders
                .insert(
                    VERTEX_SHADER.id(),
                    Shader::from_wgsl(
                        include_str!("surface_vertex.wgsl"),
                        "src/rendering/surface_vertex.wgsl",
                    ),
                )
                .expect("Shader UUID is valid");
            shaders
                .insert(
                    SHADER.id(),
                    Shader::from_wgsl(include_str!("opaque.wgsl"), "src/rendering/opaque.wgsl"),
                )
                .expect("Shader UUID is valid");
            info!(
                "Opaque surface library: {} identities, {} probes, {} instance color records",
                library.definitions.len(),
                library.probes.len(),
                library.static_colors.len()
            );
            commands.insert_resource(library);
        }
        Err(error) => {
            failure.0 = format!("Opaque surface metadata: {error}");
            error!("{}", failure.0);
            next.set(crate::GameState::LoadError);
        }
    }
}

// Bevy owns convolution and the output mip chain. Keep cubes in their source basis;
// WGSL rotates the reflection direction into that basis. Filtering retains face order.
pub(super) fn probes(
    mut commands: Commands,
    library: Option<ResMut<SurfaceLibrary>>,
    mut images: ResMut<Assets<Image>>,
    filtered: Query<(&ProbeIndex, &EnvironmentMapLight)>,
) {
    let Some(mut library) = library else { return };
    for (index, light) in &filtered {
        library.probes[index.0].reflection = Some(light.specular_map.clone());
    }
    for (index, probe) in library.probes.iter_mut().enumerate() {
        if probe.entity.is_some() {
            continue;
        }
        let Some(faces) = probe
            .faces
            .iter()
            .map(|h| images.get(h))
            .collect::<Option<Vec<_>>>()
        else {
            continue;
        };
        let size = faces[0].texture_descriptor.size;
        if faces.iter().any(|f| {
            f.texture_descriptor.size != size
                || f.data
                    .as_ref()
                    .is_none_or(|d| d.len() != size.width as usize * size.height as usize * 4)
        }) {
            error!("Invalid retained probe {index}");
            continue;
        }
        let mut cube = Image::new_fill(
            Extent3d {
                width: size.width,
                height: size.height,
                depth_or_array_layers: 6,
            },
            TextureDimension::D2,
            &[0; 8],
            TextureFormat::Rgba16Float,
            RenderAssetUsages::default(),
        );
        for (face, image) in faces.iter().enumerate() {
            for (pixel, bytes) in image.data.as_ref().unwrap().chunks_exact(4).enumerate() {
                let alpha = bytes[3].max(1) as f32;
                cube.set_color_at_3d(
                    pixel as u32 % size.width,
                    pixel as u32 / size.width,
                    face as u32,
                    Color::linear_rgb(
                        bytes[0] as f32 / alpha,
                        bytes[1] as f32 / alpha,
                        bytes[2] as f32 / alpha,
                    ),
                )
                .expect("Validated cube pixel");
            }
        }
        cube.texture_view_descriptor = Some(TextureViewDescriptor {
            dimension: Some(TextureViewDimension::Cube),
            ..default()
        });
        cube.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
            mag_filter: ImageFilterMode::Linear,
            min_filter: ImageFilterMode::Linear,
            ..default()
        });
        let handle = images.add(cube);
        probe.entity = Some(
            commands
                .spawn((
                    ProbeIndex(index),
                    GeneratedEnvironmentMapLight {
                        environment_map: handle,
                        intensity: 1.,
                        affects_lightmapped_mesh_diffuse: false,
                        ..default()
                    },
                    Transform::from_translation(probe.position),
                    Name::new(format!("Origins retained probe {index}")),
                ))
                .id(),
        );
    }
}

// Presentation continues during pause; gameplay remains on its fixed-step clock.
pub(super) fn animate_water(
    time: Res<Time<Real>>,
    library: Option<Res<SurfaceLibrary>>,
    mut buffers: ResMut<Assets<ShaderBuffer>>,
) {
    let Some(library) = library else { return };
    // Updating material assets would churn bindless slabs and invalidate cached
    // shadow queues. Bevy uploads one shared buffer in place; materials stay static.
    if let Some(mut clock) = buffers.get_mut(&library.animation_clock) {
        clock.set_data(Vec4::new(time.elapsed_secs(), 0., 0., 0.));
    }
}

fn assignment(entity: Entity, extras: &Query<&GltfExtras>, parents: &Query<&ChildOf>) -> Value {
    let mut current = Some(entity);
    while let Some(entity) = current {
        if let Ok(extra) = extras.get(entity)
            && let Ok(value) = serde_json::from_str::<Value>(&extra.value)
            && value.get("lighting").is_some()
        {
            return value;
        }
        current = parents.get(entity).ok().map(ChildOf::parent);
    }
    Value::Null
}

pub(super) fn convert(
    mut commands: Commands,
    query: Query<
        (
            Entity,
            &MeshMaterial3d<StandardMaterial>,
            &GltfMaterialName,
            &Mesh3d,
        ),
        (
            With<SurfaceReady>,
            Without<OpaqueReady>,
            Without<WorldCollider>,
            Without<super::DynamicSurface>,
        ),
    >,
    extras: Query<&GltfExtras>,
    parents: Query<&ChildOf>,
    library: Option<ResMut<SurfaceLibrary>>,
    server: Res<AssetServer>,
    standard: Res<Assets<StandardMaterial>>,
    mut materials: ResMut<Assets<OpaqueMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut status: ResMut<OpaqueSurfaceStatus>,
    profile: Res<VisualProfile>,
) {
    let Some(mut library) = library else { return };
    status.pending = 0;
    for (entity, original, name, mesh) in &query {
        let Some(base) = standard.get(&original.0) else {
            status.pending += 1;
            continue;
        };
        let Some(definition) = library.definitions.get(&name.0) else {
            continue;
        };
        if base.unlit
            || !matches!(
                definition.family.as_str(),
                "lit" | "tile_blend" | "layered_lit" | "water"
            )
        {
            continue;
        }
        let node = assignment(entity, &extras, &parents);
        let lighting = &node["lighting"];
        let probe = lighting["probe"].as_u64().map(|i| i as usize);
        if probe.is_some_and(|i| library.probes.get(i).is_none_or(|p| p.reflection.is_none())) {
            status.pending += 1;
            continue;
        }
        let key = (original.0.id(), node.to_string());
        let lightmap_path = node["lightmap"].as_str();
        let colors = lighting["color_lods"][0]
            .as_u64()
            .and_then(|i| library.static_colors.get(&i))
            .cloned();
        let handle = if let Some(handle) = library.materials.get(&key) {
            handle.clone()
        } else {
            let channel =
                |role: &str, layer| definition.channels.get(&(role.to_string(), layer)).cloned();
            let tile = definition.family == "tile_blend" && definition.tiles > 1.;
            let water = definition.family == "water";
            let wettable = definition.presentation["wettable"]
                .as_bool()
                .unwrap_or(false);
            let sh = probe
                .map(|i| library.probes[i].sh)
                .unwrap_or([Vec4::ZERO; 3]);
            // Integrate the reconstructed SH polynomial over the sphere once
            // per shared material. Spare RGB uniform lanes avoid fragment work.
            let probe_fill = if lightmap_path.is_none() && colors.is_none() && !tile {
                (sh[0].truncate() * (sh[1].w + sh[0].w / 3.)).max(Vec3::ZERO)
                    * profile.prop_probe_fill
            } else {
                Vec3::ZERO
            };
            let parameters = SurfaceParameters {
                // Raw normals use the retained opaque slope convention. Gloss is mapped
                // continuously; F0 approximates the researched squared source specular RGB.
                response: Vec4::new(
                    if tile {
                        profile.tile_normal_strength
                    } else {
                        profile.normal_strength
                    },
                    profile.gloss_exponent,
                    profile.specular_f0_scale,
                    definition.tiles,
                ),
                // Source-relative radiance needs calibration to Bevy's cd/m² exposure.
                // This replaces the old additive ambient fill on baked surfaces.
                radiance: Vec4::new(
                    if lightmap_path.is_none() && colors.is_none() {
                        if tile {
                            profile.tile_probe_radiance
                        } else {
                            profile.probe_radiance
                        }
                    } else if colors.is_some() {
                        profile.vertex_radiance
                    } else {
                        profile.baked_radiance
                    },
                    profile.reflection_radiance,
                    0.,
                    profile.instance_probe_fill,
                ),
                modes: UVec4::new(
                    tile as u32
                        | ((channel("normal", 0).is_some() as u32) << 1)
                        | ((channel("normal", 1).is_some() as u32) << 2)
                        | ((channel("normal", 2).is_some() as u32) << 3)
                        | ((water as u32) << 4),
                    channel("specular_gloss", 0).is_some() as u32,
                    channel("specular_gloss", 1).is_some() as u32
                        | ((channel("specular_gloss", 2).is_some() as u32) << 1),
                    if lightmap_path.is_some() {
                        1
                    } else if colors.is_some() {
                        3
                    } else if probe.is_some() {
                        2
                    } else {
                        0
                    },
                ),
                lighting: probe_fill.extend(probe.is_some() as u32 as f32),
                sh0: sh[0],
                sh1: sh[1],
                sh2: sh[2],
                layers: definition.layers,
                reveal: definition.reveal
                    * Vec4::new(profile.reveal_softness, 1., profile.reveal_softness, 1.),
                wetness: Vec4::new(
                    if wettable { profile.wetness } else { 0. },
                    profile.wet_darkening,
                    profile.wet_roughness,
                    profile.transparent_opacity_scale,
                ),
                water: Vec4::new(
                    profile.water_normal_strength,
                    profile.water_roughness,
                    profile.water_uv_scale,
                    profile.water_opacity,
                ),
                velocity: Vec4::from_array(profile.water_velocity),
            };
            let mut extension = OpaqueSurface {
                parameters,
                normal: channel("normal", 0),
                second_normal: channel("normal", 1),
                specular: channel("specular_gloss", 0),
                second_specular: channel("specular_gloss", 1),
                control: if tile { channel("color", 2) } else { None },
                packed_lightmap: None,
                reflection: probe
                    .map(|i| library.probes[i].reflection.as_ref().unwrap().clone())
                    .unwrap_or_else(|| library.black_cube.clone()),
                color0: channel("color", 0),
                color1: channel("color", 1),
                color2: channel("color", 2),
                color3: channel("color", 3),
                third_normal: channel("normal", 2),
                third_specular: channel("specular_gloss", 2),
                instance_lighting: colors.clone(),
                animation_clock: library.animation_clock.clone(),
            };
            let mut base = base.clone();
            base.normal_map_texture = None;
            base.metallic_roughness_texture = None;
            base.metallic = 0.;
            base.lightmap_exposure = 1.;
            if definition.layers.x > 1 {
                base.base_color_texture = None;
            }
            if water {
                base.base_color_texture = None;
                base.base_color =
                    super::profile::color(profile.water_color).with_alpha(profile.water_opacity);
                base.alpha_mode = AlphaMode::Blend;
                base.cull_mode = None;
                base.double_sided = true;
            }
            if let Some(path) = lightmap_path {
                extension.packed_lightmap = Some(
                    library
                        .lightmaps
                        .entry(path.into())
                        .or_insert_with(|| data_texture(&server, path.into(), false))
                        .clone(),
                );
            }
            let handle = materials.add(ExtendedMaterial { base, extension });
            library.materials.insert(key, handle.clone());
            handle
        };
        // Baked-light exclusion is a material pipeline key, without allocating a
        // dummy Bevy lightmap slot (and mesh bind group) for every placement.
        let mesh_id = mesh.0.id();
        if !library.prepared_meshes.contains(&mesh_id) {
            let lighting_vertices = server
                .get_path(mesh.0.id())
                .and_then(|path| {
                    path.label().and_then(|label| {
                        label
                            .strip_prefix("Mesh")?
                            .split('/')
                            .next()?
                            .parse::<usize>()
                            .ok()
                    })
                })
                .and_then(|index| library.lighting_vertices.get(&index));
            let Some(mut mesh) = meshes.get_mut(&mesh.0) else {
                status.pending += 1;
                continue;
            };
            if mesh.attribute(Mesh::ATTRIBUTE_UV_1).is_none() {
                let count = match mesh.attribute(Mesh::ATTRIBUTE_POSITION) {
                    Some(VertexAttributeValues::Float32x3(v)) => v.len(),
                    _ => continue,
                };
                mesh.insert_attribute(Mesh::ATTRIBUTE_UV_1, vec![[0., 0.]; count]);
            }
            let count = mesh.count_vertices();
            // glTF vertex colors are optional. The surface adapter requires
            // them, so uncolored meshes use the neutral multiplicative tint.
            // Preserve authored colors and alpha on meshes that supply them.
            if mesh.attribute(Mesh::ATTRIBUTE_COLOR).is_none() {
                mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, vec![[1.; 4]; count]);
            }
            if mesh.attribute(LIGHTING_VERTEX).is_none() {
                let values = lighting_vertices
                    .cloned()
                    .unwrap_or_else(|| vec![0.; count]);
                if values.len() != count {
                    error!("Static lighting corner count mismatch");
                    continue;
                }
                mesh.insert_attribute(LIGHTING_VERTEX, values);
            }
            if mesh.attribute(LAYER_UV).is_none() {
                let uv = match mesh.attribute(Mesh::ATTRIBUTE_UV_0) {
                    Some(VertexAttributeValues::Float32x2(v)) => v.clone(),
                    _ => vec![[0.; 2]; count],
                };
                mesh.insert_attribute(
                    LAYER_UV,
                    uv.iter()
                        .map(|v| [v[0], v[1], v[0], v[1]])
                        .collect::<Vec<_>>(),
                );
                mesh.insert_attribute(LAYER3_UV, uv);
                mesh.insert_attribute(ROTATION1, vec![[1., 0., 0., 1.]; count]);
                mesh.insert_attribute(ROTATION2, vec![[1., 0., 0., 1.]; count]);
            }
            library.prepared_meshes.insert(mesh_id);
        }
        commands
            .entity(entity)
            .remove::<MeshMaterial3d<StandardMaterial>>()
            .remove::<PendingLightmap>()
            .remove::<Lightmap>()
            .insert((MeshMaterial3d(handle.clone()), OpaqueReady));
        if lighting.get("lighting_id").is_some() {
            status.static_models += 1;
        } else {
            status.world += 1;
        }
        let material = materials.get(&handle).expect("Converted material exists");
        status.normal += material.extension.normal.is_some() as usize;
        status.specular_gloss += material.extension.specular.is_some() as usize;
        status.reflections += probe.is_some() as usize;
        status.directional_atlas += lightmap_path.is_some() as usize;
        status.instance_colors += colors.is_some() as usize;
        status.probe_sh +=
            (lightmap_path.is_none() && colors.is_none() && probe.is_some()) as usize;
        status.tile_macro += ((material.extension.parameters.modes.x & 1) != 0) as usize;
        status.layered += (material.extension.parameters.layers.x > 1) as usize;
        status.masked += matches!(material.base.alpha_mode, AlphaMode::Mask(_)) as usize;
        status.transparent += (material.base.alpha_mode != AlphaMode::Opaque
            && !matches!(material.base.alpha_mode, AlphaMode::Mask(_)))
            as usize;
        status.water += ((material.extension.parameters.modes.x & 16) != 0) as usize;
        status.wettable += (material.extension.parameters.wetness.x > 0.) as usize;
    }
    status.shared_materials = library.materials.len();
}

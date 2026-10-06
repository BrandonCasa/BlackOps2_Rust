//! Source-informed opaque surfaces within Bevy's standard material pipeline.
use super::{PendingLightmap, SurfaceReady};
use crate::assets::{LoadConfiguration, LoadFailure, WorldCollider};
use bevy::{
    asset::{RenderAssetUsages, uuid_handle},
    gltf::{GltfExtras, GltfMaterialName},
    image::{
        ImageAddressMode, ImageFilterMode, ImageLoaderSettings, ImageSampler,
        ImageSamplerDescriptor,
    },
    light::{EnvironmentMapLight, GeneratedEnvironmentMapLight},
    mesh::VertexAttributeValues,
    pbr::{ExtendedMaterial, Lightmap, MaterialExtension},
    prelude::*,
    render::render_resource::{
        AsBindGroup, Extent3d, ShaderType, TextureDimension, TextureFormat, TextureViewDescriptor,
        TextureViewDimension,
    },
    shader::{Shader, ShaderRef},
};
use serde_json::Value;
use std::collections::HashMap;

const SHADER: Handle<Shader> = uuid_handle!("d61788a5-711f-4eb1-a7f7-bd1b9558da81");
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
    pub pending: usize,
    shared_materials: usize,
}

impl OpaqueSurfaceStatus {
    pub(crate) fn ready(&self) -> bool {
        self.world > 0 && self.static_models > 0 && self.pending == 0
    }
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
}

#[derive(Asset, AsBindGroup, Reflect, Debug, Clone)]
pub(super) struct OpaqueSurface {
    #[uniform(100)]
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
}

impl MaterialExtension for OpaqueSurface {
    fn fragment_shader() -> ShaderRef {
        SHADER.into()
    }
}

#[derive(Component)]
pub(super) struct OpaqueReady;
#[derive(Component)]
pub(super) struct ProbeIndex(usize);

struct SurfaceDefinition {
    family: String,
    tiles: f32,
    channels: HashMap<(String, u64), Handle<Image>>,
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
    definitions: HashMap<String, SurfaceDefinition>,
    probes: Vec<ProbeDefinition>,
    static_colors: HashMap<u64, Vec3>,
    lightmaps: HashMap<String, Handle<Image>>,
    black: Handle<Image>,
    black_cube: Handle<Image>,
    // Identity + source node lighting metadata preserves assignment-specific materials.
    materials: HashMap<(AssetId<StandardMaterial>, String), Handle<OpaqueMaterial>>,
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
                if role != "color" || (family == "tile_blend" && layer == 2) {
                    channels.insert(
                        (role.into(), layer),
                        data_texture(&server, path.into(), true),
                    );
                }
            }
            let tiles = surface["preparation"]["tiles"].as_f64().unwrap_or(1.) as f32;
            definitions.insert(
                identity.into(),
                SurfaceDefinition {
                    family,
                    tiles,
                    channels,
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
        // Instance colors remain in the standard pack. A mean is deliberate: no duplicated
        // instance geometry or custom vertex stream/renderer is needed for this milestone.
        let binary = std::fs::read(pack.pack_root.join("map.bin")).map_err(|e| e.to_string())?;
        let mut static_colors = HashMap::new();
        for node in document["nodes"].as_array().ok_or("Missing nodes")? {
            if let Some(index) = node["extras"]["lighting"]["color_lods"][0].as_u64() {
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
                let mut sum = Vec3::ZERO;
                for color in bytes.chunks_exact(16) {
                    sum += Vec3::new(
                        f32::from_le_bytes(color[0..4].try_into().unwrap()),
                        f32::from_le_bytes(color[4..8].try_into().unwrap()),
                        f32::from_le_bytes(color[8..12].try_into().unwrap()),
                    );
                }
                static_colors.insert(index, sum / count as f32);
            }
        }
        let black = images.add(Image::new_fill(
            Extent3d::default(),
            TextureDimension::D2,
            &[0, 0, 0, 255],
            TextureFormat::Rgba8Unorm,
            RenderAssetUsages::default(),
        ));
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
            lightmaps: HashMap::new(),
            black,
            black_cube: images.add(cube),
            materials: HashMap::new(),
        })
    })();
    match result {
        Ok(library) => {
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
) {
    let Some(mut library) = library else { return };
    status.pending = 0;
    for (entity, original, name, mesh) in &query {
        let Some(base) = standard.get(&original.0) else {
            continue;
        };
        let Some(definition) = library.definitions.get(&name.0) else {
            continue;
        };
        if base.alpha_mode != AlphaMode::Opaque
            || base.unlit
            || !matches!(
                definition.family.as_str(),
                "lit" | "tile_blend" | "layered_lit"
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
            .copied();
        let baked = lightmap_path.is_some() || colors.is_some() || probe.is_some();
        let handle = if let Some(handle) = library.materials.get(&key) {
            handle.clone()
        } else {
            let channel =
                |role: &str, layer| definition.channels.get(&(role.to_string(), layer)).cloned();
            let tile = definition.family == "tile_blend" && definition.tiles > 1.;
            let sh = probe
                .map(|i| library.probes[i].sh)
                .unwrap_or([Vec4::ZERO; 3]);
            let parameters = SurfaceParameters {
                // Raw normals use the retained opaque slope convention. Gloss is mapped
                // continuously; F0 approximates the researched squared source specular RGB.
                response: Vec4::new(if tile { 0.5 } else { 1. }, 0.5, 0.16, definition.tiles),
                // Source-relative radiance needs calibration to Bevy's cd/m² exposure.
                // This replaces the old additive ambient fill on baked surfaces.
                radiance: Vec4::new(64., 64., 0., 0.),
                modes: UVec4::new(
                    tile as u32
                        | ((channel("normal", 0).is_some() as u32) << 1)
                        | ((channel("normal", 1).is_some() as u32) << 2),
                    channel("specular_gloss", 0).is_some() as u32,
                    channel("specular_gloss", 1).is_some() as u32,
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
                lighting: colors
                    .unwrap_or(Vec3::ZERO)
                    .extend(probe.is_some() as u32 as f32),
                sh0: sh[0],
                sh1: sh[1],
                sh2: sh[2],
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
            };
            let mut base = base.clone();
            base.normal_map_texture = None;
            base.metallic_roughness_texture = None;
            base.metallic = 0.;
            base.lightmap_exposure = 1.;
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
        // A zero Bevy lightmap marks baked meshes for standard light/ambient exclusion.
        // Our fragment supplies the directional/HDR irradiance exactly once instead.
        if baked {
            let Some(mut mesh) = meshes.get_mut(&mesh.0) else {
                continue;
            };
            if mesh.attribute(Mesh::ATTRIBUTE_UV_1).is_none() {
                let count = match mesh.attribute(Mesh::ATTRIBUTE_POSITION) {
                    Some(VertexAttributeValues::Float32x3(v)) => v.len(),
                    _ => continue,
                };
                mesh.insert_attribute(Mesh::ATTRIBUTE_UV_1, vec![[0., 0.]; count]);
            }
            commands.entity(entity).insert(Lightmap {
                image: library.black.clone(),
                uv_rect: Rect::new(0., 0., 1., 1.),
                bicubic_sampling: false,
            });
        }
        commands
            .entity(entity)
            .remove::<MeshMaterial3d<StandardMaterial>>()
            .remove::<PendingLightmap>()
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
    }
    status.shared_materials = library.materials.len();
}

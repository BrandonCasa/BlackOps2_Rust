//! Source-informed opaque surfaces within Bevy's standard material pipeline.
mod material;
use super::{SurfaceReady, profile::VisualProfile};
use crate::assets::{LoadConfiguration, LoadFailure, WorldCollider};
use bevy::{
    asset::uuid_handle,
    gltf::{GltfExtras, GltfMaterialName},
    mesh::{MeshVertexAttribute, MeshVertexBufferLayoutRef, VertexAttributeValues},
    pbr::{
        ExtendedMaterial, MaterialExtension, MaterialExtensionKey, MaterialExtensionPipeline,
        MeshPipelineKey,
    },
    prelude::*,
    render::render_resource::{
        AsBindGroup, RenderPipelineDescriptor, ShaderType, SpecializedMeshPipelineError,
        VertexFormat,
    },
    render::storage::ShaderBuffer,
    shader::{Shader, ShaderImport, ShaderRef},
};
pub(super) use material::OpaqueMaterial;
use serde_json::Value;
use std::collections::{HashMap, HashSet};

const SHADER: Handle<Shader> = uuid_handle!("d61788a5-711f-4eb1-a7f7-bd1b9558da81");
const VERTEX_SHADER: Handle<Shader> = uuid_handle!("e6ed1727-30d7-4c74-b53d-59cb68d0fa84");
const BINDINGS_SHADER: Handle<Shader> = uuid_handle!("20c29018-2914-4277-b41a-a97c8ad9cf0e");
const LIGHTING_SHADER: Handle<Shader> = uuid_handle!("6a0c079c-843a-4d97-8ee7-e4ef3d973d6e");
const EFFECT_SHADER: Handle<Shader> = uuid_handle!("0b5a6dd8-a735-40f9-9b83-12b0e09967e8");
#[derive(Resource)]
pub(super) struct MaterialShadersReady;

pub(super) fn prepare_shaders(
    mut commands: Commands,
    mut shaders: ResMut<Assets<Shader>>,
    ready: Option<Res<MaterialShadersReady>>,
) {
    if ready.is_some() {
        return;
    }
    let Some(source) = shaders.iter().find_map(|(_, shader)| {
        (shader.import_path == ShaderImport::Custom("bevy_pbr::render::pbr_functions".into()))
            .then(|| shader.source.as_str().to_string())
    }) else {
        return;
    };
    super::pbr::diagnostic_shaders(&mut shaders, &source);
    let lighting = format!(
        "import origins::surface_bindings::{{reflection_diagnostic_enabled, surface_diagnostic_mode}};\n{}",
        source.replace("package::", "bevy_pbr::")
    ).replacen(
        "    indirect_light += environment_light.diffuse * diffuse_occlusion;",
        "    if reflection_diagnostic_enabled() { return vec4(environment_light.specular * specular_occlusion * view_bindings::view.exposure, 1.0); }\n    indirect_light += environment_light.diffuse * diffuse_occlusion;",
        1,
    );
    let lighting = super::pbr::lighting_diagnostics(lighting, "surface_diagnostic_mode");
    shaders
        .insert(
            LIGHTING_SHADER.id(),
            Shader::from_wesl(lighting, "embedded://origins/surface_lighting.wesl"),
        )
        .expect("Valid shader UUID");
    commands.insert_resource(MaterialShadersReady);
}
pub(crate) const LAYER_UV: MeshVertexAttribute =
    MeshVertexAttribute::new("OriginsLayerUv", 10001, VertexFormat::Float32x4);
pub(crate) const LAYER3_UV: MeshVertexAttribute =
    MeshVertexAttribute::new("OriginsLayer3Uv", 10002, VertexFormat::Float32x2);
pub(crate) const ROTATION1: MeshVertexAttribute =
    MeshVertexAttribute::new("OriginsRotation1", 10003, VertexFormat::Float32x4);
pub(crate) const ROTATION2: MeshVertexAttribute =
    MeshVertexAttribute::new("OriginsRotation2", 10004, VertexFormat::Float32x4);

#[derive(Resource, Default, serde::Serialize)]
pub(crate) struct OpaqueSurfaceStatus {
    world: usize,
    static_models: usize,
    normal: usize,
    response_maps: usize,
    tile_macro: usize,
    layered: usize,
    masked: usize,
    transparent: usize,
    pub(crate) premultiplied_sources: usize,
    pub(crate) straight_alpha_mismatches: usize,
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

// Converted materials own the prepared texture handles after scene assembly.
pub(super) fn release_preparation(library: &mut SurfaceLibrary) {
    for definition in library.definitions.values_mut() {
        definition.channels.clear();
    }
}

pub(super) fn converted_source(
    library: &SurfaceLibrary,
    id: bevy::asset::AssetId<StandardMaterial>,
) -> bool {
    library.materials.contains_key(&id)
}

// Canonical normal scale, coating roughness, material identity, tile repetitions.
// Flags select authored texture layers; illumination always comes from Bevy.
#[derive(Clone, Copy, Debug, Reflect, ShaderType)]
pub(super) struct EffectParameters {
    pub scale_scroll: [Vec4; 3],
    pub origin_offset: [Vec4; 3],
    pub rotation_depth: [Vec4; 3],
    pub alpha_hdr: Vec4,
    pub falloff_response: Vec4,
    pub coverage_layers: UVec4,
}

impl Default for EffectParameters {
    fn default() -> Self {
        Self {
            scale_scroll: [Vec4::new(1., 1., 0., 0.); 3],
            origin_offset: [Vec4::ZERO; 3],
            rotation_depth: [Vec4::ZERO; 3],
            alpha_hdr: Vec4::ONE,
            falloff_response: Vec4::new(0., 1., 0., 0.),
            coverage_layers: UVec4::new(0, 1, 2, 0),
        }
    }
}

impl EffectParameters {
    fn read(surface: &Value) -> Option<Self> {
        let effect = &surface["effect"];
        if effect["kind"] != "native_unlit_3layer" {
            return None;
        }
        let pair = |index: usize, key: &str| {
            Vec2::new(
                effect["layers"][index][key][0].as_f64().unwrap_or(0.) as f32,
                effect["layers"][index][key][1].as_f64().unwrap_or(0.) as f32,
            )
        };
        let mut result = Self::default();
        for i in 0..3 {
            let scale = pair(i, "scale");
            let scroll = pair(i, "scroll");
            let origin = pair(i, "origin");
            let offset = pair(i, "offset");
            let rotation = pair(i, "rotation");
            result.scale_scroll[i] = Vec4::new(scale.x, scale.y, scroll.x, scroll.y);
            result.origin_offset[i] = Vec4::new(origin.x, origin.y, offset.x, offset.y);
            result.rotation_depth[i] = rotation
                .extend(effect["layers"][i]["depth"].as_f64().unwrap_or(0.) as f32)
                .extend(0.);
            result.alpha_hdr[i] = effect["layers"][i]["alpha"].as_f64().unwrap_or(1.) as f32;
            result.coverage_layers[i] =
                effect["coverage_layers"][i].as_u64().unwrap_or(i as u64) as u32;
        }
        result.alpha_hdr.w = effect["hdr"].as_f64().unwrap_or(1.) as f32;
        result.falloff_response = Vec4::new(
            effect["falloff"][0].as_f64().unwrap_or(0.) as f32,
            effect["falloff"][1].as_f64().unwrap_or(1.) as f32,
            effect["reflection"].as_f64().unwrap_or(0.) as f32,
            effect["gloss"].as_f64().unwrap_or(0.) as f32,
        );
        Some(result)
    }
}

#[derive(Clone, Copy, Debug, Reflect, ShaderType)]
pub(super) struct SurfaceParameters {
    pub response: Vec4,
    pub modes: UVec4,
    pub layers: UVec4,
    pub reveal: Vec4,
    pub wetness: Vec4,
    pub water: Vec4,
    pub velocity: Vec4,
    pub effect: EffectParameters,
}

#[derive(Asset, AsBindGroup, Reflect, Debug, Clone)]
#[data(100, SurfaceParameters, binding_array(125))]
// Stay below StandardMaterial's 2048-entry shared texture-array layout. The
// extension and base share an allocator; a larger extension limit can overflow
// the base array as additional PBR maps become ready.
#[bindless(limit(1024), index_table(range(100..124), binding(124)))]
pub(super) struct OpaqueSurface {
    parameters: SurfaceParameters,
    #[texture(101)]
    #[sampler(102)]
    #[dependency]
    normal: Option<Handle<Image>>,
    #[texture(103)]
    #[sampler(104)]
    #[dependency]
    second_normal: Option<Handle<Image>>,
    #[texture(105)]
    #[sampler(106)]
    #[dependency]
    specular: Option<Handle<Image>>,
    #[texture(107)]
    #[sampler(108)]
    #[dependency]
    second_specular: Option<Handle<Image>>,
    #[texture(109)]
    #[sampler(110)]
    #[dependency]
    control: Option<Handle<Image>>,
    #[texture(111)]
    #[sampler(112)]
    #[dependency]
    color0: Option<Handle<Image>>,
    #[texture(113)]
    #[sampler(114)]
    #[dependency]
    color1: Option<Handle<Image>>,
    #[texture(115)]
    #[sampler(116)]
    #[dependency]
    color2: Option<Handle<Image>>,
    #[texture(117)]
    #[sampler(118)]
    #[dependency]
    color3: Option<Handle<Image>>,
    #[texture(119)]
    #[sampler(120)]
    #[dependency]
    third_normal: Option<Handle<Image>>,
    #[texture(121)]
    #[sampler(122)]
    #[dependency]
    third_specular: Option<Handle<Image>>,
    #[storage(123, read_only, binding_array(126))]
    #[dependency]
    animation_clock: Handle<ShaderBuffer>,
}

impl From<&OpaqueSurface> for SurfaceParameters {
    fn from(surface: &OpaqueSurface) -> Self {
        surface.parameters
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
            // Native shadow keys specify the light projection; camera prepass
            // keys do not. Include alpha-tested casters without changing their
            // discard shader or the camera's authored backface culling.
            let light_projection = key
                .mesh_key
                .intersection(MeshPipelineKey::VIEW_PROJECTION_RESERVED_BITS);
            let shadow_view = key.mesh_key.contains(MeshPipelineKey::DEPTH_PREPASS)
                && (key
                    .mesh_key
                    .contains(MeshPipelineKey::UNCLIPPED_DEPTH_ORTHO)
                    || light_projection == MeshPipelineKey::VIEW_PROJECTION_PERSPECTIVE);
            if shadow_view {
                // Room-facing wall/ceiling sheets must also block light coming
                // from their back. Match the ray scene's two-sided visibility.
                descriptor.primitive.cull_mode = None;
            }
            return Ok(());
        }
        let mut attributes = vec![
            Mesh::ATTRIBUTE_POSITION.at_shader_location(0),
            Mesh::ATTRIBUTE_NORMAL.at_shader_location(1),
            Mesh::ATTRIBUTE_UV_0.at_shader_location(2),
        ];
        // Late scenes can specialize against the previous uploaded layout while
        // conversion adds neutral attributes to their CPU mesh. Request only
        // actual GPU inputs; the shader supplies the same defaults in that gap.
        for (attribute, location, define) in [
            (Mesh::ATTRIBUTE_TANGENT, 4, "ORIGINS_TANGENT"),
            (Mesh::ATTRIBUTE_UV_1, 3, "ORIGINS_UV_B"),
            (Mesh::ATTRIBUTE_COLOR, 5, "ORIGINS_COLOR"),
            (LAYER_UV, 8, "ORIGINS_LAYER_UV"),
            (LAYER3_UV, 9, "ORIGINS_LAYER3_UV"),
            (ROTATION1, 10, "ORIGINS_ROTATION1"),
            (ROTATION2, 11, "ORIGINS_ROTATION2"),
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
            (Mesh::ATTRIBUTE_TANGENT, "VERTEX_TANGENTS"),
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
        // Keep Bevy's native GPU joint lookup and weighted pose arithmetic.
        // The wrapper must forward both inputs whenever SKINNED is selected.
        if layout.0.contains(Mesh::ATTRIBUTE_JOINT_INDEX)
            && layout.0.contains(Mesh::ATTRIBUTE_JOINT_WEIGHT)
        {
            attributes.push(Mesh::ATTRIBUTE_JOINT_INDEX.at_shader_location(6));
            attributes.push(Mesh::ATTRIBUTE_JOINT_WEIGHT.at_shader_location(7));
        }
        descriptor.vertex.buffers = vec![layout.0.get_layout(&attributes)?];
        Ok(())
    }
}

#[derive(Component)]
pub(super) struct OpaqueReady;

pub(super) struct SurfaceDefinition {
    identity_hash: f32,
    family: String,
    tiles: f32,
    pub(super) channels: HashMap<(String, u64), Handle<Image>>,
    pub presentation: Value,
    layers: UVec4,
    reveal: Vec4,
    effect: Option<EffectParameters>,
}
#[derive(Resource)]
pub(super) struct SurfaceLibrary {
    pub definitions: HashMap<String, SurfaceDefinition>,
    // Providers outlive scene instances and their released CPU vertex copies.
    pub(super) prepared_meshes: HashSet<AssetId<Mesh>>,
    // Static placements share their physical material; no instance lighting payloads.
    materials: HashMap<AssetId<StandardMaterial>, Handle<OpaqueMaterial>>,
    animation_clock: Handle<ShaderBuffer>,
}

/// Late animated scenes supply their own complete prepared channel ownership.
/// World preparation may already have released its temporary channel table.
pub(super) fn register_prepared(
    library: &mut SurfaceLibrary,
    surface: &Value,
    channels: &[(super::pbr::PreparedTexture, Handle<Image>)],
) {
    let Some(identity) = surface["identity"].as_str() else {
        return;
    };
    let count = channels
        .iter()
        .filter(|(t, _)| t.role == "color")
        .map(|(t, _)| t.layer + 1)
        .max()
        .unwrap_or(1) as u32;
    let effect = EffectParameters::read(surface);
    let tile = surface["family"].as_str() == Some("tile_blend");
    if (count < 2 && effect.is_none() && !tile)
        || (surface["family"].as_str() == Some("unlit") && effect.is_none())
    {
        return;
    }
    let operations = surface["layers"]["operations"].as_array();
    let operation = |index| match operations
        .and_then(|v| v.get(index))
        .and_then(Value::as_str)
    {
        Some("multiply") => 2,
        Some("add") => 3,
        _ => 1,
    };
    let reveal = |i, j| {
        surface["layers"]["reveal"][i][j]
            .as_f64()
            .unwrap_or(if j == 0 { 0.25 } else { 1. }) as f32
    };
    library.definitions.insert(
        identity.to_owned(),
        SurfaceDefinition {
            identity_hash: (identity
                .bytes()
                .fold(2166136261u32, |h, b| (h ^ b as u32).wrapping_mul(16777619))
                % 65535) as f32,
            family: surface["family"].as_str().unwrap_or("lit").to_owned(),
            tiles: surface["preparation"]["tiles"].as_f64().unwrap_or(1.) as f32,
            channels: channels
                .iter()
                .map(|(t, h)| ((t.role.clone(), t.layer), h.clone()))
                .collect(),
            presentation: surface["presentation"].clone(),
            layers: UVec4::new(
                operations.map_or(if tile { 0 } else { count }, |v| v.len() as u32),
                operation(1),
                operation(2),
                operation(3),
            ),
            reveal: Vec4::new(reveal(0, 0), reveal(0, 1), reveal(1, 0), reveal(1, 1)),
            effect,
        },
    );
}

pub(super) fn requires_extension(library: &SurfaceLibrary, identity: &str) -> bool {
    library.definitions.get(identity).is_some_and(|d| {
        d.layers.x > 1 || d.effect.is_some() || (d.family == "tile_blend" && d.tiles > 1.)
    })
}

pub(super) fn initialize(
    mut commands: Commands,
    config: Res<LoadConfiguration>,
    server: Res<AssetServer>,
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
                if surface["version"] != 2 {
                    return Err(
                        "Texture migration required: expected physical material version 2".into(),
                    );
                }
                let texture: super::pbr::PreparedTexture =
                    serde_json::from_value(channel.clone()).map_err(|e| e.to_string())?;
                channels.insert((role.into(), layer), super::pbr::load(&server, &texture));
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
                    identity_hash: (identity
                        .as_bytes()
                        .iter()
                        .fold(2166136261u32, |hash, byte| {
                            (hash ^ u32::from(*byte)).wrapping_mul(16777619)
                        })
                        % 65535) as f32,
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
                    effect: EffectParameters::read(surface),
                },
            );
        }
        Ok(SurfaceLibrary {
            definitions,
            prepared_meshes: HashSet::new(),
            materials: HashMap::new(),
            animation_clock: buffers.add(ShaderBuffer::from(vec![Vec4::ZERO])),
        })
    })();
    match result {
        Ok(library) => {
            shaders
                .insert(
                    EFFECT_SHADER.id(),
                    Shader::from_wesl(
                        include_str!("surface_effect.wesl"),
                        "embedded://origins/surface_effect.wesl",
                    ),
                )
                .expect("Effect shader");
            shaders
                .insert(
                    BINDINGS_SHADER.id(),
                    Shader::from_wesl(
                        include_str!("surface_bindings.wesl"),
                        "embedded://origins/surface_bindings.wesl",
                    ),
                )
                .expect("Shader UUID is valid");
            shaders
                .insert(
                    VERTEX_SHADER.id(),
                    Shader::from_wesl(
                        include_str!("surface_vertex.wesl"),
                        "embedded://origins/surface_vertex.wesl",
                    ),
                )
                .expect("Shader UUID is valid");
            shaders
                .insert(
                    SHADER.id(),
                    Shader::from_wesl(
                        include_str!("opaque.wesl"),
                        "embedded://origins/opaque.wesl",
                    ),
                )
                .expect("Shader UUID is valid");
            info!(
                "Realtime surface library: {} physical material identities",
                library.definitions.len()
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

pub(super) fn animate_water(
    time: Res<Time<Real>>,
    mode: Res<super::pbr::MaterialDiagnostic>,
    library: Option<Res<SurfaceLibrary>>,
    mut buffers: ResMut<Assets<ShaderBuffer>>,
) {
    let Some(library) = library else { return };
    // Updating material assets would churn bindless slabs and invalidate cached
    // shadow queues. Bevy uploads one shared buffer in place; materials stay static.
    if let Some(mut clock) = buffers.get_mut(&library.animation_clock) {
        clock.clear();
        clock.extend_from_slice(&[Vec4::new(time.elapsed_secs(), mode.0 as f32, 0., 0.)]);
    }
}

fn static_instance(entity: Entity, extras: &Query<&GltfExtras>, parents: &Query<&ChildOf>) -> bool {
    let mut current = Some(entity);
    while let Some(entity) = current {
        if extras
            .get(entity)
            .ok()
            .and_then(|extra| serde_json::from_str::<Value>(&extra.value).ok())
            .is_some_and(|value| value["static_instance"] == true)
        {
            return true;
        }
        current = parents.get(entity).ok().map(ChildOf::parent);
    }
    false
}

pub(super) fn convert(
    mut commands: Commands,
    query: Query<
        (
            Entity,
            &MeshMaterial3d<StandardMaterial>,
            &GltfMaterialName,
            &Mesh3d,
            Has<super::ActorSurface>,
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
    standard: Res<Assets<StandardMaterial>>,
    mut materials: ResMut<Assets<OpaqueMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
    images: Res<Assets<Image>>,
    mut status: ResMut<OpaqueSurfaceStatus>,
    profile: Res<VisualProfile>,
) {
    let Some(mut library) = library else { return };
    status.pending = 0;
    for (entity, original, name, mesh, actor) in &query {
        let Some(base) = standard.get(&original.0) else {
            status.pending += 1;
            continue;
        };
        let Some(definition) = library.definitions.get(&name.0) else {
            continue;
        };
        if (base.unlit && definition.layers.x <= 1 && definition.effect.is_none())
            || !matches!(
                definition.family.as_str(),
                "lit" | "tile_blend" | "layered_lit" | "water" | "unlit"
            )
        {
            continue;
        }
        let is_static = static_instance(entity, &extras, &parents);
        let effect_admitted = definition.effect.is_some();
        let premultiplied_source =
            definition.presentation["alpha"].as_str() == Some("premultiplied");
        let key = original.0.id();
        let handle = if let Some(handle) = library.materials.get(&key) {
            handle.clone()
        } else {
            let channel =
                |role: &str, layer| definition.channels.get(&(role.to_string(), layer)).cloned();
            let tile = definition.family == "tile_blend" && definition.tiles > 1.;
            let water = definition.family == "water";
            let mut normal_encodings = 0;
            for layer in 0..3 {
                if let Some(handle) = channel("normal", layer)
                    && let Some(image) = images.get(&handle)
                {
                    normal_encodings |=
                        super::pbr::NormalEncoding::for_image(image).flag(layer as u32);
                }
            }
            let parameters = SurfaceParameters {
                // Canonical maps already include family normal strength and roughness cooking.
                response: Vec4::new(1., 0.12, definition.identity_hash, definition.tiles),
                modes: UVec4::new(
                    tile as u32
                        | ((channel("normal", 0).is_some() as u32) << 1)
                        | ((channel("normal", 1).is_some() as u32) << 2)
                        | ((channel("normal", 2).is_some() as u32) << 3)
                        | ((water as u32) << 4)
                        | ((definition.effect.is_some() as u32) << 5)
                        | ((base.unlit as u32) << 6)
                        | ((actor as u32) << 7),
                    channel("response", 0).is_some() as u32,
                    channel("response", 1).is_some() as u32
                        | ((channel("response", 2).is_some() as u32) << 1),
                    normal_encodings,
                ),
                layers: definition.layers,
                reveal: definition.reveal
                    * Vec4::new(profile.reveal_softness, 1., profile.reveal_softness, 1.),
                wetness: Vec4::new(0., 0., 0., profile.transparent_opacity_scale),
                water: Vec4::new(
                    profile.water_normal_strength,
                    profile.water_roughness,
                    profile.water_uv_scale,
                    profile.water_opacity,
                ),
                velocity: Vec4::from_array(profile.water_velocity),
                effect: definition.effect.unwrap_or_default(),
            };
            let extension = OpaqueSurface {
                parameters,
                normal: channel("normal", 0),
                second_normal: channel("normal", 1),
                specular: channel("response", 0),
                second_specular: channel("response", 1),
                control: if tile { channel("control", 2) } else { None },
                color0: channel("color", 0),
                color1: channel("color", 1),
                color2: channel("color", 2),
                color3: channel("color", 3),
                third_normal: channel("normal", 2),
                third_specular: channel("response", 2),
                animation_clock: library.animation_clock.clone(),
            };
            let mut base = base.clone();
            base.normal_map_texture = None;
            base.metallic_roughness_texture = None;
            base.occlusion_texture = None;
            base.reflectance = 0.5;
            base.clearcoat = 1.;
            base.clearcoat_perceptual_roughness = 0.12;
            if definition.layers.x > 1 || tile {
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
            let handle = materials.add(ExtendedMaterial { base, extension });
            library.materials.insert(key, handle.clone());
            if effect_admitted {
                info!(
                    "Native layered effect admitted: {} (three retained maps; native vertex path)",
                    name.0
                );
            }
            handle
        };
        let mesh_id = mesh.0.id();
        if !library.prepared_meshes.contains(&mesh_id) {
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
            .insert((MeshMaterial3d(handle.clone()), OpaqueReady));
        if is_static {
            status.static_models += 1;
        } else {
            status.world += 1;
        }
        let material = materials.get(&handle).expect("Converted material exists");
        status.normal += material.extension.normal.is_some() as usize;
        status.response_maps += material.extension.specular.is_some() as usize;
        status.tile_macro += ((material.extension.parameters.modes.x & 1) != 0) as usize;
        status.layered += (material.extension.parameters.layers.x > 1) as usize;
        status.masked += matches!(material.base.alpha_mode, AlphaMode::Mask(_)) as usize;
        status.transparent += (material.base.alpha_mode != AlphaMode::Opaque
            && !matches!(material.base.alpha_mode, AlphaMode::Mask(_)))
            as usize;
        status.premultiplied_sources += premultiplied_source as usize;
        status.straight_alpha_mismatches +=
            (premultiplied_source && material.base.alpha_mode != AlphaMode::Blend) as usize;
        status.water += ((material.extension.parameters.modes.x & 16) != 0) as usize;
        status.wettable += (material.extension.parameters.wetness.x > 0.) as usize;
    }
    status.shared_materials = library.materials.len();
}

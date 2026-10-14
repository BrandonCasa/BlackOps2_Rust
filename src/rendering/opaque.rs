use super::{
    SurfaceReady,
    pbr::{PbrReady, PreparedTexture},
};
use bevy::{
    gltf::{GltfExtras, GltfMaterialName},
    mesh::skinning::{SkinnedMesh, SkinnedMeshInverseBindposes},
    mesh::{MeshVertexAttribute, VertexAttributeValues},
    prelude::*,
    render::render_resource::AsBindGroup,
};
use serde_json::Value;
use std::collections::{HashMap, HashSet};

pub(crate) const FLAG_SOURCE_NORMAL: MeshVertexAttribute = MeshVertexAttribute::new(
    "_ORIGINS_NORMAL",
    1_501_001,
    bevy::render::render_resource::VertexFormat::Float32x3,
);
pub(crate) const FLAG_SOURCE_TANGENT: MeshVertexAttribute = MeshVertexAttribute::new(
    "_ORIGINS_TANGENT",
    1_501_002,
    bevy::render::render_resource::VertexFormat::Float32x3,
);
pub(crate) const FLAG_PACKED_BASIS: MeshVertexAttribute = MeshVertexAttribute::new(
    "_ORIGINS_PACKED_BASIS",
    1_501_003,
    bevy::render::render_resource::VertexFormat::Uint32x2,
);

pub(crate) fn bindless_supported(device: &bevy::render::renderer::RenderDevice) -> bool {
    StandardMaterial::bindless_supported(device)
}

#[derive(Resource, Default, serde::Serialize)]
pub(crate) struct OpaqueSurfaceStatus {
    world: usize,
    static_models: usize,
    normal: usize,
    response_maps: usize,
    masked: usize,
    transparent: usize,
    water: usize,
    pub pending: usize,
    shared_materials: usize,
}
impl OpaqueSurfaceStatus {
    pub(crate) fn ready(&self) -> bool {
        self.world > 0 && self.static_models > 0 && self.pending == 0
    }
}

pub(super) struct SurfaceDefinition {
    pub presentation: Value,
    channels: HashMap<(String, u64), Handle<Image>>,
    metadata: Value,
    frames: Vec<Frame>,
}
#[derive(Resource, Default)]
pub(crate) struct SurfaceLibrary {
    pub(super) definitions: HashMap<String, SurfaceDefinition>,
    pub(super) prepared_meshes: HashSet<AssetId<Mesh>>,
    pub(super) live_meshes: HashSet<AssetId<Mesh>>,
}
impl SurfaceLibrary {
    pub(crate) fn textures_ready(&self, server: &AssetServer) -> Result<bool, String> {
        let mut ready = true;
        for definition in self.definitions.values() {
            for image in definition.channels.values().chain(
                definition
                    .frames
                    .iter()
                    .flat_map(|frame| frame.channels.values()),
            ) {
                if let Some(bevy::asset::LoadState::Failed(error)) =
                    server.get_load_state(image.id())
                {
                    return Err(format!(
                        "Standard material texture {:?}: {error}",
                        image.path()
                    ));
                }
                ready &= server.is_loaded_with_dependencies(image.id());
            }
        }
        Ok(ready)
    }
}

pub(super) fn initialize(
    mut commands: Commands,
    flags: Query<(Entity, &FlagMotion, &Mesh3d, Option<&SkinnedMesh>)>,
    mut inputs: Query<&mut NativeArtInputs>,
) {
    for (entity, flag, current, skin) in &flags {
        let mut entity = commands.entity(entity);
        if flag.target == current.0 {
            entity.insert((Mesh3d(flag.original.clone()), flag.source_bounds()));
        }
        if skin.is_none()
            && let Some(source) = &flag.skin
        {
            entity.insert(source.original.clone());
        }
        entity.remove::<FlagMotion>();
    }
    for mut input in &mut inputs {
        input.shader_time = None;
        input.script_available = 0;
    }
    commands.insert_resource(NativeClock::default());
    commands.insert_resource(SurfaceLibrary::default());
    commands.insert_resource(MaterialTime::default());
    commands.insert_resource(OpaqueSurfaceStatus::default());
}

#[derive(Resource, Default)]
pub(super) struct MaterialTime(f64);

#[derive(Resource, Default)]
pub(crate) struct NativeClock {
    running: bool,
    realtime: std::time::Duration,
    remainder: std::time::Duration,
    server: i32,
    snapshot: Option<i32>,
    render: Option<i32>,
    delta: i32,
    interpolation: i32,
    extrapolated: bool,
    paused: bool,
}

impl NativeClock {
    pub(crate) fn render_milliseconds(&self) -> Option<i32> {
        self.render
    }

    pub(crate) fn server_milliseconds(&self) -> Option<i32> {
        self.running.then_some(self.server)
    }

    fn advance(
        &mut self,
        duration: std::time::Duration,
        paused: bool,
        scale: f32,
    ) -> Result<(), String> {
        if !self.running {
            return Ok(());
        }
        self.realtime += duration;
        let realtime = i32::try_from(self.realtime.as_millis())
            .map_err(|_| "Native client clock requires full server restart")?;
        if paused {
            self.paused = true;
            return Ok(());
        }
        self.remainder += duration;
        let ticks = self.remainder.as_millis() / 50;
        let previous = self.snapshot.unwrap_or(0);
        if ticks != 0 {
            let advance =
                i32::try_from(ticks * 50).map_err(|_| "Native server timestep overflow")?;
            self.server = self
                .server
                .checked_add(advance)
                .ok_or("Native server clock overflow")?;
            self.remainder -= std::time::Duration::from_millis(advance as u64);
            self.snapshot = Some(self.server);
        }
        let Some(snapshot) = self.snapshot else {
            return Ok(());
        };
        if self.render.is_none() || self.paused {
            self.render = Some(snapshot);
            self.delta = snapshot - realtime;
            self.extrapolated = false;
            if self.paused {
                self.interpolation = 40;
            }
            self.paused = false;
        }
        let candidate = self.delta + realtime;
        self.render = Some(candidate.max(self.render.unwrap()));
        self.extrapolated |= snapshot - self.interpolation <= candidate;
        if ticks == 0 {
            return Ok(());
        }
        self.interpolation = if self.interpolation < 11 {
            5
        } else {
            self.interpolation - 5
        };
        let target = snapshot - realtime - self.interpolation;
        let interval = if ticks > 1 { 50 } else { snapshot - previous };
        let desired = target - interval;
        let error = desired - self.delta;
        let magnitude = if error <= 0 {
            -error
        } else if interval as f32 <= scale * 1000. {
            (error - interval).max(0)
        } else {
            error
        };
        if magnitude as f32 > scale * 1000. {
            self.render = Some(snapshot);
            self.delta = target;
        } else if magnitude as f32 > scale * 100. {
            self.delta = (self.delta + desired) >> 1;
        } else if self.extrapolated {
            self.extrapolated = false;
            self.delta -= 2;
        } else {
            self.delta += (desired - self.delta).signum();
        }
        Ok(())
    }
}

#[derive(Component)]
#[require(NativeArtInputs)]
pub(crate) struct NativeShaderStart(pub f32);

pub(super) fn begin_native_time(mut clock: ResMut<NativeClock>) {
    clock.running = true;
}

pub(super) fn shutdown_native_time(
    mut clock: ResMut<NativeClock>,
    mut inputs: Query<&mut NativeArtInputs>,
) {
    *clock = NativeClock::default();
    for mut input in &mut inputs {
        input.shader_time = None;
    }
}

pub(super) fn prepare_native_time_owners(
    mut commands: Commands,
    rigs: Query<&crate::presentation::Rig>,
    extras: Query<(Entity, &GltfExtras), Added<GltfExtras>>,
    starts: Query<(), With<NativeShaderStart>>,
    inputs: Query<&NativeArtInputs>,
) {
    let mut owners: HashSet<_> = rigs.iter().map(|rig| rig.owner).collect();
    owners.extend(extras.iter().filter_map(|(entity, extras)| {
        serde_json::from_str::<Value>(&extras.value)
            .ok()
            .filter(|value| value["static_instance"] == true)
            .map(|_| entity)
    }));
    for owner in owners {
        if starts.contains(owner) {
            continue;
        }
        let input = inputs.get(owner).ok();
        let start = input
            .and_then(|input| input.shader_time)
            .map_or(0., |(_, start)| start);
        let mut entity = commands.entity(owner);
        entity.insert(NativeShaderStart(start));
    }
}

pub(crate) fn advance_native_time(
    time: Res<Time>,
    virtual_time: Res<Time<Virtual>>,
    state: Res<State<crate::GameState>>,
    mut clock: ResMut<NativeClock>,
    mut inputs: Query<(&NativeShaderStart, &mut NativeArtInputs)>,
    mut failure: ResMut<crate::assets::LoadFailure>,
    mut next: ResMut<NextState<crate::GameState>>,
) {
    if let Err(error) = clock.advance(
        time.delta(),
        **state == crate::GameState::Paused,
        virtual_time.relative_speed(),
    ) {
        failure.0 = error;
        next.set(crate::GameState::LoadError);
        return;
    }
    for (start, mut input) in &mut inputs {
        input.shader_time = clock.render.map(|milliseconds| (milliseconds, start.0));
    }
}

pub(super) fn material_scene_active(state: Res<State<crate::GameState>>) -> bool {
    matches!(
        **state,
        crate::GameState::PreparingShaders
            | crate::GameState::Playing
            | crate::GameState::Paused
            | crate::GameState::GameOver
            | crate::GameState::Ending
    )
}

pub(super) fn advance_time(time: Res<Time>, mut clock: ResMut<MaterialTime>) {
    clock.0 += time.delta_secs_f64();
}

pub(super) fn register_prepared(
    library: &mut SurfaceLibrary,
    surface: &Value,
    channels: &[(PreparedTexture, Handle<Image>)],
) -> Result<(), String> {
    let Some(identity) = surface["identity"].as_str() else {
        return Err("Prepared surface identity is missing".into());
    };
    validate_animation(surface)?;
    if let Some(existing) = library.definitions.get_mut(identity) {
        if existing.metadata != *surface {
            return Err(format!(
                "Conflicting packed definitions for material {identity}"
            ));
        }
        existing.channels.extend(
            channels
                .iter()
                .map(|(texture, image)| ((texture.role.clone(), texture.layer), image.clone())),
        );
        return Ok(());
    }
    library.definitions.insert(
        identity.into(),
        SurfaceDefinition {
            presentation: surface["presentation"].clone(),
            channels: channels
                .iter()
                .map(|(texture, image)| ((texture.role.clone(), texture.layer), image.clone()))
                .collect(),
            metadata: surface.clone(),
            frames: Vec::new(),
        },
    );
    Ok(())
}

pub(super) fn apply_factors(material: &mut StandardMaterial, factors: &Value) {
    material.uv_transform =
        super::pbr::uv_transform(factors).expect("Validated prepared UV transform");
    let scalar = |key: &str, default: f32| factors[key].as_f64().map_or(default, |v| v as f32);
    material.perceptual_roughness = scalar("roughness", material.perceptual_roughness);
    material.metallic = scalar("metallic", material.metallic);
    material.reflectance = scalar("reflectance", material.reflectance);
    material.specular_transmission = scalar("specular_transmission", 0.);
    material.diffuse_transmission = scalar("diffuse_transmission", 0.);
    material.ior = scalar("ior", 1.5);
    material.thickness = scalar("thickness", 0.);
    material.unlit = factors["unlit"].as_bool().unwrap_or(false);
    material.specular_tint =
        if let Some(color) = factors["specular_tint"].as_array().filter(|v| v.len() == 3) {
            Color::linear_rgb(
                color[0].as_f64().unwrap_or(1.) as f32,
                color[1].as_f64().unwrap_or(1.) as f32,
                color[2].as_f64().unwrap_or(1.) as f32,
            )
        } else {
            Color::WHITE
        };
    if let Some(color) = factors["base_color"].as_array().filter(|v| v.len() == 4) {
        material.base_color = Color::linear_rgba(
            color[0].as_f64().unwrap_or(1.) as f32,
            color[1].as_f64().unwrap_or(1.) as f32,
            color[2].as_f64().unwrap_or(1.) as f32,
            color[3].as_f64().unwrap_or(1.) as f32,
        );
    }
    if let Some(color) = factors["emissive"].as_array().filter(|v| v.len() == 3) {
        material.emissive = LinearRgba::rgb(
            color[0].as_f64().unwrap_or(0.) as f32,
            color[1].as_f64().unwrap_or(0.) as f32,
            color[2].as_f64().unwrap_or(0.) as f32,
        );
    }
}

#[derive(Component)]
pub(super) struct StandardReady;

pub(crate) fn reset_binding(entity: &mut bevy::ecs::system::EntityCommands) {
    entity.remove::<(
        PbrReady,
        StandardReady,
        AnimatedMaterial,
        ScalarAnimation,
        UvAnimation,
    )>();
}

pub(super) fn refresh_owners(
    mut surfaces: Query<
        (
            Entity,
            Option<&mut AnimatedMaterial>,
            Option<&mut ScalarAnimation>,
            Option<&mut UvAnimation>,
            Option<&mut FlagMotion>,
        ),
        Or<(
            With<AnimatedMaterial>,
            With<ScalarAnimation>,
            With<UvAnimation>,
            With<FlagMotion>,
        )>,
    >,
    parents: Query<&ChildOf>,
    rigs: Query<&crate::presentation::Rig>,
    stones: Query<(), With<crate::staff_puzzles::IceStone>>,
    inputs: Query<(), With<NativeArtInputs>>,
) {
    for (entity, animated, scalar, uv, flag) in &mut surfaces {
        let mut ancestor = Some(entity);
        let mut owner = None;
        while let Some(current) = ancestor {
            if inputs.contains(current) || stones.contains(current) {
                owner = Some(current);
                break;
            }
            if let Ok(rig) = rigs.get(current) {
                owner = Some(rig.owner);
                break;
            }
            ancestor = parents.get(current).ok().map(ChildOf::parent);
        }
        if let Some(mut animation) = animated {
            animation.owner = owner;
        }
        if let Some(mut animation) = scalar {
            animation.owner = owner;
        }
        if let Some(mut animation) = uv {
            animation.owner = owner;
        }
        if let Some(mut animation) = flag {
            animation.owner = owner;
        }
    }
}

#[derive(Clone)]
struct Frame {
    channels: HashMap<String, Handle<Image>>,
    factors: Value,
}
#[derive(Component)]
pub(super) struct AnimatedMaterial {
    driver: String,
    fps: f32,
    time_frames: usize,
    state_threshold: f32,
    owner: Option<Entity>,
    frames: Vec<Frame>,
    selected: usize,
}

#[derive(Component, Default)]
pub(crate) struct NativeArtInputs {
    pub script_vectors: [[f32; 4]; 4],
    pub script_available: u8,
    pub time_offset: f32,
    pub shader_time: Option<(i32, f32)>,
}

pub(super) fn game_time(milliseconds: i32, start_seconds: f32) -> Result<[f32; 4], String> {
    let seconds = milliseconds as f32 * f32::from_bits(0x3a83126f) - start_seconds;
    if !seconds.is_finite() {
        return Err("Native render-game time input is non-finite".into());
    }
    let fraction = seconds - seconds.floor();
    let (sine, cosine) = (fraction * f32::from_bits(0x40c90fdb)).sin_cos();
    Ok([sine, cosine, fraction, seconds])
}

#[derive(Component, serde::Deserialize)]
pub(super) struct ScalarAnimation {
    #[serde(default = "elapsed_time_mode")]
    time_mode: String,
    lookup: Vec<f32>,
    sampler: u32,
    speed: f32,
    minimum: f32,
    maximum: f32,
    script_vector: usize,
    script_component: usize,
    #[serde(skip)]
    owner: Option<Entity>,
}

fn elapsed_time_mode() -> String {
    "elapsed".into()
}

#[derive(Component, serde::Deserialize)]
pub(super) struct UvAnimation {
    translation: [f32; 2],
    time_mode: String,
    #[serde(skip)]
    owner: Option<Entity>,
    #[serde(skip)]
    base: bevy::math::Affine2,
}

#[derive(Clone, serde::Deserialize)]
struct FlagParameters {
    parameters: [f32; 4],
    phase_uv: [[f32; 2]; 3],
    phase_time: [f32; 3],
    displacement_amplitude: [f32; 3],
    normal_cosine_u: [f32; 3],
    normal_cosine_v: [f32; 3],
    normal_derivative_scale: f32,
}

impl FlagParameters {
    fn read(value: &Value) -> Result<Self, String> {
        let parameters: Self = serde_json::from_value(value.clone())
            .map_err(|error| format!("Invalid source flag parameters: {error}"))?;
        if value["time"]["code_constant"] != 25
            || value["time"]["component"] != 3
            || value["time"]["operation"] != "add_parameter_y_then_multiply_parameter_x"
            || value["vertex_alpha"] != "motion_amplitude_only"
            || value["uv"] != "one_minus_uv0"
            || value["fragment_vertex_alpha"] != 1.
            || value["displacement_axis"] != "decoded_model_normal"
            || value["normal_operation"]
                != "add_vertex_alpha_scaled_cosine_derivatives_in_source_tangent_basis_then_normalize"
            || parameters
                .parameters
                .iter()
                .chain(parameters.phase_uv.iter().flatten())
                .chain(&parameters.phase_time)
                .chain(&parameters.displacement_amplitude)
                .chain(&parameters.normal_cosine_u)
                .chain(&parameters.normal_cosine_v)
                .chain([&parameters.normal_derivative_scale])
                .any(|value| !value.is_finite())
        {
            return Err("Unsupported or non-finite source flag motion".into());
        }
        Ok(parameters)
    }
}

#[derive(Component)]
pub(super) struct FlagMotion {
    original: Handle<Mesh>,
    target: Handle<Mesh>,
    owner: Option<Entity>,
    parameters: FlagParameters,
    positions: Vec<[f32; 3]>,
    normals: Vec<[f32; 3]>,
    tangents: Vec<[f32; 3]>,
    uv: Vec<[f32; 2]>,
    alpha: Vec<f32>,
    sign: Vec<f32>,
    last_time: Option<f32>,
    skin: Option<FlagSkin>,
    last_pose: Vec<Mat4>,
}

struct FlagSkin {
    original: SkinnedMesh,
    joints: Vec<[u16; 4]>,
    weights: Vec<[f32; 4]>,
    packed_basis: Vec<[u32; 2]>,
}

fn flag_placement(matrix: Mat4, unit: bool) -> Result<(), String> {
    let basis = Mat3::from_mat4(matrix);
    let axes = [basis.x_axis, basis.y_axis, basis.z_axis];
    let scale = axes[0].length_squared();
    if !matrix.is_finite()
        || !matrix.row(3).abs_diff_eq(Vec4::W, 0.0001)
        || scale <= 0.
        || basis.determinant() <= 0.
        || axes
            .iter()
            .any(|axis| (axis.length_squared() - scale).abs() > scale * 0.0001)
        || [(0, 1), (0, 2), (1, 2)]
            .into_iter()
            .any(|(a, b)| axes[a].dot(axes[b]).abs() > scale * 0.0001)
        || (unit && (scale - 1.).abs() > 0.0001)
    {
        return Err(if unit {
            "Native skinned flag requires a proven unit rigid placement"
        } else {
            "Native flag source normal transform requires a proven rigid uniform-scale placement"
        }
        .into());
    }
    Ok(())
}

fn flag_skin_basis(word: u32) -> Vec3 {
    let [x, y, z] =
        [0, 10, 20].map(|shift| (((word >> shift & 1023) ^ 512) as i32 - 512) as f32 * (1. / 511.));
    Vec3::new(x, z, -y)
}

fn flag_repack_basis(value: Vec3) -> Vec3 {
    let native = [value.x, -value.z, value.y];
    let word = native
        .into_iter()
        .zip([511., 523264., 535822336.])
        .zip([0x3ff, 0xffc00, 0x3ff00000])
        .fold(0, |word, ((value, scale), mask)| {
            word | ((value.clamp(-1., 1.) * scale) as i32 as u32 & mask)
        });
    let [x, y, z] = [0, 10, 20].map(|shift| {
        let value = (word >> shift & 1023) as f32 / 1023.;
        if value >= 0.5 {
            2. * value - 2.
        } else {
            2. * value
        }
    });
    Vec3::new(x, z, -y)
}

impl FlagMotion {
    pub(super) fn deformation_time(&self) -> Option<f32> {
        self.last_time
    }

    pub(super) fn deformation_pose(&self) -> &[Mat4] {
        &self.last_pose
    }

    fn set_skin(&mut self, source: &Mesh, skin: SkinnedMesh) -> Result<(), String> {
        let Some(VertexAttributeValues::Uint16x4(joints)) =
            source.attribute(Mesh::ATTRIBUTE_JOINT_INDEX)
        else {
            return Err("Native flag source joint indices are missing".into());
        };
        let Some(VertexAttributeValues::Float32x4(weights)) =
            source.attribute(Mesh::ATTRIBUTE_JOINT_WEIGHT)
        else {
            return Err("Native flag source joint weights are missing".into());
        };
        let Some(VertexAttributeValues::Uint32x2(packed_basis)) =
            source.attribute(FLAG_PACKED_BASIS)
        else {
            return Err("Native flag original packed basis is missing".into());
        };
        if skin.joints.is_empty()
            || [joints.len(), weights.len(), packed_basis.len()]
                .into_iter()
                .any(|count| count != self.positions.len())
            || joints.iter().zip(weights).any(|(indices, weights)| {
                weights
                    .iter()
                    .any(|weight| !weight.is_finite() || !(0. ..=1.).contains(weight))
                    || (weights.iter().sum::<f32>() - 1.).abs() > 0.000001
                    || indices.iter().zip(weights).any(|(&joint, &weight)| {
                        weight != 0. && joint as usize >= skin.joints.len()
                    })
            })
        {
            return Err("Native flag source skin bindings are invalid".into());
        }
        self.skin = Some(FlagSkin {
            original: skin,
            joints: joints.clone(),
            weights: weights.clone(),
            packed_basis: packed_basis.clone(),
        });
        Ok(())
    }

    fn new(
        original: Handle<Mesh>,
        owner: Entity,
        parameters: FlagParameters,
        source: &Mesh,
    ) -> Result<(Self, Mesh), String> {
        let _ = source
            .try_attributes()
            .map_err(|error| format!("Native flag CPU geometry is unavailable: {error}"))?;
        let Some(VertexAttributeValues::Float32x3(positions)) =
            source.attribute(Mesh::ATTRIBUTE_POSITION)
        else {
            return Err("Native flag positions are missing".into());
        };
        let Some(VertexAttributeValues::Float32x3(normals)) = source.attribute(FLAG_SOURCE_NORMAL)
        else {
            return Err("Native flag raw decoded normals are missing".into());
        };
        let Some(VertexAttributeValues::Float32x3(tangents)) =
            source.attribute(FLAG_SOURCE_TANGENT)
        else {
            return Err("Native flag raw decoded tangents are missing".into());
        };
        let Some(VertexAttributeValues::Float32x4(tangent_signs)) =
            source.attribute(Mesh::ATTRIBUTE_TANGENT)
        else {
            return Err("Native flag tangent signs are missing".into());
        };
        let Some(VertexAttributeValues::Float32x2(uv)) = source.attribute(Mesh::ATTRIBUTE_UV_0)
        else {
            return Err("Native flag UV0 is missing".into());
        };
        let Some(VertexAttributeValues::Float32x4(colors)) =
            source.attribute(Mesh::ATTRIBUTE_COLOR)
        else {
            return Err("Native flag motion alpha is missing".into());
        };
        if positions.is_empty()
            || [
                normals.len(),
                tangents.len(),
                tangent_signs.len(),
                uv.len(),
                colors.len(),
            ]
            .into_iter()
            .any(|count| count != positions.len())
            || positions
                .iter()
                .flatten()
                .chain(normals.iter().flatten())
                .chain(tangents.iter().flatten())
                .chain(tangent_signs.iter().flatten())
                .chain(uv.iter().flatten())
                .chain(colors.iter().flatten())
                .any(|value| !value.is_finite())
        {
            return Err("Native flag geometry arrays are invalid".into());
        }
        let mut target = source.clone();
        target.remove_attribute(Mesh::ATTRIBUTE_JOINT_INDEX);
        target.remove_attribute(Mesh::ATTRIBUTE_JOINT_WEIGHT);
        target.asset_usage = bevy::asset::RenderAssetUsages::default();
        target.insert_attribute(
            Mesh::ATTRIBUTE_COLOR,
            colors
                .iter()
                .map(|color| [color[0], color[1], color[2], 1.])
                .collect::<Vec<_>>(),
        );
        Ok((
            Self {
                original,
                target: Handle::default(),
                owner: Some(owner),
                parameters,
                positions: positions.clone(),
                normals: normals.clone(),
                tangents: tangents.clone(),
                uv: uv.clone(),
                alpha: colors.iter().map(|color| color[3]).collect(),
                sign: tangent_signs.iter().map(|tangent| tangent[3]).collect(),
                last_time: None,
                skin: None,
                last_pose: Vec::new(),
            },
            target,
        ))
    }

    fn deform_vertex(
        &self,
        index: usize,
        time: f32,
        position: Vec3,
        normal: Vec3,
        tangent: Vec3,
    ) -> Result<(Vec3, Vec3), String> {
        let parameters = &self.parameters;
        let uv = Vec2::ONE - Vec2::from_array(self.uv[index]);
        let phases: [f32; 3] = std::array::from_fn(|wave| {
            uv.dot(Vec2::from_array(parameters.phase_uv[wave])) + time * parameters.phase_time[wave]
        });
        let mut height = 0.;
        let mut u = 0.;
        let mut v = 0.;
        for wave in 0..3 {
            let (sine, cosine) = phases[wave].sin_cos();
            height += parameters.displacement_amplitude[wave] * sine;
            u += parameters.normal_cosine_u[wave] * cosine;
            v += parameters.normal_cosine_v[wave] * cosine;
        }
        let bitangent = normal.cross(tangent) * self.sign[index];
        let position = position + normal * (height * self.alpha[index] * 0.0254);
        let normal = (normal
            + (u * tangent + v * bitangent)
                * (self.alpha[index] * parameters.normal_derivative_scale))
            .normalize();
        if !position.is_finite() || !normal.is_finite() {
            return Err("Native flag deformation produced a non-finite vertex".into());
        }
        Ok((position, normal))
    }

    fn deform(&self, seconds: f32) -> Result<(Vec<[f32; 3]>, Vec<[f32; 3]>), String> {
        let time = (seconds + self.parameters.parameters[1]) * self.parameters.parameters[0];
        let mut positions = Vec::with_capacity(self.positions.len());
        let mut normals = Vec::with_capacity(self.positions.len());
        for index in 0..self.positions.len() {
            let (position, normal) = self.deform_vertex(
                index,
                time,
                Vec3::from_array(self.positions[index]),
                Vec3::from_array(self.normals[index]),
                Vec3::from_array(self.tangents[index]),
            )?;
            positions.push(position.to_array());
            normals.push(normal.to_array());
        }
        Ok((positions, normals))
    }

    fn deform_skinned(
        &self,
        seconds: f32,
        placement: Mat4,
        pose: &[Mat4],
    ) -> Result<(Vec<[f32; 3]>, Vec<[f32; 3]>, Vec<[f32; 4]>), String> {
        let skin = self
            .skin
            .as_ref()
            .ok_or("Native flag source skin is unavailable")?;
        let inverse = placement.inverse();
        let time = (seconds + self.parameters.parameters[1]) * self.parameters.parameters[0];
        let mut positions = Vec::with_capacity(self.positions.len());
        let mut normals = Vec::with_capacity(self.positions.len());
        let mut tangents = Vec::with_capacity(self.positions.len());
        for index in 0..self.positions.len() {
            let mut matrix = Mat4::ZERO;
            for influence in 0..4 {
                let weight = skin.weights[index][influence];
                if weight != 0. {
                    matrix += *pose
                        .get(skin.joints[index][influence] as usize)
                        .ok_or("Native flag current joint pose is unavailable")?
                        * weight;
                }
            }
            let position = matrix.transform_point3(Vec3::from_array(self.positions[index]));
            let normal = flag_repack_basis(
                matrix.transform_vector3(flag_skin_basis(skin.packed_basis[index][0])),
            );
            let tangent = flag_repack_basis(
                matrix.transform_vector3(flag_skin_basis(skin.packed_basis[index][1])),
            );
            let (position, normal) = self.deform_vertex(index, time, position, normal, tangent)?;
            let position = inverse.transform_point3(position);
            let normal = inverse.transform_vector3(normal).normalize();
            let tangent = inverse.transform_vector3(tangent).normalize();
            if !position.is_finite() || !normal.is_finite() || !tangent.is_finite() {
                return Err("Native flag skinned geometry is non-finite".into());
            }
            positions.push(position.to_array());
            normals.push(normal.to_array());
            tangents.push([tangent.x, tangent.y, tangent.z, self.sign[index]]);
        }
        Ok((positions, normals, tangents))
    }

    fn bounds(&self) -> bevy::camera::primitives::Aabb {
        let amplitude: f32 = self
            .parameters
            .displacement_amplitude
            .iter()
            .map(|value| value.abs())
            .sum();
        let mut minimum = Vec3::splat(f32::INFINITY);
        let mut maximum = Vec3::splat(f32::NEG_INFINITY);
        for (index, position) in self.positions.iter().enumerate() {
            let position = Vec3::from_array(*position);
            let extent = Vec3::from_array(self.normals[index]).abs()
                * (amplitude * self.alpha[index].abs() * 0.0254);
            minimum = minimum.min(position - extent);
            maximum = maximum.max(position + extent);
        }
        bevy::camera::primitives::Aabb::from_min_max(minimum, maximum)
    }

    fn source_bounds(&self) -> bevy::camera::primitives::Aabb {
        let minimum = self
            .positions
            .iter()
            .fold(Vec3::splat(f32::INFINITY), |bounds, position| {
                bounds.min(Vec3::from_array(*position))
            });
        let maximum = self
            .positions
            .iter()
            .fold(Vec3::splat(f32::NEG_INFINITY), |bounds, position| {
                bounds.max(Vec3::from_array(*position))
            });
        bevy::camera::primitives::Aabb::from_min_max(minimum, maximum)
    }
}

fn validate_animation(surface: &Value) -> Result<(), String> {
    if !surface["source_flag_motion"].is_null() {
        FlagParameters::read(&surface["source_flag_motion"])?;
    }
    if !surface["animation"].is_null() {
        let animation = &surface["animation"];
        let frames = animation["frames"]
            .as_array()
            .filter(|frames| !frames.is_empty())
            .ok_or("Material animation has no frames")?;
        match animation["driver"].as_str() {
            Some("time" | "staff_charge") => {
                if animation["fps"]
                    .as_f64()
                    .is_none_or(|fps| !fps.is_finite() || fps <= 0.)
                {
                    return Err("Material animation has invalid source frame rate".into());
                }
                if animation["driver"] == "staff_charge" {
                    let count = animation["time_frames"]
                        .as_u64()
                        .filter(|count| *count > 0)
                        .ok_or("Staff animation has no time frame count")?;
                    if count.checked_mul(2) != Some(frames.len() as u64)
                        || animation["state_threshold"].as_f64().is_none()
                    {
                        return Err("Staff material animation has invalid state banks".into());
                    }
                }
            }
            Some("frozen_stone") => {}
            _ => return Err("Unknown source material animation driver".into()),
        }
        for frame in frames {
            super::pbr::textures(&frame["channels"])?;
            super::pbr::uv_transform(&frame["standard"])?;
        }
    }
    if !surface["scalar_animation"].is_null() {
        let animation: ScalarAnimation =
            serde_json::from_value(surface["scalar_animation"].clone())
                .map_err(|error| format!("Invalid scalar material animation: {error}"))?;
        if animation.lookup.is_empty()
            || !matches!(animation.time_mode.as_str(), "elapsed" | "native_elapsed")
            || animation.script_vector >= 4
            || animation.script_component >= 4
            || !animation
                .lookup
                .iter()
                .chain([&animation.speed, &animation.minimum, &animation.maximum])
                .all(|value| value.is_finite())
        {
            return Err("Invalid scalar material animation domain".into());
        }
    }
    if !surface["uv_animation"].is_null() {
        let animation: UvAnimation = serde_json::from_value(surface["uv_animation"].clone())
            .map_err(|error| format!("Invalid UV material animation: {error}"))?;
        if !matches!(
            animation.time_mode.as_str(),
            "elapsed" | "fractional_second" | "native_fractional_second"
        ) || !animation.translation.iter().all(|value| value.is_finite())
        {
            return Err("Unsupported UV material animation controls".into());
        }
    }
    Ok(())
}

pub(super) fn convert(
    mut commands: Commands,
    query: Query<
        (
            Entity,
            &MeshMaterial3d<StandardMaterial>,
            &GltfMaterialName,
            &Mesh3d,
            Has<super::DynamicSurface>,
            Option<&SkinnedMesh>,
            Option<&FlagMotion>,
        ),
        (With<PbrReady>, With<SurfaceReady>, Without<StandardReady>),
    >,
    extras: Query<&GltfExtras>,
    parents: Query<&ChildOf>,
    rigs: Query<&crate::presentation::Rig>,
    stones: Query<(), With<crate::staff_puzzles::IceStone>>,
    art_inputs: Query<(), With<NativeArtInputs>>,
    mut library: ResMut<SurfaceLibrary>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
    server: Res<AssetServer>,
    mut status: ResMut<OpaqueSurfaceStatus>,
    mut failure: ResMut<crate::assets::LoadFailure>,
    mut next: ResMut<NextState<crate::GameState>>,
) {
    status.pending = 0;
    for (entity, original, name, mesh, dynamic, skin, flag) in &query {
        let Some(base) = materials.get(&original.0).cloned() else {
            status.pending += 1;
            continue;
        };
        let Some(definition) = library.definitions.get_mut(&name.0) else {
            commands.entity(entity).insert(StandardReady);
            status.world += 1;
            continue;
        };
        let mut current = Some(entity);
        let mut owner = None;
        let mut is_static = false;
        while let Some(ancestor) = current {
            if owner.is_none()
                && let Ok(rig) = rigs.get(ancestor)
            {
                owner = Some(rig.owner);
            }
            if owner.is_none() && stones.contains(ancestor) {
                owner = Some(ancestor);
            }
            if owner.is_none() && art_inputs.contains(ancestor) {
                owner = Some(ancestor);
            }
            is_static |= extras
                .get(ancestor)
                .ok()
                .and_then(|v| serde_json::from_str::<Value>(&v.value).ok())
                .is_some_and(|v| v["static_instance"] == true);
            current = parents.get(ancestor).ok().map(ChildOf::parent);
        }
        let animation = &definition.metadata["animation"];
        let live_mesh = dynamic
            || !definition.metadata["uv_animation"].is_null()
            || !definition.metadata["source_flag_motion"].is_null();
        if definition.frames.is_empty()
            && let Some(source_frames) = animation["frames"].as_array()
        {
            for frame in source_frames {
                let textures = super::pbr::textures(&frame["channels"]).unwrap();
                definition.frames.push(Frame {
                    channels: textures
                        .into_iter()
                        .map(|texture| (texture.role.clone(), super::pbr::load(&server, &texture)))
                        .collect(),
                    factors: frame["standard"].clone(),
                });
            }
        }
        let frames = &definition.frames;
        if let Some((image, error)) = frames
            .iter()
            .flat_map(|frame| frame.channels.values())
            .find_map(|image| match server.get_load_state(image.id()) {
                Some(bevy::asset::LoadState::Failed(error)) => Some((image, error)),
                _ => None,
            })
        {
            failure.0 = format!(
                "Material animation {} texture {:?}: {error}",
                name.0,
                image.path()
            );
            next.set(crate::GameState::LoadError);
            return;
        }
        if frames
            .iter()
            .flat_map(|f| f.channels.values())
            .any(|image| !server.is_loaded_with_dependencies(image.id()))
        {
            status.pending += 1;
            continue;
        }
        let mut flag_original = None;
        if !definition.metadata["source_flag_motion"].is_null() {
            let result = (|| -> Result<(FlagMotion, Mesh), String> {
                let owner = owner.ok_or("Native flag render-game time owner is not connected")?;
                let parameters = FlagParameters::read(&definition.metadata["source_flag_motion"])?;
                let source_handle = flag
                    .filter(|flag| flag.target == mesh.0)
                    .map_or(&mesh.0, |flag| &flag.original);
                let source = meshes
                    .get(source_handle)
                    .ok_or("Native flag source mesh is unavailable")?;
                let (mut motion, target) =
                    FlagMotion::new(source_handle.clone(), owner, parameters, source)?;
                if let Some(skin) = skin.cloned().or_else(|| {
                    flag.filter(|flag| flag.target == mesh.0)
                        .and_then(|flag| flag.skin.as_ref())
                        .map(|skin| skin.original.clone())
                }) {
                    motion.set_skin(source, skin)?;
                }
                Ok((motion, target))
            })();
            match result {
                Ok((mut flag, target)) => {
                    flag_original = Some(flag.original.id());
                    let handle = meshes.add(target);
                    flag.target = handle.clone();
                    let bounds = flag.bounds();
                    commands.entity(entity).remove::<SkinnedMesh>().insert((
                        Mesh3d(handle),
                        bounds,
                        flag,
                    ));
                }
                Err(error) => {
                    failure.0 = format!("Material {}: {error}", name.0);
                    next.set(crate::GameState::LoadError);
                    return;
                }
            }
        } else if let Some(flag) = flag {
            if flag.target == mesh.0 {
                commands
                    .entity(entity)
                    .insert((Mesh3d(flag.original.clone()), flag.source_bounds()));
            }
            if skin.is_none()
                && let Some(source) = &flag.skin
            {
                commands.entity(entity).insert(source.original.clone());
            }
            commands.entity(entity).remove::<FlagMotion>();
        }
        if !frames.is_empty()
            || !definition.metadata["scalar_animation"].is_null()
            || !definition.metadata["uv_animation"].is_null()
        {
            commands
                .entity(entity)
                .insert(MeshMaterial3d(materials.add(base.clone())));
        }
        if !definition.metadata["scalar_animation"].is_null() {
            let mut animation: ScalarAnimation =
                serde_json::from_value(definition.metadata["scalar_animation"].clone())
                    .expect("Source-built scalar material animation");
            animation.owner = owner;
            commands.entity(entity).insert(animation);
        }
        if !definition.metadata["uv_animation"].is_null() {
            let mut animation: UvAnimation =
                serde_json::from_value(definition.metadata["uv_animation"].clone())
                    .expect("Source-built UV material animation");
            animation.owner = owner;
            animation.base = super::pbr::uv_transform(&definition.metadata["standard"])
                .expect("Validated prepared UV transform");
            commands.entity(entity).insert(animation);
        }
        if !frames.is_empty() {
            commands.entity(entity).insert(AnimatedMaterial {
                driver: animation["driver"].as_str().unwrap().into(),
                fps: animation["fps"].as_f64().unwrap_or(0.) as f32,
                time_frames: animation["time_frames"].as_u64().unwrap_or(1).max(1) as usize,
                state_threshold: animation["state_threshold"].as_f64().unwrap_or(1.) as f32,
                owner,
                frames: frames.clone(),
                selected: usize::MAX,
            });
        }
        status.static_models += is_static as usize;
        status.world += (!is_static) as usize;
        status.normal += base.normal_map_texture.is_some() as usize;
        status.response_maps += base.metallic_roughness_texture.is_some() as usize;
        status.masked += matches!(base.alpha_mode, AlphaMode::Mask(_)) as usize;
        status.transparent +=
            (!matches!(base.alpha_mode, AlphaMode::Opaque | AlphaMode::Mask(_))) as usize;
        status.water += (definition.metadata["family"] == "water") as usize;
        commands.entity(entity).insert(StandardReady);
        if let Some(original) = flag_original {
            library.live_meshes.insert(original);
        }
        library.prepared_meshes.insert(mesh.0.id());
        if live_mesh {
            library.live_meshes.insert(mesh.0.id());
        }
    }
    status.shared_materials = materials.len();
}

pub(super) fn animate(
    time: Res<MaterialTime>,
    inputs: Query<&NativeArtInputs>,
    server: Res<AssetServer>,
    inventories: Query<&crate::weapons::WeaponInventory>,
    stones: Query<&crate::staff_puzzles::FrozenStone>,
    mut entities: Query<(&mut AnimatedMaterial, &MeshMaterial3d<StandardMaterial>)>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for (mut animation, handle) in &mut entities {
        let count = animation.frames.len();
        let seconds = time.0
            - animation
                .owner
                .and_then(|owner| inputs.get(owner).ok())
                .map_or(0., |input| input.time_offset as f64);
        let phase = |count: usize| {
            (seconds * animation.fps as f64)
                .floor()
                .rem_euclid(count as f64) as usize
        };
        let selected = match animation.driver.as_str() {
            "staff_charge" => {
                let charged = animation
                    .owner
                    .and_then(|owner| inventories.get(owner).ok())
                    .is_some_and(|inventory| {
                        inventory.charge_level as f32 >= animation.state_threshold
                    });
                let phase = phase(animation.time_frames);
                (charged as usize * animation.time_frames + phase).min(count - 1)
            }
            "frozen_stone" => ((animation
                .owner
                .and_then(|owner| stones.get(owner).ok())
                .map_or(0., |stone| stone.amount)
                .clamp(0., 1.)
                * (count - 1) as f32)
                .round() as usize)
                .min(count - 1),
            _ => phase(count),
        };
        if animation.selected == selected {
            continue;
        }
        if animation.frames[selected]
            .channels
            .values()
            .any(|image| !server.is_loaded_with_dependencies(image.id()))
        {
            continue;
        }
        if let Some(mut material) = materials.get_mut(&handle.0) {
            let frame = &animation.frames[selected];
            material.base_color_texture = frame.channels.get("color").cloned();
            material.normal_map_texture = frame.channels.get("normal").cloned();
            material.metallic_roughness_texture = frame.channels.get("response").cloned();
            material.occlusion_texture = frame
                .channels
                .get("occlusion")
                .or_else(|| frame.channels.get("response"))
                .cloned();
            material.emissive_texture = frame.channels.get("emissive").cloned();
            material.specular_tint_texture = frame
                .channels
                .get("reflectance")
                .or_else(|| frame.channels.get("specular_gloss"))
                .cloned();
            apply_factors(&mut material, &frame.factors);
            animation.selected = selected;
        }
    }
}

pub(super) fn animate_uvs(
    time: Res<MaterialTime>,
    state: Res<State<crate::GameState>>,
    clock: Option<Res<NativeClock>>,
    starts: Query<(), With<NativeShaderStart>>,
    inputs: Query<&NativeArtInputs>,
    entities: Query<(&UvAnimation, &MeshMaterial3d<StandardMaterial>)>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut status: ResMut<OpaqueSurfaceStatus>,
    mut failure: ResMut<crate::assets::LoadFailure>,
    mut next: ResMut<NextState<crate::GameState>>,
) {
    for (animation, handle) in &entities {
        if animation.time_mode != "native_fractional_second" && **state != crate::GameState::Playing
        {
            continue;
        }
        let input = animation.owner.and_then(|owner| inputs.get(owner).ok());
        if animation.time_mode == "native_fractional_second"
            && input.is_none_or(|input| input.shader_time.is_none())
            && animation.owner.is_some_and(|owner| starts.contains(owner))
            && clock.as_ref().is_some_and(|clock| clock.render.is_none())
        {
            status.pending += 1;
            continue;
        }
        let seconds = time.0 - input.map_or(0., |input| input.time_offset as f64);
        let phase = if animation.time_mode == "native_fractional_second" {
            let phase = input
                .and_then(|input| input.shader_time)
                .ok_or_else(|| {
                    "Native UV render-game milliseconds and shader-start input are not connected"
                        .to_owned()
                })
                .and_then(|(milliseconds, start)| game_time(milliseconds, start));
            match phase {
                Ok(time) => time[2] as f64,
                Err(error) => {
                    failure.0 = error;
                    next.set(crate::GameState::LoadError);
                    return;
                }
            }
        } else if animation.time_mode == "fractional_second" {
            seconds - seconds.floor()
        } else {
            seconds
        };
        let uv_transform = animation.base
            * bevy::math::Affine2::from_translation(
                Vec2::from_array(animation.translation) * phase as f32,
            );
        if let Some(material) = materials.get(&handle.0)
            && material.uv_transform != uv_transform
            && let Some(mut material) = materials.get_mut(&handle.0)
        {
            material.uv_transform = uv_transform;
        }
    }
}

pub(super) fn animate_flags(
    mut commands: Commands,
    inputs: Query<&NativeArtInputs>,
    clock: Option<Res<NativeClock>>,
    starts: Query<(), With<NativeShaderStart>>,
    mut entities: Query<(
        Entity,
        &Mesh3d,
        &GlobalTransform,
        Option<&SkinnedMesh>,
        &mut FlagMotion,
    )>,
    transforms: Query<&GlobalTransform>,
    bindposes: Res<Assets<SkinnedMeshInverseBindposes>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut status: ResMut<OpaqueSurfaceStatus>,
    mut failure: ResMut<crate::assets::LoadFailure>,
    mut next: ResMut<NextState<crate::GameState>>,
) {
    for (entity, handle, transform, skin, mut flag) in &mut entities {
        if flag.target != handle.0 || skin.is_some() {
            if flag.target == handle.0 {
                commands
                    .entity(entity)
                    .insert((Mesh3d(flag.original.clone()), flag.source_bounds()));
            }
            if skin.is_none()
                && let Some(source) = &flag.skin
            {
                commands.entity(entity).insert(source.original.clone());
            }
            commands
                .entity(entity)
                .remove::<(FlagMotion, StandardReady)>();
            continue;
        }
        if flag.owner.is_some_and(|owner| starts.contains(owner))
            && flag
                .owner
                .and_then(|owner| inputs.get(owner).ok())
                .is_none_or(|input| input.shader_time.is_none())
            && clock.as_ref().is_some_and(|clock| clock.render.is_none())
        {
            status.pending += 1;
            continue;
        }
        if meshes
            .get(&handle.0)
            .is_none_or(|mesh| mesh.try_attributes().is_err())
        {
            failure.0 = "Native flag per-object CPU mesh is unavailable".into();
            next.set(crate::GameState::LoadError);
            return;
        }
        let result: Result<_, String> = (|| {
            let placement = Mat4::from(transform.affine());
            flag_placement(placement, flag.skin.is_some())?;
            let mut pose = Vec::new();
            if let Some(skin) = &flag.skin {
                pose.push(placement);
                let bindposes = bindposes
                    .get(&skin.original.inverse_bindposes)
                    .ok_or("Native flag inverse bind poses are unavailable")?;
                if bindposes.len() != skin.original.joints.len() {
                    return Err("Native flag joint and inverse bind counts differ".into());
                }
                for (joint, bind) in skin.original.joints.iter().zip(bindposes.iter()) {
                    let world = transforms
                        .get(*joint)
                        .map_err(|_| "Native flag current joint transform is unavailable")?;
                    let matrix = Mat4::from(world.affine()) * *bind;
                    flag_placement(matrix, true)?;
                    pose.push(matrix);
                }
            }
            let (milliseconds, start) = flag
                .owner
                .and_then(|owner| inputs.get(owner).ok())
                .and_then(|input| input.shader_time)
                .ok_or(
                    "Native flag render-game milliseconds and shader-start input are not connected",
                )?;
            let seconds = game_time(milliseconds, start)?[3];
            if flag.last_time == Some(seconds) && flag.last_pose == pose {
                return Ok(None);
            }
            let (positions, normals, tangents) = if flag.skin.is_some() {
                let (positions, normals, tangents) =
                    flag.deform_skinned(seconds, placement, &pose[1..])?;
                (positions, normals, Some(tangents))
            } else {
                let (positions, normals) = flag.deform(seconds)?;
                (positions, normals, None)
            };
            Ok(Some((positions, normals, tangents, seconds, pose)))
        })();
        match result {
            Ok(Some((positions, normals, tangents, seconds, pose))) => {
                let Some(mut mesh) = meshes.get_mut(&handle.0) else {
                    failure.0 = "Native flag per-object CPU mesh is unavailable".into();
                    next.set(crate::GameState::LoadError);
                    return;
                };
                if flag.skin.is_some() {
                    let mut minimum = Vec3::splat(f32::INFINITY);
                    let mut maximum = Vec3::splat(f32::NEG_INFINITY);
                    for position in &positions {
                        let position = Vec3::from_array(*position);
                        minimum = minimum.min(position);
                        maximum = maximum.max(position);
                    }
                    commands
                        .entity(entity)
                        .insert(bevy::camera::primitives::Aabb::from_min_max(
                            minimum, maximum,
                        ));
                }
                mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
                mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
                if let Some(tangents) = tangents {
                    mesh.insert_attribute(Mesh::ATTRIBUTE_TANGENT, tangents);
                }
                flag.last_time = Some(seconds);
                flag.last_pose = pose;
            }
            Ok(None) => (),
            Err(error) => {
                failure.0 = error;
                next.set(crate::GameState::LoadError);
                return;
            }
        }
    }
}

pub(super) fn animate_scalars(
    time: Res<MaterialTime>,
    state: Res<State<crate::GameState>>,
    clock: Option<Res<NativeClock>>,
    starts: Query<(), With<NativeShaderStart>>,
    normal_views: Query<(), (With<super::GameCamera>, With<Camera3d>)>,
    inputs: Query<&NativeArtInputs>,
    entities: Query<(&ScalarAnimation, &MeshMaterial3d<StandardMaterial>)>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut status: ResMut<OpaqueSurfaceStatus>,
    mut failure: ResMut<crate::assets::LoadFailure>,
    mut next: ResMut<NextState<crate::GameState>>,
) {
    for (animation, handle) in &entities {
        let native = animation.time_mode == "native_elapsed";
        if !native && **state != crate::GameState::Playing {
            continue;
        }
        let count = animation.lookup.len();
        if count == 0 {
            continue;
        }
        let input = animation.owner.and_then(|owner| inputs.get(owner).ok());
        if native
            && input.is_none_or(|input| input.shader_time.is_none())
            && animation.owner.is_some_and(|owner| starts.contains(owner))
            && clock.as_ref().is_some_and(|clock| clock.render.is_none())
        {
            status.pending += 1;
            continue;
        }
        let (coordinate, script) = if native {
            let result = (|| -> Result<(f64, f32), String> {
                let input = input.ok_or("Native scalar material-input owner is unavailable")?;
                let (milliseconds, start) = input.shader_time.ok_or("Native scalar render-game milliseconds and shader-start input are not connected")?;
                let coordinate = game_time(milliseconds, start)?[3] * animation.speed;
                let script = if input.script_available & (1 << animation.script_vector) != 0 {
                    input.script_vectors[animation.script_vector][animation.script_component]
                } else if animation.script_vector <= 1 && !normal_views.is_empty() {
                    0.
                } else {
                    return Err("Native scalar script vector is unavailable in this view".into());
                };
                if !coordinate.is_finite() || !script.is_finite() {
                    return Err("Native scalar time or script input is non-finite".into());
                }
                Ok((coordinate as f64, script))
            })();
            match result {
                Ok(values) => values,
                Err(error) => {
                    failure.0 = error;
                    next.set(crate::GameState::LoadError);
                    return;
                }
            }
        } else {
            let seconds = time.0 - input.map_or(0., |input| input.time_offset as f64);
            (
                seconds * animation.speed as f64,
                input.map_or(0., |input| {
                    input.script_vectors[animation.script_vector][animation.script_component]
                }),
            )
        };
        let coordinate = if animation.sampler & 64 != 0 {
            coordinate.clamp(0., 1.)
        } else {
            coordinate.rem_euclid(1.)
        };
        let texel = if native {
            coordinate as f32 * count as f32 - 0.5
        } else {
            (coordinate * count as f64 - 0.5) as f32
        };
        let at = |index: i64| {
            let index = if animation.sampler & 64 != 0 {
                index.clamp(0, count as i64 - 1)
            } else {
                index.rem_euclid(count as i64)
            };
            animation.lookup[index as usize]
        };
        let value = if animation.sampler & 7 == 1 {
            at((texel + 0.5).floor() as i64)
        } else {
            let lower = texel.floor();
            let fraction = texel - lower;
            at(lower as i64) * (1. - fraction) + at(lower as i64 + 1) * fraction
        };
        let intensity = (animation.minimum + (animation.maximum - animation.minimum) * value)
            * (1. - script).clamp(0., 1.);
        let emission = intensity * intensity;
        let emissive = LinearRgba::rgb(emission, emission, emission);
        if let Some(material) = materials.get(&handle.0)
            && material.emissive != emissive
            && let Some(mut material) = materials.get_mut(&handle.0)
        {
            material.emissive = emissive;
        }
    }
}

#[derive(Component)]
struct QuestLightMaterial {
    brightness: f32,
    color: Vec3,
    handle: AssetId<StandardMaterial>,
}

pub(crate) fn install_quest_lights(app: &mut App) {
    app.add_systems(
        Update,
        quest_lights
            .after(super::SurfacePreparation)
            .after(crate::staff_upgrades::lights::LightPresentation),
    );
}
fn quest_lights(
    mut commands: Commands,
    entities: Query<(
        Entity,
        &crate::staff_upgrades::lights::LightSurface,
        &MeshMaterial3d<StandardMaterial>,
        Option<&QuestLightMaterial>,
    )>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for (entity, surface, current, cloned) in &entities {
        let Some(source) = materials.get(&current.0) else {
            continue;
        };
        let brightness = cloned
            .filter(|c| c.handle == current.0.id())
            .map_or(source.emissive.red.max(1.), |c| c.brightness);
        let color = surface.color * surface.color * brightness;
        if cloned.is_none_or(|c| c.handle != current.0.id()) {
            let mut source = source.clone();
            source.emissive = LinearRgba::rgb(color.x, color.y, color.z);
            let handle = materials.add(source);
            commands.entity(entity).insert((
                MeshMaterial3d(handle.clone()),
                QuestLightMaterial {
                    brightness,
                    color: surface.color,
                    handle: handle.id(),
                },
            ));
        } else if cloned.is_some_and(|c| c.color != surface.color) {
            if let Some(mut material) = materials.get_mut(&current.0) {
                material.emissive = LinearRgba::rgb(color.x, color.y, color.z);
            }
            commands.entity(entity).insert(QuestLightMaterial {
                brightness,
                color: surface.color,
                handle: current.0.id(),
            });
        }
    }
}

#[cfg(feature = "dev-tools")]
pub(crate) fn quest_light_snapshot(world: &mut World) -> Value {
    let materials = world.resource::<Assets<StandardMaterial>>();
    Value::Array(world.iter_entities().filter_map(|entity| {
        let light = entity.get::<QuestLightMaterial>()?;
        let material = materials.get(light.handle)?;
        Some(serde_json::json!({"entity": entity.id().to_bits(), "kind": "standard", "color": light.color.to_array(), "emissive": [material.emissive.red, material.emissive.green, material.emissive.blue]}))
    }).collect())
}

#[cfg(feature = "dev-tools")]
pub(crate) fn weapon_material_snapshot(world: &mut World) -> Value {
    let materials = world.resource::<Assets<StandardMaterial>>();
    let server = world.resource::<AssetServer>();
    let mut first_person = Vec::new();
    let mut models = Vec::new();
    let mut pending = 0;
    for entity in world.iter_entities() {
        let Some(name) = entity.get::<GltfMaterialName>() else {
            continue;
        };
        let mut current = Some(entity.id());
        let mut first = entity.contains::<super::FirstPersonSurface>();
        let mut weapon = first;
        while let Some(ancestor) = current {
            first |= world.get::<crate::presentation::Rig>(ancestor).is_some();
            weapon |= first
                || world
                    .get::<crate::presentation::camo::WeaponMaterialContext>(ancestor)
                    .is_some();
            current = world.get::<ChildOf>(ancestor).map(ChildOf::parent);
        }
        if !weapon {
            continue;
        }
        let material = entity
            .get::<MeshMaterial3d<StandardMaterial>>()
            .and_then(|h| materials.get(&h.0));
        let ready = material.is_some_and(|m| {
            [
                &m.base_color_texture,
                &m.normal_map_texture,
                &m.metallic_roughness_texture,
                &m.emissive_texture,
            ]
            .into_iter()
            .flatten()
            .all(|h| server.is_loaded_with_dependencies(h.id()))
        });
        pending += (!ready) as usize;
        let row = serde_json::json!({"entity":entity.id().to_bits(), "material":name.0, "native":entity.contains::<StandardReady>(), "textures_ready":ready, "lighting_model":"solari_standard", "visible":entity.get::<InheritedVisibility>().is_some_and(|v| v.get())});
        if first {
            first_person.push(row);
        } else {
            models.push(row);
        }
    }
    serde_json::json!({"pending":pending,"first_person":first_person,"world":models})
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn flag_parameters() -> FlagParameters {
        FlagParameters {
            parameters: [1., 0., 0., 0.],
            phase_uv: [[0., 0.]; 3],
            phase_time: [std::f32::consts::TAU, 0., 0.],
            displacement_amplitude: [7.5, 0., 0.],
            normal_cosine_u: [1., 0., 0.],
            normal_cosine_v: [2., 0., 0.],
            normal_derivative_scale: 0.025,
        }
    }

    fn flag_mesh() -> Mesh {
        let mut mesh = Mesh::new(
            bevy::mesh::PrimitiveTopology::TriangleList,
            bevy::asset::RenderAssetUsages::default(),
        );
        mesh.insert_attribute(
            Mesh::ATTRIBUTE_POSITION,
            vec![[0., 0., 0.], [1., 0., 0.], [0., 0., 1.]],
        );
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0., 1., 0.]; 3]);
        mesh.insert_attribute(Mesh::ATTRIBUTE_TANGENT, vec![[1., 0., 0., -1.]; 3]);
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, vec![[0., 0.]; 3]);
        mesh.insert_attribute(
            Mesh::ATTRIBUTE_COLOR,
            vec![
                [0.25, 0.5, 0.75, 0.],
                [0.25, 0.5, 0.75, 1.],
                [0.25, 0.5, 0.75, 1.],
            ],
        );
        mesh.insert_attribute(FLAG_SOURCE_NORMAL, vec![[0., 0.5, 0.]; 3]);
        mesh.insert_attribute(FLAG_SOURCE_TANGENT, vec![[2., 0., 0.]; 3]);
        mesh
    }

    #[test]
    fn native_shader_time_keeps_fractional_and_elapsed_components_distinct() {
        let time = game_time(1250, 0.5).unwrap();
        assert!((time[0] + 1.).abs() < 0.000001 && time[1].abs() < 0.000001);
        assert_eq!([time[2], time[3]], [0.75, 0.75]);
        let time = game_time(1250, 2.).unwrap();
        assert!((time[0] - 1.).abs() < 0.000001 && time[1].abs() < 0.000001);
        assert_eq!([time[2], time[3]], [0.25, -0.75]);
        assert!(game_time(1250, f32::NAN).is_err());
    }

    #[test]
    fn native_clock_publishes_actual_snapshots_and_preserves_restart_but_resets_shutdown() {
        use std::time::Duration;
        let mut app = App::new();
        app.init_resource::<NativeClock>();
        app.world_mut()
            .run_system_cached(begin_native_time)
            .unwrap();
        {
            let mut clock = app.world_mut().resource_mut::<NativeClock>();
            clock.advance(Duration::from_millis(49), false, 1.).unwrap();
            assert_eq!((clock.server, clock.render), (0, None));
            clock.advance(Duration::from_millis(76), false, 1.).unwrap();
            assert_eq!((clock.server, clock.render), (100, Some(100)));
            clock.advance(Duration::from_secs(3), true, 1.).unwrap();
            assert_eq!((clock.server, clock.render), (100, Some(100)));
            clock.advance(Duration::ZERO, false, 1.).unwrap();
            assert_eq!(
                (clock.server, clock.render, clock.interpolation),
                (100, Some(100), 40)
            );
            clock.advance(Duration::from_millis(25), false, 1.).unwrap();
            assert_eq!((clock.server, clock.render), (150, Some(125)));
        }
        let owner = app
            .world_mut()
            .spawn(NativeArtInputs {
                shader_time: Some((125, 0.5)),
                ..default()
            })
            .id();
        app.world_mut()
            .run_system_cached(begin_native_time)
            .unwrap();
        assert_eq!(app.world().resource::<NativeClock>().server, 150);
        app.world_mut()
            .run_system_cached(shutdown_native_time)
            .unwrap();
        assert_eq!(app.world().resource::<NativeClock>().server, 0);
        assert_eq!(app.world().resource::<NativeClock>().render, None);
        assert_eq!(
            app.world()
                .get::<NativeArtInputs>(owner)
                .unwrap()
                .shader_time,
            None
        );
    }

    #[test]
    fn native_clock_delivery_waits_for_snapshot_and_preserves_explicit_object_start() {
        use std::time::Duration;
        let mut app = App::new();
        app.init_resource::<NativeClock>()
            .init_resource::<Time>()
            .init_resource::<Time<Virtual>>()
            .init_resource::<crate::assets::LoadFailure>()
            .insert_resource(NextState::<crate::GameState>::Unchanged)
            .insert_resource(State::new(crate::GameState::PreparingShaders));
        let owner = app
            .world_mut()
            .spawn(GltfExtras {
                value: json!({"static_instance":true,"source_model":"native_flag"}).to_string(),
            })
            .id();
        app.world_mut()
            .run_system_cached(prepare_native_time_owners)
            .unwrap();
        assert_eq!(app.world().get::<NativeShaderStart>(owner).unwrap().0, 0.);
        app.world_mut()
            .run_system_cached(begin_native_time)
            .unwrap();
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(Duration::from_millis(49));
        app.world_mut()
            .run_system_cached(advance_native_time)
            .unwrap();
        assert_eq!(
            app.world()
                .get::<NativeArtInputs>(owner)
                .unwrap()
                .shader_time,
            None
        );
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(Duration::from_millis(1));
        app.world_mut()
            .run_system_cached(advance_native_time)
            .unwrap();
        let input = app.world().get::<NativeArtInputs>(owner).unwrap();
        assert_eq!(input.shader_time, Some((50, 0.)));
        assert_eq!(input.script_available, 0);
        app.world_mut()
            .get_mut::<NativeShaderStart>(owner)
            .unwrap()
            .0 = 0.75;
        app.world_mut()
            .insert_resource(State::new(crate::GameState::Paused));
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(Duration::from_secs(1));
        app.world_mut()
            .run_system_cached(advance_native_time)
            .unwrap();
        assert_eq!(
            app.world()
                .get::<NativeArtInputs>(owner)
                .unwrap()
                .shader_time,
            Some((50, 0.75))
        );
    }

    #[test]
    fn native_scalar_uses_full_f32_time_repeat_filter_and_scoped_base_script_values() {
        use bevy::asset::AssetApp;
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, bevy::asset::AssetPlugin::default()))
            .init_asset::<StandardMaterial>()
            .init_resource::<crate::assets::LoadFailure>()
            .init_resource::<OpaqueSurfaceStatus>()
            .insert_resource(NextState::<crate::GameState>::Unchanged)
            .insert_resource(State::new(crate::GameState::Paused))
            .insert_resource(MaterialTime(999.))
            .add_systems(Update, (refresh_owners, animate_scalars).chain());
        let original = app
            .world_mut()
            .resource_mut::<Assets<StandardMaterial>>()
            .add(StandardMaterial::default());
        let mut objects = Vec::new();
        for start in [0., 0.5] {
            let owner = app
                .world_mut()
                .spawn(NativeArtInputs {
                    shader_time: Some((1500, start)),
                    script_vectors: [[0.; 4], [0.25, 0., 0., 0.], [0.; 4], [0.; 4]],
                    script_available: 1 << 1,
                    ..default()
                })
                .id();
            let material = app
                .world_mut()
                .resource_mut::<Assets<StandardMaterial>>()
                .add(StandardMaterial::default());
            let animation: ScalarAnimation = serde_json::from_value(json!({
                "time_mode":"native_elapsed","lookup":[0.,0.2,0.4,1.],"sampler":20,
                "speed":0.5,"minimum":2.,"maximum":4.,"script_vector":1,"script_component":0
            }))
            .unwrap();
            let surface = app
                .world_mut()
                .spawn((ChildOf(owner), MeshMaterial3d(material.clone()), animation))
                .id();
            objects.push((owner, surface, material));
        }
        app.update();
        let materials = app.world().resource::<Assets<StandardMaterial>>();
        assert!((materials.get(&objects[0].2).unwrap().emissive.red - 6.5025).abs() < 0.00001);
        assert!((materials.get(&objects[1].2).unwrap().emissive.red - 3.8025).abs() < 0.00001);
        assert_eq!(materials.get(&original).unwrap().emissive.red, 0.);
        app.world_mut()
            .get_mut::<NativeArtInputs>(objects[0].0)
            .unwrap()
            .script_available = 0;
        app.update();
        assert!(
            app.world()
                .resource::<crate::assets::LoadFailure>()
                .0
                .contains("script vector is unavailable")
        );
        app.world_mut()
            .resource_mut::<crate::assets::LoadFailure>()
            .0
            .clear();
        app.world_mut()
            .spawn((super::super::GameCamera, Camera3d::default()));
        app.update();
        assert!(
            app.world()
                .resource::<crate::assets::LoadFailure>()
                .0
                .is_empty()
        );
        assert!(
            (app.world()
                .resource::<Assets<StandardMaterial>>()
                .get(&objects[0].2)
                .unwrap()
                .emissive
                .red
                - 11.56)
                .abs()
                < 0.00001
        );
        app.world_mut()
            .get_mut::<ScalarAnimation>(objects[0].1)
            .unwrap()
            .script_vector = 2;
        app.update();
        assert!(
            app.world()
                .resource::<crate::assets::LoadFailure>()
                .0
                .contains("script vector is unavailable")
        );
    }

    #[test]
    fn native_flag_basis_keeps_signed_cpu_input_and_axis_specific_repacking() {
        assert_eq!(flag_skin_basis(0), Vec3::ZERO);
        assert_eq!(flag_skin_basis(1023), Vec3::new(-1. / 511., 0., 0.));
        let expected = Vec3::new(
            2. * (897_f32 / 1023.) - 2.,
            2. * (357_f32 / 1023.),
            -(2. * (972_f32 / 1023.) - 2.),
        );
        assert_eq!(flag_repack_basis(Vec3::new(-0.25, 0.7, 0.1)), expected);
        assert_eq!(
            flag_repack_basis(Vec3::new(2., 0., 0.)),
            Vec3::new(1022. / 1023., 0., -0.)
        );
        assert!(flag_placement(Mat4::from_scale(Vec3::splat(2.)), true).is_err());
        assert!(flag_placement(Mat4::from_rotation_z(0.37), true).is_ok());
    }

    #[test]
    fn skinned_flag_updates_paused_pose_and_private_ray_proxy_then_restores_source_skin() {
        use bevy::{asset::AssetApp, solari::prelude::RaytracingMesh3d};
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, bevy::asset::AssetPlugin::default()))
            .init_asset::<Mesh>()
            .init_asset::<StandardMaterial>()
            .init_asset::<SkinnedMeshInverseBindposes>()
            .init_resource::<crate::assets::LoadFailure>()
            .init_resource::<OpaqueSurfaceStatus>()
            .insert_resource(NextState::<crate::GameState>::Unchanged)
            .insert_resource(State::new(crate::GameState::Paused))
            .add_systems(
                Update,
                (animate_flags, super::super::solari::prepare_meshes).chain(),
            );
        let owner = app
            .world_mut()
            .spawn(NativeArtInputs {
                shader_time: Some((1250, 0.)),
                ..default()
            })
            .id();
        let placement =
            Mat4::from_rotation_translation(Quat::from_rotation_z(0.37), Vec3::new(3., 2., -4.));
        let first = app.world_mut().spawn(GlobalTransform::from(placement)).id();
        let second = app
            .world_mut()
            .spawn(GlobalTransform::from(
                placement * Mat4::from_translation(Vec3::Y * 0.2),
            ))
            .id();
        let bind = app
            .world_mut()
            .resource_mut::<Assets<SkinnedMeshInverseBindposes>>()
            .add(SkinnedMeshInverseBindposes::from(vec![Mat4::IDENTITY; 2]));
        let skin = SkinnedMesh {
            inverse_bindposes: bind,
            joints: vec![first, second],
        };
        let mut source = flag_mesh();
        source.insert_attribute(FLAG_SOURCE_NORMAL, vec![[0., 512. / 1023., 0.]; 3]);
        source.insert_attribute(FLAG_SOURCE_TANGENT, vec![[1022. / 1023., 0., 0.]; 3]);
        source.insert_attribute(FLAG_PACKED_BASIS, vec![[256_u32 << 20, 511]; 3]);
        source.insert_attribute(
            Mesh::ATTRIBUTE_JOINT_INDEX,
            VertexAttributeValues::Uint16x4(vec![[0_u16, 1, 0, 0]; 3]),
        );
        source.insert_attribute(Mesh::ATTRIBUTE_JOINT_WEIGHT, vec![[0.25, 0.75, 0., 0.]; 3]);
        let original = app
            .world_mut()
            .resource_mut::<Assets<Mesh>>()
            .add(source.clone());
        let (mut flag, target) =
            FlagMotion::new(original.clone(), owner, flag_parameters(), &source).unwrap();
        let mut missing = source.clone();
        missing.remove_attribute(FLAG_PACKED_BASIS);
        assert!(
            flag.set_skin(&missing, skin.clone())
                .unwrap_err()
                .contains("original packed basis")
        );
        flag.set_skin(&source, skin.clone()).unwrap();
        let target = app.world_mut().resource_mut::<Assets<Mesh>>().add(target);
        flag.target = target.clone();
        let material = app
            .world_mut()
            .resource_mut::<Assets<StandardMaterial>>()
            .add(StandardMaterial::default());
        let entity = app
            .world_mut()
            .spawn((
                Mesh3d(target.clone()),
                MeshMaterial3d(material),
                GlobalTransform::from(placement),
                InheritedVisibility::VISIBLE,
                flag.bounds(),
                flag,
            ))
            .id();
        app.update();
        assert!(
            app.world()
                .resource::<crate::assets::LoadFailure>()
                .0
                .is_empty()
        );
        assert!(app.world().get::<SkinnedMesh>(entity).is_none());
        let proxy = app
            .world()
            .get::<RaytracingMesh3d>(entity)
            .unwrap()
            .0
            .clone();
        let Some(VertexAttributeValues::Float32x3(before)) = app
            .world()
            .resource::<Assets<Mesh>>()
            .get(&target)
            .unwrap()
            .attribute(Mesh::ATTRIBUTE_POSITION)
        else {
            panic!()
        };
        assert!((before[0][1] - 0.15).abs() < 0.00001);
        let before = before.clone();
        app.world_mut()
            .entity_mut(second)
            .insert(GlobalTransform::from(
                placement
                    * Mat4::from_rotation_translation(Quat::from_rotation_x(0.8), Vec3::Y * 0.6),
            ));
        app.update();
        assert!(
            app.world()
                .resource::<crate::assets::LoadFailure>()
                .0
                .is_empty()
        );
        assert_eq!(
            app.world()
                .get::<NativeArtInputs>(owner)
                .unwrap()
                .shader_time,
            Some((1250, 0.))
        );
        assert_eq!(
            app.world().get::<RaytracingMesh3d>(entity).unwrap().0,
            proxy
        );
        let meshes = app.world().resource::<Assets<Mesh>>();
        let raster = meshes.get(&target).unwrap();
        for attribute in [Mesh::ATTRIBUTE_JOINT_INDEX, Mesh::ATTRIBUTE_JOINT_WEIGHT] {
            assert!(raster.attribute(attribute).is_none());
            assert!(
                meshes
                    .get(&original)
                    .unwrap()
                    .attribute(attribute)
                    .is_some()
            );
        }
        let Some(VertexAttributeValues::Float32x3(after)) =
            raster.attribute(Mesh::ATTRIBUTE_POSITION)
        else {
            panic!()
        };
        assert!((after[0][1] - 0.45).abs() < 0.00001);
        assert_ne!(*after, before);
        for attribute in [
            Mesh::ATTRIBUTE_POSITION,
            Mesh::ATTRIBUTE_NORMAL,
            Mesh::ATTRIBUTE_TANGENT,
        ] {
            assert_eq!(
                raster.attribute(attribute),
                meshes.get(&proxy).unwrap().attribute(attribute)
            );
        }
        assert_eq!(
            meshes
                .get(&original)
                .unwrap()
                .attribute(Mesh::ATTRIBUTE_POSITION),
            source.attribute(Mesh::ATTRIBUTE_POSITION)
        );
        app.world_mut().run_system_cached(initialize).unwrap();
        assert!(app.world().get::<FlagMotion>(entity).is_none());
        assert_eq!(app.world().get::<Mesh3d>(entity).unwrap().0, original);
        let restored = app.world().get::<SkinnedMesh>(entity).unwrap();
        assert_eq!(restored.joints, skin.joints);
        assert_eq!(restored.inverse_bindposes, skin.inverse_bindposes);
        assert!(
            app.world()
                .get::<NativeArtInputs>(owner)
                .unwrap()
                .shader_time
                .is_none()
        );
        let (mut motion, _) =
            FlagMotion::new(original.clone(), owner, flag_parameters(), &source).unwrap();
        motion.set_skin(&source, skin.clone()).unwrap();
        motion.target = target.clone();
        let replacement = SkinnedMesh {
            inverse_bindposes: skin.inverse_bindposes.clone(),
            joints: vec![second, first],
        };
        app.world_mut()
            .entity_mut(entity)
            .insert((Mesh3d(target), motion, replacement.clone()));
        app.world_mut().run_system_cached(animate_flags).unwrap();
        assert!(app.world().get::<FlagMotion>(entity).is_none());
        assert_eq!(app.world().get::<Mesh3d>(entity).unwrap().0, original);
        assert_eq!(
            app.world().get::<SkinnedMesh>(entity).unwrap().joints,
            replacement.joints
        );
    }

    #[test]
    fn rigid_flag_motion_preserves_raw_basis_pins_and_updates_ray_geometry_before_reset() {
        use bevy::{asset::AssetApp, solari::prelude::RaytracingMesh3d};
        let source = flag_mesh();
        let (flag, _) = FlagMotion::new(
            Handle::default(),
            Entity::PLACEHOLDER,
            flag_parameters(),
            &source,
        )
        .unwrap();
        let (_, normals) = flag.deform(0.).unwrap();
        assert_eq!(normals[0], [0., 1., 0.]);
        let expected = Vec3::new(0.05, 0.5, 0.05).normalize();
        assert!((Vec3::from_array(normals[1]) - expected).length() < 0.000001);
        let (positions, _) = flag.deform(0.25).unwrap();
        assert_eq!(positions[0], [0., 0., 0.]);
        assert!((positions[1][1] - 7.5 * 0.5 * 0.0254).abs() < 0.000001);
        let mut missing = flag_mesh();
        missing.remove_attribute(FLAG_SOURCE_NORMAL);
        assert!(
            FlagMotion::new(
                Handle::default(),
                Entity::PLACEHOLDER,
                flag_parameters(),
                &missing
            )
            .is_err()
        );
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, bevy::asset::AssetPlugin::default()))
            .init_asset::<Mesh>()
            .init_asset::<StandardMaterial>()
            .init_asset::<bevy::mesh::skinning::SkinnedMeshInverseBindposes>()
            .init_resource::<crate::assets::LoadFailure>()
            .init_resource::<OpaqueSurfaceStatus>()
            .insert_resource(NextState::<crate::GameState>::Unchanged)
            .insert_resource(State::new(crate::GameState::Paused))
            .add_systems(
                Update,
                (
                    refresh_owners,
                    animate_flags.run_if(material_scene_active),
                    super::super::solari::prepare_meshes,
                )
                    .chain(),
            );
        let original = app.world_mut().resource_mut::<Assets<Mesh>>().add(source);
        let material = app
            .world_mut()
            .resource_mut::<Assets<StandardMaterial>>()
            .add(StandardMaterial::default());
        let mut objects = Vec::new();
        for start in [0., 0.5] {
            let owner = app
                .world_mut()
                .spawn(NativeArtInputs {
                    shader_time: Some((1250, start)),
                    ..default()
                })
                .id();
            let source = app
                .world()
                .resource::<Assets<Mesh>>()
                .get(&original)
                .unwrap();
            let (mut flag, target) =
                FlagMotion::new(original.clone(), owner, flag_parameters(), source).unwrap();
            let target = app.world_mut().resource_mut::<Assets<Mesh>>().add(target);
            flag.target = target.clone();
            let bounds = flag.bounds();
            let entity = app
                .world_mut()
                .spawn((
                    ChildOf(owner),
                    Mesh3d(target.clone()),
                    MeshMaterial3d(material.clone()),
                    GlobalTransform::IDENTITY,
                    InheritedVisibility::VISIBLE,
                    bounds,
                    flag,
                ))
                .id();
            objects.push((owner, entity, target));
        }
        app.update();
        assert!(
            app.world()
                .resource::<crate::assets::LoadFailure>()
                .0
                .is_empty()
        );
        let mut proxy = app
            .world()
            .get::<RaytracingMesh3d>(objects[0].1)
            .unwrap()
            .0
            .clone();
        {
            let meshes = app.world().resource::<Assets<Mesh>>();
            let raster = meshes.get(&objects[0].2).unwrap();
            assert_eq!(
                raster.attribute(Mesh::ATTRIBUTE_POSITION),
                meshes
                    .get(&proxy)
                    .unwrap()
                    .attribute(Mesh::ATTRIBUTE_POSITION)
            );
            let Some(VertexAttributeValues::Float32x3(a)) =
                raster.attribute(Mesh::ATTRIBUTE_POSITION)
            else {
                panic!()
            };
            let Some(VertexAttributeValues::Float32x3(b)) = meshes
                .get(&objects[1].2)
                .unwrap()
                .attribute(Mesh::ATTRIBUTE_POSITION)
            else {
                panic!()
            };
            assert!(a[1][1] > 0.09 && b[1][1] < -0.09);
            let Some(VertexAttributeValues::Float32x4(colors)) =
                raster.attribute(Mesh::ATTRIBUTE_COLOR)
            else {
                panic!()
            };
            assert!(colors.iter().all(|color| *color == [0.25, 0.5, 0.75, 1.]));
            let Some(VertexAttributeValues::Float32x3(source)) = meshes
                .get(&original)
                .unwrap()
                .attribute(Mesh::ATTRIBUTE_POSITION)
            else {
                panic!()
            };
            assert!(source.iter().all(|position| position[1] == 0.));
        }
        let mut events = app
            .world()
            .resource::<Messages<AssetEvent<Mesh>>>()
            .get_cursor_current();
        app.update();
        assert_eq!(
            app.world().get::<RaytracingMesh3d>(objects[0].1).unwrap().0,
            proxy
        );
        assert!(
            !events
                .read(app.world().resource::<Messages<AssetEvent<Mesh>>>())
                .any(|event| matches!(event, AssetEvent::Added { id } | AssetEvent::Modified { id } if *id == proxy.id()))
        );
        app.world_mut()
            .get_mut::<NativeArtInputs>(objects[0].0)
            .unwrap()
            .shader_time = Some((1750, 0.));
        app.update();
        let previous_proxy = proxy;
        proxy = app
            .world()
            .get::<RaytracingMesh3d>(objects[0].1)
            .unwrap()
            .0
            .clone();
        assert_ne!(proxy, previous_proxy);
        let meshes = app.world().resource::<Assets<Mesh>>();
        let Some(VertexAttributeValues::Float32x3(positions)) = meshes
            .get(&proxy)
            .unwrap()
            .attribute(Mesh::ATTRIBUTE_POSITION)
        else {
            panic!()
        };
        assert!(positions[1][1] < -0.09);
        let changes: Vec<_> = events
            .read(app.world().resource::<Messages<AssetEvent<Mesh>>>())
            .copied()
            .collect();
        assert_eq!(
            changes
                .iter()
                .filter(|event| matches!(event, AssetEvent::Added { id } if *id == proxy.id()))
                .count(),
            1
        );
        assert!(!changes.iter().any(|event| matches!(event,
            AssetEvent::Modified { id } if *id == previous_proxy.id() || *id == proxy.id())));
        app.update();
        assert_eq!(
            app.world().get::<RaytracingMesh3d>(objects[0].1).unwrap().0,
            proxy
        );
        assert!(
            !events
                .read(app.world().resource::<Messages<AssetEvent<Mesh>>>())
                .any(|event| matches!(event, AssetEvent::Added { id } | AssetEvent::Modified { id } if *id == proxy.id()))
        );
        app.world_mut()
            .entity_mut(objects[0].1)
            .insert(InheritedVisibility::HIDDEN);
        app.world_mut()
            .get_mut::<NativeArtInputs>(objects[0].0)
            .unwrap()
            .shader_time = Some((2250, 0.));
        app.update();
        assert!(app.world().get::<RaytracingMesh3d>(objects[0].1).is_none());
        assert!(
            !events
                .read(app.world().resource::<Messages<AssetEvent<Mesh>>>())
                .any(|event| matches!(event, AssetEvent::Added { id } | AssetEvent::Modified { id } if *id == proxy.id()))
        );
        app.world_mut()
            .entity_mut(objects[0].1)
            .insert(InheritedVisibility::VISIBLE);
        app.update();
        let previous_proxy = proxy;
        proxy = app
            .world()
            .get::<RaytracingMesh3d>(objects[0].1)
            .unwrap()
            .0
            .clone();
        assert_ne!(proxy, previous_proxy);
        let changes: Vec<_> = events
            .read(app.world().resource::<Messages<AssetEvent<Mesh>>>())
            .copied()
            .collect();
        assert_eq!(
            changes
                .iter()
                .filter(|event| matches!(event, AssetEvent::Added { id } if *id == proxy.id()))
                .count(),
            1
        );
        assert!(!changes.iter().any(|event| matches!(event,
            AssetEvent::Modified { id } if *id == previous_proxy.id() || *id == proxy.id())));
        let meshes = app.world().resource::<Assets<Mesh>>();
        assert_eq!(
            meshes
                .get(&objects[0].2)
                .unwrap()
                .attribute(Mesh::ATTRIBUTE_POSITION),
            meshes
                .get(&proxy)
                .unwrap()
                .attribute(Mesh::ATTRIBUTE_POSITION)
        );
        app.world_mut()
            .entity_mut(objects[0].1)
            .insert(ChildOf(objects[1].0));
        app.world_mut()
            .get_mut::<NativeArtInputs>(objects[1].0)
            .unwrap()
            .shader_time = Some((1000, 0.));
        app.update();
        assert_eq!(
            app.world().get::<FlagMotion>(objects[0].1).unwrap().owner,
            Some(objects[1].0)
        );
        let previous_proxy = proxy;
        proxy = app
            .world()
            .get::<RaytracingMesh3d>(objects[0].1)
            .unwrap()
            .0
            .clone();
        assert_ne!(proxy, previous_proxy);
        let changes: Vec<_> = events
            .read(app.world().resource::<Messages<AssetEvent<Mesh>>>())
            .copied()
            .collect();
        assert_eq!(
            changes
                .iter()
                .filter(|event| matches!(event, AssetEvent::Added { id } if *id == proxy.id()))
                .count(),
            1
        );
        assert!(!changes.iter().any(|event| matches!(event,
            AssetEvent::Modified { id } if *id == previous_proxy.id() || *id == proxy.id())));
        let Some(VertexAttributeValues::Float32x3(positions)) = app
            .world()
            .resource::<Assets<Mesh>>()
            .get(&proxy)
            .unwrap()
            .attribute(Mesh::ATTRIBUTE_POSITION)
        else {
            panic!()
        };
        assert!(positions[1][1].abs() < 0.000001);
        app.world_mut()
            .entity_mut(objects[1].0)
            .remove::<NativeArtInputs>();
        app.update();
        assert_eq!(
            app.world().get::<FlagMotion>(objects[0].1).unwrap().owner,
            None
        );
        assert!(
            app.world()
                .resource::<crate::assets::LoadFailure>()
                .0
                .contains("shader-start input")
        );
        app.world_mut()
            .entity_mut(objects[1].0)
            .insert(NativeArtInputs {
                shader_time: Some((1250, 0.)),
                ..default()
            });
        app.world_mut()
            .resource_mut::<crate::assets::LoadFailure>()
            .0
            .clear();
        app.update();
        app.world_mut()
            .resource_mut::<Assets<Mesh>>()
            .remove(objects[0].2.id());
        app.update();
        assert!(
            app.world()
                .resource::<crate::assets::LoadFailure>()
                .0
                .contains("CPU mesh is unavailable")
        );
        app.add_systems(Update, initialize.before(refresh_owners));
        app.update();
        for (owner, entity, _) in objects {
            assert!(app.world().get::<FlagMotion>(entity).is_none());
            assert_eq!(app.world().get::<Mesh3d>(entity).unwrap().0, original);
            assert_eq!(
                app.world()
                    .get::<NativeArtInputs>(owner)
                    .unwrap()
                    .shader_time,
                None
            );
        }
    }

    #[test]
    fn repeated_surface_registration_retains_loaded_frames_and_rejects_conflicts() {
        let mut library = SurfaceLibrary::default();
        let surface = json!({"identity":"test", "presentation":{}, "channels":[]});
        register_prepared(&mut library, &surface, &[]).unwrap();
        library
            .definitions
            .get_mut("test")
            .unwrap()
            .frames
            .push(Frame {
                channels: HashMap::new(),
                factors: json!({}),
            });
        register_prepared(&mut library, &surface, &[]).unwrap();
        assert_eq!(library.definitions["test"].frames.len(), 1);
        let mut conflict = surface;
        conflict["standard"] = json!({"emissive":[1.,0.,0.]});
        assert!(register_prepared(&mut library, &conflict, &[]).is_err());
    }

    #[test]
    fn unsupported_animation_controls_are_rejected_before_playback() {
        for animation in [
            json!({"driver":"unknown", "fps":1., "frames":[{"channels":[]}]}),
            json!({"driver":"time", "fps":0., "frames":[{"channels":[]}]}),
            json!({"driver":"staff_charge", "fps":1., "time_frames":2,
                "state_threshold":1., "frames":[{"channels":[]}]}),
        ] {
            assert!(validate_animation(&json!({"animation":animation})).is_err());
        }
        assert!(
            validate_animation(&json!({"scalar_animation":{"lookup":[],"sampler":0,
            "speed":1.,"minimum":0.,"maximum":1.,"script_vector":0,"script_component":0}}))
            .is_err()
        );
    }

    #[test]
    fn source_uv_scrolling_is_composed_before_the_prepared_bake_transform() {
        use bevy::asset::AssetApp;
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, bevy::asset::AssetPlugin::default()))
            .init_asset::<StandardMaterial>()
            .init_resource::<crate::assets::LoadFailure>()
            .init_resource::<OpaqueSurfaceStatus>()
            .insert_resource(NextState::<crate::GameState>::Unchanged)
            .insert_resource(MaterialTime(2.))
            .insert_resource(State::new(crate::GameState::Playing))
            .add_systems(Update, animate_uvs);
        let base =
            super::super::pbr::uv_transform(&json!({"uv_transform":[0.5,0.,0.,0.25,-1.,2.]}))
                .unwrap();
        let handle = app
            .world_mut()
            .resource_mut::<Assets<StandardMaterial>>()
            .add(StandardMaterial::default());
        app.world_mut().spawn((
            MeshMaterial3d(handle.clone()),
            UvAnimation {
                translation: [1., -2.],
                time_mode: "elapsed".into(),
                owner: None,
                base,
            },
        ));
        app.update();
        let material = app
            .world()
            .resource::<Assets<StandardMaterial>>()
            .get(&handle)
            .unwrap();
        assert_eq!(
            material.uv_transform.transform_point2(Vec2::new(2., -8.)),
            Vec2::new(1., -1.)
        );
        assert_eq!(
            material.uv_transform.transform_point2(Vec2::new(4., -4.)),
            Vec2::new(2., 0.)
        );
        let owner = app
            .world_mut()
            .spawn(NativeArtInputs {
                shader_time: Some((1250, 0.5)),
                ..default()
            })
            .id();
        let native = app
            .world_mut()
            .resource_mut::<Assets<StandardMaterial>>()
            .add(StandardMaterial::default());
        app.world_mut().spawn((
            MeshMaterial3d(native.clone()),
            UvAnimation {
                translation: [0., -1.],
                time_mode: "native_fractional_second".into(),
                owner: Some(owner),
                base: bevy::math::Affine2::IDENTITY,
            },
        ));
        app.update();
        assert_eq!(
            app.world()
                .resource::<Assets<StandardMaterial>>()
                .get(&native)
                .unwrap()
                .uv_transform
                .translation,
            Vec2::new(0., -0.75)
        );
        app.world_mut().resource_mut::<MaterialTime>().0 = 999.;
        app.update();
        assert_eq!(
            app.world()
                .resource::<Assets<StandardMaterial>>()
                .get(&native)
                .unwrap()
                .uv_transform
                .translation,
            Vec2::new(0., -0.75)
        );
        let legacy = app
            .world()
            .resource::<Assets<StandardMaterial>>()
            .get(&handle)
            .unwrap()
            .uv_transform;
        app.world_mut()
            .insert_resource(State::new(crate::GameState::Paused));
        app.world_mut().resource_mut::<MaterialTime>().0 = 1000.;
        app.world_mut()
            .get_mut::<NativeArtInputs>(owner)
            .unwrap()
            .shader_time = Some((1800, 0.5));
        app.update();
        let materials = app.world().resource::<Assets<StandardMaterial>>();
        assert_eq!(materials.get(&handle).unwrap().uv_transform, legacy);
        assert!(
            (materials.get(&native).unwrap().uv_transform.translation.y + 0.3).abs() < 0.000001
        );
        app.world_mut()
            .get_mut::<NativeArtInputs>(owner)
            .unwrap()
            .shader_time = None;
        app.update();
        assert!(
            app.world()
                .resource::<crate::assets::LoadFailure>()
                .0
                .contains("shader-start input")
        );
    }

    #[test]
    fn native_uv_clock_tracks_nearest_live_owner_and_clears_across_loading() {
        use bevy::asset::AssetApp;
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, bevy::asset::AssetPlugin::default()))
            .init_asset::<StandardMaterial>()
            .init_resource::<crate::assets::LoadFailure>()
            .init_resource::<OpaqueSurfaceStatus>()
            .insert_resource(NextState::<crate::GameState>::Unchanged)
            .insert_resource(State::new(crate::GameState::Paused))
            .insert_resource(MaterialTime(987.))
            .add_systems(
                Update,
                (refresh_owners, animate_uvs)
                    .chain()
                    .run_if(material_scene_active),
            );
        let root = app
            .world_mut()
            .spawn(NativeArtInputs {
                shader_time: Some((1100, 0.5)),
                ..default()
            })
            .id();
        let nearest = app
            .world_mut()
            .spawn((
                ChildOf(root),
                NativeArtInputs {
                    shader_time: Some((1750, 0.)),
                    ..default()
                },
            ))
            .id();
        let material = app
            .world_mut()
            .resource_mut::<Assets<StandardMaterial>>()
            .add(StandardMaterial::default());
        let surface = app
            .world_mut()
            .spawn((
                ChildOf(nearest),
                MeshMaterial3d(material.clone()),
                UvAnimation {
                    translation: [0., -1.],
                    time_mode: "native_fractional_second".into(),
                    owner: None,
                    base: bevy::math::Affine2::IDENTITY,
                },
            ))
            .id();
        app.update();
        assert_eq!(
            app.world().get::<UvAnimation>(surface).unwrap().owner,
            Some(nearest)
        );
        assert!(
            (app.world()
                .resource::<Assets<StandardMaterial>>()
                .get(&material)
                .unwrap()
                .uv_transform
                .translation
                .y
                + 0.75)
                .abs()
                <= f32::EPSILON
        );
        app.world_mut()
            .entity_mut(nearest)
            .remove::<NativeArtInputs>();
        app.update();
        assert_eq!(
            app.world().get::<UvAnimation>(surface).unwrap().owner,
            Some(root)
        );
        assert!(
            (app.world()
                .resource::<Assets<StandardMaterial>>()
                .get(&material)
                .unwrap()
                .uv_transform
                .translation
                .y
                + 0.6)
                .abs()
                < 0.000001
        );
        app.world_mut().entity_mut(root).remove::<NativeArtInputs>();
        app.update();
        assert_eq!(app.world().get::<UvAnimation>(surface).unwrap().owner, None);
        assert!(
            app.world()
                .resource::<crate::assets::LoadFailure>()
                .0
                .contains("shader-start input")
        );
        app.world_mut().entity_mut(root).insert(NativeArtInputs {
            shader_time: Some((50, 0.)),
            ..default()
        });
        app.world_mut()
            .resource_mut::<crate::assets::LoadFailure>()
            .0
            .clear();
        app.world_mut()
            .insert_resource(State::new(crate::GameState::Loading));
        app.add_systems(Update, initialize);
        app.update();
        assert_eq!(
            app.world()
                .get::<NativeArtInputs>(root)
                .unwrap()
                .shader_time,
            None
        );
        assert!(
            app.world()
                .resource::<crate::assets::LoadFailure>()
                .0
                .is_empty()
        );
    }

    #[test]
    fn frame_selection_respects_per_object_offsets_without_mutating_shared_materials() {
        use bevy::asset::AssetApp;
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, bevy::asset::AssetPlugin::default()))
            .init_asset::<Image>()
            .init_asset::<StandardMaterial>()
            .insert_resource(MaterialTime(1.))
            .add_systems(Update, (refresh_owners, animate).chain());
        let original = app
            .world_mut()
            .resource_mut::<Assets<StandardMaterial>>()
            .add(StandardMaterial::default());
        let mut objects = Vec::new();
        for offset in [0., 1.] {
            let owner = app
                .world_mut()
                .spawn(NativeArtInputs {
                    time_offset: offset,
                    ..default()
                })
                .id();
            let material = app
                .world_mut()
                .resource_mut::<Assets<StandardMaterial>>()
                .add(StandardMaterial::default());
            let entity = app
                .world_mut()
                .spawn((
                    ChildOf(owner),
                    MeshMaterial3d(material.clone()),
                    AnimatedMaterial {
                        driver: "time".into(),
                        fps: 1.,
                        time_frames: 2,
                        state_threshold: 1.,
                        owner: Some(owner),
                        frames: vec![
                            Frame {
                                channels: HashMap::new(),
                                factors: json!({"emissive":[1.,0.,0.]}),
                            },
                            Frame {
                                channels: HashMap::new(),
                                factors: json!({"emissive":[9.,0.,0.]}),
                            },
                        ],
                        selected: usize::MAX,
                    },
                ))
                .id();
            objects.push((entity, material, owner));
        }
        app.update();
        {
            let materials = app.world().resource::<Assets<StandardMaterial>>();
            assert_eq!(materials.get(&objects[0].1).unwrap().emissive.red, 9.);
            assert_eq!(materials.get(&objects[1].1).unwrap().emissive.red, 1.);
            assert_eq!(materials.get(&original).unwrap().emissive.red, 0.);
        }
        app.world_mut()
            .entity_mut(objects[0].0)
            .insert(ChildOf(objects[1].2));
        app.update();
        let materials = app.world().resource::<Assets<StandardMaterial>>();
        assert_eq!(materials.get(&objects[0].1).unwrap().emissive.red, 1.);
    }
}

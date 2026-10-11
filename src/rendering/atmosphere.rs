use super::{GameCamera, profile::VisualProfile};
use crate::environment::weather::{Weather, WeatherKind};
use bevy::{
    asset::uuid_handle,
    prelude::*,
    shader::{Shader, ShaderImport},
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

const FOG_SHADER: Handle<Shader> = uuid_handle!("87aacd2f-2138-4cd8-b216-f392144468fd");

#[derive(Resource, Deserialize)]
pub(crate) struct SourceAtmosphere {
    pub(super) sun_angles: [f32; 3],
    pub(super) sun_color: [f32; 3],
    pub(super) snow_sun_color: [f32; 3],
    pub(super) sun_brightness: f32,
    pub(super) fog: WorldFog,
    fog_transition_seconds: f32,
    volumes: Vec<SourceVolume>,
    pub(super) softclip: f32,
    pub(super) vision: BTreeMap<String, BTreeMap<String, [f32; 4]>>,
}

impl Default for SourceAtmosphere {
    fn default() -> Self {
        serde_json::from_str(include_str!("../../config/origins-atmosphere.json"))
            .expect("Prepared original Origins atmosphere")
    }
}

#[derive(Clone, Copy, Deserialize, Serialize)]
pub(super) struct WorldFog {
    distance: [f32; 4],
    sun_angles: [f32; 4],
    color: [f32; 3],
    opacity: f32,
    sun_color: [f32; 3],
    sun_opacity: f32,
}

impl WorldFog {
    fn blend(self, target: Self, fraction: f32) -> Self {
        let lerp = |a: f32, b: f32| a + (b - a) * fraction;
        Self {
            distance: std::array::from_fn(|i| lerp(self.distance[i], target.distance[i])),
            sun_angles: std::array::from_fn(|i| lerp(self.sun_angles[i], target.sun_angles[i])),
            color: std::array::from_fn(|i| lerp(self.color[i], target.color[i])),
            opacity: lerp(self.opacity, target.opacity),
            sun_color: std::array::from_fn(|i| lerp(self.sun_color[i], target.sun_color[i])),
            sun_opacity: lerp(self.sun_opacity, target.sun_opacity),
        }
    }

    pub(super) fn native(self) -> DistanceFog {
        DistanceFog {
            color: Color::linear_rgba(self.color[0], self.color[1], self.color[2], self.opacity),
            directional_light_color: Color::linear_rgba(
                self.sun_color[0],
                self.sun_color[1],
                self.sun_color[2],
                self.sun_opacity,
            ),
            directional_light_exponent: -(self.sun_angles[3] + 1.),
            falloff: FogFalloff::Atmospheric {
                extinction: Vec3::new(self.distance[0], self.distance[1], self.distance[2]),
                inscattering: Vec3::new(self.distance[3], self.sun_angles[0], self.sun_angles[1]),
            },
        }
    }
}

#[derive(Deserialize)]
struct SourceVolume {
    order: usize,
    bank: u8,
    script_id: u16,
    transition_seconds: f32,
    planes: Vec<[f32; 4]>,
    fog: WorldFog,
}

#[derive(Component)]
struct FogVolume {
    order: usize,
    bank: u8,
    script_id: u16,
    transition_seconds: f32,
    planes: Vec<Vec4>,
    fog: WorldFog,
}

#[derive(Component)]
pub(super) struct Sun;

#[derive(Resource)]
pub(super) struct AtmosphereState {
    pub shader_ready: bool,
    initialized: bool,
    bank: u8,
    volume: Option<usize>,
    script_id: u16,
    fog: WorldFog,
    from: WorldFog,
    target: WorldFog,
    elapsed: f32,
    duration: f32,
    pub vision_blend: f32,
    vision_from: f32,
    vision_target: f32,
    vision_elapsed: f32,
}

pub(super) struct AtmospherePlugin;
impl Plugin for AtmospherePlugin {
    fn build(&self, app: &mut App) {
        let source = SourceAtmosphere::default();
        app.insert_resource(AtmosphereState {
            shader_ready: false,
            initialized: false,
            bank: 1,
            volume: None,
            script_id: 0,
            fog: source.fog,
            from: source.fog,
            target: source.fog,
            elapsed: 0.,
            duration: 0.,
            vision_blend: 0.,
            vision_from: 0.,
            vision_target: 0.,
            vision_elapsed: 0.,
        })
        .insert_resource(source)
        .add_systems(Startup, (setup.after(super::setup), super::grading::setup))
        .add_systems(
            Update,
            prepare_shader.before(super::opaque::prepare_shaders),
        )
        .add_systems(Update, update.after(super::camera_follow))
        .add_systems(Update, super::grading::prepare.after(update))
        .add_systems(OnEnter(crate::GameState::PreparingShaders), reset);
    }
}

fn setup(mut commands: Commands, source: Res<SourceAtmosphere>) {
    for volume in &source.volumes {
        commands.spawn((
            FogVolume {
                order: volume.order,
                bank: volume.bank,
                script_id: volume.script_id,
                transition_seconds: volume.transition_seconds,
                planes: volume
                    .planes
                    .iter()
                    .copied()
                    .map(Vec4::from_array)
                    .collect(),
                fog: volume.fog,
            },
            Name::new(format!("Origins fog volume {}", volume.order)),
        ));
    }
}

fn reset(mut state: ResMut<AtmosphereState>) {
    state.initialized = false;
    state.vision_blend = 0.;
    state.vision_from = 0.;
    state.vision_target = 0.;
    state.vision_elapsed = 0.;
}

fn prepare_shader(mut shaders: ResMut<Assets<Shader>>, mut state: ResMut<AtmosphereState>) {
    if state.shader_ready {
        return;
    }
    let Some((id, native)) = shaders.iter().find(|(_, shader)| {
        shader.import_path == ShaderImport::Custom("bevy_pbr::render::pbr_functions".into())
    }) else {
        return;
    };
    let marker = "    let view_to_world = fragment_world_position.xyz - view_world_position.xyz;";
    let original = native.source.as_str();
    assert_eq!(
        original.matches(marker).count(),
        1,
        "Pinned native fog entry changed"
    );
    let replacement = format!(
        "    if fog_params.directional_light_exponent < 0.0 {{\n        return origins_fog::apply(fog_params, input_color, fragment_world_position, view_world_position);\n    }}\n{marker}"
    );
    let source = format!(
        "import origins::atmosphere_fog as origins_fog;\n{}",
        original.replacen(marker, &replacement, 1)
    );
    let mut patched = Shader::from_wesl(source, native.path.clone());
    patched.import_path = native.import_path.clone();
    patched.file_dependencies = native.file_dependencies.clone();
    patched.shader_defs = native.shader_defs.clone();
    shaders
        .insert(
            FOG_SHADER.id(),
            Shader::from_wesl(
                include_str!("atmosphere_fog.wesl"),
                "embedded://origins/atmosphere_fog.wesl",
            ),
        )
        .expect("Origins fog shader handle");
    shaders
        .insert(id, patched)
        .expect("Native fog shader handle");
    state.shader_ready = true;
}

fn update(
    source: Res<SourceAtmosphere>,
    profile: Res<VisualProfile>,
    weather: Res<Weather>,
    time: Res<Time>,
    volumes: Query<&FogVolume>,
    mut state: ResMut<AtmosphereState>,
    game_state: Res<State<crate::GameState>>,
    mut cameras: Query<(&Transform, &mut DistanceFog), (With<GameCamera>, Without<Sun>)>,
    mut lights: Query<(&mut DirectionalLight, &mut Transform), (With<Sun>, Without<GameCamera>)>,
) {
    let Ok((camera, mut fog)) = cameras.single_mut() else {
        return;
    };
    let delta = if matches!(
        **game_state,
        crate::GameState::Playing | crate::GameState::PreparingShaders
    ) {
        time.delta_secs()
    } else {
        0.
    };
    let bank = match weather.presented {
        WeatherKind::Clear => 1,
        WeatherKind::Rain => 2,
        WeatherKind::Snow => 4,
    };
    let point = camera.translation.extend(1.);
    let volume = volumes
        .iter()
        .filter(|volume| {
            volume.bank & bank != 0 && volume.planes.iter().all(|plane| plane.dot(point) <= 0.)
        })
        .min_by_key(|volume| volume.order);
    let order = volume.map(|volume| volume.order);
    if !state.initialized || state.bank != bank || state.volume != order {
        state.from = state.fog;
        state.target = volume.map_or(source.fog, |volume| volume.fog);
        state.duration = volume.map_or(source.fog_transition_seconds, |volume| {
            volume.transition_seconds
        });
        state.elapsed = if state.initialized {
            0.
        } else {
            state.duration
        };
        state.bank = bank;
        state.volume = order;
        state.script_id = volume.map_or(0, |volume| volume.script_id);
        state.initialized = true;
    }
    state.elapsed = (state.elapsed + delta).min(state.duration);
    let fraction = if state.duration > 0. {
        (state.elapsed / state.duration).min(1.)
    } else {
        1.
    };
    state.fog = state.from.blend(state.target, fraction);
    *fog = state.fog.native();
    let vision_target = if weather.presented == WeatherKind::Clear {
        0.
    } else {
        1.
    };
    if state.vision_target != vision_target {
        state.vision_from = state.vision_blend;
        state.vision_target = vision_target;
        state.vision_elapsed = 0.;
    }
    state.vision_elapsed = (state.vision_elapsed + delta).min(1.);
    state.vision_blend =
        state.vision_from + (state.vision_target - state.vision_from) * state.vision_elapsed;
    let color = Vec3::from_array(source.sun_color).lerp(
        Vec3::from_array(source.snow_sun_color),
        weather.sun_color_blend(),
    );
    let to_sun = source_direction(source.sun_angles[0], source.sun_angles[1]);
    for (mut light, mut transform) in &mut lights {
        light.color = Color::linear_rgb(color.x, color.y, color.z);
        light.illuminance = profile.sun_illuminance * weather.sun_brightness(source.sun_brightness)
            / source.sun_brightness;
        transform.rotation = Transform::IDENTITY.looking_to(-to_sun, Vec3::Y).rotation;
    }
}

pub(super) fn source_direction(pitch: f32, yaw: f32) -> Vec3 {
    let (sp, cp) = pitch.to_radians().sin_cos();
    let (sy, cy) = yaw.to_radians().sin_cos();
    Vec3::new(cp * cy, -sp, -cp * sy)
}

#[cfg(feature = "dev-dynamic")]
pub(crate) fn snapshot(world: &mut World) -> serde_json::Value {
    let state = world.resource::<AtmosphereState>();
    let mut result = serde_json::json!({
        "shader_ready": state.shader_ready, "bank": state.bank, "volume": state.volume,
        "script_id": state.script_id, "fog": state.fog, "vision_blend": state.vision_blend,
        "fog_transition_seconds": state.duration, "fog_elapsed": state.elapsed,
        "lut_ready": world.resource::<super::grading::SourceGrade>().ready(),
        "fog_volume_count": world.resource::<SourceAtmosphere>().volumes.len(),
    });
    result["sun"] =
        serde_json::json!(world.query_filtered::<(&DirectionalLight, &Transform), With<Sun>>()
        .iter(world).map(|(light, pose)| serde_json::json!({
            "color_linear": light.color.to_linear().to_f32_array(),
            "color_srgb": light.color.to_srgba().to_f32_array(), "illuminance": light.illuminance,
            "direction": pose.forward().as_vec3().to_array(),
        })).collect::<Vec<_>>());
    result["cameras"] = serde_json::json!(
        world
            .query::<(
                &Transform,
                &bevy::core_pipeline::tonemapping::Tonemapping,
                &bevy::camera::Exposure,
                Option<&bevy::light::Skybox>,
                Has<GameCamera>,
                Has<crate::presentation::ViewModelCamera>,
                Has<DistanceFog>,
            )>()
            .iter(world)
            .map(|(pose, tone, exposure, sky, world, viewmodel, fog)| {
                serde_json::json!({
                    "world": world, "viewmodel": viewmodel, "fog": fog,
                    "position": pose.translation.to_array(), "tonemapping": format!("{tone:?}"),
                    "ev100": exposure.ev100,
                    "sky": sky.map(|sky| serde_json::json!({
                        "brightness": sky.brightness, "rotation": sky.rotation.to_array(),
                        "loaded": sky.image.is_some(),
                    })),
                })
            })
            .collect::<Vec<_>>()
    );
    result
}

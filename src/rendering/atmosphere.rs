use super::{GameCamera, profile::VisualProfile};
use crate::environment::weather::{Weather, WeatherKind};
use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Component, Path};

#[derive(Resource, Deserialize)]
pub(crate) struct SourceAtmosphere {
    pub(super) sun_angles: [f32; 3],
    pub(super) sun_color: [f32; 3],
    pub(super) snow_sun_color: [f32; 3],
    pub(super) sun_brightness: f32,
    pub(super) exposure: f32,
    pub(super) fog: WorldFog,
    fog_transition_seconds: f32,
    volumes: Vec<SourceVolume>,
    modifiers: Vec<SourceModifier>,
    pub(super) softclip: f32,
    pub(super) vision: BTreeMap<String, BTreeMap<String, [f32; 4]>>,
    pub(super) sky_uri: String,
    pub(super) grade_uri: String,
    sky_rotation_columns: [[f32; 3]; 3],
}

impl SourceAtmosphere {
    fn load(pack: &crate::pack::Pack, art: &crate::assets::prepared::PreparedArt) -> Result<Self, String> {
        let uri = art
            .world()
            .and_then(|world| world["atmosphere_uri"].as_str())
            .ok_or("Prepared Origins world atmosphere is missing")?;
        Self::validate_uri(uri)?;
        let source: Self = serde_json::from_slice(pack.get(uri).map_err(|error| error.to_string())?)
            .map_err(|error| format!("Invalid runtime atmosphere {uri}: {error}"))?;
        Self::validate_uri(&source.sky_uri)?;
        Self::validate_uri(&source.grade_uri)?;
        for bank in ["clear", "_rain", "_snow"] {
            let vision = source
                .vision
                .get(bank)
                .ok_or_else(|| format!("Prepared atmosphere is missing vision bank {bank}"))?;
            for key in [
                "vc_RS", "vc_RE", "vc_FSM", "vc_SMR", "vc_MMR", "vc_HMR", "vc_SMG", "vc_MMG",
                "vc_HMG", "vc_SMB", "vc_MMB", "vc_HMB", "vc_FGM", "vc_FBM",
            ] {
                if !vision
                    .get(key)
                    .is_some_and(|value| value.iter().all(|component| component.is_finite()))
                {
                    return Err(format!("Prepared atmosphere is missing valid {bank}/{key}"));
                }
            }
        }
        if !(source.sun_brightness.is_finite() && source.sun_brightness > 0.)
            || !source.exposure.is_finite()
            || !source
                .sky_rotation_columns
                .iter()
                .flatten()
                .all(|value| value.is_finite())
        {
            return Err(
                "Prepared atmosphere has invalid exposure, sunlight or sky rotation".into(),
            );
        }
        for modifier in &source.modifiers {
            if !modifier.transition_rate.is_finite()
                || modifier.transition_rate < 0.
                || !modifier.factors().iter().all(|value| value.is_finite())
                || modifier.planes.is_empty()
                || !modifier.planes.iter().flatten().all(|value| value.is_finite())
            {
                return Err(format!("Invalid source fog modifier {}", modifier.order));
            }
        }
        Ok(source)
    }

    fn validate_uri(uri: &str) -> Result<(), String> {
        if uri.is_empty()
            || Path::new(uri)
                .components()
                .any(|component| !matches!(component, Component::Normal(_)))
        {
            return Err(format!("Invalid prepared atmosphere asset URI: {uri}"));
        }
        Ok(())
    }

    pub(super) fn sky_rotation(&self) -> Quat {
        Quat::from_mat3(&Mat3::from_cols_array_2d(&self.sky_rotation_columns))
    }

    pub(crate) fn camera_exposure(&self, profile: &VisualProfile) -> bevy::camera::Exposure {
        let radiance_scale = profile.sun_illuminance / (self.sun_brightness * std::f32::consts::PI);
        bevy::camera::Exposure {
            ev100: self.exposure + (radiance_scale / 1.2).log2(),
        }
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
    fn modified(mut self, factors: [f32; 6]) -> Self {
        self.distance[1] *= factors[0];
        for channel in 0..3 {
            self.color[channel] *= factors[channel + 2];
            self.sun_color[channel] *= factors[channel + 2];
        }
        self.opacity *= factors[5];
        self.sun_opacity *= factors[5];
        self
    }

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

    pub(super) fn native(self, exposure: f32) -> DistanceFog {
        let scale = 2.0_f32.powf(-exposure);
        DistanceFog {
            color: Color::linear_rgba(
                self.color[0] * scale,
                self.color[1] * scale,
                self.color[2] * scale,
                self.opacity,
            ),
            directional_light_color: Color::linear_rgba(
                self.sun_color[0] * scale,
                self.sun_color[1] * scale,
                self.sun_color[2] * scale,
                self.sun_opacity,
            ),
            directional_light_exponent: 1.
                / (1. - self.sun_angles[3].to_radians().cos()).max(0.001),
            falloff: FogFalloff::Exponential {
                density: std::f32::consts::LN_2 * std::f32::consts::LN_2
                    / self.distance[1].max(0.0254),
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

#[derive(Deserialize)]
struct SourceModifier {
    order: usize,
    control: u32,
    transition_rate: f32,
    depth_scale: f32,
    height_scale: f32,
    color_adjust: [f32; 4],
    planes: Vec<[f32; 4]>,
}

impl SourceModifier {
    fn factors(&self) -> [f32; 6] {
        let [r, g, b, a] = self.color_adjust;
        [self.depth_scale, self.height_scale, r, g, b, a]
    }
}

struct ModifierTransition {
    target: Option<usize>,
    elapsed_ms: f32,
    rate: f32,
    from: [f32; 6],
    current: [f32; 6],
    previous_camera: Option<Vec3>,
}

impl Default for ModifierTransition {
    fn default() -> Self {
        Self { target:None, elapsed_ms:0., rate:1., from:[1.; 6], current:[1.; 6], previous_camera:None }
    }
}

impl ModifierTransition {
    fn update(&mut self, source: &SourceAtmosphere, point: Vec3, mask: u8, delta: f32, reset: bool) {
        if source.volumes.is_empty() {
            *self = Self::default();
            return;
        }
        let selected = source.modifiers.iter().enumerate()
            .filter(|(_, modifier)| modifier.control & u32::from(mask & 15) != 0
                && modifier.planes.iter().all(|plane| Vec4::from_array(*plane).dot(point.extend(1.)) <= 0.))
            .min_by_key(|(_, modifier)| modifier.order).map(|(index, _)| index);
        let snap = reset || self.previous_camera.is_none_or(|previous| previous.distance_squared(point) > (1024_f32 * 0.0254).powi(2));
        self.previous_camera = Some(point);
        if snap {
            self.target = selected;
            self.elapsed_ms = 0.;
            self.current = selected.map_or([1.; 6], |index| source.modifiers[index].factors());
            self.from = self.current;
            self.rate = selected.map_or(1., |index| source.modifiers[index].transition_rate);
            return;
        }
        self.elapsed_ms += delta * 1000.;
        let t = (self.elapsed_ms * self.rate).min(1.);
        let weight = t*t*(3. - 2.*t);
        let target = self.target.map_or([1.; 6], |index| source.modifiers[index].factors());
        self.current = std::array::from_fn(|index| self.from[index] * (1. - weight) + target[index] * weight);
        if selected != self.target {
            self.target = selected;
            self.from = self.current;
            self.elapsed_ms = 0.;
            if let Some(index) = selected { self.rate = source.modifiers[index].transition_rate; }
        }
    }
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
    pub(super) bank: u8,
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
    modifier: ModifierTransition,
}

pub(super) struct AtmospherePlugin;
impl Plugin for AtmospherePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            OnEnter(crate::GameState::Loading),
            load.after(crate::assets::begin_load),
        )
        .add_systems(
            Update,
            (super::configure, setup, super::sky::setup, super::grading::setup)
                .chain()
                .run_if(resource_exists::<SourceAtmosphere>.and_then(resource_changed::<SourceAtmosphere>))
                .before(update),
        )
        .add_systems(
            Update,
            update.after(super::camera_follow).run_if(resource_exists::<SourceAtmosphere>),
        )
        .add_systems(
            Update,
            super::grading::prepare.after(update).run_if(resource_exists::<super::grading::SourceGrade>),
        )
        .add_systems(
            OnEnter(crate::GameState::PreparingShaders),
            reset.run_if(resource_exists::<AtmosphereState>),
        );
    }
}

fn load(
    mut commands: Commands,
    configuration: Res<crate::assets::LoadConfiguration>,
    art: Option<Res<crate::assets::prepared::PreparedArt>>,
    mut failure: ResMut<crate::assets::LoadFailure>,
    mut next: ResMut<NextState<crate::GameState>>,
) {
    commands.remove_resource::<SourceAtmosphere>();
    commands.remove_resource::<AtmosphereState>();
    commands.remove_resource::<super::grading::SourceGrade>();
    let (Ok(configuration), Some(art)) = (&configuration.0, art) else {
        return;
    };
    match SourceAtmosphere::load(&configuration.contents, &art) {
        Ok(source) => {
            commands.insert_resource(AtmosphereState {
                shader_ready: true,
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
                modifier: default(),
            });
            commands.insert_resource(source);
        }
        Err(error) => {
            failure.0 = error;
            next.set(crate::GameState::LoadError);
        }
    }
}

fn setup(
    mut commands: Commands,
    source: Res<SourceAtmosphere>,
    existing: Query<Entity, With<FogVolume>>,
) {
    for entity in &existing {
        commands.entity(entity).despawn();
    }
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
    state.modifier = default();
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
    } | (state.bank & 8);
    let reset_modifier = !state.initialized;
    state.modifier.update(&source, camera.translation, bank, delta, reset_modifier);
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
    state.fog = state
        .from
        .blend(state.target, fraction * fraction * (3. - 2. * fraction));
    *fog = state.fog.modified(state.modifier.current).native(source.exposure);
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

#[cfg(feature = "dev-tools")]
pub(crate) fn snapshot(world: &mut World) -> serde_json::Value {
    let state = world.resource::<AtmosphereState>();
    let mut result = serde_json::json!({
        "shader_ready": state.shader_ready, "bank": state.bank, "volume": state.volume,
        "script_id": state.script_id, "fog": state.fog, "vision_blend": state.vision_blend,
        "modifier": state.modifier.target, "modifier_factors": state.modifier.current,
        "fog_transition_seconds": state.duration, "fog_elapsed": state.elapsed,
        "lut_ready": world.resource::<super::grading::SourceGrade>().ready(),
        "grading": super::grading::snapshot(world),
        "fog_volume_count": world.resource::<SourceAtmosphere>().volumes.len(),
        "fog_exposure": world.resource::<SourceAtmosphere>().exposure,
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
                Option<&DistanceFog>,
                Has<bevy::post_process::auto_exposure::AutoExposure>,
                Option<&bevy::post_process::bloom::Bloom>,
            )>()
            .iter(world)
            .map(|(pose, tone, exposure, sky, world, fog, automatic, bloom)| {
                serde_json::json!({
                    "world": world, "fog": fog.is_some(),
                    "auto_exposure": automatic, "bloom": bloom.map(|b| b.intensity),
                    "fog_color": fog.map(|fog| fog.color.to_linear().to_f32_array()),
                    "sun_fog_color": fog.map(|fog| fog.directional_light_color.to_linear().to_f32_array()),
                    "position": pose.translation.to_array(), "tonemapping": format!("{tone:?}"),
                    "ev100": exposure.ev100, "exposure_multiplier": exposure.exposure(),
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

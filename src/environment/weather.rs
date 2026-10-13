use crate::{
    GameState,
    assets::GameDefinition,
    gameplay::{GameStep, Session, SessionEntity},
    player::Player,
};
use avian3d::prelude::*;
use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WeatherKind {
    #[default]
    Clear,
    Rain,
    Snow,
}

#[derive(Resource, Serialize)]
pub struct Weather {
    pub kind: WeatherKind,
    pub intensity: u32,
    pub presented: WeatherKind,
    pub presented_intensity: u32,
    pub last_snow: u32,
    pub last_rain: u32,
    pub overrides: BTreeMap<u32, WeatherKind>,
    pub transitions: Vec<serde_json::Value>,
    pub ambient_room: String,
    pub ambient_alias: String,
    pub precipitation_bursts: u64,
    pub strikes: u64,
    pub flash: f32,
    pub(crate) delay: f32,
    color_blend: f32,
    color_from: f32,
    color_remaining: f32,
    client_seed: u64,
    audio_delay: f32,
    precipitation_clock: f32,
    lightning_clock: f32,
    flashes_left: u32,
    flash_duration: f32,
    flash_peak: f32,
    strike_position: Vec3,
    #[serde(skip)]
    audio: Option<Entity>,
}
impl Weather {
    pub(crate) fn sun_color_blend(&self) -> f32 {
        self.color_blend
    }

    pub(crate) fn sun_brightness(&self, normal: f32) -> f32 {
        normal + (self.flash_peak * 30. - normal) * (self.flash / self.flash_peak).clamp(0., 1.)
    }

    fn random(&mut self) -> u32 {
        self.client_seed = self
            .client_seed
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1);
        (self.client_seed >> 32) as u32
    }
}
impl Default for Weather {
    fn default() -> Self {
        Self {
            kind: WeatherKind::Clear,
            intensity: 0,
            presented: WeatherKind::Clear,
            presented_intensity: 0,
            last_snow: 0,
            last_rain: 0,
            overrides: BTreeMap::new(),
            transitions: Vec::new(),
            ambient_room: String::new(),
            ambient_alias: String::new(),
            precipitation_bursts: 0,
            strikes: 0,
            flash: 0.,
            delay: 0.,
            color_blend: 0.,
            color_from: 0.,
            color_remaining: 0.,
            client_seed: 0xA17C5,
            audio_delay: 0.,
            precipitation_clock: 0.,
            lightning_clock: 0.,
            flashes_left: 0,
            flash_duration: 0.2,
            flash_peak: 1.,
            strike_position: Vec3::ZERO,
            audio: None,
        }
    }
}
pub(crate) fn reset(world: &mut World) {
    let mut weather = Weather::default();
    let mut session = world.resource_mut::<Session>();
    weather.overrides.insert(
        if session.random() % 2 == 0 { 3 } else { 4 },
        WeatherKind::Snow,
    );
    for round in 5..=9 {
        weather.overrides.insert(
            round,
            if session.random() % 2 == 0 {
                WeatherKind::Clear
            } else {
                WeatherKind::Rain
            },
        );
    }
    weather.overrides.insert(10, WeatherKind::Snow);
    world.insert_resource(weather);
}

pub struct WeatherPlugin;
impl Plugin for WeatherPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Weather>()
            .add_systems(
                FixedUpdate,
                (round_weather, tick)
                    .chain()
                    .after(GameStep::Rounds)
                    .run_if(in_state(GameState::Playing)),
            )
            .add_systems(Update, particles.run_if(in_state(GameState::Playing)));
    }
}

fn round_weather(
    id: Res<crate::session::SessionId>,
    mut ending: MessageReader<crate::rounds::RoundEnding>,
    mut weather: ResMut<Weather>,
    mut session: ResMut<Session>,
) {
    for event in ending.read() {
        if event.session != *id {
            continue;
        }
        let kind = weather
            .overrides
            .get(&event.round)
            .copied()
            .unwrap_or_else(|| {
                let roll = session.random() % 100;
                if roll < 40 || event.round.saturating_sub(weather.last_snow) > 3 {
                    WeatherKind::Snow
                } else if roll < 80 || event.round.saturating_sub(weather.last_rain) > 4 {
                    WeatherKind::Rain
                } else {
                    WeatherKind::Clear
                }
            });
        weather.kind = kind;
        weather.intensity = if kind == WeatherKind::Clear {
            0
        } else {
            1 + session.random() % 4
        };
        match kind {
            WeatherKind::Snow => weather.last_snow = event.round,
            WeatherKind::Rain => weather.last_rain = event.round,
            _ => {}
        }
        weather.delay = 2.;
        if weather.transitions.len() >= 128 {
            weather.transitions.remove(0);
        }
        let intensity = weather.intensity;
        weather
            .transitions
            .push(serde_json::json!({"round":event.round,"kind":kind,"intensity":intensity}));
    }
}

#[derive(Component)]
struct Precipitation {
    velocity: Vec3,
    remaining: f32,
}

#[cfg(all(debug_assertions, feature = "dev-tools"))]
pub(crate) fn precipitation_snapshot(world: &mut World) -> serde_json::Value {
    serde_json::json!(world.query::<(&Precipitation, &Transform)>().iter(world).map(|(drop, pose)| {
        serde_json::json!({"position":pose.translation.to_array(),"velocity":drop.velocity.to_array(),"remaining":drop.remaining})
    }).collect::<Vec<_>>())
}

#[derive(Resource)]
pub(crate) struct PrecipitationAssets {
    rain: Handle<Mesh>,
    snow: Handle<Mesh>,
    material: Handle<StandardMaterial>,
}

pub(crate) fn tick(
    mut commands: Commands,
    time: Res<Time<Fixed>>,
    game: Res<GameDefinition>,
    mut weather: ResMut<Weather>,
    players: Query<(Entity, &Player, &Transform, &Collider)>,
    spatial: SpatialQuery,
    triggers: Query<&super::triggers::MapTrigger>,
    exists: Query<()>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    assets: Option<Res<PrecipitationAssets>>,
) {
    let Ok((player_entity, player, pose, collider)) = players.single() else {
        return;
    };
    let dt = time.delta_secs();
    if weather.delay > 0. {
        weather.delay = (weather.delay - dt).max(0.);
        if weather.delay == 0. {
            weather.presented = weather.kind;
            weather.presented_intensity = weather.intensity;
            weather.audio_delay = 1.1;
            weather.precipitation_clock = 0.;
            weather.lightning_clock = 0.;
            weather.flashes_left = 0;
            weather.flash = 0.;
            weather.color_from = weather.color_blend;
            weather.color_remaining = 10.;
        }
    }
    let audio_before = weather.audio_delay;
    weather.audio_delay = (weather.audio_delay - dt).max(0.);
    if audio_before > 0.5 && weather.audio_delay <= 0.5 {
        if let Some(entity) = weather.audio.take() {
            commands.entity(entity).try_insert(crate::audio::SoundFade {
                remaining: 0.5,
                duration: 0.5,
                out: true,
            });
        }
        weather.ambient_alias.clear();
    }
    let room = spatial
        .shape_intersections(
            collider,
            pose.translation,
            pose.rotation,
            &SpatialQueryFilter::from_mask(super::triggers::TRIGGER_LAYER),
        )
        .into_iter()
        .filter_map(|e| triggers.get(e).ok())
        .filter_map(|t| game.environment.triggers.iter().find(|d| d.id == t.0))
        .filter(|d| {
            d.source
                .get("targetname")
                .is_some_and(|n| n == "ambient_package")
        })
        .min_by_key(|d| {
            d.source
                .get("script_ambientpriority")
                .and_then(|n| n.parse::<u32>().ok())
                .unwrap_or(99)
        })
        .and_then(|d| d.source.get("script_ambientroom"))
        .map_or("tomb_outdoor", String::as_str);
    let suffix = match room {
        "tomb_cave" => "amb_cave_wind_2d".to_owned(),
        "tomb_spiral_staircase_bot" => "amb_spiral_rm_air_2d".to_owned(),
        "tomb_robot_head" => "amb_robot_head_2d".to_owned(),
        "tomb_large_chamber" => String::new(),
        _ => format!(
            "amb_weather_2d_{}_{}",
            match weather.presented {
                WeatherKind::Clear => "default",
                WeatherKind::Rain => "rain",
                WeatherKind::Snow => "snow",
            },
            if matches!(
                room,
                "tomb_bunker_sml"
                    | "tomb_bunker_med"
                    | "tomb_bunker_lrg"
                    | "tomb_spiral_staircase"
                    | "tomb_church"
            ) {
                "indoor"
            } else {
                "outdoor"
            }
        ),
    };
    weather.ambient_room = room.into();
    if weather.audio.is_some_and(|e| !exists.contains(e)) {
        weather.audio = None;
        weather.ambient_alias.clear();
    }
    if weather.audio_delay == 0. && suffix != weather.ambient_alias {
        if let Some(entity) = weather.audio.take() {
            commands.entity(entity).try_insert(crate::audio::SoundFade {
                remaining: 0.5,
                duration: 0.5,
                out: true,
            });
        }
        weather.ambient_alias = suffix.clone();
        if game.audio.aliases.contains_key(&suffix) {
            let entity = crate::audio::emit_loop(
                &mut commands,
                suffix,
                crate::audio::SoundSource::attached(player_entity),
            );
            commands.entity(entity).insert(crate::audio::SoundFade {
                remaining: 0.5,
                duration: 0.5,
                out: false,
            });
            weather.audio = Some(entity);
        }
    }
    let target = if weather.presented == WeatherKind::Snow {
        1.
    } else {
        0.
    };
    weather.color_remaining = (weather.color_remaining - dt).max(0.);
    weather.color_blend =
        weather.color_from + (target - weather.color_from) * (1. - weather.color_remaining / 10.);
    weather.flash = (weather.flash - dt * weather.flash_peak / weather.flash_duration).max(0.);
    if weather.presented == WeatherKind::Rain {
        weather.lightning_clock -= dt;
        if weather.lightning_clock <= 0. {
            if weather.flashes_left == 0 {
                let forward =
                    Quat::from_euler(EulerRot::YXZ, player.yaw, player.pitch, 0.) * Vec3::NEG_Z;
                weather.strike_position = pose.translation
                    + Vec3::new(forward.x, 0., forward.z) * 635.
                    + Vec3::new(
                        (weather.random() % 10000) as f32 * 0.0254 - 127.,
                        (weather.random() % 3000) as f32 * 0.0254,
                        (weather.random() % 10000) as f32 * 0.0254 - 127.,
                    );
                sound(
                    &mut commands,
                    &game,
                    "amb_thunder_clap_zm",
                    weather.strike_position,
                );
                weather.strikes += 1;
                weather.flashes_left = 3 + weather.random() % 2;
                weather.lightning_clock = 0.2 + (weather.random() % 1000) as f32 * 0.0001 + 0.1;
            } else {
                sound(
                    &mut commands,
                    &game,
                    "amb_thunder_flash_zm",
                    weather.strike_position,
                );
                weather.flash_duration = 0.1 + (weather.random() % 10000) as f32 * 0.000025;
                weather.flash_peak = (28. + (weather.random() % 10000) as f32 * 0.0004) / 30.;
                weather.flash = weather.flash_peak;
                weather.flashes_left -= 1;
                weather.lightning_clock = (weather.flash_duration / 0.05).ceil() * 0.05
                    + if weather.flashes_left == 0 {
                        5. + (weather.random() % 10000) as f32 * 0.0005
                    } else {
                        0.1
                    };
            }
        }
    } else {
        weather.flash = 0.;
        weather.flashes_left = 0;
    }
    if weather.presented == WeatherKind::Clear {
        return;
    }
    let Some(assets) = assets else {
        commands.insert_resource(PrecipitationAssets {
            rain: meshes.add(Cuboid::new(0.008, 0.3, 0.008)),
            snow: meshes.add(Cuboid::new(0.025, 0.025, 0.025)),
            material: materials.add(StandardMaterial {
                base_color: Color::srgba(0.82, 0.88, 1., 0.65),
                unlit: true,
                alpha_mode: AlphaMode::Blend,
                ..default()
            }),
        });
        return;
    };
    weather.precipitation_clock -= dt;
    if weather.precipitation_clock > 0. {
        return;
    }
    weather.precipitation_clock += match weather.presented {
        WeatherKind::Rain => (0.35 / weather.presented_intensity.max(1) as f32).max(0.15),
        _ => 0.5 / weather.presented_intensity.max(1) as f32,
    };
    weather.precipitation_bursts += 1;
    for _ in 0..24 {
        let position = pose.translation
            + Vec3::new(
                (weather.random() % 10000) as f32 * 0.0016 - 8.,
                6.,
                (weather.random() % 10000) as f32 * 0.0016 - 8.,
            );
        let floor = spatial
            .cast_ray(
                position,
                Dir3::NEG_Y,
                12.,
                true,
                &super::solids([player_entity]),
            )
            .map_or(12., |h| h.distance);
        let roof = spatial
            .cast_ray(
                position - Vec3::Y * 6.,
                Dir3::Y,
                100.,
                true,
                &super::solids([player_entity]),
            )
            .is_some();
        if roof || floor < 0.3 {
            continue;
        }
        let speed = if weather.presented == WeatherKind::Rain {
            20.
        } else {
            1.8
        };
        commands.spawn((
            SessionEntity,
            Precipitation {
                velocity: Vec3::new(if speed < 2. { 0.3 } else { 1. }, -speed, 0.),
                remaining: floor / speed,
            },
            Mesh3d(if speed < 2. {
                assets.snow.clone()
            } else {
                assets.rain.clone()
            }),
            MeshMaterial3d(assets.material.clone()),
            Transform::from_translation(position),
            bevy::light::NotShadowCaster,
        ));
    }
}

fn sound(commands: &mut Commands, game: &GameDefinition, alias: &str, position: Vec3) {
    if game.audio.aliases.contains_key(alias) {
        crate::audio::emit(commands, alias, crate::audio::SoundSource::World(position));
    }
}

fn particles(
    mut commands: Commands,
    time: Res<Time>,
    weather: Res<Weather>,
    spatial: SpatialQuery,
    players: Query<Entity, With<Player>>,
    mut drops: Query<(Entity, &mut Precipitation, &mut Transform)>,
) {
    let filter = super::solids(players.iter());
    for (e, mut drop, mut pose) in &mut drops {
        drop.remaining -= time.delta_secs();
        let movement = drop.velocity * time.delta_secs();
        let collision = Dir3::new(movement).ok().is_some_and(|direction| {
            spatial
                .cast_ray(
                    pose.translation,
                    direction,
                    movement.length() + 0.15,
                    true,
                    &filter,
                )
                .is_some()
        });
        if weather.presented == WeatherKind::Clear || drop.remaining <= 0. || collision {
            commands.entity(e).despawn();
        } else {
            pose.translation += movement;
        }
    }
}

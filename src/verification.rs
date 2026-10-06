//! Opt-in bounded verification of the actual application, never a second viewer.
use crate::{
    GameState,
    assets::{GameDefinition, LaunchOptions, LoadFailure},
    gameplay::{Session, Zombie},
    input::GameInput,
    player::Player,
};
use bevy::{
    prelude::*,
    render::view::screenshot::{Screenshot, save_to_disk},
};
use std::{path::PathBuf, time::Instant};
mod arsenal;
mod performance;
mod pipelines;

#[derive(Resource)]
pub(crate) struct Verification {
    output: PathBuf,
    benchmark: bool,
    combat: bool,
    only_view: bool,
    moving_view: bool,
    started: Instant,
    playing: f32,
    stage: usize,
    initial: Option<Vec3>,
    moved: bool,
    survey: usize,
    round_two: bool,
    water_times: Vec<f32>,
    camera_views: Vec<serde_json::Value>,
    reported: u32,
    ads_seen: bool,
    recoil_seen: bool,
    wall_weapons: bool,
}
pub struct VerificationPlugin;
impl Plugin for VerificationPlugin {
    fn build(&self, app: &mut App) {
        if app.world().resource::<LaunchOptions>().verify.is_some() {
            pipelines::install(app);
            if app.world().resource::<LaunchOptions>().verify_wall_weapons {
                arsenal::install(app);
            }
        }
        app.add_systems(Startup, setup)
            .add_systems(
                PreUpdate,
                drive
                    .after(bevy::input::InputSystems)
                    .after(crate::input::collect),
            )
            .add_systems(Update, check)
            .add_systems(Last, performance::record);
    }
}
fn setup(mut commands: Commands, options: Res<LaunchOptions>) {
    if let Some(output) = &options.verify {
        if output.starts_with(env!("CARGO_MANIFEST_DIR")) {
            error!("Verification output must be external");
            commands.write_message(AppExit::error());
            return;
        }
        if let Err(error) = std::fs::create_dir_all(output) {
            error!("Verification directory: {error}");
            commands.write_message(AppExit::error());
            return;
        }
        commands.insert_resource(Verification {
            output: output.clone(),
            benchmark: options.benchmark,
            combat: options.benchmark_combat,
            only_view: options.benchmark_view.is_some(),
            moving_view: options.benchmark_motion,
            started: Instant::now(),
            playing: 0.,
            stage: 0,
            initial: None,
            moved: false,
            survey: options.benchmark_view.unwrap_or(0),
            round_two: false,
            water_times: Vec::new(),
            camera_views: Vec::new(),
            reported: 0,
            ads_seen: false,
            recoil_seen: false,
            wall_weapons: options.verify_wall_weapons,
        });
    }
}
pub(crate) fn drive(
    verification: Option<Res<Verification>>,
    mut input: ResMut<GameInput>,
    players: Query<(Entity, &Player, &Transform)>,
    enemies: Query<&Transform, (With<Zombie>, Without<Player>)>,
    inventories: Query<&crate::weapons::WeaponInventory, With<Player>>,
    definition: Option<Res<GameDefinition>>,
    query: avian3d::prelude::SpatialQuery,
) {
    let (Some(verification), Some(definition), Ok((entity, player, transform))) =
        (verification, definition, players.single())
    else {
        return;
    };
    if verification.stage != 0 {
        if verification.stage != 4 {
            input.clear();
        }
        return;
    }
    input.movement = if verification.playing < 1. {
        Vec2::new(0.25, 0.35)
    } else {
        Vec2::ZERO
    };
    if let Some(target) = enemies.iter().min_by(|a, b| {
        a.translation
            .distance_squared(transform.translation)
            .total_cmp(&b.translation.distance_squared(transform.translation))
    }) {
        let delta = target.translation + Vec3::Y * 0.65
            - (transform.translation
                + Vec3::Y * (definition.tuning.eye_height - player.height * 0.5));
        let yaw = (-delta.x).atan2(-delta.z);
        if verification.playing >= 1. && delta.length() < 3. {
            input.movement = Vec2::new(0., -0.7);
        }
        let pitch = delta.y.atan2(Vec2::new(delta.x, delta.z).length());
        let yaw_delta = (yaw - player.yaw + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU)
            - std::f32::consts::PI;
        input.look =
            Vec2::new(-yaw_delta, player.pitch - pitch) / definition.tuning.mouse_sensitivity;
        if let Ok(inventory) = inventories.single() {
            input.look += inventory.recoil / definition.tuning.mouse_sensitivity;
            let sensitivity = definition.tuning.mouse_sensitivity
                + (definition.tuning.ads_mouse_sensitivity - definition.tuning.mouse_sensitivity)
                    * inventory.ads;
            input.look *= definition.tuning.mouse_sensitivity / sensitivity;
            input.ads = verification.playing >= 1.;
        }
        let origin =
            transform.translation + Vec3::Y * (definition.tuning.eye_height - player.height * 0.5);
        input.fire = Dir3::new(delta)
            .ok()
            .and_then(|dir| {
                query.cast_ray(
                    origin,
                    dir,
                    delta.length() + 0.3,
                    true,
                    &avian3d::prelude::SpatialQueryFilter::from_excluded_entities([entity]),
                )
            })
            .is_some_and(|hit| enemies.contains(hit.entity));
        input.melee = delta.length() < 2.;
        input.reload = inventories
            .single()
            .ok()
            .and_then(|i| i.active())
            .is_some_and(|w| w.magazine == 0);
    }
}
fn check(
    mut commands: Commands,
    time: Res<Time>,
    real_time: Res<Time<Real>>,
    verification: Option<ResMut<Verification>>,
    state: Res<State<GameState>>,
    failure: Res<LoadFailure>,
    definition: Option<Res<GameDefinition>>,
    session: Res<Session>,
    mut players: Query<
        (
            Entity,
            &mut Player,
            &mut Transform,
            &crate::weapons::WeaponInventory,
        ),
        Without<Zombie>,
    >,
    mut next: ResMut<NextState<GameState>>,
    enemies: Query<&Transform, (With<Zombie>, Without<Player>)>,
    query: avian3d::prelude::SpatialQuery,
    surfaces: Res<crate::rendering::OpaqueSurfaceStatus>,
    profile: Res<crate::rendering::profile::VisualProfile>,
    mut cameras: Query<
        (&Transform, &mut Projection),
        (
            With<crate::rendering::GameCamera>,
            Without<Player>,
            Without<Zombie>,
        ),
    >,
    mut overlays: Query<
        (&mut Visibility, Option<&crate::hud::Crosshair>),
        With<crate::hud::PlayerOverlay>,
    >,
) {
    let Some(mut verification) = verification else {
        return;
    };
    let ammo = players
        .iter()
        .next()
        .and_then(|(_, _, _, i)| i.active())
        .map_or(0, |w| w.magazine);
    if verification.stage == 0 {
        for (_, _, _, inventory) in &players {
            verification.recoil_seen |= inventory.recoil.length_squared() > 0.000001;
            if inventory.ads >= 0.99 && !verification.ads_seen {
                verification.ads_seen = true;
                commands
                    .spawn(Screenshot::primary_window())
                    .observe(save_to_disk(verification.output.join("ads.png")));
            }
        }
    }
    if verification.benchmark
        && !verification.combat
        && verification.stage == 0
        && **state == GameState::Playing
    {
        // Rendering measurements use an isolated paused survey. Combat remains
        // exercised separately by --verify, at the optimized frame rate.
        verification.stage = 1;
        verification.playing = 0.;
        next.set(GameState::Paused);
    }
    for (mut visibility, crosshair) in &mut overlays {
        if crosshair.is_some() && verification.stage != 1 {
            continue;
        }
        *visibility = if verification.stage == 1 {
            Visibility::Hidden
        } else {
            Visibility::Inherited
        };
    }
    if verification.started.elapsed().as_secs() > if verification.benchmark { 360 } else { 180 }
        || **state == GameState::LoadError
        || (**state == GameState::GameOver && verification.stage == 0)
    {
        let evidence = serde_json::json!({"passed": false, "state": format!("{:?}", **state),
            "round": session.round, "pending": session.pending, "kills": session.kills,
            "shots": session.shots, "ammo": ammo, "load_failure": failure.0,
            "elapsed_seconds": verification.started.elapsed().as_secs()});
        let _ = std::fs::write(
            verification.output.join("failure.json"),
            evidence.to_string(),
        );
        error!(
            "Game verification failed: {:?}, round {}, pending {}, kills {}, shots {}, ammo {}, {}",
            **state, session.round, session.pending, session.kills, session.shots, ammo, failure.0
        );
        commands.write_message(AppExit::error());
        return;
    }
    if verification.stage == 2 && **state == GameState::GameOver {
        commands.write_message(crate::input::RestartGame);
        verification.stage = 3;
        return;
    }
    if **state != GameState::Playing && !(verification.stage == 1 && **state == GameState::Paused) {
        return;
    }
    verification.playing += time.delta_secs();
    let (Some(definition), Ok((_, mut player, mut transform, _))) =
        (definition, players.single_mut())
    else {
        return;
    };
    if verification.initial.is_none() {
        verification.initial = Some(transform.translation);
        commands
            .spawn(Screenshot::primary_window())
            .observe(save_to_disk(verification.output.join("spawn.png")));
    }
    verification.moved |= verification
        .initial
        .is_some_and(|p| p.distance(transform.translation) > 0.1);
    if verification.stage == 0 && verification.playing as u32 / 5 > verification.reported {
        verification.reported = verification.playing as u32 / 5;
        info!(
            "Verification position {:?}, grounded {}, pending {}, living {}, kills {}, shots {}, ammo {}",
            transform.translation,
            player.grounded,
            session.pending,
            enemies.iter().count(),
            session.kills,
            session.shots,
            ammo
        );
        for point in definition.spawn_points.iter().take(3) {
            let position = Vec3::from_array(*point);
            let floor = query.cast_ray(
                position + Vec3::Y,
                Dir3::NEG_Y,
                4.,
                false,
                &avian3d::prelude::SpatialQueryFilter::default(),
            );
            info!(
                "Entry {:?}, floor {:?}",
                point,
                floor.map(|f| (f.distance, f.normal))
            );
        }
        commands
            .spawn(Screenshot::primary_window())
            .observe(save_to_disk(verification.output.join("playing.png")));
    }
    if verification.stage == 0 && session.round >= 2 {
        verification.round_two = true;
        verification.stage = 1;
        if verification.wall_weapons {
            verification.stage = 4;
            return;
        }
        verification.playing = 0.;
        next.set(GameState::Paused);
        commands
            .spawn(Screenshot::primary_window())
            .observe(save_to_disk(verification.output.join("round-two.png")));
    }
    if verification.stage == 1 && verification.playing > 1. {
        let view = visual_view(verification.survey, &definition);
        if let Some((position, yaw, pitch)) = view {
            if let Ok((_, mut projection)) = cameras.single_mut()
                && let Projection::Perspective(perspective) = &mut *projection
            {
                // Comparison-only projection; ordinary player projection is unchanged.
                perspective.fov = profile.comparison_fov_degrees.to_radians();
            }
            let sweep = if verification.moving_view {
                ((verification.playing - 1.) * 0.6).sin()
            } else {
                (verification.playing - 1.).clamp(0., 1.)
            };
            let moving =
                verification.moving_view || verification.survey < definition.survey_views.len();
            transform.translation =
                position + Vec3::new(if moving { sweep * 0.3 } else { 0. }, 0., 0.);
            player.velocity = Vec3::ZERO;
            player.yaw = yaw
                + if moving {
                    if verification.moving_view {
                        if verification.playing >= 60. {
                            ((verification.playing - 60.) / 0.5).floor().min(11.)
                                * std::f32::consts::TAU
                                / 12.
                        } else {
                            (verification.playing - 1.) * 0.5
                        }
                    } else {
                        sweep * 0.2
                    }
                } else {
                    0.
                };
            player.pitch = pitch;
            if verification.moving_view && verification.playing >= 60. {
                transform.translation = position;
                if verification.reported < 12
                    && verification.playing > 60.4 + verification.reported as f32 * 0.5
                {
                    commands
                        .spawn(Screenshot::primary_window())
                        .observe(save_to_disk(
                            verification
                                .output
                                .join(format!("rotation-{}.png", verification.reported)),
                        ));
                    verification.reported += 1;
                }
            }
            let water_view = verification.survey >= definition.survey_views.len() + 5;
            // Allow exposure to settle before judging atmosphere/animation.
            if verification.playing
                > if verification.moving_view {
                    66.
                } else if verification.benchmark {
                    14.
                } else if water_view {
                    8.
                } else {
                    4.
                }
            {
                if let Ok((camera, projection)) = cameras.single_mut()
                    && let Projection::Perspective(perspective) = &*projection
                {
                    let survey = verification.survey;
                    verification.camera_views.push(serde_json::json!({
                        "view": survey, "position": camera.translation.to_array(),
                        "yaw": player.yaw, "pitch": player.pitch, "vertical_fov_degrees": perspective.fov.to_degrees(),
                        "body_position": transform.translation.to_array(), "eye_height": definition.tuning.eye_height,
                        "aspect": perspective.aspect_ratio,
                    }));
                }
                if water_view {
                    verification.water_times.push(real_time.elapsed_secs());
                }
                commands
                    .spawn(Screenshot::primary_window())
                    .observe(save_to_disk(
                        verification
                            .output
                            .join(format!("map-{}.png", verification.survey)),
                    ));
                verification.survey += 1;
                verification.playing = 0.;
                if verification.only_view {
                    verification.stage = 2;
                    next.set(GameState::GameOver);
                }
            }
        } else {
            verification.stage = 2;
            verification.playing = 0.;
            next.set(GameState::GameOver);
        }
    }
    if verification.stage == 3 && verification.playing > 0.2 {
        let passed = session.round == 1
            && surfaces.pending == 0
            && if verification.benchmark {
                (!verification.combat || verification.round_two)
                    && verification.camera_views.len()
                        == if verification.only_view {
                            1
                        } else {
                            definition.survey_views.len() + 7
                        }
            } else {
                verification.round_two && verification.moved
            };
        let receipt = serde_json::json!({"passed":passed,
            "scope": if verification.benchmark && !verification.combat { "paused render survey and restart" } else { "combat movement round advance render survey and restart" },
            "continuous_camera_motion": verification.moving_view,
            "round_two":verification.round_two, "movement":verification.moved, "restart_round":session.round,
            "ads_reached": verification.ads_seen, "recoil_observed": verification.recoil_seen,
            "source_surfaces":definition.source_surfaces, "static_instances":definition.static_model_instances,
            "survey_views":verification.camera_views.len(), "water_animation_seconds":verification.water_times,
            "camera_views":verification.camera_views, "opaque_materials": &*surfaces});
        let _ = std::fs::write(
            verification.output.join("verification.json"),
            receipt.to_string(),
        );
        info!("Game verification: {receipt}");
        commands.write_message(if passed {
            AppExit::Success
        } else {
            AppExit::error()
        });
    }
}

// Additional actual-map views target the supplied spawn-room references and a
// retained puddle. The repeated water view checks animation at a stationary camera.
fn visual_view(index: usize, definition: &GameDefinition) -> Option<(Vec3, f32, f32)> {
    if let Some(position) = definition.survey_views.get(index) {
        return Some((
            Vec3::from_array(*position) + Vec3::Y * 1.8,
            index as f32,
            0.,
        ));
    }
    let local = index - definition.survey_views.len();
    let spawn = Vec3::from_array(definition.spawn) + Vec3::new(0., 0.2, 1.);
    match local {
        0..=3 => Some((
            spawn,
            definition.yaw + local as f32 * std::f32::consts::FRAC_PI_2,
            0.05,
        )),
        4 => Some((spawn, definition.yaw, 0.7)),
        5..=6 => Some((Vec3::new(-38.7, 4.5, -13.), 0., -0.6)),
        _ => None,
    }
}

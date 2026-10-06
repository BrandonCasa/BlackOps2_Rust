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

#[derive(Resource)]
pub(crate) struct Verification {
    output: PathBuf,
    started: Instant,
    playing: f32,
    stage: usize,
    initial: Option<Vec3>,
    moved: bool,
    survey: usize,
    round_two: bool,
    reported: u32,
}
pub struct VerificationPlugin;
impl Plugin for VerificationPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup)
            .add_systems(
                PreUpdate,
                drive
                    .after(bevy::input::InputSystems)
                    .after(crate::input::collect),
            )
            .add_systems(Update, check);
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
            started: Instant::now(),
            playing: 0.,
            stage: 0,
            initial: None,
            moved: false,
            survey: 0,
            round_two: false,
            reported: 0,
        });
    }
}
pub(crate) fn drive(
    verification: Option<Res<Verification>>,
    mut input: ResMut<GameInput>,
    players: Query<(Entity, &Player, &Transform)>,
    enemies: Query<&Transform, (With<Zombie>, Without<Player>)>,
    session: Res<Session>,
    definition: Option<Res<GameDefinition>>,
    query: avian3d::prelude::SpatialQuery,
) {
    let (Some(verification), Some(definition), Ok((entity, player, transform))) =
        (verification, definition, players.single())
    else {
        return;
    };
    if verification.stage != 0 {
        input.clear();
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
        let delta = target.translation
            - (transform.translation
                + Vec3::Y * (definition.tuning.eye_height - player.height * 0.5));
        let yaw = (-delta.x).atan2(-delta.z);
        let pitch = delta.y.atan2(Vec2::new(delta.x, delta.z).length());
        let yaw_delta = (yaw - player.yaw + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU)
            - std::f32::consts::PI;
        input.look =
            Vec2::new(-yaw_delta, player.pitch - pitch) / definition.tuning.mouse_sensitivity;
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
        input.reload = session.ammo == 0;
    }
}
fn check(
    mut commands: Commands,
    time: Res<Time>,
    verification: Option<ResMut<Verification>>,
    state: Res<State<GameState>>,
    failure: Res<LoadFailure>,
    definition: Option<Res<GameDefinition>>,
    session: Res<Session>,
    mut players: Query<(Entity, &mut Player, &mut Transform), Without<Zombie>>,
    mut next: ResMut<NextState<GameState>>,
    enemies: Query<&Transform, (With<Zombie>, Without<Player>)>,
    query: avian3d::prelude::SpatialQuery,
) {
    let Some(mut verification) = verification else {
        return;
    };
    if verification.started.elapsed().as_secs() > 110
        || **state == GameState::LoadError
        || (**state == GameState::GameOver && verification.stage == 0)
    {
        error!(
            "Game verification failed: {:?}, round {}, pending {}, kills {}, {}",
            **state, session.round, session.pending, session.kills, failure.0
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
    let (Some(definition), Ok((_, mut player, mut transform))) = (definition, players.single_mut())
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
            session.ammo
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
        verification.playing = 0.;
        next.set(GameState::Paused);
        commands
            .spawn(Screenshot::primary_window())
            .observe(save_to_disk(verification.output.join("round-two.png")));
    }
    if verification.stage == 1 && verification.playing > 1. {
        if verification.survey < definition.survey_views.len() {
            transform.translation =
                Vec3::from_array(definition.survey_views[verification.survey]) + Vec3::Y * 1.8;
            player.velocity = Vec3::ZERO;
            player.yaw = verification.survey as f32;
            if verification.playing > 2. {
                commands
                    .spawn(Screenshot::primary_window())
                    .observe(save_to_disk(
                        verification
                            .output
                            .join(format!("map-{}.png", verification.survey)),
                    ));
                verification.survey += 1;
                verification.playing = 0.;
            }
        } else {
            verification.stage = 2;
            verification.playing = 0.;
            next.set(GameState::GameOver);
        }
    }
    if verification.stage == 3 && verification.playing > 0.2 {
        let passed = verification.round_two && verification.moved && session.round == 1;
        let receipt = serde_json::json!({"passed":passed, "round_two":verification.round_two, "movement":verification.moved, "restart_round":session.round, "source_surfaces":definition.source_surfaces, "static_instances":definition.static_model_instances, "survey_views":verification.survey});
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

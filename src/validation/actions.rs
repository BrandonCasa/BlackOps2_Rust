use super::{Command, Controller, Phase, Request, executed, fail, pause};
use crate::{GameState, gameplay::Session, player::Player, weapons::WeaponInventory};
use avian3d::prelude::*;
use bevy::prelude::*;
use std::time::Instant;

pub(super) fn execute(world: &mut World, request: &Request) -> Result<bool, String> {
    let state = *world.resource::<State<GameState>>().get();
    if matches!(
        request.action,
        Command::SpawnChoice { .. }
            | Command::RobotChoice { .. }
            | Command::WeatherChoice { .. }
            | Command::PowerupChoice { .. }
    ) && !matches!(state, GameState::Playing | GameState::Paused)
    {
        return Err("Opportunity selection requires a playing or paused match".into());
    }
    let key = (request.session, request.id);
    match &request.action {
        Command::Start => {
            {
                let mut c = world.resource_mut::<Controller>();
                c.session = Some(request.session);
                c.heartbeat = Instant::now();
                if matches!(state, GameState::StartingSession | GameState::Playing) {
                    c.receipts.get_mut(&key).unwrap().resulting_session = Some(request.session);
                    return Ok(false);
                }
            }
            if state == GameState::Loading {
                return Ok(false);
            }
            if !matches!(state, GameState::MainMenu | GameState::GameOver) {
                return Err("Start requires loading, a fresh match, the menu or game over".into());
            }
            world.write_message(crate::frontend::FrontendAction::SelectDifficulty(
                crate::frontend::Difficulty::Original,
            ));
            world.write_message(if state == GameState::GameOver {
                crate::frontend::FrontendAction::Restart
            } else {
                crate::frontend::FrontendAction::StartSolo
            });
            Ok(false)
        }
        Command::Pause => {
            pause(world);
            Ok(true)
        }
        Command::Resume => {
            if world.resource::<Controller>().aborted {
                return Err("An aborted run cannot resume".into());
            }
            if state == GameState::Ending && world.resource::<Controller>().paused {
                world
                    .resource_mut::<super::cinematic::Playback>()
                    .pause(false);
                let mut c = world.resource_mut::<Controller>();
                c.paused = false;
                c.failure = None;
                c.heartbeat = Instant::now();
                world.resource_mut::<Time<Virtual>>().unpause();
                return Ok(true);
            }
            if state != GameState::Paused {
                return Err("Resume requires a paused match".into());
            }
            let saved = world.resource_mut::<Controller>().suspended.take();
            if let Some(saved) = saved {
                crate::input::resume_script(world, saved);
            }
            {
                let mut c = world.resource_mut::<Controller>();
                c.paused = false;
                c.failure = None;
                c.heartbeat = Instant::now();
            }
            world
                .resource_mut::<NextState<GameState>>()
                .set(GameState::Playing);
            Ok(true)
        }
        Command::Step => {
            if state != GameState::Paused
                || world.resource::<Controller>().failure.is_some()
                || world.resource::<Controller>().aborted
            {
                return Err("Step requires a healthy paused gameplay session".into());
            }
            let saved = world.resource_mut::<Controller>().suspended.take();
            if let Some(saved) = saved {
                crate::input::resume_script(world, saved);
            }
            {
                let mut c = world.resource_mut::<Controller>();
                c.paused = false;
                c.step = Some(key);
                c.step_frame = true;
            }
            world
                .resource_mut::<NextState<GameState>>()
                .set(GameState::Playing);
            Ok(false)
        }
        Command::Cancel { command } => {
            let target = (request.session, *command);
            let Some(receipt) = world.resource::<Controller>().receipts.get(&target) else {
                return Err("Unknown command to cancel".into());
            };
            if !matches!(receipt.phase, Phase::Accepted | Phase::Executed)
                || !matches!(receipt.request.action, Command::Input { .. })
            {
                return Err("Only pending input can be cancelled".into());
            }
            let active = world
                .resource::<Controller>()
                .active
                .as_ref()
                .is_some_and(|(r, _)| r.id == *command);
            {
                let mut c = world.resource_mut::<Controller>();
                c.queue.retain(|r| r.id != *command);
                if active {
                    c.active = None;
                }
                c.completed.retain(|k| *k != target);
                c.receipts.get_mut(&target).unwrap().phase = Phase::Cancelled;
            }
            if active {
                crate::input::suspend_script(world);
            }
            Ok(true)
        }
        Command::Abort => {
            pause(world);
            super::cancel_opportunities(world);
            let mut c = world.resource_mut::<Controller>();
            c.aborted = true;
            c.controls.clear();
            c.queue.clear();
            c.active = None;
            c.completed.clear();
            c.label = "ABORTED: scene retained for inspection".into();
            for receipt in c
                .receipts
                .values_mut()
                .filter(|r| r.phase == Phase::Accepted && r.request.id != request.id)
            {
                receipt.phase = Phase::Cancelled;
            }
            Ok(true)
        }
        Command::Finish => {
            let c = world.resource::<Controller>();
            if c.failure.is_some() || c.aborted || c.active.is_some() || !c.queue.is_empty() {
                return Err("Finish requires a healthy run with no queued input".into());
            }
            pause(world);
            super::cancel_opportunities(world);
            let mut c = world.resource_mut::<Controller>();
            c.session = None;
            c.suspended = None;
            c.god = false;
            c.label = "CHECKPOINT COMPLETE: scene paused for inspection".into();
            Ok(true)
        }
        Command::Fail { reason } => {
            fail(world, reason.clone());
            Ok(false)
        }
        Command::Assist {
            god,
            points,
            ammo,
            doors,
        } => {
            require_playing(state)?;
            world.resource_mut::<Controller>().god = *god;
            world.resource_mut::<Session>().points = *points;
            if *ammo {
                world.resource_scope(|world, game: Mut<crate::assets::GameDefinition>| {
                    for mut inventory in world
                        .query_filtered::<&mut WeaponInventory, With<Player>>()
                        .iter_mut(world)
                    {
                        inventory.refill(&game.content.weapons);
                    }
                });
            }
            if *doors {
                let connections: Vec<_> = world
                    .query::<&crate::interactions::Usable>()
                    .iter(world)
                    .filter_map(|u| match &u.source.action {
                        crate::origins::content::InteractionAction::Door { connection, .. } => {
                            Some(connection.clone())
                        }
                        _ => None,
                    })
                    .collect();
                let flags =
                    world.resource_scope(|world, game: Mut<crate::assets::GameDefinition>| {
                        let mut access = world.resource_mut::<crate::access::ZoneAccess>();
                        let mut activated = Vec::new();
                        for connection in connections {
                            if access.unlock(&game, &connection).is_ok() {
                                for link in game
                                    .access
                                    .flag_links
                                    .iter()
                                    .filter(|link| link.connections.contains(&connection))
                                {
                                    access.set_flag(&game.content, &game.access, &link.flag);
                                    activated.push(link.flag.clone());
                                }
                            }
                        }
                        activated
                    });
                for flag in flags {
                    world.write_message(crate::access::ZoneFlagSet(flag));
                }
            }
            Ok(true)
        }
        Command::Teleport { feet } => {
            require_playing(state)?;
            world
                .run_system_cached_with(teleport, Vec3::from_array(*feet))
                .map_err(|e| e.to_string())??;
            Ok(true)
        }
        Command::SpawnChoice { candidate } => {
            world
                .resource_mut::<crate::navigation::Navigation>()
                .select_spawn(*candidate)?;
            Ok(true)
        }
        Command::RobotChoice { robot, foot } => {
            let session = world.resource::<Session>();
            let quest = world.resource::<crate::quests::MainQuest>();
            let robots = world.resource::<crate::robots::Robots>();
            let triple = session.round % 4 == 0 && robots.triple_round != session.round
                || quest.enabled && quest.stage == crate::quests::QuestStage::BreakSeal;
            let nml = world
                .resource::<crate::access::ZoneAccess>()
                .open
                .iter()
                .any(|id| id.0.contains("activate_zone_nml"));
            world
                .resource_mut::<crate::robots::Robots>()
                .select(*robot, *foot, nml, triple)?;
            Ok(true)
        }
        Command::WeatherChoice { kind } => {
            let round = world.resource::<Session>().round;
            world
                .resource_mut::<crate::environment::weather::Weather>()
                .select(round, *kind)?;
            Ok(true)
        }
        Command::PowerupChoice { id } => {
            world.resource_scope(|world, game: Mut<crate::assets::GameDefinition>| {
                world
                    .resource_mut::<crate::powerups::Drops>()
                    .select(id, &game)
            })?;
            Ok(true)
        }
        Command::Capture { name } => {
            let path = world
                .resource::<Controller>()
                .report
                .join(format!("{name}.png"));
            if path.exists() {
                return Err("Capture path already exists".into());
            }
            use bevy::render::view::screenshot::{Screenshot, ScreenshotCaptured, save_to_disk};
            let save_path = path.clone();
            world.spawn(Screenshot::primary_window()).observe(save_to_disk(save_path)).observe(
                move |event:On<ScreenshotCaptured>,mut commands:Commands| {
                    let size=event.image.size(); let path=path.clone();
                    commands.queue(move |world:&mut World| {
                        if let Some(receipt)=world.resource_mut::<Controller>().receipts.get_mut(&key) {
                            receipt.after=serde_json::json!({"capture":path,"width":size.x,"height":size.y});
                        }
                        executed(world,key);
                    });
                });
            Ok(false)
        }
        Command::Quit => {
            world.write_message(AppExit::Success);
            Ok(true)
        }
        Command::Input { .. } => unreachable!("Input is executed only by FixedPreUpdate"),
    }
}
fn teleport(
    In(feet): In<Vec3>,
    spatial: SpatialQuery,
    access: Res<crate::access::ZoneAccess>,
    volumes: crate::environment::hazards::BoundarySites,
    mut players: Query<(
        Entity,
        &mut Player,
        &mut Transform,
        &Collider,
        &mut crate::movement::Mobility,
    )>,
    mut commands: Commands,
) -> Result<(), String> {
    let (entity, mut player, mut pose, shape, mut mobility) =
        players.single_mut().map_err(|_| "Expected one player")?;
    let center = feet + Vec3::Y * player.height * 0.5;
    if !crate::environment::hazards::playable(
        &spatial,
        shape,
        &Transform::from_translation(center),
        &access,
        &volumes,
    ) {
        return Err("Teleport destination is outside a reachable playable volume".into());
    }
    let filter = crate::environment::player_solids([entity]);
    if spatial
        .cast_ray(feet + Vec3::Y * 0.2, Dir3::NEG_Y, 0.8, true, &filter)
        .is_none()
    {
        return Err("Teleport has no nearby supporting floor".into());
    }
    if !spatial
        .shape_intersections(shape, center + Vec3::Y * 0.03, Quat::IDENTITY, &filter)
        .is_empty()
    {
        return Err("Teleport capsule intersects solid geometry".into());
    }
    *mobility = crate::movement::Mobility::new(player.eye_height);
    mobility.fall_origin = feet.y;
    crate::portals::relocate(
        &mut commands,
        entity,
        &mut player,
        &mut pose,
        &mut mobility,
        feet,
    );
    Ok(())
}
fn require_playing(state: GameState) -> Result<(), String> {
    if state == GameState::Playing {
        Ok(())
    } else {
        Err("Action requires a playing match".into())
    }
}

pub(super) fn look_delta(world: &mut World, target: Vec3) -> Vec2 {
    let Ok((player, pose, inventory, feedback)) = world
        .query::<(
            &Player,
            &Transform,
            &WeaponInventory,
            &crate::movement::MotionFeedback,
        )>()
        .single(world)
    else {
        return Vec2::ZERO;
    };
    let eye = pose.translation + Vec3::Y * (player.eye_height - player.height * 0.5);
    let direction = (target - eye).normalize_or_zero();
    let yaw = (-direction.x).atan2(-direction.z);
    let pitch = direction.y.asin().clamp(-1.5, 1.5);
    let view_yaw = player.yaw + inventory.recoil.x + feedback.view_angles.y.to_radians();
    let view_pitch = player.pitch + inventory.recoil.y - feedback.view_angles.x.to_radians();
    let dy = (yaw - view_yaw + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU)
        - std::f32::consts::PI;
    let dp = pitch - view_pitch;
    let game = world.resource::<crate::assets::GameDefinition>();
    let settings = world.resource::<crate::input::ControlSettings>();
    let hip = game.tuning.mouse_sensitivity * settings.sensitivity;
    let ads = game.tuning.ads_mouse_sensitivity * settings.ads_sensitivity;
    let sensitivity = hip + (ads - hip) * inventory.ads;
    let maximum = std::f32::consts::FRAC_PI_2 / 60.;
    Vec2::new(
        -dy.clamp(-maximum, maximum) / sensitivity,
        -dp.clamp(-maximum, maximum) / sensitivity * if settings.invert_mouse_y { -1. } else { 1. },
    )
}

pub(super) fn protect_player(
    controller: Res<Controller>,
    players: Query<Entity, With<Player>>,
    mut queue: ResMut<crate::damage::DamageQueue>,
    mut commands: Commands,
) {
    if controller.owns_input()
        && queue.0.iter().any(|impact| {
            players.contains(impact.target)
                && matches!(impact.source, crate::damage::ImpactSource::OutOfBounds)
        })
    {
        commands.queue(|world: &mut World| {
            fail(world, "Player entered an out-of-bounds volume".into())
        });
    }
    if controller.god && controller.owns_input() {
        queue.0.retain(|impact| !players.contains(impact.target));
    }
}

//! Bounded actual-window exercise of the production window and shared use path.
use super::*;
use crate::{
    barriers::{Barriers, BoardState},
    interactions::InteractionFocus,
    life::Vitality,
    movement::Mobility,
};
use avian3d::prelude::*;
#[derive(Resource, Default)]
struct Coverage {
    phase: u8,
    age: f32,
    actor: Option<Entity>,
    owner: Option<Entity>,
    center: Vec3,
    aim: Vec3,
    spawn: Vec3,
    window: Option<crate::origins::ContentId>,
    rows: Vec<serde_json::Value>,
    captured: bool,
    ingress_attempt: u8,
    jump_peak: f32,
    blocked_capture: bool,
    slats: std::collections::BTreeSet<usize>,
}
pub fn install(app: &mut App) {
    app.init_resource::<Coverage>().add_systems(
        FixedUpdate,
        drive
            .in_set(crate::gameplay::GameStep::Movement)
            .before(crate::player::move_player),
    );
}
fn actor(commands: &mut Commands, game: &GameDefinition, position: Vec3) -> Entity {
    let e = game
        .content
        .enemies
        .iter()
        .find(|e| e.family == "german")
        .unwrap();
    commands
        .spawn((
            Zombie::new(position, e.id.clone(), crate::rounds::Gait::Walk, e.speed),
            crate::player::capsule(0.3, 1.7),
            Vitality::ordinary_actor(150, 150).unwrap(),
            crate::rounds::PopulationKind::Ordinary,
            crate::gameplay::SessionEntity,
            Transform::from_translation(position),
            Visibility::Inherited,
        ))
        .id()
}
fn drive(
    mut commands: Commands,
    v: Option<Res<Verification>>,
    game: Option<Res<GameDefinition>>,
    time: Res<Time<Fixed>>,
    state: Res<State<GameState>>,
    mut c: ResMut<Coverage>,
    mut input: ResMut<GameInput>,
    mut session: ResMut<Session>,
    query: SpatialQuery,
    mut players: Query<
        (
            Entity,
            &mut Player,
            &mut Transform,
            &Vitality,
            &mut Mobility,
        ),
        Without<Zombie>,
    >,
    enemies: Query<(Entity, &Transform, &Zombie), Without<Player>>,
    barriers: Res<Barriers>,
    focus: Res<InteractionFocus>,
    mut next: ResMut<NextState<GameState>>,
) {
    let (Some(v), Some(game)) = (v, game) else {
        return;
    };
    if !v.barriers {
        return;
    }
    input.clear();
    c.age += time.delta_secs();
    if v.started.elapsed().as_secs() > 180 {
        c.rows
            .push(serde_json::json!({"case":"timeout","passed":false}));
        finish(&mut commands, &v, &c, &barriers);
        return;
    }
    let Ok((owner, mut p, mut pose, health, mut mobility)) = players.single_mut() else {
        return;
    };
    if c.phase < 12 || c.phase == 20 {
        session.pending = 6;
        session.spawn_timer = Timer::from_seconds(1000., TimerMode::Once);
        for (e, _, _) in &enemies {
            if c.actor != Some(e) {
                commands.entity(e).despawn();
            }
        }
    }
    if c.phase > 0 && c.phase < 12 {
        pose.translation = c.center;
        p.velocity = Vec3::ZERO;
        mobility.airborne = false;
        mobility.peak = c.center.y - p.height * 0.5;
        mobility.fall_speed = 0.;
        let eye = p.eye_height;
        super::semtex::aim(&mut p, &pose, c.aim, eye);
    }
    let mut advance = None;
    let boards = c.window.as_ref().and_then(|id| barriers.windows.get(id));
    match c.phase {
        0 if c.age > 4. => {
            let d = game
                .content
                .barriers
                .iter()
                .find(|d| d.id.0 == "barrier.pf2237_auto1")
                .unwrap();
            let placement = game
                .content
                .placements
                .iter()
                .find(|p| p.id == d.placement)
                .unwrap();
            let origin = Vec3::from_array(placement.position);
            let end = Vec3::from_array(
                game.content
                    .navigation_nodes
                    .iter()
                    .find(|n| n.id == d.exit)
                    .unwrap()
                    .position,
            );
            let inward = (end - origin).normalize();
            let Some(center) = super::semtex::standing(
                &query,
                owner,
                origin + inward * 3.8,
                p.height,
                game.tuning.radius,
            ) else {
                return;
            };
            let Some(spawn) =
                super::semtex::standing(&query, owner, origin - inward * 0.6, 1.7, 0.3)
            else {
                return;
            };
            c.center = center;
            c.spawn = spawn;
            c.aim = origin + Vec3::Y * 0.7;
            c.owner = Some(owner);
            c.window = Some(d.id.clone());
            c.rows.push(serde_json::json!({"case":"authored_windows","passed":barriers.windows.len()==12 && barriers.windows.values().all(|b|b.len()==6 && b.iter().all(|b|*b==BoardState::Closed)) && barriers.animated_boards==72,"windows":barriers.windows.len(),"animated_boards":barriers.animated_boards}));
            c.actor = Some(actor(&mut commands, &game, spawn));
            advance = Some(1);
        }
        1 => {
            if let Some(boards) = boards {
                for (index, board) in boards.iter().enumerate() {
                    if matches!(board,BoardState::Pulling(_,age)|BoardState::Opening(age) if *age>0.04 && *age<0.3)
                        && c.slats.insert(index)
                    {
                        commands
                            .spawn(Screenshot::primary_window())
                            .observe(save_to_disk(
                                v.output
                                    .join(format!("native-board-pull-{}.png", index + 1)),
                            ));
                    }
                }
            }
            if !c.captured
                && enemies.iter().any(|(e, _, z)| {
                    Some(e) == c.actor
                        && z.action == crate::origins::EnemyAnimation::Tear
                        && z.elapsed > 0.2
                })
            {
                commands
                    .spawn(Screenshot::primary_window())
                    .observe(save_to_disk(v.output.join("native-board-tear.png")));
                c.captured = true;
            }
            if barriers.destroyed == 6
                && barriers.entries > 0
                && enemies.iter().any(|(e, _, z)| {
                    Some(e) == c.actor
                        && matches!(
                            z.action,
                            crate::origins::EnemyAnimation::Landing
                                | crate::origins::EnemyAnimation::Walk
                                | crate::origins::EnemyAnimation::Attack
                        )
                })
            {
                let captured = c.captured;
                let individual_slats = c.slats.len();
                c.rows.push(serde_json::json!({"case":"native_tearing_and_entry","passed":boards.is_some_and(|b|b.iter().all(|b|*b==BoardState::Missing)) && captured && individual_slats==6,"destroyed":barriers.destroyed,"entries":barriers.entries,"sounds":barriers.playing_sounds,"individual_slats":individual_slats}));
                if let Some(e) = c.actor.take() {
                    commands.entity(e).despawn();
                }
                commands
                    .spawn(Screenshot::primary_window())
                    .observe(save_to_disk(v.output.join("open-window.png")));
                advance = Some(20);
            }
        }
        20 => {
            let origin = c.aim - Vec3::Y * 0.7;
            let inward = (c.center - origin).with_y(0.).normalize();
            let tangent = Vec3::new(-inward.z, 0., inward.x);
            let mode = c.ingress_attempt / 3;
            let lane = f32::from(c.ingress_attempt % 3) - 1.;
            if c.age <= time.delta_secs() * 1.5 {
                pose.translation = c.center + tangent * lane * 0.8;
                p.velocity = Vec3::ZERO;
                mobility.airborne = false;
                c.jump_peak = pose.translation.y;
            }
            let eye = p.eye_height;
            let aim = pose.translation - inward * 4.;
            super::semtex::aim(&mut p, &pose, aim, eye);
            if c.age > 0.4 {
                input.movement = Vec2::Y;
                input.sprint = mode == 1;
                input.crouch = mode == 2;
                input.jump = mode == 1 && (c.age % 0.8) < time.delta_secs() * 1.5;
            }
            c.jump_peak = c.jump_peak.max(pose.translation.y);
            if mode == 1 && lane == 0. && c.age > 2.3 && !c.blocked_capture {
                commands
                    .spawn(Screenshot::primary_window())
                    .observe(save_to_disk(v.output.join("player-window-blocked.png")));
                c.blocked_capture = true;
            }
            if c.age > 3. {
                let signed = (pose.translation - origin).dot(inward);
                let jumped = mode != 1 || c.jump_peak > c.center.y + 0.15;
                let attempt = c.ingress_attempt;
                let mode_name = ["walk", "sprint_jump", "crouch"][usize::from(mode)];
                c.rows.push(serde_json::json!({"case":"player_ingress_rejected","attempt":attempt,"mode":mode_name,"lane":lane,"passed":signed>2.4 && jumped,"interior_distance":signed,"jumped":jumped}));
                c.ingress_attempt += 1;
                c.age = 0.;
                if c.ingress_attempt == 9 {
                    // Keep the actual safe-side endpoint rather than warping
                    // to a ground probe beside the sandbags.
                    c.center =
                        pose.translation + Vec3::Y * (game.tuning.standing_height - p.height) * 0.5;
                    pose.translation = c.center;
                    p.velocity = Vec3::ZERO;
                    mobility.airborne = false;
                    advance = Some(2);
                }
            }
        }
        2 => {
            input.use_held = c.age > 0.4;
            if c.age > 0.8 {
                let center = c.center;
                let aim = c.aim;
                c.rows.push(serde_json::json!({"case":"partial_repair_started","passed":focus.progress>0.2 && focus.progress<0.6,"progress":focus.progress,"center":center.to_array(),"aim":aim.to_array()}));
                advance = Some(3);
            }
        }
        3 if c.age > 0.25 => {
            c.rows.push(serde_json::json!({"case":"released_partial_repair","passed":barriers.repaired==0 && focus.progress==0.,"progress":focus.progress}));
            advance = Some(4);
        }
        4 => {
            input.use_held = true;
            if barriers.repaired == 6 {
                c.rows.push(serde_json::json!({"case":"six_board_repair_cap","passed":barriers.awarded==40 && barriers.reward_counter==60 && boards.is_some_and(|b|b.iter().all(|b|*b==BoardState::Closed)),"awarded":barriers.awarded,"counter":barriers.reward_counter,"points":session.points}));
                commands
                    .spawn(Screenshot::primary_window())
                    .observe(save_to_disk(v.output.join("repaired-window.png")));
                advance = Some(5);
            }
        }
        5 => {
            input.use_held = true;
            if c.age > 2. {
                c.rows.push(serde_json::json!({"case":"full_window_stops","passed":barriers.repaired==6 && barriers.awarded==40 && focus.target.is_none()}));
                c.actor = Some(actor(&mut commands, &game, c.spawn));
                advance = Some(6);
            }
        }
        6 if boards.is_some_and(|b| b.iter().any(|b| matches!(b, BoardState::Reserved(_)))) => {
            if let Some(e) = c.actor.take() {
                commands.entity(e).despawn();
            }
            advance = Some(7);
        }
        7 if c.age > 0.2 => {
            c.rows.push(serde_json::json!({"case":"retired_tear_owner","passed":boards.is_some_and(|b|b.iter().all(|b|*b==BoardState::Closed)) && barriers.destroyed==6}));
            c.actor = Some(actor(&mut commands, &game, c.spawn));
            advance = Some(8);
        }
        8 if barriers.destroyed == 12 && barriers.entries > 1 => {
            if let Some(e) = c.actor.take() {
                commands.entity(e).despawn();
            }
            advance = Some(9);
        }
        9 => {
            input.use_held = true;
            if barriers.repaired == 12 {
                c.rows.push(serde_json::json!({"case":"repairs_beyond_reward_cap","passed":barriers.awarded==40 && barriers.reward_counter==120}));
                session.round = 2;
                c.actor = Some(actor(&mut commands, &game, c.spawn));
                advance = Some(10);
            }
        }
        10 if barriers.destroyed == 18 && barriers.entries > 2 => {
            if let Some(e) = c.actor.take() {
                commands.entity(e).despawn();
            }
            advance = Some(11);
        }
        11 => {
            input.use_held = true;
            if barriers.repaired == 18 && barriers.playing_sounds == barriers.sounds {
                c.rows.push(serde_json::json!({"case":"new_round_reward_reset","passed":barriers.awarded==100 && barriers.reward_counter==60 && barriers.playing_sounds==barriers.sounds,"awarded":barriers.awarded,"counter":barriers.reward_counter,"sounds":barriers.sounds,"playing_sounds":barriers.playing_sounds}));
                next.set(GameState::GameOver);
                advance = Some(12);
            }
        }
        12 if **state == GameState::GameOver => {
            commands.write_message(crate::input::RestartGame);
            advance = Some(13);
        }
        13 if **state == GameState::Playing && c.age > 0.6 => {
            let old = c.owner;
            c.rows.push(serde_json::json!({"case":"fresh_session","passed":old!=Some(owner) && barriers.repaired==0 && barriers.destroyed==0 && barriers.awarded==0 && barriers.reward_counter==0 && barriers.windows.len()==12 && barriers.windows.values().all(|b|b.iter().all(|b|*b==BoardState::Closed)) && health.current()==Some(100) && session.points==500}));
            finish(&mut commands, &v, &c, &barriers);
            advance = Some(14);
        }
        _ => {}
    }
    if c.age > 35. && c.phase < 12 {
        let phase = c.phase;
        c.rows.push(serde_json::json!({"case":"phase_timeout","passed":false,"phase":phase,"focus":focus.prompt,"rejection":format!("{:?}",focus.rejection),"boards":format!("{:?}",boards),"actors":enemies.iter().map(|(_,t,z)|serde_json::json!({"position":t.translation.to_array(),"action":format!("{:?}",z.action),"elapsed":z.elapsed})).collect::<Vec<_>>()}));
        finish(&mut commands, &v, &c, &barriers);
        advance = Some(14);
    }
    if let Some(phase) = advance {
        c.phase = phase;
        c.age = 0.;
        let _ = std::fs::write(
            v.output.join("barriers-progress.json"),
            serde_json::json!({"phase":phase,"rows":c.rows}).to_string(),
        );
    }
}
fn finish(commands: &mut Commands, v: &Verification, c: &Coverage, b: &Barriers) {
    let passed = c.rows.iter().all(|r| r["passed"] == true);
    let _=std::fs::write(v.output.join("barriers.json"),serde_json::json!({"passed":passed,"rows":c.rows,"repaired":b.repaired,"destroyed":b.destroyed,"entries":b.entries}).to_string());
    commands.write_message(if passed {
        AppExit::Success
    } else {
        AppExit::error()
    });
}

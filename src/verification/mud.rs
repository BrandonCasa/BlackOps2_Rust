//! Authored mud, rendered motion feedback and native clearance in the game window.
use super::*;
use crate::{
    environment::{Footstep, FootstepSurface, TerrainMotion},
    life::Vitality,
    movement::{Mobility, MotionFeedback},
    solo_revive::PerkInventory,
    weapons::WeaponInventory,
};
use avian3d::prelude::*;
#[derive(Resource, Default)]
struct Coverage {
    phase: u8,
    age: f32,
    outside: Vec3,
    lane: Vec3,
    mud: Vec3,
    blocked: Option<Vec3>,
    owner: Option<Entity>,
    first: f32,
    walk: f32,
    sprint: f32,
    sway: f32,
    mud_steps: u32,
    rows: Vec<serde_json::Value>,
}
fn warp(p: &mut Player, pose: &mut Transform, m: &mut Mobility, center: Vec3) {
    pose.translation = center;
    p.velocity = Vec3::ZERO;
    m.airborne = false;
    m.dive = None;
    m.recovery = 0.;
    m.fall_speed = 0.;
    m.peak = center.y - p.height * 0.5;
}
pub fn install(app: &mut App) {
    app.init_resource::<Coverage>().add_systems(
        FixedUpdate,
        drive
            .in_set(crate::gameplay::GameStep::Movement)
            .before(crate::environment::slow),
    );
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
    slide: MoveAndSlide,
    mut players: Query<(
        Entity,
        &mut Player,
        &mut Transform,
        &Vitality,
        &mut PerkInventory,
        &mut WeaponInventory,
        &TerrainMotion,
        &MotionFeedback,
        &mut Mobility,
        Option<&crate::perks::PerkDrinking>,
    )>,
    enemies: Query<Entity, With<Zombie>>,
    mut steps: MessageReader<Footstep>,
    evidence: Res<crate::movement::MovementEvidence>,
) {
    let (
        Some(v),
        Some(game),
        Ok((
            owner,
            mut p,
            mut pose,
            health,
            mut perks,
            mut inv,
            terrain,
            motion,
            mut mobility,
            drink,
        )),
    ) = (v, game, players.single_mut())
    else {
        return;
    };
    if !v.mud {
        return;
    }
    for step in steps.read() {
        if step.owner == owner && step.surface == FootstepSurface::Mud && step.position.is_finite()
        {
            c.mud_steps += 1;
        }
    }
    input.clear();
    c.age += time.delta_secs();
    if v.started.elapsed().as_secs() > 120 {
        c.phase = 99;
    }
    let _=std::fs::write(v.output.join("mud-progress.json"),serde_json::json!({"phase":c.phase,"age":c.age,"position":pose.translation.to_array(),"health":health.current(),"mud":terrain.muddy,"scale":terrain.scale,"bob":motion.bob.to_array(),"sway":motion.sway.to_array(),"mud_steps":c.mud_steps,"rows":c.rows,"events":evidence.events}).to_string());
    let mut next = None;
    match c.phase {
        0 if **state == GameState::Playing && c.age > 3. => {
            c.owner = Some(owner);
            c.outside = super::semtex::standing(
                &query,
                owner,
                pose.translation,
                p.height,
                game.tuning.radius,
            )
            .unwrap_or(pose.translation);
            let mut lane = None;
            let candidates = game.waypoints.iter().copied().chain(
                game.environment
                    .mud
                    .iter()
                    .flat_map(|m| m.volumes.iter().map(|v| v.position)),
            );
            for point in candidates {
                if let Some(center) = super::semtex::standing(
                    &query,
                    owner,
                    Vec3::from_array(point),
                    p.height,
                    game.tuning.radius,
                ) {
                    let filter = crate::environment::solids([owner]);
                    let clear = query
                        .cast_shape(
                            &crate::player::capsule(game.tuning.radius, p.height),
                            center,
                            Quat::IDENTITY,
                            Dir3::NEG_Z,
                            &ShapeCastConfig::from_max_distance(4.),
                            &filter,
                        )
                        .is_none();
                    let flat = [0., 1., 2., 3.].iter().all(|d| {
                        query
                            .cast_ray(
                                center - Vec3::Z * (*d) + Vec3::Y,
                                Dir3::NEG_Y,
                                3.,
                                false,
                                &filter,
                            )
                            .is_some_and(|h| {
                                h.normal.y > 0.9 && (h.distance - 1. - p.height * 0.5).abs() < 0.7
                            })
                    });
                    let overhead = query
                        .cast_shape(
                            &crate::player::capsule(game.tuning.radius, p.height),
                            center,
                            Quat::IDENTITY,
                            Dir3::Y,
                            &ShapeCastConfig::from_max_distance(10.),
                            &filter,
                        )
                        .is_none();
                    if clear && flat && overhead {
                        lane = Some(center);
                        break;
                    }
                }
            }
            if let Some(lane) = lane {
                c.lane = lane;
            } else {
                c.phase = 99;
                return;
            }
            for e in &enemies {
                commands.entity(e).despawn();
            }
            session.pending = 0;
            session.intermission = Some(Timer::from_seconds(300., TimerMode::Once));
            let mut point = None;
            if let Some(mud) = &game.environment.mud {
                for volume in &mud.volumes {
                    if let Some(center) = super::semtex::standing(
                        &query,
                        owner,
                        Vec3::from_array(volume.position),
                        p.height,
                        game.tuning.radius,
                    ) {
                        if !query
                            .shape_intersections(
                                &crate::player::capsule(game.tuning.radius, p.height),
                                center,
                                Quat::IDENTITY,
                                &SpatialQueryFilter::from_mask(2u32),
                            )
                            .is_empty()
                        {
                            point = Some(center);
                            break;
                        }
                    }
                }
            }
            if let Some(point) = point {
                c.mud = point;
                warp(&mut p, &mut pose, &mut mobility, point);
                next = Some(1);
            } else {
                next = Some(99);
            }
        }
        1 => {
            if c.age > 0.11 && c.first == 0. {
                c.first = terrain.scale;
            }
            if c.age > 0.55 {
                let first = c.first;
                c.rows.push(serde_json::json!({"case":"gradual_mud","passed":terrain.muddy && first<1. && first>0.6 && (terrain.scale-0.6).abs()<0.001,"initial":first,"final":terrain.scale}));
                warp(&mut p, &mut pose, &mut mobility, c.outside);
                next = Some(2);
            }
        }
        2 if c.age > 0.2 => {
            c.rows.push(
                serde_json::json!({"case":"mud_exit","passed":!terrain.muddy && terrain.scale==1.}),
            );
            inv.cancel_action();
            let perk = game
                .content
                .perks
                .iter()
                .find(|p| p.id.0 == "perk.stamin_up")
                .unwrap();
            commands
                .entity(owner)
                .insert(crate::perks::PerkDrinking::new(perk, &inv, &game).unwrap());
            next = Some(3);
        }
        3 if c.age > 4.5 && drink.is_none() => {
            warp(&mut p, &mut pose, &mut mobility, c.mud);
            c.first = 0.;
            next = Some(4);
        }
        4 => {
            if c.age > 0.11 && c.first == 0. {
                c.first = terrain.scale;
            }
            if c.age > 0.75 {
                let first = c.first;
                c.rows.push(serde_json::json!({"case":"stamin_mud","passed":terrain.muddy && first>0.7 && first<1. && (terrain.scale-0.7).abs()<0.001,"initial":first,"final":terrain.scale}));
                next = Some(5);
            }
        }
        5 => {
            input.movement = Vec2::Y;
            if c.age > 1.5 {
                let count = c.mud_steps;
                c.rows.push(
                    serde_json::json!({"case":"mud_footsteps","passed":count>0,"steps":count}),
                );
                warp(&mut p, &mut pose, &mut mobility, c.lane);
                perks.owned.clear();
                p.yaw = 0.;
                next = Some(6);
            }
        }
        6 => {
            input.movement = Vec2::Y;
            c.walk = c.walk.max(motion.bob.y.abs());
            if c.age > 0.6 {
                let bob = c.walk;
                c.rows.push(serde_json::json!({"case":"walking_bob","passed":bob>0.002 && bob<=game.tuning.mobility.walk_bob[1]+0.001,"peak":bob}));
                commands
                    .spawn(Screenshot::primary_window())
                    .observe(save_to_disk(v.output.join("walking.png")));
                warp(&mut p, &mut pose, &mut mobility, c.lane);
                next = Some(7);
            }
        }
        7 => {
            input.movement = Vec2::Y;
            input.sprint = true;
            c.sprint = c.sprint.max(motion.bob.y.abs());
            if c.age > 0.5 {
                let bob = c.sprint;
                c.rows.push(serde_json::json!({"case":"sprinting_bob","passed":bob>0.002 && bob<=game.tuning.mobility.sprint_bob[1]+0.001,"peak":bob}));
                commands
                    .spawn(Screenshot::primary_window())
                    .observe(save_to_disk(v.output.join("sprinting.png")));
                next = Some(8);
            }
        }
        8 => {
            p.yaw += 0.12;
            c.sway = c.sway.max(motion.sway.length());
            if c.age > 0.35 {
                let sway = c.sway;
                c.rows.push(serde_json::json!({"case":"weapon_sway","passed":sway>0.005 && sway<=game.tuning.mobility.sway_limit*1.5,"peak":sway}));
                commands
                    .spawn(Screenshot::primary_window())
                    .observe(save_to_disk(v.output.join("sway.png")));
                next = Some(9);
            }
        }
        9 if c.age > 0.7 => {
            c.rows.push(
                serde_json::json!({"case":"sway_recovery","passed":motion.sway.length()<0.003}),
            );
            warp(&mut p, &mut pose, &mut mobility, c.outside);
            input.prone = true;
            next = Some(10);
        }
        10 if c.age > 0.4 => {
            let mut blocked = None;
            'search: for x in -12..=12 {
                for z in -12..=12 {
                    let probe = c.outside + Vec3::new(x as f32 * 0.5, 0., z as f32 * 0.5);
                    if let Some(center) = super::semtex::standing(
                        &query,
                        owner,
                        probe,
                        game.tuning.mobility.prone_height,
                        game.tuning.radius,
                    ) {
                        let expanded = center
                            + Vec3::Y
                                * (game.tuning.standing_height - game.tuning.mobility.prone_height)
                                * 0.5;
                        let mut penetrates = false;
                        slide.intersections(
                            &crate::player::capsule(
                                game.tuning.radius,
                                game.tuning.standing_height,
                            ),
                            expanded,
                            Quat::IDENTITY,
                            0.,
                            &crate::environment::solids([owner]),
                            |_, point, _| {
                                penetrates |= point.penetration > 0.03;
                                true
                            },
                        );
                        if penetrates {
                            blocked = Some(center);
                            break 'search;
                        }
                    }
                }
            }
            if let Some(center) = blocked {
                c.blocked = Some(center);
                warp(&mut p, &mut pose, &mut mobility, center);
                input.prone = true;
                next = Some(11);
            } else {
                next = Some(99);
            }
        }
        11 if c.age > 0.3 => {
            c.rows.push(serde_json::json!({"case":"blocked_stance","passed":p.prone && (p.height-game.tuning.mobility.prone_height).abs()<0.001 && evidence.events.iter().any(|e|e["event"]=="stance_blocked")}));
            let center = c.outside - Vec3::Y * (game.tuning.standing_height - p.height) * 0.5;
            warp(&mut p, &mut pose, &mut mobility, center);
            input.prone = true;
            next = Some(12);
        }
        12 if c.age > 0.4 => {
            c.rows
                .push(serde_json::json!({"case":"clear_stance","passed":!p.prone && !p.crouched}));
            warp(&mut p, &mut pose, &mut mobility, c.lane + Vec3::Y * 9.);
            next = Some(13);
        }
        13 if **state == GameState::GameOver => {
            commands.write_message(crate::input::RestartGame);
            next = Some(14);
        }
        14 if **state == GameState::Playing && c.age > 0.8 => {
            let old = c.owner;
            c.rows.push(serde_json::json!({"case":"restart","passed":Some(owner)!=old && terrain.scale==1. && !terrain.muddy && motion.sway.length()<0.001 && !mobility.busy() && health.current()==Some(100)}));
            let passed = c.rows.iter().all(|r| r["passed"] == true);
            let _=std::fs::write(v.output.join("mud.json"),serde_json::json!({"passed":passed,"rows":c.rows,"native_volumes":game.environment.mud.as_ref().map(|m|m.volumes.len()),"events":evidence.events}).to_string());
            commands.write_message(if passed {
                AppExit::Success
            } else {
                AppExit::error()
            });
            next = Some(15);
        }
        99 => {
            commands.write_message(AppExit::error());
            next = Some(15);
        }
        _ => {}
    }
    if let Some(phase) = next {
        c.phase = phase;
        c.age = 0.;
    }
}

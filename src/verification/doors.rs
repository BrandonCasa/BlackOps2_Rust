//! Actual-window purchases across every authored blocker group, then session reset.
use super::*;
use crate::{
    access::{DoorBlocker, DoorEvidence, DoorProp, ZoneAccess},
    interactions::{InteractionFocus, InteractionRejection},
    life::Vitality,
    movement::Mobility,
};
use avian3d::prelude::*;
#[derive(Resource, Default)]
struct Coverage {
    phase: u8,
    index: usize,
    age: f32,
    owner: Option<Entity>,
    center: Vec3,
    aim: Vec3,
    before: usize,
    initial_links: usize,
    initial_revision: u32,
    rows: Vec<serde_json::Value>,
}
pub fn install(app: &mut App) {
    app.init_resource::<Coverage>().add_systems(
        FixedUpdate,
        drive
            .in_set(crate::gameplay::GameStep::Movement)
            .before(crate::player::move_player),
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
    enemies: Query<(Entity, &Transform), With<Zombie>>,
    blockers: Query<(Entity, &DoorBlocker)>,
    props: Query<&DoorProp>,
    access: Res<ZoneAccess>,
    focus: Res<InteractionFocus>,
    inspection: (
        Res<DoorEvidence>,
        ResMut<crate::navigation::Navigation>,
        Res<crate::access::CurrentZone>,
        Res<crate::enemies::EnemyEvidence>,
    ),
) {
    let (evidence, mut nav, current_zone, enemy_evidence) = inspection;
    let (Some(v), Some(game), Ok((owner, mut p, mut pose, health, mut mobility))) =
        (v, game, players.single_mut())
    else {
        return;
    };
    if !v.doors {
        return;
    }
    input.clear();
    c.age += time.delta_secs();
    for (e, body) in &enemies {
        if !v.navigation || c.phase >= 6 || body.translation.distance(pose.translation) < 3. {
            commands.entity(e).despawn();
        }
    }
    let mut next = None;
    if c.phase < 6 && c.phase > 0 {
        pose.translation = c.center;
        p.velocity = Vec3::ZERO;
        mobility.airborne = false;
        mobility.peak = c.center.y - p.height * 0.5;
        mobility.fall_speed = 0.;
        let eye = p.eye_height;
        super::semtex::aim(&mut p, &pose, c.aim, eye);
    }
    match c.phase {
        0 if c.age > 4. => {
            if v.navigation && c.index == 0 {
                c.initial_links = nav.active_links(&access);
                c.initial_revision = nav.revision;
                let members = current_zone
                    .0
                    .as_ref()
                    .is_some_and(|z| access.reachable.contains(z));
                c.rows.push(serde_json::json!({"case":"compiled_graph","passed":nav.node_count()==2776 && nav.link_count()==23197 && members,"zone":current_zone.0,"rejected_nodes":nav.rejected_nodes,"rejected_links":nav.rejected_links,"admissions":nav.admissions.len()}));
                for d in &game.access.doors {
                    let gate = nav.inspect_gate(&d.connection, &access);
                    c.rows.push(serde_json::json!({"case":"closed_gate","door":d.connection,"passed":gate.is_some_and(|(_,_,allowed)|!allowed),"found":gate.is_some(),"links":nav.inspect_gate_nodes(&d.connection)}));
                }
            }
            c.owner = Some(owner);
            c.before = access.reachable.len();
            let Some(d) = game.access.doors.get(c.index) else {
                next = Some(6);
                if let Some(n) = next {
                    c.phase = n;
                    c.age = 0.;
                }
                return;
            };
            let placements=game.content.interactions.iter().filter_map(|i|matches!(&i.action,crate::origins::InteractionAction::Door{connection,..} if *connection==d.connection).then(||game.content.placements.iter().find(|p|p.id==i.placement)).flatten());
            let excluded: Vec<_> = std::iter::once(owner)
                .chain(
                    blockers
                        .iter()
                        .filter(|(_, b)| b.0 == d.connection)
                        .map(|(e, _)| e),
                )
                .collect();
            let filter = crate::environment::solids(excluded);
            let approach = placements
                .flat_map(|t| {
                    let endpoint = Vec3::from_array(t.position);
                    (0..32).flat_map(move |i| {
                        let angle = i as f32 * std::f32::consts::TAU / 32.;
                        let dir = Vec3::new(angle.cos(), 0., angle.sin());
                        [0.6, 1., 1.4, 1.8, 2.2]
                            .into_iter()
                            .flat_map(move |distance| {
                                [-2., 0., 2., 4.].into_iter().map(move |lift| {
                                    (endpoint, endpoint + dir * distance + Vec3::Y * lift)
                                })
                            })
                    })
                })
                .find_map(|(endpoint, probe)| {
                    let center = super::semtex::standing(
                        &query,
                        owner,
                        probe,
                        p.height,
                        game.tuning.radius,
                    )?;
                    let eye = center + Vec3::Y * (p.eye_height - p.height * 0.5);
                    let delta = endpoint - eye;
                    if delta.length() > 2.25
                        || query
                            .cast_ray(
                                eye,
                                Dir3::new(delta).ok()?,
                                (delta.length() - 0.12).max(0.),
                                true,
                                &filter,
                            )
                            .is_some()
                    {
                        return None;
                    }
                    Some((center, endpoint))
                });
            if let Some((center, aim)) = approach {
                c.center = center;
                c.aim = aim;
                session.points = 0;
                next = Some(1);
            } else {
                let index = c.index;
                c.rows.push(
                    serde_json::json!({"case":"native_approach","index":index,"passed":false}),
                );
                next = Some(8);
            }
        }
        1 => {
            if (c.index == 0 || c.index == 4) && c.age > 0.1 && c.age < 0.12 {
                commands
                    .spawn(Screenshot::primary_window())
                    .observe(save_to_disk(
                        v.output.join(format!("door-{}-closed.png", c.index)),
                    ));
            }
            input.use_held = true;
            if c.age > 0.2 {
                let d = &game.access.doors[c.index];
                let index = c.index;
                c.rows.push(serde_json::json!({"case":"insufficient_funds","index":index,"passed":session.points==0 && !access.opened(&d.connection) && focus.rejection==Some(InteractionRejection::InsufficientPoints)}));
                next = Some(2);
            }
        }
        2 if c.age > 0.1 => {
            session.points = 10000;
            next = Some(3);
        }
        3 => {
            input.use_held = true;
            if c.age > 0.2 {
                next = Some(4);
            }
        }
        4 => {
            if c.index == 4 && c.age < 0.02 {
                commands
                    .spawn(Screenshot::primary_window())
                    .observe(save_to_disk(v.output.join("door-bunker-mid.png")));
            }
            input.use_held = true;
            if c.age > 1.4 {
                let d = &game.access.doors[c.index];
                let price = game
                    .content
                    .interactions
                    .iter()
                    .find_map(|i| {
                        if let crate::origins::InteractionAction::Door { connection, price } =
                            &i.action
                        {
                            (*connection == d.connection).then_some(*price)
                        } else {
                            None
                        }
                    })
                    .unwrap();
                let index = c.index;
                let passed = session.points == 10000 - price
                    && d.connections.iter().all(|id| access.opened(id))
                    && blockers.iter().all(|(_, b)| b.0 != d.connection)
                    && props.iter().all(|p| p.connection != d.connection)
                    && access.reachable.len() >= c.before;
                c.rows.push(serde_json::json!({"case":"group_purchase","index":index,"passed":passed,"price":price,"points":session.points,"reachable":access.reachable.len(),"all_edges":d.connections.len()}));
                if c.index == 0 || c.index == 4 {
                    commands
                        .spawn(Screenshot::primary_window())
                        .observe(save_to_disk(
                            v.output.join(format!("door-{}-open.png", c.index)),
                        ));
                }
                next = Some(5);
            }
        }
        5 if c.age > 0.2 => {
            c.index += 1;
            next = Some(0);
        }
        6 => {
            if v.navigation {
                for d in &game.access.doors {
                    let gate = nav.inspect_gate(&d.connection, &access);
                    let passed = gate.is_some_and(|(from, to, allowed)| {
                        let point = nav.destination(from, to, &access);
                        allowed && point.distance(to) < 0.5
                    });
                    c.rows.push(serde_json::json!({"case":"opened_route","door":d.connection,"passed":passed}));
                }
                let initial = c.initial_links;
                let epoch = c.initial_revision;
                let active = nav.active_links(&access);
                c.rows.push(serde_json::json!({"case":"replan_after_unlocks","passed":active>initial && nav.revision>epoch,"initial_links":initial,"active_links":active,"revision":nav.revision,"graph_changes":nav.graph_changes}));
                c.rows.push(serde_json::json!({"case":"live_actor_replanning","passed":enemy_evidence.target_replans>0 && enemy_evidence.blocker_replans>0,"target":enemy_evidence.target_replans,"blocker":enemy_evidence.blocker_replans,"direct":enemy_evidence.direct_pursuits,"stuck":enemy_evidence.stuck_recoveries,"separation":enemy_evidence.separations}));
                let bounded = nav.inspect_cache(&access);
                let hits = nav.cache_hits;
                c.rows.push(serde_json::json!({"case":"bounded_route_cache","passed":bounded && hits>0,"cache":nav.cache_len(),"requests":nav.requests,"hits":hits,"fallbacks":nav.fallbacks}));
            }

            let expected = game
                .access
                .doors
                .iter()
                .map(|d| d.props.len())
                .sum::<usize>() as u32;
            c.rows.push(serde_json::json!({"case":"all_groups","passed":access.purchases as usize==game.access.doors.len() && blockers.is_empty() && props.is_empty() && evidence.animated==expected && evidence.sounds==expected && evidence.playing_sounds==expected && access.reachable.len()>3,"purchases":access.purchases,"removed":evidence.removed,"animated":evidence.animated,"sounds":evidence.sounds,"playing_sounds":evidence.playing_sounds}));
            pose.translation = Vec3::from_array(game.spawn) + Vec3::Y * 30.;
            p.velocity = Vec3::ZERO;
            mobility.airborne = false;
            mobility.peak = pose.translation.y - p.height * 0.5;
            next = Some(7);
        }
        7 if **state == GameState::GameOver => {
            commands.write_message(crate::input::RestartGame);
            next = Some(8);
        }
        8 if **state == GameState::Playing && c.age > 1. => {
            let old = c.owner;
            c.rows.push(serde_json::json!({"case":"restart","passed":Some(owner)!=old && access.purchases==0 && blockers.iter().count()==game.access.doors.iter().map(|d|d.blocks.len()).sum::<usize>() && props.iter().count()==game.access.doors.iter().map(|d|d.props.len()).sum::<usize>() && evidence.removed==0 && health.current()==Some(100) && session.points==500}));
            if v.navigation {
                let initial = c.initial_links;
                let active = nav.active_links(&access);
                c.rows.push(serde_json::json!({"case":"navigation_restart","passed":nav.cache_len()==0 && active==initial && nav.graph_changes==1,"cache":nav.cache_len(),"active_links":active,"zone":current_zone.0,"admissions":nav.admissions.len()}));
            }
            let passed = c.rows.iter().all(|r| r["passed"] == true);
            let _=std::fs::write(v.output.join(if v.navigation {"navigation.json"}else{"doors.json"}),serde_json::json!({"passed":passed,"rows":c.rows,"groups":game.access.doors.len(),"zones":game.content.zones.len()}).to_string());
            commands.write_message(if passed {
                AppExit::Success
            } else {
                AppExit::error()
            });
            next = Some(9);
        }
        _ => {}
    }
    if c.age > 30. && c.phase != 9 {
        let phase = c.phase;
        c.rows
            .push(serde_json::json!({"case":"timeout","phase":phase,"passed":false}));
        next = Some(8);
    }
    if let Some(phase) = next {
        let _ = std::fs::write(
            v.output.join("doors-progress.json"),
            serde_json::json!({"phase":phase,"index":c.index,"rows":c.rows}).to_string(),
        );
        c.phase = phase;
        c.age = 0.;
    }
}

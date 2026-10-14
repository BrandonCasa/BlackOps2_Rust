use super::*;
use crate::{
    access::{DoorBlocker, DoorEvidence, DoorProp, ZoneAccess},
    interactions::{InteractionFocus, InteractionRejection},
    life::Vitality,
    movement::Mobility,
};
use avian3d::prelude::*;
#[derive(Component)]
struct ControlledMine;
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
    actor: Option<Entity>,
    traversal_start: u32,
    traversal_finish: u32,
    traversal_blocked_start: u32,
    climb_capture: bool,
    landed_at: Option<f32>,
    landing_seen: bool,
    traversal_blast: Option<u32>,
    traversal_hit: bool,
    traversal_yaw: Option<f32>,
    heading_checked: bool,
    combat_case: usize,
    combat_before: Option<usize>,
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
    enemies: Query<
        (
            Entity,
            &Transform,
            &Zombie,
            &Vitality,
            Option<&ColliderDisabled>,
        ),
        With<Zombie>,
    >,
    blockers: Query<(Entity, &DoorBlocker)>,
    props: Query<&DoorProp>,
    access: Res<ZoneAccess>,
    focus: Res<InteractionFocus>,
    inspection: (
        Res<DoorEvidence>,
        Res<crate::audio::AudioEvidence>,
        ResMut<crate::navigation::Navigation>,
        Res<crate::access::CurrentZone>,
        Res<crate::enemies::EnemyEvidence>,
        ResMut<crate::equipment::BlastQueue>,
        Res<crate::damage::CombatHistory>,
        Res<crate::mines::MineEvidence>,
        Query<Entity, With<ControlledMine>>,
        Res<crate::session::SessionId>,
    ),
) {
    let (
        evidence,
        audio,
        mut nav,
        current_zone,
        enemy_evidence,
        mut blasts,
        history,
        mine_evidence,
        controlled_mines,
        session_id,
    ) = inspection;
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
    for (e, body, _, _, _) in &enemies {
        if c.actor != Some(e)
            && (!v.navigation || c.phase >= 6 || body.translation.distance(pose.translation) < 3.)
        {
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
                let observed = nav.negotiation_observation(&game, &access);
                let _ = std::fs::write(v.output.join("traversal-links.json"), observed.to_string());
                c.rows.push(serde_json::json!({"case":"compiled_negotiations","passed":observed["passed"],"expected":observed["expected"],"retained":observed["retained"],"geometry_admitted":observed["geometry_admitted"]}));
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
                c.rows.push(serde_json::json!({"case":"insufficient_funds","index":index,"passed":audio.events.get("zmb_cha_ching").copied().unwrap_or(0)==index as u32 && session.points==0 && !access.opened(&d.connection) && focus.rejection==Some(InteractionRejection::InsufficientPoints)}));
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
                let passed = audio.events.get("zmb_cha_ching").copied().unwrap_or(0)
                    == index as u32 + 1
                    && session.points == 10000 - price
                    && d.connections.iter().all(|id| access.opened(id))
                    && blockers.iter().all(|(_, b)| b.0 != d.connection)
                    && props.iter().all(|p| p.connection != d.connection)
                    && access.reachable.len() >= c.before;
                c.rows.push(serde_json::json!({"case":"group_purchase","index":index,"passed":passed,"price":price,"points":session.points,"reachable":access.reachable.len(),"all_edges":d.connections.len(),"buy_cues":audio.events.get("zmb_cha_ching").copied().unwrap_or(0)}));
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
            c.rows.push(serde_json::json!({"case":"all_groups","passed":access.purchases as usize==game.access.doors.len() && blockers.is_empty() && props.is_empty() && evidence.animated==expected && evidence.sounds==expected && evidence.playing_sounds>0 && evidence.playing_sounds<=expected && access.reachable.len()>3,"purchases":access.purchases,"buy_cues":audio.events.get("zmb_cha_ching"),"removed":evidence.removed,"animated":evidence.animated,"sounds":evidence.sounds,"playing_sounds":evidence.playing_sounds}));
            if v.traversal {
                if let Some((start, end, alias)) = nav.inspect_traversal(&access) {
                    c.traversal_yaw = game
                        .content
                        .navigation_links
                        .iter()
                        .filter_map(|l| l.negotiation.as_ref())
                        .find(|n| {
                            (Vec3::from_array(n.start.position) - start)
                                .xz()
                                .length_squared()
                                < 0.0001
                                && (Vec3::from_array(n.end.position) - end)
                                    .xz()
                                    .length_squared()
                                    < 0.0001
                        })
                        .map(|n| n.start.yaw);
                    let enemy = game
                        .content
                        .enemies
                        .iter()
                        .find(|e| e.family == "german")
                        .unwrap();
                    c.actor = Some(
                        commands
                            .spawn((
                                Zombie::new(
                                    start + Vec3::Y * 0.85,
                                    enemy.id.clone(),
                                    crate::rounds::Gait::Walk,
                                ),
                                crate::player::capsule(0.3, 1.7),
                                crate::life::Vitality::ordinary_actor(150, 150).unwrap(),
                                crate::rounds::PopulationKind::Ordinary,
                                crate::gameplay::SessionEntity,
                                Transform::from_translation(start + Vec3::Y * 0.85),
                                Visibility::Inherited,
                            ))
                            .id(),
                    );
                    session.pending = 0;
                    c.center = end + Vec3::Y * 0.9;
                    c.aim = start + Vec3::Y * 0.85;
                    c.traversal_start = enemy_evidence.traversals_started;
                    c.traversal_finish = enemy_evidence.traversals_finished;
                    c.traversal_blocked_start = enemy_evidence.traversal_blocked_landings;
                    let source_yaw = c.traversal_yaw;
                    c.rows.push(serde_json::json!({"case":"native_traversal_selected","passed":source_yaw.is_some(),"alias":alias,"source_yaw":source_yaw,"start":start.to_array(),"end":end.to_array()}));
                    c.phase = 10;
                    c.age = 0.;
                    return;
                }
                c.rows
                    .push(serde_json::json!({"case":"native_traversal_selected","passed":false}));
            }
            pose.translation = Vec3::from_array(game.spawn) + Vec3::Y * 30.;
            p.velocity = Vec3::ZERO;
            mobility.airborne = false;
            mobility.peak = pose.translation.y - p.height * 0.5;
            next = Some(7);
        }
        10 => {
            pose.translation = c.center;
            if enemy_evidence.traversal_blocked_landings > c.traversal_blocked_start {
                pose.translation += Vec3::Y * 2.;
            }
            p.velocity = Vec3::ZERO;
            let eye = p.eye_height;
            let aim = enemies
                .iter()
                .find(|(e, _, _, _, _)| Some(*e) == c.actor)
                .map_or(c.aim, |(_, body, _, _, _)| body.translation + Vec3::Y * 0.3);
            super::semtex::aim(&mut p, &pose, aim, eye);
            if !c.climb_capture
                && enemy_evidence.traversals_started > c.traversal_start
                && c.age > 1.2
            {
                commands
                    .spawn(bevy::render::view::screenshot::Screenshot::primary_window())
                    .observe(bevy::render::view::screenshot::save_to_disk(
                        v.output.join("native-traversal.png"),
                    ));
                c.climb_capture = true;
            }
            let actor_action = enemies
                .iter()
                .find(|(e, _, _, _, _)| Some(*e) == c.actor)
                .map(|(_, _, z, _, _)| (z.action, z.elapsed));
            if let Some((entity, body, z, vitality, disabled)) =
                enemies.iter().find(|(e, _, _, _, _)| Some(*e) == c.actor)
            {
                if !c.heading_checked && z.action == crate::origins::EnemyAnimation::Traverse {
                    let forward = body.rotation * Vec3::X;
                    let matched = c.traversal_yaw.is_some_and(|yaw| {
                        forward.distance(Vec3::new(yaw.cos(), 0., -yaw.sin())) < 0.0001
                    });
                    let source_yaw = c.traversal_yaw;
                    let endpoint_yaw = (-((c.center - c.aim).z)).atan2((c.center - c.aim).x);
                    let difference = source_yaw.map(|yaw| {
                        ((yaw - endpoint_yaw + std::f32::consts::PI)
                            .rem_euclid(std::f32::consts::TAU)
                            - std::f32::consts::PI)
                            .abs()
                            .to_degrees()
                    });
                    let changed = difference.is_some_and(|d| d > 1.);
                    c.rows.push(serde_json::json!({"case":"authored_traversal_heading","passed":matched && changed,"source_yaw":source_yaw,"endpoint_heading_difference_degrees":difference,"actual_forward":forward.to_array()}));
                    c.heading_checked = true;
                }
                if c.traversal_blast.is_none()
                    && disabled.is_some()
                    && z.action == crate::origins::EnemyAnimation::Traverse
                {
                    c.traversal_blast = vitality.current();
                    blasts
                        .0
                        .push(combat_blast(*session_id, owner, body.translation, 0.5, 10));
                } else if !c.traversal_hit
                    && let Some(before) = c.traversal_blast
                    && vitality.current() == Some(before - 10)
                {
                    let attributed = history.recent.iter().any(|r| {
                        r.impact.target == entity
                            && r.impact.attacker == owner
                            && r.impact.source.id() == "weapon.frag_grenade_zm"
                            && r.impact.amount == 10
                    });
                    c.rows.push(serde_json::json!({"case":"traversing_blast_damage","passed":attributed,"before":before,"after":vitality.current()}));
                    c.traversal_hit = attributed;
                }
            }
            if enemy_evidence.traversals_finished > c.traversal_finish {
                if c.landed_at.is_none() {
                    c.landed_at = Some(c.age);
                }
                if actor_action.is_some_and(|(a, t)| {
                    a == crate::origins::content::EnemyAnimation::Landing && t > 0.1
                }) {
                    c.landing_seen = true;
                }
            }
            if c.landed_at.is_some_and(|age| c.age - age > 0.6) || c.age > 15. {
                let passed = enemy_evidence.traversals_started > c.traversal_start
                    && enemy_evidence.traversals_finished > c.traversal_finish;
                c.rows.push(serde_json::json!({"case":"native_traversal_completed","passed":passed,"started":enemy_evidence.traversals_started,"finished":enemy_evidence.traversals_finished,"blocked_landings":enemy_evidence.traversal_blocked_landings,"rising_finished":enemy_evidence.rising_finished}));
                let recovered = c.landing_seen
                    && actor_action.is_some_and(|(a, _)| {
                        a != crate::origins::content::EnemyAnimation::Landing
                    });
                let held = c.landing_seen;
                c.rows.push(serde_json::json!({"case":"landing_recovery","passed":recovered,"observed_hold":held,"released":actor_action.map(|(a,_)|format!("{a:?}"))}));
                let observed = c.traversal_hit;
                c.rows.push(
                    serde_json::json!({"case":"traversing_blast_observed","passed":observed}),
                );
                if let Some(e) = c.actor.take() {
                    commands.entity(e).despawn();
                }
                next = Some(11);
            }
        }
        11 => {
            let enemy = game
                .content
                .enemies
                .iter()
                .find(|e| e.family == "german" && e.traversals.contains_key("rise"))
                .unwrap();
            let point = c.aim;
            let mut zombie = Zombie::new(point, enemy.id.clone(), crate::rounds::Gait::Walk)
                .rising(&enemy.traversals["rise"][0]);
            zombie.stunned = 100.;
            c.actor = Some(
                commands
                    .spawn((
                        zombie,
                        crate::player::capsule(0.3, 1.7),
                        Vitality::ordinary_actor(150, 150).unwrap(),
                        ColliderDisabled,
                        crate::rounds::PopulationKind::Ordinary,
                        crate::gameplay::SessionEntity,
                        Transform::from_translation(point),
                        Visibility::Inherited,
                    ))
                    .id(),
            );
            c.combat_case = 0;
            next = Some(12);
        }
        12 => {
            pose.translation = c.center + Vec3::Y * 2.;
            p.velocity = Vec3::ZERO;
            if let Some(actor) = c.actor
                && let Ok((_, body, z, vitality, disabled)) = enemies.get(actor)
            {
                let receipts = history
                    .recent
                    .iter()
                    .filter(|r| {
                        r.impact.target == actor && r.impact.source.id() == "weapon.frag_grenade_zm"
                    })
                    .count();
                if let Some(before) = c.combat_before {
                    if c.age > 0.15 {
                        let expected_hit = c.combat_case == 0 || c.combat_case == 3;
                        let passed = receipts == before + usize::from(expected_hit)
                            && (c.combat_case != 0
                                || (vitality.current() == Some(140)
                                    && disabled.is_some()
                                    && z.action == crate::origins::EnemyAnimation::Traverse));
                        let names = [
                            "rising_blast_damage",
                            "outside_blast_radius",
                            "blast_cover",
                            "lethal_blast",
                            "dead_blast_excluded",
                        ];
                        let name = names[c.combat_case];
                        c.rows.push(serde_json::json!({"case":name,"passed":passed,"receipts_before":before,"receipts_after":receipts,"health":vitality.current()}));
                        c.combat_case += 1;
                        c.combat_before = None;
                        c.age = 0.;
                        if c.combat_case == 5 {
                            commands.entity(actor).despawn();
                            c.actor = None;
                            next = Some(13);
                        }
                    }
                } else {
                    let point = body.translation;
                    let request = match c.combat_case {
                        0 => Some(combat_blast(*session_id, owner, point, 0.5, 10)),
                        1 => Some(combat_blast(
                            *session_id,
                            owner,
                            point + Vec3::X * 2.,
                            0.1,
                            10,
                        )),
                        2 => [Vec3::X, Vec3::NEG_X, Vec3::Z, Vec3::NEG_Z, Vec3::Y]
                            .into_iter()
                            .find_map(|axis| {
                                let hit = query.cast_ray(
                                    point,
                                    Dir3::new(axis).unwrap(),
                                    8.,
                                    true,
                                    &crate::environment::solids([actor, owner]),
                                )?;
                                let center = point + axis * (hit.distance + 0.3);
                                Some(combat_blast(
                                    *session_id,
                                    owner,
                                    center,
                                    hit.distance + 1.,
                                    10,
                                ))
                            }),
                        3 => Some(combat_blast(*session_id, owner, point, 0.5, 200)),
                        _ => Some(combat_blast(*session_id, owner, point, 0.5, 10)),
                    };
                    if let Some(request) = request {
                        blasts.0.push(request);
                        c.combat_before = Some(receipts);
                        c.age = 0.;
                    }
                }
            }
        }
        13 => {
            let enemy = game
                .content
                .enemies
                .iter()
                .find(|e| e.family == "german")
                .unwrap();
            let point = c.aim;
            let mut zombie = Zombie::new(point, enemy.id.clone(), crate::rounds::Gait::Walk)
                .rising(&enemy.traversals["rise"][0]);
            zombie.stunned = 100.;
            c.actor = Some(
                commands
                    .spawn((
                        zombie,
                        crate::player::capsule(0.3, 1.7),
                        Vitality::ordinary_actor(150, 150).unwrap(),
                        ColliderDisabled,
                        crate::gameplay::SessionEntity,
                        Transform::from_translation(point),
                        Visibility::Inherited,
                    ))
                    .id(),
            );
            c.combat_case = 0;
            c.combat_before = None;
            next = Some(14);
        }
        14 => {
            pose.translation = c.center + Vec3::Y * 2.;
            p.velocity = Vec3::ZERO;
            if let Some(actor) = c.actor
                && let Ok((_, body, _, _, _)) = enemies.get(actor)
            {
                if let Some(before) = c.combat_before {
                    if c.age > 0.15 {
                        let passed = mine_evidence.triggered as usize
                            == before + usize::from(c.combat_case == 4);
                        let names = [
                            "claymore_radius_excluded",
                            "claymore_height_excluded",
                            "claymore_cone_excluded",
                            "claymore_cover_excluded",
                            "claymore_disabled_actor_trigger",
                        ];
                        let name = names[c.combat_case];
                        c.rows.push(serde_json::json!({"case":name,"passed":passed,"triggers_before":before,"triggers_after":mine_evidence.triggered}));
                        if c.combat_case == 3 {
                            commands
                                .entity(actor)
                                .insert(Transform::from_translation(c.aim));
                        }
                        c.combat_before = None;
                        c.combat_case += 1;
                        c.age = 0.;
                        for entity in &controlled_mines {
                            commands.entity(entity).despawn();
                        }
                        if c.combat_case == 5 {
                            commands.entity(actor).despawn();
                            c.actor = None;
                            pose.translation = Vec3::from_array(game.spawn) + Vec3::Y * 30.;
                            mobility.airborne = false;
                            mobility.peak = pose.translation.y - p.height * 0.5;
                            next = Some(7);
                        }
                    }
                } else {
                    let profile = game
                        .content
                        .weapons
                        .iter()
                        .find(|w| w.id.0 == "weapon.claymore_zm")
                        .unwrap()
                        .mine
                        .as_ref()
                        .unwrap();
                    let mut point = body.translation;
                    let mut center = match c.combat_case {
                        0 => point + Vec3::Z * (profile.trigger_radius + 1.),
                        1 => point + Vec3::Z * 1.5 - Vec3::Y * 4.,
                        2 => point - Vec3::Z * 1.5,
                        _ => point + Vec3::Z * 1.5,
                    };
                    if c.combat_case == 3 {
                        if let Some(hit) = [Vec3::X, Vec3::NEG_X, Vec3::Z, Vec3::NEG_Z]
                            .into_iter()
                            .find_map(|axis| {
                                query
                                    .cast_ray(
                                        point,
                                        Dir3::new(axis).unwrap(),
                                        8.,
                                        true,
                                        &crate::environment::solids([actor, owner]),
                                    )
                                    .map(|h| (axis, h.distance))
                            })
                        {
                            point += hit.0 * (hit.1 - 0.8);
                            commands
                                .entity(actor)
                                .insert(Transform::from_translation(point));
                            center = point + hit.0;
                        } else {
                            c.rows.push(serde_json::json!({"case":"claymore_cover_geometry","passed":false}));
                            commands.write_message(AppExit::error());
                            return;
                        }
                    }
                    let toward = (point - center).normalize();
                    commands.spawn((
                        crate::gameplay::SessionEntity,
                        ControlledMine,
                        crate::mines::PlantedMine {
                            owner,
                            definition: crate::origins::WeaponId("weapon.claymore_zm".into()),
                            countdown: None,
                            unsafe_plant: false,
                        },
                        Transform::from_translation(center).with_rotation(Quat::from_rotation_arc(
                            Vec3::NEG_Z,
                            if c.combat_case == 2 { -toward } else { toward },
                        )),
                        Visibility::Hidden,
                    ));
                    c.combat_before = Some(mine_evidence.triggered as usize);
                    c.age = 0.;
                }
                let _ = std::fs::write(v.output.join("doors-progress.json"), serde_json::json!({"phase":c.phase,"index":c.index,"combat_case":c.combat_case,"rows":c.rows}).to_string());
            }
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
                let observed = nav.negotiation_observation(&game, &access);
                let _ = std::fs::write(
                    v.output.join("traversal-links-restarted.json"),
                    observed.to_string(),
                );
                c.rows.push(serde_json::json!({"case":"restarted_negotiations","passed":observed["passed"],"retained":observed["retained"]}));
            }
            let passed = c.rows.iter().all(|r| r["passed"] == true);
            let _=std::fs::write(v.output.join(if v.traversal {"traversal.json"}else if v.navigation {"navigation.json"}else{"doors.json"}),serde_json::json!({"passed":passed,"rows":c.rows,"groups":game.access.doors.len(),"zones":game.content.zones.len()}).to_string());
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

fn combat_blast(
    session: crate::session::SessionId,
    owner: Entity,
    center: Vec3,
    radius: f32,
    amount: u32,
) -> crate::equipment::BlastRequest {
    crate::equipment::BlastRequest {
        session,
        owner,
        definition: crate::origins::WeaponId("weapon.frag_grenade_zm".into()),
        center,
        radius,
        inner_radius: 0.,
        inner_damage: amount,
        outer_damage: amount,
        direction: None,
        category: None,
        player_damage: None,
        source: None,
    }
}

use super::*;
use crate::{
    life::Vitality,
    movement::{Mobility, MovementEvidence},
    solo_revive::PerkInventory,
};
#[derive(Resource, Default)]
struct Coverage {
    phase: u8,
    age: f32,
    owner: Option<Entity>,
    base: Vec3,
    rows: Vec<serde_json::Value>,
    last_speed: f32,
    dipped: bool,
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
    time: Res<Time>,
    state: Res<State<GameState>>,
    mut c: ResMut<Coverage>,
    mut input: ResMut<GameInput>,
    mut session: ResMut<Session>,
    mut players: Query<(
        Entity,
        &mut Player,
        &mut Transform,
        &Vitality,
        &mut PerkInventory,
        &Mobility,
    )>,
    enemies: Query<Entity, With<Zombie>>,
    movement: Res<MovementEvidence>,
    effects: Res<crate::perk_effects::EffectEvidence>,
    query: avian3d::prelude::SpatialQuery,
    history: Res<crate::damage::CombatHistory>,
) {
    let (Some(v), Some(game), Ok((owner, mut p, mut pose, health, mut perks, m))) =
        (v, game, players.single_mut())
    else {
        return;
    };
    if !v.mobility {
        return;
    }
    input.clear();
    c.age += time.delta_secs();
    if v.started.elapsed().as_secs() > 120 {
        c.phase = 99;
    }
    let _=std::fs::write(v.output.join("mobility-progress.json"),serde_json::json!({"phase":c.phase,"age":c.age,"state":format!("{:?}",**state),"health":health.current(),"position":pose.translation.to_array(),"stance":format!("{:?}",m.stance),"grounded":p.grounded,"velocity":p.velocity.to_array(),"eye":p.eye_height,"events":movement.events,"effects":effects.events,"rows":c.rows}).to_string());
    let mut next = None;
    match c.phase {
        0 if **state == GameState::Playing && c.age > 3. => {
            for e in &enemies {
                commands.entity(e).despawn();
            }
            session.pending = 0;
            session.intermission = Some(Timer::from_seconds(300., TimerMode::Once));
            c.owner = Some(owner);
            let filter = crate::environment::solids([owner]);
            let mut clear = None;
            'search: for x in -12..=12 {
                for z in -12..=12 {
                    let probe = pose.translation + Vec3::new(x as f32 * 4., 0., z as f32 * 4.);
                    if let Some(base) =
                        super::semtex::standing(&query, owner, probe, p.height, game.tuning.radius)
                    {
                        let shape = crate::player::capsule(game.tuning.radius, p.height);
                        let config = avian3d::prelude::ShapeCastConfig::from_max_distance(10.);
                        if query
                            .cast_shape(&shape, base, Quat::IDENTITY, Dir3::Y, &config, &filter)
                            .is_none()
                            && query
                                .cast_shape(
                                    &shape,
                                    base,
                                    Quat::IDENTITY,
                                    Dir3::NEG_Z,
                                    &avian3d::prelude::ShapeCastConfig::from_max_distance(8.),
                                    &filter,
                                )
                                .is_none()
                        {
                            clear = Some(base);
                            break 'search;
                        }
                    }
                }
            }
            if let Some(base) = clear {
                pose.translation = base;
            } else {
                next = Some(99);
            }
            c.base = pose.translation;
            p.velocity = Vec3::ZERO;
            p.yaw = 0.;
            p.pitch = 0.;
            input.prone = true;
            if clear.is_some() {
                next = Some(1);
            }
        }
        1 if c.age > 0.5 => {
            c.rows.push(serde_json::json!({"case":"standing_prone","passed":p.prone && (p.eye_height-game.tuning.mobility.prone_eye).abs()<0.01,"height":p.height,"eye":p.eye_height}));
            commands
                .spawn(Screenshot::primary_window())
                .observe(save_to_disk(v.output.join("prone.png")));
            input.crouch = true;
            next = Some(2);
        }
        2 => {
            input.crouch = true;
            if c.age > 0.4 {
                c.rows.push(serde_json::json!({"case":"prone_crouch","passed":p.crouched && !p.prone && (p.height-game.tuning.crouch_height).abs()<0.01}));
                input.prone = true;
                next = Some(3);
            }
        }
        3 if c.age > 0.4 => {
            c.rows
                .push(serde_json::json!({"case":"crouch_prone","passed":p.prone}));
            input.jump = true;
            next = Some(4);
        }
        4 if c.age > 0.3 => {
            c.rows.push(serde_json::json!({"case":"prone_jump_rejected","passed":p.prone && p.grounded && p.velocity.y<=0.}));
            input.prone = true;
            next = Some(5);
        }
        5 if c.age > 0.4 => {
            c.rows.push(serde_json::json!({"case":"prone_stand","passed":!p.crouched && (p.eye_height-game.tuning.eye_height).abs()<0.01}));
            input.jump = true;
            next = Some(21);
        }
        21 => {
            input.movement = Vec2::X;
            if c.age > 0.12 {
                c.rows.push(serde_json::json!({"case":"air_control","passed":!p.grounded && p.velocity.x>0. && p.velocity.x < game.tuning.walk_speed,"velocity":p.velocity.to_array()}));
                next = Some(22);
            }
        }
        22 => {
            if p.grounded && m.jump_recovery > 0. {
                input.jump = true;
                next = Some(23);
            }
        }
        23 if c.age > 0.06 => {
            c.rows.push(serde_json::json!({"case":"jump_recovery","passed":p.grounded && p.velocity.y<=0. && m.jump_recovery>0.}));
            next = Some(6);
        }
        6 => {
            input.movement = Vec2::Y;
            if c.age > 0.08 {
                c.last_speed = p.velocity.xz().length();
                next = Some(7);
            }
        }
        7 => {
            input.movement = Vec2::Y;
            if c.age > 0.3 {
                let speed = p.velocity.xz().length();
                let before = c.last_speed;
                c.rows.push(serde_json::json!({"case":"acceleration","passed":before>0. && speed>before,"initial":before,"final":speed}));
                next = Some(8);
            }
        }
        8 if c.age > 0.3 => {
            c.rows.push(
                serde_json::json!({"case":"deceleration","passed":p.velocity.xz().length()<0.01}),
            );
            pose.translation = c.base;
            p.velocity = Vec3::ZERO;
            next = Some(9);
        }
        9 => {
            input.movement = Vec2::Y;
            input.sprint = true;
            if c.age > 0.3 && p.grounded {
                input.prone = true;
                next = Some(10);
            }
        }
        10 => {
            c.dipped |= m.landing_dip > 0.;
            if c.age > 0.12 && c.age < 0.2 {
                commands
                    .spawn(Screenshot::primary_window())
                    .observe(save_to_disk(v.output.join("dive.png")));
            }
            if c.age > 1.3 {
                let dipped = c.dipped;
                c.rows.push(serde_json::json!({"case":"dive_landing","passed":movement.events.iter().any(|e|e["event"]=="landed" && e["diving"]==true),"landing_dip":dipped}));
                input.prone = true;
                next = Some(11);
            }
        }
        11 if c.age > 0.5 => {
            pose.translation = c.base + Vec3::Y * 5.;
            p.velocity = Vec3::ZERO;
            next = Some(12);
        }
        12 if p.grounded && c.age > 0.8 => {
            c.rows.push(serde_json::json!({"case":"fall_damage","passed":health.current().is_some_and(|h|h<100) && effects.events.iter().any(|e|e["event"]=="fall_impact"),"health":health.current()}));
            perks
                .owned
                .insert(crate::origins::PerkId("perk.phd_flopper".into()));
            pose.translation = c.base + Vec3::Y * 5.;
            p.velocity = Vec3::ZERO;
            next = Some(13);
        }
        13 if p.grounded && c.age > 0.8 => {
            c.rows.push(serde_json::json!({"case":"phd_fall_protection","passed":effects.events.iter().any(|e|e["event"]=="fall_protected")}));
            pose.translation = c.base;
            p.velocity = Vec3::ZERO;
            next = Some(14);
        }
        14 => {
            input.movement = Vec2::Y;
            input.sprint = true;
            if c.age > 0.3 && p.grounded {
                input.prone = true;
                next = Some(15);
            }
        }
        15 if m.dive.is_some() => {
            pose.translation += Vec3::Y * 2.;
            let position = c.base + Vec3::new(1.5, 0., -5.);
            let id = game
                .content
                .enemies
                .iter()
                .find(|e| e.family == "german")
                .unwrap()
                .id
                .clone();
            let mut zombie = Zombie::new(position, id, crate::rounds::Gait::Walk);
            zombie.stunned = 5.;
            commands.spawn((
                zombie,
                crate::player::capsule(0.3, 1.7),
                Vitality::ordinary_actor(150, 150).unwrap(),
                crate::rounds::PopulationKind::Ordinary,
                crate::gameplay::SessionEntity,
                Visibility::Inherited,
                Transform::from_translation(position),
            ));
            next = Some(16);
        }
        16 if p.grounded && c.age > 0.7 => {
            c.rows.push(serde_json::json!({"case":"phd_dive_explosion","passed":effects.events.iter().any(|e|e["event"]=="dive_explosion") && history.recent.iter().any(|r|r.lethal && r.impact.source.id()=="weapon.zombie_perk_bottle_nuke")}));
            perks.owned.clear();
            input.prone = true;
            next = Some(17);
        }
        17 if c.age > 0.7 => {
            pose.translation = c.base + Vec3::Y * 9.;
            p.velocity = Vec3::ZERO;
            next = Some(18);
        }
        18 if **state == GameState::GameOver => {
            commands.write_message(crate::input::RestartGame);
            next = Some(19);
        }
        19 if **state == GameState::Playing && c.age > 0.7 => {
            let original = c.owner;
            c.rows.push(serde_json::json!({"case":"restart","passed":Some(owner)!=original && health.current()==Some(100) && !p.prone && !m.busy() && perks.owned.is_empty()}));
            let passed = c.rows.iter().all(|r| r["passed"] == true);
            let _=std::fs::write(v.output.join("mobility.json"),serde_json::json!({"passed":passed,"rows":c.rows,"events":movement.events,"effects":effects.events}).to_string());
            commands.write_message(if passed {
                AppExit::Success
            } else {
                AppExit::error()
            });
            next = Some(20);
        }
        99 => {
            commands.write_message(AppExit::error());
            next = Some(20);
        }
        _ => {}
    }
    if let Some(phase) = next {
        c.phase = phase;
        c.age = 0.;
    }
}

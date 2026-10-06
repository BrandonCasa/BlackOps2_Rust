//! Actual-window reload discharges, native actor damage, cooked frags and loss/reset.
use super::*;
use crate::{
    life::{LifeState, Vitality},
    perk_effects::{CherryState, EffectEvidence},
    solo_revive::PerkInventory,
    weapons::{WeaponAction, WeaponInventory},
};
use avian3d::prelude::*;
#[derive(Resource, Default)]
struct Coverage {
    phase: u8,
    original_owner: Option<Entity>,
    case: usize,
    elapsed: f32,
    begin: usize,
    protected: bool,
    picture: bool,
    lost: bool,
    rows: Vec<serde_json::Value>,
}
pub fn install(app: &mut App) {
    app.init_resource::<Coverage>().add_systems(
        PreUpdate,
        drive.after(super::drive).before(crate::player::look),
    );
}
fn drink(
    commands: &mut Commands,
    owner: Entity,
    id: &str,
    game: &GameDefinition,
    inv: &mut WeaponInventory,
) {
    let p = game.content.perks.iter().find(|p| p.id.0 == id).unwrap();
    inv.cancel_action();
    commands
        .entity(owner)
        .insert(crate::perks::PerkDrinking::new(p, inv, game).unwrap());
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
        &Transform,
        &Vitality,
        &mut PerkInventory,
        &mut WeaponInventory,
        &CherryState,
        &crate::equipment::GrenadeInventory,
        &crate::player_health::Recovery,
        Option<&crate::perks::PerkDrinking>,
    )>,
    enemies: Query<(Entity, &Vitality), With<Zombie>>,
    evidence: Res<EffectEvidence>,
    history: Res<crate::damage::CombatHistory>,
    query: SpatialQuery,
) {
    let (
        Some(v),
        Some(game),
        Ok((
            owner,
            mut player,
            pose,
            health,
            mut perks,
            mut inv,
            cherry,
            grenades,
            recovery,
            bottle,
        )),
    ) = (v, game, players.single_mut())
    else {
        return;
    };
    if !v.defensive_perks {
        return;
    }
    input.clear();
    c.elapsed += time.delta_secs();
    if v.started.elapsed().as_secs() > 180
        || **state == GameState::LoadError
        || (**state == GameState::GameOver && c.phase < 13)
    {
        c.phase = 99;
    }
    let _=std::fs::write(v.output.join("defensive-perks-progress.json"),serde_json::json!({"state":format!("{:?}",**state),"health":health.current(),"life":format!("{:?}",health.state()),"position":pose.translation.to_array(),"drinking":bottle.is_some(),"phase":c.phase,"case":c.case,"elapsed":c.elapsed,"magazine":inv.active().map(|w|w.magazine),"action":format!("{:?}",inv.action),"consecutive":cherry.consecutive,"cooldown":cherry.cooldown,"events":evidence.events,"rows":c.rows}).to_string());
    match c.phase {
        0 if **state == GameState::Playing && c.elapsed > 3. => {
            for (e, _) in &enemies {
                commands.entity(e).despawn();
            }
            session.pending = 0;
            session.intermission = Some(Timer::from_seconds(300., TimerMode::Once));
            c.original_owner = Some(owner);
            player.pitch = 1.4;
            player.yaw = 0.;
            drink(
                &mut commands,
                owner,
                "perk.electric_cherry",
                &game,
                &mut inv,
            );
            c.phase = 1;
            c.elapsed = 0.;
        }
        1 if bottle.is_none() && c.elapsed > 4.6 => {
            let identity = game
                .content
                .enemies
                .iter()
                .find(|e| e.family == "german")
                .unwrap()
                .id
                .clone();
            let feet = pose.translation - Vec3::Y * player.height * 0.5;
            for i in 0..12 {
                let a = i as f32 * std::f32::consts::TAU / 12.;
                let base = feet + Vec3::new(a.cos() * 1.85, 0., a.sin() * 1.85);
                let floor = query.cast_ray(
                    base + Vec3::Y * 1.,
                    Dir3::NEG_Y,
                    3.,
                    false,
                    &crate::environment::solids([owner]),
                );
                let position = base + Vec3::Y * (floor.map_or(0., |h| 1. - h.distance) + 0.87);
                let amount = if i == 0 { 500 } else { 10000 };
                commands.spawn((
                    Zombie::new(position, identity.clone(), crate::rounds::Gait::Walk, 0.),
                    crate::player::capsule(0.3, 1.7),
                    Vitality::ordinary_actor(amount, amount).unwrap(),
                    crate::rounds::PopulationKind::Ordinary,
                    crate::gameplay::SessionEntity,
                    Visibility::Inherited,
                    Transform::from_translation(position),
                ));
            }
            c.phase = 2;
            c.elapsed = 0.;
        }
        2 if c.elapsed > 2. => {
            c.phase = 3;
            c.elapsed = 0.;
        }
        3 => {
            let desired = match c.case {
                0 | 1 | 7 => 0,
                6 => 7,
                _ => 4,
            };
            player.pitch = 1.4;
            player.yaw = 0.;
            if inv.active().unwrap().magazine > desired {
                input.fire = true;
            } else {
                c.begin = evidence.events.len();
                c.phase = 4;
                c.elapsed = 0.;
            }
        }
        4 => {
            input.reload = true;
            if inv.action != WeaponAction::Ready {
                c.phase = 5;
                c.elapsed = 0.;
            }
        }
        5 if inv.action == WeaponAction::Ready && c.elapsed > 0.5 => {
            let events: Vec<_> = evidence.events[c.begin..]
                .iter()
                .filter(|e| e["event"] == "cherry_discharge")
                .cloned()
                .collect();
            let Some(event) = events.first() else {
                c.phase = 99;
                return;
            };
            let expected = match c.case {
                0 | 5 | 7 => None,
                1 => Some(8),
                2 => Some(4),
                6 => Some(8),
                3 => Some(2),
                4 => Some(0),
                _ => Some(0),
            };
            let count = event["targets"].as_u64().unwrap();
            let passed = events.len() == 1
                && event["limit"].as_u64() == expected
                && match c.case {
                    0 => count == 12,
                    1 => count == 8,
                    2 => count == 4,
                    3 => count == 2,
                    4 => count == 0,
                    _ => true,
                };
            let row = serde_json::json!({"case":c.case,"passed":passed,"discharge":event,"impacts":evidence.events[c.begin..].iter().filter(|e|e["event"]=="cherry_impact").count()});
            c.rows.push(row);
            if !passed {
                c.phase = 99;
                return;
            }
            if c.case == 0 {
                commands
                    .spawn(Screenshot::primary_window())
                    .observe(save_to_disk(v.output.join("cherry-actors.png")));
            }
            match c.case {
                4 => {
                    for (e, _) in &enemies {
                        commands.entity(e).despawn();
                    }
                    c.phase = 6;
                    c.elapsed = 0.;
                }
                6 => {
                    c.phase = 7;
                    c.elapsed = 0.;
                }
                7 => {
                    drink(&mut commands, owner, "perk.phd_flopper", &game, &mut inv);
                    c.phase = 8;
                    c.elapsed = 0.;
                }
                _ => {
                    c.case += 1;
                    c.phase = 3;
                    c.elapsed = 0.;
                }
            }
        }
        5 => {
            player.pitch = -0.3;
            if c.case == 0 && c.elapsed > 0.25 && !c.picture {
                c.picture = true;
                commands
                    .spawn(Screenshot::primary_window())
                    .observe(save_to_disk(v.output.join("cherry-discharge.png")));
            }
        }
        6 if cherry.consecutive == 0 && c.elapsed > 5. => {
            let w = inv.definition(&game.content.weapons).unwrap().clone();
            let _ = inv.refill_weapon(&w);
            c.case = 5;
            c.phase = 3;
            c.elapsed = 0.;
        }
        7 if cherry.consecutive == 0 && c.elapsed > 5. => {
            perks
                .owned
                .insert(crate::origins::PerkId("perk.speed_cola".into()));
            c.case = 7;
            c.phase = 3;
            c.elapsed = 0.;
        }
        8 if bottle.is_none() && c.elapsed > 4.6 => {
            c.phase = 9;
            c.elapsed = 0.;
        }
        9 => {
            input.grenade_held = true;
            if c.elapsed > 5. && !grenades.busy() {
                input.grenade_held = false;
                c.protected = health.current() == Some(100)
                    && recovery.immunity == 0.
                    && evidence
                        .events
                        .iter()
                        .any(|e| e["event"] == "explosive_protected");
                if c.protected {
                    perks
                        .owned
                        .remove(&crate::origins::PerkId("perk.phd_flopper".into()));
                    let _ = perks.buy_solo(&game.solo_revive);
                    c.phase = 10;
                    c.elapsed = 0.;
                } else {
                    c.phase = 99;
                }
            }
        }
        10 if c.elapsed > 0.2 => {
            c.phase = 11;
            c.elapsed = 0.;
        }
        11 => {
            input.grenade_held = true;
            if health.state() == LifeState::Downed {
                input.grenade_held = false;
                c.phase = 12;
                c.elapsed = 0.;
            }
        }
        12 => {
            c.lost |= perks.owned.is_empty() && cherry.consecutive == 0 && cherry.cooldown == 0.;
            if health.state() == LifeState::Standing && c.elapsed > 10.5 {
                let killed=history.recent.iter().filter(|r|matches!(&r.impact.source,crate::damage::ImpactSource::Perk(p) if p.0=="perk.electric_cherry") && r.lethal).count();
                let native_reward=history.recent.iter().any(|r|matches!(&r.impact.source,crate::damage::ImpactSource::Perk(p) if p.0=="perk.electric_cherry") && r.lethal && r.points==40);
                let speed = c.rows[7]["discharge"]["cooldown"].as_f64().unwrap();
                let expected_speed = game
                    .content
                    .weapons
                    .iter()
                    .find(|w| w.id.0 == "weapon.c96_zm")
                    .unwrap()
                    .reload_seconds
                    * game
                        .content
                        .perks
                        .iter()
                        .find(|p| p.id.0 == "perk.speed_cola")
                        .unwrap()
                        .reload_duration_scale
                        .unwrap()
                    + game
                        .content
                        .perks
                        .iter()
                        .find_map(|p| p.cherry.as_ref())
                        .unwrap()
                        .cooldown_extra;
                let scaled = c.rows[5]["discharge"]["damage"] == 523
                    && c.rows[6]["discharge"]["damage"] == 132;
                let passed = c.protected
                    && c.lost
                    && killed == 1
                    && native_reward
                    && scaled
                    && (speed - f64::from(expected_speed)).abs() < 0.0001;
                let _=std::fs::write(v.output.join("defensive-perks.json"),serde_json::json!({"passed":passed,"rows":c.rows,"protected":c.protected,"lost":c.lost,"cherry_kills":killed,"native_reward":native_reward,"scaled":scaled,"events":evidence.events,"scope":"actual input-driven shots/reloads and two cooked frags, native bottle/actor rendering and delayed shocks; controlled perk awards, ammo refill, controlled native actors/health/population, actor removal after target-limit coverage and PhD removal; legitimate acquisition/audio/native stun animation and interrupted latch remain unqualified"}).to_string());
                if passed {
                    c.phase = 13;
                    c.elapsed = 0.;
                } else {
                    c.phase = 99;
                }
            }
        }
        13 => {
            input.grenade_held = true;
            if **state == GameState::GameOver {
                input.grenade_held = false;
                commands.write_message(crate::input::RestartGame);
                c.phase = 14;
                c.elapsed = 0.;
            }
        }
        14 if **state == GameState::Playing && c.elapsed > 0.7 => {
            let passed = perks.owned.is_empty()
                && cherry.consecutive == 0
                && cherry.cooldown == 0.
                && health.current() == Some(100)
                && Some(owner) != c.original_owner;
            let _ = std::fs::write(
                v.output.join("defensive-perks-restart.json"),
                serde_json::json!({"passed":passed}).to_string(),
            );
            c.phase = if passed { 15 } else { 99 };
            c.elapsed = 0.;
        }
        15 if c.elapsed > 0.5 => {
            commands.write_message(AppExit::Success);
        }
        99 => {
            let _=std::fs::write(v.output.join("defensive-perks-failure.json"),serde_json::json!({"passed":false,"state":format!("{:?}",**state),"health":health.current(),"life":format!("{:?}",health.state()),"position":pose.translation.to_array(),"phase":c.phase,"case":c.case,"rows":c.rows,"events":evidence.events}).to_string());
            commands.write_message(AppExit::error());
        }
        _ => {}
    }
}

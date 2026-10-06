//! Paid native-machine and wall-buy coverage of third-slot ownership and loss.
use super::*;
use crate::{
    life::{LifeState, Vitality},
    solo_revive::PerkInventory,
    weapons::{WeaponAction, WeaponInventory},
};
use avian3d::prelude::*;
#[derive(Resource, Default)]
struct Coverage {
    phase: u8,
    elapsed: f32,
    normal: Vec<(String, u32, u32)>,
    insufficient: bool,
    expanded: bool,
    assigned: bool,
    replaced: bool,
    selected: bool,
    cycled: bool,
    fired: bool,
    downed: bool,
    restored: bool,
    reacquired: bool,
    rows: Vec<serde_json::Value>,
    picture: bool,
}
pub fn install(app: &mut App) {
    app.init_resource::<Coverage>().add_systems(
        PreUpdate,
        drive.after(super::drive).before(crate::player::look),
    );
}
fn ordinary(inv: &WeaponInventory, game: &GameDefinition) -> Vec<(String, u32, u32)> {
    inv.owned()
        .filter(|(slot, _)| *slot < 2)
        .filter_map(|(_, w)| {
            game.content
                .weapons
                .iter()
                .find(|d| d.id == w.definition)
                .map(|d| (w.definition.0.clone(), w.magazine, inv.reserve(d)))
        })
        .collect()
}
fn wall_approach(
    query: &SpatialQuery,
    owner: Entity,
    player: &Player,
    position: Vec3,
    normal: Vec3,
    game: &GameDefinition,
) -> Option<Vec3> {
    let filter = SpatialQueryFilter::from_excluded_entities([owner]);
    [
        normal,
        -normal,
        Vec3::X,
        Vec3::NEG_X,
        Vec3::Z,
        Vec3::NEG_Z,
        Vec3::new(1., 0., 1.).normalize(),
        Vec3::new(-1., 0., 1.).normalize(),
        Vec3::new(1., 0., -1.).normalize(),
        Vec3::new(-1., 0., -1.).normalize(),
    ]
    .into_iter()
    .find_map(|offset| {
        let probe = position + offset * 1.25;
        let floor = query.cast_ray(probe + Vec3::Y * 1.5, Dir3::NEG_Y, 4., false, &filter)?;
        let center = probe + Vec3::Y * (1.5 - floor.distance + player.height * 0.5 + 0.03);
        let eye = center + Vec3::Y * (game.tuning.eye_height - player.height * 0.5);
        let delta = position - eye;
        let distance = delta.length();
        if distance > 2.
            || query
                .cast_ray(
                    eye,
                    Dir3::new(delta).ok()?,
                    (distance - 0.12).max(0.),
                    true,
                    &filter,
                )
                .is_some()
        {
            None
        } else {
            Some(center)
        }
    })
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
        &PerkInventory,
        &WeaponInventory,
        Option<&crate::perks::PerkDrinking>,
    )>,
    focus: Res<crate::interactions::InteractionFocus>,
    query: SpatialQuery,
    targets: Query<(&crate::interactions::Usable, &Transform), Without<Player>>,
    enemies: Query<Entity, With<Zombie>>,
    mut damage: ResMut<crate::damage::DamageQueue>,
    evidence: Res<crate::perks::PerkEvidence>,
    solo: Res<crate::solo_revive::SoloReviveEvidence>,
) {
    let (Some(v), Some(game), Ok((owner, mut player, mut transform, health, perks, inv, drink))) =
        (v, game, players.single_mut())
    else {
        return;
    };
    if !v.mule_kick {
        return;
    }
    input.clear();
    c.elapsed += time.delta_secs();
    if v.started.elapsed().as_secs() > 180
        || **state == GameState::LoadError
        || (**state == GameState::GameOver && c.phase < 20)
    {
        c.phase = 99;
    }
    let _=std::fs::write(v.output.join("mule-kick-progress.json"),serde_json::json!({"phase":c.phase,"seconds":c.elapsed,"capacity":inv.capacity(),"active":inv.active_slot(),"owned":inv.owned().map(|(s,w)|(s,&w.definition.0,w.magazine)).collect::<Vec<_>>(),"assigned":inv.perk_weapon().map(|w|&w.definition.0),"prompt":focus.prompt,"rejection":format!("{:?}",focus.rejection),"rows":c.rows}).to_string());
    let wall = match c.phase {
        1 => Some("weapon.m14_zm"),
        6 => Some("weapon.mp40_zm"),
        8 | 19 => Some("weapon.870mcs_zm"),
        _ => None,
    };
    let perk = match c.phase {
        2..=4 | 17..=18 => Some("placement.mule_kick"),
        12..=13 => Some("placement.quick_revive"),
        _ => None,
    };
    if **state == GameState::Playing {
        let mut target = None;
        if let Some(wall) = wall {
            if let Some((_,p))=targets.iter().find(|(u,_)|matches!(&u.source.action,crate::origins::InteractionAction::Weapon{weapon,..}if weapon.0==wall)) {
                if let Some(center)=wall_approach(&query,owner,&player,p.translation,p.rotation*Vec3::Z,&game){transform.translation=center;target=Some(p.translation);}
            }
        } else if let Some(perk) = perk {
            let p = game
                .content
                .placements
                .iter()
                .find(|p| p.id.0 == perk)
                .expect("native machine");
            let machine = Vec3::from_array(p.position);
            let rotation = Quat::from_rotation_y(p.yaw);
            transform.translation =
                machine + rotation * Vec3::new(0., player.height * 0.5 + 0.02, 2.2);
            target = Some(machine + Vec3::Y + rotation * Vec3::new(0., 0., 0.8));
        }
        if let Some(target) = target {
            let delta = target
                - (transform.translation
                    + Vec3::Y * (game.tuning.eye_height - player.height * 0.5));
            player.yaw = (-delta.x).atan2(-delta.z) - inv.recoil.x;
            player.pitch = delta.y.atan2(delta.xz().length()) - inv.recoil.y;
            player.velocity = Vec3::ZERO;
            commands
                .entity(owner)
                .insert(Position(transform.translation));
        }
    }
    match c.phase {
        0 if **state == GameState::Playing && c.elapsed > 3. => {
            for enemy in &enemies {
                commands.entity(enemy).despawn();
            }
            session.pending = 0;
            session.intermission = Some(Timer::from_seconds(300., TimerMode::Once));
            session.points = 100000;
            c.phase = 1;
            c.elapsed = 0.;
        }
        1 => {
            input.use_held = c.elapsed > 0.25;
            if inv.owned().count() == 2 && inv.action == WeaponAction::Ready {
                c.normal = ordinary(inv, &game);
                if inv.capacity() != 2 || inv.perk_weapon().is_some() {
                    c.phase = 99;
                    return;
                }
                session.points = 3999;
                c.phase = 2;
                c.elapsed = 0.;
            }
        }
        2 => {
            input.use_held = c.elapsed > 0.25;
            if focus.rejection
                == Some(crate::interactions::InteractionRejection::InsufficientPoints)
            {
                c.insufficient = session.points == 3999 && inv.capacity() == 2;
                c.phase = 3;
                c.elapsed = 0.;
            }
        }
        3 if c.elapsed > 0.2 => {
            session.points = 4000;
            c.phase = 4;
            c.elapsed = 0.;
        }
        4 => {
            input.use_held = true;
            if drink.is_some_and(|d| d.elapsed < 3.2) && inv.capacity() != 2 {
                c.phase = 99;
                return;
            }
            if drink.is_none()
                && perks
                    .owned
                    .contains(&crate::origins::PerkId("perk.mule_kick".into()))
            {
                c.expanded = inv.capacity() == 3
                    && inv.perk_weapon().is_none()
                    && session.points == 0
                    && ordinary(inv, &game) == c.normal;
                c.phase = 5;
                c.elapsed = 0.;
            }
        }
        5 if c.elapsed > 0.2 => {
            session.points = 100000;
            c.phase = 6;
            c.elapsed = 0.;
        }
        6 => {
            input.use_held = c.elapsed > 0.25;
            if inv
                .perk_weapon()
                .is_some_and(|w| w.definition.0 == "weapon.mp40_zm")
                && inv.action == WeaponAction::Ready
            {
                c.assigned = inv.owned().count() == 3
                    && inv.active_slot() == 2
                    && ordinary(inv, &game) == c.normal;
                let row = serde_json::json!({"event":"third_acquired","assigned":inv.perk_weapon().map(|w|&w.definition.0),"ordinary":ordinary(inv,&game),"capacity":inv.capacity()});
                c.rows.push(row);
                c.phase = 7;
                c.elapsed = 0.;
            }
        }
        7 if c.elapsed > 0.2 => {
            c.phase = 8;
            c.elapsed = 0.;
        }
        8 => {
            input.use_held = c.elapsed > 0.25;
            if inv
                .perk_weapon()
                .is_some_and(|w| w.definition.0 == "weapon.870mcs_zm")
                && inv.action == WeaponAction::Ready
            {
                c.replaced = inv.owned().count() == 3
                    && inv.active_slot() == 2
                    && ordinary(inv, &game) == c.normal;
                let row = serde_json::json!({"event":"third_replaced","assigned":inv.perk_weapon().map(|w|&w.definition.0),"ordinary":ordinary(inv,&game)});
                c.rows.push(row);
                c.phase = 9;
                c.elapsed = 0.;
            }
        }
        9 => {
            input.weapon_slot = Some(0);
            if inv.active_slot() == 0 && inv.action == WeaponAction::Ready {
                c.selected = true;
                c.phase = 10;
                c.elapsed = 0.;
            }
        }
        10 => {
            input.cycle_weapon = c.elapsed < 0.1;
            if inv.active_slot() == 1 && inv.action == WeaponAction::Ready {
                c.phase = 11;
                c.elapsed = 0.;
            }
        }
        11 => {
            input.weapon_slot = Some(2);
            if inv.active_slot() == 2 && inv.action == WeaponAction::Ready {
                c.cycled = true;
                if !c.picture {
                    commands
                        .spawn(Screenshot::primary_window())
                        .observe(save_to_disk(v.output.join("third-weapon-selected.png")));
                    c.picture = true;
                    input.fire = true;
                }
                if inv.perk_weapon().is_some_and(|w| w.magazine == 6) {
                    input.fire = true;
                }
                if inv.perk_weapon().is_some_and(|w| w.magazine == 5) {
                    c.fired = true;
                    c.phase = 12;
                    c.elapsed = 0.;
                    session.points = 500;
                }
            }
        }
        12 if c.elapsed > 0.25 => {
            c.phase = 13;
            c.elapsed = 0.;
        }
        13 => {
            input.use_held = true;
            if drink.is_none() && perks.solo_charges == 1 {
                damage.0.push(crate::damage::Impact {
                    attacker: owner,
                    target: owner,
                    source: crate::damage::ImpactSource::Projectile(crate::origins::WeaponId(
                        "weapon.frag_grenade_zm".into(),
                    )),
                    category: crate::origins::DamageCategory::Explosive,
                    region: crate::weapons::HitRegion::Torso,
                    amount: 300,
                });
                c.phase = 14;
                c.elapsed = 0.;
            }
        }
        14 => {
            if health.state() == LifeState::Downed {
                c.downed = perks.owned.is_empty()
                    && inv.capacity() == 2
                    && inv.perk_weapon().is_none()
                    && solo
                        .events
                        .iter()
                        .any(|e| e["lost_third_weapon"] == "weapon.870mcs_zm");
            }
            if health.state() == LifeState::Standing && c.elapsed > 1. {
                c.restored = inv.capacity() == 2
                    && inv.perk_weapon().is_none()
                    && inv.active_slot() == 0
                    && ordinary(inv, &game) == c.normal
                    && inv.reserve(
                        game.content
                            .weapons
                            .iter()
                            .find(|w| w.id.0 == "weapon.870mcs_zm")
                            .unwrap(),
                    ) == 0;
                let row = serde_json::json!({"event":"restored","ordinary":ordinary(inv,&game),"assigned":inv.perk_weapon().map(|w|&w.definition.0),"active":inv.active_slot(),"restored":c.restored});
                c.rows.push(row);
                c.phase = 15;
                c.elapsed = 0.;
            }
        }
        15 => {
            input.weapon_slot = Some(2);
            if c.elapsed > 0.5 {
                if inv.active_slot() != 0 || inv.owned().count() != 2 {
                    c.phase = 99;
                    return;
                }
                c.phase = 17;
                c.elapsed = 0.;
            }
        }
        17 => {
            session.points = 4000;
            if c.elapsed > 0.25 {
                c.phase = 18;
                c.elapsed = 0.;
            }
        }
        18 => {
            input.use_held = true;
            if drink.is_none()
                && inv.capacity() == 3
                && perks
                    .owned
                    .contains(&crate::origins::PerkId("perk.mule_kick".into()))
            {
                if inv.perk_weapon().is_some() || ordinary(inv, &game) != c.normal {
                    c.phase = 99;
                    return;
                }
                session.points = 100000;
                c.phase = 19;
                c.elapsed = 0.;
            }
        }
        19 => {
            input.use_held = c.elapsed > 0.25;
            if inv
                .perk_weapon()
                .is_some_and(|w| w.definition.0 == "weapon.870mcs_zm")
                && inv.action == WeaponAction::Ready
            {
                let w = game
                    .content
                    .weapons
                    .iter()
                    .find(|w| w.id.0 == "weapon.870mcs_zm")
                    .unwrap();
                c.reacquired = inv.perk_weapon().unwrap().magazine == w.magazine
                    && inv.reserve(w) == w.mechanics.starting_reserve
                    && ordinary(inv, &game) == c.normal;
                let passed = c.insufficient
                    && c.expanded
                    && c.assigned
                    && c.replaced
                    && c.selected
                    && c.cycled
                    && c.fired
                    && c.downed
                    && c.restored
                    && c.reacquired;
                let _=std::fs::write(v.output.join("mule-kick.json"),serde_json::json!({"passed":passed,"scope":"real held-F native Mule Kick/Quick Revive and M14/MP40/Remington wall purchases, direct/cyclic selection and third-weapon firing; controlled travel/budgets/empty population and resolved downing damage","rows":c.rows,"perk_events":evidence.events,"solo_events":solo.events,"insufficient":c.insufficient,"expanded":c.expanded,"assigned":c.assigned,"replaced":c.replaced,"selected":c.selected,"cycled":c.cycled,"fired":c.fired,"downed":c.downed,"restored":c.restored,"reacquired":c.reacquired}).to_string());
                if passed {
                    damage.0.push(crate::damage::Impact {
                        attacker: owner,
                        target: owner,
                        source: crate::damage::ImpactSource::Projectile(crate::origins::WeaponId(
                            "weapon.frag_grenade_zm".into(),
                        )),
                        category: crate::origins::DamageCategory::Explosive,
                        region: crate::weapons::HitRegion::Torso,
                        amount: 300,
                    });
                    c.phase = 20;
                } else {
                    c.phase = 99;
                }
                c.elapsed = 0.;
            }
        }
        20 if **state == GameState::GameOver => {
            commands.write_message(crate::input::RestartGame);
            c.phase = 21;
            c.elapsed = 0.;
        }
        21 if **state == GameState::Playing && c.elapsed > 0.5 => {
            let passed = inv.capacity() == 2
                && inv.owned().count() == 1
                && inv.perk_weapon().is_none()
                && perks.owned.is_empty()
                && drink.is_none();
            let _ = std::fs::write(
                v.output.join("mule-kick-restart.json"),
                serde_json::json!({"passed":passed}).to_string(),
            );
            c.phase = if passed { 22 } else { 99 };
            c.elapsed = 0.;
        }
        22 if c.elapsed > 0.5 => {
            commands.write_message(AppExit::Success);
        }
        99 => {
            let _=std::fs::write(v.output.join("mule-kick-failure.json"),serde_json::json!({"passed":false,"phase":c.phase,"rows":c.rows,"events":evidence.events,"solo":solo.events}).to_string());
            commands.write_message(AppExit::error());
        }
        _ => {}
    }
}

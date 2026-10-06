//! Controlled inventory branches across actual application round transitions.
use super::Verification;
use crate::{
    assets::GameDefinition,
    equipment::{EquipmentEvidence, GrenadeInventory, TacticalInventory},
    gameplay::{Session, Zombie},
    input::GameInput,
    mines::{MineEvidence, MineInventory},
    player::Player,
    weapons::WeaponInventory,
};
use bevy::prelude::*;

#[derive(Resource, Default)]
struct Coverage {
    index: usize,
    waiting: bool,
    before_awards: u32,
    rounds_before: u32,
    primary: Vec<(String, u32, u32)>,
    records: Vec<serde_json::Value>,
    elapsed: f32,
}
pub fn install(app: &mut App) {
    app.init_resource::<Coverage>().add_systems(
        PreUpdate,
        drive.after(super::drive).before(crate::player::look),
    );
}
fn primary_ammo(
    inventory: &WeaponInventory,
    definition: &GameDefinition,
) -> Vec<(String, u32, u32)> {
    inventory
        .owned()
        .map(|(_, w)| {
            let tuning = definition
                .content
                .weapons
                .iter()
                .find(|v| v.id == w.definition)
                .unwrap();
            (
                w.definition.0.clone(),
                w.magazine,
                inventory.reserve(tuning),
            )
        })
        .collect()
}
fn drive(
    mut commands: Commands,
    verification: Option<ResMut<Verification>>,
    definition: Option<Res<GameDefinition>>,
    time: Res<Time>,
    mut coverage: ResMut<Coverage>,
    mut input: ResMut<GameInput>,
    mut session: ResMut<Session>,
    evidence: Res<EquipmentEvidence>,
    mines: Res<MineEvidence>,
    rounds: Res<crate::rounds::RoundEvidence>,
    enemies: Query<Entity, With<Zombie>>,
    mut players: Query<
        (
            &mut GrenadeInventory,
            &mut TacticalInventory,
            &mut MineInventory,
            &WeaponInventory,
        ),
        With<Player>,
    >,
    state: Res<State<crate::GameState>>,
    mut next: ResMut<NextState<crate::GameState>>,
) {
    let (Some(mut verification), Some(definition)) = (verification, definition) else {
        return;
    };
    let Ok((mut lethal, mut tactical, mut mine, primary)) = players.single_mut() else {
        return;
    };
    if verification.stage == 3
        && coverage.index == 11
        && *state == crate::GameState::Playing
        && session.round == 1
    {
        let cleared = lethal.definition.0 == "weapon.frag_grenade_zm"
            && lethal.count == 4
            && tactical.0.is_none()
            && mine.definition.is_none()
            && mine.count == 0;
        let path = verification.output.join("equipment-refills.json");
        if let Ok(text) = std::fs::read_to_string(&path)
            && let Ok(mut receipt) = serde_json::from_str::<serde_json::Value>(&text)
        {
            receipt["restart_clear"] = cleared.into();
            let _ = std::fs::write(path, receipt.to_string());
        }
        if !cleared {
            commands.write_message(AppExit::error());
        }
        return;
    }
    if verification.stage != 4 {
        return;
    }
    input.clear();
    coverage.elapsed += time.delta_secs();
    for entity in &enemies {
        commands.entity(entity).despawn();
    }
    session.pending = 0;
    if coverage.elapsed > 30. {
        let _ = std::fs::write(
            verification.output.join("equipment-refills.json"),
            serde_json::json!({"passed":false,"index":coverage.index,"records":coverage.records})
                .to_string(),
        );
        commands.write_message(AppExit::error());
        return;
    }
    if !coverage.waiting {
        if coverage.index == 11 {
            let passed = coverage.records.len() == 11
                && coverage.records.iter().all(|r| r["passed"] == true);
            let receipt = serde_json::json!({"passed":passed,"scope":"normal Round 1 combat then controlled count branches and accelerated empty intermissions; actual round-start consumers; no primary ammo grant","records":coverage.records,"round_awards":evidence.round_awards,"mine_refills":mines.replenished});
            let _ = std::fs::write(
                verification.output.join("equipment-refills.json"),
                receipt.to_string(),
            );
            if !passed {
                commands.write_message(AppExit::error());
                return;
            }
            verification.equipment_refills_exercised = true;
            verification.stage = 1;
            verification.playing = 0.;
            next.set(crate::GameState::Paused);
            return;
        }
        let id = if coverage.index < 5 {
            "weapon.frag_grenade_zm"
        } else {
            "weapon.sticky_grenade_zm"
        };
        let weapon = definition
            .content
            .weapons
            .iter()
            .find(|w| w.id.0 == id)
            .unwrap();
        *lethal = GrenadeInventory::new(weapon.id.clone(), weapon.throwable.as_ref().unwrap());
        lethal.count = coverage.index as u32 % 5;
        if coverage.index >= 5 && tactical.0.is_none() {
            let monkey = definition
                .content
                .weapons
                .iter()
                .find(|w| w.id.0 == "weapon.cymbal_monkey_zm")
                .unwrap();
            tactical.receive(monkey).unwrap();
            tactical.0.as_mut().unwrap().count = 1;
            let claymore = definition
                .content
                .weapons
                .iter()
                .find(|w| w.id.0 == "weapon.claymore_zm")
                .unwrap();
            mine.receive(claymore).unwrap();
        }
        mine.count = 0;
        if coverage.index == 10 {
            session.round = 255;
        }
        coverage.primary = primary_ammo(primary, &definition);
        coverage.before_awards = evidence.round_awards;
        coverage.rounds_before = rounds.transitions;
        coverage.waiting = true;
        session.intermission = Some(Timer::from_seconds(0.05, TimerMode::Once));
    } else if rounds.transitions > coverage.rounds_before {
        let before = coverage.index as u32 % 5;
        let expected = (before + 2).min(4);
        let tactical_ok = if coverage.index < 5 {
            tactical.0.is_none()
        } else {
            tactical.0.as_ref().is_some_and(|i| i.count == 1)
        };
        let mine_ok = if coverage.index < 5 {
            mine.definition.is_none() && mine.count == 0
        } else {
            mine.count == 2
        };
        let primary_ok = coverage.primary == primary_ammo(primary, &definition);
        let passed = lethal.count == expected
            && tactical_ok
            && mine_ok
            && primary_ok
            && evidence.round_awards == coverage.before_awards + 1;
        coverage.records.push(serde_json::json!({"passed":passed,"round":session.round,"weapon":lethal.definition.0,"before":before,"after":lethal.count,"expected":expected,"tactical":tactical.0.as_ref().map(|i|i.count),"mines":mine.count,"primary_ammo_unchanged":primary_ok}));
        session.intermission = Some(Timer::from_seconds(300., TimerMode::Once));
        coverage.index += 1;
        coverage.waiting = false;
    }
}

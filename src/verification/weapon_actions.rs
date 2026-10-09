use crate::{
    GameState,
    assets::{GameDefinition, LaunchOptions},
    gameplay::{GameStep, Session, Zombie},
    player::{Player, TickActions},
    weapons::{InventoryError, WeaponAction, WeaponInventory},
};
use bevy::prelude::*;
use std::time::Instant;

#[derive(Resource)]
struct Run {
    started: Instant,
    phase: u8,
    case: usize,
    age: f32,
    shots: u32,
    reserve: u32,
    magazine: u32,
    audio: u32,
    sinks: u32,
    completions: Vec<String>,
    rows: Vec<serde_json::Value>,
}
pub(super) fn install(app: &mut App) {
    app.insert_resource(Run {
        started: Instant::now(),
        phase: 0,
        case: 0,
        age: 0.,
        shots: 0,
        reserve: 0,
        magazine: 0,
        audio: 0,
        sinks: 0,
        completions: vec![],
        rows: vec![],
    })
    .add_systems(
        FixedUpdate,
        drive
            .after(GameStep::Movement)
            .before(GameStep::Interactions),
    )
    .add_systems(FixedUpdate, observe.after(crate::gameplay::combat))
    .add_systems(Update, finish);
}
fn record(run: &mut Run, case: &str, passed: bool) {
    run.rows
        .push(serde_json::json!({"case":case,"variant":run.case,"passed":passed}));
}
fn advance(run: &mut Run, phase: u8) {
    run.phase = phase;
    run.age = 0.;
}
fn observe(mut run: ResMut<Run>, mut events: MessageReader<crate::perk_effects::ReloadEvent>) {
    for event in events.read() {
        if event.start.is_none() {
            run.completions.push(event.weapon.0.clone());
        }
    }
}
fn drive(
    mut commands: Commands,
    state: Res<State<GameState>>,
    game: Option<Res<GameDefinition>>,
    time: Res<Time<Fixed>>,
    mut run: ResMut<Run>,
    mut actions: ResMut<TickActions>,
    mut input: ResMut<crate::input::GameInput>,
    mut session: ResMut<Session>,
    mut players: Query<(&mut WeaponInventory, &mut crate::player_health::Recovery), With<Player>>,
    enemies: Query<Entity, With<Zombie>>,
    audio: Res<crate::audio::AudioEvidence>,
) {
    *actions = TickActions::default();
    input.clear();
    let (Some(game), Ok((mut inventory, mut recovery))) = (game, players.single_mut()) else {
        return;
    };
    if **state != GameState::Playing {
        return;
    }
    recovery.immunity = 30.;
    session.pending = 0;
    session.intermission = Some(Timer::from_seconds(300., TimerMode::Once));
    for entity in &enemies {
        commands.entity(entity).despawn();
    }
    run.age += time.delta_secs();
    let find = |id: &str| game.content.weapons.iter().find(|w| w.id.0 == id).unwrap();
    let pistol = find("weapon.c96_zm");
    let replacement = find("weapon.mp40_zm");
    let other = find("weapon.fiveseven_zm");
    let current = find(match run.case {
        1 => "weapon.870mcs_zm",
        2 => "weapon.fivesevendw_zm",
        _ => "weapon.c96_zm",
    });
    let alias = &game.audio.bindings["weapon.c96_zm.dry_fire"];
    match run.phase {
        0 if run.age > 2. => {
            *inventory = WeaponInventory::temporary(pistol, 0);
            run.shots = session.shots;
            advance(&mut run, 1);
        }
        1 => {
            if inventory.active().is_some_and(|w| w.magazine == 0) {
                let passed = session.shots - run.shots == pistol.magazine;
                record(&mut run, "native magazine exhausted through combat", passed);
                run.shots = session.shots;
                run.audio = audio.events.get(alias).copied().unwrap_or(0);
                run.sinks = audio.sinks.get(alias).copied().unwrap_or(0);
                advance(&mut run, 2);
            } else if run.age > 0.2 {
                actions.fire = true;
                run.age = 0.;
            }
        }
        2 if run.age > 0.3 => {
            actions.fire = true;
            advance(&mut run, 3);
        }
        3 if run.age > 1. => {
            let silent_shot = session.shots == run.shots
                && inventory.active().is_some_and(|w| w.magazine == 0)
                && inventory.reserve(pistol) == 0;
            record(
                &mut run,
                "empty trigger consumes no ammo and emits no shot",
                silent_shot,
            );
            let heard = audio.events.get(alias).copied().unwrap_or(0) == run.audio + 1
                && audio.sinks.get(alias).copied().unwrap_or(0) == run.sinks + 1;
            record(
                &mut run,
                "one native dry fire request reaches audio sink",
                heard,
            );
            advance(&mut run, 4);
        }
        4 => {
            actions.fire_held = true;
            if run.age > 0.6 {
                let passed = session.shots == run.shots
                    && audio.events.get(alias).copied().unwrap_or(0) == run.audio + 1;
                record(
                    &mut run,
                    "held semi automatic empty trigger does not repeat",
                    passed,
                );
                advance(&mut run, 10);
            }
        }
        10 => {
            *inventory = WeaponInventory::temporary(current, 4);
            inventory.grant(replacement).unwrap();
            run.shots = session.shots;
            run.completions.clear();
            advance(&mut run, 11);
        }
        11 => {
            let count = if run.case == 0 { 1 } else { 2 };
            if session.shots - run.shots == count {
                actions.reload = true;
                advance(&mut run, 12);
            } else if run.age > 0.2 {
                actions.fire = true;
                run.age = 0.;
            }
        }
        12 if run.age > 0.1 => {
            let active = inventory.active_slot();
            let action = inventory.action;
            let right = inventory.hand_reload_timing(false);
            let left = inventory.hand_reload_timing(true);
            let reloading =
                inventory.reload_timing(current).is_some() || right.is_some() && left.is_some();
            let rejected = matches!(
                inventory.replace(active, replacement),
                Err(InventoryError::AlreadyOwned)
            );
            let preserved = rejected
                && inventory.action == action
                && inventory.hand_reload_timing(false) == right
                && inventory.hand_reload_timing(true) == left;
            record(
                &mut run,
                "failed replacement preserves active reload",
                reloading && preserved,
            );
            let replaced = inventory.replace(1, other).is_ok();
            let preserved = inventory.action == action
                && inventory.hand_reload_timing(false) == right
                && inventory.hand_reload_timing(true) == left;
            record(
                &mut run,
                "inactive replacement preserves active reload",
                replaced && preserved,
            );
            advance(&mut run, 13);
        }
        13 if run.case != 1 || run.age > current.mechanics.reload_start_insert => {
            run.reserve = inventory.reserve(current);
            run.magazine = inventory.active().unwrap().magazine;
            if run.case == 1 {
                let inserted = run.magazine == current.magazine - 1
                    && run.reserve == current.magazine * 4 - 1
                    && matches!(inventory.action, WeaponAction::ShellReload { .. });
                record(
                    &mut run,
                    "native shell insertion occurs before replacement",
                    inserted,
                );
            }
            let received = inventory.receive(replacement).is_ok();
            let cancelled = matches!(inventory.action, WeaponAction::Equipping { .. })
                && inventory.hand_reload_timing(false).is_none()
                && inventory.hand_reload_timing(true).is_none();
            record(
                &mut run,
                "full inventory receipt replaces and cancels reload",
                received && cancelled,
            );
            run.completions.clear();
            advance(&mut run, 14);
        }
        14 if run.age > current.mechanics.empty_reload.max(current.reload_seconds) + 1. => {
            let passed = inventory.active().is_some_and(|w| {
                w.definition == replacement.id && w.magazine == replacement.magazine
            }) && inventory.reserve(replacement)
                == replacement.mechanics.starting_reserve
                && inventory.reserve(current) == run.reserve
                && run.completions.is_empty();
            record(
                &mut run,
                "cancelled reload causes no delayed ammo transfer or completion",
                passed,
            );
            if run.case < 2 {
                run.case += 1;
                advance(&mut run, 10);
            } else {
                advance(&mut run, 20);
            }
        }
        20 => {
            *inventory = WeaponInventory::temporary(pistol, 3);
            run.shots = session.shots;
            advance(&mut run, 21);
        }
        21 => {
            if session.shots > run.shots {
                actions.reload = true;
                advance(&mut run, 22);
            } else if run.age > 0.2 {
                actions.fire = true;
            }
        }
        22 if run.age > 0.1 => {
            let reloading = inventory.reload_timing(pistol).is_some();
            let received = inventory.receive(replacement).is_ok();
            record(
                &mut run,
                "free slot receipt cancels previous weapon reload",
                reloading && received && inventory.active_slot() == 1,
            );
            run.completions.clear();
            advance(&mut run, 23);
        }
        23 if run.age > pistol.reload_seconds + 1. => {
            let passed = inventory
                .owned()
                .any(|(_, w)| w.definition == pistol.id && w.magazine == pistol.magazine - 1)
                && inventory.reserve(pistol) == pistol.magazine * 3
                && run.completions.is_empty();
            record(
                &mut run,
                "inactive cancelled magazine and reserve remain unchanged",
                passed,
            );
            advance(&mut run, 30);
        }
        30 => {
            let base = find("weapon.ak74u_zm");
            let peer = find("weapon.ak74u_extclip_zm");
            *inventory = WeaponInventory::default();
            inventory.grant(base).unwrap();
            inventory.grant(peer).unwrap();
            let shared = base.ammo_group == peer.ammo_group
                && inventory.reserve(base) == inventory.reserve(peer);
            record(
                &mut run,
                "native attachment variants share one reserve pool",
                shared,
            );
            run.reserve = inventory.reserve(base);
            run.shots = session.shots;
            advance(&mut run, 31);
        }
        31 => {
            if session.shots > run.shots {
                actions.reload = true;
                advance(&mut run, 32);
            } else if run.age > 0.2 {
                actions.fire = true;
            }
        }
        32 if run.age > find("weapon.ak74u_zm").reload_seconds + 0.2 => {
            let base = find("weapon.ak74u_zm");
            let peer = find("weapon.ak74u_extclip_zm");
            let shared = inventory.reserve(base) == run.reserve - 1
                && inventory.reserve(peer) == run.reserve - 1
                && inventory.owned().all(|(_, w)| {
                    w.magazine
                        == if w.definition == base.id {
                            base.magazine
                        } else {
                            peer.magazine
                        }
                });
            record(
                &mut run,
                "reload debits shared reserve once and keeps clips independent",
                shared,
            );
            let refilled = inventory.refill_weapon(peer);
            let shared = inventory.reserve(base) == peer.reserve.max(base.reserve)
                && inventory.reserve(base) == inventory.reserve(peer);
            record(
                &mut run,
                "peer ammo refill updates the same bounded reserve",
                refilled && shared,
            );
            advance(&mut run, 99);
        }
        _ => {}
    }
}
fn finish(
    mut commands: Commands,
    options: Res<LaunchOptions>,
    state: Res<State<GameState>>,
    mut run: ResMut<Run>,
) {
    if run.phase == 100 {
        return;
    }
    if run.started.elapsed().as_secs() > 180 || **state == GameState::LoadError {
        record(&mut run, "loaded and finished before deadline", false);
        run.phase = 99;
    }
    if run.phase != 99 {
        return;
    }
    let passed = run.rows.len() == 22 && run.rows.iter().all(|r| r["passed"] == true);
    let output = options.verify.as_ref().unwrap();
    let receipt = serde_json::json!({"passed":passed,"checks":run.rows});
    let saved = !output.starts_with(env!("CARGO_MANIFEST_DIR"))
        && std::fs::create_dir_all(output).is_ok()
        && std::fs::write(output.join("weapon-actions.json"), receipt.to_string()).is_ok();
    commands.write_message(if passed && saved {
        AppExit::Success
    } else {
        AppExit::error()
    });
    run.phase = 100;
}

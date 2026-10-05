#![forbid(unsafe_code)]

use std::time::Duration;

use bevy::{app::ScheduleRunnerPlugin, prelude::*, time::TimeUpdateStrategy};
use bo2_gameplay::*;

type DemoResult<T> = Result<T, Box<dyn std::error::Error>>;

fn step(
    app: &mut App,
    id: u64,
    target: Entity,
    command: LifeCommand,
) -> DemoResult<LifeCommandResult> {
    app.world_mut()
        .resource_mut::<LifeCommandQueue>()
        .submit(LifeCommandRequest {
            id,
            target,
            command,
        })?;
    app.update();
    let results: Vec<_> = app
        .world_mut()
        .resource_mut::<LifeCommandResults>()
        .drain()
        .collect();
    if results.len() != 1 {
        return Err("expected one lifecycle result".into());
    }
    let result = results[0];
    if result.id != id || result.target != target {
        return Err("request order mismatch".into());
    }
    println!("request {}: {:?}", id, result.result);
    Ok(result)
}

fn token(result: LifeCommandResult) -> DemoResult<ReviveToken> {
    match result.result {
        Ok(LifeCommandOutcome::ReviveBegan(token)) => Ok(token),
        _ => Err("revive did not begin".into()),
    }
}

fn main() -> DemoResult<()> {
    let mut app = bo2_core::headless_app(ScheduleRunnerPlugin::run_once());
    app.add_plugins(LifeStatePlugin);
    app.insert_resource(TimeUpdateStrategy::FixedTimesteps(0));
    app.update();
    app.insert_resource(TimeUpdateStrategy::FixedTimesteps(1));
    let human = app
        .world_mut()
        .spawn(bo2_origins::unperked_human(
            100,
            PlayerLethalPolicy::DownWithEligibleTeammate,
        )?)
        .id();
    let teammate = app
        .world_mut()
        .spawn(bo2_origins::unperked_human(
            100,
            PlayerLethalPolicy::DownWithEligibleTeammate,
        )?)
        .id();
    let actor = app
        .world_mut()
        .spawn(bo2_origins::fallback_ordinary_actor(150)?)
        .id();
    let damage = |amount| ResolvedDamage::new(amount).map(LifeCommand::Damage);
    let restore = bo2_origins::ordinary_coop_revive_restoration()?;
    let mut results = vec![
        step(&mut app, 0, human, damage(60)?)?,
        step(&mut app, 1, human, damage(40)?)?,
    ];
    let episode = app
        .world()
        .get::<Vitality>(human)
        .and_then(|v| v.downed_episode())
        .ok_or("player not downed")?;
    if episode.number() != 1 {
        return Err("unexpected first episode".into());
    }
    let old_expiry = app
        .world()
        .get::<Vitality>(human)
        .ok_or("missing player")?
        .downed_expiry(human)?;
    let begin = step(
        &mut app,
        2,
        human,
        LifeCommand::BeginRevive {
            reviver: teammate,
            episode,
        },
    )?;
    let old = token(begin)?;
    results.push(begin);
    results.push(step(&mut app, 3, human, LifeCommand::CancelRevive(old))?);
    if app.world().get::<Vitality>(human).map(|v| v.state()) != Some(LifeState::Downed) {
        return Err("cancel cleared downed state".into());
    }
    results.push(step(
        &mut app,
        4,
        human,
        LifeCommand::CompleteRevive {
            token: old,
            restore,
        },
    )?);
    let begin = step(
        &mut app,
        5,
        human,
        LifeCommand::BeginRevive {
            reviver: teammate,
            episode,
        },
    )?;
    let fresh = token(begin)?;
    if fresh.attempt() != 2 || fresh.reviver() != teammate {
        return Err("unexpected new attempt".into());
    }
    results.push(begin);
    results.push(step(
        &mut app,
        6,
        human,
        LifeCommand::CompleteRevive {
            token: fresh,
            restore,
        },
    )?);
    let restored = app.world().get::<Vitality>(human).ok_or("missing player")?;
    if (restored.current(), restored.maximum(), restored.state())
        != (Some(100), 100, LifeState::Standing)
    {
        return Err("revive restoration mismatch".into());
    }
    results.push(step(&mut app, 7, human, damage(60)?)?);
    results.push(step(&mut app, 8, human, damage(40)?)?);
    let next_episode = app
        .world()
        .get::<Vitality>(human)
        .and_then(|v| v.downed_episode())
        .ok_or("player not redowned")?;
    if next_episode.number() != 2 {
        return Err("redown reused episode".into());
    }
    results.push(step(
        &mut app,
        9,
        human,
        LifeCommand::CompleteRevive {
            token: fresh,
            restore,
        },
    )?);
    results.push(step(&mut app, 10, actor, damage(100)?)?);
    results.push(step(&mut app, 11, actor, damage(50)?)?);
    results.push(step(&mut app, 12, actor, damage(1)?)?);
    results.push(step(
        &mut app,
        13,
        human,
        LifeCommand::ExpireDowned(old_expiry),
    )?);
    let expiry = app
        .world()
        .get::<Vitality>(human)
        .ok_or("missing player")?
        .downed_expiry(human)?;
    let begin = step(
        &mut app,
        14,
        human,
        LifeCommand::BeginRevive {
            reviver: teammate,
            episode: next_episode,
        },
    )?;
    let terminal_attempt = token(begin)?;
    results.push(begin);
    results.push(step(
        &mut app,
        15,
        human,
        LifeCommand::ExpireDowned(expiry),
    )?);
    results.push(step(
        &mut app,
        16,
        human,
        LifeCommand::CancelRevive(terminal_attempt),
    )?);
    results.push(step(
        &mut app,
        17,
        human,
        LifeCommand::ExpireDowned(expiry),
    )?);
    results.push(step(
        &mut app,
        18,
        human,
        LifeCommand::CompleteRevive {
            token: terminal_attempt,
            restore,
        },
    )?);
    results.push(step(&mut app, 19, human, damage(100)?)?);
    results.push(step(
        &mut app,
        20,
        human,
        LifeCommand::ExpireDowned(expiry),
    )?);
    let expected = [
        Ok(LifeCommandOutcome::Damage(DamageOutcome::Hurt {
            before: 100,
            after: 40,
        })),
        Ok(LifeCommandOutcome::Damage(DamageOutcome::Downed {
            before: 40,
            after: 1,
            episode,
        })),
        Ok(LifeCommandOutcome::ReviveBegan(old)),
        Ok(LifeCommandOutcome::ReviveCanceled),
        Err(LifeStateError::StaleAttempt),
        Ok(LifeCommandOutcome::ReviveBegan(fresh)),
        Ok(LifeCommandOutcome::Revived {
            current: 100,
            maximum: 100,
        }),
        Ok(LifeCommandOutcome::Damage(DamageOutcome::Hurt {
            before: 100,
            after: 40,
        })),
        Ok(LifeCommandOutcome::Damage(DamageOutcome::Downed {
            before: 40,
            after: 1,
            episode: next_episode,
        })),
        Err(LifeStateError::StaleEpisode),
        Ok(LifeCommandOutcome::Damage(DamageOutcome::Hurt {
            before: 150,
            after: 50,
        })),
        Ok(LifeCommandOutcome::Damage(DamageOutcome::ActorDied {
            subtraction: ActorSubtraction {
                before: 50,
                amount: 50,
                after: 0,
            },
        })),
        Ok(LifeCommandOutcome::Damage(DamageOutcome::Ignored {
            state: LifeState::ActorDead,
        })),
        Err(LifeStateError::StaleEpisode),
        Ok(LifeCommandOutcome::ReviveBegan(terminal_attempt)),
        Ok(LifeCommandOutcome::Expiry(ExpiryOutcome::Deferred(expiry))),
        Ok(LifeCommandOutcome::ReviveCanceled),
        Ok(LifeCommandOutcome::Expiry(ExpiryOutcome::Terminated {
            current: 1,
            maximum: 100,
        })),
        Err(LifeStateError::NotDowned),
        Ok(LifeCommandOutcome::Damage(DamageOutcome::Ignored {
            state: LifeState::HumanTerminal,
        })),
        Ok(LifeCommandOutcome::Expiry(ExpiryOutcome::Ignored)),
    ];
    if results.iter().map(|r| r.result).collect::<Vec<_>>() != expected
        || app
            .world()
            .get::<Vitality>(actor)
            .map(|v| (v.state(), v.current(), v.actor_subtraction()))
            != Some((
                LifeState::ActorDead,
                None,
                Some(ActorSubtraction {
                    before: 50,
                    amount: 50,
                    after: 0,
                }),
            ))
        || app
            .world()
            .get::<Vitality>(human)
            .map(|v| (v.state(), v.current(), v.maximum()))
            != Some((LifeState::HumanTerminal, Some(1), 100))
    {
        return Err("ordered lifecycle verification failed".into());
    }
    println!(
        "PASS: down/cancel/restart/revive100/redown; stale inputs rejected; expiry deferred then terminal after cancel; positive terminal health preserved; actor dead with unknown final health and explicit signed pre-notification subtraction. Fidelity APPROXIMATE: ordinary positive player subtraction statically qualified; actor direct subtraction statically qualified before notifications; final callback health unknown; active native dispatch unqualified; scoped live player100->40 agrees with admitted60; full/coop/actor comparisons incomplete; revive100 combines Steam script and qualified Plutonium native evidence; terminal network-frame interval collapsed; final spectator health unknown; coop comparison pending. Initial health and teammate/interaction/expiry timing are admitted demonstration inputs; no raw revive interaction."
    );
    timed_bleedout_demo()?;
    Ok(())
}

#[derive(Resource)]
struct ClockDemo {
    target: Entity,
    next: u64,
    receipts: Vec<LifeCommandResult>,
    error: Option<String>,
}

// One explicitly selected target; assembly supplies IDs, never query iteration.
fn produce_clock(
    time: Res<Time<Fixed>>,
    mut targets: Query<(&Vitality, &mut BleedoutClock)>,
    mut queue: ResMut<LifeCommandQueue>,
    mut demo: ResMut<ClockDemo>,
) {
    if demo.error.is_some() {
        return;
    }
    let target = demo.target;
    let Ok((vitality, mut clock)) = targets.get_mut(target) else {
        demo.error = Some("missing timed target".into());
        return;
    };
    let due = match clock.tick(target, Some(*vitality), time.delta()) {
        Ok(due) => due,
        Err(error) => {
            demo.error = Some(error.to_string());
            return;
        }
    };
    if due.is_some() {
        let Some(next) = demo.next.checked_add(1) else {
            demo.error = Some("demo request ID exhausted".into());
            return;
        };
        if let Err(error) = clock.enqueue(&mut queue, demo.next) {
            demo.error = Some(error.to_string());
            return;
        }
        demo.next = next;
    }
}

fn acknowledge_clock(
    mut clocks: Query<&mut BleedoutClock>,
    mut results: ResMut<LifeCommandResults>,
    mut demo: ResMut<ClockDemo>,
) {
    for receipt in results.drain() {
        match clocks.get_mut(demo.target) {
            Ok(mut clock) if clock.in_flight() == Some(receipt.id) => {
                if let Err(error) = clock.acknowledge(receipt) {
                    demo.error = Some(error.to_string());
                }
            }
            Ok(_) => {}
            Err(_) => demo.error = Some("missing timed clock".into()),
        }
        demo.receipts.push(receipt);
    }
}

fn timed_bleedout_demo() -> DemoResult<()> {
    let mut app = bo2_core::headless_app(ScheduleRunnerPlugin::run_once());
    app.add_plugins(LifeStatePlugin);
    app.insert_resource(Time::<Fixed>::from_duration(Duration::from_millis(100)));
    app.insert_resource(TimeUpdateStrategy::FixedTimesteps(0));
    app.update();
    let mut vitality =
        bo2_origins::unperked_human(100, PlayerLethalPolicy::DownWithEligibleTeammate)?;
    vitality.apply(ResolvedDamage::new(100)?)?;
    let target = app.world_mut().spawn(vitality).id();
    let teammate = app
        .world_mut()
        .spawn(bo2_origins::unperked_human(
            100,
            PlayerLethalPolicy::DownWithEligibleTeammate,
        )?)
        .id();
    let clock = BleedoutClock::new(
        target,
        vitality,
        bo2_origins::ordinary_coop_bleedout_timing()?,
    )?;
    let expiry = clock.expiry();
    app.world_mut().entity_mut(target).insert(clock);
    app.insert_resource(ClockDemo {
        target,
        next: 1,
        receipts: Vec::new(),
        error: None,
    });
    app.add_systems(FixedUpdate, produce_clock.before(ApplyLifeCommands));
    app.add_systems(FixedUpdate, acknowledge_clock.after(ApplyLifeCommands));
    app.world_mut()
        .resource_mut::<LifeCommandQueue>()
        .submit(LifeCommandRequest {
            id: 0,
            target,
            command: LifeCommand::BeginRevive {
                reviver: teammate,
                episode: expiry.episode(),
            },
        })?;
    app.insert_resource(TimeUpdateStrategy::FixedTimesteps(1));
    for _ in 0..449 {
        app.update();
    }
    if app.world().resource::<ClockDemo>().receipts.len() != 1 {
        return Err("expiry occurred before45 seconds".into());
    }
    app.update();
    let receipts = &app.world().resource::<ClockDemo>().receipts;
    let token = receipts
        .first()
        .copied()
        .ok_or("missing timed begin")
        .and_then(|r| {
            if let Ok(LifeCommandOutcome::ReviveBegan(token)) = r.result {
                Ok(token)
            } else {
                Err("timed begin failed")
            }
        })?;
    if receipts.len() != 2
        || receipts[1].result != Ok(LifeCommandOutcome::Expiry(ExpiryOutcome::Deferred(expiry)))
    {
        return Err("45-second active attempt did not defer".into());
    }
    app.world_mut()
        .resource_mut::<LifeCommandQueue>()
        .submit(LifeCommandRequest {
            id: 2,
            target,
            command: LifeCommand::CancelRevive(token),
        })?;
    app.world_mut().resource_mut::<ClockDemo>().next = 3;
    app.update();
    let demo = app.world().resource::<ClockDemo>();
    if let Some(error) = &demo.error {
        return Err(error.clone().into());
    }
    let expected = [
        Ok(LifeCommandOutcome::ReviveBegan(token)),
        Ok(LifeCommandOutcome::Expiry(ExpiryOutcome::Deferred(expiry))),
        Ok(LifeCommandOutcome::ReviveCanceled),
        Ok(LifeCommandOutcome::Expiry(ExpiryOutcome::Terminated {
            current: 1,
            maximum: 100,
        })),
    ];
    if demo.receipts.iter().map(|r| r.result).collect::<Vec<_>>() != expected
        || demo.receipts.iter().map(|r| r.id).collect::<Vec<_>>() != [0, 1, 2, 3]
        || app.world().resource::<Time<Fixed>>().elapsed() != Duration::from_millis(45100)
        || app.world().get::<Vitality>(target).map(|v| v.state()) != Some(LifeState::HumanTerminal)
        || !app
            .world()
            .get::<BleedoutClock>(target)
            .is_some_and(BleedoutClock::is_retired)
    {
        return Err("timed expiry verification failed".into());
    }
    println!(
        "PASS timed coop:450 fixed100ms steps count through active revival to Deferred at45s; ordered cancel then100ms poll terminates at45.1s; clock retired. APPROXIMATE ideal simulation/quantization and collapsed network-frame cleanup; live coop comparison pending."
    );
    Ok(())
}

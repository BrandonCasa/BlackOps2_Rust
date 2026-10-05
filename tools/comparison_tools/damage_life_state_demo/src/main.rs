#![forbid(unsafe_code)]

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
    if (restored.current(), restored.maximum(), restored.state()) != (100, 100, LifeState::Standing)
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
            before: 50,
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
            .map(|v| (v.state(), v.current()))
            != Some((LifeState::ActorDead, 0))
        || app
            .world()
            .get::<Vitality>(human)
            .map(|v| (v.state(), v.current(), v.maximum()))
            != Some((LifeState::HumanTerminal, 1, 100))
    {
        return Err("ordered lifecycle verification failed".into());
    }
    println!(
        "PASS: down/cancel/restart/revive100/redown; stale inputs rejected; expiry deferred then terminal after cancel; positive terminal health preserved; actor dead. Fidelity APPROXIMATE: ordinary positive player subtraction statically qualified; actor arithmetic inferred; active native dispatch/live comparison unqualified; revive100 combines Steam script and qualified Plutonium native evidence; terminal network-frame interval collapsed; final spectator health unknown; coop comparison pending. Initial health and teammate/interaction/expiry timing are admitted demonstration inputs; no raw revive or bleedout timer."
    );
    Ok(())
}

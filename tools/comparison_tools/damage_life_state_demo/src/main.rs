#![forbid(unsafe_code)]

use bevy::{app::ScheduleRunnerPlugin, prelude::*, time::TimeUpdateStrategy};
use bo2_gameplay::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut app = bo2_core::headless_app(ScheduleRunnerPlugin::run_once());
    app.add_plugins(ResolvedDamagePlugin);
    // Initialize Bevy's clock before the admitted demonstration fixed step.
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
    let actor = app
        .world_mut()
        .spawn(bo2_origins::fallback_ordinary_actor(150)?)
        .id();
    for (id, target, amount) in [
        (0, human, 60),
        (1, actor, 100),
        (2, human, 40),
        (3, actor, 50),
        (4, actor, 1),
        (5, human, 1),
    ] {
        app.world_mut()
            .resource_mut::<DamageQueue>()
            .submit(DamageRequest {
                id,
                target,
                damage: ResolvedDamage::new(amount)?,
            })?;
    }
    app.update();
    let results: Vec<_> = app
        .world_mut()
        .resource_mut::<DamageResults>()
        .drain()
        .collect();
    let player = *app
        .world()
        .get::<Vitality>(human)
        .ok_or("missing demonstration player")?;
    let zombie = *app
        .world()
        .get::<Vitality>(actor)
        .ok_or("missing demonstration actor")?;
    let expected = [
        (
            0,
            human,
            Ok(DamageOutcome::Hurt {
                before: 100,
                after: 40,
            }),
        ),
        (
            1,
            actor,
            Ok(DamageOutcome::Hurt {
                before: 150,
                after: 50,
            }),
        ),
        (
            2,
            human,
            Ok(DamageOutcome::Downed {
                before: 40,
                after: 1,
            }),
        ),
        (3, actor, Ok(DamageOutcome::ActorDied { before: 50 })),
        (
            4,
            actor,
            Ok(DamageOutcome::Ignored {
                state: LifeState::ActorDead,
            }),
        ),
        (
            5,
            human,
            Ok(DamageOutcome::Ignored {
                state: LifeState::Downed,
            }),
        ),
    ];
    if results
        .iter()
        .map(|r| (r.id, r.target, r.result))
        .collect::<Vec<_>>()
        != expected
        || player.state() != LifeState::Downed
        || player.current() != 1
        || zombie.state() != LifeState::ActorDead
        || zombie.current() != 0
    {
        return Err("demonstration end-state verification failed".into());
    }
    for result in results {
        println!("request {}: {:?}", result.id, result.result);
    }
    println!(
        "PASS: player downed health=1; actor dead health=0. Fidelity APPROXIMATE: native player/actor subtraction and actor clamping inferred; integer admission restricted; original comparison pending. Initial health explicitly demo-selected; teammate eligibility admitted by caller; impacts already resolved, no universal melee/timing claim."
    );
    Ok(())
}

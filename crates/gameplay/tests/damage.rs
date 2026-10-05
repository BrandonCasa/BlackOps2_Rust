#![forbid(unsafe_code)]

use bevy::{prelude::*, time::TimeUpdateStrategy};
use bo2_gameplay::*;

fn damage(amount: u32) -> ResolvedDamage {
    ResolvedDamage::new(amount).unwrap()
}

#[test]
fn human_and_actor_boundary_cases() {
    for amount in [99, 100, 101, u32::MAX] {
        let mut human =
            Vitality::human(100, 100, 1, PlayerLethalPolicy::DownWithEligibleTeammate).unwrap();
        let mut actor = Vitality::ordinary_actor(100, 150).unwrap();
        human.apply(damage(amount)).unwrap();
        actor.apply(damage(amount)).unwrap();
        assert_eq!(human.maximum(), 100);
        assert_eq!(actor.maximum(), 150);
        if amount < 100 {
            assert_eq!((human.state(), human.current()), (LifeState::Standing, 1));
            assert_eq!((actor.state(), actor.current()), (LifeState::ActorAlive, 1));
        } else {
            assert_eq!((human.state(), human.current()), (LifeState::Downed, 1));
            assert_eq!((actor.state(), actor.current()), (LifeState::ActorDead, 0));
            assert_eq!(
                human.apply(damage(u32::MAX)),
                Ok(DamageOutcome::Ignored {
                    state: LifeState::Downed
                })
            );
            assert_eq!(
                actor.apply(damage(u32::MAX)),
                Ok(DamageOutcome::Ignored {
                    state: LifeState::ActorDead
                })
            );
        }
    }
}

#[test]
fn invalid_values_and_maximum_amount_do_not_overflow() {
    assert_eq!(ResolvedDamage::new(0), Err(LifeStateError::ZeroDamage));
    assert_eq!(
        Vitality::ordinary_actor(1, 0),
        Err(LifeStateError::ZeroMaximum)
    );
    assert_eq!(
        Vitality::ordinary_actor(0, 100),
        Err(LifeStateError::InvalidCurrent)
    );
    assert_eq!(
        Vitality::ordinary_actor(101, 100),
        Err(LifeStateError::InvalidCurrent)
    );
    assert_eq!(
        Vitality::human(100, 100, 0, PlayerLethalPolicy::DownWithEligibleTeammate),
        Err(LifeStateError::InvalidDownedHealth)
    );
    assert_eq!(
        Vitality::human(100, 100, 101, PlayerLethalPolicy::DownWithEligibleTeammate),
        Err(LifeStateError::InvalidDownedHealth)
    );
    let mut actor = Vitality::ordinary_actor(u32::MAX, u32::MAX).unwrap();
    actor.apply(damage(u32::MAX - 1)).unwrap();
    assert_eq!(actor.current(), 1);
    actor.apply(damage(u32::MAX)).unwrap();
    assert_eq!(actor.current(), 0);
}

#[test]
fn unresolved_session_policy_is_atomic_and_can_be_refreshed() {
    let mut human =
        Vitality::human(100, 100, 1, PlayerLethalPolicy::SoloOrLastEligiblePlayer).unwrap();
    human.apply(damage(20)).unwrap();
    let before = human;
    assert_eq!(
        human.apply(damage(80)),
        Err(LifeStateError::UnsupportedPlayerPolicy)
    );
    assert_eq!(human, before);
    human.set_player_policy(PlayerLethalPolicy::DownWithEligibleTeammate);
    assert!(matches!(
        human.apply(damage(80)),
        Ok(DamageOutcome::Downed {
            before: 80,
            after: 1,
            ..
        })
    ));
}

fn app() -> App {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, LifeStatePlugin));
    // Bevy's first real-time update initializes the clock with zero elapsed time.
    app.insert_resource(TimeUpdateStrategy::FixedTimesteps(0));
    app.update();
    app
}

#[test]
fn actual_fixed_schedule_handles_missing_despawned_and_ambiguous_requests() {
    let mut app = app();
    let actor = app
        .world_mut()
        .spawn(Vitality::ordinary_actor(10, 10).unwrap())
        .id();
    let absent_component = app.world_mut().spawn_empty().id();
    let despawned = app.world_mut().spawn_empty().id();
    app.world_mut().despawn(despawned);
    let mut queue = app.world_mut().resource_mut::<LifeCommandQueue>();
    for (id, target) in [(1, actor), (2, absent_component), (3, despawned)] {
        queue
            .submit(LifeCommandRequest {
                id,
                target,
                command: LifeCommand::Damage(damage(10)),
            })
            .unwrap();
    }
    assert_eq!(
        queue.submit(LifeCommandRequest {
            id: 3,
            target: actor,
            command: LifeCommand::Damage(damage(1))
        }),
        Err(LifeStateError::NonIncreasingRequestId)
    );
    assert_eq!(
        queue.submit(LifeCommandRequest {
            id: 2,
            target: actor,
            command: LifeCommand::Damage(damage(1))
        }),
        Err(LifeStateError::NonIncreasingRequestId)
    );
    assert_eq!(app.world().get::<Vitality>(actor).unwrap().current(), 10);
    app.world_mut().run_schedule(FixedUpdate);
    let results: Vec<_> = app
        .world_mut()
        .resource_mut::<LifeCommandResults>()
        .drain()
        .collect();
    assert_eq!(results.iter().map(|r| r.id).collect::<Vec<_>>(), [1, 2, 3]);
    assert_eq!(
        results[0].result,
        Ok(LifeCommandOutcome::Damage(DamageOutcome::ActorDied {
            before: 10
        }))
    );
    assert_eq!(results[1].result, Err(LifeStateError::MissingTarget));
    assert_eq!(results[2].result, Err(LifeStateError::MissingTarget));
    app.world_mut()
        .resource_mut::<LifeCommandQueue>()
        .submit(LifeCommandRequest {
            id: u64::MAX,
            target: actor,
            command: LifeCommand::Damage(damage(1)),
        })
        .unwrap();
    app.world_mut().run_schedule(FixedUpdate);
    assert_eq!(
        app.world_mut()
            .resource_mut::<LifeCommandResults>()
            .drain()
            .next()
            .unwrap()
            .result,
        Ok(LifeCommandOutcome::Damage(DamageOutcome::Ignored {
            state: LifeState::ActorDead
        }))
    );
}

#[derive(Debug, PartialEq, Eq)]
struct ReplaySnapshot {
    human: Vitality,
    actor: Vitality,
    results: Vec<(u64, Result<LifeCommandOutcome, LifeStateError>)>,
}

fn replay(pacing: &[u32], reverse_spawn: bool) -> ReplaySnapshot {
    let mut app = app();
    // Different entity/query ordering must not influence caller request order.
    let human_v =
        Vitality::human(100, 100, 1, PlayerLethalPolicy::DownWithEligibleTeammate).unwrap();
    let actor_v = Vitality::ordinary_actor(150, 150).unwrap();
    let (human, actor) = if reverse_spawn {
        let actor = app.world_mut().spawn(actor_v).id();
        (app.world_mut().spawn(human_v).id(), actor)
    } else {
        let human = app.world_mut().spawn(human_v).id();
        (human, app.world_mut().spawn(actor_v).id())
    };
    for (id, target, amount) in [
        (0, human, 60),
        (1, human, 40),
        (2, actor, 149),
        (3, actor, 1),
        (4, human, 10),
        (5, actor, 1),
    ] {
        app.world_mut()
            .resource_mut::<LifeCommandQueue>()
            .submit(LifeCommandRequest {
                id,
                target,
                command: LifeCommand::Damage(damage(amount)),
            })
            .unwrap();
    }
    for &steps in pacing {
        app.insert_resource(TimeUpdateStrategy::FixedTimesteps(steps));
        app.update();
    }
    let h = *app.world().get::<Vitality>(human).unwrap();
    let a = *app.world().get::<Vitality>(actor).unwrap();
    let results = app
        .world_mut()
        .resource_mut::<LifeCommandResults>()
        .drain()
        .map(|r| (r.id, r.result))
        .collect();
    ReplaySnapshot {
        human: h,
        actor: a,
        results,
    }
}

#[test]
fn replay_equivalence_across_entity_order_and_fixed_loop_pacing() {
    let expected = replay(&[1, 1, 1], false);
    assert_eq!(expected.human.state(), LifeState::Downed);
    assert_eq!(expected.actor.state(), LifeState::ActorDead);
    assert_eq!(expected.results.len(), 6);
    assert_eq!(
        expected.results[0].1,
        Ok(LifeCommandOutcome::Damage(DamageOutcome::Hurt {
            before: 100,
            after: 40
        }))
    );
    assert_eq!(expected, replay(&[1, 1, 1], false));
    assert_eq!(expected, replay(&[0, 0, 3], true));
    assert_eq!(expected, replay(&[3], false));
}

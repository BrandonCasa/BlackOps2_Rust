#![forbid(unsafe_code)]

use bevy::{prelude::*, time::TimeUpdateStrategy};
use bo2_gameplay::*;

fn human() -> Vitality {
    Vitality::human(100, 100, 1, PlayerLethalPolicy::DownWithEligibleTeammate).unwrap()
}
fn downed() -> Vitality {
    let mut v = human();
    v.apply(ResolvedDamage::new(100).unwrap()).unwrap();
    v
}
fn restore() -> ReviveRestoration {
    ReviveRestoration::new(100, 100).unwrap()
}

#[test]
fn cancel_restart_complete_and_redown_rejects_stale_episode() {
    let mut v = downed();
    let target = Entity::from_bits(1);
    let reviver = Entity::from_bits(2);
    let episode = v.downed_episode().unwrap();
    let old = v.begin_revive(target, reviver, episode).unwrap();
    assert_eq!(
        v.begin_revive(target, reviver, episode),
        Err(LifeStateError::AttemptAlreadyActive)
    );
    v.cancel_revive(target, old).unwrap();
    assert_eq!((v.current(), v.state()), (Some(1), LifeState::Downed));
    let token = v.begin_revive(target, reviver, episode).unwrap();
    let before = v;
    assert_eq!(
        v.complete_revive(target, old, restore()),
        Err(LifeStateError::StaleAttempt)
    );
    assert_eq!(v, before);
    v.complete_revive(target, token, restore()).unwrap();
    assert_eq!(
        (v.current(), v.maximum(), v.state()),
        (Some(100), 100, LifeState::Standing)
    );
    assert_eq!(
        v.complete_revive(target, token, restore()),
        Err(LifeStateError::NotDowned)
    );
    v.apply(ResolvedDamage::new(60).unwrap()).unwrap();
    assert_eq!(v.current(), Some(40));
    v.apply(ResolvedDamage::new(40).unwrap()).unwrap();
    assert_eq!(v.downed_episode().unwrap().number(), 2);
    assert_eq!(
        v.complete_revive(target, token, restore()),
        Err(LifeStateError::StaleEpisode)
    );
    assert_eq!(
        v.begin_revive(target, reviver, episode),
        Err(LifeStateError::StaleEpisode)
    );
}

#[test]
fn restoration_validation_wrong_target_and_nonhuman_are_atomic() {
    assert_eq!(
        ReviveRestoration::new(0, 100),
        Err(LifeStateError::InvalidCurrent)
    );
    assert_eq!(
        ReviveRestoration::new(101, 100),
        Err(LifeStateError::InvalidCurrent)
    );
    assert_eq!(
        ReviveRestoration::new(1, 0),
        Err(LifeStateError::ZeroMaximum)
    );
    let mut v = downed();
    let target = Entity::from_bits(1);
    let reviver = Entity::from_bits(2);
    let token = v
        .begin_revive(target, reviver, v.downed_episode().unwrap())
        .unwrap();
    let before = v;
    for excess in [i32::MAX.unsigned_abs() + 1, u32::MAX] {
        for (current, maximum) in [(excess, i32::MAX.unsigned_abs()), (1, excess)] {
            // A failed restoration cannot enter the command/API or consume the attempt.
            let result = ReviveRestoration::new(current, maximum)
                .and_then(|restoration| v.complete_revive(target, token, restoration));
            assert_eq!(result, Err(LifeStateError::HealthOutOfRange));
            assert_eq!(v, before);
        }
    }
    assert_eq!(
        v.complete_revive(reviver, token, restore()),
        Err(LifeStateError::WrongTarget)
    );
    assert_eq!(v, before);
    v.complete_revive(
        target,
        token,
        ReviveRestoration::new(i32::MAX.unsigned_abs(), i32::MAX.unsigned_abs()).unwrap(),
    )
    .unwrap();
    assert_eq!(
        (v.current(), v.maximum()),
        (Some(i32::MAX.unsigned_abs()), i32::MAX.unsigned_abs())
    );
    let mut actor = Vitality::ordinary_actor(10, 10).unwrap();
    let episode = token.episode();
    assert_eq!(
        actor.begin_revive(target, reviver, episode),
        Err(LifeStateError::NotHuman)
    );
    actor.apply(ResolvedDamage::new(10).unwrap()).unwrap();
    assert_eq!(
        actor.complete_revive(target, token, restore()),
        Err(LifeStateError::NotHuman)
    );
    assert_eq!(
        human().begin_revive(target, reviver, episode),
        Err(LifeStateError::NotDowned)
    );
    let mut custom =
        Vitality::human(100, 100, 50, PlayerLethalPolicy::DownWithEligibleTeammate).unwrap();
    custom.apply(ResolvedDamage::new(100).unwrap()).unwrap();
    let token = custom
        .begin_revive(target, reviver, custom.downed_episode().unwrap())
        .unwrap();
    let before = custom;
    assert_eq!(
        custom.complete_revive(target, token, ReviveRestoration::new(1, 10).unwrap()),
        Err(LifeStateError::InvalidDownedHealth)
    );
    assert_eq!(custom, before);
}

fn app() -> App {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, LifeStatePlugin));
    app
}
fn submit(app: &mut App, id: u64, target: Entity, command: LifeCommand) {
    app.world_mut()
        .resource_mut::<LifeCommandQueue>()
        .submit(LifeCommandRequest {
            id,
            target,
            command,
        })
        .unwrap();
}
fn step(app: &mut App) -> Vec<LifeCommandResult> {
    app.world_mut().run_schedule(FixedUpdate);
    app.world_mut()
        .resource_mut::<LifeCommandResults>()
        .drain()
        .collect()
}

#[test]
fn fixed_step_order_includes_revive_and_following_damage() {
    let mut app = app();
    let target = app.world_mut().spawn(downed()).id();
    let reviver = app.world_mut().spawn(human()).id();
    let episode = app
        .world()
        .get::<Vitality>(target)
        .unwrap()
        .downed_episode()
        .unwrap();
    submit(
        &mut app,
        0,
        target,
        LifeCommand::BeginRevive { reviver, episode },
    );
    let LifeCommandOutcome::ReviveBegan(token) = step(&mut app)[0].result.unwrap() else {
        panic!("begin failed")
    };
    submit(
        &mut app,
        1,
        target,
        LifeCommand::CompleteRevive {
            token,
            restore: restore(),
        },
    );
    submit(
        &mut app,
        2,
        target,
        LifeCommand::Damage(ResolvedDamage::new(100).unwrap()),
    );
    submit(
        &mut app,
        3,
        target,
        LifeCommand::CompleteRevive {
            token,
            restore: restore(),
        },
    );
    let results = step(&mut app);
    assert_eq!(
        results[0].result,
        Ok(LifeCommandOutcome::Revived {
            current: 100,
            maximum: 100
        })
    );
    assert!(matches!(
        results[1].result,
        Ok(LifeCommandOutcome::Damage(DamageOutcome::Downed { .. }))
    ));
    assert_eq!(results[2].result, Err(LifeStateError::StaleEpisode));
    assert_eq!(
        app.world().get::<Vitality>(target).unwrap().state(),
        LifeState::Downed
    );
}

#[test]
fn missing_and_unavailable_reviver_do_not_restore_or_alias_entities() {
    let mut app = app();
    let target = app.world_mut().spawn(downed()).id();
    let reviver = app.world_mut().spawn(human()).id();
    let episode = app
        .world()
        .get::<Vitality>(target)
        .unwrap()
        .downed_episode()
        .unwrap();
    submit(
        &mut app,
        0,
        target,
        LifeCommand::BeginRevive {
            reviver: target,
            episode,
        },
    );
    assert_eq!(
        step(&mut app)[0].result,
        Err(LifeStateError::InvalidReviver)
    );
    submit(
        &mut app,
        1,
        target,
        LifeCommand::BeginRevive { reviver, episode },
    );
    let LifeCommandOutcome::ReviveBegan(token) = step(&mut app)[0].result.unwrap() else {
        panic!("begin failed")
    };
    app.world_mut().despawn(reviver);
    submit(
        &mut app,
        2,
        target,
        LifeCommand::CompleteRevive {
            token,
            restore: restore(),
        },
    );
    assert_eq!(
        step(&mut app)[0].result,
        Err(LifeStateError::InvalidReviver)
    );
    assert_eq!(
        app.world().get::<Vitality>(target).unwrap().state(),
        LifeState::Downed
    );
    app.world_mut().despawn(target);
    let fresh = app.world_mut().spawn(downed()).id();
    assert_ne!(fresh, target);
    submit(&mut app, 3, target, LifeCommand::CancelRevive(token));
    assert_eq!(step(&mut app)[0].result, Err(LifeStateError::MissingTarget));
    assert_eq!(
        app.world().get::<Vitality>(fresh).unwrap().state(),
        LifeState::Downed
    );
}

#[test]
fn repeated_resolved_lifecycle_replay_is_equivalent() {
    #[derive(Debug, PartialEq, Eq)]
    struct Snapshot {
        vitality: Vitality,
        results: Vec<Result<LifeCommandOutcome, LifeStateError>>,
    }
    fn run(pacing: &[u32]) -> Snapshot {
        let mut app = app();
        app.insert_resource(TimeUpdateStrategy::FixedTimesteps(0));
        app.update();
        let target = app.world_mut().spawn(downed()).id();
        let reviver = app.world_mut().spawn(human()).id();
        let episode = app
            .world()
            .get::<Vitality>(target)
            .unwrap()
            .downed_episode()
            .unwrap();
        submit(
            &mut app,
            0,
            target,
            LifeCommand::BeginRevive { reviver, episode },
        );
        let first = step(&mut app);
        let LifeCommandOutcome::ReviveBegan(token) = first[0].result.unwrap() else {
            panic!("begin failed")
        };
        submit(&mut app, 1, target, LifeCommand::CancelRevive(token));
        submit(
            &mut app,
            2,
            target,
            LifeCommand::CompleteRevive {
                token,
                restore: restore(),
            },
        );
        let mut results = first.into_iter().map(|r| r.result).collect::<Vec<_>>();
        results.extend(step(&mut app).into_iter().map(|r| r.result));
        submit(
            &mut app,
            3,
            target,
            LifeCommand::BeginRevive { reviver, episode },
        );
        let started = step(&mut app);
        let LifeCommandOutcome::ReviveBegan(fresh) = started[0].result.unwrap() else {
            panic!("begin failed")
        };
        results.extend(started.into_iter().map(|r| r.result));
        submit(
            &mut app,
            4,
            target,
            LifeCommand::CompleteRevive {
                token: fresh,
                restore: restore(),
            },
        );
        submit(
            &mut app,
            5,
            target,
            LifeCommand::Damage(ResolvedDamage::new(100).unwrap()),
        );
        submit(
            &mut app,
            6,
            target,
            LifeCommand::CompleteRevive {
                token: fresh,
                restore: restore(),
            },
        );
        for &steps in pacing {
            app.insert_resource(TimeUpdateStrategy::FixedTimesteps(steps));
            app.update();
        }
        results.extend(
            app.world_mut()
                .resource_mut::<LifeCommandResults>()
                .drain()
                .map(|r| r.result),
        );
        assert_eq!(results.len(), 7);
        assert_eq!(
            results[4],
            Ok(LifeCommandOutcome::Revived {
                current: 100,
                maximum: 100
            })
        );
        assert_eq!(results[6], Err(LifeStateError::StaleEpisode));
        Snapshot {
            vitality: *app.world().get::<Vitality>(target).unwrap(),
            results,
        }
    }
    assert_eq!(run(&[1, 1, 1]), run(&[1, 1, 1]));
    assert_eq!(run(&[1, 1, 1]), run(&[0, 0, 3]));
}

#[test]
fn component_replacement_requires_explicit_input_retirement() {
    let mut app = app();
    let target = app.world_mut().spawn(downed()).id();
    let reviver = app.world_mut().spawn(human()).id();
    let actor = app
        .world_mut()
        .spawn(Vitality::ordinary_actor(10, 10).unwrap())
        .id();
    let episode = app
        .world()
        .get::<Vitality>(target)
        .unwrap()
        .downed_episode()
        .unwrap();
    submit(
        &mut app,
        0,
        target,
        LifeCommand::BeginRevive { reviver, episode },
    );
    let LifeCommandOutcome::ReviveBegan(token) = step(&mut app)[0].result.unwrap() else {
        panic!("begin failed")
    };
    submit(
        &mut app,
        1,
        target,
        LifeCommand::CompleteRevive {
            token,
            restore: restore(),
        },
    );
    submit(
        &mut app,
        2,
        actor,
        LifeCommand::Damage(ResolvedDamage::new(10).unwrap()),
    );
    let retired = app
        .world_mut()
        .resource_mut::<LifeCommandQueue>()
        .retire_pending_target(target);
    assert_eq!(retired.len(), 1);
    assert_eq!(retired[0].id, 1);
    // Retire held token/result references too; never resubmit token after replacement.
    // Entity identity alone remains unchanged; replacement resets component counters.
    app.world_mut().entity_mut(target).insert(downed());
    let results = step(&mut app);
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].target, actor);
    assert_eq!(
        app.world().get::<Vitality>(target).unwrap().state(),
        LifeState::Downed
    );
    assert_eq!(
        app.world_mut()
            .resource_mut::<LifeCommandQueue>()
            .submit(LifeCommandRequest {
                id: 1,
                target,
                command: LifeCommand::CancelRevive(token)
            }),
        Err(LifeStateError::NonIncreasingRequestId)
    );
}

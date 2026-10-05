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
fn deferral_cancel_terminal_and_repeats_preserve_positive_health() {
    let target = Entity::from_bits(1);
    let reviver = Entity::from_bits(2);
    let mut v = downed();
    let expiry = v.downed_expiry(target).unwrap();
    assert_eq!(expiry.target(), target);
    assert_eq!(expiry.episode(), v.downed_episode().unwrap());
    let token = v.begin_revive(target, reviver, expiry.episode()).unwrap();
    let before = v;
    assert_eq!(
        v.expire_downed(target, expiry),
        Ok(ExpiryOutcome::Deferred(expiry))
    );
    assert_eq!(v, before);
    v.cancel_revive(target, token).unwrap();
    assert_eq!(
        v.expire_downed(target, expiry),
        Ok(ExpiryOutcome::Terminated {
            current: 1,
            maximum: 100
        })
    );
    let terminal = v;
    assert_eq!(
        (v.state(), v.current(), v.maximum()),
        (LifeState::HumanTerminal, Some(1), 100)
    );
    assert_eq!(v.downed_episode(), None);
    assert_eq!(v.expire_downed(target, expiry), Ok(ExpiryOutcome::Ignored));
    assert_eq!(
        v.apply(ResolvedDamage::new(i32::MAX.unsigned_abs()).unwrap()),
        Ok(DamageOutcome::Ignored {
            state: LifeState::HumanTerminal
        })
    );
    assert_eq!(
        v.complete_revive(target, token, restore()),
        Err(LifeStateError::NotDowned)
    );
    assert_eq!(
        v.cancel_revive(target, token),
        Err(LifeStateError::NotDowned)
    );
    assert_eq!(
        v.begin_revive(target, reviver, expiry.episode()),
        Err(LifeStateError::NotDowned)
    );
    assert_eq!(v.downed_expiry(target), Err(LifeStateError::NotDowned));
    assert_eq!(v, terminal);
}

#[test]
fn revived_and_redowned_expiry_is_stale_and_errors_are_atomic() {
    let target = Entity::from_bits(1);
    let other = Entity::from_bits(2);
    let mut v = downed();
    let expiry = v.downed_expiry(target).unwrap();
    let before = v;
    assert_eq!(
        v.expire_downed(other, expiry),
        Err(LifeStateError::WrongTarget)
    );
    assert_eq!(v, before);
    v.set_player_policy(PlayerLethalPolicy::SoloOrLastEligiblePlayer);
    let before = v;
    assert_eq!(
        v.expire_downed(target, expiry),
        Err(LifeStateError::UnsupportedPlayerPolicy)
    );
    assert_eq!(v, before);
    v.set_player_policy(PlayerLethalPolicy::DownWithEligibleTeammate);
    let token = v.begin_revive(target, other, expiry.episode()).unwrap();
    v.complete_revive(target, token, restore()).unwrap();
    let standing = v;
    assert_eq!(
        v.expire_downed(target, expiry),
        Err(LifeStateError::StaleEpisode)
    );
    assert_eq!(v, standing);
    v.apply(ResolvedDamage::new(100).unwrap()).unwrap();
    let redowned = v;
    assert_eq!(
        v.expire_downed(target, expiry),
        Err(LifeStateError::StaleEpisode)
    );
    assert_eq!(v, redowned);
    let mut actor = Vitality::ordinary_actor(150, 150).unwrap();
    assert_eq!(actor.downed_expiry(target), Err(LifeStateError::NotHuman));
    let alive = actor;
    assert_eq!(
        actor.expire_downed(target, expiry),
        Err(LifeStateError::NotHuman)
    );
    assert_eq!(actor, alive);
    actor.apply(ResolvedDamage::new(150).unwrap()).unwrap();
    let dead = actor;
    assert_eq!(
        actor.expire_downed(target, expiry),
        Err(LifeStateError::NotHuman)
    );
    assert_eq!(actor, dead);
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
fn drain(app: &mut App) -> Vec<Result<LifeCommandOutcome, LifeStateError>> {
    app.world_mut()
        .resource_mut::<LifeCommandResults>()
        .drain()
        .map(|r| r.result)
        .collect()
}
fn fixed(app: &mut App) -> Vec<Result<LifeCommandOutcome, LifeStateError>> {
    app.world_mut().run_schedule(FixedUpdate);
    drain(app)
}
fn setup() -> (App, Entity, Entity, DownedExpiry, ReviveToken) {
    let mut app = app();
    let target = app.world_mut().spawn(downed()).id();
    let reviver = app.world_mut().spawn(human()).id();
    let expiry = app
        .world()
        .get::<Vitality>(target)
        .unwrap()
        .downed_expiry(target)
        .unwrap();
    submit(
        &mut app,
        0,
        target,
        LifeCommand::BeginRevive {
            reviver,
            episode: expiry.episode(),
        },
    );
    let Ok(LifeCommandOutcome::ReviveBegan(token)) = fixed(&mut app)[0] else {
        panic!("begin failed")
    };
    (app, target, reviver, expiry, token)
}

#[test]
fn fixed_order_has_no_implicit_expiry_priority() {
    let (mut app, target, _, expiry, token) = setup();
    submit(&mut app, 1, target, LifeCommand::ExpireDowned(expiry));
    submit(
        &mut app,
        2,
        target,
        LifeCommand::CompleteRevive {
            token,
            restore: restore(),
        },
    );
    submit(&mut app, 3, target, LifeCommand::ExpireDowned(expiry));
    assert_eq!(
        fixed(&mut app),
        vec![
            Ok(LifeCommandOutcome::Expiry(ExpiryOutcome::Deferred(expiry))),
            Ok(LifeCommandOutcome::Revived {
                current: 100,
                maximum: 100
            }),
            Err(LifeStateError::StaleEpisode)
        ]
    );
    let (mut app, target, _, expiry, token) = setup();
    submit(
        &mut app,
        1,
        target,
        LifeCommand::CompleteRevive {
            token,
            restore: restore(),
        },
    );
    submit(&mut app, 2, target, LifeCommand::ExpireDowned(expiry));
    assert_eq!(
        fixed(&mut app),
        vec![
            Ok(LifeCommandOutcome::Revived {
                current: 100,
                maximum: 100
            }),
            Err(LifeStateError::StaleEpisode)
        ]
    );
    let (mut app, target, _, expiry, token) = setup();
    submit(&mut app, 1, target, LifeCommand::CancelRevive(token));
    submit(&mut app, 2, target, LifeCommand::ExpireDowned(expiry));
    submit(
        &mut app,
        3,
        target,
        LifeCommand::CompleteRevive {
            token,
            restore: restore(),
        },
    );
    assert_eq!(
        fixed(&mut app),
        vec![
            Ok(LifeCommandOutcome::ReviveCanceled),
            Ok(LifeCommandOutcome::Expiry(ExpiryOutcome::Terminated {
                current: 1,
                maximum: 100
            })),
            Err(LifeStateError::NotDowned)
        ]
    );
}

#[test]
fn missing_generation_wrong_target_and_retirement_do_not_alias() {
    let (mut app, target, _, expiry, _) = setup();
    let other = app.world_mut().spawn(downed()).id();
    submit(&mut app, 1, other, LifeCommand::ExpireDowned(expiry));
    assert_eq!(fixed(&mut app), vec![Err(LifeStateError::WrongTarget)]);
    submit(&mut app, 2, target, LifeCommand::ExpireDowned(expiry));
    let retired = app
        .world_mut()
        .resource_mut::<LifeCommandQueue>()
        .retire_pending_target(target);
    assert_eq!(retired.len(), 1);
    assert_eq!(retired[0].command, LifeCommand::ExpireDowned(expiry));
    assert!(fixed(&mut app).is_empty());
    app.world_mut().despawn(target);
    let fresh = app.world_mut().spawn(downed()).id();
    assert_ne!(fresh, target);
    submit(&mut app, 3, target, LifeCommand::ExpireDowned(expiry));
    assert_eq!(fixed(&mut app), vec![Err(LifeStateError::MissingTarget)]);
    submit(&mut app, 4, fresh, LifeCommand::ExpireDowned(expiry));
    assert_eq!(fixed(&mut app), vec![Err(LifeStateError::WrongTarget)]);
    assert_eq!(
        app.world().get::<Vitality>(fresh).unwrap().state(),
        LifeState::Downed
    );
}

#[test]
fn repeated_mixed_replay_and_update_pacing_are_equivalent() {
    fn run(pacing: &[u32]) -> (Vitality, Vec<Result<LifeCommandOutcome, LifeStateError>>) {
        let (mut app, target, _, expiry, token) = setup();
        app.insert_resource(TimeUpdateStrategy::FixedTimesteps(0));
        app.update();
        submit(&mut app, 1, target, LifeCommand::ExpireDowned(expiry));
        let mut results = fixed(&mut app);
        submit(&mut app, 2, target, LifeCommand::CancelRevive(token));
        submit(&mut app, 3, target, LifeCommand::ExpireDowned(expiry));
        submit(
            &mut app,
            4,
            target,
            LifeCommand::CompleteRevive {
                token,
                restore: restore(),
            },
        );
        submit(
            &mut app,
            5,
            target,
            LifeCommand::Damage(ResolvedDamage::new(100).unwrap()),
        );
        submit(&mut app, 6, target, LifeCommand::ExpireDowned(expiry));
        for count in pacing {
            app.insert_resource(TimeUpdateStrategy::FixedTimesteps(*count));
            app.update();
            results.extend(drain(&mut app));
        }
        results.extend(fixed(&mut app));
        (*app.world().get::<Vitality>(target).unwrap(), results)
    }
    let expected = run(&[1]);
    assert_eq!(expected.0.state(), LifeState::HumanTerminal);
    assert_eq!(expected.1.len(), 6);
    assert!(matches!(
        expected.1[0],
        Ok(LifeCommandOutcome::Expiry(ExpiryOutcome::Deferred(_)))
    ));
    assert_eq!(expected.1[1], Ok(LifeCommandOutcome::ReviveCanceled));
    assert_eq!(
        expected.1[2],
        Ok(LifeCommandOutcome::Expiry(ExpiryOutcome::Terminated {
            current: 1,
            maximum: 100
        }))
    );
    assert_eq!(expected.1[3], Err(LifeStateError::NotDowned));
    assert_eq!(
        expected.1[4],
        Ok(LifeCommandOutcome::Damage(DamageOutcome::Ignored {
            state: LifeState::HumanTerminal
        }))
    );
    assert_eq!(
        expected.1[5],
        Ok(LifeCommandOutcome::Expiry(ExpiryOutcome::Ignored))
    );
    for pacing in [&[1][..], &[0, 0, 1, 3][..], &[3, 0, 1][..]] {
        assert_eq!(run(pacing), expected);
        assert_eq!(run(pacing), expected);
    }
}

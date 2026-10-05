use bevy::{prelude::*, time::TimeUpdateStrategy};
use bo2_gameplay::*;
use std::time::Duration;
fn human() -> Vitality {
    Vitality::human(100, 100, 1, PlayerLethalPolicy::DownWithEligibleTeammate).unwrap()
}
fn timing() -> ReviveTiming {
    ReviveTiming::new(Duration::from_secs(3), Duration::from_millis(50)).unwrap()
}
fn restore() -> ReviveRestoration {
    ReviveRestoration::new(100, 100).unwrap()
}
fn setup() -> (Entity, Entity, Vitality, ReviveToken, ReviveClock) {
    let target = Entity::from_bits(1);
    let reviver = Entity::from_bits(2);
    let mut v = human();
    v.apply(ResolvedDamage::new(100).unwrap()).unwrap();
    let token = v
        .begin_revive(target, reviver, v.downed_episode().unwrap())
        .unwrap();
    let c = ReviveClock::new(target, v, token, timing(), restore()).unwrap();
    (target, reviver, v, token, c)
}
fn sample(c: &mut ReviveClock, target: Entity, v: Vitality, reviver: Entity) {
    c.sample_eligibility(target, Some(v), reviver, ReviveEligibility::Eligible)
        .unwrap();
}
fn wait(c: &mut ReviveClock, target: Entity, v: Vitality, reviver: Entity) {
    c.tick(
        target,
        Some(v),
        reviver,
        Some(human()),
        Duration::from_millis(50),
    )
    .unwrap();
}
fn receipt(
    id: u64,
    target: Entity,
    result: Result<LifeCommandOutcome, LifeStateError>,
) -> LifeCommandResult {
    LifeCommandResult { id, target, result }
}

#[test]
fn ideal_boundary_and_final_wait_release_preserve_sample_before_wait() {
    let (target, reviver, v, _, mut c) = setup();
    for _ in 0..59 {
        sample(&mut c, target, v, reviver);
        wait(&mut c, target, v, reviver);
    }
    assert_eq!(c.progress(), Duration::from_millis(2950));
    assert_eq!(c.decision(), None);
    sample(&mut c, target, v, reviver);
    assert_eq!(
        c.tick(
            target,
            Some(v),
            reviver,
            Some(human()),
            Duration::from_millis(49)
        ),
        Ok(None)
    );
    let before = c.clone();
    // Releasing use inside an admitted wait is not another loop eligibility sample.
    assert_eq!(
        c.sample_eligibility(target, Some(v), reviver, ReviveEligibility::Ineligible),
        Err(ReviveClockError::WrongPhase)
    );
    assert_eq!(c, before);
    assert_eq!(
        c.tick(
            target,
            Some(v),
            reviver,
            Some(human()),
            Duration::from_millis(1)
        ),
        Ok(Some(ReviveDecision::Complete))
    );
    assert_eq!(c.progress(), Duration::from_secs(3));
}

#[test]
fn cancellation_restart_and_reviver_down_before_threshold() {
    let (target, reviver, mut v, token, mut c) = setup();
    for _ in 0..20 {
        sample(&mut c, target, v, reviver);
        wait(&mut c, target, v, reviver);
    }
    c.sample_eligibility(target, Some(v), reviver, ReviveEligibility::Ineligible)
        .unwrap();
    assert_eq!(c.decision(), Some(ReviveDecision::Cancel));
    let mut queue = LifeCommandQueue::default();
    c.enqueue(&mut queue, 0).unwrap();
    v.cancel_revive(target, token).unwrap();
    c.acknowledge(receipt(0, target, Ok(LifeCommandOutcome::ReviveCanceled)))
        .unwrap();
    assert!(c.is_retired());
    let next = v
        .begin_revive(target, reviver, v.downed_episode().unwrap())
        .unwrap();
    assert_ne!(next, token);
    let mut c = ReviveClock::new(target, v, next, timing(), restore()).unwrap();
    assert_eq!(c.progress(), Duration::ZERO);
    for _ in 0..59 {
        sample(&mut c, target, v, reviver);
        wait(&mut c, target, v, reviver);
    }
    sample(&mut c, target, v, reviver);
    let mut downed_reviver = human();
    downed_reviver
        .apply(ResolvedDamage::new(100).unwrap())
        .unwrap();
    assert_eq!(
        c.tick(
            target,
            Some(v),
            reviver,
            Some(downed_reviver),
            Duration::from_millis(50)
        ),
        Ok(Some(ReviveDecision::Cancel))
    );
    assert_eq!(c.progress(), Duration::from_millis(2950));
    let (_, _, missing_v, _, mut missing) = setup();
    sample(&mut missing, target, missing_v, reviver);
    assert_eq!(
        missing.tick(
            target,
            Some(missing_v),
            reviver,
            None,
            Duration::from_millis(50)
        ),
        Ok(Some(ReviveDecision::Cancel))
    );
}

#[test]
fn stale_identity_and_lifecycle_retire_without_component_alias() {
    let (target, reviver, mut v, token, mut c) = setup();
    let before = c.clone();
    assert_eq!(
        c.sample_eligibility(reviver, Some(v), reviver, ReviveEligibility::Eligible),
        Err(ReviveClockError::Lifecycle(LifeStateError::WrongTarget))
    );
    assert_eq!(c, before);
    assert_eq!(
        c.tick(target, Some(v), target, Some(human()), Duration::ZERO),
        Err(ReviveClockError::Lifecycle(LifeStateError::InvalidReviver))
    );
    assert_eq!(c, before);
    v.cancel_revive(target, token).unwrap();
    let fresh = v
        .begin_revive(target, reviver, v.downed_episode().unwrap())
        .unwrap();
    assert_eq!(
        c.tick(target, Some(v), reviver, Some(human()), Duration::ZERO),
        Err(ReviveClockError::Lifecycle(LifeStateError::StaleAttempt))
    );
    assert!(c.is_retired());
    let mut c = ReviveClock::new(target, v, fresh, timing(), restore()).unwrap();
    v.complete_revive(target, fresh, restore()).unwrap();
    v.apply(ResolvedDamage::new(100).unwrap()).unwrap();
    assert_eq!(
        c.tick(target, Some(v), reviver, Some(human()), Duration::ZERO),
        Err(ReviveClockError::Lifecycle(LifeStateError::StaleEpisode))
    );
    assert!(c.is_retired());
    let (_, _, _, _, mut missing) = setup();
    assert_eq!(
        missing.tick(target, None, reviver, Some(human()), Duration::ZERO),
        Err(ReviveClockError::Lifecycle(LifeStateError::MissingTarget))
    );
    assert!(missing.is_retired());
    let (_, _, v, _, mut retired) = setup();
    retired.retire();
    assert_eq!(
        retired.tick(target, Some(v), reviver, Some(human()), Duration::ZERO),
        Err(ReviveClockError::Retired)
    );
}

#[test]
fn pending_receipt_and_queue_rejection_are_atomic() {
    let (target, reviver, v, _, mut c) = setup();
    let mut queue = LifeCommandQueue::default();
    assert_eq!(c.enqueue(&mut queue, 0), Err(ReviveClockError::NotReady));
    for _ in 0..60 {
        sample(&mut c, target, v, reviver);
        wait(&mut c, target, v, reviver);
    }
    c.enqueue(&mut queue, u64::MAX).unwrap();
    assert_eq!(c.enqueue(&mut queue, 0), Err(ReviveClockError::InFlight));
    let before = c.clone();
    for bad in [
        receipt(
            0,
            target,
            Ok(LifeCommandOutcome::Revived {
                current: 100,
                maximum: 100,
            }),
        ),
        receipt(
            u64::MAX,
            reviver,
            Ok(LifeCommandOutcome::Revived {
                current: 100,
                maximum: 100,
            }),
        ),
        receipt(u64::MAX, target, Ok(LifeCommandOutcome::ReviveCanceled)),
        receipt(
            u64::MAX,
            target,
            Ok(LifeCommandOutcome::Revived {
                current: 1,
                maximum: 100,
            }),
        ),
    ] {
        assert_eq!(
            c.acknowledge(bad),
            Err(ReviveClockError::UnexpectedAcknowledgement)
        );
        assert_eq!(c, before);
    }
    c.acknowledge(receipt(
        u64::MAX,
        target,
        Ok(LifeCommandOutcome::Revived {
            current: 100,
            maximum: 100,
        }),
    ))
    .unwrap();
    assert!(c.is_retired());
    assert_eq!(
        c.acknowledge(receipt(
            u64::MAX,
            target,
            Err(LifeStateError::MissingTarget)
        )),
        Err(ReviveClockError::Retired)
    );
    let (_, _, _, _, mut c) = setup();
    c.sample_eligibility(target, Some(v), reviver, ReviveEligibility::Ineligible)
        .unwrap();
    let before = c.clone();
    assert_eq!(
        c.enqueue(&mut queue, 0),
        Err(ReviveClockError::Lifecycle(
            LifeStateError::NonIncreasingRequestId
        ))
    );
    assert_eq!(c, before);
    assert_eq!(queue.retire_pending_target(target).len(), 1);
    c.retire(); // Held attempt clock retired before replacing the same Entity's component.
}

#[test]
fn invalid_timing_restore_and_policy_do_not_invent_routes() {
    assert_eq!(
        ReviveTiming::new(Duration::ZERO, Duration::from_millis(50)),
        Err(ReviveClockError::ZeroDuration)
    );
    assert_eq!(
        ReviveTiming::new(Duration::from_secs(3), Duration::ZERO),
        Err(ReviveClockError::ZeroInterval)
    );
    assert_eq!(
        ReviveTiming::new(Duration::from_secs(3), Duration::from_secs(4)),
        Err(ReviveClockError::UnsupportedInterval)
    );
    let (target, _, mut v, token, _) = setup();
    v.set_player_policy(PlayerLethalPolicy::SoloOrLastEligiblePlayer);
    assert_eq!(
        ReviveClock::new(target, v, token, timing(), restore()),
        Err(ReviveClockError::Lifecycle(
            LifeStateError::UnsupportedPlayerPolicy
        ))
    );
    let mut v =
        Vitality::human(100, 100, 50, PlayerLethalPolicy::DownWithEligibleTeammate).unwrap();
    v.apply(ResolvedDamage::new(100).unwrap()).unwrap();
    let token = v
        .begin_revive(target, Entity::from_bits(2), v.downed_episode().unwrap())
        .unwrap();
    assert_eq!(
        ReviveClock::new(
            target,
            v,
            token,
            timing(),
            ReviveRestoration::new(1, 1).unwrap()
        ),
        Err(ReviveClockError::Lifecycle(
            LifeStateError::InvalidDownedHealth
        ))
    );
}

#[derive(Resource)]
struct Harness {
    target: Entity,
    reviver: Entity,
    next: u64,
    receipts: Vec<LifeCommandResult>,
}
fn produce(
    time: Res<Time<Fixed>>,
    mut clocks: Query<(&mut ReviveClock, &mut BleedoutClock)>,
    vitality: Query<&Vitality>,
    mut queue: ResMut<LifeCommandQueue>,
    mut h: ResMut<Harness>,
) {
    let target = h.target;
    let reviver = h.reviver;
    let v = vitality.get(target).ok().copied();
    let rv = vitality.get(reviver).ok().copied();
    let (mut revive, mut bleed) = clocks.get_mut(target).unwrap();
    // Explicit source phase: eligibility sampled before wait. Assembly supplies resolved eligible.
    if !revive.is_retired() {
        if revive.awaits_eligibility() {
            revive
                .sample_eligibility(target, v, reviver, ReviveEligibility::Eligible)
                .unwrap();
        }
        if revive
            .tick(target, v, reviver, rv, time.delta())
            .unwrap()
            .is_some()
        {
            revive.enqueue(&mut queue, h.next).unwrap();
            h.next = h.next.checked_add(1).unwrap();
        }
    }
    // Explicit producer order: revive decision before bleedout poll on the same step.
    if bleed.tick(target, v, time.delta()).unwrap().is_some() {
        bleed.enqueue(&mut queue, h.next).unwrap();
        h.next = h.next.checked_add(1).unwrap();
    }
}
fn acknowledge(
    mut clocks: Query<(&mut ReviveClock, &mut BleedoutClock)>,
    mut results: ResMut<LifeCommandResults>,
    mut h: ResMut<Harness>,
) {
    for r in results.drain() {
        let (mut revive, mut bleed) = clocks.get_mut(h.target).unwrap();
        if revive.in_flight() == Some(r.id) {
            revive.acknowledge(r).unwrap();
        }
        if bleed.in_flight() == Some(r.id) {
            bleed.acknowledge(r).unwrap();
        }
        h.receipts.push(r);
    }
}
#[test]
fn real_fixed_replay_revives_while_expired_bleedout_defers() {
    fn run(pacing: &[u32]) -> (Vitality, ReviveClock, BleedoutClock, Vec<LifeCommandResult>) {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, LifeStatePlugin));
        app.insert_resource(Time::<Fixed>::from_duration(Duration::from_millis(50)));
        app.world_mut()
            .resource_mut::<Time<Virtual>>()
            .set_max_delta(Duration::from_secs(1));
        app.insert_resource(TimeUpdateStrategy::FixedTimesteps(0));
        app.update();
        let target = app.world_mut().spawn(human()).id();
        let reviver = app.world_mut().spawn(human()).id();
        let mut v = human();
        v.apply(ResolvedDamage::new(100).unwrap()).unwrap();
        let token = v
            .begin_revive(target, reviver, v.downed_episode().unwrap())
            .unwrap();
        let mut bleed = BleedoutClock::new(
            target,
            v,
            BleedoutTiming::new(Duration::from_secs(45), Duration::from_millis(100)).unwrap(),
        )
        .unwrap();
        bleed
            .tick(target, Some(v), Duration::from_secs(45))
            .unwrap();
        app.world_mut().entity_mut(target).insert(v);
        bleed
            .enqueue(&mut app.world_mut().resource_mut::<LifeCommandQueue>(), 0)
            .unwrap();
        app.world_mut().run_schedule(FixedUpdate);
        let receipts: Vec<_> = app
            .world_mut()
            .resource_mut::<LifeCommandResults>()
            .drain()
            .collect();
        assert_eq!(receipts.len(), 1);
        assert_eq!(
            receipts[0].result,
            Ok(LifeCommandOutcome::Expiry(ExpiryOutcome::Deferred(
                bleed.expiry()
            )))
        );
        bleed.acknowledge(receipts[0]).unwrap();
        app.world_mut().entity_mut(target).insert((
            ReviveClock::new(target, v, token, timing(), restore()).unwrap(),
            bleed,
        ));
        app.insert_resource(Harness {
            target,
            reviver,
            next: 1,
            receipts,
        });
        app.add_systems(FixedUpdate, produce.before(ApplyLifeCommands));
        app.add_systems(FixedUpdate, acknowledge.after(ApplyLifeCommands));
        let mut steps = 0;
        let mut frame = 0;
        while steps < 62 {
            let count = pacing[frame % pacing.len()].min(62 - steps);
            app.insert_resource(TimeUpdateStrategy::FixedTimesteps(count));
            app.update();
            steps += count;
            frame += 1;
        }
        let h = app.world().resource::<Harness>();
        assert!(h.receipts.iter().any(|r| matches!(
            r.result,
            Ok(LifeCommandOutcome::Expiry(ExpiryOutcome::Deferred(_)))
        )));
        let revived: Vec<_> = h
            .receipts
            .iter()
            .filter(|r| matches!(r.result, Ok(LifeCommandOutcome::Revived { .. })))
            .collect();
        assert_eq!(revived.len(), 1);
        assert_eq!(
            revived[0].result,
            Ok(LifeCommandOutcome::Revived {
                current: 100,
                maximum: 100
            })
        );
        assert!(
            h.receipts[..h.receipts.len() - 1]
                .iter()
                .all(|r| r.result.is_ok())
        );
        assert_eq!(
            h.receipts[h.receipts.len() - 2].result,
            Ok(LifeCommandOutcome::Revived {
                current: 100,
                maximum: 100
            })
        );
        assert_eq!(
            h.receipts.last().unwrap().result,
            Err(LifeStateError::StaleEpisode)
        );
        // At the3s boundary assembly queued completion before the due bleedout poll.
        // That poll consequently becomes stale, rather than overriding success.
        assert_eq!(
            app.world().resource::<Time<Fixed>>().elapsed(),
            Duration::from_millis(3100)
        );
        let (revive, bleed) = app
            .world()
            .entity(target)
            .get_components::<(&ReviveClock, &BleedoutClock)>()
            .unwrap();
        (
            *app.world().get::<Vitality>(target).unwrap(),
            revive.clone(),
            bleed.clone(),
            h.receipts.clone(),
        )
    }
    let expected = run(&[1]);
    assert_eq!(
        (expected.0.state(), expected.0.current()),
        (LifeState::Standing, Some(100))
    );
    assert!(expected.1.is_retired() && expected.2.is_retired());
    for pacing in [&[1][..], &[0, 3, 1][..], &[2, 0, 1][..]] {
        assert_eq!(run(pacing), expected);
        assert_eq!(run(pacing), expected);
    }
}

#[test]
fn oversized_delta_does_not_speculate_more_eligibility_samples() {
    let (target, reviver, v, _, mut c) = setup();
    sample(&mut c, target, v, reviver);
    assert_eq!(
        c.tick(target, Some(v), reviver, Some(human()), Duration::MAX),
        Ok(None)
    );
    assert_eq!(c.progress(), Duration::from_millis(50));
    assert!(c.awaits_eligibility());
    assert_eq!(
        c.tick(target, Some(v), reviver, Some(human()), Duration::MAX),
        Ok(None)
    );
    assert_eq!(c.progress(), Duration::from_millis(50));
    c.sample_eligibility(target, Some(v), reviver, ReviveEligibility::Ineligible)
        .unwrap();
    assert_eq!(c.decision(), Some(ReviveDecision::Cancel));
}

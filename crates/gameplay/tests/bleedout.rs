use bevy::{prelude::*, time::TimeUpdateStrategy};
use bo2_gameplay::*;
use std::time::Duration;

fn human() -> Vitality {
    Vitality::human(100, 100, 1, PlayerLethalPolicy::DownWithEligibleTeammate).unwrap()
}
fn downed() -> Vitality {
    let mut v = human();
    v.apply(ResolvedDamage::new(100).unwrap()).unwrap();
    v
}
fn timing() -> BleedoutTiming {
    BleedoutTiming::new(Duration::from_secs(45), Duration::from_millis(100)).unwrap()
}
fn receipt(
    id: u64,
    target: Entity,
    result: Result<LifeCommandOutcome, LifeStateError>,
) -> LifeCommandResult {
    LifeCommandResult { id, target, result }
}

#[test]
fn countdown_boundary_active_revival_and_cancel_do_not_reset() {
    let target = Entity::from_bits(1);
    let mut v = downed();
    let token = v
        .begin_revive(target, Entity::from_bits(2), v.downed_episode().unwrap())
        .unwrap();
    let mut c = BleedoutClock::new(target, v, timing()).unwrap();
    let expiry = c.expiry();
    assert_eq!(
        c.tick(target, Some(v), Duration::from_millis(44999)),
        Ok(None)
    );
    assert_eq!(
        c.tick(target, Some(v), Duration::from_millis(1)),
        Ok(Some(expiry))
    );
    let mut queue = LifeCommandQueue::default();
    c.enqueue(&mut queue, 0).unwrap();
    assert_eq!(c.enqueue(&mut queue, 1), Err(BleedoutClockError::InFlight));
    assert_eq!(c.tick(target, Some(v), Duration::from_secs(99)), Ok(None));
    assert_eq!(
        c.acknowledge(receipt(
            0,
            target,
            Ok(LifeCommandOutcome::Expiry(ExpiryOutcome::Deferred(expiry)))
        )),
        Ok(())
    );
    v.cancel_revive(target, token).unwrap();
    assert_eq!(c.tick(target, Some(v), Duration::from_millis(99)), Ok(None));
    assert_eq!(
        c.tick(target, Some(v), Duration::from_millis(1)),
        Ok(Some(expiry))
    );
    c.enqueue(&mut queue, 1).unwrap();
    let result = v
        .expire_downed(target, expiry)
        .map(LifeCommandOutcome::Expiry);
    assert_eq!(
        result,
        Ok(LifeCommandOutcome::Expiry(ExpiryOutcome::Terminated {
            current: 1,
            maximum: 100
        }))
    );
    c.acknowledge(receipt(1, target, result)).unwrap();
    assert!(c.is_retired());
    assert_eq!(c.tick(target, Some(v), Duration::from_secs(45)), Ok(None));
    assert_eq!(c.enqueue(&mut queue, 2), Err(BleedoutClockError::Retired));
}

#[test]
fn delayed_receipts_and_queue_rejections_are_atomic() {
    let target = Entity::from_bits(1);
    let v = downed();
    let mut c = BleedoutClock::new(target, v, timing()).unwrap();
    let mut queue = LifeCommandQueue::default();
    assert_eq!(c.enqueue(&mut queue, 0), Err(BleedoutClockError::NotDue));
    c.tick(target, Some(v), Duration::from_secs(45)).unwrap();
    c.enqueue(&mut queue, u64::MAX).unwrap();
    let before = c.clone();
    for bad in [
        receipt(0, target, Err(LifeStateError::StaleEpisode)),
        receipt(
            u64::MAX,
            Entity::from_bits(2),
            Err(LifeStateError::StaleEpisode),
        ),
        receipt(u64::MAX, target, Ok(LifeCommandOutcome::ReviveCanceled)),
    ] {
        assert_eq!(
            c.acknowledge(bad),
            Err(BleedoutClockError::UnexpectedAcknowledgement)
        );
        assert_eq!(c, before);
    }
    c.acknowledge(receipt(
        u64::MAX,
        target,
        Ok(LifeCommandOutcome::Expiry(ExpiryOutcome::Deferred(
            c.expiry(),
        ))),
    ))
    .unwrap();
    c.tick(target, Some(v), Duration::from_millis(100)).unwrap();
    let before = c.clone();
    assert_eq!(
        c.enqueue(&mut queue, 0),
        Err(BleedoutClockError::Lifecycle(
            LifeStateError::NonIncreasingRequestId
        ))
    );
    assert_eq!(c, before);
    c.retire();
    assert!(c.is_retired());
    assert_eq!(c.in_flight(), None);
}

#[test]
fn revive_redown_missing_and_wrong_identity_retire_without_alias() {
    let target = Entity::from_bits(1);
    let other = Entity::from_bits(2);
    let mut v = downed();
    let mut c = BleedoutClock::new(target, v, timing()).unwrap();
    let before = c.clone();
    assert_eq!(
        c.tick(other, Some(v), Duration::from_secs(45)),
        Err(BleedoutClockError::Lifecycle(LifeStateError::WrongTarget))
    );
    assert_eq!(c, before);
    let token = v
        .begin_revive(target, other, v.downed_episode().unwrap())
        .unwrap();
    v.complete_revive(target, token, ReviveRestoration::new(100, 100).unwrap())
        .unwrap();
    assert_eq!(c.tick(target, Some(v), Duration::ZERO), Ok(None));
    assert!(c.is_retired());
    let mut c = before.clone();
    v.apply(ResolvedDamage::new(100).unwrap()).unwrap();
    assert_eq!(c.tick(target, Some(v), Duration::from_secs(45)), Ok(None));
    assert!(c.is_retired());
    assert_ne!(
        BleedoutClock::new(target, v, timing()).unwrap().expiry(),
        before.expiry()
    );
    let mut c = before;
    assert_eq!(
        c.tick(target, None, Duration::ZERO),
        Err(BleedoutClockError::Lifecycle(LifeStateError::MissingTarget))
    );
    assert!(c.is_retired());
}

#[test]
fn invalid_configuration_and_unsupported_policy_fail_normally() {
    assert_eq!(
        BleedoutTiming::new(Duration::ZERO, Duration::from_millis(100)),
        Err(BleedoutClockError::ZeroCountdown)
    );
    assert_eq!(
        BleedoutTiming::new(Duration::from_millis(45001), Duration::from_millis(100)),
        Err(BleedoutClockError::UnsupportedFractionalCountdown)
    );
    assert_eq!(
        BleedoutTiming::new(Duration::from_secs(45), Duration::ZERO),
        Err(BleedoutClockError::ZeroPollInterval)
    );
    let target = Entity::from_bits(1);
    let mut v = downed();
    v.set_player_policy(PlayerLethalPolicy::SoloOrLastEligiblePlayer);
    assert_eq!(
        BleedoutClock::new(target, v, timing()),
        Err(BleedoutClockError::Lifecycle(
            LifeStateError::UnsupportedPlayerPolicy
        ))
    );
    assert_eq!(
        BleedoutClock::new(target, human(), timing()),
        Err(BleedoutClockError::Lifecycle(LifeStateError::NotDowned))
    );
    assert_eq!(
        BleedoutClock::new(
            target,
            Vitality::ordinary_actor(150, 150).unwrap(),
            timing()
        ),
        Err(BleedoutClockError::Lifecycle(LifeStateError::NotHuman))
    );
    v.set_player_policy(PlayerLethalPolicy::DownWithEligibleTeammate);
    let mut c = BleedoutClock::new(target, v, timing()).unwrap();
    v.set_player_policy(PlayerLethalPolicy::SoloOrLastEligiblePlayer);
    assert_eq!(
        c.tick(target, Some(v), Duration::from_secs(45)),
        Err(BleedoutClockError::Lifecycle(
            LifeStateError::UnsupportedPlayerPolicy
        ))
    );
    assert!(c.is_retired());
}

#[derive(Resource)]
struct Harness {
    target: Entity,
    next: u64,
    results: Vec<LifeCommandResult>,
}
fn produce(
    time: Res<Time<Fixed>>,
    mut clocks: Query<&mut BleedoutClock>,
    vitality: Query<&Vitality>,
    mut queue: ResMut<LifeCommandQueue>,
    mut harness: ResMut<Harness>,
) {
    let target = harness.target;
    let mut clock = clocks.get_mut(target).unwrap();
    if clock
        .tick(target, vitality.get(target).ok().copied(), time.delta())
        .unwrap()
        .is_some()
    {
        clock.enqueue(&mut queue, harness.next).unwrap();
        harness.next = harness.next.checked_add(1).unwrap();
    }
}
fn consume(
    mut clocks: Query<&mut BleedoutClock>,
    mut results: ResMut<LifeCommandResults>,
    mut harness: ResMut<Harness>,
) {
    for result in results.drain() {
        let mut c = clocks.get_mut(harness.target).unwrap();
        if c.in_flight() == Some(result.id) {
            c.acknowledge(result).unwrap();
        }
        harness.results.push(result);
    }
}
#[test]
fn real_fixed_queue_replays_with_render_pacing_and_deferred_retry() {
    fn run(pacing: &[u32]) -> (Vitality, BleedoutClock, Vec<LifeCommandResult>) {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, LifeStatePlugin));
        app.insert_resource(Time::<Fixed>::from_duration(Duration::from_millis(100)));
        app.world_mut()
            .resource_mut::<Time<Virtual>>()
            .set_max_delta(Duration::from_secs(1));
        app.insert_resource(TimeUpdateStrategy::FixedTimesteps(0));
        app.update();
        let target = app.world_mut().spawn(downed()).id();
        let reviver = app.world_mut().spawn(human()).id();
        let v = *app.world().get::<Vitality>(target).unwrap();
        let expiry = v.downed_expiry(target).unwrap();
        app.world_mut()
            .entity_mut(target)
            .insert(BleedoutClock::new(target, v, timing()).unwrap());
        app.insert_resource(Harness {
            target,
            next: 1,
            results: Vec::new(),
        });
        app.add_systems(FixedUpdate, produce.before(ApplyLifeCommands));
        app.add_systems(FixedUpdate, consume.after(ApplyLifeCommands));
        app.world_mut()
            .resource_mut::<LifeCommandQueue>()
            .submit(LifeCommandRequest {
                id: 0,
                target,
                command: LifeCommand::BeginRevive {
                    reviver,
                    episode: expiry.episode(),
                },
            })
            .unwrap();
        let mut steps = 0;
        let mut frame = 0;
        while steps < 450 {
            let n = pacing[frame % pacing.len()].min(450 - steps);
            app.insert_resource(TimeUpdateStrategy::FixedTimesteps(n));
            app.update();
            steps += n;
            frame += 1;
        }
        let h = app.world().resource::<Harness>();
        assert_eq!(h.results.len(), 2);
        assert_eq!(
            h.results[1].result,
            Ok(LifeCommandOutcome::Expiry(ExpiryOutcome::Deferred(expiry)))
        );
        let Ok(LifeCommandOutcome::ReviveBegan(token)) = h.results[0].result else {
            panic!("missing attempt")
        };
        app.world_mut()
            .resource_mut::<LifeCommandQueue>()
            .submit(LifeCommandRequest {
                id: 2,
                target,
                command: LifeCommand::CancelRevive(token),
            })
            .unwrap();
        app.world_mut().resource_mut::<Harness>().next = 3;
        app.insert_resource(TimeUpdateStrategy::FixedTimesteps(1));
        app.update();
        let h = app.world().resource::<Harness>();
        assert_eq!(h.results.len(), 4);
        assert_eq!(h.results[2].result, Ok(LifeCommandOutcome::ReviveCanceled));
        assert_eq!(
            h.results[3].result,
            Ok(LifeCommandOutcome::Expiry(ExpiryOutcome::Terminated {
                current: 1,
                maximum: 100
            }))
        );
        assert_eq!(
            app.world().resource::<Time<Fixed>>().elapsed(),
            Duration::from_millis(45100)
        );
        (
            *app.world().get::<Vitality>(target).unwrap(),
            app.world().get::<BleedoutClock>(target).unwrap().clone(),
            h.results.clone(),
        )
    }
    let expected = run(&[1]);
    assert_eq!(expected.0.state(), LifeState::HumanTerminal);
    assert!(expected.1.is_retired());
    for pacing in [&[1][..], &[0, 3, 1][..], &[2, 0, 1][..]] {
        assert_eq!(run(pacing), expected);
        assert_eq!(run(pacing), expected);
    }
}

#[test]
fn repeated_deferred_poll_and_explicit_component_retirement() {
    let target = Entity::from_bits(1);
    let mut v = downed();
    let token = v
        .begin_revive(target, Entity::from_bits(2), v.downed_episode().unwrap())
        .unwrap();
    let mut c = BleedoutClock::new(target, v, timing()).unwrap();
    let expiry = c.expiry();
    let mut queue = LifeCommandQueue::default();
    c.tick(target, Some(v), Duration::from_secs(45)).unwrap();
    for id in 0..3 {
        c.enqueue(&mut queue, id).unwrap();
        assert_eq!(
            v.expire_downed(target, expiry),
            Ok(ExpiryOutcome::Deferred(expiry))
        );
        c.acknowledge(receipt(
            id,
            target,
            Ok(LifeCommandOutcome::Expiry(ExpiryOutcome::Deferred(expiry))),
        ))
        .unwrap();
        let before = c.clone();
        assert_eq!(
            c.acknowledge(receipt(
                id,
                target,
                Ok(LifeCommandOutcome::Expiry(ExpiryOutcome::Deferred(expiry)))
            )),
            Err(BleedoutClockError::UnexpectedAcknowledgement)
        );
        assert_eq!(c, before);
        assert_eq!(c.tick(target, Some(v), Duration::from_millis(99)), Ok(None));
        assert_eq!(
            c.tick(target, Some(v), Duration::from_millis(1)),
            Ok(Some(expiry))
        );
    }
    c.enqueue(&mut queue, 3).unwrap();
    v.complete_revive(target, token, ReviveRestoration::new(100, 100).unwrap())
        .unwrap();
    assert_eq!(c.tick(target, Some(v), Duration::ZERO), Ok(None));
    assert!(c.is_retired());
    // Assembly retires the queue and held clock before same-Entity replacement.
    assert_eq!(queue.retire_pending_target(target).len(), 4);
    assert_eq!(
        c.acknowledge(receipt(3, target, Err(LifeStateError::StaleEpisode))),
        Err(BleedoutClockError::Retired)
    );
    let replacement = downed();
    assert_eq!(
        c.tick(target, Some(replacement), Duration::from_secs(45)),
        Ok(None)
    );
    assert_eq!(c.enqueue(&mut queue, 4), Err(BleedoutClockError::Retired));
    let mut fresh = BleedoutClock::new(target, replacement, timing()).unwrap();
    fresh
        .tick(target, Some(replacement), Duration::from_secs(45))
        .unwrap();
    fresh.enqueue(&mut queue, 4).unwrap();
    fresh
        .acknowledge(receipt(4, target, Err(LifeStateError::MissingTarget)))
        .unwrap();
    assert!(fresh.is_retired());
}

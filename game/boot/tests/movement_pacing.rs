//! Integration of persistent receipts with Bevy's actual fixed schedule runner.
//! These synthetic durations establish host behavior, not native movement timing.
use std::time::Duration;

use bevy::{prelude::*, time::TimeUpdateStrategy};
use bo2_boot::{
    movement_host::{ConsumedMovementInput, MovementHostPlugin, consume_movement_input},
    movement_input::{HostInputEvent, MovementButton, OrderedHostInput, PendingMovementInput},
};

fn run(pacing: &[u64], reverse_creation: bool) -> Vec<OrderedHostInput> {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins(MovementHostPlugin)
        .insert_resource(Time::<Fixed>::from_duration(Duration::from_millis(10)))
        .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::ZERO))
        .add_systems(FixedUpdate, consume_movement_input);
    let ids = if reverse_creation { [2_u32, 1] } else { [1, 2] };
    for id in ids {
        app.world_mut().spawn(Name::new(id.to_string()));
    }
    app.update();
    let events = [
        HostInputEvent::CursorCaptured(true),
        HostInputEvent::Button {
            button: MovementButton::Jump,
            pressed: true,
        },
        HostInputEvent::Button {
            button: MovementButton::Jump,
            pressed: false,
        },
        HostInputEvent::Mouse { delta: [2.5, -1.0] },
        HostInputEvent::Focus(false),
    ];
    for event in events {
        app.world_mut()
            .resource_mut::<PendingMovementInput>()
            .push(event)
            .unwrap();
    }
    for &millis in pacing {
        app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_millis(
            millis,
        )));
        app.update();
    }
    assert!(
        app.world()
            .resource::<PendingMovementInput>()
            .pending()
            .is_empty()
    );
    let receipts = app.world().resource::<ConsumedMovementInput>().0.clone();
    assert_eq!(
        receipts
            .iter()
            .map(|receipt| receipt.event)
            .collect::<Vec<_>>(),
        events
    );
    receipts
}

#[test]
fn actual_fixed_runner_preserves_receipts_under_zero_and_multiple_ticks() {
    let expected = run(&[40], false);
    assert_eq!(run(&[1; 40], false), expected);
    assert_eq!(run(&[0, 0, 20, 0, 20], false), expected);
    assert_eq!(run(&[0, 40, 0], true), expected);
}

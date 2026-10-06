use bevy::prelude::*;
use bo2_boot::{
    movement_host::{ConsumedMovementInput, MovementHostPlugin, consume_movement_input},
    movement_input::{
        HostInputError, HostInputEvent as Event, MovementButton as Button, PendingMovementInput,
    },
};

#[test]
fn zero_fixed_updates_keep_short_edges_and_mouse_until_consumed_once() {
    let mut app = App::new();
    app.init_schedule(Update);
    app.add_plugins(MovementHostPlugin)
        .add_systems(FixedUpdate, consume_movement_input);
    let expected = [
        Event::CursorCaptured(true),
        Event::Button {
            button: Button::Jump,
            pressed: true,
        },
        Event::Button {
            button: Button::Jump,
            pressed: false,
        },
        Event::Mouse { delta: [4.0, -2.0] },
    ];
    for event in expected {
        app.world_mut()
            .resource_mut::<PendingMovementInput>()
            .push(event)
            .unwrap();
    }
    for _ in 0..5 {
        app.world_mut().run_schedule(Update);
    }
    assert_eq!(
        app.world()
            .resource::<PendingMovementInput>()
            .pending()
            .len(),
        4
    );
    app.world_mut().run_schedule(FixedUpdate);
    app.world_mut().run_schedule(FixedUpdate);
    let consumed = &app.world().resource::<ConsumedMovementInput>().0;
    assert_eq!(
        consumed
            .iter()
            .map(|record| record.event)
            .collect::<Vec<_>>(),
        expected
    );
    assert_eq!(
        consumed
            .iter()
            .map(|record| record.sequence)
            .collect::<Vec<_>>(),
        [0, 1, 2, 3]
    );
    assert!(
        !app.world()
            .resource::<PendingMovementInput>()
            .held(Button::Jump)
    );
}

#[test]
fn focus_loss_and_cursor_release_clear_held_and_gate_mouse() {
    let mut input = PendingMovementInput::default();
    input.push(Event::CursorCaptured(true)).unwrap();
    input
        .push(Event::Button {
            button: Button::Forward,
            pressed: true,
        })
        .unwrap();
    input.push(Event::Focus(false)).unwrap();
    assert!(!input.held(Button::Forward));
    assert!(!input.captured());
    let size = input.pending().len();
    input.push(Event::Mouse { delta: [10.0, 5.0] }).unwrap();
    input
        .push(Event::Button {
            button: Button::Jump,
            pressed: true,
        })
        .unwrap();
    assert_eq!(input.pending().len(), size);
    input.push(Event::Focus(true)).unwrap();
    input.push(Event::CursorCaptured(true)).unwrap();
    input
        .push(Event::Button {
            button: Button::Sprint,
            pressed: true,
        })
        .unwrap();
    input.push(Event::CursorCaptured(false)).unwrap();
    assert!(!input.held(Button::Sprint));
    assert_eq!(
        input.drain().last().unwrap().event,
        Event::CursorCaptured(false)
    );
    assert!(input.drain().is_empty());
}

#[test]
fn rejected_event_is_atomic_and_consumption_releases_capacity() {
    let mut input = PendingMovementInput::new(1);
    input.push(Event::CursorCaptured(true)).unwrap();
    assert_eq!(
        input.push(Event::Focus(false)),
        Err(HostInputError::Capacity)
    );
    assert!(input.focused());
    assert!(input.captured());
    assert_eq!(
        input.push(Event::Mouse {
            delta: [f32::NAN, 0.0]
        }),
        Err(HostInputError::NonFiniteMouse)
    );
    input.drain();
    input.push(Event::Focus(false)).unwrap();
    assert_eq!(input.pending().front().unwrap().sequence, 1);
}

#[test]
fn render_partition_does_not_change_ordered_authoritative_host_receipts() {
    let events = [
        Event::Button {
            button: Button::Crouch,
            pressed: true,
        },
        Event::Button {
            button: Button::Crouch,
            pressed: false,
        },
        Event::Button {
            button: Button::Prone,
            pressed: true,
        },
        Event::Focus(false),
    ];
    let run = |batch: usize| {
        let mut input = PendingMovementInput::default();
        let mut result = Vec::new();
        for chunk in events.chunks(batch) {
            for event in chunk {
                input.push(*event).unwrap();
            }
            result.extend(input.drain());
        }
        result
    };
    assert_eq!(run(1), run(4));
    assert_eq!(run(2), run(4));
}

#[cfg(feature = "client")]
#[test]
fn client_collector_retains_keyboard_edges_and_releases_on_focus_or_cursor_loss() {
    use bevy::{
        input::{
            ButtonState,
            keyboard::{Key, KeyboardInput, NativeKey},
            mouse::MouseMotion,
        },
        window::{CursorGrabMode, CursorOptions, WindowFocused},
    };
    use bo2_boot::movement_input::{ClientMovementInputPlugin, MovementInputBindings};
    let mut app = App::new();
    app.add_message::<KeyboardInput>()
        .add_message::<MouseMotion>()
        .add_message::<WindowFocused>()
        .add_plugins(ClientMovementInputPlugin);
    let window = app
        .world_mut()
        .spawn((
            Window {
                focused: true,
                ..default()
            },
            CursorOptions {
                grab_mode: CursorGrabMode::Locked,
                ..default()
            },
        ))
        .id();
    app.insert_resource(MovementInputBindings {
        window,
        keys: vec![(KeyCode::Space, Button::Jump)],
    });
    for state in [ButtonState::Pressed, ButtonState::Released] {
        app.world_mut().write_message(KeyboardInput {
            key_code: KeyCode::Space,
            logical_key: Key::Unidentified(NativeKey::Unidentified),
            state,
            text: None,
            repeat: false,
            window,
        });
    }
    app.world_mut().write_message(MouseMotion {
        delta: Vec2::new(3., -7.),
    });
    app.world_mut().run_schedule(PreUpdate);
    let first = app
        .world_mut()
        .resource_mut::<PendingMovementInput>()
        .drain();
    assert_eq!(
        first.iter().map(|event| event.event).collect::<Vec<_>>(),
        [
            Event::CursorCaptured(true),
            Event::Button {
                button: Button::Jump,
                pressed: true
            },
            Event::Button {
                button: Button::Jump,
                pressed: false
            },
            Event::Mouse { delta: [3., -7.] }
        ]
    );
    app.world_mut().run_schedule(PreUpdate);
    assert!(
        app.world()
            .resource::<PendingMovementInput>()
            .pending()
            .is_empty()
    );
    app.world_mut().get_mut::<Window>(window).unwrap().focused = false;
    app.world_mut().write_message(WindowFocused {
        window,
        focused: false,
    });
    app.world_mut()
        .write_message(MouseMotion { delta: Vec2::ONE });
    app.world_mut().run_schedule(PreUpdate);
    assert_eq!(
        app.world_mut()
            .resource_mut::<PendingMovementInput>()
            .drain()
            .iter()
            .map(|event| event.event)
            .collect::<Vec<_>>(),
        [Event::Focus(false)]
    );
    app.world_mut().get_mut::<Window>(window).unwrap().focused = true;
    app.world_mut().run_schedule(PreUpdate);
    app.world_mut()
        .resource_mut::<PendingMovementInput>()
        .drain();
    app.world_mut()
        .resource_mut::<PendingMovementInput>()
        .push(Event::Button {
            button: Button::Jump,
            pressed: true,
        })
        .unwrap();
    app.world_mut()
        .get_mut::<CursorOptions>(window)
        .unwrap()
        .grab_mode = CursorGrabMode::None;
    app.world_mut().run_schedule(PreUpdate);
    assert!(
        !app.world()
            .resource::<PendingMovementInput>()
            .held(Button::Jump)
    );
    assert_eq!(
        app.world()
            .resource::<PendingMovementInput>()
            .pending()
            .back()
            .unwrap()
            .event,
        Event::CursorCaptured(false)
    );
    app.world_mut().write_message(MouseMotion {
        delta: Vec2::new(f32::NAN, 0.0),
    });
    app.world_mut().run_schedule(PreUpdate);
    assert_eq!(
        app.world()
            .resource::<bo2_boot::movement_input::MovementInputFailures>()
            .0,
        [HostInputError::NonFiniteMouse]
    );
    app.world_mut()
        .resource_mut::<PendingMovementInput>()
        .drain();
    assert!(app.world_mut().despawn(window));
    app.world_mut().run_schedule(PreUpdate);
    assert!(!app.world().resource::<PendingMovementInput>().focused());
    assert_eq!(
        app.world()
            .resource::<PendingMovementInput>()
            .pending()
            .back()
            .unwrap()
            .event,
        Event::Focus(false)
    );
}

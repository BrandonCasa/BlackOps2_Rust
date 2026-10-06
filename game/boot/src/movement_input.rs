//! Ordered host input. No key mapping, native timing or camera sensitivity is implied.
use bevy::prelude::*;
use std::collections::{BTreeSet, VecDeque};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum MovementButton {
    Forward,
    Backward,
    Left,
    Right,
    Sprint,
    Jump,
    Crouch,
    Prone,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum HostInputEvent {
    Button {
        button: MovementButton,
        pressed: bool,
    },
    Mouse {
        delta: [f32; 2],
    },
    Focus(bool),
    CursorCaptured(bool),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OrderedHostInput {
    pub sequence: u64,
    pub event: HostInputEvent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostInputError {
    Capacity,
    SequenceExhausted,
    NonFiniteMouse,
}

/// Persistent across render updates; only an explicit consumer removes events.
#[derive(Resource)]
pub struct PendingMovementInput {
    events: VecDeque<OrderedHostInput>,
    held: BTreeSet<MovementButton>,
    focused: bool,
    captured: bool,
    next_sequence: u64,
    capacity: usize,
}

impl Default for PendingMovementInput {
    fn default() -> Self {
        Self::new(4096)
    }
}

impl PendingMovementInput {
    pub fn new(capacity: usize) -> Self {
        Self {
            events: VecDeque::new(),
            held: BTreeSet::new(),
            focused: true,
            captured: false,
            next_sequence: 0,
            capacity,
        }
    }
    pub fn held(&self, button: MovementButton) -> bool {
        self.held.contains(&button)
    }
    pub fn focused(&self) -> bool {
        self.focused
    }
    pub fn captured(&self) -> bool {
        self.captured
    }
    pub fn pending(&self) -> &VecDeque<OrderedHostInput> {
        &self.events
    }

    /// Invalid/capacity-failed events leave queue and authoritative host state unchanged.
    pub fn push(&mut self, event: HostInputEvent) -> Result<(), HostInputError> {
        let event = match event {
            HostInputEvent::CursorCaptured(value) => {
                HostInputEvent::CursorCaptured(value && self.focused)
            }
            event => event,
        };
        if let HostInputEvent::Mouse { delta } = event {
            if !delta.into_iter().all(f32::is_finite) {
                return Err(HostInputError::NonFiniteMouse);
            }
            if !self.focused || !self.captured {
                return Ok(());
            }
        }
        if matches!(event, HostInputEvent::Button { .. }) && !self.focused {
            return Ok(());
        }
        if self.events.len() >= self.capacity {
            return Err(HostInputError::Capacity);
        }
        let next = self
            .next_sequence
            .checked_add(1)
            .ok_or(HostInputError::SequenceExhausted)?;
        match event {
            HostInputEvent::Button { button, pressed } => {
                if pressed {
                    self.held.insert(button);
                } else {
                    self.held.remove(&button);
                }
            }
            HostInputEvent::Focus(value) => {
                self.focused = value;
                if !value {
                    self.held.clear();
                    self.captured = false;
                }
            }
            HostInputEvent::CursorCaptured(value) => {
                self.captured = value && self.focused;
                if !self.captured {
                    self.held.clear();
                }
            }
            HostInputEvent::Mouse { .. } => {}
        }
        self.events.push_back(OrderedHostInput {
            sequence: self.next_sequence,
            event,
        });
        self.next_sequence = next;
        Ok(())
    }
    pub fn drain(&mut self) -> Vec<OrderedHostInput> {
        self.events.drain(..).collect()
    }
}

pub struct MovementInputPlugin;
impl Plugin for MovementInputPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PendingMovementInput>();
    }
}

/// The assembly explicitly selects bindings and its one controlled window.
#[cfg(feature = "client")]
#[derive(Resource)]
pub struct MovementInputBindings {
    pub window: Entity,
    pub keys: Vec<(KeyCode, MovementButton)>,
}

#[derive(Resource, Default)]
pub struct MovementInputFailures(pub Vec<HostInputError>);

#[cfg(feature = "client")]
pub struct ClientMovementInputPlugin;

#[cfg(feature = "client")]
impl Plugin for ClientMovementInputPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PendingMovementInput>()
            .init_resource::<MovementInputFailures>()
            .add_systems(
                PreUpdate,
                collect_client_input.after(bevy::input::InputSystems),
            );
    }
}

/// Bevy channels have no common event timestamp. Host policy processes focus and
/// capture changes first, then ordered keyboard edges, then mouse deltas. This
/// does not reconstruct cross-channel OS ordering or native command time.
#[cfg(feature = "client")]
fn collect_client_input(
    bindings: Option<Res<MovementInputBindings>>,
    windows: Query<(&Window, &bevy::window::CursorOptions)>,
    mut keyboard: MessageReader<bevy::input::keyboard::KeyboardInput>,
    mut mouse: MessageReader<bevy::input::mouse::MouseMotion>,
    mut focus: MessageReader<bevy::window::WindowFocused>,
    mut pending: ResMut<PendingMovementInput>,
    mut failures: ResMut<MovementInputFailures>,
) {
    let Some(bindings) = bindings else {
        keyboard.clear();
        mouse.clear();
        focus.clear();
        return;
    };
    for event in focus.read().filter(|event| event.window == bindings.window) {
        if let Err(error) = pending.push(HostInputEvent::Focus(event.focused)) {
            failures.0.push(error);
        }
    }
    let (focused, captured) =
        windows
            .get(bindings.window)
            .map_or((false, false), |(window, cursor)| {
                (
                    window.focused,
                    cursor.grab_mode != bevy::window::CursorGrabMode::None,
                )
            });
    if pending.focused() != focused
        && let Err(error) = pending.push(HostInputEvent::Focus(focused))
    {
        failures.0.push(error);
    }
    if pending.captured() != (captured && focused)
        && let Err(error) = pending.push(HostInputEvent::CursorCaptured(captured && focused))
    {
        failures.0.push(error);
    }
    for event in keyboard
        .read()
        .filter(|event| event.window == bindings.window && !event.repeat)
    {
        for (_, button) in bindings
            .keys
            .iter()
            .filter(|(key, _)| *key == event.key_code)
        {
            if let Err(error) = pending.push(HostInputEvent::Button {
                button: *button,
                pressed: event.state == bevy::input::ButtonState::Pressed,
            }) {
                failures.0.push(error);
            }
        }
    }
    for event in mouse.read() {
        if let Err(error) = pending.push(HostInputEvent::Mouse {
            delta: event.delta.to_array(),
        }) {
            failures.0.push(error);
        }
    }
}

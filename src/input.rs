use crate::GameState;
use bevy::{
    input::{
        InputSystems,
        keyboard::KeyboardInput,
        mouse::{MouseButtonInput, MouseMotion},
    },
    prelude::*,
    window::{CursorGrabMode, CursorOptions},
};

#[derive(Resource, Default)]
pub struct GameInput {
    pub movement: Vec2,
    pub sprint: bool,
    pub crouch: bool,
    pub jump: bool,
    pub fire: bool,
    pub melee: bool,
    pub reload: bool,
    pub look: Vec2,
}

impl GameInput {
    pub fn take_edges(&mut self) -> (bool, bool, bool, bool) {
        let result = (self.jump, self.fire, self.melee, self.reload);
        self.jump = false;
        self.fire = false;
        self.melee = false;
        self.reload = false;
        result
    }
    pub fn clear(&mut self) {
        *self = Self::default();
    }
}

pub struct GameInputPlugin;
#[derive(Message)]
pub struct RestartGame;
impl Plugin for GameInputPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GameInput>()
            .add_message::<RestartGame>()
            .add_systems(PreUpdate, collect.after(InputSystems))
            .add_systems(Update, pause_and_restart);
    }
}

pub(crate) fn collect(
    keys: Res<ButtonInput<KeyCode>>,
    mut key_events: MessageReader<KeyboardInput>,
    mut mouse_events: MessageReader<MouseButtonInput>,
    mut motion: MessageReader<MouseMotion>,
    windows: Query<(&Window, &CursorOptions)>,
    state: Res<State<GameState>>,
    options: Res<crate::assets::LaunchOptions>,
    mut input: ResMut<GameInput>,
) {
    let active = **state == GameState::Playing
        && (options.verify.is_some()
            || windows.iter().any(|(window, cursor)| {
                window.focused && cursor.grab_mode != CursorGrabMode::None
            }));
    if !active {
        input.clear();
        key_events.clear();
        mouse_events.clear();
        motion.clear();
        return;
    }
    input.movement = Vec2::new(
        (keys.pressed(KeyCode::KeyD) as i8 - keys.pressed(KeyCode::KeyA) as i8) as f32,
        (keys.pressed(KeyCode::KeyW) as i8 - keys.pressed(KeyCode::KeyS) as i8) as f32,
    )
    .clamp_length_max(1.);
    input.sprint = keys.pressed(KeyCode::ShiftLeft);
    input.crouch = keys.pressed(KeyCode::ControlLeft);
    for event in key_events
        .read()
        .filter(|e| e.state.is_pressed() && !e.repeat)
    {
        match event.key_code {
            KeyCode::Space => input.jump = true,
            KeyCode::KeyR => input.reload = true,
            KeyCode::KeyV => input.melee = true,
            _ => {}
        }
    }
    for event in mouse_events.read().filter(|e| e.state.is_pressed()) {
        match event.button {
            MouseButton::Left => input.fire = true,
            MouseButton::Right => input.melee = true,
            _ => {}
        }
    }
    for event in motion.read() {
        input.look += event.delta;
    }
}

fn pause_and_restart(
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    state: Res<State<GameState>>,
    mut next: ResMut<NextState<GameState>>,
    mut cursors: Query<(&Window, &mut CursorOptions)>,
    mut commands: Commands,
    session: Query<Entity, With<crate::gameplay::SessionEntity>>,
    options: Res<crate::assets::LaunchOptions>,
    mut restart: MessageReader<RestartGame>,
) {
    if keys.just_pressed(KeyCode::Escape) {
        match **state {
            GameState::Playing => next.set(GameState::Paused),
            GameState::Paused => next.set(GameState::Playing),
            _ => {}
        }
    }
    if options.verify.is_none()
        && **state == GameState::Playing
        && cursors.iter().any(|(w, _)| !w.focused)
    {
        next.set(GameState::Paused);
    }
    if **state == GameState::GameOver
        && (keys.just_pressed(KeyCode::KeyR) || restart.read().next().is_some())
    {
        for entity in &session {
            commands.entity(entity).despawn();
        }
        next.set(GameState::Playing);
    }
    for (window, mut cursor) in &mut cursors {
        let capture = options.verify.is_none() && window.focused && **state == GameState::Playing;
        cursor.grab_mode = if capture {
            CursorGrabMode::Locked
        } else {
            CursorGrabMode::None
        };
        cursor.visible = !capture;
    }
    if **state == GameState::Paused && mouse.just_pressed(MouseButton::Left) {
        next.set(GameState::Playing);
    }
}

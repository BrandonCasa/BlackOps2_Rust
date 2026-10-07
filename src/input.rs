use crate::GameState;
use bevy::{
    input::{
        InputSystems,
        keyboard::KeyboardInput,
        mouse::{MouseButtonInput, MouseMotion, MouseWheel},
    },
    prelude::*,
    window::{CursorGrabMode, CursorOptions},
};

#[derive(Resource, Default)]
pub struct GameInput {
    pub movement: Vec2,
    pub sprint: bool,
    pub crouch: bool,
    pub prone: bool,
    pub jump: bool,
    pub fire: bool,
    pub fire_held: bool,
    pub ads: bool,
    pub cycle_weapon: bool,
    pub weapon_slot: Option<usize>,
    pub melee: bool,
    pub reload: bool,
    pub use_held: bool,
    pub grenade_held: bool,
    pub mine_held: bool,
    pub tactical_held: bool,
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
    mut wheel: MessageReader<MouseWheel>,
    mouse: Res<ButtonInput<MouseButton>>,
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
        wheel.clear();
        return;
    }
    input.movement = Vec2::new(
        (keys.pressed(KeyCode::KeyD) as i8 - keys.pressed(KeyCode::KeyA) as i8) as f32,
        (keys.pressed(KeyCode::KeyW) as i8 - keys.pressed(KeyCode::KeyS) as i8) as f32,
    )
    .clamp_length_max(1.);
    input.sprint = keys.pressed(KeyCode::ShiftLeft);
    input.crouch = keys.pressed(KeyCode::ControlLeft);
    input.fire_held = mouse.pressed(MouseButton::Left);
    input.ads = mouse.pressed(MouseButton::Right);
    input.use_held = keys.pressed(KeyCode::KeyF);
    input.grenade_held = keys.pressed(KeyCode::KeyG);
    input.mine_held = keys.pressed(KeyCode::Digit4);
    input.tactical_held = keys.pressed(KeyCode::KeyT);
    for event in wheel.read() {
        input.cycle_weapon |= event.y != 0.;
    }
    for event in key_events
        .read()
        .filter(|e| e.state.is_pressed() && !e.repeat)
    {
        match event.key_code {
            KeyCode::Space => input.jump = true,
            KeyCode::KeyZ => input.prone = true,
            KeyCode::KeyR => input.reload = true,
            KeyCode::KeyV => input.melee = true,
            KeyCode::KeyQ => input.cycle_weapon = true,
            KeyCode::Digit1 => input.weapon_slot = Some(0),
            KeyCode::Digit2 => input.weapon_slot = Some(1),
            KeyCode::Digit3 => input.weapon_slot = Some(2),
            _ => {}
        }
    }
    for event in mouse_events.read().filter(|e| e.state.is_pressed()) {
        match event.button {
            MouseButton::Left => input.fire = true,
            _ => {}
        }
    }
    for event in motion.read() {
        input.look += event.delta;
    }
}

pub(crate) fn pause_and_restart(
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    state: Res<State<GameState>>,
    mut next: ResMut<NextState<GameState>>,
    mut cursors: Query<(&Window, &mut CursorOptions)>,
    mut commands: Commands,
    session: Query<Entity, With<crate::gameplay::SessionEntity>>,
    options: Res<crate::assets::LaunchOptions>,
    mut restart: MessageReader<RestartGame>,
    mut life_queue: ResMut<crate::life::LifeCommandQueue>,
    mut life_results: ResMut<crate::life::LifeCommandResults>,
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
            life_queue.retire_pending_target(entity);
            commands.entity(entity).despawn();
        }
        life_results.drain().for_each(drop);
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

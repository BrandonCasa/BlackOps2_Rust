use crate::GameState;
use bevy::{
    input::{
        InputSystems,
        keyboard::KeyboardInput,
        mouse::{MouseButtonInput, MouseMotion, MouseWheel},
    },
    prelude::*,
    window::{
        CursorGrabMode, CursorOptions, MonitorSelection, PrimaryWindow, VideoModeSelection,
        WindowMode,
    },
};
use std::collections::HashSet;
mod bindings;
pub(crate) mod menu;
pub mod settings;
pub use bindings::{Action, Binding, ControlSettings, PressMode};

#[derive(Clone, Copy, Default, Debug)]
pub struct ActionState {
    pub held: bool,
    pub pressed: bool,
    pub released: bool,
    toggled: bool,
}
#[derive(Resource)]
pub struct Actions {
    states: [ActionState; Action::ALL.len()],
    physical: HashSet<Binding>,
    suppressed: HashSet<Binding>,
    settings: ControlSettings,
}
impl Default for Actions {
    fn default() -> Self {
        Self {
            states: [ActionState::default(); Action::ALL.len()],
            physical: default(),
            suppressed: default(),
            settings: ControlSettings::default(),
        }
    }
}
impl Actions {
    pub fn state(&self, action: Action) -> ActionState {
        self.states[action as usize]
    }
    fn active(&self, action: Action) -> bool {
        let state = self.state(action);
        if self.settings.mode(action) == PressMode::Toggle {
            state.toggled
        } else {
            state.held || state.pressed
        }
    }
    fn transition(&mut self, action: Action, held: bool) {
        let state = &mut self.states[action as usize];
        if held != state.held {
            if held {
                state.pressed = true;
                state.toggled = !state.toggled;
            } else {
                state.released = true;
            }
            state.held = held;
        }
    }
    fn event(&mut self, binding: Binding, pressed: bool) {
        if pressed && self.suppressed.contains(&binding) {
            return;
        }
        if pressed {
            self.physical.insert(binding);
        } else {
            self.physical.remove(&binding);
            self.suppressed.remove(&binding);
        }
        for action in Action::ALL {
            let held = self.settings.bindings[&action]
                .iter()
                .any(|b| self.physical.contains(b));
            self.transition(action, held);
        }
    }
    fn reset(
        &mut self,
        keys: &ButtonInput<KeyCode>,
        mouse: &ButtonInput<MouseButton>,
        preserve_posture: bool,
    ) {
        let posture = [Action::Aim, Action::Crouch].map(|a| self.state(a).toggled);
        self.states.fill(ActionState::default());
        if preserve_posture {
            for (action, toggled) in [Action::Aim, Action::Crouch].into_iter().zip(posture) {
                self.states[action as usize].toggled = toggled;
            }
        }
        self.physical.clear();
        self.suppressed = keys
            .get_pressed()
            .map(|key| Binding::Key(*key))
            .chain(mouse.get_pressed().map(|b| Binding::Mouse(*b)))
            .collect();
    }
}

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
    pub scoreboard: bool,
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
        let options = app.world().resource::<crate::assets::LaunchOptions>();
        let path = options
            .verify
            .as_ref()
            .map(|root| root.join("settings.json"))
            .or_else(settings::local_path);
        let (settings, store) = settings::SettingsStore::load(path);
        if let Some(error) = &store.error {
            warn!("{error}");
        }
        app.insert_resource(settings)
            .insert_resource(store)
            .init_resource::<Actions>();
        app.init_resource::<GameInput>()
            .add_message::<RestartGame>()
            .add_systems(
                PreUpdate,
                toggle_fullscreen
                    .after(InputSystems)
                    .before(collect)
                    .run_if(|options: Res<crate::assets::LaunchOptions>| !options.verify_ads),
            )
            .add_systems(
                PreUpdate,
                collect
                    .after(InputSystems)
                    .run_if(|options: Res<crate::assets::LaunchOptions>| {
                        options.validation_remote.is_none() && !options.flythrough
                    }),
            )
            .add_systems(
                Update,
                pause_and_restart.run_if(|options: Res<crate::assets::LaunchOptions>| {
                    options.validation_remote.is_none() && !options.flythrough
                }),
            )
            .add_systems(
                FixedUpdate,
                finish_tick
                    .after(crate::gameplay::GameStep::Rounds)
                    .run_if(in_state(GameState::Playing)),
            )
            .add_systems(Startup, settings::apply_audio)
            .add_systems(Last, (settings::capture_audio, settings::save).chain());
        menu::install(app);
    }
}

fn toggle_fullscreen(
    mut keys: ResMut<ButtonInput<KeyCode>>,
    mut windows: Query<&mut Window, With<PrimaryWindow>>,
) {
    if !keys.just_pressed(KeyCode::Enter)
        || !(keys.pressed(KeyCode::AltLeft) || keys.pressed(KeyCode::AltRight))
    {
        return;
    }
    let Ok(mut window) = windows.single_mut() else {
        return;
    };
    if !window.focused {
        return;
    }
    window.mode = match window.mode {
        WindowMode::Windowed => {
            WindowMode::Fullscreen(MonitorSelection::Current, VideoModeSelection::Current)
        }
        _ => WindowMode::Windowed,
    };
    keys.clear_just_pressed(KeyCode::Enter);
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
    settings: Res<ControlSettings>,
    mut actions: ResMut<Actions>,
    mut input: ResMut<GameInput>,
) {
    let active = **state == GameState::Playing
        && ((options.verify.is_some() && !options.verify_controls)
            || options.validation_remote.is_some()
            || windows.iter().any(|(window, cursor)| {
                window.focused && cursor.grab_mode != CursorGrabMode::None
            }));
    let mappings_changed = settings.bindings != actions.settings.bindings
        || settings.aim_mode != actions.settings.aim_mode
        || settings.crouch_mode != actions.settings.crouch_mode
        || settings.sprint_mode != actions.settings.sprint_mode;
    if !active || mappings_changed {
        input.clear();
        actions.settings = settings.clone();
        actions.reset(
            &keys,
            &mouse,
            !mappings_changed && matches!(**state, GameState::Playing | GameState::Paused),
        );
        key_events.clear();
        mouse_events.clear();
        motion.clear();
        wheel.clear();
        return;
    }
    if *settings != actions.settings {
        actions.settings = settings.clone();
    }
    for event in key_events.read().filter(|e| !e.repeat) {
        if event.key_code == KeyCode::Enter
            && (keys.pressed(KeyCode::AltLeft) || keys.pressed(KeyCode::AltRight))
        {
            continue;
        }
        actions.event(Binding::Key(event.key_code), event.state.is_pressed());
    }
    for event in mouse_events.read() {
        actions.event(Binding::Mouse(event.button), event.state.is_pressed());
    }
    for event in wheel.read().filter(|e| e.y != 0.) {
        let binding = if event.y > 0. {
            Binding::WheelUp
        } else {
            Binding::WheelDown
        };
        actions.event(binding, true);
        actions.event(binding, false);
    }
    let stale = actions
        .physical
        .iter()
        .copied()
        .filter(|b| !b.held(&keys, &mouse))
        .collect::<Vec<_>>();
    for binding in stale {
        actions.event(binding, false);
    }
    actions.suppressed.retain(|b| b.held(&keys, &mouse));
    use Action::*;
    input.movement = Vec2::new(
        (actions.state(Right).held as i8 - actions.state(Left).held as i8) as f32,
        (actions.state(Forward).held as i8 - actions.state(Backward).held as i8) as f32,
    )
    .clamp_length_max(1.);
    input.sprint = actions.active(Sprint);
    input.crouch = actions.active(Crouch);
    input.fire_held = actions.active(Fire);
    input.ads = actions.active(Aim);
    input.use_held = actions.active(Interact);
    input.scoreboard = actions.active(Scoreboard);
    input.grenade_held = actions.active(Lethal);
    input.mine_held = actions.active(Mine);
    input.tactical_held = actions.active(Tactical);
    input.jump |= actions.state(Jump).pressed;
    input.prone |= actions.state(Prone).pressed;
    input.fire |= actions.state(Fire).pressed;
    input.reload |= actions.state(Reload).pressed;
    input.melee |= actions.state(Melee).pressed;
    input.cycle_weapon |= actions.state(CycleWeapon).pressed;
    for (slot, action) in [WeaponOne, WeaponTwo, WeaponThree].into_iter().enumerate() {
        if actions.state(action).pressed {
            input.weapon_slot = Some(slot);
        }
    }
    for event in motion.read() {
        input.look += event.delta;
    }
}

fn finish_tick(mut actions: ResMut<Actions>, mut input: ResMut<GameInput>) {
    let input = &mut *input;
    for (action, held) in [
        (Action::Fire, &mut input.fire_held),
        (Action::Interact, &mut input.use_held),
        (Action::Scoreboard, &mut input.scoreboard),
        (Action::Lethal, &mut input.grenade_held),
        (Action::Tactical, &mut input.tactical_held),
        (Action::Mine, &mut input.mine_held),
    ] {
        let state = actions.state(action);
        if state.pressed && !state.held {
            *held = false;
        }
    }
    for (action, held) in [
        (Action::Aim, &mut input.ads),
        (Action::Sprint, &mut input.sprint),
        (Action::Crouch, &mut input.crouch),
    ] {
        let state = actions.state(action);
        if actions.settings.mode(action) == PressMode::Hold && state.pressed && !state.held {
            *held = false;
        }
    }
    for state in &mut actions.states {
        state.pressed = false;
        state.released = false;
    }
}

pub(crate) fn pause_and_restart(
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    state: Res<State<GameState>>,
    mut next: ResMut<NextState<GameState>>,
    mut cursors: Query<(&Window, &mut CursorOptions)>,
    mut commands: Commands,
    options: Res<crate::assets::LaunchOptions>,
    settings: Res<ControlSettings>,
    menu: Res<menu::ControlMenu>,
    mut restart: MessageReader<RestartGame>,
) {
    if options.flythrough {
        return;
    }
    if menu.capturing.is_none()
        && !menu.handled_input
        && (keys.just_pressed(KeyCode::Escape)
            || settings.just_pressed(Action::Pause, &keys, &mouse))
    {
        match **state {
            GameState::Playing => next.set(GameState::Paused),
            GameState::Paused => next.set(GameState::Playing),
            _ => {}
        }
    }
    if options.validation_remote.is_none()
        && (options.verify.is_none() || options.verify_controls)
        && **state == GameState::Playing
        && cursors.iter().any(|(w, _)| !w.focused)
    {
        next.set(GameState::Paused);
    }
    if **state == GameState::GameOver
        && (settings.just_pressed(Action::Restart, &keys, &mouse)
            || restart.read().next().is_some())
    {
        commands.queue(crate::session::teardown);
        next.set(GameState::Playing);
    }
    for (window, mut cursor) in &mut cursors {
        let capture = (options.verify.is_none() || options.verify_controls)
            && window.focused
            && **state == GameState::Playing;
        let grab_mode = if capture {
            CursorGrabMode::Locked
        } else {
            CursorGrabMode::None
        };
        if cursor.grab_mode != grab_mode {
            cursor.grab_mode = grab_mode;
        }
        if cursor.visible == capture {
            cursor.visible = !capture;
        }
    }
}

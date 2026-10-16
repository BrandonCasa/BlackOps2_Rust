use crate::GameState;
use bevy::{
    input::{
        InputSystems,
        keyboard::KeyboardInput,
        mouse::{MouseButtonInput, MouseMotion, MouseWheel},
    },
    prelude::*,
    window::{CursorGrabMode, CursorOptions, MonitorSelection, PrimaryWindow, WindowMode},
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
    pub(crate) fn clear_toggle(&mut self, action: Action) {
        self.states[action as usize].toggled = false;
    }
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
                state.toggled = action == Action::Sprint || !state.toggled;
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
    pub buildable: bool,
    pub drone: bool,
    pub staff_secondary: bool,
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

pub(crate) fn reset(world: &mut World) {
    world.resource_scope(|world, mut actions: Mut<Actions>| {
        actions.settings = world.resource::<ControlSettings>().clone();
        actions.reset(
            world.resource::<ButtonInput<KeyCode>>(),
            world.resource::<ButtonInput<MouseButton>>(),
            false,
        );
    });
    world.resource_mut::<GameInput>().clear();
    world.resource_mut::<ButtonInput<KeyCode>>().clear();
    world.resource_mut::<ButtonInput<MouseButton>>().clear();
    world.resource_mut::<Messages<KeyboardInput>>().clear();
    world.resource_mut::<Messages<MouseButtonInput>>().clear();
    world.resource_mut::<Messages<MouseMotion>>().clear();
    world.resource_mut::<Messages<MouseWheel>>().clear();
    menu::reset(world);
}

pub struct GameInputPlugin;
#[derive(Message)]
pub struct RestartGame;
impl Plugin for GameInputPlugin {
    fn build(&self, app: &mut App) {
        let path = settings::local_path();
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
                toggle_fullscreen.after(InputSystems).before(collect),
            )
            .add_systems(PreUpdate, collect.after(InputSystems))
            .add_systems(Update, pause_and_restart)
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
        WindowMode::Windowed => WindowMode::BorderlessFullscreen(MonitorSelection::Current),
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
    windows: Query<&Window, With<PrimaryWindow>>,
    inventories: Query<&crate::weapons::WeaponInventory, With<crate::player::Player>>,
    state: Res<State<GameState>>,
    settings: Res<ControlSettings>,
    mut actions: ResMut<Actions>,
    mut input: ResMut<GameInput>,
    #[cfg(feature = "dev-tools")] validator: Option<Res<crate::validation::Controller>>,
) {
    #[cfg(feature = "dev-tools")]
    if validator
        .as_ref()
        .is_some_and(|controller| controller.owns_input())
    {
        key_events.clear();
        mouse_events.clear();
        motion.clear();
        wheel.clear();
        return;
    }
    let active =
        **state == GameState::Playing && windows.single().is_ok_and(|window| window.focused);
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
    let dual = inventories.single().is_ok_and(|inventory| {
        inventory
            .active()
            .is_some_and(|weapon| weapon.left_magazine.is_some())
    });
    write_actions(&mut actions, &mut input, dual);
    for event in motion.read() {
        input.look += event.delta;
    }
}

fn write_actions(actions: &mut Actions, input: &mut GameInput, dual: bool) {
    use Action::*;
    if dual {
        actions.states[Aim as usize].toggled = false;
    }
    input.movement = Vec2::new(
        (actions.state(Right).held as i8 - actions.state(Left).held as i8) as f32,
        (actions.state(Forward).held as i8 - actions.state(Backward).held as i8) as f32,
    );
    input.sprint = actions.active(Sprint);
    input.crouch = actions.active(Crouch);
    input.fire_held = actions.active(Fire);
    input.ads = !dual && actions.active(Aim);
    input.use_held = actions.active(Interact);
    input.scoreboard = actions.active(Scoreboard);
    input.grenade_held = actions.active(Lethal);
    input.mine_held = actions.active(Mine);
    input.tactical_held = actions.active(Tactical);
    input.buildable |= actions.state(Buildable).pressed;
    input.drone |= actions.state(Drone).pressed;
    input.staff_secondary |= actions.state(StaffSecondary).pressed;
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
}

/// Development input uses the same transitions, toggles, and edge projection as physical input.
#[cfg(feature = "dev-tools")]
pub(crate) fn scripted_tick(world: &mut World, held: &[Action], look: Vec2) {
    let dual = world
        .query_filtered::<&crate::weapons::WeaponInventory, With<crate::player::Player>>()
        .single(world)
        .is_ok_and(|inventory| {
            inventory
                .active()
                .is_some_and(|weapon| weapon.left_magazine.is_some())
        });
    world.resource_scope(|world, mut actions: Mut<Actions>| {
        for action in Action::ALL {
            actions.transition(action, held.contains(&action));
        }
        let mut input = world.resource_mut::<GameInput>();
        write_actions(&mut actions, &mut input, dual);
        input.look = look;
    });
}

#[cfg(feature = "dev-tools")]
#[derive(Clone)]
pub(crate) struct SuspendedActions([ActionState; Action::ALL.len()]);

#[cfg(feature = "dev-tools")]
pub(crate) fn suspend_script(world: &mut World) -> SuspendedActions {
    let mut saved = world.resource::<Actions>().states;
    for state in &mut saved {
        state.pressed = false;
        state.released = false;
    }
    reset(world);
    world.insert_resource(crate::player::TickActions::default());
    SuspendedActions(saved)
}

#[cfg(feature = "dev-tools")]
pub(crate) fn resume_script(world: &mut World, saved: SuspendedActions) {
    world.resource_mut::<Actions>().states = saved.0;
}

pub(crate) fn finish_tick(mut actions: ResMut<Actions>, mut input: ResMut<GameInput>) {
    input.buildable = false;
    input.drone = false;
    input.staff_secondary = false;
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
    settings: Res<ControlSettings>,
    menu: Res<menu::ControlMenu>,
    cheats: Option<Res<crate::cheats::Cheats>>,
    mut restart: MessageReader<RestartGame>,
    #[cfg(feature = "dev-tools")] mut validator: Option<ResMut<crate::validation::Controller>>,
) {
    #[cfg(feature = "dev-tools")]
    if let Some(ref mut validator) = validator
        && validator.owns_input()
    {
        if keys.just_pressed(KeyCode::Escape) {
            validator.request_pause_toggle();
        }
        if keys.just_pressed(KeyCode::F12) {
            validator.request_abort();
        }
        // The controller owns pause transitions; physical gameplay input cannot resume a failed run.
        return;
    }
    let requested = restart.read().count() > 0;
    if !cheats.is_some_and(|c| c.menu_open)
        && menu.capturing.is_none()
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
    if **state == GameState::Playing && cursors.iter().any(|(w, _)| !w.focused) {
        next.set(GameState::Paused);
    }
    if **state == GameState::GameOver
        && (settings.just_pressed(Action::Restart, &keys, &mouse) || requested)
    {
        next.set(GameState::StartingSession);
    }
    for (window, mut cursor) in &mut cursors {
        let capture = window.focused && **state == GameState::Playing;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_sprint_press_latches_until_movement_cancels_it() {
        let mut actions = Actions::default();
        for held in [true, false, true, false] {
            actions.transition(Action::Sprint, held);
            assert!(actions.active(Action::Sprint));
        }
        actions.clear_toggle(Action::Sprint);
        assert!(!actions.active(Action::Sprint));
        actions.transition(Action::Sprint, true);
        assert!(actions.active(Action::Sprint));
        let mut input = GameInput::default();
        actions.event(Binding::Key(KeyCode::ControlLeft), true);
        write_actions(&mut actions, &mut input, false);
        assert!(input.prone && !input.crouch);
    }

    #[test]
    fn focused_gameplay_accepts_movement_and_look_before_cursor_capture() {
        let mut app = App::new();
        app.add_plugins(bevy::input::InputPlugin)
            .init_resource::<Actions>()
            .init_resource::<GameInput>()
            .init_resource::<ControlSettings>()
            .insert_resource(State::new(GameState::Playing))
            .add_systems(PreUpdate, collect.after(InputSystems));
        let window = app
            .world_mut()
            .spawn((Window::default(), PrimaryWindow))
            .id();
        app.world_mut().write_message(KeyboardInput {
            key_code: KeyCode::KeyW,
            logical_key: bevy::input::keyboard::Key::Character("w".into()),
            state: bevy::input::ButtonState::Pressed,
            text: None,
            repeat: false,
            window,
        });
        app.world_mut()
            .write_message(MouseMotion { delta: Vec2::ONE });
        app.update();
        let input = app.world().resource::<GameInput>();
        assert_eq!(input.movement, Vec2::Y);
        assert_eq!(input.look, Vec2::ONE);

        app.world_mut().get_mut::<Window>(window).unwrap().focused = false;
        app.world_mut()
            .write_message(MouseMotion { delta: Vec2::ONE });
        app.update();
        let input = app.world().resource::<GameInput>();
        assert_eq!(input.movement, Vec2::ZERO);
        assert_eq!(input.look, Vec2::ZERO);

        app.world_mut().get_mut::<Window>(window).unwrap().focused = true;
        app.update();
        assert_eq!(app.world().resource::<GameInput>().movement, Vec2::ZERO);
        for state in [
            bevy::input::ButtonState::Released,
            bevy::input::ButtonState::Pressed,
        ] {
            app.world_mut().write_message(KeyboardInput {
                key_code: KeyCode::KeyW,
                logical_key: bevy::input::keyboard::Key::Character("w".into()),
                state,
                text: None,
                repeat: false,
                window,
            });
            app.update();
        }
        assert_eq!(app.world().resource::<GameInput>().movement, Vec2::Y);

        app.insert_resource(State::new(GameState::Paused));
        app.world_mut()
            .write_message(MouseMotion { delta: Vec2::ONE });
        app.update();
        let input = app.world().resource::<GameInput>();
        assert_eq!(input.movement, Vec2::ZERO);
        assert_eq!(input.look, Vec2::ZERO);
    }

    #[test]
    fn akimbo_trigger_does_not_latch_toggle_aim_for_the_next_weapon() {
        use crate::{origins::content::WeaponDefinition, weapons::WeaponInventory};
        let mut weapon: WeaponDefinition = serde_json::from_value(serde_json::json!({
            "id":"weapon.dual", "name":"dual", "damage":20, "category":"bullet",
            "magazine":8, "reserve":0, "fire_interval":0.1, "reload_seconds":1.,
            "dual_weapon":"weapon.left"
        }))
        .unwrap();
        let mut inventory = WeaponInventory::default();
        inventory.grant(&weapon).unwrap();
        let mut app = App::new();
        app.init_resource::<Actions>()
            .init_resource::<GameInput>()
            .init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<ButtonInput<MouseButton>>()
            .add_message::<KeyboardInput>()
            .add_message::<MouseButtonInput>()
            .add_message::<MouseMotion>()
            .add_message::<MouseWheel>()
            .insert_resource(State::new(GameState::Playing))
            .add_systems(Update, collect);
        let settings = ControlSettings {
            aim_mode: PressMode::Toggle,
            ..default()
        };
        app.world_mut().resource_mut::<Actions>().settings = settings.clone();
        app.insert_resource(settings);
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
                PrimaryWindow,
            ))
            .id();
        let player = app
            .world_mut()
            .spawn((
                crate::player::Player {
                    yaw: 0.,
                    pitch: 0.,
                    velocity: Vec3::ZERO,
                    height: 1.8,
                    grounded: true,
                    crouched: false,
                    prone: false,
                    eye_height: 1.6,
                },
                inventory,
            ))
            .id();
        for pressed in [true, false] {
            if pressed {
                app.world_mut()
                    .resource_mut::<ButtonInput<MouseButton>>()
                    .press(MouseButton::Right);
            } else {
                app.world_mut()
                    .resource_mut::<ButtonInput<MouseButton>>()
                    .release(MouseButton::Right);
            }
            app.world_mut().write_message(MouseButtonInput {
                button: MouseButton::Right,
                state: if pressed {
                    bevy::input::ButtonState::Pressed
                } else {
                    bevy::input::ButtonState::Released
                },
                window,
            });
            app.update();
            assert_eq!(
                app.world().resource::<Actions>().state(Action::Aim).held,
                pressed
            );
            assert!(!app.world().resource::<GameInput>().ads);
        }
        weapon.dual_weapon = None;
        let mut inventory = WeaponInventory::default();
        inventory.grant(&weapon).unwrap();
        app.world_mut().entity_mut(player).insert(inventory);
        app.update();
        assert!(!app.world().resource::<GameInput>().ads);
        app.world_mut()
            .resource_mut::<ButtonInput<MouseButton>>()
            .press(MouseButton::Right);
        app.world_mut().write_message(MouseButtonInput {
            button: MouseButton::Right,
            state: bevy::input::ButtonState::Pressed,
            window,
        });
        app.update();
        assert!(app.world().resource::<GameInput>().ads);
    }

    #[test]
    fn session_reset_discards_actions_and_buffers_until_held_buttons_are_released() {
        let mut world = World::new();
        world.init_resource::<Actions>();
        world.init_resource::<GameInput>();
        world.init_resource::<ControlSettings>();
        world.init_resource::<menu::ControlMenu>();
        world.init_resource::<ButtonInput<KeyCode>>();
        world.init_resource::<ButtonInput<MouseButton>>();
        world.init_resource::<Messages<KeyboardInput>>();
        world.init_resource::<Messages<MouseButtonInput>>();
        world.init_resource::<Messages<MouseMotion>>();
        world.init_resource::<Messages<MouseWheel>>();
        world.resource_mut::<ControlSettings>().crouch_mode = PressMode::Toggle;
        world
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::KeyW);
        world
            .resource_mut::<ButtonInput<MouseButton>>()
            .press(MouseButton::Left);
        {
            let mut actions = world.resource_mut::<Actions>();
            actions.event(Binding::Key(KeyCode::KeyW), true);
            actions.event(Binding::Key(KeyCode::KeyC), true);
            actions.event(Binding::Mouse(MouseButton::Left), true);
        }
        world.resource_mut::<GameInput>().fire = true;
        world.resource_mut::<GameInput>().movement = Vec2::Y;
        world.write_message(MouseMotion { delta: Vec2::ONE });

        reset(&mut world);

        assert_eq!(world.resource::<GameInput>().movement, Vec2::ZERO);
        assert!(!world.resource::<GameInput>().fire);
        assert!(world.resource::<Messages<MouseMotion>>().is_empty());
        assert!(
            !world
                .resource::<ButtonInput<KeyCode>>()
                .just_pressed(KeyCode::KeyW)
        );
        assert!(
            world
                .resource::<ButtonInput<KeyCode>>()
                .pressed(KeyCode::KeyW)
        );
        let mut actions = world.resource_mut::<Actions>();
        assert!(!actions.active(Action::Crouch));
        for (binding, action) in [
            (Binding::Key(KeyCode::KeyW), Action::Forward),
            (Binding::Mouse(MouseButton::Left), Action::Fire),
        ] {
            actions.event(binding, true);
            assert!(!actions.active(action));
            actions.event(binding, false);
            actions.event(binding, true);
            assert!(actions.active(action));
        }
    }
}

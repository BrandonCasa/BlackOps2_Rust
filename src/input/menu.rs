use super::{Action, Binding, ControlSettings, PressMode, settings::SettingsStore};
use crate::GameState;
use bevy::{
    input::{
        InputSystems,
        keyboard::KeyboardInput,
        mouse::{MouseButtonInput, MouseWheel},
    },
    picking::{events::PointerPress, hover::Hovered, pointer::PointerButton},
    prelude::*,
    ui_widgets::Button as UiButton,
};

const PAGE_SIZE: usize = 8;
#[derive(Resource, Default)]
pub(crate) struct ControlMenu {
    pub capturing: Option<Action>,
    pub page: Option<usize>,
    pub selected: usize,
    pub notice: String,
    pub revision: u64,
    pub handled_input: bool,
    pointer_choice: Option<Choice>,
    pending_action: Option<crate::frontend::FrontendAction>,
}
#[derive(Component)]
struct MenuRoot;
#[derive(Component)]
struct MenuNotice;
#[derive(Component)]
pub(crate) struct MenuRow {
    pub index: usize,
    choice: Choice,
}
#[derive(Component)]
struct RowText(Choice);
#[derive(Clone, Copy)]
enum Choice {
    Resume,
    Controls,
    Restart,
    MainMenu,
    Exit,
    Sensitivity,
    AdsSensitivity,
    Invert,
    AimMode,
    CrouchMode,
    SprintMode,
    Bind(Action),
    Previous,
    Next,
    Defaults,
    Back,
}

fn choices(menu: &ControlMenu) -> Vec<Choice> {
    let Some(page) = menu.page else {
        return vec![Choice::Resume, Choice::Controls, Choice::Restart, Choice::MainMenu, Choice::Exit];
    };
    let mut rows = vec![
        Choice::Sensitivity,
        Choice::AdsSensitivity,
        Choice::Invert,
        Choice::AimMode,
        Choice::CrouchMode,
        Choice::SprintMode,
    ];
    rows.extend(
        Action::ALL
            .into_iter()
            .skip(page * PAGE_SIZE)
            .take(PAGE_SIZE)
            .map(Choice::Bind),
    );
    if page > 0 {
        rows.push(Choice::Previous);
    }
    if (page + 1) * PAGE_SIZE < Action::ALL.len() {
        rows.push(Choice::Next);
    }
    rows.extend([Choice::Defaults, Choice::Back]);
    rows
}
fn mode(mode: PressMode) -> &'static str {
    if mode == PressMode::Hold {
        "Hold"
    } else {
        "Toggle"
    }
}
fn label(choice: Choice, settings: &ControlSettings) -> String {
    match choice {
        Choice::Resume => "Resume game".into(),
        Choice::Controls => "Controls".into(),
        Choice::Restart => "Restart game".into(),
        Choice::MainMenu => "Quit to menu".into(),
        Choice::Exit => "Exit application".into(),
        Choice::Sensitivity => format!(
            "Mouse sensitivity                 {:.1}x",
            settings.sensitivity
        ),
        Choice::AdsSensitivity => format!(
            "ADS mouse sensitivity         {:.1}x",
            settings.ads_sensitivity
        ),
        Choice::Invert => format!(
            "Invert vertical mouse           {}",
            if settings.invert_mouse_y { "On" } else { "Off" }
        ),
        Choice::AimMode => format!(
            "Aim down sights                  {}",
            mode(settings.aim_mode)
        ),
        Choice::CrouchMode => format!(
            "Crouch                                 {}",
            mode(settings.crouch_mode)
        ),
        Choice::SprintMode => format!(
            "Sprint                                    {}",
            mode(settings.sprint_mode)
        ),
        Choice::Bind(action) => format!("{}: {}", action.label(), settings.label(action)),
        Choice::Previous => "Previous bindings".into(),
        Choice::Next => "Next bindings".into(),
        Choice::Defaults => "Restore default controls".into(),
        Choice::Back => "Back".into(),
    }
}
pub(super) fn install(app: &mut App) {
    app.init_resource::<ControlMenu>();
    let options = app.world().resource::<crate::assets::LaunchOptions>();
    if options.verify.is_some() && !options.verify_controls {
        return;
    }
    app.add_observer(pointer_activate)
        .add_systems(OnEnter(GameState::Paused), open)
        .add_systems(OnExit(GameState::Paused), close)
        .add_systems(
            PreUpdate,
            capture.after(InputSystems).before(super::collect),
        )
        .add_systems(
            Update,
            (navigate, rebuild, labels)
                .chain()
                .before(super::pause_and_restart)
                .run_if(in_state(GameState::Paused)),
        );
}
fn open(mut menu: ResMut<ControlMenu>) {
    *menu = ControlMenu {
        revision: menu.revision.wrapping_add(1),
        ..default()
    };
}
fn close(
    mut commands: Commands,
    roots: Query<Entity, With<MenuRoot>>,
    mut menu: ResMut<ControlMenu>,
) {
    for root in &roots {
        commands.entity(root).despawn();
    }
    menu.capturing = None;
}
fn rebuild(
    mut commands: Commands,
    menu: Res<ControlMenu>,
    settings: Res<ControlSettings>,
    roots: Query<Entity, With<MenuRoot>>,
    cameras: Query<Entity, With<crate::presentation::ViewModelCamera>>,
    mut built: Local<Option<u64>>,
) {
    if *built == Some(menu.revision) && !roots.is_empty() {
        return;
    }
    let Ok(camera) = cameras.single() else {
        return;
    };
    for root in &roots {
        commands.entity(root).despawn();
    }
    *built = Some(menu.revision);
    commands
        .spawn((
            MenuRoot,
            UiTargetCamera(camera),
            GlobalZIndex(100),
            Node {
                position_type: PositionType::Absolute,
                top: px(16.),
                left: px(16.),
                width: px(540.),
                padding: UiRect::all(px(12.)),
                flex_direction: FlexDirection::Column,
                row_gap: px(3.),
                ..default()
            },
            BackgroundColor(Color::srgba(0.025, 0.025, 0.025, 0.96)),
        ))
        .with_children(|parent| {
            let title = menu
                .page
                .map_or("PAUSED".into(), |p| format!("CONTROLS   {} / {}", p + 1, Action::ALL.len().div_ceil(PAGE_SIZE)));
            parent.spawn((
                Text::new(title),
                TextFont {
                    font_size: FontSize::Px(22.),
                    ..default()
                },
                TextColor(Color::srgb(0.95, 0.72, 0.3)),
            ));
            parent.spawn((
                Text::new("Up/Down select   Left/Right adjust   Enter choose\nEsc resume   Left/right click changes values"),
                TextFont {
                    font_size: FontSize::Px(13.),
                    ..default()
                },
            ));
            for (index, choice) in choices(&menu).into_iter().enumerate() {
                parent
                    .spawn((
                        UiButton,
                        Hovered::default(),
                        MenuRow { index, choice },
                        Node {
                            min_height: px(27.),
                            padding: UiRect::axes(px(8.), px(4.)),
                            ..default()
                        },
                        BackgroundColor(Color::NONE),
                    ))
                    .with_children(|row| {
                        row.spawn((
                            Text::new(label(choice, &settings)),
                            RowText(choice),
                            TextFont {
                                font_size: FontSize::Px(15.),
                                ..default()
                            },
                        ));
                    });
            }
            parent.spawn((
                Text::new(""),
                MenuNotice,
                TextFont {
                    font_size: FontSize::Px(13.),
                    ..default()
                },
                TextColor(Color::srgb(0.9, 0.75, 0.5)),
            ));
        });
}
fn activate(
    choice: Choice,
    direction: f32,
    menu: &mut ControlMenu,
    settings: &mut ControlSettings,
    next: &mut NextState<GameState>,
    store: &mut SettingsStore,
) {
    match choice {
        Choice::Resume => next.set(GameState::Playing),
        Choice::Restart => menu.pending_action = Some(crate::frontend::FrontendAction::Restart),
        Choice::MainMenu => menu.pending_action = Some(crate::frontend::FrontendAction::MainMenu),
        Choice::Exit => menu.pending_action = Some(crate::frontend::FrontendAction::Exit),
        Choice::Controls => {
            menu.page = Some(0);
            menu.selected = 0;
            menu.revision += 1;
        }
        Choice::Sensitivity => {
            settings.sensitivity =
                ((settings.sensitivity * 10.).round() + direction).clamp(1., 100.) / 10.
        }
        Choice::AdsSensitivity => {
            settings.ads_sensitivity =
                ((settings.ads_sensitivity * 10.).round() + direction).clamp(1., 100.) / 10.
        }
        Choice::Invert => settings.invert_mouse_y = !settings.invert_mouse_y,
        Choice::AimMode | Choice::CrouchMode | Choice::SprintMode => {
            let mode = match choice {
                Choice::AimMode => &mut settings.aim_mode,
                Choice::CrouchMode => &mut settings.crouch_mode,
                _ => &mut settings.sprint_mode,
            };
            *mode = if *mode == PressMode::Hold {
                PressMode::Toggle
            } else {
                PressMode::Hold
            };
        }
        Choice::Bind(action) => {
            menu.capturing = Some(action);
            menu.notice.clear();
        }
        Choice::Previous => {
            menu.page = menu.page.map(|p| p.saturating_sub(1));
            menu.selected = 0;
            menu.revision += 1;
        }
        Choice::Next => {
            menu.page = menu.page.map(|p| p + 1);
            menu.selected = 0;
            menu.revision += 1;
        }
        Choice::Defaults => {
            let audio = settings.audio.clone();
            *settings = ControlSettings::default();
            settings.audio = audio;
            store.force_save = true;
            menu.notice = "Default controls restored.".into();
        }
        Choice::Back => {
            menu.page = None;
            menu.selected = 0;
            menu.revision += 1;
        }
    }
}
fn navigate(
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut menu: ResMut<ControlMenu>,
    mut settings: ResMut<ControlSettings>,
    mut next: ResMut<NextState<GameState>>,
    mut store: ResMut<SettingsStore>,
    rows: Query<(&MenuRow, Ref<Hovered>)>,
    mut frontend: MessageWriter<crate::frontend::FrontendAction>,
) {
    let pointer_choice = menu.pointer_choice.take();
    if menu.capturing.is_some() || menu.handled_input {
        return;
    }
    let choices = choices(&menu);
    if keys.just_pressed(KeyCode::ArrowUp) {
        menu.selected = (menu.selected + choices.len() - 1) % choices.len();
    }
    if keys.just_pressed(KeyCode::ArrowDown) {
        menu.selected = (menu.selected + 1) % choices.len();
    }
    let selected = menu.selected.min(choices.len() - 1);
    if keys.just_pressed(KeyCode::Enter)
        || keys.just_pressed(KeyCode::ArrowLeft)
        || keys.just_pressed(KeyCode::ArrowRight)
    {
        let choice = choices[selected];
        let adjust = matches!(
            choice,
            Choice::Sensitivity
                | Choice::AdsSensitivity
                | Choice::Invert
                | Choice::AimMode
                | Choice::CrouchMode
                | Choice::SprintMode
        );
        if keys.just_pressed(KeyCode::Enter) || adjust {
            let direction = if keys.just_pressed(KeyCode::ArrowLeft) {
                -1.
            } else {
                1.
            };
            activate(
                choice,
                direction,
                &mut menu,
                &mut settings,
                &mut next,
                &mut store,
            );
        }
    }
    if mouse.just_pressed(MouseButton::Right) {
        let choice = choices[selected];
        if matches!(choice, Choice::Sensitivity | Choice::AdsSensitivity) {
            activate(choice, -1., &mut menu, &mut settings, &mut next, &mut store);
        }
    }
    for (row, hovered) in &rows {
        if hovered.is_changed() && hovered.0 {
            menu.selected = row.index;
        }
    }
    if let Some(choice) = pointer_choice {
        activate(choice, 1., &mut menu, &mut settings, &mut next, &mut store);
    }
    if let Some(action) = menu.pending_action.take() { frontend.write(action); }
}
fn pointer_activate(event: On<PointerPress>, rows: Query<&MenuRow>, mut menu: ResMut<ControlMenu>) {
    if event.button == PointerButton::Primary
        && menu.capturing.is_none()
        && let Ok(row) = rows.get(event.entity)
    {
        menu.selected = row.index;
        menu.pointer_choice = Some(row.choice);
    }
}
fn capture(
    state: Res<State<GameState>>,
    held: Res<ButtonInput<KeyCode>>,
    mut menu: ResMut<ControlMenu>,
    mut settings: ResMut<ControlSettings>,
    mut keys: MessageReader<KeyboardInput>,
    mut mouse: MessageReader<MouseButtonInput>,
    mut wheel: MessageReader<MouseWheel>,
) {
    menu.handled_input = false;
    let key = keys
        .read()
        .find(|e| {
            e.state.is_pressed()
                && !e.repeat
                && !(e.key_code == KeyCode::Enter
                    && (held.pressed(KeyCode::AltLeft) || held.pressed(KeyCode::AltRight)))
        })
        .map(|e| e.key_code);
    let button = mouse
        .read()
        .find(|e| e.state.is_pressed())
        .map(|e| e.button);
    let scroll = wheel.read().find(|e| e.y != 0.).map(|e| {
        if e.y > 0. {
            Binding::WheelUp
        } else {
            Binding::WheelDown
        }
    });
    keys.clear();
    mouse.clear();
    wheel.clear();
    if **state != GameState::Paused {
        return;
    }
    let Some(action) = menu.capturing else {
        return;
    };
    menu.handled_input = key.is_some() || button.is_some() || scroll.is_some();
    if key == Some(KeyCode::Escape) {
        menu.capturing = None;
        menu.notice = "Binding unchanged.".into();
        return;
    }
    let unbind = key == Some(KeyCode::Backspace);
    let binding = key
        .map(Binding::Key)
        .or(button.map(Binding::Mouse))
        .or(scroll);
    if binding.is_none() {
        return;
    }
    match settings.rebind(action, if unbind { None } else { binding }) {
        Ok(()) => {
            menu.capturing = None;
            menu.notice = format!("{}: {}", action.label(), settings.label(action));
        }
        Err(error) => menu.notice = error,
    }
}
fn labels(
    menu: Res<ControlMenu>,
    settings: Res<ControlSettings>,
    store: Res<SettingsStore>,
    mut rows: Query<(&MenuRow, &mut BackgroundColor)>,
    mut texts: Query<(&RowText, &mut Text), Without<MenuNotice>>,
    mut notices: Query<&mut Text, With<MenuNotice>>,
) {
    for (row, mut background) in &mut rows {
        let color = if row.index == menu.selected {
            Color::srgba(0.35, 0.25, 0.08, 0.9)
        } else {
            Color::NONE
        };
        if background.0 != color {
            background.0 = color;
        }
    }
    if settings.is_changed() {
        for (row, mut text) in &mut texts {
            text.0 = label(row.0, &settings);
        }
    }
    for mut text in &mut notices {
        let value = if let Some(action) = menu.capturing {
            format!(
                "{}: press a key, mouse button or wheel.\nBackspace clears; Esc cancels.\n{}",
                action.label(),
                menu.notice
            )
        } else {
            store.error.clone().unwrap_or_else(|| menu.notice.clone())
        };
        if text.0 != value {
            text.0 = value;
        }
    }
}

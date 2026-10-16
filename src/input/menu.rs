use super::{Action, Binding, ControlSettings, PressMode, settings::SettingsStore};
use crate::GameState;
use crate::hud::native::{Canvas, NativeText, rect};
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
    audio: bool,
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
    Audio,
    Master,
    Music,
    Effects,
    Dialogue,
    Subtitles,
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
    if menu.audio {
        return vec![
            Choice::Master,
            Choice::Music,
            Choice::Effects,
            Choice::Dialogue,
            Choice::Subtitles,
            Choice::Back,
        ];
    }
    let Some(page) = menu.page else {
        return vec![
            Choice::Resume,
            Choice::Controls,
            Choice::Audio,
            Choice::Restart,
            Choice::MainMenu,
            Choice::Exit,
        ];
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
        Choice::Audio => "Audio".into(),
        Choice::Master => format!("Master Volume     {:.0}%", settings.audio.master * 100.),
        Choice::Music => format!("Music Volume     {:.0}%", settings.audio.music * 100.),
        Choice::Effects => format!("SFX Volume     {:.0}%", settings.audio.effects * 100.),
        Choice::Dialogue => format!("Voice Volume     {:.0}%", settings.audio.dialogue * 100.),
        Choice::Subtitles => format!(
            "Subtitles     {}",
            if settings.audio.subtitles {
                "Enabled"
            } else {
                "Disabled"
            }
        ),
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
    app.add_observer(pointer_activate)
        .add_systems(OnEnter(GameState::Paused), open)
        .add_systems(OnExit(GameState::Paused), close)
        .add_systems(
            PreUpdate,
            capture.after(InputSystems).before(super::collect),
        )
        .add_systems(
            Update,
            (rebuild, labels)
                .chain()
                .before(super::pause_and_restart)
                .run_if(in_state(GameState::Paused)),
        );

    app.add_systems(
        Update,
        navigate
            .before(rebuild)
            .before(crate::frontend::ApplyFrontendActions)
            .run_if(in_state(GameState::Paused)),
    );
}
fn open(mut menu: ResMut<ControlMenu>) {
    *menu = ControlMenu {
        revision: menu.revision.wrapping_add(1),
        ..default()
    };
}
pub(super) fn reset(world: &mut World) {
    let mut menu = world.resource_mut::<ControlMenu>();
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
    cameras: Query<Entity, With<crate::rendering::display::GameUiCamera>>,
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
            Canvas,
            UiTransform::default(),
            UiTargetCamera(camera),
            GlobalZIndex(100),
            rect(0., 0., 1280., 720.),
            BackgroundColor(Color::linear_rgba(0., 0., 0., 0.62)),
        ))
        .with_children(|parent| {
            parent
                .spawn(Node {
                    flex_direction: FlexDirection::Column,
                    row_gap: px(2.),
                    ..rect(110., 100., 550., 580.)
                })
                .with_children(|parent| {
                    let title =
                        menu.page
                            .map_or(if menu.audio { "AUDIO" } else { "PAUSED" }.into(), |p| {
                                format!(
                                    "CONTROLS   {} / {}",
                                    p + 1,
                                    Action::ALL.len().div_ceil(PAGE_SIZE)
                                )
                            });
                    parent.spawn((
                        NativeText::new(title, 42.),
                        Node {
                            width: px(550.),
                            height: px(58.),
                            ..default()
                        },
                    ));
                    parent.spawn((
                        NativeText::new("Origins - Excavation Site 64", 24.)
                            .color(Color::linear_rgb(0.7, 0.7, 0.7)),
                        Node {
                            width: px(550.),
                            height: px(38.),
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
                                    min_height: px(if menu.page.is_some() { 23. } else { 32. }),
                                    height: px(if menu.page.is_some() { 23. } else { 32. }),
                                    padding: UiRect::axes(px(8.), px(0.)),
                                    ..default()
                                },
                                BackgroundColor(Color::NONE),
                            ))
                            .with_children(|row| {
                                row.spawn((
                                    NativeText::new(
                                        label(choice, &settings),
                                        if menu.page.is_some() { 20. } else { 25. },
                                    ),
                                    RowText(choice),
                                    Node {
                                        width: px(525.),
                                        height: px(27.),
                                        ..default()
                                    },
                                ));
                            });
                    }
                    parent.spawn((
                        NativeText::new("", 18.).color(Color::linear_rgb(1., 0.65, 0.25)),
                        MenuNotice,
                        Node {
                            width: px(550.),
                            height: px(60.),
                            ..default()
                        },
                    ));
                });
            parent.spawn((
                NativeText::new(
                    "ENTER  Select       ESC  Back       Arrow Keys  Navigate",
                    18.,
                ),
                rect(110., 680., 900., 30.),
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
    audio: &mut crate::audio::AudioSettings,
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
        Choice::Audio => {
            menu.audio = true;
            menu.selected = 0;
            menu.revision += 1;
        }
        Choice::Master | Choice::Music | Choice::Effects | Choice::Dialogue => {
            let value = match choice {
                Choice::Master => &mut audio.master,
                Choice::Music => &mut audio.music,
                Choice::Effects => &mut audio.effects,
                _ => &mut audio.dialogue,
            };
            *value = (*value + direction * 0.05).clamp(0., 1.);
        }
        Choice::Subtitles => audio.subtitles = !audio.subtitles,
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
            menu.audio = false;
            menu.selected = 0;
            menu.revision += 1;
        }
    }
}
pub(crate) fn navigate(
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut menu: ResMut<ControlMenu>,
    mut settings: ResMut<ControlSettings>,
    mut next: ResMut<NextState<GameState>>,
    mut store: ResMut<SettingsStore>,
    rows: Query<(&MenuRow, Ref<Hovered>)>,
    mut frontend: MessageWriter<crate::frontend::FrontendAction>,
    mut audio: ResMut<crate::audio::AudioSettings>,
) {
    let pointer_choice = menu.pointer_choice.take();
    if menu.capturing.is_some() || menu.handled_input {
        return;
    }
    if keys.just_pressed(KeyCode::Escape) && (menu.page.is_some() || menu.audio) {
        menu.page = None;
        menu.audio = false;
        menu.selected = 0;
        menu.revision += 1;
        menu.handled_input = true;
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
                | Choice::Master
                | Choice::Music
                | Choice::Effects
                | Choice::Dialogue
                | Choice::Subtitles
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
                &mut audio,
            );
        }
    }
    if mouse.just_pressed(MouseButton::Right) {
        let choice = choices[selected];
        if matches!(choice, Choice::Sensitivity | Choice::AdsSensitivity) {
            activate(
                choice,
                -1.,
                &mut menu,
                &mut settings,
                &mut next,
                &mut store,
                &mut audio,
            );
        }
    }
    for (row, hovered) in &rows {
        if hovered.is_changed() && hovered.0 {
            menu.selected = row.index;
        }
    }
    if let Some(choice) = pointer_choice {
        activate(
            choice,
            1.,
            &mut menu,
            &mut settings,
            &mut next,
            &mut store,
            &mut audio,
        );
    }
    if let Some(action) = menu.pending_action.take() {
        frontend.write(action);
    }
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
    mut texts: Query<(&RowText, &mut NativeText), Without<MenuNotice>>,
    mut notices: Query<&mut NativeText, With<MenuNotice>>,
) {
    for (row, mut background) in &mut rows {
        let color = if row.index == menu.selected {
            Color::linear_rgba(0.85, 0.3, 0.02, 0.8)
        } else {
            Color::NONE
        };
        if background.0 != color {
            background.0 = color;
        }
    }
    if settings.is_changed() {
        for (row, mut text) in &mut texts {
            text.value = label(row.0, &settings);
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
        if text.value != value {
            text.value = value;
        }
    }
}

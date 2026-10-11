use crate::{GameState, assets::LaunchOptions};
use bevy::prelude::*;
use bevy::picking::{events::PointerPress, hover::Hovered, pointer::PointerButton};
use bevy::ui_widgets::Button as UiButton;

#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Difficulty {
    #[default]
    Original,
    Easy,
}

impl Difficulty {
    pub fn label(self) -> &'static str {
        match self {
            Self::Original => "Original",
            Self::Easy => "Easy",
        }
    }

    pub fn quests_enabled(self) -> bool {
        self == Self::Original
    }

    pub fn gait(self, profile: &crate::rounds::RoundProfile, round: u32, random: u32) -> crate::rounds::Gait {
        if self == Self::Original {
            return profile.gait(round, random);
        }
        let base = if round <= 1 { 1 } else { round.clamp(1, 255) * 2 };
        let roll = base.saturating_add(random % profile.speed_random_band);
        if roll <= profile.gait_thresholds[0] {
            crate::rounds::Gait::Walk
        } else if roll <= profile.gait_thresholds[1] {
            crate::rounds::Gait::Run
        } else {
            crate::rounds::Gait::Sprint
        }
    }
}

#[derive(Message, Clone, Copy)]
pub enum FrontendAction {
    StartSolo,
    SelectDifficulty(Difficulty),
    Restart,
    MainMenu,
    RetryLoad,
    Exit,
}

#[derive(Component)]
struct FrontendRoot;

#[derive(Component)]
struct FrontendButton(FrontendAction);

pub struct FrontendPlugin;

impl Plugin for FrontendPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Difficulty>()
            .add_message::<FrontendAction>()
            .add_observer(buttons)
            .add_systems(Update, (shortcuts, apply_actions).chain())
            .add_systems(OnEnter(GameState::MainMenu), show)
            .add_systems(OnEnter(GameState::LoadError), show)
            .add_systems(OnEnter(GameState::GameOver), show)
            .add_systems(OnExit(GameState::MainMenu), hide)
            .add_systems(OnExit(GameState::LoadError), hide)
            .add_systems(OnExit(GameState::GameOver), hide);
    }
}

fn show(
    mut commands: Commands,
    state: Res<State<GameState>>,
    difficulty: Res<Difficulty>,
    options: Res<LaunchOptions>,
    cameras: Query<Entity, With<crate::presentation::ViewModelCamera>>,
) {
    if options.verify.is_some() || options.validation_remote.is_some() || options.flythrough {
        return;
    }
    let Ok(camera) = cameras.single() else { return };
    let rows = match **state {
        GameState::MainMenu => vec![
            ("Start solo", FrontendAction::StartSolo),
            ("Original difficulty", FrontendAction::SelectDifficulty(Difficulty::Original)),
            ("Easy difficulty", FrontendAction::SelectDifficulty(Difficulty::Easy)),
            ("Exit application", FrontendAction::Exit),
        ],
        GameState::LoadError => vec![
            ("Retry loading", FrontendAction::RetryLoad),
            ("Exit application", FrontendAction::Exit),
        ],
        GameState::GameOver => vec![
            ("Restart", FrontendAction::Restart),
            ("Quit to menu", FrontendAction::MainMenu),
            ("Exit application", FrontendAction::Exit),
        ],
        _ => return,
    };
    commands.spawn((
        FrontendRoot,
        UiTargetCamera(camera),
        GlobalZIndex(110),
        Node {
            position_type: PositionType::Absolute,
            top: percent(15.),
            left: percent(30.),
            width: percent(40.),
            padding: UiRect::all(px(24.)),
            flex_direction: FlexDirection::Column,
            row_gap: px(12.),
            ..default()
        },
        BackgroundColor(Color::srgba(0.02, 0.02, 0.02, 0.95)),
    )).with_children(|parent| {
        parent.spawn((
            Text::new(format!("ORIGINS\nSolo — {}", difficulty.label())),
            FrontendRootTitle,
            TextFont { font_size: FontSize::Px(28.), ..default() },
            TextColor(Color::srgb(0.95, 0.72, 0.3)),
        ));
        for (label, action) in rows {
            parent.spawn((
                UiButton,
                Hovered::default(),
                FrontendButton(action),
                Node { padding: UiRect::all(px(12.)), ..default() },
                BackgroundColor(Color::srgb(0.12, 0.12, 0.12)),
            )).with_children(|button| {
                button.spawn((Text::new(label), TextFont { font_size: FontSize::Px(20.), ..default() }));
            });
        }
        parent.spawn((Text::new("Enter: start / retry   1: Original   2: Easy   Esc: menu / exit"),
            TextFont { font_size: FontSize::Px(14.), ..default() }));
    });
}

fn hide(mut commands: Commands, roots: Query<Entity, With<FrontendRoot>>) {
    for root in &roots { commands.entity(root).despawn(); }
}

fn buttons(
    event: On<PointerPress>,
    buttons: Query<&FrontendButton>,
    mut actions: MessageWriter<FrontendAction>,
) {
    if event.button == PointerButton::Primary && let Ok(button) = buttons.get(event.entity) {
        actions.write(button.0);
    }
}

fn shortcuts(
    keys: Res<ButtonInput<KeyCode>>,
    state: Res<State<GameState>>,
    mut actions: MessageWriter<FrontendAction>,
) {
    match **state {
        GameState::MainMenu => {
            if keys.just_pressed(KeyCode::Enter) { actions.write(FrontendAction::StartSolo); }
            if keys.just_pressed(KeyCode::Digit1) { actions.write(FrontendAction::SelectDifficulty(Difficulty::Original)); }
            if keys.just_pressed(KeyCode::Digit2) { actions.write(FrontendAction::SelectDifficulty(Difficulty::Easy)); }
            if keys.just_pressed(KeyCode::Escape) { actions.write(FrontendAction::Exit); }
        }
        GameState::LoadError => {
            if keys.just_pressed(KeyCode::Enter) { actions.write(FrontendAction::RetryLoad); }
            if keys.just_pressed(KeyCode::Escape) { actions.write(FrontendAction::Exit); }
        }
        GameState::GameOver => {
            if keys.just_pressed(KeyCode::Escape) { actions.write(FrontendAction::MainMenu); }
        }
        _ => {}
    }
}

fn apply_actions(
    mut commands: Commands,
    mut actions: MessageReader<FrontendAction>,
    mut difficulty: ResMut<Difficulty>,
    state: Res<State<GameState>>,
    mut next: ResMut<NextState<GameState>>,
    mut exit: MessageWriter<AppExit>,
    mut title: Query<&mut Text, With<FrontendRootTitle>>,
) {
    for action in actions.read() {
        match *action {
            FrontendAction::SelectDifficulty(value) if **state == GameState::MainMenu => {
                *difficulty = value;
                for mut text in &mut title { text.0 = format!("ORIGINS\nSolo — {}", value.label()); }
            }
            FrontendAction::StartSolo if **state == GameState::MainMenu => {
                commands.queue(crate::session::teardown);
                next.set(GameState::PreparingShaders);
            }
            FrontendAction::Restart if matches!(**state, GameState::Paused | GameState::GameOver) => {
                commands.queue(crate::session::teardown);
                next.set(GameState::PreparingShaders);
            }
            FrontendAction::MainMenu if matches!(**state, GameState::Paused | GameState::GameOver) => {
                commands.queue(crate::session::teardown);
                next.set(GameState::MainMenu);
            }
            FrontendAction::RetryLoad if **state == GameState::LoadError => next.set(GameState::Loading),
            FrontendAction::Exit => { exit.write(AppExit::Success); }
            _ => {}
        }
    }
}

#[derive(Component)]
struct FrontendRootTitle;

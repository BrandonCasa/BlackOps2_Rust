use crate::hud::native::{self, Canvas, NativeText, rect};
use crate::GameState;
use bevy::picking::{events::PointerPress, hover::Hovered, pointer::PointerButton};
use bevy::prelude::*;
use bevy::ui_widgets::Button as UiButton;

#[derive(
    Resource, Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize,
)]
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

    pub fn gait(
        self,
        profile: &crate::rounds::RoundProfile,
        round: u32,
        random: u32,
    ) -> crate::rounds::Gait {
        if self == Self::Original {
            return profile.gait(round, random);
        }
        let base = if round <= 1 {
            1
        } else {
            round.saturating_sub(1).clamp(1, 255) * 2
        };
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
    Exit,
}

#[derive(Component)]
struct FrontendRoot;

#[derive(Component)]
struct FrontendButton(FrontendAction, usize);

#[derive(Resource, Default)]
struct FrontendSelection(usize);


#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct ApplyFrontendActions;

pub struct FrontendPlugin;

impl Plugin for FrontendPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Difficulty>()
            .init_resource::<FrontendSelection>()
            .add_message::<FrontendAction>()
            .add_observer(buttons)
            .add_systems(
                Update,
                (
                    shortcuts,
                    apply_actions.in_set(ApplyFrontendActions),
                    selection,
                )
                    .chain(),
            )
            .add_systems(OnEnter(GameState::MainMenu), show)
            .add_systems(OnEnter(GameState::GameOver), show)
            .add_systems(OnEnter(GameState::Ending), show)
            .add_systems(OnExit(GameState::MainMenu), hide)
            .add_systems(OnExit(GameState::GameOver), hide)
            .add_systems(OnExit(GameState::Ending), hide);
    }
}

fn show(
    mut commands: Commands,
    state: Res<State<GameState>>,
    difficulty: Res<Difficulty>,
    cameras: Query<Entity, With<crate::rendering::GameCamera>>,
    game: Option<Res<crate::assets::GameDefinition>>,
    server: Res<AssetServer>,
    mut selected: ResMut<FrontendSelection>,
) {
    let Ok(camera) = cameras.single() else { return };
    selected.0 = 0;
    let rows = match **state {
        GameState::MainMenu => vec![
            ("Start solo", FrontendAction::StartSolo),
            (
                "Original difficulty",
                FrontendAction::SelectDifficulty(Difficulty::Original),
            ),
            (
                "Easy difficulty",
                FrontendAction::SelectDifficulty(Difficulty::Easy),
            ),
            ("Exit application", FrontendAction::Exit),
        ],
        GameState::GameOver => vec![
            ("Restart", FrontendAction::Restart),
            ("Quit to menu", FrontendAction::MainMenu),
            ("Exit application", FrontendAction::Exit),
        ],
        _ => return,
    };
    commands
        .spawn((
            FrontendRoot,
            Canvas,
            UiTransform::default(),
            UiTargetCamera(camera),
            GlobalZIndex(110),
            rect(0., 0., 1280., 720.),
            BackgroundColor(if **state == GameState::GameOver {
                Color::NONE
            } else {
                Color::srgb(0.015, 0.015, 0.015)
            }),
        ))
        .with_children(|parent| {
            if !matches!(**state, GameState::GameOver) {
                if let Some(game) = &game
                    && game.content.ui_assets.iter().any(|asset| asset.id.0 == "ui.menu_zm_map_tomb")
                {
                    parent.spawn((
                        native::image(game, &server, "menu_zm_map_tomb"),
                        rect(665., 86., 430., 530.),
                    ));
                }
                parent.spawn((
                    NativeText::new(
                        format!("ZOMBIES\nORIGINS - {}", difficulty.label().to_uppercase()),
                        42.,
                    ),
                    FrontendRootTitle,
                    rect(110., 92., 560., 120.),
                ));
            }
            let y = if matches!(**state, GameState::GameOver) {
                558.
            } else {
                264.
            };
            for (index, (label, action)) in rows.into_iter().enumerate() {
                parent
                    .spawn((
                        UiButton,
                        Hovered::default(),
                        FrontendButton(action, index),
                        rect(110., y + index as f32 * 38., 480., 34.),
                        BackgroundColor(Color::NONE),
                    ))
                    .with_children(|button| {
                        button.spawn((
                            NativeText::new(label.to_uppercase(), 27.),
                            rect(10., 0., 460., 34.),
                        ));
                    });
            }
            parent.spawn((
                NativeText::new(
                    "ENTER  Select       ESC  Back       Arrow Keys  Navigate",
                    18.,
                ),
                rect(110., 690., 900., 26.),
            ));
        });
}

fn hide(mut commands: Commands, roots: Query<Entity, With<FrontendRoot>>) {
    for root in &roots {
        commands.entity(root).despawn();
    }
}

fn buttons(
    event: On<PointerPress>,
    buttons: Query<&FrontendButton>,
    mut actions: MessageWriter<FrontendAction>,
    mut selected: ResMut<FrontendSelection>,
) {
    if event.button == PointerButton::Primary
        && let Ok(button) = buttons.get(event.entity)
    {
        selected.0 = button.1;
        actions.write(button.0);
    }
}

fn shortcuts(
    keys: Res<ButtonInput<KeyCode>>,
    state: Res<State<GameState>>,
    mut actions: MessageWriter<FrontendAction>,
    mut selected: ResMut<FrontendSelection>,
    buttons: Query<&FrontendButton>,
) {
    if keys.just_pressed(KeyCode::ArrowUp) && !buttons.is_empty() {
        selected.0 = (selected.0 + buttons.iter().count() - 1) % buttons.iter().count();
    }
    if keys.just_pressed(KeyCode::ArrowDown) && !buttons.is_empty() {
        selected.0 = (selected.0 + 1) % buttons.iter().count();
    }
    let chosen = buttons
        .iter()
        .find(|button| button.1 == selected.0)
        .map(|button| button.0);
    match **state {
        GameState::MainMenu => {
            if keys.just_pressed(KeyCode::Enter) {
                actions.write(chosen.unwrap_or(FrontendAction::StartSolo));
            }
            if keys.just_pressed(KeyCode::Digit1) {
                actions.write(FrontendAction::SelectDifficulty(Difficulty::Original));
            }
            if keys.just_pressed(KeyCode::Digit2) {
                actions.write(FrontendAction::SelectDifficulty(Difficulty::Easy));
            }
            if keys.just_pressed(KeyCode::Escape) {
                actions.write(FrontendAction::Exit);
            }
        }
        GameState::GameOver => {
            if keys.just_pressed(KeyCode::Enter)
                && let Some(action) = chosen
            {
                actions.write(action);
            }
            if keys.just_pressed(KeyCode::Escape) {
                actions.write(FrontendAction::MainMenu);
            }
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
    mut title: Query<&mut NativeText, With<FrontendRootTitle>>,
) {
    for action in actions.read() {
        match *action {
            FrontendAction::SelectDifficulty(value) if **state == GameState::MainMenu => {
                *difficulty = value;
                for mut text in &mut title {
                    text.value = format!("ZOMBIES\nORIGINS - {}", value.label().to_uppercase());
                }
            }
            FrontendAction::StartSolo if **state == GameState::MainMenu => {
                next.set(GameState::StartingSession);
            }
            FrontendAction::Restart
                if matches!(**state, GameState::Paused | GameState::GameOver) =>
            {
                next.set(GameState::StartingSession);
            }
            FrontendAction::MainMenu
                if matches!(**state, GameState::Paused | GameState::GameOver) =>
            {
                commands.queue(crate::session::teardown);
                next.set(GameState::MainMenu);
            }
            FrontendAction::Exit => {
                exit.write(AppExit::Success);
            }
            _ => {}
        }
    }
}

#[derive(Component)]
struct FrontendRootTitle;

fn selection(
    mut selected: ResMut<FrontendSelection>,
    mut buttons: Query<(&FrontendButton, Ref<Hovered>, &mut BackgroundColor)>,
) {
    for (button, hovered, _) in &buttons {
        if hovered.is_changed() && hovered.0 {
            selected.0 = button.1;
        }
    }
    for (button, _, mut background) in &mut buttons {
        background.0 = if button.1 == selected.0 {
            Color::srgba(0.85, 0.3, 0.02, 0.8)
        } else {
            Color::NONE
        };
    }
}

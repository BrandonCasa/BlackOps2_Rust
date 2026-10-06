use crate::{GameState, assets::LoadFailure, gameplay::Session, life::Vitality, player::Player};
use bevy::prelude::*;

#[derive(Component)]
struct HudText;
#[derive(Component)]
pub(crate) struct PlayerOverlay;
pub struct HudPlugin;
impl Plugin for HudPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Startup,
            setup.after(crate::presentation::first_person_light),
        )
        .add_systems(Update, update);
    }
}
fn setup(
    mut commands: Commands,
    cameras: Query<Entity, With<crate::presentation::ViewModelCamera>>,
) {
    let Ok(camera) = cameras.single() else {
        return;
    };
    commands.spawn((
        Text::new("Loading Origins…"),
        TextFont {
            font_size: FontSize::Px(19.),
            ..default()
        },
        TextColor(Color::WHITE),
        Node {
            position_type: PositionType::Absolute,
            left: px(22.),
            bottom: px(20.),
            right: px(22.),
            ..default()
        },
        HudText,
        PlayerOverlay,
        UiTargetCamera(camera),
    ));
    commands.spawn((
        Text::new("+"),
        PlayerOverlay,
        UiTargetCamera(camera),
        TextFont {
            font_size: FontSize::Px(24.),
            ..default()
        },
        TextColor(Color::WHITE),
        Node {
            position_type: PositionType::Absolute,
            left: percent(49.5),
            top: percent(48.),
            ..default()
        },
    ));
}
fn update(
    state: Res<State<GameState>>,
    failure: Res<LoadFailure>,
    session: Res<Session>,
    definition: Option<Res<crate::assets::GameDefinition>>,
    inventories: Query<&crate::weapons::WeaponInventory, With<Player>>,
    players: Query<&Vitality, With<Player>>,
    mut text: Query<&mut Text, With<HudText>>,
) {
    let Ok(mut text) = text.single_mut() else {
        return;
    };
    let health = players
        .single()
        .map(|v| v.current().unwrap_or(0))
        .unwrap_or(0);
    let inventory = inventories.single().ok();
    let weapon = inventory.and_then(|i| {
        definition
            .as_ref()
            .and_then(|d| i.definition(&d.content.weapons))
    });
    let ammo = inventory.and_then(|i| i.active()).map_or(0, |w| w.magazine);
    let reserve = inventory.zip(weapon).map_or(0, |(i, w)| i.reserve(w));
    let value = match **state {
        GameState::Loading => "Loading Origins…".into(),
        GameState::LoadError => format!("Unable to load Origins\n{}", failure.0),
        GameState::Paused => "Paused — Escape or click to resume\nWASD move | mouse look | Shift sprint | Space jump | Ctrl crouch\nLeft click fire | right click aim | V melee | R reload | Q/wheel cycle | 1–3 weapon slots".into(),
        GameState::GameOver => format!("Game over — Round {}\nPress R to restart", session.round),
        GameState::Playing => format!(
            "Round {}    Health {}    Points {}\n{}    Ammo {}/{}{}",
            session.round,
            health,
            session.points,
            weapon.map_or("", |w| w.name.as_str()),
            ammo,
            reserve,
            if inventory.is_some_and(|i| matches!(i.action, crate::weapons::WeaponAction::Reloading { .. } | crate::weapons::WeaponAction::ShellReload { .. })) {
                "  Reloading"
            } else {
                ""
            }
        ),
    };
    if text.0 != value {
        text.0 = value;
    }
}

use crate::{
    GameState, assets::LoadFailure, gameplay::Session, life::Vitality, player::Player,
    rendering::GameCamera,
};
use bevy::prelude::*;

#[derive(Component)]
struct HudText;
pub struct HudPlugin;
impl Plugin for HudPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup.after(crate::rendering::setup))
            .add_systems(Update, update);
    }
}
fn setup(
    mut commands: Commands,
    cameras: Query<Entity, With<GameCamera>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn((
        Text::new("Loading Origins…"),
        TextFont {
            font_size: FontSize::Px(24.),
            ..default()
        },
        TextColor(Color::WHITE),
        Node {
            position_type: PositionType::Absolute,
            left: px(22.),
            top: px(20.),
            ..default()
        },
        HudText,
    ));
    commands.spawn((
        Text::new("+"),
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
    for entity in &cameras {
        commands.entity(entity).with_children(|parent| {
            parent.spawn((
                Mesh3d(meshes.add(Cuboid::new(0.08, 0.1, 0.3))),
                MeshMaterial3d(materials.add(Color::srgb(0.12, 0.13, 0.15))),
                Transform::from_xyz(0.25, -0.22, -0.5),
                Name::new("Temporary pistol"),
            ));
        });
    }
}
fn update(
    state: Res<State<GameState>>,
    failure: Res<LoadFailure>,
    session: Res<Session>,
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
    **text = match **state {
        GameState::Loading => "Loading Origins…".into(),
        GameState::LoadError => format!("Unable to load Origins\n{}", failure.0),
        GameState::Paused => "Paused\nEscape or click to resume".into(),
        GameState::GameOver => format!("Game over — Round {}\nPress R to restart", session.round),
        GameState::Playing => format!(
            "Round {}    Health {}    Points {}\nAmmo {}/{}{}\nWASD | mouse look | Shift sprint | Space jump | Ctrl crouch\nLeft click fire | right click/V melee | R reload | Escape pause",
            session.round,
            health,
            session.points,
            session.ammo,
            session.reserve,
            if session.reloading > 0. {
                "  Reloading"
            } else {
                ""
            }
        ),
    };
}

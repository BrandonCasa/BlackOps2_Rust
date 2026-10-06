use crate::{GameState, assets::LoadFailure, gameplay::Session, life::Vitality, player::Player};
use bevy::prelude::*;

#[derive(Component)]
struct HudText;
#[derive(Component)]
pub(crate) struct Crosshair;
#[derive(Component)]
pub(crate) struct PlayerOverlay;
pub struct HudPlugin;
impl Plugin for HudPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Startup,
            setup.after(crate::presentation::first_person_light),
        )
        .add_systems(Update, update)
        .add_systems(Update, damage_feedback)
        .add_systems(Update, crosshair.run_if(in_state(GameState::Playing)));
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
        Crosshair,
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
    // Reconstructed peripheral red feedback; Bevy UI owns compositing and lifetime.
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                width: percent(100.),
                height: percent(100.),
                ..default()
            },
            GlobalZIndex(20),
            PlayerOverlay,
            UiTargetCamera(camera),
        ))
        .with_children(|parent| {
            for band in 0..12 {
                let weight = (1. - band as f32 / 12.).powi(2);
                let horizontal = band as f32 * 9. / 12.;
                let vertical = band as f32 * 8. / 12.;
                for node in [
                    Node {
                        left: percent(horizontal),
                        top: px(0.),
                        width: percent(9. / 12.),
                        height: percent(100.),
                        ..default()
                    },
                    Node {
                        right: percent(horizontal),
                        top: px(0.),
                        width: percent(9. / 12.),
                        height: percent(100.),
                        ..default()
                    },
                    Node {
                        left: px(0.),
                        top: percent(vertical),
                        width: percent(100.),
                        height: percent(8. / 12.),
                        ..default()
                    },
                    Node {
                        left: px(0.),
                        bottom: percent(vertical),
                        width: percent(100.),
                        height: percent(8. / 12.),
                        ..default()
                    },
                ] {
                    parent.spawn((
                        Node {
                            position_type: PositionType::Absolute,
                            ..node
                        },
                        DamageEdge { weight },
                        BackgroundColor(Color::NONE),
                    ));
                }
            }
        });
}
#[derive(Component)]
struct DamageEdge {
    weight: f32,
}
fn damage_feedback(
    players: Query<(&Vitality, &crate::player_health::Recovery), With<Player>>,
    definition: Option<Res<crate::assets::GameDefinition>>,
    state: Res<State<GameState>>,
    mut edges: Query<(&DamageEdge, &mut BackgroundColor)>,
) {
    let alpha = if **state == GameState::Playing {
        players.single().ok().zip(definition.as_ref()).map_or(
            0.,
            |((health, recovery), definition)| {
                let flash = recovery.feedback
                    * (1. - recovery.age / definition.player_health.feedback_seconds).max(0.);
                let critical =
                    if recovery.critical && health.state() == crate::life::LifeState::Standing {
                        0.3 + 0.15 * (recovery.age * std::f32::consts::TAU / 0.8).sin()
                    } else {
                        0.
                    };
                (flash + critical).clamp(0., 0.65)
            },
        )
    } else {
        0.
    };
    for (edge, mut background) in &mut edges {
        background.0 = Color::srgba(0.45, 0., 0., alpha * edge.weight);
    }
}
fn crosshair(
    inventories: Query<&crate::weapons::WeaponInventory, With<Player>>,
    mut text: Query<&mut Visibility, With<Crosshair>>,
) {
    let aiming = inventories.single().is_ok_and(|i| i.ads > 0.05);
    for mut visibility in &mut text {
        *visibility = if aiming {
            Visibility::Hidden
        } else {
            Visibility::Inherited
        };
    }
}
fn update(
    state: Res<State<GameState>>,
    failure: Res<LoadFailure>,
    session: Res<Session>,
    definition: Option<Res<crate::assets::GameDefinition>>,
    inventories: Query<&crate::weapons::WeaponInventory, With<Player>>,
    players: Query<&Vitality, With<Player>>,
    focus: Res<crate::interactions::InteractionFocus>,
    grenades: Query<&crate::equipment::GrenadeInventory, With<Player>>,
    mines: Query<&crate::mines::MineInventory, With<Player>>,
    tactical: Query<&crate::equipment::TacticalInventory, With<Player>>,
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
    let mut value = match **state {
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
    if **state == GameState::Playing && !focus.prompt.is_empty() {
        value.push_str(&format!("\n{}", focus.prompt));
        if let Some(reason) = focus.rejection {
            let reason = match reason {
                crate::interactions::InteractionRejection::InsufficientPoints => {
                    "Not enough points"
                }
                crate::interactions::InteractionRejection::FullAmmo => "Ammo full",
                crate::interactions::InteractionRejection::Busy => "Weapon busy",
                _ => "Unavailable",
            };
            value.push_str(&format!(" — {reason}"));
        }
    }
    if **state == GameState::Playing
        && let Ok(grenades) = grenades.single()
    {
        let label = definition
            .as_ref()
            .and_then(|d| {
                d.content
                    .weapons
                    .iter()
                    .find(|w| w.id == grenades.definition)
            })
            .map_or("Grenades", |w| w.name.as_str());
        value.push_str(&format!("    {} {}", label, grenades.count));
    }
    if **state == GameState::Playing
        && let Ok(mines) = mines.single()
        && mines.definition.is_some()
    {
        value.push_str(&format!("    Claymores {}", mines.count));
    }
    if **state == GameState::Playing
        && let Ok(tactical) = tactical.single()
        && let Some(inventory) = &tactical.0
    {
        let label = definition
            .as_ref()
            .and_then(|d| {
                d.content
                    .weapons
                    .iter()
                    .find(|w| w.id == inventory.definition)
            })
            .map_or("Tactical", |w| w.name.as_str());
        value.push_str(&format!("    {} {}", label, inventory.count));
    }
    if **state == GameState::Paused {
        value.push_str("\nG grenade | T tactical | 4 Claymore");
    }
    if text.0 != value {
        text.0 = value;
    }
}

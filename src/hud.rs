use crate::{GameState, assets::LoadFailure, gameplay::Session, life::Vitality, player::Player};
use bevy::prelude::*;

mod performance;

#[derive(Component)]
struct HudText;
#[derive(Component)]
struct ReviveText;
#[derive(Component)]
pub(crate) struct Crosshair;
#[derive(Component)]
pub(crate) struct PlayerOverlay;
pub struct HudPlugin;
impl Plugin for HudPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(performance::PerformanceOverlayPlugin)
            .add_systems(
                Startup,
                setup.after(crate::presentation::first_person_light),
            )
            .add_systems(Update, update)
            .add_systems(Update, damage_feedback)
            .add_systems(Update, revive_feedback)
            .add_systems(Update, subtitle_feedback)
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
    commands.spawn((
        Text::new(""),
        SubtitleText,
        PlayerOverlay,
        UiTargetCamera(camera),
        TextFont {
            font_size: FontSize::Px(20.),
            ..default()
        },
        TextColor(Color::WHITE),
        TextLayout::justify(Justify::Center),
        Node {
            position_type: PositionType::Absolute,
            left: px(0.),
            bottom: px(120.),
            width: percent(100.),
            ..default()
        },
    ));
    commands.spawn((
        Text::new(""),
        ReviveText,
        PlayerOverlay,
        UiTargetCamera(camera),
        TextFont {
            font_size: FontSize::Px(24.),
            ..default()
        },
        TextColor(Color::WHITE),
        TextLayout::justify(Justify::Center),
        Visibility::Hidden,
        Node {
            position_type: PositionType::Absolute,
            left: px(0.),
            bottom: px(160.),
            width: percent(100.),
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
                (flash
                    + critical
                    + if health.state() == crate::life::LifeState::Downed {
                        0.5
                    } else {
                        0.
                    })
                .clamp(0., 0.65)
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
    perks: Query<&crate::solo_revive::PerkInventory, With<Player>>,
    audio: Res<crate::audio::AudioSettings>,
    mut text: Query<(&mut Text, &mut Node), With<HudText>>,
    generators: Res<crate::generators::Generators>,
    settings: Res<crate::input::ControlSettings>,
) {
    let Ok((mut text, mut node)) = text.single_mut() else {
        return;
    };
    let left = if **state == GameState::Paused {
        percent(45.)
    } else {
        px(22.)
    };
    if node.left != left {
        node.left = left;
    }
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
        GameState::Paused => String::new(),
        GameState::GameOver => format!(
            "Game over — Round {}\nPress {} to restart",
            session.round,
            settings.label(crate::input::Action::Restart)
        ),
        GameState::Playing => format!(
            "Round {}    Health {}    Points {}\n{}    Ammo {}/{}{}",
            session.round,
            health,
            session.points,
            weapon.map_or("", |w| w.name.as_str()),
            ammo,
            reserve,
            if inventory.is_some_and(|i| matches!(
                i.action,
                crate::weapons::WeaponAction::Reloading { .. }
                    | crate::weapons::WeaponAction::ShellReload { .. }
            )) {
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
        && let Some(game) = &definition
    {
        let sites = game
            .content
            .generators
            .iter()
            .filter_map(|d| {
                generators.sites.get(&d.id).map(|s| {
                    let label = match s.state {
                        crate::generators::GeneratorState::Active => "ON".to_owned(),
                        crate::generators::GeneratorState::Capturing => {
                            format!("{}%", (s.progress * 100.) as u32)
                        }
                        _ => "OFF".to_owned(),
                    };
                    format!("{}:{}", d.number, label)
                })
            })
            .collect::<Vec<_>>();
        if !sites.is_empty() {
            value.push_str(&format!("\nGenerators {}", sites.join("  ")));
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
    if **state == GameState::Playing {
        if let Ok(perks) = perks.single()
            && !perks.owned.is_empty()
        {
            if let Some(game) = &definition {
                let names: Vec<&str> = game
                    .content
                    .perks
                    .iter()
                    .filter(|p| perks.owned.contains(&p.id))
                    .map(|p| p.name.as_str())
                    .collect();
                value.push_str(&format!("\n{}", names.join("    ")));
            }
        }
    }
    if **state == GameState::Paused {
        value.push_str(&format!(
            "\n{} grenade | {} tactical | {} Claymore",
            settings.label(crate::input::Action::Lethal),
            settings.label(crate::input::Action::Tactical),
            settings.label(crate::input::Action::Mine)
        ));
        value.push_str(&format!("\nF6 Master {:.0}% | F7 Music {:.0}%\nF8 Effects {:.0}% | F9 Voice {:.0}%\nShift + key lowers volume\nF10 Captions {}",audio.master*100.,audio.music*100.,audio.effects*100.,audio.dialogue*100.,if audio.subtitles{"On"}else{"Off"}));
    }
    if text.0 != value {
        text.0 = value;
    }
}

#[derive(Component)]
struct SubtitleText;
fn subtitle_feedback(
    settings: Res<crate::audio::AudioSettings>,
    subtitle: Res<crate::audio::Subtitle>,
    state: Res<State<GameState>>,
    mut labels: Query<&mut Text, With<SubtitleText>>,
) {
    for mut text in &mut labels {
        text.0 = if settings.subtitles && **state == GameState::Playing {
            subtitle.text.clone()
        } else {
            String::new()
        };
    }
}

fn revive_feedback(
    state: Res<State<GameState>>,
    definition: Option<Res<crate::assets::GameDefinition>>,
    episodes: Query<&crate::solo_revive::SoloEpisode, With<Player>>,
    mut labels: Query<(&mut Text, &mut TextColor, &mut Visibility), With<ReviveText>>,
) {
    for (mut text, mut color, mut visibility) in &mut labels {
        let Some((episode, definition)) = episodes
            .single()
            .ok()
            .zip(definition.as_ref())
            .filter(|_| **state == GameState::Playing)
        else {
            *visibility = Visibility::Hidden;
            continue;
        };
        if text.0.is_empty() {
            text.0 = definition
                .content
                .localized_strings
                .iter()
                .find(|s| s.key == "ZOMBIE_REVIVING_SOLO")
                .map_or("Reviving", |s| s.text.trim())
                .to_owned();
        }
        *visibility = Visibility::Inherited;
        color.0 = Color::srgba(
            1.,
            1.,
            1.,
            (episode.remaining / definition.solo_revive.seconds).clamp(0., 1.),
        );
    }
}

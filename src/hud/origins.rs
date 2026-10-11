use crate::{GameState, player::Player};
use bevy::prelude::*;
#[derive(Component)]
struct InventoryReadout;
#[derive(Component)]
struct PerkRow;
#[derive(Component)]
struct RobotFade;
#[derive(Resource, Default)]
struct PerkCache(Option<Vec<String>>);
pub struct OriginsHudPlugin;
impl Plugin for OriginsHudPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PerkCache>()
            .add_systems(
                Startup,
                setup.after(crate::presentation::first_person_light),
            )
            .add_systems(OnEnter(GameState::PreparingShaders), reset)
            .add_systems(Update, (update, icons).run_if(in_state(GameState::Playing)))
            .add_systems(Update, (visibility, robot_fade));
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
        InventoryReadout,
        super::PlayerOverlay,
        UiTargetCamera(camera),
        Text::new(""),
        TextFont {
            font_size: FontSize::Px(18.),
            ..default()
        },
        TextColor(Color::WHITE),
        Node {
            position_type: PositionType::Absolute,
            left: percent(3.),
            top: percent(16.),
            width: percent(80.),
            ..default()
        },
    ));
    commands.spawn((
        PerkRow,
        super::PlayerOverlay,
        UiTargetCamera(camera),
        Node {
            position_type: PositionType::Absolute,
            left: percent(3.),
            bottom: percent(34.),
            column_gap: px(6.),
            ..default()
        },
    ));
    commands.spawn((
        RobotFade,
        super::PlayerOverlay,
        UiTargetCamera(camera),
        GlobalZIndex(150),
        Node {
            position_type: PositionType::Absolute,
            width: percent(100.),
            height: percent(100.),
            ..default()
        },
        BackgroundColor(Color::NONE),
        Visibility::Hidden,
    ));
}
fn robot_fade(
    state: Res<State<GameState>>,
    players: Query<
        (
            Option<&crate::robots::RobotTransit>,
            Option<&crate::robots::StompReaction>,
            Option<&crate::tank::TankRescue>,
            Option<&crate::finale::FistPickupFlash>,
        ),
        With<Player>,
    >,
    mut overlay: Query<(&mut BackgroundColor, &mut Visibility), With<RobotFade>>,
) {
    let fade = players
        .single()
        .ok()
        .and_then(|(transit, stomp, tank, fist)| {
            [
                transit.map(|t| (t.fade, t.black)),
                stomp.map(|s| (s.fade, s.black)),
                tank.map(|t| (t.fade, true)),
                fist.map(|f| (f.fade, false)),
            ]
            .into_iter()
            .flatten()
            .max_by(|a, b| a.0.total_cmp(&b.0))
        });
    for (mut color, mut visibility) in &mut overlay {
        let (opacity, black) = fade.unwrap_or((0., true));
        *visibility = if **state == GameState::Playing && opacity > 0. {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        let value = if black { 0. } else { 1. };
        color.0 = Color::srgba(value, value, value, opacity.clamp(0., 1.));
    }
}
fn update(
    input: Res<crate::input::GameInput>,
    raids: Res<crate::generators::GeneratorRaids>,
    (crafting, parts): (
        Res<crate::crafting::Crafting>,
        Res<crate::environment::parts::Parts>,
    ),
    (upgrades, quest, rituals): (
        Res<crate::staff_upgrades::StaffUpgrades>,
        Res<crate::quests::MainQuest>,
        Res<crate::rituals::Rituals>,
    ),
    players: Query<
        (
            &crate::weapons::WeaponInventory,
            &crate::crafting::BuildableInventory,
            &crate::environment::digging::Digger,
        ),
        With<Player>,
    >,
    mut readouts: Query<(&mut Text, &mut Visibility), With<InventoryReadout>>,
) {
    let Ok((weapons, buildable, dig)) = players.single() else {
        return;
    };
    for (mut text, mut visible) in &mut readouts {
        *visible = Visibility::Inherited;
        let equipment = buildable
            .equipped
            .as_deref()
            .filter(|id| {
                buildable.owned.contains(*id)
                    && (*id != "tomb_shield_zm" || buildable.shield_health > 0)
            })
            .unwrap_or("");
        let mut value = format!(
            "{}{}{}  {}{}",
            if dig.shovel { "Shovel " } else { "" },
            if dig.golden { "Golden " } else { "" },
            if dig.helmet { "Golden Helmet" } else { "" },
            equipment,
            if weapons.charge_level > 0 {
                format!("  Staff charge {}", weapons.charge_level)
            } else {
                String::new()
            }
        );
        if let Some(target) = &raids.target {
            value.push_str(&format!(
                "\n{}: {}",
                if raids.under_attack {
                    "GENERATOR ATTACK"
                } else {
                    "GENERATOR RAID MOVING TO"
                },
                target.0
            ));
        }
        if input.scoreboard {
            value.push_str(&format!("\nParts {}\nRecords {}\nStaffs {}\nChallenges: {}/115 headshots  {}/6 generators  {}/30000 points  {}/4 chests\nQuest: {:?}",parts.pieces.iter().filter(|(_,p)|matches!(p,crate::environment::parts::PartState::Owned{..})).map(|(p,_)|p.strip_prefix("part.").unwrap_or(p)).collect::<Vec<_>>().join(", "),crafting.records.iter().cloned().collect::<Vec<_>>().join(", "),[("Fire","fire"),("Ice","water"),("Wind","air"),("Lightning","lightning")].iter().enumerate().map(|(i,(n,element))|format!("{n}:{}",if !crafting.crafted.contains(&format!("elemental_staff_{element}")){"unassembled"}else if upgrades.charged[i]{"upgraded"}else if upgrades.inserted[i]{"charging"}else{"base"})).collect::<Vec<_>>().join("  "),rituals.headshots,rituals.generators,rituals.spent,rituals.chests,quest.stage));
        }
        if text.0 != value {
            text.0 = value;
        }
    }
}
fn visibility(
    state: Res<State<GameState>>,
    mut roots: Query<
        (&mut Visibility, Option<&mut Text>),
        Or<(With<InventoryReadout>, With<PerkRow>)>,
    >,
) {
    for (mut visibility, text) in &mut roots {
        *visibility = if **state == GameState::Playing {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        if **state != GameState::Playing
            && let Some(mut text) = text
        {
            text.0.clear();
        }
    }
}
fn reset(
    mut commands: Commands,
    mut cache: ResMut<PerkCache>,
    roots: Query<&Children, With<PerkRow>>,
) {
    cache.0 = None;
    for children in &roots {
        for child in children.iter() {
            commands.entity(child).despawn();
        }
    }
}
fn icons(
    mut commands: Commands,
    game: Res<crate::assets::GameDefinition>,
    server: Res<AssetServer>,
    players: Query<&crate::solo_revive::PerkInventory, With<Player>>,
    roots: Query<(Entity, Option<&Children>), With<PerkRow>>,
    mut previous: ResMut<PerkCache>,
) {
    let Ok(perks) = players.single() else {
        return;
    };
    let ids: Vec<_> = perks.owned.iter().map(|p| p.0.clone()).collect();
    if previous.0.as_ref() == Some(&ids) {
        return;
    }
    previous.0 = Some(ids);
    for (root, children) in &roots {
        if let Some(children) = children {
            for child in children.iter() {
                commands.entity(child).despawn();
            }
        }
        commands.entity(root).with_children(|parent| {
            for id in &perks.owned {
                if let Some(perk) = game.content.perks.iter().find(|p| p.id == *id) {
                    if let Some(icon) = perk
                        .icon
                        .as_ref()
                        .and_then(|id| game.content.ui_assets.iter().find(|a| a.id == *id))
                    {
                        parent.spawn((
                            ImageNode::new(server.load(icon.path.clone())),
                            Node {
                                width: px(42.),
                                height: px(42.),
                                ..default()
                            },
                        ));
                    } else {
                        parent.spawn((
                            Text::new(&perk.name),
                            TextFont {
                                font_size: FontSize::Px(16.),
                                ..default()
                            },
                        ));
                    }
                }
            }
        });
    }
}

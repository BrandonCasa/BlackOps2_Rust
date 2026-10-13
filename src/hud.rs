use crate::{GameState, life::Vitality, player::Player};
use bevy::prelude::*;

pub(crate) mod native;
mod origins;
mod performance;
mod scoreboard;
mod widgets;

#[derive(Component)]
pub(crate) struct Crosshair;
#[derive(Component)]
pub(crate) struct PlayerOverlay;
#[derive(Component)]
struct DamageEdge;

pub struct HudPlugin;
impl Plugin for HudPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            performance::PerformanceOverlayPlugin,
            origins::OriginsHudPlugin,
        ))
        .add_systems(OnEnter(GameState::PreparingShaders), setup)
        .add_systems(Update, damage_feedback);
    }
}

fn setup(
    mut commands: Commands,
    cameras: Query<Entity, With<crate::rendering::GameCamera>>,
    server: Res<AssetServer>,
    game: Res<crate::assets::GameDefinition>,
    existing: Query<Entity, With<DamageEdge>>,
) {
    let Ok(camera) = cameras.single() else { return };
    for entity in &existing {
        commands.entity(entity).despawn();
    }
    let Some(image) = game
        .content
        .ui_assets
        .iter()
        .find(|image| image.id.0 == "ui.overlay_low_health")
    else {
        return;
    };
    commands.spawn((
        PlayerOverlay,
        DamageEdge,
        ImageNode {
            image: server.load(image.path.clone()),
            color: Color::NONE,
            ..default()
        },
        UiTargetCamera(camera),
        GlobalZIndex(20),
        Pickable::IGNORE,
        Node {
            position_type: PositionType::Absolute,
            width: percent(100.),
            height: percent(100.),
            ..default()
        },
    ));
}

fn damage_feedback(
    players: Query<(&Vitality, &crate::player_health::Recovery), With<Player>>,
    definition: Option<Res<crate::assets::GameDefinition>>,
    state: Res<State<GameState>>,
    mut edges: Query<&mut ImageNode, With<DamageEdge>>,
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
    for mut image in &mut edges {
        image.color = Color::srgba(1., 1., 1., alpha);
    }
}

pub(crate) fn readout(world: &mut World) -> String {
    let state = *world.resource::<State<GameState>>().get();
    let scoreboard =
        state == GameState::GameOver || world.resource::<crate::input::GameInput>().scoreboard;
    world
        .query::<(&origins::Field, &native::NativeText)>()
        .iter(world)
        .filter_map(|(field, text)| {
            let name = match field {
                origins::Field::Score if scoreboard => "Score",
                origins::Field::Kills if scoreboard => "Kills",
                origins::Field::Downs if scoreboard => "Downs",
                origins::Field::Headshots if scoreboard => "Headshots",
                origins::Field::GameOver if scoreboard => "Scoreboard",
                origins::Field::Weapon if !scoreboard => "Weapon",
                origins::Field::Points if !scoreboard => "Points",
                _ => return None,
            };
            Some(format!("{name} {}", text.value))
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(all(debug_assertions, feature = "dev-tools"))]
pub(crate) fn snapshot(world: &mut World) -> serde_json::Value {
    let labels: Vec<_> = world
        .query::<(
            Entity,
            &native::NativeText,
            &InheritedVisibility,
            &ComputedNode,
        )>()
        .iter(world)
        .filter(|(_, _, visible, _)| visible.get())
        .map(|(entity, text, _, node)| {
            serde_json::json!({"entity":entity.to_bits(), "text":text.value,
            "size":node.size().to_array(), "font":text.style, "font_size":text.size})
        })
        .collect();
    let visible_images: Vec<_> = world
        .query::<(&ImageNode, &InheritedVisibility, &ComputedNode)>()
        .iter(world)
        .filter(|(_, visible, _)| visible.get())
        .map(|(image, _, node)| (image.image.clone(), node.size()))
        .collect();
    let images = world.resource::<Assets<Image>>();
    let loaded = visible_images
        .iter()
        .filter(|(handle, _)| images.contains(handle))
        .count();
    let server = world.resource::<AssetServer>();
    let mut paths = std::collections::BTreeMap::<String, usize>::new();
    for (handle, _) in &visible_images {
        let path = server
            .get_path(handle.id())
            .map_or_else(|| "<unregistered>".to_string(), |path| path.to_string());
        *paths.entry(path).or_default() += 1;
    }
    let crosshair = world
        .query_filtered::<&InheritedVisibility, With<Crosshair>>()
        .iter(world)
        .any(|visible| visible.get());
    let crafting = world.resource::<crate::crafting::Crafting>();
    let inventory = serde_json::json!({"gramophone":crafting.gramophone,
        "records":crafting.records,"consumed":crafting.consumed});
    let targets: Vec<_> = world.query::<(Entity, &crate::crafting::CraftTarget, &Transform)>()
        .iter(world).filter_map(|(entity, target, transform)| {
            let crate::crafting::CraftTarget::Part(id) = target else { return None };
            Some(serde_json::json!({"entity":entity.to_bits(),"id":id,"position":transform.translation.to_array()}))
        }).collect();
    serde_json::json!({"labels":labels, "visible_images":visible_images.len(), "loaded_images":loaded,
        "images":paths, "crosshair":crosshair,"inventory":inventory,"part_targets":targets,
        "scoreboard":world.resource::<crate::input::GameInput>().scoreboard,
        "selected_menu_row":(*world.resource::<State<GameState>>().get() == GameState::Paused)
            .then(|| world.resource::<crate::input::menu::ControlMenu>().selected)})
}

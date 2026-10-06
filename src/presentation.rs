//! Bevy-owned first-person scenes and action-driven animation playback.
use crate::{
    GameState,
    assets::GameDefinition,
    gameplay::SessionEntity,
    hud::PlayerOverlay,
    origins::content::{WeaponAnimation, WeaponId},
    player::Player,
    rendering::GameCamera,
    weapons::{WeaponInventory, WeaponSignal},
};
use bevy::{
    camera::{
        Exposure, Hdr,
        visibility::{NoFrustumCulling, RenderLayers},
    },
    core_pipeline::tonemapping::Tonemapping,
    light::NotShadowCaster,
    prelude::*,
    world_serialization::WorldInstanceReady,
};
use std::collections::BTreeMap;

pub struct PresentationPlugin;
#[derive(Component)]
pub(crate) struct ViewModelCamera;
#[derive(Component)]
struct FirstPerson {
    owner: Entity,
    weapon: WeaponId,
}
#[derive(Component)]
struct Rig {
    owner: Entity,
    source: Handle<Gltf>,
    graph: Option<Handle<AnimationGraph>>,
    nodes: BTreeMap<String, AnimationNodeIndex>,
    players: Vec<Entity>,
    sequence: Option<u64>,
    arms: bool,
    attached: bool,
}
impl Plugin for PresentationPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (spawn_viewmodels, build_graphs, attach_weapons, animate).chain(),
        )
        .add_systems(Startup, first_person_light.after(crate::rendering::setup))
        .add_observer(rig_ready);
    }
}
pub(crate) fn first_person_light(
    mut commands: Commands,
    cameras: Query<Entity, With<GameCamera>>,
    profile: Res<crate::rendering::profile::VisualProfile>,
) {
    for camera in &cameras {
        commands.entity(camera).with_children(|parent| {
            parent.spawn((
                Camera3d::default(),
                Camera {
                    order: 1,
                    clear_color: ClearColorConfig::None,
                    ..default()
                },
                Projection::Perspective(PerspectiveProjection {
                    fov: 65_f32.to_radians(),
                    near: 0.01,
                    far: 4.,
                    ..default()
                }),
                Transform::default(),
                Hdr,
                Exposure {
                    ev100: profile.indoor_ev100,
                },
                Tonemapping::AgX,
                match profile.antialiasing {
                    crate::rendering::profile::Antialiasing::Msaa4 => Msaa::Sample4,
                    crate::rendering::profile::Antialiasing::Smaa => Msaa::Off,
                },
                RenderLayers::layer(1),
                ViewModelCamera,
                Name::new("First-person camera"),
            ));
            parent.spawn((
                PointLight {
                    color: Color::srgb(0.70, 0.80, 0.90),
                    intensity: 25_000.,
                    range: 3.,
                    radius: 0.15,
                    shadow_maps_enabled: false,
                    ..default()
                },
                Transform::from_xyz(-0.3, 0.5, 0.4),
                RenderLayers::layer(1),
                Name::new("First-person reconstruction fill"),
            ));
        });
    }
}
fn spawn_viewmodels(
    mut commands: Commands,
    server: Res<AssetServer>,
    definition: Option<Res<GameDefinition>>,
    state: Res<State<GameState>>,
    owners: Query<(Entity, &WeaponInventory), With<Player>>,
    cameras: Query<Entity, With<GameCamera>>,
    existing: Query<(Entity, &FirstPerson)>,
) {
    if **state != GameState::Playing {
        return;
    }
    let (Some(definition), Ok(camera)) = (definition, cameras.single()) else {
        return;
    };
    for (owner, inventory) in &owners {
        let Some(weapon) = inventory.definition(&definition.content.weapons) else {
            continue;
        };
        if existing
            .iter()
            .any(|(_, view)| view.owner == owner && view.weapon == weapon.id)
        {
            continue;
        }
        for (entity, view) in &existing {
            if view.owner == owner {
                commands.entity(entity).despawn();
            }
        }
        let Some(scene) = weapon
            .presentation
            .as_ref()
            .and_then(|id| {
                definition
                    .content
                    .presentations
                    .iter()
                    .find(|p| p.id == *id)
            })
            .and_then(|p| p.scene.as_ref())
        else {
            continue;
        };
        let arms = "actors/c_zom_dempsey_viewhands_lod0.gltf#Scene0";
        let root = commands
            .spawn((
                FirstPerson {
                    owner,
                    weapon: weapon.id.clone(),
                },
                SessionEntity,
                PlayerOverlay,
                Transform::from_rotation(Quat::from_rotation_y(std::f32::consts::FRAC_PI_2)),
                Visibility::Inherited,
                Name::new("Original first-person rig"),
            ))
            .id();
        commands.entity(camera).add_child(root);
        for (is_arms, scene) in [(true, arms), (false, scene.as_str())] {
            let source = server.load(scene.split('#').next().unwrap_or(scene).to_owned());
            let rig = commands
                .spawn((
                    WorldAssetRoot(server.load(scene.to_owned())),
                    Rig {
                        owner,
                        source,
                        graph: None,
                        nodes: BTreeMap::new(),
                        players: Vec::new(),
                        sequence: None,
                        arms: is_arms,
                        attached: is_arms,
                    },
                    Transform::default(),
                    if is_arms {
                        Visibility::Inherited
                    } else {
                        Visibility::Hidden
                    },
                ))
                .id();
            commands.entity(root).add_child(rig);
        }
    }
}
fn attach_weapons(
    mut commands: Commands,
    mut rigs: Query<(Entity, &mut Rig)>,
    children: Query<&Children>,
    names: Query<&Name>,
) {
    let sockets: Vec<_> = rigs
        .iter()
        .filter(|(_, rig)| rig.arms && !rig.players.is_empty())
        .filter_map(|(entity, rig)| {
            children
                .iter_descendants(entity)
                .find(|&child| {
                    names
                        .get(child)
                        .is_ok_and(|name| name.as_str() == "tag_weapon")
                })
                .map(|socket| (rig.owner, socket))
        })
        .collect();
    for (entity, mut rig) in &mut rigs {
        if rig.attached || rig.players.is_empty() {
            continue;
        }
        if let Some((_, socket)) = sockets.iter().find(|(owner, _)| *owner == rig.owner) {
            // The hand socket is already in OAT's Y-up basis. Undo that basis on
            // the gun scene root before inheriting the authored socket pose.
            commands.entity(*socket).add_child(entity);
            commands.entity(entity).insert((
                Transform::from_rotation(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2)),
                Visibility::Inherited,
            ));
            rig.attached = true;
        }
    }
}
fn rig_ready(
    event: On<WorldInstanceReady>,
    mut commands: Commands,
    mut rigs: Query<&mut Rig>,
    children: Query<&Children>,
    players: Query<(), With<AnimationPlayer>>,
    meshes: Query<(), With<Mesh3d>>,
) {
    let Ok(mut rig) = rigs.get_mut(event.entity) else {
        return;
    };
    for child in children.iter_descendants(event.entity) {
        if players.contains(child) {
            rig.players.push(child);
        }
        if meshes.contains(child) {
            commands.entity(child).insert((
                NotShadowCaster,
                RenderLayers::layer(1),
                NoFrustumCulling,
            ));
        }
    }
    info!(
        "First-person scene ready: {} animation players",
        rig.players.len()
    );
}
fn build_graphs(
    mut rigs: Query<&mut Rig>,
    assets: Res<Assets<Gltf>>,
    mut graphs: ResMut<Assets<AnimationGraph>>,
) {
    for mut rig in &mut rigs {
        if rig.graph.is_some() {
            continue;
        }
        let Some(asset) = assets.get(&rig.source) else {
            continue;
        };
        let mut named: Vec<_> = asset.named_animations.iter().collect();
        named.sort_by(|(a, _), (b, _)| a.cmp(b));
        let (graph, nodes) =
            AnimationGraph::from_clips(named.iter().map(|(_, handle)| (*handle).clone()));
        rig.nodes = named
            .into_iter()
            .zip(nodes)
            .map(|((name, _), node)| (name.to_string(), node))
            .collect();
        rig.graph = Some(graphs.add(graph));
    }
}
fn animate(
    mut commands: Commands,
    state: Res<State<GameState>>,
    definition: Option<Res<GameDefinition>>,
    inventories: Query<&WeaponInventory>,
    mut rigs: Query<&mut Rig>,
    mut players: Query<&mut AnimationPlayer>,
) {
    let Some(definition) = definition else {
        return;
    };
    for mut rig in &mut rigs {
        let Ok(inventory) = inventories.get(rig.owner) else {
            continue;
        };
        let Some(weapon) = inventory.definition(&definition.content.weapons) else {
            continue;
        };
        let Some(graph) = rig.graph.clone() else {
            continue;
        };
        let action = match inventory.signal {
            WeaponSignal::Equip => WeaponAnimation::Equip,
            WeaponSignal::Unequip => WeaponAnimation::Unequip,
            WeaponSignal::Fire => WeaponAnimation::Fire,
            WeaponSignal::Reload => WeaponAnimation::Reload,
            WeaponSignal::ReloadEmpty => WeaponAnimation::ReloadEmpty,
            WeaponSignal::ShellInsert => WeaponAnimation::Reload,
            WeaponSignal::Melee => WeaponAnimation::Melee,
            _ => WeaponAnimation::Idle,
        };
        let clip = weapon
            .animation_actions
            .get(&action)
            .or_else(|| weapon.animation_actions.get(&WeaponAnimation::Idle));
        let name = clip.and_then(|id| id.0.rsplit('.').next());
        let Some(node) = name.and_then(|name| rig.nodes.get(name)).copied() else {
            continue;
        };
        if rig.players.is_empty() {
            continue;
        }
        for &entity in &rig.players {
            let Ok(mut player) = players.get_mut(entity) else {
                continue;
            };
            if rig.sequence != Some(inventory.sequence) {
                commands
                    .entity(entity)
                    .insert(AnimationGraphHandle(graph.clone()));
                player.stop_all();
                let active = player.play(node);
                if action == WeaponAnimation::Idle {
                    active.repeat();
                }
                // Action timers remain authoritative; pause preserves the current pose.
            }
            if **state == GameState::Playing {
                player.resume_all();
            } else {
                player.pause_all();
            }
        }
        rig.sequence = Some(inventory.sequence);
    }
}

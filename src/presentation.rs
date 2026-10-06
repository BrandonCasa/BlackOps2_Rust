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

pub(crate) mod offhand;
pub use offhand::EquipmentPresentationEvidence;
pub(crate) use offhand::selection as equipment_selection;
pub struct PresentationPlugin;
#[derive(Component)]
pub(crate) struct ViewModelCamera;
#[derive(Component)]
struct AimingJoint {
    owner: Entity,
    weapon: WeaponId,
    translation: Vec3,
    rotation: Quat,
}
#[derive(Component)]
struct FirstPerson {
    owner: Entity,
    weapon: WeaponId,
}
#[derive(Component)]
struct ViewModelGroup {
    owner: Entity,
}
#[derive(Component)]
struct Rig {
    owner: Entity,
    weapon: WeaponId,
    phase: Option<WeaponAnimation>,
    tracked: Vec<(Entity, Option<(Vec3, Quat)>)>,
    sampled_phase: Option<WeaponAnimation>,
    source: Handle<Gltf>,
    graph: Option<Handle<AnimationGraph>>,
    nodes: BTreeMap<String, AnimationNodeIndex>,
    players: Vec<Entity>,
    sequence: Option<u64>,
    arms: bool,
    attached: bool,
    ready: bool,
    knife: bool,
}
impl Plugin for PresentationPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<offhand::GraphCache>()
            .init_resource::<EquipmentPresentationEvidence>()
            .add_systems(
                Update,
                (
                    spawn_viewmodels,
                    build_graphs,
                    attach_weapons,
                    offhand::select_visible,
                    animate,
                )
                    .chain(),
            )
            .add_systems(Startup, first_person_light.after(crate::rendering::setup))
            .add_systems(
                PostUpdate,
                aim_pose
                    .after(bevy::app::AnimationSystems)
                    .before(bevy::transform::TransformSystems::Propagate),
            )
            .add_systems(
                PostUpdate,
                offhand::record_motion
                    .after(aim_pose)
                    .before(bevy::transform::TransformSystems::Propagate),
            )
            .add_systems(Last, offhand::record_visibility)
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
    owners: offhand::Owners,
    cameras: Query<Entity, With<GameCamera>>,
    existing: Query<(Entity, &FirstPerson)>,
    groups: Query<(Entity, &ViewModelGroup)>,
) {
    if **state != GameState::Playing {
        return;
    }
    let (Some(definition), Ok(camera)) = (definition, cameras.single()) else {
        return;
    };
    for (owner, inventory, lethal, tactical, mines, drinking) in &owners {
        let mut wanted = offhand::owned(inventory, lethal, tactical, mines);
        if let Some(drink) = drinking {
            wanted.push(drink.bottle.clone());
        }
        let parent = groups
            .iter()
            .find(|(_, group)| group.owner == owner)
            .map(|(entity, _)| entity)
            .unwrap_or_else(|| {
                let entity = commands
                    .spawn((
                        ViewModelGroup { owner },
                        SessionEntity,
                        PlayerOverlay,
                        Transform::default(),
                        Visibility::Inherited,
                    ))
                    .id();
                commands.entity(camera).add_child(entity);
                entity
            });
        for (entity, view) in &existing {
            if view.owner == owner && !wanted.contains(&view.weapon) {
                commands.entity(entity).despawn();
            }
        }
        for id in wanted {
            if existing
                .iter()
                .any(|(_, view)| view.owner == owner && view.weapon == id)
            {
                continue;
            }
            let Some(weapon) = definition.content.weapons.iter().find(|w| w.id == id) else {
                continue;
            };
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
                    Transform::from_translation(Vec3::from_array(weapon.view_offset))
                        .with_rotation(Quat::from_rotation_y(std::f32::consts::FRAC_PI_2)),
                    Visibility::Hidden,
                    Name::new("Original first-person rig"),
                ))
                .id();
            commands.entity(parent).add_child(root);
            let knife = (weapon.throwable.is_none() && weapon.mine.is_none())
                .then(|| {
                    definition
                        .content
                        .weapons
                        .iter()
                        .find(|w| w.id.0 == "weapon.knife_zm")
                        .and_then(|w| w.presentation.as_ref())
                        .and_then(|id| {
                            definition
                                .content
                                .presentations
                                .iter()
                                .find(|p| p.id == *id)
                        })
                        .and_then(|p| p.scene.as_deref())
                })
                .flatten();
            for (is_arms, is_knife, scene) in [
                (true, false, Some(arms)),
                (false, false, Some(scene.as_str())),
                (false, true, knife),
            ] {
                let Some(scene) = scene else {
                    continue;
                };
                let source = server.load(scene.split('#').next().unwrap_or(scene).to_owned());
                let rig = commands
                    .spawn((
                        WorldAssetRoot(server.load(scene.to_owned())),
                        Rig {
                            owner,
                            weapon: weapon.id.clone(),
                            phase: None,
                            tracked: Vec::new(),
                            sampled_phase: None,
                            source,
                            graph: None,
                            nodes: BTreeMap::new(),
                            players: Vec::new(),
                            sequence: None,
                            arms: is_arms,
                            attached: is_arms,
                            ready: false,
                            knife: is_knife,
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
}
fn attach_weapons(
    mut commands: Commands,
    mut rigs: Query<(Entity, &mut Rig)>,
    children: Query<&Children>,
    names: Query<&Name>,
) {
    let sockets: Vec<_> = rigs
        .iter()
        .filter(|(_, rig)| rig.arms && rig.ready)
        .filter_map(|(entity, rig)| {
            let find = |tag: &str| {
                children
                    .iter_descendants(entity)
                    .find(|&child| names.get(child).is_ok_and(|name| name.as_str() == tag))
            };
            find("tag_weapon")
                .zip(find("tag_knife_attach"))
                .map(|(weapon, knife)| (rig.owner, rig.weapon.clone(), weapon, knife))
        })
        .collect();
    for (entity, mut rig) in &mut rigs {
        if rig.attached || !rig.ready {
            continue;
        }
        if let Some((_, _, weapon, knife)) = sockets
            .iter()
            .find(|(owner, id, _, _)| *owner == rig.owner && *id == rig.weapon)
        {
            let socket = if rig.knife { knife } else { weapon };
            // The hand socket is already in OAT's Y-up basis. Undo that basis on
            // the gun scene root before inheriting the authored socket pose.
            commands.entity(*socket).add_child(entity);
            commands.entity(entity).insert((
                Transform::from_rotation(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2)),
                if rig.knife {
                    Visibility::Hidden
                } else {
                    Visibility::Inherited
                },
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
    names: Query<&Name>,
) {
    let Ok(mut rig) = rigs.get_mut(event.entity) else {
        return;
    };
    rig.ready = true;
    for child in children.iter_descendants(event.entity) {
        if rig.arms
            && names
                .get(child)
                .is_ok_and(|name| name.as_str() == "tag_torso")
        {
            commands.entity(child).insert(AimingJoint {
                owner: rig.owner,
                weapon: rig.weapon.clone(),
                translation: Vec3::ZERO,
                rotation: Quat::IDENTITY,
            });
        }
        if names.contains(child) {
            rig.tracked.push((child, None));
        }
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
fn aim_pose(
    definition: Option<Res<GameDefinition>>,
    inventories: offhand::Inventories,
    mut joints: Query<(&mut AimingJoint, &mut Transform)>,
    mut cameras: Query<&mut Projection, With<ViewModelCamera>>,
) {
    let Some(definition) = definition else {
        return;
    };
    for (mut joint, mut pose) in &mut joints {
        let Ok((inventory, lethal, tactical, mines, drinking)) = inventories.get(joint.owner)
        else {
            continue;
        };
        let Some(weapon) = definition
            .content
            .weapons
            .iter()
            .find(|w| w.id == joint.weapon)
        else {
            continue;
        };
        let equipment = drinking
            .map(|d| d.selection())
            .or_else(|| offhand::selection(lethal, tactical, mines, &definition));
        let ads = if equipment.is_none()
            && inventory
                .active()
                .is_some_and(|w| w.definition == joint.weapon)
        {
            inventory.ads
        } else {
            0.
        };
        if !pose.is_changed() {
            pose.translation -= joint.translation;
            pose.rotation *= joint.rotation.inverse();
        }
        joint.translation = Vec3::from_array(weapon.mechanics.ads_translation) * ads;
        joint.rotation = Quat::IDENTITY.slerp(Quat::from_array(weapon.mechanics.ads_rotation), ads);
        pose.translation += joint.translation;
        pose.rotation *= joint.rotation;
        let visible = equipment
            .as_ref()
            .map(|s| &s.weapon)
            .or_else(|| inventory.active().map(|w| &w.definition));
        if visible != Some(&joint.weapon) {
            continue;
        }
        for mut projection in &mut cameras {
            if let Projection::Perspective(p) = &mut *projection {
                p.fov = (65. + (weapon.mechanics.ads_fov - 65.) * ads).to_radians();
            }
        }
    }
}
fn build_graphs(
    mut rigs: Query<&mut Rig>,
    assets: Res<Assets<Gltf>>,
    mut graphs: ResMut<Assets<AnimationGraph>>,
    mut cache: ResMut<offhand::GraphCache>,
) {
    for mut rig in &mut rigs {
        if rig.graph.is_some() {
            continue;
        }
        if let Some((graph, nodes)) = cache.0.get(&rig.source.id()) {
            rig.graph = Some(graph.clone());
            rig.nodes = nodes.clone();
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
        let graph = graphs.add(graph);
        cache
            .0
            .insert(rig.source.id(), (graph.clone(), rig.nodes.clone()));
        rig.graph = Some(graph);
    }
}
fn animate(
    mut commands: Commands,
    state: Res<State<GameState>>,
    definition: Option<Res<GameDefinition>>,
    inventories: offhand::Inventories,
    mut evidence: ResMut<EquipmentPresentationEvidence>,
    mut rigs: Query<(&mut Rig, &mut Visibility)>,
    clips: Res<Assets<AnimationClip>>,
    gltfs: Res<Assets<Gltf>>,
    mut players: Query<&mut AnimationPlayer>,
) {
    let Some(definition) = definition else {
        return;
    };
    for (mut rig, mut visibility) in &mut rigs {
        let Ok((inventory, lethal, tactical, mines, drinking)) = inventories.get(rig.owner) else {
            continue;
        };
        let selected = drinking
            .map(|d| d.selection())
            .or_else(|| offhand::selection(lethal, tactical, mines, &definition));
        let visible = selected
            .as_ref()
            .map(|s| &s.weapon)
            .or_else(|| inventory.active().map(|w| &w.definition));
        if visible != Some(&rig.weapon) {
            for entity in &rig.players {
                if let Ok(mut player) = players.get_mut(*entity) {
                    player.pause_all();
                }
            }
            rig.sequence = None;
            rig.phase = None;
            rig.sampled_phase = None;
            continue;
        }
        let Some(weapon) = definition
            .content
            .weapons
            .iter()
            .find(|w| w.id == rig.weapon)
        else {
            continue;
        };
        if rig.knife {
            *visibility = if inventory.signal == WeaponSignal::Melee && rig.attached {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            };
            continue;
        }
        let Some(graph) = rig.graph.clone() else {
            continue;
        };
        let action = selected.as_ref().map_or_else(
            || match inventory.signal {
                WeaponSignal::Equip => WeaponAnimation::Equip,
                WeaponSignal::Unequip => WeaponAnimation::Unequip,
                WeaponSignal::Fire => WeaponAnimation::Fire,
                WeaponSignal::Reload => WeaponAnimation::Reload,
                WeaponSignal::ReloadEmpty => WeaponAnimation::ReloadEmpty,
                WeaponSignal::ShellInsert => WeaponAnimation::Reload,
                WeaponSignal::ReloadStart => WeaponAnimation::ReloadStart,
                WeaponSignal::ReloadEnd => WeaponAnimation::ReloadEnd,
                WeaponSignal::Melee => WeaponAnimation::Melee,
                _ => WeaponAnimation::Idle,
            },
            |s| s.action,
        );
        let animation_weapon = if action == WeaponAnimation::Melee && rig.arms {
            definition
                .content
                .weapons
                .iter()
                .find(|w| w.id.0 == "weapon.knife_zm")
                .unwrap_or(weapon)
        } else {
            weapon
        };
        let clip = animation_weapon
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
            if rig.sequence != Some(inventory.sequence)
                || rig.phase != selected.as_ref().map(|s| s.action)
            {
                commands
                    .entity(entity)
                    .insert(AnimationGraphHandle(graph.clone()));
                player.stop_all();
                let active = player.play(node);
                if action == WeaponAnimation::Idle {
                    active.repeat();
                } else if action == WeaponAnimation::Melee && rig.arms {
                    let duration = animation_weapon.mechanics.melee_seconds[1]
                        .max(animation_weapon.mechanics.melee_seconds[0]);
                    if let Some(clip) = name
                        .and_then(|name| gltfs.get(&rig.source)?.named_animations.get(name))
                        .and_then(|handle| clips.get(handle))
                        && duration > 0.
                    {
                        active.set_speed(clip.duration() / duration);
                    }
                }
                if selected.is_some() {
                    evidence.phases.insert(format!(
                        "{}:{action:?}:{}",
                        weapon.id.0,
                        if rig.arms { "arms" } else { "model" }
                    ));
                }
                // Action timers remain authoritative; pause preserves the current pose.
            }
            if let Some((duration, remaining)) = selected
                .as_ref()
                .and_then(|s| s.timing)
                .or_else(|| inventory.reload_timing(weapon))
                && let Some(clip) = name
                    .and_then(|name| gltfs.get(&rig.source)?.named_animations.get(name))
                    .and_then(|h| clips.get(h))
                && let Some(active) = player.animation_mut(node)
            {
                active.set_speed(0.);
                active.set_seek_time(
                    ((duration - remaining).max(0.) / duration * clip.duration())
                        .min(clip.duration()),
                );
            }
            if **state == GameState::Playing {
                player.resume_all();
            } else {
                player.pause_all();
            }
        }
        rig.sequence = Some(inventory.sequence);
        rig.phase = selected.as_ref().map(|s| s.action);
    }
}

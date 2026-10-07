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
// Shared camera-space calibration against the supplied Mauser hip-fire view.
// Weapon-specific grips and offsets remain authored by their original rigs.
const VIEWMODEL_FOV_DEGREES: f32 = 65.;
const VIEWMODEL_ORIGIN: Vec3 = Vec3::new(0., 0.025, 0.);
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
struct RigRestPose(Transform);
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
    playback: Option<WeaponAnimation>,
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
                clear_aim_pose.before(bevy::app::AnimationSystems),
            )
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
                    fov: VIEWMODEL_FOV_DEGREES.to_radians(),
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
                        Transform::from_translation(VIEWMODEL_ORIGIN),
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
                            playback: None,
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
    poses: Query<&Transform>,
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
            if let Ok(pose) = poses.get(child) {
                commands.entity(child).insert(RigRestPose(*pose));
            }
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
// Sparse clips omit channels that stay at their authored baseline. Bevy leaves
// those channels untouched, so a melee lunge otherwise survives into idle.
// Reset before sampling, then apply ADS once afterwards. Pausing retains the
// sampled native pose and only removes/reapplies our previous ADS modifier.
fn clear_aim_pose(
    state: Res<State<GameState>>,
    mut bones: Query<(&mut Transform, &RigRestPose, Option<&mut AimingJoint>)>,
) {
    for (mut pose, rest, joint) in &mut bones {
        if **state == GameState::Playing {
            *pose = rest.0;
        } else if let Some(joint) = joint.as_ref() {
            pose.translation -= joint.translation;
            pose.rotation *= joint.rotation.inverse();
        }
        if let Some(mut joint) = joint {
            joint.translation = Vec3::ZERO;
            joint.rotation = Quat::IDENTITY;
        }
    }
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
                // ADS changes the world lens; native joint offsets align the
                // sights without also magnifying the hands in the overlay lens.
                p.fov = VIEWMODEL_FOV_DEGREES.to_radians();
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
            rig.playback = None;
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
        let mut action = selected.as_ref().map_or_else(
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
        // fireTime gates the next shot, not the lifetime of a single-shot pose.
        // Keep sampling the native clip tail until a new action interrupts it.
        if selected.is_none() && action == WeaponAnimation::Idle {
            let fire_duration = weapon
                .animation_actions
                .get(&WeaponAnimation::Fire)
                .and_then(|id| id.0.rsplit('.').next())
                .and_then(|name| gltfs.get(&rig.source)?.named_animations.get(name))
                .and_then(|handle| clips.get(handle))
                .map(|clip| clip.duration());
            if inventory
                .shot_elapsed()
                .zip(fire_duration)
                .is_some_and(|(elapsed, duration)| elapsed < duration)
            {
                action = WeaponAnimation::Fire;
            }
        }
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
        // A primed throwable holds the last pull-pin pose. Its native idle is
        // the unprimed pose; replaying it resets the hands and replaces the pin.
        let holding_throwable =
            selected.is_some() && action == WeaponAnimation::Idle && weapon.throwable.is_some();
        let clip_action = if holding_throwable {
            WeaponAnimation::Prime
        } else {
            action
        };
        let clip = animation_weapon
            .animation_actions
            .get(&clip_action)
            .or_else(|| weapon.animation_actions.get(&WeaponAnimation::Idle));
        let name = clip.and_then(|id| id.0.rsplit('.').next());
        let Some(node) = name.and_then(|name| rig.nodes.get(name)).copied() else {
            continue;
        };
        if rig.players.is_empty() {
            continue;
        }
        let native_clip = name
            .and_then(|name| gltfs.get(&rig.source)?.named_animations.get(name))
            .and_then(|handle| clips.get(handle));
        let melee = definition
            .content
            .weapons
            .iter()
            .find(|w| w.id.0 == "weapon.knife_zm")
            .unwrap_or(weapon);
        for &entity in &rig.players {
            let Ok(mut player) = players.get_mut(entity) else {
                continue;
            };
            let finishing_shot = selected.is_none()
                && action == WeaponAnimation::Fire
                && inventory.signal == WeaponSignal::Idle;
            if (rig.sequence != Some(inventory.sequence) && !finishing_shot)
                || rig.playback != Some(action)
                || rig.phase != selected.as_ref().map(|s| s.action)
            {
                commands
                    .entity(entity)
                    .insert(AnimationGraphHandle(graph.clone()));
                player.stop_all();
                let active = player.play(node);
                if action == WeaponAnimation::Idle {
                    active.repeat();
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
            let timing = holding_throwable
                .then_some((1., 0.))
                .or_else(|| selected.as_ref().and_then(|s| s.timing))
                .or_else(|| inventory.presentation_timing(weapon, melee));
            let seek = native_clip.and_then(|clip| {
                if let Some((duration, remaining)) = timing {
                    Some(
                        ((duration - remaining).max(0.) / duration * clip.duration())
                            .min(clip.duration()),
                    )
                } else if selected.is_none() && action == WeaponAnimation::Fire {
                    inventory.shot_elapsed().map(|age| age.min(clip.duration()))
                } else {
                    None
                }
            });
            if let Some(seek) = seek
                && let Some(active) = player.animation_mut(node)
            {
                active.set_speed(0.);
                active.set_seek_time(seek);
                let key = format!(
                    "{}:{action:?}:{}",
                    rig.weapon.0,
                    if rig.arms { "arms" } else { "model" }
                );
                let clock = evidence.clocks.entry(key).or_default();
                clock[0] = native_clip.map_or(0., |clip| clip.duration());
                clock[1] = clock[1].max(seek);
                clock[2] += 1.;
            }
            // Bevy wraps seek_time == duration even at speed zero. Paused
            // players still evaluate their curves, without advancing/wrapping
            // the gameplay-owned clock or losing its terminal held pose.
            if **state == GameState::Playing && seek.is_none() {
                player.resume_all();
            } else {
                player.pause_all();
            }
        }
        rig.sequence = Some(inventory.sequence);
        rig.phase = selected.as_ref().map(|s| s.action);
        rig.playback = Some(action);
    }
}

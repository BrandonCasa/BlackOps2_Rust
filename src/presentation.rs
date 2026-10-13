use crate::{
    GameState,
    animation::{AnimationGraphs, AnimationLayer, AnimationPlayback},
    assets::GameDefinition,
    gameplay::SessionEntity,
    hud::PlayerOverlay,
    origins::content::{WeaponAnimation, WeaponId},
    player::Player,
    rendering::GameCamera,
    weapons::{WeaponInventory, WeaponSignal},
};
use bevy::{
    camera::visibility::{NoFrustumCulling, RenderLayers},
    prelude::*,
    world_serialization::WorldInstanceReady,
};
use std::collections::BTreeMap;

pub(crate) mod camo;
pub(crate) mod locomotion;
pub(crate) mod offhand;
pub use offhand::EquipmentPresentationEvidence;
pub(crate) use offhand::selection as equipment_selection;
pub struct PresentationPlugin;
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct PresentationAnimation;
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct CameraPose;
const VIEWMODEL_ORIGIN: Vec3 = Vec3::new(0., 0.025, 0.);
#[derive(Component)]
struct AimingJoint {
    owner: Entity,
    weapon: WeaponId,
    translation: Vec3,
    rotation: Quat,
}
#[derive(Component)]
struct SocketAttachment(Entity);
#[derive(Component)]
struct WeaponAttachment {
    rig: Entity,
    selected: String,
}
#[derive(Component)]
struct AttachmentHidden;
#[derive(Component)]
struct FirstPerson {
    owner: Entity,
    weapon: WeaponId,
}
#[derive(Component)]
pub(crate) struct ViewModelGroup {
    owner: Entity,
}
#[derive(Component, Clone)]
pub(crate) struct Rig {
    pub(crate) owner: Entity,
    pub(crate) weapon: WeaponId,
    phase: Option<WeaponAnimation>,
    tracked: Vec<(Entity, Option<(Vec3, Quat)>)>,
    sampled_phase: Option<WeaponAnimation>,
    source: Handle<Gltf>,
    graph: Option<Handle<AnimationGraph>>,
    nodes: BTreeMap<String, AnimationNodeIndex>,
    pub(crate) players: Vec<Entity>,
    sequence: Option<u64>,
    pub(crate) arms: bool,
    pub(crate) attached: bool,
    pub(crate) ready: bool,
    pub(crate) knife: bool,
    ads: bool,
    pub(crate) left: bool,
    dual: bool,
    playback: Option<WeaponAnimation>,
    sprinting: bool,
    sprint_phase: Option<(WeaponAnimation, f32)>,
    playback_revision: u64,
}
impl Plugin for PresentationPlugin {
    fn build(&self, app: &mut App) {
        camo::install(app);
        locomotion::install(app);
        app.init_resource::<EquipmentPresentationEvidence>()
            .add_systems(
                Update,
                (
                    spawn_viewmodels.run_if(crate::flythrough::normal_play),
                    build_graphs,
                    attach_weapons,
                    attachment_visuals,
                    offhand::select_visible,
                    animate,
                )
                    .chain()
                    .in_set(PresentationAnimation),
            )
            .add_systems(
                PostUpdate,
                clear_aim_pose.before(bevy::app::AnimationSystems),
            )
            .add_systems(
                PostUpdate,
                aim_pose
                    .in_set(crate::animation::AnimationPoseModifiers)
                    .after(bevy::app::AnimationSystems)
                    .before(bevy::camera::CameraUpdateSystems)
                    .before(bevy::transform::TransformSystems::Propagate),
            )
            .add_systems(
                PostUpdate,
                hide_attachment_joints
                    .in_set(crate::animation::AnimationPoseModifiers)
                    .after(bevy::app::AnimationSystems)
                    .before(bevy::transform::TransformSystems::Propagate),
            )
            .add_systems(
                PostUpdate,
                camera_pose
                    .in_set(crate::animation::AnimationPoseModifiers)
                    .run_if(crate::flythrough::normal_play)
                    .in_set(CameraPose)
                    .after(aim_pose)
                    .before(bevy::transform::TransformSystems::Propagate),
            )
            .add_systems(
                PostUpdate,
                offhand::record_motion
                    .after(camera_pose)
                    .before(bevy::transform::TransformSystems::Propagate),
            )
            .add_systems(
                PostUpdate,
                follow_sockets
                    .in_set(crate::animation::AnimationPoseModifiers)
                    .after(camera_pose)
                    .before(bevy::transform::TransformSystems::Propagate),
            )
            .add_systems(Last, offhand::record_visibility)
            .add_observer(rig_ready);
        app.add_observer(attachment_ready);
    }
}
fn spawn_viewmodels(
    mut commands: Commands,
    session: Res<crate::gameplay::Session>,
    server: Res<AssetServer>,
    definition: Option<Res<GameDefinition>>,
    state: Res<State<GameState>>,
    owners: offhand::Owners,
    cameras: Query<Entity, With<GameCamera>>,
    existing: Query<(Entity, &FirstPerson)>,
    groups: Query<(Entity, &ViewModelGroup)>,
) {
    if !matches!(**state, GameState::PreparingShaders | GameState::Playing) {
        return;
    }
    let (Some(definition), Ok(camera)) = (definition, cameras.single()) else {
        return;
    };
    for (owner, inventory, lethal, tactical, mines, drinking, buildables) in &owners {
        let mut wanted = offhand::owned(inventory, lethal, tactical, mines);
        if buildables.is_some_and(|e| e.shield_health > 0) {
            wanted.push(WeaponId("weapon.tomb_shield_zm".into()));
        }
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
            let character = ["dempsey", "nikolai", "richtofen", "takeo"][session.character];
            let arms =
                crate::assets::native_scene(&definition, &format!("c_zom_{character}_viewhands"));
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
            let ads_scene = weapon
                .ads_presentation
                .as_ref()
                .and_then(|id| {
                    definition
                        .content
                        .presentations
                        .iter()
                        .find(|p| p.id == *id)
                })
                .and_then(|p| p.scene.as_deref());
            let left_scene = weapon
                .dual_weapon
                .as_ref()
                .and_then(|id| definition.content.weapons.iter().find(|w| w.id == *id))
                .and_then(|w| w.presentation.as_ref())
                .and_then(|id| {
                    definition
                        .content
                        .presentations
                        .iter()
                        .find(|p| p.id == *id)
                })
                .and_then(|p| p.scene.as_deref());
            for (is_arms, is_knife, is_ads, is_left, scene) in [
                (true, false, false, false, arms.map(String::as_str)),
                (false, false, false, false, Some(scene.as_str())),
                (false, true, false, false, knife),
                (false, false, true, false, ads_scene),
                (false, false, false, true, left_scene),
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
                            ads: is_ads,
                            left: is_left,
                            dual: weapon.dual_weapon.is_some(),
                            playback: None,
                            sprinting: false,
                            sprint_phase: None,
                            playback_revision: 0,
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
    bindings: Query<&SocketAttachment>,
) {
    let sockets: Vec<_> = rigs
        .iter()
        .filter(|(_, rig)| rig.arms && !rig.left && rig.ready)
        .filter_map(|(entity, rig)| {
            let find = |tag: &str| {
                children
                    .iter_descendants(entity)
                    .find(|&child| names.get(child).is_ok_and(|name| name.as_str() == tag))
            };
            find("tag_weapon")
                .zip(find("tag_knife_attach"))
                .map(|(weapon, knife)| {
                    (
                        rig.owner,
                        rig.weapon.clone(),
                        weapon,
                        knife,
                        find("tag_weapon1"),
                    )
                })
        })
        .collect();
    for (entity, mut rig) in &mut rigs {
        if rig.arms || !rig.ready {
            continue;
        }
        if let Some((_, _, weapon, knife, left)) = sockets
            .iter()
            .find(|(owner, id, _, _, _)| *owner == rig.owner && *id == rig.weapon)
        {
            let socket = if rig.knife {
                knife
            } else if rig.left {
                let Some(left) = left else { continue };
                left
            } else {
                weapon
            };
            if bindings
                .get(entity)
                .is_ok_and(|binding| binding.0 == *socket)
            {
                continue;
            }
            commands.entity(entity).insert((
                SocketAttachment(*socket),
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
fn attachment_visuals(
    mut commands: Commands,
    definition: Res<GameDefinition>,
    server: Res<AssetServer>,
    inventories: Query<&WeaponInventory>,
    rigs: Query<(Entity, &Rig)>,
    existing: Query<(Entity, &WeaponAttachment)>,
    children: Query<&Children>,
    names: Query<&Name>,
    hidden: Query<(), With<AttachmentHidden>>,
) {
    for (entity, rig) in &rigs {
        if rig.arms || rig.knife || rig.ads || rig.left || !rig.ready {
            continue;
        }
        let Ok(inventory) = inventories.get(rig.owner) else {
            continue;
        };
        let Some(weapon) = definition
            .content
            .weapons
            .iter()
            .find(|w| w.id == rig.weapon)
        else {
            continue;
        };
        let selected = if inventory
            .active()
            .is_some_and(|w| w.definition == rig.weapon)
        {
            inventory.attachment()
        } else {
            "defaultattachment"
        };
        if existing
            .iter()
            .any(|(_, a)| a.rig == entity && a.selected == selected)
        {
            continue;
        }
        for (attachment, _) in existing.iter().filter(|(_, a)| a.rig == entity) {
            commands.entity(attachment).despawn();
        }
        let attachment = weapon.attachments.get(selected);
        for child in children.iter_descendants(entity) {
            if hidden.contains(child) {
                commands
                    .entity(child)
                    .remove::<AttachmentHidden>()
                    .insert(Visibility::Inherited);
                commands
                    .entity(child)
                    .entry::<Transform>()
                    .and_modify(|mut pose| pose.scale = Vec3::ONE);
            }
            if attachment.is_some_and(|a| {
                names
                    .get(child)
                    .is_ok_and(|n| a.hide_tags.iter().any(|tag| tag == n.as_str()))
            }) {
                commands
                    .entity(child)
                    .insert((AttachmentHidden, Visibility::Hidden));
            }
        }
        let mut holder = commands.spawn((
            WeaponAttachment {
                rig: entity,
                selected: selected.into(),
            },
            Transform::default(),
            Visibility::Inherited,
        ));
        if let Some(a) = attachment
            && let Some(scene) = a
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
        {
            holder.insert((
                WorldAssetRoot(server.load(scene.clone())),
                Transform::from_translation(Vec3::new(a.offset[0], a.offset[2], -a.offset[1]))
                    .with_rotation(Quat::from_euler(
                        EulerRot::YXZ,
                        -a.rotation[1],
                        a.rotation[0],
                        -a.rotation[2],
                    )),
            ));
        }
        let attached = holder.id();
        commands.entity(entity).add_child(attached);
    }
}
fn attachment_ready(
    event: On<WorldInstanceReady>,
    mut commands: Commands,
    attachments: Query<(), With<WeaponAttachment>>,
    children: Query<&Children>,
    meshes: Query<(), With<Mesh3d>>,
) {
    if !attachments.contains(event.entity) {
        return;
    }
    for child in children.iter_descendants(event.entity) {
        commands
            .entity(child)
            .insert(avian3d::prelude::CollisionLayers::new(
                avian3d::prelude::LayerMask::NONE,
                avian3d::prelude::LayerMask::NONE,
            ));
        if meshes.contains(child) {
            commands
                .entity(child)
                .insert((RenderLayers::layer(1), crate::rendering::FirstPersonSurface));
        }
    }
}
fn hide_attachment_joints(mut joints: Query<&mut Transform, With<AttachmentHidden>>) {
    for mut joint in &mut joints {
        joint.scale = Vec3::ZERO;
    }
}
fn follow_sockets(
    mut poses: ParamSet<(
        Query<(&Transform, Option<&ChildOf>)>,
        Query<(&SocketAttachment, &ChildOf, &mut Transform)>,
    )>,
) {
    let attachments: Vec<_> = poses
        .p1()
        .iter()
        .map(|(socket, parent, _)| (socket.0, parent.parent()))
        .collect();
    let transforms = poses.p0();
    let followed: Vec<_> = attachments
        .into_iter()
        .map(|(socket, parent)| {
            let mut cursor = socket;
            let mut local = Mat4::IDENTITY;
            while cursor != parent {
                let Ok((pose, Some(ancestor))) = transforms.get(cursor) else {
                    return None;
                };
                local = pose.to_matrix() * local;
                cursor = ancestor.parent();
            }
            Some(
                Transform::from_matrix(local).mul_transform(Transform::from_rotation(
                    Quat::from_rotation_x(std::f32::consts::FRAC_PI_2),
                )),
            )
        })
        .collect();
    for ((_, _, mut pose), followed) in poses.p1().iter_mut().zip(followed) {
        if let Some(followed) = followed {
            pose.set_if_neq(followed);
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
    rig.tracked.clear();
    rig.players.clear();
    rig.sequence = None;
    rig.sampled_phase = None;
    commands
        .entity(event.entity)
        .insert(avian3d::prelude::CollisionLayers::new(
            avian3d::prelude::LayerMask::NONE,
            avian3d::prelude::LayerMask::NONE,
        ));
    for child in children.iter_descendants(event.entity) {
        commands
            .entity(child)
            .insert(avian3d::prelude::CollisionLayers::new(
                avian3d::prelude::LayerMask::NONE,
                avian3d::prelude::LayerMask::NONE,
            ));
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
            commands
                .entity(child)
                .insert((RenderLayers::layer(1), crate::rendering::FirstPersonSurface))
                .remove::<NoFrustumCulling>();
        }
    }
    if rig.arms && rig.dual && !rig.left {
        let left_player = commands.spawn(AnimationPlayer::default()).id();
        let mut left_rig = rig.clone();
        left_rig.left = true;
        left_rig.players = vec![left_player];
        left_rig.tracked.clear();
        commands.entity(left_player).insert((
            left_rig,
            Transform::default(),
            Visibility::Inherited,
        ));
        commands.entity(event.entity).add_child(left_player);
        for child in children.iter_descendants(event.entity) {
            if names
                .get(child)
                .is_ok_and(|n| matches!(n.as_str(), "j_shoulder_le" | "tag_weapon1"))
            {
                commands
                    .entity(child)
                    .insert(bevy::animation::AnimatedBy(left_player));
                for bone in children.iter_descendants(child) {
                    commands
                        .entity(bone)
                        .insert(bevy::animation::AnimatedBy(left_player));
                }
            }
        }
    }
    info!(
        "First-person scene ready: {} animation players",
        rig.players.len()
    );
}
fn clear_aim_pose(mut joints: Query<&mut AimingJoint>) {
    for mut joint in &mut joints {
        joint.translation = Vec3::ZERO;
        joint.rotation = Quat::IDENTITY;
    }
}
fn aim_pose(
    definition: Option<Res<GameDefinition>>,
    inventories: offhand::Inventories,
    mut joints: Query<(&mut AimingJoint, &mut Transform)>,
) {
    let Some(definition) = definition else {
        return;
    };
    for (mut joint, mut pose) in &mut joints {
        let Ok((inventory, lethal, tactical, mines, drinking, buildables)) =
            inventories.get(joint.owner)
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
            .or_else(|| offhand::buildable_selection(buildables))
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
        if let Some(pair) = weapon
            .mechanics
            .ads_pose
            .windows(2)
            .find(|p| ads <= p[1].fraction)
        {
            let t =
                ((ads - pair[0].fraction) / (pair[1].fraction - pair[0].fraction)).clamp(0., 1.);
            joint.translation = Vec3::from_array(pair[0].translation)
                .lerp(Vec3::from_array(pair[1].translation), t);
            joint.rotation =
                Quat::from_array(pair[0].rotation).slerp(Quat::from_array(pair[1].rotation), t);
        } else {
            joint.translation = Vec3::from_array(weapon.mechanics.ads_translation) * ads;
            joint.rotation =
                Quat::IDENTITY.slerp(Quat::from_array(weapon.mechanics.ads_rotation), ads);
        }
        pose.translation += joint.translation;
        pose.rotation *= joint.rotation;
    }
}
fn camera_pose(
    state: Res<State<GameState>>,
    rigs: Query<(Entity, &Rig)>,
    names: Query<(Entity, &Name)>,
    inventories: offhand::Inventories,
    definition: Option<Res<GameDefinition>>,
    mut poses: ParamSet<(
        Query<(&Transform, Option<&ChildOf>)>,
        Query<(&mut Transform, Option<&ViewModelGroup>, Option<&GameCamera>)>,
    )>,
) {
    if !matches!(
        **state,
        GameState::PreparingShaders | GameState::Playing | GameState::Paused | GameState::GameOver
    ) {
        return;
    }
    let Some(definition) = definition else {
        return;
    };
    let mut eyes = Vec::new();
    for (root, rig) in &rigs {
        if !rig.arms || rig.knife || !rig.ready {
            continue;
        }
        let Ok((inventory, lethal, tactical, mines, drinking, buildables)) =
            inventories.get(rig.owner)
        else {
            continue;
        };
        let selected = drinking
            .map(|d| d.selection())
            .or_else(|| offhand::buildable_selection(buildables))
            .or_else(|| offhand::selection(lethal, tactical, mines, &definition));
        let visible = selected
            .as_ref()
            .map(|s| &s.weapon)
            .or_else(|| inventory.active().map(|w| &w.definition));
        if visible != Some(&rig.weapon) {
            continue;
        }
        for (entity, name) in &names {
            if name.as_str() != "tag_camera" {
                continue;
            }
            let mut cursor = entity;
            let mut local = Mat4::IDENTITY;
            let transforms = poses.p0();
            while cursor != root {
                if rigs.contains(cursor) {
                    break;
                }
                let Ok((pose, Some(parent))) = transforms.get(cursor) else {
                    break;
                };
                local = pose.to_matrix() * local;
                cursor = parent.parent();
            }
            if cursor != root {
                continue;
            }
            let basis = Quat::from_rotation_y(std::f32::consts::FRAC_PI_2);
            let (_, rotation, translation) = local.to_scale_rotation_translation();
            let eye = Transform {
                translation: basis * translation,
                rotation: basis
                    * rotation
                    * Quat::from_rotation_x(std::f32::consts::FRAC_PI_2)
                    * basis.inverse(),
                ..default()
            };
            eyes.push((
                rig.owner,
                eye,
                if selected.is_none() {
                    inventory.ads
                } else {
                    0.
                },
                definition
                    .content
                    .weapons
                    .iter()
                    .find(|w| w.id == rig.weapon)
                    .and_then(|w| inventory.effective_ads_sight(w))
                    .map_or(Quat::IDENTITY, |s| Quat::from_array(s.rotation)),
            ));
        }
    }
    for (owner, eye, ads, calibration) in eyes {
        let inverse = Transform::from_matrix(eye.to_matrix().inverse());
        for (mut pose, group, camera) in &mut poses.p1() {
            if camera.is_some() {
                *pose = pose.mul_transform(eye);
            } else if group.is_some_and(|g| g.owner == owner) {
                *pose = inverse
                    .mul_transform(Transform::from_rotation(
                        Quat::IDENTITY.slerp(calibration, ads),
                    ))
                    .mul_transform(Transform::from_translation(VIEWMODEL_ORIGIN * (1. - ads)));
            }
        }
    }
}
fn build_graphs(
    mut rigs: Query<&mut Rig>,
    assets: Res<Assets<Gltf>>,
    mut graphs: ResMut<Assets<AnimationGraph>>,
    mut cache: ResMut<AnimationGraphs>,
) {
    for mut rig in &mut rigs {
        if rig.graph.is_some() {
            continue;
        }
        let Some(bindings) = cache.scene(&rig.source, &assets, &mut graphs) else {
            continue;
        };
        rig.nodes = bindings.named.clone();
        rig.graph = Some(bindings.graph.clone());
    }
}
#[cfg(debug_assertions)]
pub(crate) fn animation_observation(world: &mut World) -> Vec<serde_json::Value> {
    let rigs: Vec<_> = world.query::<&Rig>().iter(world).cloned().collect();
    rigs.iter().filter(|r| r.playback.is_some() && !r.knife).map(|rig| {
        let clocks: Vec<_> = rig.players.iter().filter_map(|e| world.get::<AnimationPlayer>(*e))
            .flat_map(|p| p.playing_animations().map(|(node, active)| {
                serde_json::json!({"clip":rig.nodes.iter().find(|(_,n)| **n == *node).map(|(name,_)|name),
                    "seconds":active.seek_time(),"paused":active.is_paused()})
            })).collect();
        let bones: BTreeMap<_,_> = rig.tracked.iter().filter_map(|(e,_)| {
            let name = world.get::<Name>(*e)?;
            let pose = world.get::<Transform>(*e)?;
            Some((name.as_str().to_owned(), serde_json::json!({"translation":pose.translation.to_array(),"rotation":pose.rotation.to_array()})))
        }).collect();
        serde_json::json!({"weapon":rig.weapon.0,"arms":rig.arms,"left":rig.left,"ready":rig.ready && (rig.arms || rig.attached),
            "action":rig.playback.map(|a|format!("{a:?}")),"clocks":clocks,"bones":bones,
            "locomotion":world.get::<locomotion::WeaponLocomotion>(rig.owner).map(|state|state.observation())})
    }).collect()
}
pub(crate) fn hand_action(
    inventory: &WeaponInventory,
    weapon: &crate::origins::content::WeaponDefinition,
    game: &GameDefinition,
    left: bool,
) -> (WeaponAnimation, Option<f32>) {
    let signal = inventory.hand_signal(left);
    let empty = inventory.active().is_some_and(|w| {
        if left {
            w.left_magazine == Some(0)
        } else {
            w.magazine == 0
        }
    });
    let idle = if empty
        && weapon
            .animation_actions
            .contains_key(&WeaponAnimation::IdleEmpty)
    {
        WeaponAnimation::IdleEmpty
    } else {
        WeaponAnimation::Idle
    };
    let aimed = inventory.ads >= 0.99;
    let fire = if empty
        && aimed
        && weapon
            .animation_actions
            .contains_key(&WeaponAnimation::AdsLastShot)
    {
        WeaponAnimation::AdsLastShot
    } else if empty
        && weapon
            .animation_actions
            .contains_key(&WeaponAnimation::LastShot)
    {
        WeaponAnimation::LastShot
    } else if aimed
        && weapon
            .animation_actions
            .contains_key(&WeaponAnimation::AdsFire)
    {
        WeaponAnimation::AdsFire
    } else {
        WeaponAnimation::Fire
    };
    let action = match signal {
        WeaponSignal::Prime => WeaponAnimation::Prime,
        WeaponSignal::Equip => WeaponAnimation::Equip,
        WeaponSignal::Unequip => WeaponAnimation::Unequip,
        WeaponSignal::Fire => fire,
        WeaponSignal::Reload => WeaponAnimation::Reload,
        WeaponSignal::ReloadEmpty => WeaponAnimation::ReloadEmpty,
        WeaponSignal::ShellInsert => WeaponAnimation::Reload,
        WeaponSignal::ReloadStart => WeaponAnimation::ReloadStart,
        WeaponSignal::ReloadEnd => WeaponAnimation::ReloadEnd,
        WeaponSignal::Melee => WeaponAnimation::Melee,
        _ => idle,
    };
    if matches!(signal, WeaponSignal::Fire | WeaponSignal::Idle)
        && let Some(age) = inventory.hand_shot_elapsed(left)
    {
        let duration = |action| {
            weapon
                .animation_actions
                .get(&action)
                .and_then(|id| game.content.animations.iter().find(|a| a.id == *id))
                .and_then(|a| a.notifications.last())
                .map_or(0., |n| n.seconds)
        };
        let fire_duration = duration(fire);
        if age < fire_duration {
            return (fire, Some(age));
        }
        let rechamber = if aimed
            && weapon
                .animation_actions
                .contains_key(&WeaponAnimation::AdsRechamber)
        {
            WeaponAnimation::AdsRechamber
        } else {
            WeaponAnimation::Rechamber
        };
        if matches!(fire, WeaponAnimation::Fire | WeaponAnimation::AdsFire)
            && age - fire_duration < duration(rechamber)
        {
            return (rechamber, Some(age - fire_duration));
        }
        return (idle, None);
    }
    (action, None)
}
fn animate(
    mut commands: Commands,
    time: Res<Time>,
    state: Res<State<GameState>>,
    definition: Option<Res<GameDefinition>>,
    inventories: offhand::Inventories,
    sprints: Query<&crate::movement::SprintState>,
    locomotions: Query<&locomotion::WeaponLocomotion>,
    mut evidence: ResMut<EquipmentPresentationEvidence>,
    mut rigs: Query<(&mut Rig, &mut Visibility)>,
    clips: Res<Assets<AnimationClip>>,
    gltfs: Res<Assets<Gltf>>,
    players: Query<Option<&AnimationPlayback>, With<AnimationPlayer>>,
) {
    let Some(definition) = definition else {
        return;
    };
    for (mut rig, mut visibility) in &mut rigs {
        let Ok((inventory, lethal, tactical, mines, drinking, buildables)) =
            inventories.get(rig.owner)
        else {
            continue;
        };
        let selected = drinking
            .map(|d| d.selection())
            .or_else(|| offhand::buildable_selection(buildables))
            .or_else(|| offhand::selection(lethal, tactical, mines, &definition));
        let visible = selected
            .as_ref()
            .map(|s| &s.weapon)
            .or_else(|| inventory.active().map(|w| &w.definition));
        if visible != Some(&rig.weapon) {
            for entity in &rig.players {
                if let Ok(Some(playback)) = players.get(*entity) {
                    let mut playback = playback.clone();
                    playback.enabled = false;
                    commands.entity(*entity).insert(playback);
                }
            }
            rig.sequence = None;
            rig.phase = None;
            rig.playback = None;
            rig.sampled_phase = None;
            rig.sprinting = false;
            rig.sprint_phase = None;
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
        let weapon = if rig.left {
            weapon
                .dual_weapon
                .as_ref()
                .and_then(|id| definition.content.weapons.iter().find(|w| w.id == *id))
                .unwrap_or(weapon)
        } else {
            weapon
        };
        let sequence = inventory.hand_sequence(rig.left);
        let signal = inventory.hand_signal(rig.left);
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
        let (mut action, mut shot_seek) = selected.as_ref().map_or_else(
            || hand_action(inventory, weapon, &definition, rig.left),
            |s| (s.action, None),
        );
        let locomotion = (selected.is_none()
            && matches!(action, WeaponAnimation::Idle | WeaponAnimation::IdleEmpty))
        .then(|| {
            let empty = inventory.active().is_some_and(|w| {
                if rig.left {
                    w.left_magazine == Some(0)
                } else {
                    w.magazine == 0
                }
            });
            locomotions.get(rig.owner).ok()?.sample(weapon, empty)
        })
        .flatten();
        if let Some((phase, _, _, _)) = locomotion {
            action = phase;
            shot_seek = None;
        }
        let sprint_duration = |action, index: usize| {
            let name = weapon
                .animation_actions
                .get(&action)?
                .0
                .rsplit('.')
                .next()?;
            rig.nodes.get(name)?;
            let handle = gltfs.get(&rig.source)?.named_animations.get(name)?;
            let duration = clips.get(handle)?.duration();
            (duration > 0.).then_some(if weapon.mechanics.sprint_seconds[index] > 0. {
                weapon.mechanics.sprint_seconds[index]
            } else {
                duration
            })
        };
        let enter = sprint_duration(WeaponAnimation::SprintEnter, 0);
        let looping = sprint_duration(WeaponAnimation::SprintLoop, 1);
        let leave = sprint_duration(WeaponAnimation::SprintLeave, 2);
        let sprinting = sprints.get(rig.owner).is_ok_and(|s| s.sprinting);
        let idle = selected.is_none()
            && matches!(action, WeaponAnimation::Idle | WeaponAnimation::IdleEmpty);
        let sprint_fraction = if !idle {
            rig.sprinting = false;
            rig.sprint_phase = None;
            None
        } else {
            if sprinting != rig.sprinting {
                rig.sprint_phase = if sprinting {
                    enter
                        .map(|_| (WeaponAnimation::SprintEnter, 0.))
                        .or_else(|| looping.map(|_| (WeaponAnimation::SprintLoop, 0.)))
                } else {
                    leave.map(|_| (WeaponAnimation::SprintLeave, 0.))
                };
                rig.sprinting = sprinting;
            } else if let Some((phase, elapsed)) = rig.sprint_phase {
                let elapsed = elapsed
                    + if **state == GameState::Playing {
                        time.delta_secs()
                    } else {
                        0.
                    };
                rig.sprint_phase = match phase {
                    WeaponAnimation::SprintEnter if enter.is_none_or(|d| elapsed >= d) => looping
                        .map(|duration| {
                            (
                                WeaponAnimation::SprintLoop,
                                (elapsed - enter.unwrap_or(0.)).rem_euclid(duration),
                            )
                        }),
                    WeaponAnimation::SprintLoop => looping.map(|duration| {
                        (WeaponAnimation::SprintLoop, elapsed.rem_euclid(duration))
                    }),
                    WeaponAnimation::SprintLeave if leave.is_none_or(|d| elapsed >= d) => None,
                    _ => Some((phase, elapsed)),
                };
            }
            rig.sprint_phase.and_then(|(phase, elapsed)| {
                let duration = match phase {
                    WeaponAnimation::SprintEnter => enter,
                    WeaponAnimation::SprintLoop => looping,
                    WeaponAnimation::SprintLeave => leave,
                    _ => None,
                }?;
                action = phase;
                shot_seek = None;
                Some(elapsed / duration)
            })
        };
        if !rig.arms {
            let ads =
                selected.is_none() && inventory.ads >= 0.99 && weapon.ads_presentation.is_some();
            *visibility = if rig.attached && ads == rig.ads {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            };
        }
        let animation_weapon = if action == WeaponAnimation::Melee && rig.arms {
            inventory
                .melee_definition(&definition.content.weapons)
                .or_else(|| {
                    definition
                        .content
                        .weapons
                        .iter()
                        .find(|w| w.id.0 == "weapon.knife_zm")
                })
                .unwrap_or(weapon)
        } else {
            weapon
        };
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
            if !rig.arms {
                rig.playback = Some(action);
                rig.phase = selected.as_ref().map(|s| s.action);
            }
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
        let finishing_shot =
            selected.is_none() && shot_seek.is_some() && signal == WeaponSignal::Idle;
        let changed = (rig.sequence != Some(sequence) && !finishing_shot)
            || rig.playback != Some(action)
            || rig.phase != selected.as_ref().map(|s| s.action);
        if changed {
            rig.playback_revision = rig.playback_revision.wrapping_add(1);
            if selected.is_some() {
                evidence.phases.insert(format!(
                    "{}:{action:?}:{}",
                    weapon.id.0,
                    if rig.arms { "arms" } else { "model" }
                ));
            }
        }
        let timing = holding_throwable
            .then_some((1., 0.))
            .or_else(|| selected.as_ref().and_then(|s| s.timing))
            .or_else(|| inventory.hand_reload_timing(rig.left))
            .or_else(|| inventory.presentation_timing(weapon, melee));
        let seek = native_clip.and_then(|clip| {
            if let Some((_, elapsed, duration, looping)) = locomotion {
                let duration = if duration > 0. {
                    duration
                } else {
                    clip.duration()
                };
                if duration <= 0. {
                    return None;
                }
                let elapsed = if looping {
                    elapsed.rem_euclid(duration)
                } else {
                    elapsed.min(duration)
                };
                Some(elapsed / duration * clip.duration())
            } else if let Some(fraction) = sprint_fraction {
                Some(fraction.clamp(0., 1.) * clip.duration())
            } else if let Some((duration, remaining)) = timing {
                Some(
                    ((duration - remaining).max(0.) / duration * clip.duration())
                        .min(clip.duration()),
                )
            } else if selected.is_none() && shot_seek.is_some() {
                shot_seek.map(|age| age.min(clip.duration()))
            } else {
                None
            }
        });
        let looping = locomotion.is_some_and(|(_, _, _, looping)| looping)
            || matches!(
                action,
                WeaponAnimation::Idle | WeaponAnimation::IdleEmpty | WeaponAnimation::SprintLoop
            );
        let mut playback =
            AnimationPlayback::new(graph, vec![AnimationLayer::new(node, seek, looping)]);
        playback.restart = rig.playback_revision;
        for &entity in &rig.players {
            commands.entity(entity).insert(playback.clone());
        }
        if let Some(seek) = seek {
            let key = format!(
                "{}:{action:?}:{}",
                rig.weapon.0,
                if rig.arms { "arms" } else { "model" }
            );
            let clock = evidence.clocks.entry(key).or_default();
            clock[0] = native_clip.map_or(0., |clip| clip.duration());
            clock[1] = clock[1].max(seek);
            clock[2] += rig.players.len() as f32;
        }
        rig.sequence = Some(sequence);
        rig.phase = selected.as_ref().map(|s| s.action);
        rig.playback = Some(action);
    }
}

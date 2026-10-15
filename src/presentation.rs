use crate::{
    GameState,
    assets::GameDefinition,
    gameplay::SessionEntity,
    hud::PlayerOverlay,
    origins::content::{ContentId, WeaponAnimation, WeaponId},
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
pub(crate) mod cues;
pub(crate) mod locomotion;
pub(crate) mod offhand;
mod source_units;
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
struct SocketAttachment {
    chain: Vec<Entity>,
    bind_rotation: Quat,
}
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
#[require(crate::animation::SceneClips)]
pub(crate) struct Rig {
    pub(crate) owner: Entity,
    pub(crate) weapon: WeaponId,
    phase: Option<WeaponAnimation>,
    source: Handle<Gltf>,
    nodes: BTreeMap<String, AnimationNodeIndex>,
    prefetched: bool,
    sockets: [Option<Entity>; 3],
    camera_chain: Vec<Entity>,
    bind_rotation: Quat,
    sequence: Option<u64>,
    pub(crate) arms: bool,
    pub(crate) attached: bool,
    pub(crate) ready: bool,
    pub(crate) knife: bool,
    ads: bool,
    pub(crate) left: bool,
    dual: bool,
    playback: Option<WeaponAnimation>,
    main_node: Option<AnimationNodeIndex>,
    selected_clip: Option<ContentId>,
    source_sample: Option<f32>,
    selection_sequence: u64,
    sprinting: bool,
    sprint_phase: Option<(WeaponAnimation, f32)>,
    ads_from: [f32; 2],
    ads_elapsed: f32,
    ads_requested: Option<bool>,
}
impl Rig {
    pub(crate) fn action_sample<'a>(
        &'a self,
        player: &AnimationPlayer,
    ) -> Option<(Entity, bool, &'a ContentId, u64, f32, bool)> {
        if !self.arms || self.knife || self.playback.is_none() {
            return None;
        }
        let active = player.animation(self.main_node?)?;
        Some((
            self.owner,
            self.left,
            self.selected_clip.as_ref()?,
            self.selection_sequence,
            self.source_sample.unwrap_or_else(|| active.elapsed()),
            matches!(
                active.repeat_mode(),
                bevy::animation::RepeatAnimation::Forever
            ),
        ))
    }
}
impl Plugin for PresentationPlugin {
    fn build(&self, app: &mut App) {
        camo::install(app);

        source_units::install(app);
        locomotion::install(app);
        app.add_systems(
            Update,
            (
                spawn_viewmodels,
                build_graphs,
                attach_weapons,
                attachment_visuals.run_if(resource_exists::<GameDefinition>),
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
                .in_set(CameraPose)
                .after(aim_pose)
                .before(bevy::transform::TransformSystems::Propagate),
        )
        .add_systems(
            PostUpdate,
            follow_sockets
                .in_set(crate::animation::AnimationPoseModifiers)
                .after(camera_pose)
                .before(bevy::transform::TransformSystems::Propagate),
        )
        .add_observer(rig_ready);
        app.add_observer(attachment_ready);
        app.add_observer(invalidate_socket_bindings);
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
    if !matches!(**state, GameState::StartingSession | GameState::Playing) {
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
                let source = server
                    .load_builder()
                    .with_settings(|settings: &mut bevy::gltf::GltfLoaderSettings| {
                        settings.load_animations = false;
                        settings.load_materials = bevy::asset::RenderAssetUsages::RENDER_WORLD;
                    })
                    .load(scene.split('#').next().unwrap_or(scene).to_owned());
                let rig = commands
                    .spawn((
                        WorldAssetRoot(
                            server
                                .load_builder()
                                .with_settings(|settings: &mut bevy::gltf::GltfLoaderSettings| {
                                    settings.load_animations = false;
                                    settings.load_materials =
                                        bevy::asset::RenderAssetUsages::RENDER_WORLD;
                                })
                                .load(scene.to_owned()),
                        ),
                        Rig {
                            owner,
                            weapon: weapon.id.clone(),
                            phase: None,
                            source,
                            nodes: BTreeMap::new(),
                            prefetched: false,
                            sockets: [None; 3],
                            camera_chain: Vec::new(),
                            bind_rotation: Quat::IDENTITY,
                            sequence: None,
                            arms: is_arms,
                            attached: is_arms,
                            ready: false,
                            knife: is_knife,
                            ads: is_ads,
                            left: is_left,
                            dual: weapon.dual_weapon.is_some(),
                            playback: None,
                            main_node: None,
                            selected_clip: None,
                            source_sample: None,
                            selection_sequence: 0,
                            sprinting: false,
                            sprint_phase: None,
                            ads_from: [0.; 2],
                            ads_elapsed: 0.,
                            ads_requested: None,
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
    parents: Query<&ChildOf>,
) {
    let sockets: Vec<_> = rigs
        .iter()
        .filter(|(_, rig)| rig.arms && !rig.left && rig.ready)
        .map(|(_, rig)| (rig.owner, rig.weapon.clone(), rig.sockets))
        .collect();
    for (entity, mut rig) in &mut rigs {
        if rig.arms || !rig.ready || rig.attached {
            continue;
        }
        let Some((_, _, sockets)) = sockets
            .iter()
            .find(|(owner, id, _)| *owner == rig.owner && *id == rig.weapon)
        else {
            continue;
        };
        let Some(socket) = sockets[if rig.knife {
            1
        } else if rig.left {
            2
        } else {
            0
        }] else {
            continue;
        };
        let root = parents.get(entity).expect("Viewmodel parent").parent();
        let mut chain = vec![socket];
        for ancestor in parents.iter_ancestors(socket) {
            if ancestor == root {
                break;
            }
            chain.push(ancestor);
        }
        commands.entity(entity).insert((
            SocketAttachment {
                chain,
                bind_rotation: rig.bind_rotation,
            },
            Transform::default(),
            if rig.knife {
                Visibility::Hidden
            } else {
                Visibility::Inherited
            },
        ));
        rig.attached = true;
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
                WorldAssetRoot(
                    server
                        .load_builder()
                        .with_settings(|settings: &mut bevy::gltf::GltfLoaderSettings| {
                            settings.load_animations = false;
                            settings.load_materials = bevy::asset::RenderAssetUsages::RENDER_WORLD;
                        })
                        .load(scene.clone()),
                ),
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
            commands.entity(child).insert(RenderLayers::layer(1));
        }
    }
}
fn hide_attachment_joints(mut joints: Query<&mut Transform, With<AttachmentHidden>>) {
    for mut joint in &mut joints {
        joint.scale = Vec3::ZERO;
    }
}
// Bevy replaces instance entities on asset modification, then sends Ready again.
// Bindings belong to those entities, not to the persistent WorldAssetRoot.
fn invalidate_socket_bindings(
    event: On<WorldInstanceReady>,
    mut commands: Commands,
    mut attachments: Query<(Entity, &SocketAttachment, &mut Rig)>,
) {
    for (entity, socket, mut rig) in &mut attachments {
        if entity == event.entity || socket.chain.contains(&event.entity) {
            rig.attached = false;
            commands
                .entity(entity)
                .remove::<SocketAttachment>()
                .insert(Visibility::Hidden);
        }
    }
}

fn follow_sockets(
    joints: Query<&Transform, Without<SocketAttachment>>,
    mut attachments: Query<(&SocketAttachment, &mut Transform)>,
) {
    // A socket already includes the hands' exported source-axis conversion.
    // Remove the weapon's copy at this boundary, retaining socket translation.
    for (socket, mut transform) in &mut attachments {
        let mut local = Mat4::IDENTITY;
        for &joint in &socket.chain {
            local = joints
                .get(joint)
                .expect("Bound weapon socket joint")
                .to_matrix()
                * local;
        }
        let mut pose = Transform::from_matrix(local).with_scale(Vec3::ONE);
        pose.rotation *= socket.bind_rotation.inverse();
        transform.set_if_neq(pose);
    }
}
fn rig_ready(
    event: On<WorldInstanceReady>,
    mut commands: Commands,
    mut rigs: Query<(&mut Rig, &mut crate::animation::SceneClips)>,
    joints: crate::animation::SceneJoints,
    children: Query<&Children>,
    parents: Query<&ChildOf>,
    players: Query<(), With<AnimationPlayer>>,
    meshes: Query<(), With<Mesh3d>>,
    names: Query<&Name>,
    gltfs: Res<Assets<Gltf>>,
    skins: Res<Assets<bevy::gltf::GltfSkin>>,
    nodes: Res<Assets<bevy::gltf::GltfNode>>,
) {
    let Ok((mut rig, mut clips)) = rigs.get_mut(event.entity) else {
        return;
    };
    joints.bind(event.entity, &mut commands);
    clips.ready = true;
    rig.ready = true;
    rig.sequence = None;
    rig.sockets = [None; 3];
    rig.camera_chain.clear();
    let source = gltfs.get(&rig.source).expect("Ready viewmodel glTF");
    let skin = source
        .skins
        .first()
        .and_then(|skin| skins.get(skin))
        .expect("Viewmodel skin");
    let root = skin
        .joints
        .first()
        .and_then(|joint| nodes.get(joint))
        .expect("Viewmodel bind root");
    rig.bind_rotation = root.transform.rotation;
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
        if let Ok(name) = names.get(child) {
            match name.as_str() {
                "tag_weapon" => rig.sockets[0] = Some(child),
                "tag_knife_attach" => rig.sockets[1] = Some(child),
                "tag_weapon1" => rig.sockets[2] = Some(child),
                "tag_camera" => {
                    rig.camera_chain = std::iter::once(child)
                        .chain(
                            parents
                                .iter_ancestors(child)
                                .take_while(|ancestor| *ancestor != event.entity),
                        )
                        .collect();
                }
                _ => {}
            }
        }
        if players.contains(child) {
            commands.entity(child).remove::<AnimationPlayer>();
        }
        if meshes.contains(child) {
            commands
                .entity(child)
                .insert(RenderLayers::layer(1))
                .remove::<NoFrustumCulling>();
        }
    }
    if rig.arms && rig.dual && !rig.left {
        let left_player = commands.spawn_empty().id();
        let mut left_rig = rig.clone();
        left_rig.left = true;
        left_rig.nodes.clear();
        left_rig.prefetched = false;
        commands.entity(left_player).insert((
            left_rig,
            crate::animation::SceneClips::bound(),
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
        pose.translation += joint.translation / crate::assets::SOURCE_UNIT_METERS;
        pose.rotation *= joint.rotation;
    }
}
fn camera_tag_pose(local: Mat4) -> Transform {
    let basis = Quat::from_rotation_y(std::f32::consts::FRAC_PI_2);
    let (_, rotation, translation) = local.to_scale_rotation_translation();
    Transform {
        translation: basis * translation,
        rotation: basis * rotation * basis.inverse(),
        ..default()
    }
}
fn camera_pose(
    state: Res<State<GameState>>,
    rigs: Query<&Rig>,
    inventories: offhand::Inventories,
    definition: Option<Res<GameDefinition>>,
    mut poses: ParamSet<(
        Query<(&Transform, Option<&ChildOf>)>,
        Query<(&mut Transform, Option<&ViewModelGroup>, Option<&GameCamera>)>,
    )>,
) {
    if !matches!(
        **state,
        GameState::StartingSession | GameState::Playing | GameState::Paused | GameState::GameOver
    ) {
        return;
    }
    let Some(definition) = definition else {
        return;
    };
    let mut eyes = Vec::new();
    for rig in &rigs {
        if !rig.arms || rig.left || rig.knife || !rig.ready || rig.camera_chain.is_empty() {
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
        {
            let transforms = poses.p0();
            let mut local = Mat4::IDENTITY;
            for &joint in &rig.camera_chain {
                let (pose, _) = transforms.get(joint).expect("Bound viewmodel camera joint");
                local = pose.to_matrix() * local;
            }
            let eye = camera_tag_pose(local * Mat4::from_quat(rig.bind_rotation.inverse()));
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
                definition
                    .content
                    .weapons
                    .iter()
                    .find(|w| w.id == rig.weapon)
                    .and_then(|w| w.mechanics.native_ads_pitch),
            ));
        }
    }
    for (owner, eye, ads, calibration, native_pitch) in eyes {
        let inverse = Transform::from_matrix(eye.to_matrix().inverse());
        for (mut pose, group, camera) in &mut poses.p1() {
            if camera.is_some() {
                *pose = pose.mul_transform(eye);
            } else if group.is_some_and(|g| g.owner == owner) {
                *pose = inverse
                    .mul_transform(Transform::from_rotation(native_pitch.map_or_else(
                        || Quat::IDENTITY.slerp(calibration, ads),
                        |pitch| Quat::from_rotation_x(-pitch.to_radians() * ads),
                    )))
                    .mul_transform(Transform::from_translation(if native_pitch.is_some() {
                        Vec3::ZERO
                    } else {
                        VIEWMODEL_ORIGIN * (1. - ads)
                    }));
            }
        }
    }
}
fn build_graphs(
    mut commands: Commands,
    mut rigs: Query<(
        Entity,
        &mut Rig,
        &mut crate::animation::SceneClips,
        &mut AnimationGraphHandle,
        Has<AnimationTransitions>,
    )>,
    mut assets: crate::animation::ClipAssets,
    definition: Option<Res<GameDefinition>>,
) {
    let Some(definition) = definition else { return };
    for (entity, mut rig, mut loaded, mut graph, transitions) in &mut rigs {
        if rig.prefetched {
            continue;
        }
        let Some(weapon) = definition
            .content
            .weapons
            .iter()
            .find(|weapon| weapon.id == rig.weapon)
        else {
            continue;
        };
        let mut complete = true;
        for action in weapon.animation_actions.values() {
            let name = action.0.rsplit('.').next().unwrap();
            let Some(path) = crate::animation::clip_path(&definition, &rig.source, name) else {
                continue;
            };
            if let Some((node, _)) = loaded.node(path, &mut graph, &mut assets) {
                if !rig.nodes.contains_key(name) {
                    rig.nodes.insert(name.to_owned(), node);
                }
            } else {
                complete = false;
            }
        }
        rig.prefetched = complete;
        if !transitions
            && !(rig.arms && weapon.mechanics.native_ads_pitch.is_some())
            && !rig.nodes.is_empty()
        {
            commands.entity(entity).insert(AnimationTransitions::new());
        }
    }
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
    mut rigs: Query<(
        &mut Rig,
        &mut Visibility,
        &mut AnimationPlayer,
        Option<&mut AnimationTransitions>,
        &mut crate::animation::SceneClips,
        &mut AnimationGraphHandle,
    )>,
    mut assets: crate::animation::ClipAssets,
) {
    let Some(definition) = definition else {
        return;
    };
    for (mut rig, mut visibility, mut player, mut transitions, mut loaded, mut graph) in &mut rigs {
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
            player.pause_all();
            rig.sequence = None;
            rig.phase = None;
            rig.playback = None;
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
        if !rig.ready {
            continue;
        }
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
            let duration = assets.duration(*rig.nodes.get(name)?, &graph)?;
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
                        .map(|_| (WeaponAnimation::SprintLoop, elapsed - enter.unwrap_or(0.))),
                    WeaponAnimation::SprintLoop => {
                        looping.map(|_| (WeaponAnimation::SprintLoop, elapsed))
                    }
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
        if name.is_some_and(|name| {
            crate::animation::clip_binding(&definition, &rig.source, name) == Some(None)
        }) {
            // This action has no channels for this part; restore its authored bind pose.
            player.stop_all();
            rig.main_node = None;
            rig.playback = Some(action);
            rig.phase = selected.as_ref().map(|s| s.action);
            rig.sequence = Some(sequence);
            continue;
        }
        let Some((name, path)) = name.and_then(|name| {
            crate::animation::clip_path(&definition, &rig.source, name).map(|path| (name, path))
        }) else {
            if !rig.arms {
                rig.playback = Some(action);
                rig.phase = selected.as_ref().map(|s| s.action);
            }
            continue;
        };
        let Some((node, clip_duration)) = loaded.node(path, &mut graph, &mut assets) else {
            continue;
        };
        if !rig.nodes.contains_key(name) {
            rig.nodes.insert(name.to_owned(), node);
        }
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
        let timing = holding_throwable
            .then_some((1., 0.))
            .or_else(|| selected.as_ref().and_then(|s| s.timing))
            .or_else(|| inventory.hand_reload_timing(rig.left))
            .or_else(|| inventory.presentation_timing(weapon, melee));
        let source_sample = (|| {
            if let Some((_, elapsed, duration, looping)) = locomotion {
                let duration = if duration > 0. {
                    duration
                } else {
                    clip_duration
                };
                if duration <= 0. {
                    return None;
                }
                Some(
                    if looping {
                        elapsed
                    } else {
                        elapsed.min(duration)
                    } / duration
                        * clip_duration,
                )
            } else if let Some(fraction) = sprint_fraction {
                Some(
                    if action == WeaponAnimation::SprintLoop {
                        fraction
                    } else {
                        fraction.clamp(0., 1.)
                    } * clip_duration,
                )
            } else if let Some((duration, remaining)) = timing {
                Some(((duration - remaining).max(0.) / duration * clip_duration).min(clip_duration))
            } else if selected.is_none() && shot_seek.is_some() {
                shot_seek.map(|age| age.min(clip_duration))
            } else {
                None
            }
        })();
        let looping = locomotion.is_some_and(|(_, _, _, looping)| looping)
            || action == WeaponAnimation::SprintLoop
            || clip
                .and_then(|id| {
                    definition
                        .content
                        .animations
                        .iter()
                        .find(|source| source.id == *id)
                })
                .is_some_and(|source| source.looping);
        let restarted = source_sample
            .zip(rig.source_sample)
            .is_some_and(|(now, previous)| now + 0.001 < previous);
        let changed = changed || rig.selected_clip.as_ref() != clip || restarted;
        if changed {
            rig.selection_sequence = rig.selection_sequence.wrapping_add(1);
            rig.selected_clip = clip.cloned();
            if rig.arms {
                let event = match action {
                    WeaponAnimation::Equip => Some("equip"),
                    WeaponAnimation::Unequip => Some("unequip"),
                    WeaponAnimation::Prime => Some("prime"),
                    WeaponAnimation::Melee => Some("melee"),
                    WeaponAnimation::Fire if selected.is_some() => Some("throw"),
                    _ => None,
                };
                if let Some(event) = event {
                    crate::audio::binding(
                        &mut commands,
                        &definition,
                        &format!("{}.{}", weapon.id.0, event),
                        crate::audio::SoundSource::attached(rig.owner),
                    );
                }
            }
            if let Some(previous) = rig.main_node {
                // Main action clips have no source crossfade; ADS layers retain their weights.
                player.stop(previous);
            }
            if let Some(transitions) = transitions.as_mut() {
                transitions.play(&mut player, node, std::time::Duration::ZERO);
            } else {
                player.start(node);
            }
            rig.main_node = Some(node);
        }
        let active = player.play(node);
        active.set_weight(1.).set_repeat(if looping {
            bevy::animation::RepeatAnimation::Forever
        } else {
            bevy::animation::RepeatAnimation::Never
        });
        if let Some(seconds) = source_sample {
            let seek = if looping && clip_duration > 0. {
                seconds.rem_euclid(clip_duration)
            } else {
                seconds.min(clip_duration)
            };
            active.set_seek_time(seek).pause();
        } else if **state == GameState::Playing {
            active.resume();
        } else {
            active.pause();
        }
        if rig.arms && selected.is_none() && weapon.mechanics.native_ads_pitch.is_some() {
            let requested = inventory.native_ads_requested();
            let actions = [WeaponAnimation::AdsEnter, WeaponAnimation::AdsLeave];
            if rig.ads_requested != Some(requested) {
                for (index, action) in actions.iter().enumerate() {
                    rig.ads_from[index] = weapon
                        .animation_actions
                        .get(action)
                        .and_then(|id| id.0.rsplit('.').next())
                        .and_then(|name| rig.nodes.get(name))
                        .and_then(|node| player.animation(*node))
                        .map_or(0., |active| active.weight());
                }
                rig.ads_requested = Some(requested);
                rig.ads_elapsed = 0.;
            }
            if **state == GameState::Playing {
                rig.ads_elapsed += time.delta_secs();
            }
            let blend = (rig.ads_elapsed / 0.1).min(1.);
            for (index, action) in actions.iter().enumerate() {
                let Some(name) = weapon
                    .animation_actions
                    .get(action)
                    .and_then(|id| id.0.rsplit('.').next())
                else {
                    continue;
                };
                let Some(&node) = rig.nodes.get(name) else {
                    continue;
                };
                let Some(duration) = assets.duration(node, &graph) else {
                    continue;
                };
                let fraction = if index == 0 {
                    inventory.ads
                } else {
                    1. - inventory.ads
                };
                let weight = if requested == (index == 0) { 1. } else { 0. };
                player
                    .play(node)
                    .set_weight(rig.ads_from[index].lerp(weight, blend))
                    .set_seek_time(fraction * duration)
                    .pause();
            }
        }
        rig.source_sample = source_sample;
        rig.sequence = Some(sequence);
        rig.phase = selected.as_ref().map(|s| s.action);
        rig.playback = Some(action);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converted_camera_tags_preserve_neutral_and_native_motion() {
        assert!(
            camera_tag_pose(Mat4::IDENTITY)
                .to_matrix()
                .abs_diff_eq(Mat4::IDENTITY, 1e-6)
        );
        let angle = 0.37;
        for (tag_rotation, camera_rotation) in [
            (Quat::from_rotation_x(angle), Quat::from_rotation_z(-angle)),
            (Quat::from_rotation_y(angle), Quat::from_rotation_y(angle)),
            (Quat::from_rotation_z(angle), Quat::from_rotation_x(angle)),
        ] {
            let eye = camera_tag_pose(Mat4::from_rotation_translation(
                tag_rotation,
                Vec3::new(0.1, 0.2, 0.3),
            ));
            assert!(eye.translation.abs_diff_eq(Vec3::new(0.3, 0.2, -0.1), 1e-6));
            for axis in [Vec3::X, Vec3::Y, Vec3::Z] {
                assert!((eye.rotation * axis).abs_diff_eq(camera_rotation * axis, 1e-6));
            }
        }
    }

    #[test]
    fn converted_weapon_model_follows_socket_without_an_extra_axis_rotation() {
        let mut app = App::new();
        app.add_systems(Update, follow_sockets);
        let root = app
            .world_mut()
            .spawn(Transform::from_rotation(Quat::from_rotation_y(
                std::f32::consts::FRAC_PI_2,
            )))
            .id();
        let ancestor_pose =
            Transform::from_xyz(0.2, -0.1, 0.3).with_rotation(Quat::from_rotation_y(0.2));
        let ancestor = app.world_mut().spawn((ChildOf(root), ancestor_pose)).id();
        let socket = app
            .world_mut()
            .spawn((ChildOf(ancestor), Transform::default()))
            .id();
        let model = app
            .world_mut()
            .spawn((
                ChildOf(root),
                SocketAttachment {
                    chain: vec![socket, ancestor],
                    bind_rotation: Quat::IDENTITY,
                },
                Transform::default(),
            ))
            .id();
        for socket_pose in [
            Transform::default(),
            Transform::from_xyz(0.4, -0.2, 0.1)
                .with_rotation(Quat::from_rotation_x(0.4) * Quat::from_rotation_z(-0.3)),
        ] {
            *app.world_mut().get_mut::<Transform>(socket).unwrap() = socket_pose;
            app.update();
            let actual = app.world().get::<Transform>(model).unwrap();
            let expected = ancestor_pose.to_matrix() * socket_pose.to_matrix();
            assert!(actual.to_matrix().abs_diff_eq(expected, 1e-6));
        }
    }
    #[test]
    fn replaced_arms_instance_rebinds_weapon_to_new_socket() {
        let mut app = App::new();
        app.add_observer(invalidate_socket_bindings);
        app.add_systems(Update, attach_weapons);
        app.add_systems(PostUpdate, follow_sockets);
        let root = app.world_mut().spawn(Transform::default()).id();
        let make_rig = || Rig {
            owner: Entity::PLACEHOLDER,
            weapon: WeaponId("weapon.test".into()),
            phase: None,
            source: Handle::default(),
            nodes: BTreeMap::new(),
            prefetched: false,
            sockets: [None; 3],
            camera_chain: Vec::new(),
            bind_rotation: Quat::IDENTITY,
            sequence: None,
            arms: false,
            attached: false,
            ready: true,
            knife: false,
            ads: false,
            left: false,
            dual: false,
            playback: None,
            main_node: None,
            selected_clip: None,
            source_sample: None,
            selection_sequence: 0,
            sprinting: false,
            sprint_phase: None,
            ads_from: [0.; 2],
            ads_elapsed: 0.,
            ads_requested: None,
        };
        let mut arms_rig = make_rig();
        arms_rig.arms = true;
        arms_rig.attached = true;
        let arms = app
            .world_mut()
            .spawn((arms_rig, ChildOf(root), Transform::default()))
            .id();
        let old = app
            .world_mut()
            .spawn((ChildOf(arms), Transform::from_xyz(1., 0., 0.)))
            .id();
        app.world_mut().get_mut::<Rig>(arms).unwrap().sockets[0] = Some(old);
        let weapon = app
            .world_mut()
            .spawn((make_rig(), ChildOf(root), Transform::default()))
            .id();
        app.update();
        assert_eq!(
            app.world().get::<Transform>(weapon).unwrap().translation.x,
            1.
        );
        app.world_mut().despawn(old);
        let new = app
            .world_mut()
            .spawn((ChildOf(arms), Transform::from_xyz(2., 0., 0.)))
            .id();
        app.world_mut().get_mut::<Rig>(arms).unwrap().sockets[0] = Some(new);
        let instance_id = bevy::world_serialization::WorldInstanceSpawner::default()
            .spawn(Handle::<bevy::world_serialization::WorldAsset>::default());
        app.world_mut().trigger(WorldInstanceReady {
            entity: arms,
            instance_id,
        });
        app.world_mut().flush();
        assert!(app.world().get::<SocketAttachment>(weapon).is_none());
        assert!(!app.world().get::<Rig>(weapon).unwrap().attached);
        app.update();
        assert_eq!(
            app.world().get::<Transform>(weapon).unwrap().translation.x,
            2.
        );
        assert_eq!(
            app.world().get::<SocketAttachment>(weapon).unwrap().chain[0],
            new
        );
        assert!(app.world().get::<Rig>(weapon).unwrap().attached);
    }
}

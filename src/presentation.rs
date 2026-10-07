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
        CameraOutputMode, Exposure, Hdr,
        visibility::{NoFrustumCulling, RenderLayers},
    },
    core_pipeline::tonemapping::{Tonemapping, TonemappingLuts},
    light::NotShadowCaster,
    post_process::auto_exposure::{AutoExposure, AutoExposureCompensationCurve},
    prelude::*,
    render::{
        render_resource::BlendState,
        view::{ColorGrading, ColorGradingGlobal},
    },
    world_serialization::WorldInstanceReady,
};
use std::collections::BTreeMap;

pub(crate) mod offhand;
pub use offhand::EquipmentPresentationEvidence;
pub(crate) use offhand::selection as equipment_selection;
pub struct PresentationPlugin;
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct PresentationAnimation;
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct CameraPose;
// Retained hip framing is a reconstruction. Native CG positioning (00533090)
// removes its FOV correction at full ADS; the hip origin bias fades out too.
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
pub(crate) struct ViewModelGroup {
    owner: Entity,
}
#[derive(Component)]
pub(crate) struct Rig {
    owner: Entity,
    pub(crate) weapon: WeaponId,
    phase: Option<WeaponAnimation>,
    tracked: Vec<(Entity, Option<(Vec3, Quat)>)>,
    sampled_phase: Option<WeaponAnimation>,
    source: Handle<Gltf>,
    graph: Option<Handle<AnimationGraph>>,
    nodes: BTreeMap<String, AnimationNodeIndex>,
    players: Vec<Entity>,
    sequence: Option<u64>,
    pub(crate) arms: bool,
    pub(crate) attached: bool,
    pub(crate) ready: bool,
    pub(crate) knife: bool,
    ads: bool,
    playback: Option<WeaponAnimation>,
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
                    .chain()
                    .in_set(PresentationAnimation),
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
                    .before(bevy::camera::CameraUpdateSystems)
                    .before(bevy::transform::TransformSystems::Propagate),
            )
            .add_systems(
                PostUpdate,
                camera_pose
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
            .add_systems(Last, offhand::record_visibility)
            .add_observer(rig_ready);
    }
}
pub(crate) fn first_person_light(
    mut commands: Commands,
    mut cameras: Query<(Entity, &mut Camera), With<GameCamera>>,
    profile: Res<crate::rendering::profile::VisualProfile>,
    mut curves: ResMut<Assets<AutoExposureCompensationCurve>>,
    mut images: ResMut<Assets<Image>>,
    mut luts: ResMut<TonemappingLuts>,
) {
    crate::rendering::grading::install(&mut images, &mut luts, &profile);
    // Meter the composed linear HDR image once. A roof is not a light meter:
    // bright ruins must not receive the dark-room exposure boost. Target
    // photographic middle gray and bound adaptation using the authored EVs.
    let middle_gray = 0.18_f32.log2();
    let brighten = (profile.outdoor_ev100 - profile.indoor_ev100).clamp(0., 5.);
    let compensation = curves.add(
        AutoExposureCompensationCurve::from_curve(bevy::math::cubic_splines::LinearSpline::new([
            Vec2::new(-8., -8. + brighten),
            Vec2::new(middle_gray - brighten, middle_gray),
            Vec2::new(middle_gray + 1., middle_gray),
            Vec2::new(8., 7.),
        ]))
        .expect("Monotonic exposure compensation curve"),
    );
    for (camera, mut world_camera) in &mut cameras {
        // Both views already compose into the same linear HDR intermediate.
        // Publish only the completed image; alpha-blending it over an earlier
        // untonemapped world output makes translucent surfaces reveal that copy.
        world_camera.output_mode = CameraOutputMode::Skip;
        commands.entity(camera).with_children(|parent| {
            let mut final_camera = parent.spawn((
                Camera3d::default(),
                Camera {
                    order: 1,
                    clear_color: ClearColorConfig::None,
                    output_mode: CameraOutputMode::Write {
                        blend_state: Some(BlendState::REPLACE),
                        clear_color: ClearColorConfig::None,
                    },
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
                    ev100: profile.outdoor_ev100,
                },
                AutoExposure {
                    speed_brighten: profile.adaptation_speed,
                    speed_darken: profile.adaptation_speed,
                    compensation_curve: compensation.clone(),
                    ..default()
                },
                Tonemapping::AgX,
                ColorGrading {
                    global: ColorGradingGlobal {
                        post_saturation: profile.post_saturation,
                        ..default()
                    },
                    ..default()
                },
                crate::rendering::postprocess::bloom(&profile),
                match profile.antialiasing {
                    crate::rendering::profile::Antialiasing::Msaa4 => Msaa::Sample4,
                    crate::rendering::profile::Antialiasing::Smaa => Msaa::Off,
                },
                RenderLayers::layer(1),
                ViewModelCamera,
                Name::new("First-person camera"),
            ));
            if matches!(
                profile.antialiasing,
                crate::rendering::profile::Antialiasing::Smaa
            ) {
                final_camera.insert(crate::rendering::postprocess::smaa());
            }
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
            for (is_arms, is_knife, is_ads, scene) in [
                (true, false, false, Some(arms)),
                (false, false, false, Some(scene.as_str())),
                (false, true, false, knife),
                (false, false, true, ads_scene),
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
                            playback: None,
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
        let visible = equipment
            .as_ref()
            .map(|s| &s.weapon)
            .or_else(|| inventory.active().map(|w| &w.definition));
        if visible != Some(&joint.weapon) {
            continue;
        }
        for mut projection in &mut cameras {
            if let Projection::Perspective(p) = &mut *projection {
                // CG_CalcFov (004a0890/004a10a0): weapon zoom fractions and
                // reference 4:3 horizontal FOV. Keep the reconstructed hip lens.
                p.fov = if equipment.is_none() {
                    inventory.ads_fov(VIEWMODEL_FOV_DEGREES, &weapon.mechanics)
                } else {
                    VIEWMODEL_FOV_DEGREES.to_radians()
                };
            }
        }
    }
}
// CG camera-tag composition (004a0730): the weapon is assembled in the base
// view frame, then tag_camera supplies the animated eye. Undo that eye motion
// on the model parent so it is not applied to both the eye and the weapon.
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
        GameState::Playing | GameState::Paused | GameState::GameOver
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
                    break; // Attached model tags do not supply the hand rig's eye.
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
                    .and_then(|w| w.mechanics.ads_sight.as_ref())
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
                // Bounded reconstruction derived from the authored front post.
                // Rotate about the eye so calibration is independent of FOV/depth.
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
/// One native action selection and fixed-step shot clock for meshes and audio.
pub(crate) fn primary_action(
    inventory: &WeaponInventory,
    weapon: &crate::origins::content::WeaponDefinition,
    game: &GameDefinition,
) -> (WeaponAnimation, Option<f32>) {
    let empty = inventory.active().is_some_and(|w| w.magazine == 0);
    let idle = if empty
        && weapon
            .animation_actions
            .contains_key(&WeaponAnimation::IdleEmpty)
    {
        WeaponAnimation::IdleEmpty
    } else {
        WeaponAnimation::Idle
    };
    let fire = if empty
        && weapon
            .animation_actions
            .contains_key(&WeaponAnimation::LastShot)
    {
        WeaponAnimation::LastShot
    } else {
        WeaponAnimation::Fire
    };
    let action = match inventory.signal {
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
    if matches!(inventory.signal, WeaponSignal::Fire | WeaponSignal::Idle)
        && let Some(age) = inventory.shot_elapsed()
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
        if fire == WeaponAnimation::Fire
            && age - fire_duration < duration(WeaponAnimation::Rechamber)
        {
            return (WeaponAnimation::Rechamber, Some(age - fire_duration));
        }
        return (idle, None);
    }
    (action, None)
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
        let (action, shot_seek) = selected.as_ref().map_or_else(
            || primary_action(inventory, weapon, &definition),
            |s| (s.action, None),
        );
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
        for &entity in &rig.players {
            let Ok(mut player) = players.get_mut(entity) else {
                continue;
            };
            let finishing_shot =
                selected.is_none() && shot_seek.is_some() && inventory.signal == WeaponSignal::Idle;
            if (rig.sequence != Some(inventory.sequence) && !finishing_shot)
                || rig.playback != Some(action)
                || rig.phase != selected.as_ref().map(|s| s.action)
            {
                commands
                    .entity(entity)
                    .insert(AnimationGraphHandle(graph.clone()));
                player.stop_all();
                let active = player.play(node);
                if matches!(action, WeaponAnimation::Idle | WeaponAnimation::IdleEmpty) {
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
                } else if selected.is_none() && shot_seek.is_some() {
                    shot_seek.map(|age| age.min(clip.duration()))
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

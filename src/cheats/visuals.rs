use super::{Cheats, Gun, physics::PhysicsGun};
use crate::{GameState, gameplay::SessionEntity, player::Player};
use bevy::{
    camera::visibility::{NoFrustumCulling, RenderLayers},
    prelude::*,
    world_serialization::{WorldAssetRoot, WorldInstanceReady},
};

#[derive(Component)]
struct GunModel(Gun);
#[derive(Component)]
struct ToolHud;
#[derive(Component)]
struct ToolLabel;
#[derive(Component)]
struct Beam;

pub(super) fn install(app: &mut App) {
    app.add_observer(ready)
        .add_systems(
            Update,
            models.after(crate::presentation::PresentationAnimation),
        )
        .add_systems(
            PostUpdate,
            hud.after(crate::presentation::CameraPose)
                .after(bevy::transform::TransformSystems::Propagate),
        );
}

fn models(
    mut commands: Commands,
    cheats: Res<Cheats>,
    state: Res<State<GameState>>,
    game: Option<Res<crate::assets::GameDefinition>>,
    server: Res<AssetServer>,
    cameras: Query<Entity, With<crate::rendering::GameCamera>>,
    players: Query<Entity, With<Player>>,
    mut models: Query<(&GunModel, &mut Visibility), Without<crate::presentation::ViewModelGroup>>,
    mut weapons: Query<
        &mut Visibility,
        (With<crate::presentation::ViewModelGroup>, Without<GunModel>),
    >,
) {
    let active = matches!(**state, GameState::Playing | GameState::Paused);
    // Hide the ordinary first-person group without changing the player's actual inventory.
    for mut visibility in &mut weapons {
        let wanted = if active && cheats.gun != Gun::Weapon {
            Visibility::Hidden
        } else {
            Visibility::Inherited
        };
        if *visibility != wanted {
            *visibility = wanted;
        }
    }
    let mut exists = false;
    for (model, mut visibility) in &mut models {
        let visible = active && model.0 == cheats.gun;
        exists |= model.0 == cheats.gun;
        let wanted = if visible {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        if *visibility != wanted {
            *visibility = wanted;
        }
    }
    if !active || cheats.gun == Gun::Weapon || exists {
        return;
    }
    let (Some(game), Ok(camera), Ok(owner)) = (game, cameras.single(), players.single()) else {
        return;
    };
    let model = if cheats.gun == Gun::Tool {
        "t6_wpn_zmb_raygun_view"
    } else {
        "t6_wpn_zmb_raygun2_view"
    };
    let Some(scene) = crate::assets::native_scene(&game, model) else {
        return;
    };
    let entity = commands
        .spawn((
            GunModel(cheats.gun),
            SessionEntity,
            WorldAssetRoot(
                server
                    .load_builder()
                    .with_settings(|settings: &mut bevy::gltf::GltfLoaderSettings| {
                        settings.load_animations = false;
                        settings.load_materials = bevy::asset::RenderAssetUsages::RENDER_WORLD;
                    })
                    .load(scene.clone()),
            ),
            crate::rendering::native_weapon_view::NativeWeaponViewOwner(owner),
            Transform::from_xyz(0.22, -0.25, -0.38).with_rotation(
                Quat::from_rotation_y(std::f32::consts::FRAC_PI_2)
                    * Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2),
            ),
            Visibility::Inherited,
            Name::new(if cheats.gun == Gun::Tool {
                "Tool Gun"
            } else {
                "Physics Gun"
            }),
        ))
        .id();
    commands.entity(camera).add_child(entity);
}

fn ready(
    event: On<WorldInstanceReady>,
    mut commands: Commands,
    models: Query<(), With<GunModel>>,
    children: Query<&Children>,
    meshes: Query<(), With<Mesh3d>>,
) {
    if !models.contains(event.entity) {
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
                .insert((RenderLayers::layer(1), NoFrustumCulling));
        }
    }
}

fn hud(
    mut commands: Commands,
    cheats: Res<Cheats>,
    state: Res<State<GameState>>,
    gun: Res<PhysicsGun>,
    ui: Query<Entity, With<crate::rendering::display::GameUiCamera>>,
    cameras: Query<(&Camera, &GlobalTransform), With<crate::rendering::GameCamera>>,
    actors: Query<&GlobalTransform, With<crate::enemies::Zombie>>,
    roots: Query<Entity, With<ToolHud>>,
    mut label: Query<&mut Text, With<ToolLabel>>,
    mut beams: Query<(&mut Node, &mut UiTransform, &mut Visibility), With<Beam>>,
) {
    let visible = **state == GameState::Playing && cheats.gun != Gun::Weapon;
    if !visible {
        for root in &roots {
            commands.entity(root).despawn();
        }
        return;
    }
    if roots.is_empty() {
        let Ok(camera) = ui.single() else { return };
        commands
            .spawn((
                ToolHud,
                SessionEntity,
                UiTargetCamera(camera),
                GlobalZIndex(80),
                Node {
                    width: percent(100.),
                    height: percent(100.),
                    position_type: PositionType::Absolute,
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .with_children(|parent| {
                parent.spawn((
                    ToolLabel,
                    Text::new(""),
                    TextFont {
                        font_size: 18.0.into(),
                        ..default()
                    },
                    TextColor(Color::srgb(0.4, 0.9, 1.)),
                    Node {
                        position_type: PositionType::Absolute,
                        left: px(24.),
                        bottom: px(130.),
                        padding: UiRect::all(px(12.)),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.02, 0.04, 0.06, 0.85)),
                ));
                parent.spawn((
                    Beam,
                    Node {
                        position_type: PositionType::Absolute,
                        height: px(3.),
                        ..default()
                    },
                    UiTransform::default(),
                    BackgroundColor(Color::srgb(0.2, 0.8, 1.)),
                    Visibility::Hidden,
                ));
            });
    }
    let caption = match cheats.gun {
        Gun::Tool => format!("TOOL GUN  /  {}\nFire: spawn    Aim: undo    F1: choose spawn    F4: weapon", cheats.spawn.label()),
        Gun::Physics => "PHYSICS GUN\nHold fire: grab enemy    Release: drop / reactivate    Wheel: distance    F4: weapon".into(),
        Gun::Weapon => return,
    };
    for mut text in &mut label {
        let caption = if cheats.notice.is_empty() {
            caption.clone()
        } else {
            format!("{caption}\n{}", cheats.notice)
        };
        if text.0 != caption {
            text.0 = caption;
        }
    }
    let point = gun
        .held
        .and_then(|entity| actors.get(entity).ok())
        .map(|pose| pose.translation());
    for (mut node, mut rotation, mut visibility) in &mut beams {
        *visibility = Visibility::Hidden;
        let (Some(point), Ok((camera, pose))) = (point, cameras.single()) else {
            continue;
        };
        let (Ok(end), Some(size)) = (
            camera.world_to_viewport(pose, point),
            camera.logical_viewport_size(),
        ) else {
            continue;
        };
        let start = size * Vec2::new(0.62, 0.72);
        let delta = end - start;
        let midpoint = (start + end) * 0.5;
        node.width = px(delta.length());
        node.left = px(midpoint.x - delta.length() * 0.5);
        node.top = px(midpoint.y - 1.5);
        rotation.rotation = Rot2::radians(delta.y.atan2(delta.x));
        *visibility = Visibility::Inherited;
    }
}

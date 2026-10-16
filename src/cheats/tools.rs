use super::{Cheats, Gun, physics};
use crate::{GameState, assets::GameDefinition, gameplay::SessionEntity, player::Player};
use avian3d::prelude::*;
use bevy::{
    ecs::system::SystemState, input::mouse::MouseWheel, prelude::*,
    world_serialization::WorldAssetRoot,
};

#[derive(Default, Clone, PartialEq, Eq)]
pub(super) enum Spawn {
    Enemy(String),
    Powerup(String),
    #[default]
    MysteryBox,
}
impl Spawn {
    pub fn label(&self) -> &str {
        match self {
            Self::Enemy(id) | Self::Powerup(id) => id,
            Self::MysteryBox => "Mystery Box",
        }
    }
}

/// Also identifies the extra stations so ordinary station power/relocation rules stay local.
#[derive(Component)]
pub(crate) struct SandboxSpawn;
#[derive(Resource, Default)]
pub(super) struct SpawnHistory(Vec<Entity>);
#[derive(Resource, Default)]
pub(super) struct GunInput {
    pub fire: bool,
    pub held: bool,
    pub secondary: bool,
    pub wheel: f32,
}

pub(super) fn install(app: &mut App) {
    app.add_systems(
        Update,
        operate
            .run_if(in_state(GameState::Playing))
            .run_if(equipped),
    );
}
fn equipped(cheats: Res<Cheats>, physics: Res<physics::PhysicsGun>) -> bool {
    cheats.gun != Gun::Weapon || physics.held.is_some()
}

pub(super) fn input(
    mut cheats: ResMut<Cheats>,
    state: Res<State<GameState>>,
    keys: Res<ButtonInput<KeyCode>>,
    mut game: ResMut<crate::input::GameInput>,
    controls: Res<crate::input::Actions>,
    mut trigger: ResMut<GunInput>,
    mut wheel: MessageReader<MouseWheel>,
) {
    let scroll = wheel.read().map(|event| event.y).sum::<f32>();
    if **state != GameState::Playing || cheats.menu_open {
        *trigger = default();
        return;
    }
    if keys.just_pressed(KeyCode::F2) && cheats.tool_granted {
        cheats.gun = Gun::Tool;
    }
    if keys.just_pressed(KeyCode::F3) && cheats.physics_granted {
        cheats.gun = Gun::Physics;
    }
    if keys.just_pressed(KeyCode::F4) {
        cheats.gun = Gun::Weapon;
    }
    if cheats.gun == Gun::Weapon {
        *trigger = default();
        return;
    }
    trigger.fire |= game.fire;
    trigger.held = game.fire_held;
    trigger.secondary |= controls.state(crate::input::Action::Aim).pressed;
    trigger.wheel += scroll;
    game.fire = false;
    game.fire_held = false;
    game.ads = false;
    game.melee = false;
    game.reload = false;
    game.cycle_weapon = false;
    game.weapon_slot = None;
    game.grenade_held = false;
    game.mine_held = false;
    game.tactical_held = false;
    game.buildable = false;
    game.drone = false;
    game.staff_secondary = false;
}

pub(super) fn aim(world: &mut World) -> Option<(Entity, Vec3, Dir3, f32)> {
    let mut players = world.query::<(Entity, &Player, &Transform)>();
    let (entity, player, pose) = players.single(world).ok()?;
    let origin = pose.translation + Vec3::Y * (player.eye_height - player.height * 0.5);
    let direction = Dir3::new(
        Quat::from_rotation_y(player.yaw) * Quat::from_rotation_x(player.pitch) * Vec3::NEG_Z,
    )
    .ok()?;
    Some((entity, origin, direction, player.yaw))
}

fn operate(world: &mut World, mut spatial: Local<SystemState<SpatialQuery>>) {
    let gun = world.resource::<Cheats>().gun;
    let (fire, held, secondary, wheel) = {
        let mut input = world.resource_mut::<GunInput>();
        (
            std::mem::take(&mut input.fire),
            input.held,
            std::mem::take(&mut input.secondary),
            std::mem::take(&mut input.wheel),
        )
    };
    if gun != Gun::Physics || !held {
        physics::release(world);
    }
    if gun == Gun::Physics {
        if fire {
            physics::grab(world);
        }
        let mut physics = world.resource_mut::<physics::PhysicsGun>();
        physics.distance = (physics.distance + wheel * 0.75).clamp(1.5, 30.);
        return;
    }
    if gun != Gun::Tool {
        return;
    }
    if secondary {
        undo(world);
    }
    if !fire {
        return;
    }
    let Some((player, origin, direction, yaw)) = aim(world) else {
        return;
    };
    let point = {
        let query = spatial.get(world).expect("game physics is installed");
        let point = query.cast_ray(
            origin,
            direction,
            40.,
            true,
            &crate::environment::solids([player]),
        );
        point.map(|hit| origin + *direction * hit.distance)
    };
    let Some(point) = point else {
        world.resource_mut::<Cheats>().notice = "Aim at the ground within 40 metres".into();
        return;
    };
    let floor = {
        let query = spatial.get(world).expect("game physics is installed");
        query
            .cast_ray(
                point + Vec3::Y * 1.5,
                Dir3::NEG_Y,
                4.,
                true,
                &crate::environment::solids([player]),
            )
            .filter(|hit| hit.normal.y > 0.5)
            .map(|hit| point + Vec3::Y * (1.5 - hit.distance + 0.04))
    };
    let Some(floor) = floor else {
        world.resource_mut::<Cheats>().notice = "No supporting ground at the target".into();
        return;
    };
    let selection = world.resource::<Cheats>().spawn.clone();
    match spawn(world, &selection, floor, yaw) {
        Ok(entity) => {
            world.resource_mut::<SpawnHistory>().0.push(entity);
            world.resource_mut::<Cheats>().notice = format!("Spawned {}", selection.label());
        }
        Err(error) => world.resource_mut::<Cheats>().notice = error.into(),
    }
}

fn spawn(
    world: &mut World,
    selection: &Spawn,
    feet: Vec3,
    yaw: f32,
) -> Result<Entity, &'static str> {
    world.resource_scope(|world, game: Mut<GameDefinition>| {
        let server = world.resource::<AssetServer>().clone();
        match selection {
            Spawn::Enemy(id) => {
                let enemy = game
                    .content
                    .enemies
                    .iter()
                    .find(|enemy| enemy.id.0 == *id)
                    .ok_or("Enemy definition is unavailable")?;
                let health = if id == "enemy.panzer" {
                    enemy.health
                } else {
                    game.rounds
                        .health(world.resource::<crate::gameplay::Session>().round)
                }
                .max(1);
                let entity = if id == "enemy.panzer" {
                    let entity = crate::panzer::spawn(
                        &mut world.commands(),
                        feet,
                        health,
                        false,
                        crate::panzer::PanzerPhase::Pursuit,
                        Quat::from_rotation_y(yaw),
                    );
                    world
                        .commands()
                        .entity(entity)
                        .remove::<(crate::rounds::PopulationKind, ColliderDisabled)>();
                    entity
                } else {
                    let center = feet + crate::enemies::body_center(enemy, false);
                    world
                        .spawn((
                            crate::enemies::Zombie::new(
                                center,
                                enemy.id.clone(),
                                crate::rounds::Gait::Run,
                            ),
                            crate::enemies::body_shape(enemy, false),
                            crate::life::Vitality::ordinary_actor(health, health)
                                .expect("positive sandbox health"),
                            SessionEntity,
                            Visibility::Inherited,
                            Transform::from_translation(center)
                                .with_rotation(Quat::from_rotation_y(yaw)),
                            Name::new(format!("Sandbox {}", id)),
                        ))
                        .id()
                };
                world.commands().entity(entity).insert(SandboxSpawn);
                Ok(entity)
            }
            Spawn::Powerup(id) => {
                let entity = crate::powerups::spawn_named(
                    &mut world.commands(),
                    &game,
                    &server,
                    id,
                    feet + Vec3::Y * 0.8,
                )
                .ok_or("Powerup has no native pickup model")?;
                world.commands().entity(entity).insert(SandboxSpawn);
                Ok(entity)
            }
            Spawn::MysteryBox => {
                let scene = crate::assets::native_scene(&game, "p6_anim_zm_tm_magic_box")
                    .ok_or("Mystery Box model is unavailable")?
                    .clone();
                let index = world
                    .resource_mut::<crate::stations::Stations>()
                    .add_sandbox_box();
                Ok(world
                    .spawn((
                        SandboxSpawn,
                        SessionEntity,
                        crate::stations::Station::Box(index),
                        Transform::from_translation(feet).with_rotation(Quat::from_rotation_y(yaw)),
                        Visibility::Inherited,
                        Name::new("Sandbox Mystery Box"),
                    ))
                    .with_children(|parent| {
                        parent.spawn((
                            WorldAssetRoot(server.load(scene)),
                            Transform::from_rotation(Quat::from_rotation_x(
                                -std::f32::consts::FRAC_PI_2,
                            )),
                            Visibility::Inherited,
                        ));
                    })
                    .id())
            }
        }
    })
}

pub(super) fn undo(world: &mut World) {
    while let Some(entity) = world.resource_mut::<SpawnHistory>().0.pop() {
        if world.get_entity(entity).is_err() {
            continue;
        }
        physics::restore(world, entity);
        if let Some(crate::stations::Station::Box(index)) =
            world.get::<crate::stations::Station>(entity)
        {
            let index = *index;
            world
                .resource_mut::<crate::stations::Stations>()
                .remove_sandbox_box(index);
        }
        world.despawn(entity);
        world.resource_mut::<Cheats>().notice = "Undid last Tool Gun spawn".into();
        return;
    }
    world.resource_mut::<Cheats>().notice = "No Tool Gun spawns to undo".into();
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::{
        gltf::GltfPlugin, image::ImagePlugin, mesh::MeshPlugin,
        world_serialization::WorldSerializationPlugin,
    };

    #[test]
    #[ignore = "requires exported installed Origins assets"]
    fn exported_tool_gun_mystery_box_loads_and_undoes() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("target/assets");
        let game = GameDefinition::load_from_root(&root);
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            TransformPlugin,
            AssetPlugin {
                file_path: root.to_string_lossy().into_owned(),
                ..default()
            },
            WorldSerializationPlugin,
            MeshPlugin,
            ImagePlugin::default(),
            AnimationPlugin,
            GltfPlugin::default(),
        ))
        .insert_resource(game)
        .init_resource::<crate::stations::Stations>()
        .init_resource::<Cheats>()
        .init_resource::<SpawnHistory>();
        app.finish();
        app.cleanup();
        let entity = spawn(app.world_mut(), &Spawn::MysteryBox, Vec3::ZERO, 0.).unwrap();
        let scene = app
            .world_mut()
            .query::<&WorldAssetRoot>()
            .single(app.world())
            .unwrap()
            .0
            .clone();
        let server = app.world().resource::<AssetServer>().clone();
        assert_eq!(
            server.get_path(scene.id()).unwrap().to_string(),
            "model_export/p6_anim_zm_tm_magic_box_lod0.glb#Scene0"
        );
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
        while !server.is_loaded_with_dependencies(scene.id()) {
            app.update();
            if let Some(bevy::asset::LoadState::Failed(error)) = server.get_load_state(scene.id()) {
                panic!("Tool Gun Mystery Box failed to load: {error}");
            }
            assert!(
                std::time::Instant::now() < deadline,
                "Mystery Box load timed out"
            );
            std::thread::yield_now();
        }
        app.world_mut()
            .resource_mut::<SpawnHistory>()
            .0
            .push(entity);
        undo(app.world_mut());
        assert!(app.world().get_entity(entity).is_err());
        assert_eq!(
            app.world_mut()
                .resource_mut::<crate::stations::Stations>()
                .add_sandbox_box(),
            6
        );
    }
}

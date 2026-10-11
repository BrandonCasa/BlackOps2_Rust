use crate::{
    GameState,
    gameplay::{GameStep, Session, SessionEntity},
    quests::{MainQuest, QuestStage},
};
use avian3d::prelude::*;
use bevy::prelude::*;
const CENTER: Vec3 = Vec3::new(262.636, -7.9756, 200.8124);
#[derive(Resource, Default)]
pub struct Chamber {
    pub active: Option<usize>,
    pub changes: u32,
    pub crystal_collected: bool,
    timer: f32,
    startup: f32,
    occupancy_poll: f32,
    waiting_for_crystal: bool,
    connectivity_dirty: bool,
}
#[derive(Component)]
struct Divider {
    index: usize,
    elevation: f32,
    ready: bool,
}
#[derive(Component)]
pub(crate) struct DividerCollider;
#[derive(Message, Clone, Copy)]
pub(crate) struct ChamberImpact {
    pub wall: Entity,
}
#[derive(Component)]
struct WallDust {
    wall: Entity,
    next: usize,
    remaining: f32,
}
pub struct ChamberPlugin;
impl Plugin for ChamberPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Chamber>()
            .add_message::<ChamberImpact>()
            .add_observer(ready)
            .add_systems(
                OnEnter(GameState::PreparingShaders),
                setup.in_set(crate::session::SessionSetup::World),
            )
            .add_systems(
                FixedUpdate,
                tick.in_set(GameStep::Movement)
                    .before(crate::movement::platforms::carry)
                    .run_if(in_state(GameState::Playing)),
            );
        app.add_systems(
            FixedUpdate,
            dust.in_set(GameStep::Rewards)
                .run_if(in_state(GameState::Playing)),
        );
        app.add_systems(
            FixedPostUpdate,
            connectivity
                .after(SpatialQuerySystems)
                .run_if(in_state(GameState::Playing)),
        );
    }
}
fn dust(
    mut commands: Commands,
    time: Res<Time<Fixed>>,
    mut impacts: MessageReader<ChamberImpact>,
    mut pulses: Query<(Entity, &mut WallDust)>,
    children: Query<&Children>,
    tags: Query<(&Name, &GlobalTransform)>,
) {
    for impact in impacts.read() {
        commands.spawn((
            SessionEntity,
            WallDust {
                wall: impact.wall,
                next: 1,
                remaining: 0.,
            },
        ));
    }
    for (entity, mut pulse) in &mut pulses {
        pulse.remaining -= time.delta_secs();
        if pulse.remaining > 0. {
            continue;
        }
        let name = format!("tag_fx_dust_{:02}", pulse.next);
        if let Some((_, pose)) = children
            .iter_descendants(pulse.wall)
            .filter_map(|e| tags.get(e).ok())
            .find(|(n, _)| n.as_str() == name)
        {
            crate::native_fx::emit(
                &mut commands,
                "maps/zombie_tomb/fx_tomb_chamber_walls_impact",
                pose.compute_transform(),
            );
        }
        pulse.next += 1;
        pulse.remaining += 0.05;
        if pulse.next > 9 {
            commands.entity(entity).despawn();
        }
    }
}
fn setup(
    mut commands: Commands,
    mut chamber: ResMut<Chamber>,
    game: Res<crate::assets::GameDefinition>,
    server: Res<AssetServer>,
    extras: Query<(Entity, &bevy::gltf::GltfExtras)>,
    children: Query<&Children>,
) {
    for (entity, extras) in &extras {
        if serde_json::from_str::<serde_json::Value>(&extras.value)
            .ok()
            .is_some_and(|data| {
                data["static_instance"] == true
                    && data["source_model"]
                        .as_str()
                        .is_some_and(|model| model.contains("p6_zm_tm_elm_divider"))
            })
        {
            commands
                .entity(entity)
                .insert((Visibility::Hidden, ColliderDisabled));
            for descendant in children.iter_descendants(entity) {
                commands.entity(descendant).insert(ColliderDisabled);
            }
        }
    }
    *chamber = Chamber {
        startup: 3.,
        ..default()
    };
    let scene = game
        .content
        .presentations
        .iter()
        .find(|p| p.id.0 == "source.model.f9817b0fba952a9788ff05e2.lod0")
        .and_then(|p| p.scene.as_ref());
    for (index, degrees) in [270_f32, 0., 180., 90.].into_iter().enumerate() {
        let mut entity = commands.spawn((
            SessionEntity,
            Divider {
                index,
                elevation: 0.,
                ready: false,
            },
            Transform::from_translation(CENTER)
                .with_rotation(Quat::from_rotation_y(degrees.to_radians())),
            RigidBody::Kinematic,
        ));
        if let Some(scene) = scene {
            entity.insert(WorldAssetRoot(server.load(scene.clone())));
        }
    }
}
fn ready(
    event: On<bevy::world_serialization::WorldInstanceReady>,
    mut walls: Query<&mut Divider>,
    children: Query<&Children>,
    meshes: Res<Assets<Mesh>>,
    surfaces: Query<&Mesh3d>,
    mut commands: Commands,
) {
    let Ok(mut wall) = walls.get_mut(event.entity) else {
        return;
    };
    let mut count = 0;
    for entity in children.iter_descendants(event.entity) {
        if let Ok(mesh) = surfaces.get(entity)
            && let Some(shape) = meshes.get(&mesh.0).and_then(Collider::trimesh_from_mesh)
        {
            commands.entity(entity).insert((shape, DividerCollider));
            count += 1;
        }
    }
    wall.ready = count > 0;
}
fn tick(
    mut commands: Commands,
    time: Res<Time<Fixed>>,
    mut chamber: ResMut<Chamber>,
    mut session: ResMut<Session>,
    quest: Res<MainQuest>,
    parts: Res<crate::environment::parts::Parts>,
    players: Query<&Transform, (With<crate::player::Player>, Without<Divider>)>,
    mut walls: Query<(Entity, &mut Divider, &mut Transform, &mut Visibility)>,
    mut impacts: MessageWriter<ChamberImpact>,
    mut rumble: MessageWriter<crate::actions::Rumble>,
) {
    let dt = time.delta_secs();
    if walls.iter().any(|(_, w, _, _)| !w.ready) {
        return;
    }
    chamber.crystal_collected |= parts.pieces.iter().any(|(id, state)| {
        id.contains("elemental_staff")
            && id.ends_with(".gem")
            && !matches!(
                state,
                crate::environment::parts::PartState::Available { .. }
            )
    });
    let occupied = players
        .iter()
        .any(|p| (p.translation - CENTER).with_y(0.).length_squared() < 45.72_f32.powi(2));
    let stopped = quest.enabled && quest.stage == QuestStage::Complete;
    let startup = chamber.startup;
    chamber.startup = (startup - dt).max(0.);
    chamber.timer = (chamber.timer - dt).max(0.);
    if stopped && chamber.active.is_some() {
        chamber.active = None;
    }
    if !stopped && chamber.timer == 0. && !chamber.waiting_for_crystal {
        chamber.occupancy_poll = (chamber.occupancy_poll - dt).max(0.);
        if chamber.occupancy_poll == 0. {
            if occupied {
                chamber.waiting_for_crystal = true;
            } else {
                chamber.occupancy_poll = 1.;
            }
        }
    }
    if !stopped && chamber.waiting_for_crystal && chamber.crystal_collected {
        let old = chamber.active;
        let candidates: Vec<_> = (0..4).filter(|i| Some(*i) != old).collect();
        chamber.active = Some(candidates[session.random() as usize % candidates.len()]);
        chamber.timer = 15. - (session.round.clamp(10, 30) - 10) as f32 * 0.5;
        chamber.waiting_for_crystal = false;
        chamber.occupancy_poll = 0.;
        chamber.changes += 1;
        crate::audio::emit(
            &mut commands,
            "zmb_chamber_wallchange",
            crate::audio::SoundSource::World(CENTER),
        );
    }
    for (entity, mut wall, mut pose, mut visibility) in &mut walls {
        if stopped {
            *visibility = Visibility::Hidden;
        }
        let target = if chamber.startup > 0. || chamber.active == Some(wall.index) {
            0.
        } else {
            25.4
        };
        let initial_raise = startup > 0. || (chamber.changes == 0 && wall.elevation < 25.4);
        let speed = if initial_raise { 25.4 / 0.05 } else { 25.4 };
        let delta = (target - wall.elevation).clamp(-speed * dt, speed * dt);
        let previous = wall.elevation;
        wall.elevation += delta;
        pose.translation = CENTER + Vec3::Y * wall.elevation;
        if previous != target && wall.elevation == target {
            chamber.connectivity_dirty = true;
        }
        if previous > 0. && wall.elevation == 0. {
            impacts.write(ChamberImpact { wall: entity });
            rumble.write(crate::actions::Rumble {
                origin: CENTER,
                radius: 45.72,
                seconds: 0.1,
                strength: 2.,
            });
        }
    }
}
fn connectivity(
    mut chamber: ResMut<Chamber>,
    spatial: SpatialQuery,
    mut navigation: ResMut<crate::navigation::Navigation>,
    colliders: Query<Entity, With<DividerCollider>>,
) {
    if chamber.connectivity_dirty {
        navigation.refresh_chamber(
            &spatial,
            &colliders.iter().collect::<std::collections::HashSet<_>>(),
        );
        chamber.connectivity_dirty = false;
    }
}

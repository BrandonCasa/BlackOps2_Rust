use crate::{gameplay::{GameStep, Session, SessionEntity}, quests::{MainQuest, QuestStage}, GameState};
use avian3d::prelude::*;
use bevy::prelude::*;
const CENTER: Vec3 = Vec3::new(262.636, -7.9756, 200.8124);
#[derive(Resource, Default)]
pub struct Chamber {
    pub active: Option<usize>,
    pub changes: u32,
    pub crystal_collected: bool,
    timer: f32,
    navigation_delay: f32,
}
#[derive(Component)]
struct Divider { index: usize, elevation: f32 }
pub struct ChamberPlugin;
impl Plugin for ChamberPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Chamber>()
            .add_systems(OnEnter(GameState::PreparingShaders), setup.in_set(crate::session::SessionSetup::World))
            .add_systems(FixedUpdate, tick.in_set(GameStep::Movement).before(crate::movement::platforms::carry).run_if(in_state(GameState::Playing)));
    }
}
fn setup(mut commands: Commands, mut chamber: ResMut<Chamber>, game: Res<crate::assets::GameDefinition>, server: Res<AssetServer>, extras: Query<(Entity, &bevy::gltf::GltfExtras)>, children: Query<&Children>) {
    for (entity, extras) in &extras {
        if serde_json::from_str::<serde_json::Value>(&extras.value).ok().is_some_and(|data| data["static_instance"] == true && data["source_model"].as_str().is_some_and(|model| model.contains("p6_zm_tm_elm_divider"))) {
            commands.entity(entity).insert((Visibility::Hidden, ColliderDisabled));
            for descendant in children.iter_descendants(entity) { commands.entity(descendant).insert(ColliderDisabled); }
        }
    }
    *chamber = Chamber { timer: 3., ..default() };
    let scene = game.content.presentations.iter().find(|p| p.id.0 == "source.model.f9817b0fba952a9788ff05e2.lod0").and_then(|p| p.scene.as_ref());
    for (index, degrees) in [270_f32, 0., 180., 90.].into_iter().enumerate() {
        let mut entity = commands.spawn((SessionEntity, Divider { index, elevation: 25.4 },
            Transform::from_translation(CENTER + Vec3::Y * 25.4).with_rotation(Quat::from_rotation_y(degrees.to_radians())),
            RigidBody::Kinematic, ColliderConstructorHierarchy::new(ColliderConstructor::TrimeshFromMesh)));
        if let Some(scene) = scene { entity.insert(WorldAssetRoot(server.load(scene.clone()))); }
    }
}
fn tick(mut commands: Commands, time: Res<Time<Fixed>>, mut chamber: ResMut<Chamber>, mut session: ResMut<Session>, quest: Res<MainQuest>, parts: Res<crate::environment::parts::Parts>,
    players: Query<&Transform, (With<crate::player::Player>, Without<Divider>)>, mut walls: Query<(&mut Divider, &mut Transform)>,
    spatial: SpatialQuery, mut navigation: ResMut<crate::navigation::Navigation>) {
    let dt = time.delta_secs();
    chamber.crystal_collected |= parts.pieces.iter().any(|(id, state)| id.contains("elemental_staff") && id.ends_with(".gem") && !matches!(state, crate::environment::parts::PartState::Available { .. }));
    let occupied = players.iter().any(|p| (p.translation - CENTER).with_y(0.).length_squared() < 45.72_f32.powi(2));
    let stopped = quest.enabled && quest.stage == QuestStage::Complete;
    chamber.timer -= dt;
    if stopped && chamber.active.is_some() {
        chamber.active = None; chamber.navigation_delay = 1.1;
    } else if !stopped && occupied && chamber.crystal_collected && chamber.timer <= 0. {
        let old = chamber.active;
        let candidates: Vec<_> = (0..4).filter(|i| Some(*i) != old).collect();
        chamber.active = Some(candidates[session.random() as usize % candidates.len()]);
        chamber.timer = 15. - (session.round.clamp(10, 30) - 10) as f32 * 0.5;
        chamber.changes += 1; chamber.navigation_delay = 1.1;
        crate::audio::emit(&mut commands, "zmb_chamber_wallchange", crate::audio::SoundSource::World(CENTER));
    }
    for (mut wall, mut pose) in &mut walls {
        let target = if chamber.active == Some(wall.index) { 0. } else { 25.4 };
        let delta = (target - wall.elevation).clamp(-25.4 * dt, 25.4 * dt);
        wall.elevation += delta; pose.translation = CENTER + Vec3::Y * wall.elevation;
    }
    if chamber.navigation_delay > 0. {
        chamber.navigation_delay -= dt;
        if chamber.navigation_delay <= 0. { navigation.refresh_chamber(&spatial); }
    }
}

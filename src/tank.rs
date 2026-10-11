use crate::{assets::GameDefinition, economy::PointLedger, environment::support::DamageEmitter, gameplay::{GameStep, Session, SessionEntity}, input::GameInput, interactions::{InteractionFocus, InteractionRejection}, life::{LifeState, Vitality}, movement::platforms::{MovingPlatform, PlatformSupport}, player::Player, GameState};
use avian3d::prelude::*;
use bevy::prelude::*;
mod source;

#[derive(Component)]
pub(crate) struct TankBody;
#[derive(Component)]
pub(crate) struct CallBox(usize);
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum TankPhase { #[default] Ready, Departing, Moving, Cooling }
#[derive(Resource, Default)]
pub struct Tank {
    pub entity: Option<Entity>,
    pub phase: TankPhase,
    pub station: usize,
    pub trips: u32,
    pub timer: f32,
    node: usize,
    rider_seconds: f32,
    free_next: bool,
    called: bool,
    latched: bool,
    engine: Option<Entity>,
}
pub struct TankPlugin;
impl Plugin for TankPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Tank>()
            .add_systems(OnEnter(GameState::PreparingShaders), setup.in_set(crate::session::SessionSetup::World))
            .add_systems(FixedUpdate, movement.in_set(GameStep::Movement).before(crate::movement::platforms::carry).run_if(in_state(GameState::Playing)))
            .add_systems(FixedUpdate, interact.in_set(GameStep::Interactions).after(crate::buildables::collect_shield).run_if(in_state(GameState::Playing)));
    }
}
fn setup(mut commands: Commands, mut tank: ResMut<Tank>, game: Res<GameDefinition>, server: Res<AssetServer>, mut parts: ResMut<crate::environment::parts::Parts>) {
    *tank = Tank::default();
    let entity = commands.spawn((TankBody, MovingPlatform, SessionEntity, RigidBody::Kinematic, CustomPositionIntegration,
        Collider::cuboid(3., 2., 6.), Transform::from_translation(Vec3::from_array(source::ROUTE[0].0) + Vec3::Y), Visibility::Inherited,
        DamageEmitter::Vehicle("vehicle.tank".into()))).id();
    tank.entity = Some(entity);
    tank.node = 1;
    if let Some(scene) = game.content.presentations.iter().find(|p| p.id.0 == "actor.veh_t6_dlc_mkiv_tank").and_then(|p| p.scene.as_ref()) {
        commands.entity(entity).with_children(|parent| { parent.spawn((WorldAssetRoot(server.load(scene.clone())), Transform::from_xyz(0., -1., 0.))); });
    }
    for (site, position) in [[9.5758, 2.413, 75.819], [-6.9342, -6.4516, -115.2398]].into_iter().enumerate() {
        commands.spawn((CallBox(site), SessionEntity, Transform::from_translation(Vec3::from_array(position))));
    }
    for (piece, position) in [("upper_staff", [58.019188,-2.85623,-76.949554]), ("middle_staff", [-3.69824,5.207254,-10.920476]), ("lower_staff", [1.49987,10.255504,70.197218])] {
        crate::environment::digging::spawn_part(&mut commands, &server, &game, &mut parts,
            &crate::origins::ContentId(format!("part.elemental_staff_lightning.{piece}")), Vec3::from_array(position));
    }
    tank.engine = Some(crate::audio::emit_loop(&mut commands, "zmb_tank_idle", crate::audio::SoundSource::attached(entity)));
}
fn engine(commands: &mut Commands, tank: &mut Tank, alias: &str, entity: Entity) {
    if let Some(previous) = tank.engine.take() { commands.entity(previous).despawn(); }
    tank.engine = Some(crate::audio::emit_loop(commands, alias, crate::audio::SoundSource::attached(entity)));
}
fn movement(mut commands: Commands, time: Res<Time<Fixed>>, mut tank: ResMut<Tank>,
    mut bodies: Query<(Entity, &mut Transform, &DamageEmitter), (With<TankBody>, Without<Vitality>)>,
    actors: Query<(Entity, &Transform, &Vitality, Option<&PlatformSupport>)>, mut damage: ResMut<crate::damage::DamageQueue>) {
    let Ok((entity, mut pose, emitter)) = bodies.single_mut() else { return; };
    let dt = time.delta_secs();
    match tank.phase {
        TankPhase::Ready => return,
        TankPhase::Departing => {
            tank.timer -= dt;
            if tank.timer <= 0. { tank.phase = TankPhase::Moving; engine(&mut commands, &mut tank, "zmb_tank_loop", entity); }
            return;
        }
        TankPhase::Cooling => {
            tank.timer -= dt;
            if tank.timer <= 0. { tank.phase = TankPhase::Ready; crate::audio::emit(&mut commands, "zmb_tank_ready", crate::audio::SoundSource::attached(entity)); engine(&mut commands, &mut tank, "zmb_tank_idle", entity); }
            return;
        }
        TankPhase::Moving => {}
    }
    tank.rider_seconds += dt * actors.iter().filter(|(_,_,health,support)| health.state() == LifeState::Standing && support.is_some_and(|s| s.entity == Some(entity))).count() as f32;
    let (point, speed, stop) = source::ROUTE[tank.node];
    let destination = Vec3::from_array(point) + Vec3::Y;
    let delta = destination - pose.translation;
    let step = delta.clamp_length_max(speed * dt);
    let before = pose.translation;
    pose.translation += step;
    let forward = delta.with_y(0.).normalize_or_zero();
    if forward.length_squared() > 0. { pose.rotation = Quat::from_rotation_y((-forward.x).atan2(-forward.z)); }
    for (target, position, health, support) in &actors {
        if !matches!(health.state(), LifeState::Standing | LifeState::ActorAlive) || support.is_some_and(|s| s.entity == Some(entity)) { continue; }
        let local = pose.rotation.inverse() * (position.translation - pose.translation);
        if local.x.abs() < 1.7 && local.z.abs() < 3.1 && local.y.abs() < 1.5 && (position.translation - before).dot(forward) > 0. {
            damage.0.push(crate::damage::Impact { attacker: entity, target, source: emitter.source(), category: crate::origins::DamageCategory::Crush, region: crate::weapons::HitRegion::Torso, amount: health.maximum() });
        }
    }
    if delta.length() <= step.length() + 0.001 {
        tank.node = if tank.node + 1 >= source::ROUTE.len() { 1 } else { tank.node + 1 };
        if stop {
            tank.station = 1 - tank.station;
            tank.trips += 1;
            tank.phase = TankPhase::Cooling;
            tank.timer = tank.rider_seconds.clamp(2., 120.);
            tank.rider_seconds = 0.;
            tank.free_next = tank.called;
            tank.called = false;
            crate::audio::emit(&mut commands, "zmb_tank_stop", crate::audio::SoundSource::attached(entity));
            engine(&mut commands, &mut tank, "zmb_tank_idle", entity);
        }
    }
}
pub(crate) fn interact(mut commands: Commands, input: Res<GameInput>, settings: Res<crate::input::ControlSettings>, spatial: SpatialQuery,
    mut tank: ResMut<Tank>, bodies: Query<(Entity, &Transform), With<TankBody>>, calls: Query<(Entity, &CallBox, &Transform)>,
    players: Query<(Entity, &Player, &Transform, &Vitality, &crate::weapons::WeaponInventory, &crate::equipment::GrenadeInventory, &crate::equipment::TacticalInventory, &crate::mines::MineInventory, Has<crate::perks::PerkDrinking>)>,
    mut focus: ResMut<InteractionFocus>, mut ledger: ResMut<PointLedger>, mut session: ResMut<Session>) {
    if !input.use_held { tank.latched = false; }
    if focus.target.is_some() { return; }
    let Ok((owner, player, pose, health, inventory, grenades, tactical, mines, drinking)) = players.single() else { return; };
    if health.state() != LifeState::Standing { return; }
    let eye = pose.translation + Vec3::Y * (player.eye_height - player.height * 0.5);
    let forward = Quat::from_rotation_y(player.yaw) * Quat::from_rotation_x(player.pitch) * Vec3::NEG_Z;
    let Ok((body, body_pose)) = bodies.single() else { return; };
    let options = std::iter::once((body, body_pose.translation + Vec3::Y, false)).chain(calls.iter().filter(|(_, call, _)| call.0 != tank.station).map(|(entity, _, pose)| (entity, pose.translation, true)));
    for (entity, point, called) in options {
        let delta = point - eye;
        let Ok(direction) = Dir3::new(delta) else { continue; };
        if delta.length() > 3. || forward.dot(*direction) < 0.8 || spatial.cast_ray(eye, direction, (delta.length() - 0.15).max(0.), true, &crate::environment::solids([owner, body])).is_some() { continue; }
        focus.target = Some(entity);
        let price = if tank.free_next && !called { 0 } else { 500 };
        focus.prompt = match tank.phase {
            TankPhase::Ready => format!("Press {} to {} tank [Cost: {price}]", settings.label(crate::input::Action::Interact), if called { "call" } else { "activate" }),
            TankPhase::Cooling => format!("Tank cooling down [{:.0}s]", tank.timer),
            _ => "Tank route in progress".into(),
        };
        if !input.use_held || tank.latched { return; }
        tank.latched = true;
        if tank.phase != TankPhase::Ready || inventory.action != crate::weapons::WeaponAction::Ready || grenades.busy() || tactical.busy() || mines.busy() || drinking { focus.rejection = Some(InteractionRejection::Busy); return; }
        let purchase = ledger.command(price);
        if let Err(error) = ledger.accept(&mut session.points, purchase, || Ok::<_,InteractionRejection>(())) { focus.rejection = Some(error.into()); return; }
        tank.phase = TankPhase::Departing;
        tank.timer = 4.;
        tank.called = called;
        tank.free_next = false;
        focus.accepted += 1;
        crate::audio::emit(&mut commands, "evt_tank_call", crate::audio::SoundSource::attached(body));
        crate::audio::emit(&mut commands, "zmb_tank_start", crate::audio::SoundSource::attached(body));
        return;
    }
}

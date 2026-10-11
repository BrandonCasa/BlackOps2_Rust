use crate::{assets::GameDefinition, damage::{DamageQueue, Impact}, environment::support::{DamageEmitter, EnvironmentOwner, EnvironmentalHit, ShootableEnvironment}, gameplay::{GameStep, Session, SessionEntity}, input::GameInput, interactions::InteractionFocus, life::{LifeState, Vitality}, player::Player, quests::{MainQuest, QuestStage}, GameState};
use avian3d::prelude::*;
use bevy::prelude::*;
mod source;
#[derive(Clone, Copy, Debug)]
pub struct RobotPass { pub delay: f32, pub step: usize, pub clock: f32, pub eligible: Option<usize>, pub hatch_open: f32 }
#[derive(Clone, Copy, Debug)]
pub struct Occupancy { pub player: Entity, pub robot: usize, pub remaining: f32, pub exit: Vec3 }
#[derive(Resource)]
pub struct Robots {
    pub passes: [Option<RobotPass>; 3],
    pub occupancy: Option<Occupancy>,
    pub entries: u32,
    pub evacuations: u32,
    pub stomps: u32,
    pub triple_round: u32,
    pub triple_active: bool,
    next: f32,
    last: Option<usize>,
    latched: bool,
}
impl Default for Robots {
    fn default() -> Self { Self { passes: [None; 3], occupancy: None, entries: 0, evacuations: 0, stomps: 0, triple_round: 0, triple_active: false, next: 5., last: Some(1), latched: false } }
}
#[derive(Message, Clone, Copy)]
pub struct RobotStomp(pub Vec3);
#[derive(Component)]
struct Foot { robot: usize }
#[derive(Component)]
pub(crate) struct Evacuation(usize);
#[derive(Component)]
struct RobotBody(usize);
pub struct RobotPlugin;
impl Plugin for RobotPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(FixedUpdate, bodies.in_set(GameStep::Movement).run_if(in_state(GameState::Playing)));
        app.add_message::<RobotStomp>();
        app.init_resource::<Robots>()
            .add_systems(OnEnter(GameState::PreparingShaders), setup.in_set(crate::session::SessionSetup::World))
            .add_systems(FixedUpdate, tick.in_set(GameStep::Combat).run_if(in_state(GameState::Playing)))
            .add_systems(FixedUpdate, evacuate.in_set(GameStep::Interactions).after(crate::portals::interact).run_if(in_state(GameState::Playing)));
    }
}
fn pass(delay: f32, eligible: bool, random: u32) -> RobotPass { RobotPass { delay, step: 0, clock: 8., eligible: eligible.then_some(random as usize % 2), hatch_open: 0. } }
fn setup(mut commands: Commands, mut robots: ResMut<Robots>, mut parts: ResMut<crate::environment::parts::Parts>, mut session: ResMut<Session>, mut access: ResMut<crate::access::ZoneAccess>, game: Res<GameDefinition>, server: Res<AssetServer>) {
    *robots = default(); robots.passes[1] = Some(pass(0., false, session.random()));
    for i in 0..3 {
        let mut body = commands.spawn((SessionEntity, RobotBody(i), Transform::default(), Visibility::Hidden));
        if let Some(scene) = crate::assets::native_scene(&game, "veh_t6_dlc_zm_robot") { body.insert(WorldAssetRoot(server.load(scene.clone()))); }
        commands.spawn((SessionEntity, Foot { robot: i }, Transform::from_translation(Vec3::from_array(source::ROUTES[i][0]) + Vec3::Y * 20.), Visibility::Hidden,
            Collider::cuboid(4.5, 0.5, 4.5), Sensor, CollisionLayers::new(crate::environment::brushes::BRUSH_LAYER, LayerMask::NONE),
            EnvironmentOwner::Robot, ShootableEnvironment, DamageEmitter::Robot(format!("robot.{i}"))));
        commands.spawn((SessionEntity, Evacuation(i), Transform::from_translation(Vec3::from_array(source::EVACUATION[i]))));
    }
    access.activated.extend(game.content.zones.iter().filter(|z| z.id.0 == "zone.robot_head").map(|z| z.id.clone())); access.refresh(&game.content);
    for (id, point) in [("upper_staff", [-171.628562,4.8895,170.53687]), ("middle_staff", [-157.954218,4.918964,170.675554]), ("lower_staff", [-144.915128,4.91617,170.575986])] {
        crate::crafting::reveal_part(&mut commands, &mut parts, &format!("part.elemental_staff_air.{id}"), Vec3::from_array(point));
    }
}
fn tick(mut commands: Commands, time: Res<Time<Fixed>>, mut robots: ResMut<Robots>, mut session: ResMut<Session>, access: Res<crate::access::ZoneAccess>, quest: Res<MainQuest>,
    mut feet: Query<(Entity, &Foot, &mut Transform), Without<Player>>, mut hits: MessageReader<EnvironmentalHit>,
    mut players: Query<(Entity, &mut Player, &mut Transform, &Vitality, &mut crate::movement::Mobility), (With<Player>, Without<Foot>)>,
    enemies: Query<(Entity, &Transform, &Vitality), (With<crate::enemies::Zombie>, Without<Player>, Without<Foot>)>, mut damage: ResMut<DamageQueue>, mut stomps: MessageWriter<RobotStomp>) {
    let dt = time.delta_secs();
    for hit in hits.read() {
        if hit.amount == 0 { continue; }
        if let Ok((_, foot, _)) = feet.get(hit.target) && let Some(p) = robots.passes[foot.robot].as_mut() {
            if p.eligible.is_some_and(|side| p.step % 2 == side) && p.delay <= 0. { p.hatch_open = 5.; }
        }
    }
    if robots.passes.iter().all(Option::is_none) {
        robots.triple_active = false;
        robots.next -= dt;
        if robots.next <= 0. {
            let triple = session.round % 4 == 0 && robots.triple_round != session.round || quest.enabled && quest.stage == QuestStage::BreakSeal;
            if triple {
                robots.triple_active = true;
                let eligible = session.random() as usize % 3;
                robots.passes[2] = Some(pass(10., eligible == 2, session.random()));
                robots.passes[0] = Some(pass(15., eligible == 0, session.random()));
                robots.passes[1] = Some(pass(20., eligible == 1, session.random()));
                robots.triple_round = session.round; robots.last = None;
            } else {
                let count = if access.reachable.iter().any(|z| z.0.starts_with("zone.nml")) { 3 } else { 2 };
                let mut index = session.random() as usize % count;
                if robots.last == Some(index) { index = (index + 1) % count; }
                robots.passes[index] = Some(pass(10., true, session.random())); robots.last = Some(index);
            }
            robots.next = 5.;
        }
    }
    for (entity, foot, mut pose) in &mut feet {
        let Some(mut p) = robots.passes[foot.robot] else { continue; };
        p.hatch_open = (p.hatch_open - dt).max(0.);
        if p.delay > 0. { p.delay -= dt; robots.passes[foot.robot] = Some(p); continue; }
        p.clock -= dt;
        let center = Vec3::from_array(source::ROUTES[foot.robot][p.step]);
        pose.translation = center + Vec3::Y * (p.clock / 8.).clamp(0., 1.) * 12.;
        if p.clock <= 0. {
            robots.stomps += 1; stomps.write(RobotStomp(center));
            crate::audio::emit(&mut commands, "zmb_robot_foot_impact", crate::audio::SoundSource::World(center));
            for (owner, mut player, mut body, vitality, mut mobility) in &mut players {
                if !matches!(vitality.state(), LifeState::Standing | LifeState::Downed) || robots.occupancy.is_some() { continue; }
                let delta = body.translation - center;
                if delta.xz().length_squared() > 6.5_f32.powi(2) || delta.y.abs() > 6. { continue; }
                if p.hatch_open > 0. && p.eligible.is_some_and(|side| p.step % 2 == side) && vitality.state() == LifeState::Standing && delta.xz().length_squared() < 2.54_f32.powi(2) {
                    crate::portals::relocate(&mut commands, owner, &mut player, &mut body, &mut mobility, Vec3::from_array(source::INTERIORS[foot.robot]));
                    robots.occupancy = Some(Occupancy { player: owner, robot: foot.robot, remaining: 25., exit: center - Vec3::Y * 1.5 }); robots.entries += 1;
                } else {
                    damage.0.push(Impact { attacker: entity, target: owner, source: crate::damage::ImpactSource::Robot(format!("robot.{}", foot.robot)), category: crate::origins::DamageCategory::Crush, region: crate::weapons::HitRegion::Torso, amount: 999999 });
                }
            }
            for (target, body, vitality) in &enemies {
                if vitality.state() == LifeState::ActorAlive && (body.translation - center).xz().length_squared() < 6.5_f32.powi(2) && (body.translation.y - center.y).abs() < 6. {
                    damage.0.push(Impact { attacker: entity, target, source: crate::damage::ImpactSource::Robot(format!("robot.{}", foot.robot)), category: crate::origins::DamageCategory::Crush, region: crate::weapons::HitRegion::Torso, amount: vitality.maximum() });
                }
            }
            p.step += 1; p.clock = 8.; p.hatch_open = 0.;
            if p.step >= source::ROUTES[foot.robot].len() { robots.passes[foot.robot] = None; pose.translation.y = -100.; continue; }
        }
        robots.passes[foot.robot] = Some(p);
    }
    if let Some(mut occupancy) = robots.occupancy {
        occupancy.remaining -= dt;
        if let Ok((owner, mut player, mut pose, health, mut mobility)) = players.get_mut(occupancy.player) {
            if health.state() == LifeState::HumanTerminal { robots.occupancy = None; }
            else if occupancy.remaining <= 0. {
                crate::portals::relocate(&mut commands, owner, &mut player, &mut pose, &mut mobility, occupancy.exit); robots.occupancy = None; robots.evacuations += 1;
            } else { robots.occupancy = Some(occupancy); }
        } else { robots.occupancy = None; }
    }
}
pub(crate) fn evacuate(mut commands: Commands, input: Res<GameInput>, settings: Res<crate::input::ControlSettings>, mut robots: ResMut<Robots>, mut focus: ResMut<InteractionFocus>,
    exits: Query<(Entity, &Evacuation, &Transform), Without<Player>>, mut players: Query<(Entity, &mut Player, &mut Transform, &Vitality, &mut crate::movement::Mobility), With<Player>>) {
    if !input.use_held { robots.latched = false; }
    let Some(occupancy) = robots.occupancy else { return; };
    if focus.target.is_some() { return; }
    let Ok((owner, mut player, mut pose, health, mut mobility)) = players.get_mut(occupancy.player) else { return; };
    if health.state() != LifeState::Standing { return; }
    let Some((entity, _, _)) = exits.iter().find(|(_, exit, t)| exit.0 == occupancy.robot && t.translation.distance_squared(pose.translation) < 3.0_f32.powi(2)) else { return; };
    focus.target = Some(entity); focus.progress = 0.; focus.hold_seconds = 0.; focus.prompt = format!("Press {} to evacuate [{:.0}s]", settings.label(crate::input::Action::Interact), occupancy.remaining);
    if input.use_held && !robots.latched {
        robots.latched = true; crate::portals::relocate(&mut commands, owner, &mut player, &mut pose, &mut mobility, occupancy.exit);
        robots.occupancy = None; robots.evacuations += 1; focus.accepted += 1;
    }
}

fn bodies(robots: Res<Robots>, mut bodies: Query<(&RobotBody, &mut Transform, &mut Visibility)>) {
    for (body, mut pose, mut visibility) in &mut bodies {
        let Some(pass) = robots.passes[body.0].filter(|p| p.delay <= 0.) else { *visibility = Visibility::Hidden; continue; };
        let route = source::ROUTES[body.0];
        let to = Vec3::from_array(route[pass.step]);
        let from = Vec3::from_array(route[pass.step.saturating_sub(1)]);
        pose.translation = from.lerp(to, (1. - pass.clock / 8.).clamp(0., 1.));
        let direction = (to - from).with_y(0.).normalize_or_zero();
        if direction != Vec3::ZERO { pose.rotation = Quat::from_rotation_y((-direction.x).atan2(-direction.z)); }
        *visibility = Visibility::Inherited;
    }
}

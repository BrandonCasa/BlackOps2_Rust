use crate::{assets::GameDefinition, combat_effects::Armor, damage::{DamageApplied, DamageQueue, EnemyDeath, Impact, ImpactSource}, enemies::Zombie, gameplay::{GameStep, Session, SessionEntity}, life::{LifeState, Vitality}, origins::content::{ContentId, DamageCategory}, player::Player, rounds::{Gait, PopulationKind}, weapons::HitRegion, GameState};
use avian3d::prelude::*;
use bevy::prelude::*;
mod source;

#[derive(Component, Default)]
pub struct CapturedByPanzer(pub Option<Entity>);
#[derive(Clone, Copy, Debug)]
pub enum PanzerPhase {
    Landing(f32), Pursuit, Melee(f32), FlameWindup(f32), Flame(f32),
    Claw { tip: Vec3, direction: Vec3, remaining: f32 }, Pulling(f32), Dead(f32),
}
#[derive(Component)]
pub struct Panzer {
    pub phase: PanzerPhase,
    pub quest: bool,
    pub captured: Option<Entity>,
    flame_cooldown: f32,
    claw_cooldown: f32,
    flame_tick: f32,
    claw_damage: u32,
    plate_rewarded: bool,
    vocal_timer: f32,
}
#[derive(Resource)]
pub struct PanzerWaves {
    pub next_round: u32,
    pub pending: u32,
    pub spawned: u32,
    pub killed: u32,
    pub fire_part_dropped: bool,
    timer: f32,
}
impl Default for PanzerWaves {
    fn default() -> Self { Self { next_round: 8, pending: 0, spawned: 0, killed: 0, fire_part_dropped: false, timer: 0. } }
}
pub struct PanzerPlugin;
impl Plugin for PanzerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PanzerWaves>()
            .add_systems(OnEnter(GameState::PreparingShaders), reset.in_set(crate::session::SessionSetup::World))
            .add_systems(FixedUpdate, schedule.in_set(GameStep::Rounds).before(crate::rounds::advance).run_if(in_state(GameState::Playing)))
            .add_systems(FixedUpdate, behavior.in_set(GameStep::EnemyIntent).run_if(in_state(GameState::Playing)))
            .add_systems(FixedUpdate, deaths.in_set(GameStep::Rewards).after(crate::damage::reward_deaths).run_if(in_state(GameState::Playing)));
    }
}
fn reset(mut waves: ResMut<PanzerWaves>) { *waves = PanzerWaves::default(); }
pub fn spawn(commands: &mut Commands, game: &GameDefinition, server: &AssetServer, position: Vec3, health: u32, quest: bool) -> Entity {
    let entity = commands.spawn((Panzer { phase: PanzerPhase::Landing(1.5), quest, captured: None, flame_cooldown: 2., claw_cooldown: 5., flame_tick: 0., claw_damage: 0, plate_rewarded: false, vocal_timer: 3. },
        Zombie::new(position + Vec3::Y * 1.2, ContentId("enemy.panzer".into()), Gait::Run, 2.5),
        SessionEntity, PopulationKind::Special, Collider::capsule(0.5, 1.4), Transform::from_translation(position + Vec3::Y * 1.2), Visibility::Inherited,
        Vitality::ordinary_actor(health, health).expect("source Panzer health"), Armor::panzer(health), crate::hit_regions::Hitboxes::humanoid(2.4, 0.5))).id();
    if let Some(scene) = game.content.presentations.iter().find(|p| p.id.0 == "actor.c_zom_mech_body").and_then(|p| p.scene.as_ref()) {
        commands.entity(entity).insert(WorldAssetRoot(server.load(scene.clone())));
    }
    crate::audio::emit(commands, "zmb_ai_mechz_vox_spawn", crate::audio::SoundSource::attached(entity));
    entity
}
fn schedule(mut commands: Commands, time: Res<Time<Fixed>>, mut session: ResMut<Session>, mut waves: ResMut<PanzerWaves>,
    access: Res<crate::access::ZoneAccess>, game: Res<GameDefinition>, server: Res<AssetServer>, query: SpatialQuery,
    players: Query<&Transform, With<Player>>, panzers: Query<&Vitality, With<Panzer>>) {
    if !access.reachable.iter().any(|z| z.0.starts_with("zone.nml")) { return; }
    if waves.pending == 0 && session.round >= waves.next_round && !panzers.iter().any(|v| v.state() == LifeState::ActorAlive) {
        waves.pending = 1; waves.timer = 10. + (session.random() % 5000) as f32 / 1000.;
    }
    if waves.pending == 0 { return; }
    waves.timer -= time.delta_secs();
    if waves.timer > 0. { return; }
    let Ok(player) = players.single() else { return; };
    let choices: Vec<_> = source::SPAWNS.iter().filter(|(zone, point)| {
        access.reachable.iter().any(|z| z.0 == *zone) && (12.7..63.5).contains(&Vec3::from_array(*point).distance(player.translation))
    }).collect();
    if choices.is_empty() { waves.timer = 0.5; return; }
    let (_, point) = choices[session.random() as usize % choices.len()];
    let position = Vec3::from_array(*point);
    if !query.shape_intersections(&Collider::capsule(0.5, 1.4), position + Vec3::Y * 1.2, Quat::IDENTITY, &crate::environment::solids([])).is_empty() { waves.timer = 0.5; return; }
    let health = (5000 + 1000 * waves.spawned).min(22500);
    spawn(&mut commands, &game, &server, position, health, false);
    waves.spawned += 1; waves.pending -= 1;
    waves.next_round = session.round + 4 + session.random() % 2;
}
fn strike(damage: &mut DamageQueue, attacker: Entity, target: Entity, amount: u32, category: DamageCategory) {
    damage.0.push(Impact { attacker, target, source: ImpactSource::Enemy(ContentId("enemy.panzer".into())), category, region: HitRegion::Torso, amount });
}
fn behavior(mut commands: Commands, time: Res<Time<Fixed>>, query: SpatialQuery, slide: MoveAndSlide,
    mut panzers: Query<(Entity, &mut Panzer, &mut Transform, &Collider, &Vitality, &Armor), Without<Player>>,
    mut players: Query<(Entity, &mut Player, &mut Transform, &Collider, &Vitality, &mut CapturedByPanzer), (With<Player>, Without<Panzer>)>,
    mut damage: ResMut<DamageQueue>, mut received: MessageReader<DamageApplied>, mut navigation: ResMut<crate::navigation::Navigation>, access: Res<crate::access::ZoneAccess>,
    effects: Res<crate::powerups::PowerupEffects>, mut session: ResMut<Session>) {
    let dt = time.delta_secs();
    for event in received.read() {
        if let Ok((_, mut panzer, _, _, health, armor)) = panzers.get_mut(event.impact.target) {
            if matches!(event.impact.category, DamageCategory::Explosive) || event.impact.source.id().contains("staff_") { panzer.claw_damage = panzer.claw_damage.saturating_add(event.impact.amount); }
            if event.impact.region == HitRegion::Arm || panzer.claw_damage > health.maximum() / 10 {
                if let Some(owner) = panzer.captured.take() { if let Ok((_, _, _, _, _, mut captured)) = players.get_mut(owner) { captured.0 = None; } }
                if matches!(panzer.phase, PanzerPhase::Pulling(_) | PanzerPhase::Claw { .. }) { panzer.phase = PanzerPhase::Pursuit; }
            }
            if armor.broken && !panzer.plate_rewarded {
                panzer.plate_rewarded = true;
                if event.impact.source.player_attributed() { session.award(100); }
            }
        }
    }
    for (entity, mut panzer, mut pose, collider, health, _) in &mut panzers {
        if health.state() != LifeState::ActorAlive {
            if let Some(owner) = panzer.captured.take() { if let Ok((_, _, _, _, _, mut captured)) = players.get_mut(owner) { captured.0 = None; } }
            panzer.phase = match panzer.phase { PanzerPhase::Dead(time) => PanzerPhase::Dead(time - dt), _ => { commands.entity(entity).insert(ColliderDisabled); PanzerPhase::Dead(5.) } };
            if matches!(panzer.phase, PanzerPhase::Dead(time) if time <= 0.) { commands.entity(entity).despawn(); }
            continue;
        }
        panzer.flame_cooldown = (panzer.flame_cooldown - dt).max(0.);
        panzer.claw_cooldown = (panzer.claw_cooldown - dt).max(0.);
        panzer.vocal_timer -= dt;
        if panzer.vocal_timer <= 0. { panzer.vocal_timer = 3. + (session.random() % 3000) as f32 / 1000.; crate::audio::emit(&mut commands, "zmb_ai_mechz_vox_ambient", crate::audio::SoundSource::attached(entity)); }
        let Ok((owner, mut player, mut player_pose, player_collider, vitality, mut captured)) = players.single_mut() else { continue; };
        if vitality.state() != LifeState::Standing {
            captured.0 = None; panzer.captured = None; panzer.phase = PanzerPhase::Pursuit; continue;
        }
        if effects.zombie_blood > 0. && panzer.captured.is_none() { continue; }
        let delta = player_pose.translation - pose.translation;
        let distance = delta.length();
        let direction = delta.with_y(0.).normalize_or_zero();
        if direction.length_squared() > 0. { pose.rotation = Quat::from_rotation_y((-direction.x).atan2(-direction.z)); }
        let visible = Dir3::new(delta).ok().is_some_and(|d| query.cast_ray(pose.translation, d, distance, true, &crate::environment::solids([entity, owner])).is_none());
        match panzer.phase {
            PanzerPhase::Landing(remaining) => { panzer.phase = if remaining > dt { PanzerPhase::Landing(remaining - dt) } else { PanzerPhase::Pursuit }; }
            PanzerPhase::Pursuit => {
                if visible && distance < 1.9 { panzer.phase = PanzerPhase::Melee(0.55); }
                else if visible && (2.54..5.68).contains(&distance) && panzer.flame_cooldown == 0. { panzer.phase = PanzerPhase::FlameWindup(0.5); panzer.flame_cooldown = 5.; }
                else if visible && (5.68..25.4).contains(&distance) && panzer.claw_cooldown == 0. && captured.0.is_none() {
                    panzer.phase = PanzerPhase::Claw { tip: pose.translation + Vec3::Y * 0.4, direction: delta.normalize_or_zero(), remaining: 1.5 };
                    panzer.claw_cooldown = 7.; panzer.claw_damage = 0;
                    crate::audio::emit(&mut commands, "zmb_ai_mechz_claw_fire", crate::audio::SoundSource::attached(entity));
                } else {
                    let feet = pose.translation - Vec3::Y * 1.2;
                    let target = navigation.destination(feet, player_pose.translation - Vec3::Y * player.height * 0.5, &access) + Vec3::Y * 1.2;
                    let velocity = (target - pose.translation).with_y(0.).normalize_or_zero() * if distance > 9.144 { 3. } else { 1.8 };
                    let result = crate::player::slide_character(&slide, &query, crate::player::MotionShape { collider, radius: 0.5, step_height: 0.45, grounded: true }, pose.translation, velocity - Vec3::Y * 2., time.delta(), &crate::environment::solids([entity]));
                    pose.translation = result.position;
                }
            }
            PanzerPhase::Melee(remaining) => {
                if remaining > dt { panzer.phase = PanzerPhase::Melee(remaining - dt); }
                else { if distance < 2.2 && visible { strike(&mut damage, entity, owner, 75, DamageCategory::Melee); } panzer.phase = PanzerPhase::Pursuit; }
            }
            PanzerPhase::FlameWindup(remaining) => {
                panzer.phase = if remaining > dt { PanzerPhase::FlameWindup(remaining - dt) } else { panzer.flame_tick = 0.; PanzerPhase::Flame(2.5) };
            }
            PanzerPhase::Flame(remaining) => {
                panzer.flame_tick -= dt;
                if panzer.flame_tick <= 0. { panzer.flame_tick += 0.5; if distance < 6. && visible { strike(&mut damage, entity, owner, 10, DamageCategory::Fire); } }
                panzer.phase = if remaining > dt { PanzerPhase::Flame(remaining - dt) } else { PanzerPhase::Pursuit };
            }
            PanzerPhase::Claw { tip, direction, remaining } => {
                let step = direction * 30.48 * dt;
                let blocked = Dir3::new(step).ok().and_then(|d| query.cast_shape(&Collider::sphere(0.18), tip, Quat::IDENTITY, d, &ShapeCastConfig::from_max_distance(step.length()), &crate::environment::solids([entity])));
                if blocked.is_some_and(|hit| hit.entity == owner) || (tip + step).distance(player_pose.translation) < 0.65 {
                    captured.0 = Some(entity); panzer.captured = Some(owner); panzer.phase = PanzerPhase::Pulling(8.);
                } else if blocked.is_some() || remaining <= dt { panzer.phase = PanzerPhase::Pursuit; }
                else { panzer.phase = PanzerPhase::Claw { tip: tip + step, direction, remaining: remaining - dt }; }
            }
            PanzerPhase::Pulling(remaining) => {
                if panzer.captured != Some(owner) || captured.0 != Some(entity) || remaining <= dt || distance < 1.8 {
                    captured.0 = None; panzer.captured = None; panzer.phase = PanzerPhase::Pursuit;
                } else {
                    let velocity = (pose.translation - player_pose.translation).normalize_or_zero() * 5.08;
                    let result = slide.move_and_slide(player_collider, player_pose.translation, Quat::IDENTITY, velocity, time.delta(), &MoveAndSlideConfig::default(), &crate::environment::player_solids([owner, entity]), |_| MoveAndSlideHitResponse::Accept);
                    player_pose.translation = result.position; player.velocity = Vec3::ZERO;
                    panzer.phase = PanzerPhase::Pulling(remaining - dt);
                }
            }
            PanzerPhase::Dead(_) => {}
        }
    }
}
fn deaths(mut commands: Commands, mut deaths: MessageReader<EnemyDeath>, panzers: Query<(&Panzer, &Transform)>,
    mut waves: ResMut<PanzerWaves>, mut parts: ResMut<crate::environment::parts::Parts>) {
    for event in deaths.read() {
        let Ok((_, pose)) = panzers.get(event.0.impact.target) else { continue; };
        waves.killed += 1;
        if !waves.fire_part_dropped {
            crate::crafting::reveal_part(&mut commands, &mut parts, "part.elemental_staff_fire.lower_staff", pose.translation - Vec3::Y);
            waves.fire_part_dropped = true;
        }
        crate::audio::emit(&mut commands, "zmb_ai_mechz_vox_death", crate::audio::SoundSource::World(pose.translation));
    }
}

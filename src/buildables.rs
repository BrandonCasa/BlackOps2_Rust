use crate::{assets::GameDefinition, crafting::{BuildableInventory, Crafting}, damage::{DamageQueue, Impact, ImpactSource}, gameplay::{GameStep, SessionEntity}, input::GameInput, life::{LifeState, Vitality}, origins::content::{DamageCategory, WeaponId}, player::{Player, TickActions}, weapons::HitRegion, GameState};
use avian3d::prelude::*;
use bevy::prelude::*;

#[derive(Component)]
pub struct ShieldAggro(pub Entity);
#[derive(Component)]
struct DeployedShield;
#[derive(Component)]
pub struct Drone {
    pub owner: Entity,
    pub remaining: f32,
    pub returning: bool,
    pub upgraded: bool,
    home: Vec3,
    cooldown: f32,
}
#[derive(Resource, Default)]
pub struct BuildableState {
    pub drone: Option<Entity>,
    pub drone_cooldown: f32,
    pub drone_upgraded: bool,
    melee_cooldown: f32,
    use_latched: bool,
}
pub struct BuildablePlugin;
impl Plugin for BuildablePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<BuildableState>()
            .add_systems(OnEnter(GameState::PreparingShaders), reset.in_set(crate::session::SessionSetup::World))
            .add_systems(FixedUpdate, (actions, shield_targets).chain().before(crate::enemies::lifecycle)
                .in_set(GameStep::EnemyIntent).run_if(in_state(GameState::Playing)))
            .add_systems(FixedUpdate, drones.in_set(GameStep::Combat).run_if(in_state(GameState::Playing)))
            .add_systems(FixedUpdate, collect_shield.in_set(GameStep::Interactions).after(crate::crafting::interact).run_if(in_state(GameState::Playing)))
            .add_systems(FixedUpdate, cleanup.in_set(GameStep::Zombies).run_if(in_state(GameState::Playing)));
    }
}
fn reset(mut state: ResMut<BuildableState>) { *state = BuildableState::default(); }
fn visual(commands: &mut Commands, entity: Entity, model: &str, game: &GameDefinition, server: &AssetServer) {
    if let Some(scene) = game.content.presentations.iter().find(|p| p.id.0 == format!("actor.{model}")).and_then(|p| p.scene.as_ref()) {
        commands.entity(entity).insert(WorldAssetRoot(server.load(scene.clone())));
    }
}
fn actions(mut commands: Commands, time: Res<Time<Fixed>>, input: Res<GameInput>, mut actions: ResMut<TickActions>,
    mut state: ResMut<BuildableState>, game: Res<GameDefinition>, crafting: Res<Crafting>, server: Res<AssetServer>,
    mut players: Query<(Entity, &Player, &Transform, &Vitality, &mut BuildableInventory)>,
    mut enemies: Query<(Entity, &Transform, &Vitality, &mut crate::enemies::Zombie), Without<Player>>,
    mut damage: ResMut<DamageQueue>, query: SpatialQuery) {
    let dt = time.delta_secs();
    state.melee_cooldown = (state.melee_cooldown - dt).max(0.);
    state.drone_cooldown = (state.drone_cooldown - dt).max(0.);
    let Ok((owner, player, pose, health, mut equipment)) = players.single_mut() else { return; };
    if health.state() != LifeState::Standing { equipment.shield_held = false; return; }
    let forward = Quat::from_rotation_y(player.yaw) * Vec3::NEG_Z;
    if input.buildable {
        match equipment.equipped.as_deref() {
            Some("tomb_shield_zm") if equipment.shield_health > 0 => {
                if equipment.shield_held && input.use_held {
                    let position = pose.translation + forward * 1.2;
                    let entity = commands.spawn((DeployedShield, SessionEntity, Transform::from_translation(position).with_rotation(Quat::from_rotation_y(player.yaw)), Visibility::Inherited,
                        RigidBody::Static, Collider::cuboid(0.75, 1.2, 0.2), Vitality::ordinary_actor(equipment.shield_health, 1500).expect("shield health"))).id();
                    visual(&mut commands, entity, "t6_wpn_zmb_shield_dlc4_dmg0_world", &game, &server);
                    equipment.shield_health = 0;
                    equipment.shield_held = false;
                    equipment.owned.remove("tomb_shield_zm");
                    equipment.equipped = None;
                } else { equipment.shield_held = !equipment.shield_held; }
            }
            Some("equip_dieseldrone_zm") if state.drone.is_none() && state.drone_cooldown <= 0. => {
                let Some(home) = crafting.drone_home() else { return; };
                let entity = commands.spawn((Drone { owner, remaining: 90., returning: false, upgraded: state.drone_upgraded, home, cooldown: 0. },
                    SessionEntity, Transform::from_translation(pose.translation + Vec3::Y * 2.), Visibility::Inherited)).id();
                visual(&mut commands, entity, "veh_t6_dlc_zm_quadrotor", &game, &server);
                crate::audio::emit_loop(&mut commands, "veh_qrdrone_idle_loop", crate::audio::SoundSource::attached(entity));
                equipment.owned.remove("equip_dieseldrone_zm");
                equipment.equipped = None;
                state.drone = Some(entity);
            }
            _ => {}
        }
    }
    if equipment.shield_held {
        let attack = actions.fire || actions.melee;
        actions.fire = false;
        actions.fire_held = false;
        actions.melee = false;
        actions.ads = false;
        if attack && state.melee_cooldown <= 0. {
            state.melee_cooldown = 0.8;
            for (target, position, vitality, mut zombie) in &mut enemies {
                let delta = position.translation - pose.translation;
                if vitality.state() != LifeState::ActorAlive || delta.length() > 2.286 || delta.normalize_or_zero().dot(forward) < 0.4 { continue; }
                let Some(direction) = Dir3::new(delta).ok() else { continue; };
                if query.cast_ray(pose.translation, direction, (delta.length() - 0.35).max(0.), true, &crate::environment::solids([owner,target])).is_some() { continue; }
                damage.0.push(Impact { attacker: owner, target, source: ImpactSource::Weapon(WeaponId("weapon.tomb_shield_zm".into())), category: DamageCategory::Melee, region: HitRegion::Torso, amount: 75 });
                zombie.stunned = zombie.stunned.max(0.8);
                equipment.shield_health = equipment.shield_health.saturating_sub(100);
            }
            if equipment.shield_health == 0 { equipment.owned.remove("tomb_shield_zm"); equipment.shield_held = false; }
        }
    }
}
fn shield_targets(mut commands: Commands, shields: Query<(Entity, &Transform, &Vitality), With<DeployedShield>>,
    enemies: Query<(Entity, &Transform, &crate::rounds::PopulationKind, Option<&crate::enemies::EnemyGoal>, Has<ShieldAggro>), With<crate::enemies::Zombie>>) {
    for (entity, pose, kind, goal, assigned) in &enemies {
        if *kind != crate::rounds::PopulationKind::Ordinary || (goal.is_some() && !assigned) { continue; }
        let selected = shields.iter().filter(|(_, position, health)| health.state() == LifeState::ActorAlive && position.translation.distance_squared(pose.translation) < 12. * 12.)
            .min_by(|(_, a, _), (_, b, _)| a.translation.distance_squared(pose.translation).total_cmp(&b.translation.distance_squared(pose.translation)));
        if let Some((shield, position, _)) = selected {
            commands.entity(entity).insert((ShieldAggro(shield), crate::enemies::EnemyGoal { position: position.translation, face: position.translation, radius: 1.65 }));
        } else if assigned { commands.entity(entity).remove::<(ShieldAggro, crate::enemies::EnemyGoal)>(); }
    }
}
fn drones(mut commands: Commands, time: Res<Time<Fixed>>, game: Res<GameDefinition>, query: SpatialQuery,
    players: Query<(&Player, &Transform, &Vitality), Without<Drone>>,
    enemies: crate::hit_regions::RayTargets, mut drones: Query<(Entity, &mut Drone, &mut Transform), Without<Vitality>>,
    mut damage: ResMut<DamageQueue>, mut state: ResMut<BuildableState>) {
    let dt = time.delta_secs();
    for (entity, mut drone, mut pose) in &mut drones {
        drone.remaining -= dt;
        drone.cooldown = (drone.cooldown - dt).max(0.);
        let player = players.get(drone.owner).ok();
        drone.returning |= drone.remaining <= 0. || player.is_none_or(|(_, _, health)| health.state() != LifeState::Standing);
        let goal = if drone.returning { drone.home + Vec3::Y }
            else { let (body, pose, _) = player.unwrap(); pose.translation + Quat::from_rotation_y(body.yaw) * Vec3::NEG_Z * 3.2512 + Vec3::Y * 3.2512 };
        let delta = goal - pose.translation;
        let step = delta.clamp_length_max(6. * dt);
        if let Ok(direction) = Dir3::new(step) {
            if query.cast_ray(pose.translation, direction, step.length(), true, &crate::environment::solids([])).is_none() { pose.translation += step; }
            else { pose.translation.y += dt * 2.; }
        }
        if drone.returning {
            if delta.length() < 0.5 || drone.remaining < -30. {
                commands.entity(entity).despawn(); state.drone = None; state.drone_cooldown = 60.;
            }
            continue;
        }
        if drone.cooldown > 0. { continue; }
        let weapon_id = if drone.upgraded { "weapon.quadrotorturret_upgraded_zm" } else { "weapon.quadrotorturret_zm" };
        let Some(weapon) = game.content.weapons.iter().find(|w| w.id.0 == weapon_id) else { continue; };
        let target = enemies.iter().filter(|(_, health, position, _)| health.state() == LifeState::ActorAlive && (2.4384..9.7536).contains(&position.translation.distance(pose.translation)))
            .filter_map(|(target, _, position, _)| {
                let direction = Dir3::new(position.translation - pose.translation).ok()?;
                let hit = crate::hit_regions::cast(&enemies, &query, drone.owner, pose.translation, direction, 9.7536)?;
                (hit.target == target).then_some(hit)
            }).min_by(|a,b| a.hit.distance.total_cmp(&b.hit.distance));
        if let Some(hit) = target {
            damage.0.push(Impact { attacker: drone.owner, target: hit.target, source: ImpactSource::Weapon(weapon.id.clone()), category: weapon.category, region: hit.hit.region, amount: crate::weapons::damage_at(weapon, hit.hit.distance, hit.hit.region) });
            drone.cooldown = weapon.fire_interval.max(0.05);
            crate::audio::binding(&mut commands, &game, &format!("{weapon_id}.fire"), crate::audio::SoundSource::attached(entity));
        }
    }
}
pub(crate) fn collect_shield(mut commands: Commands, input: Res<GameInput>, mut state: ResMut<BuildableState>,
    mut focus: ResMut<crate::interactions::InteractionFocus>, settings: Res<crate::input::ControlSettings>,
    shields: Query<(Entity, &Transform, &Vitality), With<DeployedShield>>,
    mut players: Query<(&Player, &Transform, &Vitality, &mut BuildableInventory)>) {
    if !input.use_held { state.use_latched = false; }
    if focus.target.is_some() { return; }
    let Ok((body, pose, health, mut equipment)) = players.single_mut() else { return; };
    if health.state() != LifeState::Standing { return; }
    let forward = Quat::from_rotation_y(body.yaw) * Vec3::NEG_Z;
    for (entity, position, vitality) in &shields {
        let delta = position.translation - pose.translation;
        if vitality.state() != LifeState::ActorAlive || delta.length() > 2. || delta.normalize_or_zero().dot(forward) < 0.85 { continue; }
        focus.target = Some(entity);
        focus.prompt = format!("Press {} to take Zombie Shield", settings.label(crate::input::Action::Interact));
        if input.use_held && !state.use_latched && equipment.shield_health == 0 {
            equipment.shield_health = vitality.current().unwrap_or(0);
            equipment.owned.insert("tomb_shield_zm".into()); equipment.equipped = Some("tomb_shield_zm".into());
            commands.entity(entity).despawn(); state.use_latched = true; focus.accepted += 1;
        }
        break;
    }
}
fn cleanup(mut commands: Commands, shields: Query<(Entity, &Vitality), With<DeployedShield>>) {
    for (entity, health) in &shields { if health.state() != LifeState::ActorAlive { commands.entity(entity).despawn(); } }
}

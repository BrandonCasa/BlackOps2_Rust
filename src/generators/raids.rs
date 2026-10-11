use super::{GeneratorState, Generators};
use crate::{assets::GameDefinition, damage::EnemyDeath, enemies::{EnemyGoal, Zombie}, gameplay::{Session, SessionEntity}, life::{LifeState, Vitality}, origins::ContentId, rounds::{Gait, PopulationKind}};
use bevy::prelude::*;

#[derive(Resource)]
pub struct GeneratorRaids {
    pub next_round: u32,
    pub target: Option<ContentId>,
    pub waves: u32,
    pub shutdowns: u32,
    pub rewards: u32,
    spawned: usize,
    killed: usize,
    spawn_clock: f32,
}
impl Default for GeneratorRaids {
    fn default() -> Self { Self { next_round: 10, target: None, waves: 0, shutdowns: 0, rewards: 0, spawned: 0, killed: 0, spawn_clock: 0. } }
}
#[derive(Component)]
pub struct GeneratorAttacker { slot: usize }
pub fn reset(mut raids: ResMut<GeneratorRaids>) { *raids = default(); }
fn origin(game: &GameDefinition, id: &ContentId) -> Option<Vec3> {
    let site = game.content.generators.iter().find(|g| g.id == *id)?;
    game.content.placements.iter().find(|p| p.id == site.placement).map(|p| Vec3::from_array(p.position))
}
pub fn tick(mut commands: Commands, time: Res<Time<Fixed>>, game: Res<GameDefinition>, mut session: ResMut<Session>,
    mut generators: ResMut<Generators>, mut raids: ResMut<GeneratorRaids>,
    navigation: Res<crate::navigation::Navigation>, access: Res<crate::access::ZoneAccess>, query: avian3d::prelude::SpatialQuery,
    attackers: Query<(Entity, &Vitality, &Transform, &GeneratorAttacker)>, actors: Query<&Vitality, With<Zombie>>) {
    let dt = time.delta_secs();
    for (id, site) in &mut generators.sites {
        if site.state == GeneratorState::Active && raids.target.as_ref() != Some(id) {
            site.progress = (site.progress + dt / 10.).min(1.);
        }
    }
    if !generators.raid_active && session.round >= raids.next_round && generators.active_count() >= 4 && !generators.busy() {
        let choices: Vec<_> = generators.sites.iter().filter(|(_, s)| s.state == GeneratorState::Active).map(|(id, _)| id.clone()).collect();
        raids.target = Some(choices[session.random() as usize % choices.len()].clone());
        raids.spawned = 0; raids.killed = 0; raids.spawn_clock = 0.; raids.waves += 1;
        raids.next_round = session.round + 3 + session.random() % 3;
        generators.raid_active = true;
        generators.retirement_pending = true;
    }
    if !generators.raid_active { return; }
    let alive = attackers.iter().filter(|(_, v, _, _)| v.state() == LifeState::ActorAlive).count();
    if raids.spawned == 4 && alive == 0 {
        raids.target = None; generators.raid_active = false; return;
    }
    let Some(target) = raids.target.clone() else { return; };
    let Some(center) = origin(&game, &target) else { return; };
    if !generators.powered(&target) {
        let next = generators.sites.iter().filter(|(_, s)| s.state == GeneratorState::Active)
            .filter_map(|(id, _)| origin(&game, id).map(|p| (id.clone(), p.distance_squared(center))))
            .min_by(|a, b| a.1.total_cmp(&b.1)).map(|(id, _)| id);
        raids.target = next;
        if raids.target.is_none() {
            for (entity, _, _, _) in &attackers { commands.entity(entity).despawn(); }
            generators.raid_active = false;
        }
        return;
    }
    for (entity, health, _, attacker) in &attackers {
        if health.state() != LifeState::ActorAlive { continue; }
        let offset = [Vec3::X, Vec3::NEG_X, Vec3::Z, Vec3::NEG_Z][attacker.slot % 4] * 1.3208;
        commands.entity(entity).insert(EnemyGoal { position: center + offset + Vec3::Y * 0.85, face: center + Vec3::Y * 0.85, radius: 0.762 });
    }
    if alive > 0 && let Some(site) = generators.sites.get_mut(&target) {
        site.progress = (site.progress - dt / 40.).max(0.);
        if site.progress <= 0. {
            site.state = GeneratorState::Disabled; site.age = 0.; site.purchaser = None;
            raids.shutdowns += 1;
            crate::audio::emit(&mut commands, "zmb_capturezone_donut_stop", crate::audio::SoundSource::World(center));
        }
    }
    raids.spawn_clock -= dt;
    if raids.spawned >= 4 || raids.spawn_clock > 0. || actors.iter().filter(|v| v.state() == LifeState::ActorAlive).count() >= game.rounds.actor_cap as usize { return; }
    raids.spawn_clock = 0.5;
    let Some(site) = game.content.generators.iter().find(|g| g.id == target) else { return; };
    let Some(base) = navigation.project_emergence(Vec3::from_array(site.emergence[raids.spawned]), &access) else { return; };
    let position = base + Vec3::Y * 0.85;
    let shape = crate::player::capsule(0.3, 1.7);
    if !query.shape_intersections(&shape, position, Quat::IDENTITY, &crate::environment::solids([])).is_empty() { return; }
    let variants: Vec<_> = game.content.enemies.iter().filter(|e| e.family == "crusader").collect();
    let Some(enemy) = variants.get(raids.spawned % variants.len().max(1)) else { return; };
    let health = game.rounds.health(session.round);
    let Ok(health) = Vitality::ordinary_actor(health, health) else { return; };
    let zombie = Zombie::new(position, enemy.id.clone(), Gait::Run, game.rounds.speed(Gait::Run));
    let zombie = if let Some(rise) = enemy.traversals.get("rise") { zombie.rising(rise) } else { zombie };
    let mut actor = commands.spawn((SessionEntity, zombie, shape, health, PopulationKind::GeneratorDefender,
        GeneratorAttacker { slot: raids.spawned }, Transform::from_translation(position), Visibility::Inherited, Name::new("Generator attacker")));
    if enemy.traversals.contains_key("rise") { actor.insert(avian3d::prelude::ColliderDisabled); }
    raids.spawned += 1;
}
pub fn deaths(mut commands: Commands, mut events: MessageReader<EnemyDeath>, attackers: Query<&Transform, With<GeneratorAttacker>>,
    mut raids: ResMut<GeneratorRaids>, generators: Res<Generators>, game: Res<GameDefinition>, server: Res<AssetServer>) {
    for event in events.read() {
        let Ok(pose) = attackers.get(event.0.impact.target) else { continue; };
        raids.killed += 1;
        if raids.killed == 4 && raids.spawned == 4 && generators.raid_active && raids.target.as_ref().is_some_and(|id| generators.powered(id)) {
            crate::environment::digging::spawn_powerup(&mut commands, &server, &game, "full_ammo", pose.translation);
            raids.rewards += 1;
        }
    }
}

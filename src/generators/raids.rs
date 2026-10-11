use super::{GeneratorState, Generators};
use crate::{
    assets::GameDefinition,
    damage::EnemyDeath,
    enemies::{EnemyGoal, Zombie},
    gameplay::{Session, SessionEntity},
    life::{LifeState, Vitality},
    origins::ContentId,
    rounds::{Gait, PopulationKind},
};
use bevy::prelude::*;

#[derive(Resource)]
pub struct GeneratorRaids {
    pub next_round: u32,
    pub target: Option<ContentId>,
    pub waves: u32,
    pub shutdowns: u32,
    pub rewards: u32,
    pub under_attack: bool,
    attacked: bool,
    warning: Option<Entity>,
    warning_clock: f32,
    spawned: usize,
    killed: usize,
    spawn_clock: f32,
}
impl Default for GeneratorRaids {
    fn default() -> Self {
        Self {
            next_round: 10,
            target: None,
            waves: 0,
            shutdowns: 0,
            rewards: 0,
            under_attack: false,
            attacked: false,
            warning: None,
            warning_clock: 0.,
            spawned: 0,
            killed: 0,
            spawn_clock: 0.,
        }
    }
}
#[derive(Component)]
pub struct GeneratorAttacker {
    slot: usize,
    target: Option<ContentId>,
    pub attacking: bool,
}
pub fn reset(mut raids: ResMut<GeneratorRaids>) {
    *raids = default();
}
fn origin(game: &GameDefinition, id: &ContentId) -> Option<Vec3> {
    let site = game.content.generators.iter().find(|g| g.id == *id)?;
    game.content
        .placements
        .iter()
        .find(|p| p.id == site.placement)
        .map(|p| Vec3::from_array(p.position))
}
pub fn tick(
    mut commands: Commands,
    time: Res<Time<Fixed>>,
    game: Res<GameDefinition>,
    mut session: ResMut<Session>,
    mut generators: ResMut<Generators>,
    mut raids: ResMut<GeneratorRaids>,
    navigation: Res<crate::navigation::Navigation>,
    access: Res<crate::access::ZoneAccess>,
    query: avian3d::prelude::SpatialQuery,
    mut rounds: MessageReader<crate::rounds::RoundStarted>,
    mut attackers: Query<(Entity, &Vitality, &Transform, &mut GeneratorAttacker)>,
    actors: Query<&Vitality, With<Zombie>>,
) {
    let dt = time.delta_secs();
    for (id, site) in &mut generators.sites {
        if site.state == GeneratorState::Active && raids.target.as_ref() != Some(id) {
            site.progress = (site.progress + dt / 10.).min(1.);
        }
    }
    let boundary = rounds.read().any(|event| event.round == session.round);
    if boundary
        && !generators.raid_active
        && session.round >= raids.next_round
        && generators.active_count() >= 4
        && !generators.busy()
    {
        let choices: Vec<_> = generators
            .sites
            .iter()
            .filter(|(_, s)| s.state == GeneratorState::Active)
            .map(|(id, _)| id.clone())
            .collect();
        raids.target = Some(choices[session.random() as usize % choices.len()].clone());
        raids.attacked = false;
        raids.under_attack = false;
        raids.spawned = 0;
        raids.killed = 0;
        raids.spawn_clock = 0.;
        raids.waves += 1;
        raids.next_round = session.round + 3 + session.random() % 3;
        generators.raid_active = true;
        generators.retirement_pending = true;
    }
    if !generators.raid_active {
        return;
    }
    let alive = attackers
        .iter()
        .filter(|(_, v, _, _)| v.state() == LifeState::ActorAlive)
        .count();
    if raids.spawned == 4 && alive == 0 {
        raids.target = None;
        generators.raid_active = false;
        raids.under_attack = false;
        if let Some(sound) = raids.warning.take() {
            commands.entity(sound).despawn();
        }
        return;
    }
    let Some(target) = raids.target.clone() else {
        return;
    };
    let Some(center) = origin(&game, &target) else {
        return;
    };
    if !generators.powered(&target) {
        let next = generators
            .sites
            .iter()
            .filter(|(_, s)| s.state == GeneratorState::Active)
            .filter_map(|(id, _)| {
                origin(&game, id).map(|p| (id.clone(), p.distance_squared(center)))
            })
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(id, _)| id);
        raids.target = next;
        raids.attacked = false;
        raids.under_attack = false;
        if let Some(sound) = raids.warning.take() {
            commands.entity(sound).despawn();
        }
        if raids.target.is_none() {
            for (entity, _, _, _) in &attackers {
                commands.entity(entity).despawn();
            }
            generators.raid_active = false;
        }
        return;
    }
    let mut claimed: std::collections::BTreeSet<_> = attackers
        .iter()
        .filter(|(_, v, _, a)| {
            v.state() == LifeState::ActorAlive && a.target.as_ref() == Some(&target)
        })
        .map(|(_, _, _, a)| a.slot)
        .collect();
    let number = game
        .content
        .generators
        .iter()
        .find(|g| g.id == target)
        .map_or(1, |g| g.number) as usize;
    let yaw = [270_f32, 90., 90., 270., 0., 90.][number - 1].to_radians();
    let forward = Vec3::new(yaw.cos(), 0., -yaw.sin());
    let right = Vec3::new(yaw.sin(), 0., yaw.cos());
    for (entity, health, _, mut attacker) in &mut attackers {
        if health.state() != LifeState::ActorAlive {
            continue;
        }
        if attacker.target.as_ref() != Some(&target) {
            let allowed = |i: usize| {
                !claimed.contains(&i)
                    && !(number == 1 && [5, 11].contains(&i))
                    && !(number == 2 && [4, 5, 6].contains(&i))
            };
            let group = if claimed.range(0..3).count() < 3 {
                0..3
            } else if claimed.range(4..7).count() < 1 {
                4..7
            } else if claimed.range(8..11).count() < 1 {
                8..11
            } else {
                0..11
            };
            let mut candidates: Vec<_> = group.filter(|i| allowed(*i)).collect();
            if candidates.is_empty() {
                candidates = (0..11).filter(|i| allowed(*i)).collect();
            }
            if candidates.is_empty() {
                continue;
            }
            attacker.slot = candidates[session.random() as usize % candidates.len()];
            claimed.insert(attacker.slot);
            attacker.target = Some(target.clone());
            attacker.attacking = false;
        }
        let pillar = match attacker.slot / 4 {
            1 => right * 4.318,
            2 => right * -4.318,
            _ => Vec3::ZERO,
        };
        let scale = if attacker.slot < 4 { 1.3208 } else { 0.8128 };
        let offset = [forward, right, -forward, -right][attacker.slot % 4] * scale;
        commands.entity(entity).insert(EnemyGoal {
            position: center + pillar + offset + Vec3::Y * 0.85,
            face: center + pillar + Vec3::Y * 0.85,
            radius: 0.762,
        });
    }
    raids.spawn_clock -= dt;
    if raids.spawned >= 4
        || raids.spawn_clock > 0.
        || actors
            .iter()
            .filter(|v| v.state() == LifeState::ActorAlive)
            .count()
            >= game.rounds.actor_cap as usize
    {
        return;
    }
    raids.spawn_clock = 0.5;
    let Some(site) = game.content.generators.iter().find(|g| g.id == target) else {
        return;
    };
    let Some(base) =
        navigation.project_emergence(Vec3::from_array(site.emergence[raids.spawned]), &access)
    else {
        return;
    };
    let position = base + Vec3::Y * 0.85;
    let shape = crate::player::capsule(0.3, 1.7);
    if !query
        .shape_intersections(
            &shape,
            position,
            Quat::IDENTITY,
            &crate::environment::solids([]),
        )
        .is_empty()
    {
        return;
    }
    let variants: Vec<_> = game
        .content
        .enemies
        .iter()
        .filter(|e| e.family == "crusader")
        .collect();
    let Some(enemy) = variants.get(raids.spawned % variants.len().max(1)) else {
        return;
    };
    let health = game.rounds.health(session.round);
    let Ok(health) = Vitality::ordinary_actor(health, health) else {
        return;
    };
    let zombie = Zombie::new(
        position,
        enemy.id.clone(),
        Gait::Sprint,
        game.rounds.speed(Gait::Sprint),
    );
    let zombie = if let Some(rise) = enemy.traversals.get("rise") {
        zombie.rising(rise)
    } else {
        zombie
    };
    let mut actor = commands.spawn((
        SessionEntity,
        zombie,
        shape,
        health,
        PopulationKind::GeneratorDefender,
        GeneratorAttacker {
            slot: usize::MAX,
            target: None,
            attacking: false,
        },
        Transform::from_translation(position),
        Visibility::Inherited,
        Name::new("Generator attacker"),
    ));
    if enemy.traversals.contains_key("rise") {
        actor.insert(avian3d::prelude::ColliderDisabled);
    }
    raids.spawned += 1;
}
pub fn deaths(
    mut commands: Commands,
    mut events: MessageReader<EnemyDeath>,
    attackers: Query<&Transform, With<GeneratorAttacker>>,
    mut raids: ResMut<GeneratorRaids>,
    generators: Res<Generators>,
    game: Res<GameDefinition>,
    server: Res<AssetServer>,
) {
    for event in events.read() {
        let Ok(pose) = attackers.get(event.0.impact.target) else {
            continue;
        };
        raids.killed += 1;
        if raids.killed == 4 && raids.spawned == 4 && generators.raid_active && raids.under_attack {
            crate::environment::digging::spawn_powerup(
                &mut commands,
                &server,
                &game,
                "full_ammo",
                pose.translation,
            );
            raids.rewards += 1;
        }
    }
}

pub(crate) fn attack_state(
    mut attackers: Query<(
        &Zombie,
        &Vitality,
        &Transform,
        &mut GeneratorAttacker,
        &EnemyGoal,
        &crate::combat_effects::Wounds,
    )>,
    raids: Res<GeneratorRaids>,
) {
    for (zombie, health, pose, mut attacker, goal, wounds) in &mut attackers {
        attacker.attacking = health.state() == LifeState::ActorAlive
            && attacker.target == raids.target
            && zombie.completed_emerging()
            && zombie.action == crate::origins::content::EnemyAnimation::Attack
            && zombie.stunned <= 0.
            && !wounds.melee_disabled()
            && pose.translation.distance_squared(goal.position) < (goal.radius + 0.3).powi(2);
    }
}
pub(crate) fn drain(
    mut commands: Commands,
    time: Res<Time<Fixed>>,
    game: Res<GameDefinition>,
    mut raids: ResMut<GeneratorRaids>,
    mut generators: ResMut<Generators>,
    attackers: Query<(&Vitality, &GeneratorAttacker)>,
) {
    let Some(target) = raids.target.clone() else {
        return;
    };
    let attacking = attackers
        .iter()
        .filter(|(v, a)| {
            v.state() == LifeState::ActorAlive && a.target.as_ref() == Some(&target) && a.attacking
        })
        .count();
    raids.under_attack = attacking > 0;
    raids.attacked |= raids.under_attack;
    let Some(center) = origin(&game, &target) else {
        return;
    };
    if raids.under_attack && raids.warning.is_none() {
        raids.warning = Some(crate::audio::emit_loop(
            &mut commands,
            "zmb_capturezone_losing",
            crate::audio::SoundSource::World(center),
        ));
        raids.warning_clock = 0.;
    }
    if raids.under_attack {
        raids.warning_clock -= time.delta_secs();
        if raids.warning_clock <= 0. {
            raids.warning_clock += 4.;
            crate::native_fx::emit(
                &mut commands,
                "maps/zombie/fx_zmb_tranzit_marker_fl",
                Transform::from_translation(center),
            );
        }
    }
    if !raids.under_attack
        && let Some(sound) = raids.warning.take()
    {
        commands.entity(sound).despawn();
    }
    if let Some(site) = generators.sites.get_mut(&target)
        && site.state == GeneratorState::Active
    {
        let rate = if raids.under_attack {
            -1. / 40.
        } else if raids.attacked {
            1. / 10.
        } else {
            0.
        };
        site.progress = (site.progress + time.delta_secs() * rate).clamp(0., 1.);
        if site.progress <= 0. {
            site.state = GeneratorState::Disabled;
            site.age = 0.;
            site.purchaser = None;
            raids.shutdowns += 1;
            crate::audio::emit(
                &mut commands,
                "zmb_capturezone_donut_stop",
                crate::audio::SoundSource::World(center),
            );
        }
    }
}

use crate::{
    GameState,
    assets::GameDefinition,
    gameplay::{Session, SessionEntity},
    player::Player,
};
use bevy::prelude::*;
use std::hash::BuildHasher;

#[derive(Resource, Default, Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
pub(crate) struct SessionId(pub u64);

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum SessionSetup {
    Initialize,
    World,
    Player,
}

pub(crate) struct SessionPlugin;
impl Plugin for SessionPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SessionId>()
            .configure_sets(
                OnEnter(GameState::PreparingShaders),
                (
                    SessionSetup::Initialize,
                    SessionSetup::World,
                    SessionSetup::Player,
                )
                    .chain(),
            )
            .add_systems(
                OnEnter(GameState::PreparingShaders),
                initialize.in_set(SessionSetup::Initialize),
            );
    }
}

fn initialize(world: &mut World) {
    if world
        .query_filtered::<Entity, With<Player>>()
        .iter(world)
        .next()
        .is_some()
    {
        return;
    }
    teardown(world);
    let next = world
        .resource::<SessionId>()
        .0
        .checked_add(1)
        .expect("Session identity exhausted");
    world.insert_resource(SessionId(next));
    let definition = world.resource::<GameDefinition>();
    let mut session = Session {
        difficulty: *world.resource::<crate::frontend::Difficulty>(),
        character: definition.audio.character,
        pending: definition.rounds.population(1),
        spawn_timer: Timer::from_seconds(definition.rounds.first_spawn_seconds, TimerMode::Once),
        ..default()
    };
    if world
        .resource::<crate::assets::LaunchOptions>()
        .verify
        .is_none()
    {
        session.seed = std::collections::hash_map::RandomState::new().hash_one(std::process::id());
        session.character = (session.seed % 4) as usize;
    }
    world.insert_resource(session);
    world.insert_resource(crate::quests::MainQuest::reset(*world.resource::<crate::frontend::Difficulty>()));
    world.insert_resource(crate::damage::CombatHistory::default());
    world.insert_resource(crate::powerups::PowerupEffects::default());
    world.insert_resource(crate::powerups::Drops::default());
    world.insert_resource(crate::generators::Generators::default());
    world.insert_resource(crate::rounds::Population::default());
    world.insert_resource(crate::economy::PointLedger::default());
    crate::environment::weather::reset(world);
    world.insert_resource(crate::environment::parts::Parts::default());
    world.insert_resource(crate::environment::digging::Digging::default());
    world.insert_resource(crate::environment::support::EnvironmentEvidence::default());
}

pub(crate) fn teardown(world: &mut World) {
    let entities: Vec<_> = world
        .query_filtered::<Entity, With<SessionEntity>>()
        .iter(world)
        .collect();
    for entity in entities {
        if let Ok(entity) = world.get_entity_mut(entity) {
            entity.despawn();
        }
    }
    world
        .resource_mut::<crate::life::LifeCommandQueue>()
        .clear();
    world
        .resource_mut::<crate::life::LifeCommandResults>()
        .drain()
        .for_each(drop);
    world.resource_mut::<crate::damage::DamageQueue>().0.clear();
    world
        .resource_mut::<crate::equipment::BlastQueue>()
        .0
        .clear();
    world.insert_resource(crate::interactions::InteractionFocus::default());
    world.insert_resource(crate::input::GameInput::default());
    world.insert_resource(crate::player::TickActions::default());
    clear::<crate::damage::DamageApplied>(world);
    clear::<crate::damage::EnemyDeath>(world);
    clear::<crate::player_health::PlayerDamage>(world);
    clear::<crate::player_health::PlayerDeath>(world);
    clear::<crate::solo_revive::PlayerDowned>(world);
    clear::<crate::perk_effects::ReloadEvent>(world);
    clear::<crate::movement::Landing>(world);
    clear::<crate::environment::Footstep>(world);
    clear::<crate::input::RestartGame>(world);
    world.insert_resource(crate::lures::Attractions::default());
    clear::<crate::rounds::RoundStarted>(world);
    clear::<crate::rounds::RoundEnding>(world);
    clear::<crate::environment::support::EnvironmentalHit>(world);
    clear::<crate::quests::SoulCollected>(world);
    clear::<crate::projectiles::ProjectileImpact>(world);
    clear::<crate::robots::RobotStomp>(world);
}

fn clear<M: Message>(world: &mut World) {
    world.resource_mut::<Messages<M>>().clear();
}

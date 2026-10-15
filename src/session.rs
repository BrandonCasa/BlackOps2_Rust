use crate::{
    GameState,
    assets::GameDefinition,
    gameplay::{Session, SessionEntity},
    player::Player,
};
use bevy::{
    prelude::*,
    world_serialization::{WorldAssetRoot, WorldInstance, WorldInstanceSpawner},
};
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
                OnEnter(GameState::StartingSession),
                (
                    SessionSetup::Initialize,
                    SessionSetup::World,
                    SessionSetup::Player,
                )
                    .chain(),
            )
            .add_systems(
                OnEnter(GameState::StartingSession),
                initialize.in_set(SessionSetup::Initialize),
            )
            .add_systems(Update, finish_starting_session.run_if(in_state(GameState::StartingSession)));
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
        spawn_timer: Timer::from_seconds(definition.rounds.initial_spawn_seconds, TimerMode::Once),
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
    world.insert_resource(
        if world.resource::<GameDefinition>().delivery_scope
            == crate::origins::DeliveryScope::FirstPlayableSolo
        {
            crate::quests::MainQuest::default()
        } else {
            crate::quests::MainQuest::reset(*world.resource::<crate::frontend::Difficulty>())
        },
    );
    world.insert_resource(crate::damage::CombatHistory::default());
    world.insert_resource(crate::powerups::PowerupEffects::default());
    world.insert_resource(crate::powerups::Drops::default());
    world.insert_resource(crate::rounds::Population::default());
    world.insert_resource(crate::economy::PointLedger::default());
    crate::environment::weather::reset(world);
    world.insert_resource(crate::environment::parts::Parts::default());
    world.insert_resource(crate::environment::digging::Digging::default());
    world.insert_resource(crate::environment::support::EnvironmentEvidence::default());
    world.insert_resource(crate::movement::MovementEvidence::default());
    world.insert_resource(crate::equipment::EquipmentEvidence::default());
}

pub(crate) fn teardown(world: &mut World) {
    let entities: Vec<_> = world
        .query_filtered::<Entity, With<SessionEntity>>()
        .iter(world)
        .collect();
    let owners: std::collections::HashSet<_> = entities.iter().copied().collect();
    let instances: Vec<_> = world
        .query::<(Entity, &WorldInstance)>()
        .iter(world)
        .filter_map(|(entity, instance)| {
            let mut ancestor = Some(entity);
            while let Some(current) = ancestor {
                if owners.contains(&current) {
                    return Some((entity, **instance));
                }
                ancestor = world.get::<ChildOf>(current).map(ChildOf::parent);
            }
            None
        })
        .collect();
    for (entity, instance) in instances {
        world
            .entity_mut(entity)
            .remove::<WorldAssetRoot>();
        world
            .resource_mut::<WorldInstanceSpawner>()
            .unregister_instance(instance);
    }
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
    crate::solo_revive::reset(world);
    crate::enemies::reset(world);
    crate::generators::reset(world);
    world.resource_mut::<crate::damage::DamageQueue>().0.clear();
    world
        .resource_mut::<crate::equipment::BlastQueue>()
        .0
        .clear();
    world.insert_resource(crate::interactions::InteractionFocus::default());
    crate::input::reset(world);
    world.insert_resource(crate::player::TickActions::default());
    world.insert_resource(crate::access::CurrentZone::default());
    world.insert_resource(crate::access::PlayerZoneContacts::default());
    world.insert_resource(crate::access::ZoneScan::default());
    clear::<crate::access::NewZoneActive>(world);
    clear::<crate::damage::DamageApplied>(world);
    clear::<crate::damage::EnemyDeath>(world);
    clear::<crate::player_health::PlayerDamage>(world);
    clear::<crate::player_health::PlayerDeath>(world);
    clear::<crate::solo_revive::PlayerDowned>(world);
    clear::<crate::perk_effects::ReloadEvent>(world);
    clear::<crate::movement::Landing>(world);
    clear::<crate::environment::Footstep>(world);
    clear::<crate::input::RestartGame>(world);
    clear::<crate::frontend::FrontendAction>(world);
    world.insert_resource(crate::lures::Attractions::default());
    clear::<crate::rounds::RoundStarted>(world);
    clear::<crate::rounds::RoundEnding>(world);
    clear::<crate::environment::support::EnvironmentalHit>(world);
    clear::<crate::access::ZoneFlagSet>(world);
    crate::environment::triggers::reset(world);
    clear::<crate::quests::SoulCollected>(world);
    clear::<crate::projectiles::ProjectileImpact>(world);
    if world
        .get_resource::<GameDefinition>()
        .is_none_or(|game| game.delivery_scope == crate::origins::DeliveryScope::FullOrigins)
    {
        clear::<crate::robots::RobotStomp>(world);
        clear::<crate::tablets::BeaconStrike>(world);
    }
}

fn clear<M: Message>(world: &mut World) {
    world.resource_mut::<Messages<M>>().clear();
}


fn finish_starting_session(
    player: Query<(), With<Player>>,
    scenes: Query<(&WorldAssetRoot, Option<&WorldInstance>)>,
    server: Res<AssetServer>,
    rigs: Query<&crate::presentation::Rig>,
    textures: Query<&crate::presentation::camo::CamoTexture>,
    definition: Res<GameDefinition>,
    cinematic: Res<crate::cinematics::LoadingPlayback>,
    mut failure: ResMut<crate::assets::LoadFailure>,
    mut next: ResMut<NextState<GameState>>,
) {
    if !failure.0.is_empty() {
        next.set(GameState::LoadError);
        return;
    }
    // Inspect every requested scene for failures before waiting for readiness.
    let mut scenes_ready = true;
    for (scene, instance) in &scenes {
        let error = match server.get_load_state(scene.0.id()) {
            Some(bevy::asset::LoadState::Failed(error)) => Some(error),
            _ => match server.get_recursive_dependency_load_state(scene.0.id()) {
                Some(bevy::asset::RecursiveDependencyLoadState::Failed(error)) => Some(error),
                _ => None,
            },
        };
        if let Some(error) = error {
            failure.0 = format!("Required scene {:?}: {error}", scene.0.path());
            next.set(GameState::LoadError);
            return;
        }
        scenes_ready &= instance.is_some() && server.is_loaded_with_dependencies(scene.0.id());
    }
    let textures_ready = match crate::presentation::camo::assets_ready(&server, &textures) {
        Ok(ready) => ready,
        Err(error) => {
            failure.0 = error;
            next.set(GameState::LoadError);
            return;
        }
    };
    if definition.delivery_scope == crate::origins::DeliveryScope::FullOrigins {
        if !definition.content.cinematics.iter().any(|sequence| {
            (sequence.id.0.contains("loading") || sequence.id.0.contains("intro"))
                && !sequence.frames.is_empty()
        }) {
            failure.0 = "Required Origins loading cinematic is missing".into();
            next.set(GameState::LoadError);
            return;
        }
        if !cinematic.done { return; }
    }
    if !scenes_ready || !textures_ready || player.is_empty() || rigs.is_empty()
        || rigs.iter().any(|rig| !rig.ready || (!rig.arms && !rig.attached)) {
        return;
    }
    next.set(GameState::Playing);
}

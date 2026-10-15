use crate::{
    GameState,
    assets::{GameDefinition, LoadFailure},
    gameplay::SessionEntity,
};
use bevy::prelude::*;
#[derive(Component)]
struct Effect {
    name: String,
}
#[derive(Component)]
struct Bound;
#[derive(Component)]
struct Lifetime(Timer);
pub(crate) struct NativeFxPlugin;
impl Plugin for NativeFxPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (bind, expire).chain().run_if(in_state(GameState::Playing)),
        );
    }
}
pub(crate) fn emit(
    commands: &mut Commands,
    name: impl Into<String>,
    transform: Transform,
) -> Entity {
    commands
        .spawn((
            SessionEntity,
            Effect { name: name.into() },
            transform,
            Visibility::default(),
        ))
        .id()
}
pub(crate) fn attach(
    commands: &mut Commands,
    name: impl Into<String>,
    owner: Entity,
    offset: Transform,
) -> Entity {
    let entity = commands
        .spawn((
            SessionEntity,
            Effect { name: name.into() },
            offset,
            Visibility::default(),
        ))
        .id();
    commands.entity(owner).add_child(entity);
    entity
}
fn bind(
    mut commands: Commands,
    game: Res<GameDefinition>,
    server: Res<AssetServer>,
    effects: Query<(Entity, &Effect), Without<Bound>>,
    mut failure: ResMut<LoadFailure>,
    mut next: ResMut<NextState<GameState>>,
) {
    for (entity, effect) in &effects {
        let Some(path) = game.effects.get(effect.name.trim_start_matches(',')) else {
            failure.0 = format!("Missing exported effect scene: {}", effect.name);
            next.set(GameState::LoadError);
            return;
        };
        let key = effect.name.trim_start_matches(',');
        if let Some(seconds) = game.effect_lifetimes.get(key) {
            if !seconds.is_finite() || *seconds <= 0. {
                failure.0 = format!(
                    "Invalid source lifetime for exported effect: {}",
                    effect.name
                );
                next.set(GameState::LoadError);
                return;
            }
            commands
                .entity(entity)
                .insert(Lifetime(Timer::from_seconds(*seconds, TimerMode::Once)));
        } else if !game.looping_effects.contains(key) {
            failure.0 = format!(
                "Missing source lifetime or looping classification for exported effect: {}",
                effect.name
            );
            next.set(GameState::LoadError);
            return;
        }
        commands.entity(entity).insert((
            WorldAssetRoot(server.load(path.clone())),
            Bound,
            Name::new(format!("Effect {}", effect.name)),
        ));
    }
}

fn expire(
    mut commands: Commands,
    time: Res<Time>,
    server: Res<AssetServer>,
    mut effects: Query<(Entity, &WorldAssetRoot, Option<&mut Lifetime>), With<Effect>>,
    mut failure: ResMut<LoadFailure>,
    mut next: ResMut<NextState<GameState>>,
) {
    for (entity, root, life) in &mut effects {
        match server.get_recursive_dependency_load_state(root.0.id()) {
            Some(bevy::asset::RecursiveDependencyLoadState::Failed(error)) => {
                failure.0 = format!("Exported effect dependencies failed: {error}");
                next.set(GameState::LoadError);
                return;
            }
            Some(bevy::asset::RecursiveDependencyLoadState::Loaded) => {}
            _ => continue,
        }
        if let Some(mut life) = life {
            if life.0.tick(time.delta()).is_finished() {
                commands.entity(entity).despawn();
            }
        }
    }
}

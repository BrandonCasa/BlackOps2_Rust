use std::{
    ffi::OsString,
    path::{Path, PathBuf},
    time::Instant,
};

use avian3d::prelude::*;
use bevy::{
    asset::{LoadState, RecursiveDependencyLoadState},
    prelude::*,
    world_serialization::{WorldAsset, WorldAssetRoot, WorldInstanceReady},
};
use serde::Deserialize;

use crate::GameState;

#[derive(Resource, Default)]
pub struct LaunchOptions {
    pub config: Option<PathBuf>,
    pub verify: Option<PathBuf>,
    pub error: Option<String>,
}

impl LaunchOptions {
    pub fn parse(mut args: impl Iterator<Item = OsString>) -> Self {
        let mut result = Self::default();
        while let Some(argument) = args.next() {
            match argument.to_str() {
                Some("--config") => result.config = args.next().map(PathBuf::from),
                Some("--verify") => result.verify = args.next().map(PathBuf::from),
                _ => {
                    result.error = Some(
                        "Usage: origins [--config file.toml] [--verify external-output-folder]"
                            .into(),
                    )
                }
            }
            if matches!(argument.to_str(), Some("--config" | "--verify"))
                && (argument == "--config" && result.config.is_none()
                    || argument == "--verify" && result.verify.is_none())
            {
                result.error = Some("Missing command-line argument value.".into());
            }
        }
        result
    }
}

#[derive(Debug)]
pub struct LocalConfig {
    pub pack_root: PathBuf,
}

#[derive(Deserialize)]
struct ConfigFile {
    game: GameConfig,
}
#[derive(Deserialize)]
struct GameConfig {
    asset_pack: PathBuf,
}

impl LocalConfig {
    pub fn discover(explicit: Option<&Path>) -> Result<Self, String> {
        let executable = std::env::current_exe().map_err(|e| e.to_string())?;
        let beside = executable
            .parent()
            .unwrap_or(Path::new("."))
            .join("config/local_assets.toml");
        let path = explicit.map(Path::to_path_buf).unwrap_or_else(|| {
            if beside.is_file() {
                beside
            } else {
                PathBuf::from("config/local_assets.toml")
            }
        });
        let text = std::fs::read_to_string(&path).map_err(|_| "Local asset configuration is missing. Prepare your licensed asset pack and set [game] asset_pack in config/local_assets.toml.".to_string())?;
        let config: ConfigFile = toml::from_str(text.trim_start_matches('\u{feff}'))
            .map_err(|e| format!("Invalid local configuration: {e}"))?;
        let pack_root = config.game.asset_pack.canonicalize().map_err(|_| {
            "Configured asset pack does not exist. Run the preparation command first.".to_string()
        })?;
        if pack_root.starts_with(Path::new(env!("CARGO_MANIFEST_DIR"))) {
            return Err("Licensed assets must remain outside the repository.".into());
        }
        Ok(Self { pack_root })
    }
}

#[derive(Resource)]
pub struct LoadConfiguration(pub Result<LocalConfig, String>);
#[derive(Resource, Default)]
pub struct LoadFailure(pub String);

pub use crate::origins::{GameDefinition, Tuning};
#[derive(Component)]
struct MapRoot {
    collision: bool,
}
#[derive(Component)]
pub struct WorldCollider;
#[derive(Resource)]
struct Loading {
    handles: Vec<Handle<WorldAsset>>,
    ready: usize,
    colliders: usize,
    started: Instant,
    ready_at: Option<Instant>,
}

pub struct GameAssetsPlugin;
impl Plugin for GameAssetsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<LoadFailure>()
            .add_systems(Startup, begin_load)
            .add_observer(scene_ready)
            .add_systems(Update, finish_load.run_if(in_state(GameState::Loading)));
    }
}

fn begin_load(
    mut commands: Commands,
    config: Res<LoadConfiguration>,
    options: Res<LaunchOptions>,
    server: Res<AssetServer>,
    mut failure: ResMut<LoadFailure>,
    mut next: ResMut<NextState<GameState>>,
) {
    let definition = (|| {
        if let Some(error) = &options.error {
            return Err(error.clone());
        }
        let config = config.0.as_ref().map_err(Clone::clone)?;
        let path = config.pack_root.join("game.json");
        if std::fs::metadata(&path).map_err(|e| e.to_string())?.len() > 4 * 1024 * 1024 {
            return Err("Gameplay definition is too large.".into());
        }
        let definition: GameDefinition =
            serde_json::from_slice(&std::fs::read(path).map_err(|e| e.to_string())?)
                .map_err(|e| e.to_string())?;
        definition.validate()?;
        Ok(definition)
    })();
    match definition {
        Ok(definition) => {
            let mut handles = Vec::new();
            for (path, collision) in [
                (&definition.scene, false),
                (&definition.collision_scene, true),
            ] {
                let handle: Handle<WorldAsset> = server.load(path.clone());
                commands.spawn((
                    WorldAssetRoot(handle.clone()),
                    MapRoot { collision },
                    if collision {
                        Visibility::Hidden
                    } else {
                        Visibility::Inherited
                    },
                ));
                handles.push(handle);
            }
            commands.insert_resource(definition);
            commands.insert_resource(Loading {
                handles,
                ready: 0,
                colliders: 0,
                started: Instant::now(),
                ready_at: None,
            });
        }
        Err(error) => {
            failure.0 = error;
            next.set(GameState::LoadError);
        }
    }
}

fn scene_ready(
    event: On<WorldInstanceReady>,
    roots: Query<&MapRoot>,
    children: Query<&Children>,
    mesh_entities: Query<&Mesh3d>,
    meshes: Res<Assets<Mesh>>,
    mut loading: ResMut<Loading>,
    mut commands: Commands,
    mut failure: ResMut<LoadFailure>,
    mut next: ResMut<NextState<GameState>>,
    mut cache: Local<std::collections::HashMap<AssetId<Mesh>, Collider>>,
) {
    let Ok(root) = roots.get(event.entity) else {
        return;
    };
    if root.collision {
        for entity in children.iter_descendants(event.entity) {
            if let Ok(mesh) = mesh_entities.get(entity) {
                let collider = cache.get(&mesh.0.id()).cloned().or_else(|| {
                    let collider = meshes.get(&mesh.0).and_then(Collider::trimesh_from_mesh)?;
                    cache.insert(mesh.0.id(), collider.clone());
                    Some(collider)
                });
                if let Some(collider) = collider {
                    commands
                        .entity(entity)
                        .insert((RigidBody::Static, collider, WorldCollider));
                    loading.colliders += 1;
                } else {
                    error!("Could not build collision for {:?}", mesh.0.path());
                    failure.0 = "A map collision mesh could not be constructed.".into();
                    next.set(GameState::LoadError);
                }
            }
        }
    }
    loading.ready += 1;
}

fn finish_load(
    loading: Option<ResMut<Loading>>,
    definition: Option<Res<GameDefinition>>,
    server: Res<AssetServer>,
    mut failure: ResMut<LoadFailure>,
    mut next: ResMut<NextState<GameState>>,
) {
    let (Some(mut loading), Some(definition)) = (loading, definition) else {
        return;
    };
    for handle in &loading.handles {
        if matches!(
            server.get_load_state(handle.id()),
            Some(LoadState::Failed(_))
        ) || matches!(
            server.get_recursive_dependency_load_state(handle.id()),
            Some(RecursiveDependencyLoadState::Failed(_))
        ) {
            failure.0 = "A scene or texture failed to load. Re-run asset preparation and inspect the application log.".into();
            next.set(GameState::LoadError);
            return;
        }
    }
    if loading.ready == 2 && loading.colliders == definition.collision_meshes {
        if loading
            .ready_at
            .get_or_insert_with(Instant::now)
            .elapsed()
            .as_secs_f32()
            < 0.1
        {
            return;
        }
        info!(
            "Origins scene ready: {} source surfaces, {} colliders, {} static instances",
            definition.source_surfaces, loading.colliders, definition.static_model_instances
        );
        next.set(GameState::Playing);
    } else if loading.started.elapsed().as_secs() > 120 {
        failure.0 = "Scene loading timed out or collision coverage is incomplete.".into();
        next.set(GameState::LoadError);
    }
}

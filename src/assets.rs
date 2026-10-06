use std::{ffi::OsString, path::PathBuf, time::Instant};

use avian3d::prelude::*;
use bevy::{
    asset::{LoadState, RecursiveDependencyLoadState},
    prelude::*,
    world_serialization::{WorldAsset, WorldAssetRoot, WorldInstanceReady},
};

use crate::GameState;

#[derive(Resource, Default)]
pub struct LaunchOptions {
    pub verify: Option<PathBuf>,
    pub benchmark: bool,
    pub benchmark_view: Option<usize>,
    pub benchmark_motion: bool,
    pub benchmark_combat: bool,
    pub uncached_shadows: bool,
    pub error: Option<String>,
}

impl LaunchOptions {
    pub fn parse(mut args: impl Iterator<Item = OsString>) -> Self {
        let mut result = Self::default();
        if let Some(argument) = args.next() {
            if argument == "--verify"
                || argument == "--benchmark"
                || argument == "--benchmark-view"
                || argument == "--benchmark-moving-view"
                || argument == "--benchmark-combat"
            {
                result.benchmark = argument != "--verify";
                result.verify = args.next().map(PathBuf::from);
                result.benchmark_motion = argument == "--benchmark-moving-view";
                result.benchmark_combat = argument == "--benchmark-combat";
                if result.benchmark_combat {
                    result.benchmark_view = Some(8);
                }
                if argument == "--benchmark-view" || result.benchmark_motion {
                    result.benchmark_view =
                        args.next().and_then(|value| value.to_str()?.parse().ok());
                    if result.benchmark_view.is_none() {
                        result.error = Some(
                            "View benchmarks require an external folder and a view index".into(),
                        );
                    }
                }
            }
            let remaining = args.next();
            if result.benchmark
                && remaining.as_deref() == Some(std::ffi::OsStr::new("--uncached-shadows"))
            {
                result.uncached_shadows = true;
            } else if remaining.is_some() {
                result.error = Some("Unexpected launch argument".into());
            }
            if result.verify.is_none() || args.next().is_some() {
                result.error =
                    Some("Usage: origins [--verify|--benchmark external-output-folder] or [--benchmark-view|--benchmark-moving-view external-output-folder index]".into());
            }
        }
        result
    }
}

#[derive(Debug)]
pub struct AssetPack {
    pub pack_root: PathBuf,
}

impl AssetPack {
    pub fn discover() -> Result<Self, String> {
        let executable = std::env::current_exe().map_err(|e| e.to_string())?;
        let directory = executable
            .parent()
            .ok_or("Executable directory is unavailable.")?;
        let beside = directory.join("assets/origins");
        let pack_root = if beside.exists() {
            beside
        } else if let Some(repository) = directory
            .ancestors()
            .find(|p| p.join("Cargo.toml").is_file() && p.join("src/main.rs").is_file())
        {
            repository.join("assets/origins")
        } else {
            beside
        };
        let pack_root = pack_root.canonicalize().map_err(|e| {
            format!("Cannot open asset pack {}: {e}\nRestore assets/origins beside origins.exe, or run git lfs pull in the checkout.", pack_root.display())
        })?;
        if !pack_root.join("game.json").is_file() {
            return Err(format!(
                "Missing {}\nRestore the complete assets/origins directory.",
                pack_root.join("game.json").display()
            ));
        }
        Ok(Self { pack_root })
    }
}

#[derive(Resource)]
pub struct LoadConfiguration(pub Result<AssetPack, String>);
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
        info!("Origins asset pack: {}", config.pack_root.display());
        let path = config.pack_root.join("game.json");
        if std::fs::metadata(&path)
            .map_err(|e| format!("{}: {e}", path.display()))?
            .len()
            > 4 * 1024 * 1024
        {
            return Err("Gameplay definition is too large.".into());
        }
        let definition: GameDefinition = serde_json::from_slice(
            &std::fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))?,
        )
        .map_err(|e| {
            format!(
                "Invalid {}: {e}\nRestore game.json from the runtime pack.",
                path.display()
            )
        })?;
        definition
            .validate()
            .map_err(|e| format!("Invalid {}: {e}", path.display()))?;
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
                    // Reveal render geometry only after material/lightmap assembly.
                    // Switching lightmap layouts on already queued scene instances can
                    // otherwise leave a transient stale Bevy pipeline specialization.
                    Visibility::Hidden,
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
            error!("Unable to load Origins: {error}");
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
    mut commands: Commands,
    roots: Query<(Entity, &MapRoot)>,
    surfaces: Res<crate::rendering::OpaqueSurfaceStatus>,
    loading: Option<ResMut<Loading>>,
    definition: Option<Res<GameDefinition>>,
    config: Res<LoadConfiguration>,
    server: Res<AssetServer>,
    mut failure: ResMut<LoadFailure>,
    mut next: ResMut<NextState<GameState>>,
) {
    let (Some(mut loading), Some(definition)) = (loading, definition) else {
        return;
    };
    for handle in &loading.handles {
        let error = match server.get_load_state(handle.id()) {
            Some(LoadState::Failed(error)) => Some(error),
            _ => match server.get_recursive_dependency_load_state(handle.id()) {
                Some(RecursiveDependencyLoadState::Failed(error)) => Some(error),
                _ => None,
            },
        };
        if let Some(error) = error {
            let root = config
                .0
                .as_ref()
                .map(|c| c.pack_root.display().to_string())
                .unwrap_or_default();
            failure.0 = format!(
                "Asset pack: {root}\nScene {:?}: {error}\nRestore the complete pack (map.gltf, map.bin and textures).\nFor a Git checkout, run git lfs pull; for a release, copy assets/origins beside origins.exe.",
                handle.path()
            );
            error!("Unable to load Origins: {}", failure.0);
            next.set(GameState::LoadError);
            return;
        }
    }
    if loading.ready == 2 && loading.colliders == definition.collision_meshes {
        if loading.started.elapsed().as_secs() > 120 {
            failure.0 = "Opaque material assembly timed out.".into();
            next.set(GameState::LoadError);
            return;
        }
        if !surfaces.ready() {
            return;
        }
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
        for (entity, root) in &roots {
            if !root.collision {
                commands.entity(entity).insert(Visibility::Inherited);
            }
        }
        next.set(GameState::Playing);
    } else if loading.started.elapsed().as_secs() > 120 {
        failure.0 = "Scene loading timed out or collision coverage is incomplete.".into();
        next.set(GameState::LoadError);
    }
}

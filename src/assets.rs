use std::{ffi::OsString, path::PathBuf, time::Instant};

use avian3d::prelude::*;
use bevy::{
    asset::{LoadState, RecursiveDependencyLoadState},
    prelude::*,
    world_serialization::{WorldAsset, WorldAssetRoot, WorldInstanceReady},
};

use crate::GameState;

pub(crate) mod prepared;

#[derive(Resource, Default)]
pub struct LaunchOptions {
    pub flythrough: bool,
    pub validation_remote: Option<u16>,
    pub material_debug: u32,
    pub verify: Option<PathBuf>,
    pub verify_controls: bool,
    pub verify_locomotion: bool,
    pub verify_resident_textures: bool,
    pub verify_viewmodel_shadows: bool,
    pub verify_actor_shadows: bool,
    pub benchmark: bool,
    pub release_benchmark: bool,
    pub benchmark_view: Option<usize>,
    pub benchmark_motion: bool,
    pub benchmark_combat: bool,
    pub test_combat: bool,
    pub benchmark_hands: bool,
    pub verify_wall_weapons: bool,
    pub verify_weapon_assemblies: bool,
    pub verify_ads: bool,
    pub verify_rounds: bool,
    pub verify_health: bool,
    pub verify_solo_revive: bool,
    pub verify_solo_bleedout: bool,
    pub verify_perk_health: bool,
    pub verify_speed_cola: bool,
    pub verify_stamin_up: bool,
    pub verify_mule_kick: bool,
    pub verify_firing_perks: bool,
    pub verify_defensive_perks: bool,
    pub verify_mobility: bool,
    pub verify_mud: bool,
    pub verify_doors: bool,
    pub verify_navigation: bool,
    pub verify_traversal: bool,
    pub verify_audio: bool,
    pub verify_barriers: bool,
    pub verify_generators: bool,
    pub verify_hazards: bool,
    pub verify_powerups: bool,
    pub verify_weapon_actions: bool,
    pub verify_replacements: bool,
    pub verify_hit_regions: bool,
    pub verify_grenades: bool,
    pub verify_semtex: bool,
    pub verify_claymores: bool,
    pub verify_monkeys: bool,
    pub verify_equipment_refills: bool,
    pub error: Option<String>,
}

impl LaunchOptions {
    pub fn parse(mut args: impl Iterator<Item = OsString>) -> Self {
        let mut result = Self::default();
        if let Some(argument) = args.next() {
            if argument == "--validation-remote" {
                let port = args
                    .next()
                    .map_or(Some(15702), |v| v.to_str()?.parse::<u16>().ok());
                if cfg!(all(debug_assertions, feature = "dev-tools"))
                    && port.is_some_and(|p| p > 0 && p != 15703)
                    && args.next().is_none()
                {
                    result.validation_remote = port;
                } else {
                    result.error =
                        Some("Usage: development origins --validation-remote [port]; port 15703 is reserved for render BRP".into());
                }
                return result;
            }
            if argument == "--verify-resident-textures"
                || argument == "--verify-viewmodel-shadows"
                || argument == "--verify-actor-shadows"
            {
                result.verify_resident_textures = argument == "--verify-resident-textures";
                result.verify_viewmodel_shadows = argument == "--verify-viewmodel-shadows";
                result.verify_actor_shadows = argument == "--verify-actor-shadows";
                result.verify = args.next().map(PathBuf::from);
                if result.verify.is_none() || args.next().is_some() {
                    result.error = Some(
                        "Usage: origins --verify-resident-textures/--verify-viewmodel-shadows/--verify-actor-shadows external-output-folder".into(),
                    );
                }
                return result;
            }
            if argument == "--release-benchmark-moving" || argument == "--release-benchmark-combat"
            {
                result.benchmark = true;
                result.release_benchmark = true;
                result.benchmark_combat = argument == "--release-benchmark-combat";
                result.verify = args.next().map(PathBuf::from);
                if result.verify.is_none() || args.next().is_some() {
                    result.error = Some("Usage: origins --release-benchmark-moving|--release-benchmark-combat external-output-folder".into());
                }
                return result;
            }
            if argument == "--verify-controls"
                || argument == "--verify-hazards"
                || argument == "--verify-powerups"
                || argument == "--verify-weapon-actions"
                || argument == "--verify-replacements"
            {
                result.verify_controls = argument == "--verify-controls";
                result.verify_hazards = argument == "--verify-hazards";
                result.verify_powerups = argument == "--verify-powerups";
                result.verify_weapon_actions = argument == "--verify-weapon-actions";
                result.verify_replacements = argument == "--verify-replacements";
                result.verify = args.next().map(PathBuf::from);
                if result.verify.is_none() || args.next().is_some() {
                    result.error = Some(
                        "Expected one external output folder after the verification flag".into(),
                    );
                }
                return result;
            }
            if argument == "--material-debug"
                || argument == "--material-debug-combat"
                || argument == "--material-debug-hands"
            {
                result.benchmark = true;
                result.benchmark_combat = argument == "--material-debug-combat";
                result.benchmark_hands = argument == "--material-debug-hands";
                result.verify = args.next().map(PathBuf::from);
                result.material_debug = args
                    .next()
                    .and_then(|v| v.to_str()?.parse().ok())
                    .unwrap_or(0);
                result.benchmark_view = if result.benchmark_combat {
                    Some(8)
                } else {
                    args.next().and_then(|v| v.to_str()?.parse().ok())
                };
                if result.verify.is_none()
                    || !(u32::from(!result.benchmark_hands)..=12).contains(&result.material_debug)
                    || !result.benchmark_view.is_some_and(|view| view < 18)
                    || args.next().is_some()
                {
                    result.error = Some("Usage: origins --material-debug external-output-folder mode[1..12] view[0..17], or --material-debug-combat external-output-folder mode[1..12]".into());
                }
                return result;
            }
            if argument == "--verify"
                || argument == "--benchmark"
                || argument == "--benchmark-view"
                || argument == "--benchmark-moving-view"
                || argument == "--benchmark-combat"
                || argument == "--test-combat"
                || argument == "--verify-wall-weapons"
                || argument == "--verify-weapon-assemblies"
                || argument == "--verify-ads"
                || argument == "--verify-rounds"
                || argument == "--verify-solo-bleedout"
                || argument == "--verify-solo-revive"
                || argument == "--verify-perk-health"
                || argument == "--verify-speed-cola"
                || argument == "--verify-stamin-up"
                || argument == "--verify-mule-kick"
                || argument == "--verify-firing-perks"
                || argument == "--verify-defensive-perks"
                || argument == "--verify-mobility"
                || argument == "--verify-mud"
                || argument == "--verify-doors"
                || argument == "--verify-navigation"
                || argument == "--verify-traversal"
                || argument == "--verify-locomotion"
                || argument == "--verify-audio"
                || argument == "--verify-barriers"
                || argument == "--verify-generators"
                || argument == "--verify-health"
                || argument == "--verify-hit-regions"
                || argument == "--verify-grenades"
                || argument == "--verify-monkeys"
                || argument == "--verify-equipment-refills"
                || argument == "--verify-claymores"
                || argument == "--verify-semtex"
            {
                result.benchmark = argument != "--verify";
                result.verify = args.next().map(PathBuf::from);
                result.benchmark_motion = argument == "--benchmark-moving-view";
                result.test_combat = argument == "--test-combat";
                result.benchmark_combat = result.test_combat
                    || argument == "--benchmark-combat"
                    || argument == "--verify-wall-weapons"
                    || argument == "--verify-rounds"
                    || argument == "--verify-solo-bleedout"
                    || argument == "--verify-solo-revive"
                    || argument == "--verify-perk-health"
                    || argument == "--verify-speed-cola"
                    || argument == "--verify-stamin-up"
                    || argument == "--verify-mule-kick"
                    || argument == "--verify-firing-perks"
                    || argument == "--verify-defensive-perks"
                    || argument == "--verify-mobility"
                    || argument == "--verify-mud"
                    || argument == "--verify-doors"
                    || argument == "--verify-navigation"
                    || argument == "--verify-traversal"
                    || argument == "--verify-audio"
                    || argument == "--verify-barriers"
                    || argument == "--verify-generators"
                    || argument == "--verify-health"
                    || argument == "--verify-hit-regions"
                    || argument == "--verify-grenades"
                    || argument == "--verify-monkeys"
                    || argument == "--verify-equipment-refills"
                    || argument == "--verify-claymores"
                    || argument == "--verify-semtex";
                result.verify_wall_weapons = argument == "--verify-wall-weapons";
                result.verify_weapon_assemblies = argument == "--verify-weapon-assemblies";
                result.verify_ads = argument == "--verify-ads";
                result.verify_rounds = argument == "--verify-rounds";
                result.verify_health = argument == "--verify-health";
                result.verify_solo_revive =
                    argument == "--verify-solo-revive" || argument == "--verify-solo-bleedout";
                result.verify_solo_bleedout = argument == "--verify-solo-bleedout";
                result.verify_perk_health = argument == "--verify-perk-health";
                result.verify_speed_cola = argument == "--verify-speed-cola";
                result.verify_stamin_up = argument == "--verify-stamin-up";
                result.verify_mule_kick = argument == "--verify-mule-kick";
                result.verify_firing_perks = argument == "--verify-firing-perks";
                result.verify_defensive_perks = argument == "--verify-defensive-perks";
                result.verify_mobility = argument == "--verify-mobility";
                result.verify_mud = argument == "--verify-mud";
                result.verify_doors = argument == "--verify-doors"
                    || argument == "--verify-navigation"
                    || argument == "--verify-traversal";
                result.verify_navigation =
                    argument == "--verify-navigation" || argument == "--verify-traversal";
                result.verify_traversal = argument == "--verify-traversal";
                result.verify_locomotion = argument == "--verify-locomotion";
                result.verify_audio = argument == "--verify-audio";
                result.verify_barriers = argument == "--verify-barriers";
                result.verify_generators = argument == "--verify-generators";
                result.verify_hit_regions = argument == "--verify-hit-regions";
                result.verify_grenades = argument == "--verify-grenades";
                result.verify_semtex = argument == "--verify-semtex";
                result.verify_claymores = argument == "--verify-claymores";
                result.verify_monkeys = argument == "--verify-monkeys";
                result.verify_equipment_refills = argument == "--verify-equipment-refills";
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
            for remaining in args.by_ref() {
                if !(result.verify_weapon_assemblies && remaining == "--dual-wield-only") {
                    result.error = Some("Unexpected or repeated launch argument".into());
                }
            }
            if result.verify.is_none() {
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
    pub contents: crate::pack::Pack,
}

impl AssetPack {
    pub fn discover() -> Result<Self, String> {
        let executable = std::env::current_exe().map_err(|error| error.to_string())?;
        let path = executable.parent().ok_or("Executable directory is unavailable")?.join("origins.pack");
        let contents = crate::pack::Pack::read(&path).map_err(|error| {
            format!("Cannot load runtime pack {}: {error}. Build origins.pack from the authorized game installation and place it beside origins.exe.", path.display())
        })?;
        Ok(Self { pack_root: path, contents })
    }
}

#[derive(Resource)]
pub struct LoadConfiguration(pub Result<AssetPack, String>);
#[derive(Resource, Default)]
pub struct LoadFailure(pub String);

pub use crate::origins::{GameDefinition, Tuning};
#[derive(Component)]
pub(crate) struct MapRoot {
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
            .add_systems(OnEnter(GameState::Loading), begin_load)
            .add_observer(scene_ready)
            .add_systems(Update, finish_load.run_if(in_state(GameState::Loading)));
    }
}

pub(crate) fn begin_load(
    mut commands: Commands,
    config: Res<LoadConfiguration>,
    options: Res<LaunchOptions>,
    server: Res<AssetServer>,
    mut failure: ResMut<LoadFailure>,
    mut next: ResMut<NextState<GameState>>,
    roots: Query<Entity, With<MapRoot>>,
) {
    for root in &roots {
        commands.entity(root).despawn();
    }
    commands.remove_resource::<Loading>();
    commands.remove_resource::<prepared::PreparedArt>();
    failure.0.clear();
    let definition = (|| {
        if let Some(error) = &options.error {
            return Err(error.clone());
        }
        let config = config.0.as_ref().map_err(Clone::clone)?;
        info!("Origins asset pack: {}", config.pack_root.display());
        reload_failed_pack_assets(&server, &config.contents);
        let mut definition = GameDefinition::load(config.contents.get("game.json").map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
        let art = prepared::PreparedArt::load(&config.contents)?;
        art.bind(&mut definition)?;
        definition.validate().map_err(|error| error.to_string())?;
        commands.insert_resource(art);
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
                server.reload(path.clone());
                commands.spawn((
                    WorldAssetRoot(handle.clone()),
                    MapRoot { collision },
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

fn reload_failed_pack_assets(server: &AssetServer, pack: &crate::pack::Pack) {
    for path in pack.entries.keys() {
        if server.get_path_ids(path.clone()).iter().any(|id| {
            matches!(server.get_load_state(*id), Some(LoadState::Failed(_)))
        }) {
            server.reload(path.clone());
        }
    }
}

pub(crate) fn native_scene<'a>(game: &'a GameDefinition, model: &str) -> Option<&'a String> {
    let actor = format!("actor.{model}");
    if let Some(scene) = game
        .content
        .presentations
        .iter()
        .find(|presentation| presentation.id.0 == actor)
        .and_then(|presentation| presentation.scene.as_ref())
    {
        return Some(scene);
    }
    let name = format!("/{model}_lod0.gltf#Scene0");
    let prepared = format!("-{model}.gltf#Scene0");
    game.content
        .presentations
        .iter()
        .filter_map(|p| p.scene.as_ref())
        .filter(|scene| scene.ends_with(&name) || scene.ends_with(&prepared))
        .min_by_key(|scene| {
            (
                !scene.contains("-zm_tomb_patch-"),
                !scene.contains("-zm_tomb-"),
                scene.as_str(),
            )
        })
}

fn scene_ready(
    event: On<WorldInstanceReady>,
    roots: Query<&MapRoot>,
    children: Query<&Children>,
    mesh_entities: Query<(
        &Mesh3d,
        Option<&bevy::gltf::GltfMaterialName>,
        Option<&bevy::gltf::GltfMaterialExtras>,
    )>,
    extras: Query<&bevy::gltf::GltfExtras>,
    definition: Res<GameDefinition>,
    meshes: Res<Assets<Mesh>>,
    loading: Option<ResMut<Loading>>,
    mut commands: Commands,
    mut failure: ResMut<LoadFailure>,
    mut next: ResMut<NextState<GameState>>,
    mut cache: Local<std::collections::HashMap<AssetId<Mesh>, Collider>>,
) {
    let Some(mut loading) = loading else {
        return;
    };
    let Ok(root) = roots.get(event.entity) else {
        return;
    };
    if !root.collision {
        for entity in children.iter_descendants(event.entity) {
            let Some(metadata) = extras
                .get(entity)
                .ok()
                .and_then(|e| serde_json::from_str::<serde_json::Value>(&e.value).ok())
            else {
                continue;
            };
            if metadata["static_instance"].as_bool() == Some(true)
                && metadata["fixture_emitter"].as_bool() == Some(true)
            {
                for surface in children.iter_descendants(entity) {
                    if mesh_entities.contains(surface) {
                        commands
                            .entity(surface)
                            .insert(crate::rendering::FixtureEmitter(
                                metadata["source_model"]
                                    .as_str()
                                    .unwrap_or_default()
                                    .to_owned(),
                            ));
                    }
                }
            }
            if metadata["static_instance"].as_bool() == Some(true)
                && metadata["source_model"].as_str().is_some_and(|model| {
                    matches!(
                        model,
                        "veh_t6_dlc_mkiv_tank"
                            | "veh_t6_dlc_zm_robot"
                            | "p6_zm_tm_soul_box"
                            | "p6_zm_tm_elm_divider"
                            | "p6_zm_tm_wind_ceiling_ring"
                            | "p6_zm_tm_chamber_mid_piece"
                    )
                })
            {
                commands
                    .entity(entity)
                    .insert((Visibility::Hidden, ColliderDisabled));
                for child in children.iter_descendants(entity) {
                    commands.entity(child).insert(ColliderDisabled);
                }
            }
            let Some(id) = metadata["brush_entity"].as_str() else {
                continue;
            };
            if let Some(source) = definition
                .environment
                .brush_entities
                .iter()
                .find(|b| b.id == id)
            {
                let meshes = children
                    .iter_descendants(entity)
                    .filter(|e| mesh_entities.contains(*e))
                    .count();
                crate::environment::brushes::bind(
                    &mut commands,
                    entity,
                    source,
                    meshes,
                    definition
                        .environment
                        .ownership
                        .iter()
                        .find(|o| o.id == source.id),
                );
            } else {
                failure.0 = format!("Unknown brush entity {id}");
                next.set(GameState::LoadError);
            }
        }
    }
    if root.collision {
        for entity in children.iter_descendants(event.entity) {
            if let Ok((mesh, material, metadata)) = mesh_entities.get(entity) {
                let collider = cache.get(&mesh.0.id()).cloned().or_else(|| {
                    let collider = meshes.get(&mesh.0).and_then(Collider::trimesh_from_mesh)?;
                    cache.insert(mesh.0.id(), collider.clone());
                    Some(collider)
                });
                if let Some(collider) = collider {
                    commands.entity(entity).insert((
                        RigidBody::Static,
                        collider,
                        WorldCollider,
                        crate::movement::physics::SurfaceFlags(
                            metadata
                                .and_then(|extras| {
                                    serde_json::from_str::<serde_json::Value>(&extras.value).ok()
                                })
                                .and_then(|value| value["surface"]["flags"].as_u64())
                                .unwrap_or(0) as u32,
                        ),
                        crate::audio::SurfaceSound(
                            material
                                .and_then(|m| definition.audio.surfaces.get(&m.0))
                                .cloned()
                                .unwrap_or_else(|| "default".into()),
                        ),
                    ));
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
    effects: Option<Res<crate::native_fx::NativeFxAssets>>,
    library: Option<Res<crate::rendering::opaque::SurfaceLibrary>>,
    loading: Option<ResMut<Loading>>,
    definition: Option<Res<GameDefinition>>,
    config: Res<LoadConfiguration>,
    server: Res<AssetServer>,
    mut failure: ResMut<LoadFailure>,
    mut next: ResMut<NextState<GameState>>,
    options: Res<LaunchOptions>,
) {
    if !failure.0.is_empty() {
        return;
    }
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
                "Asset pack: {root}\nScene {:?}: {error}\nReplace origins.pack with a complete runtime pack and restart the game.",
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
        let Some(library) = library else { return };
        let textures_ready = match library.textures_ready(&server) {
            Ok(ready) => ready,
            Err(error) => {
                failure.0 = error;
                next.set(GameState::LoadError);
                return;
            }
        };
        let Some(effects) = effects else {
            return;
        };
        if let Some(RecursiveDependencyLoadState::Failed(error)) =
            server.get_recursive_dependency_load_state(effects.0.id())
        {
            failure.0 = format!("Native effects dependency: {error}");
            next.set(GameState::LoadError);
            return;
        }
        if let Some(LoadState::Failed(error)) = server.get_load_state(effects.0.id()) {
            failure.0 = format!("Native effects: {error}");
            next.set(GameState::LoadError);
            return;
        }
        if !surfaces.ready()
            || !textures_ready
            || !server.is_loaded_with_dependencies(effects.0.id())
        {
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
        next.set(
            if options.verify.is_some() || options.validation_remote.is_some() || options.flythrough
            {
                GameState::PreparingShaders
            } else {
                GameState::MainMenu
            },
        );
    } else if loading.started.elapsed().as_secs() > 120 {
        failure.0 = "Scene loading timed out or collision coverage is incomplete.".into();
        next.set(GameState::LoadError);
    }
}

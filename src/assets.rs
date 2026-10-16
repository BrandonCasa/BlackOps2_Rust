use std::{ffi::OsString, path::PathBuf};

use avian3d::prelude::*;
use bevy::{
    prelude::*,
    world_serialization::{
        WorldAsset, WorldAssetRoot, WorldInstance, WorldInstanceReady, WorldInstanceSpawner,
    },
};

use crate::GameState;

pub(crate) const SOURCE_UNIT_METERS: f32 = 0.0254;

pub const ATTRIBUTE_SURFACE: bevy::mesh::MeshVertexAttribute = bevy::mesh::MeshVertexAttribute::new(
    "Surface",
    0x4f52_4947_5355_5246,
    bevy::render::render_resource::VertexFormat::Float32,
);

// Canonical IDs written by the unchanged upstream collision exporter.
const ENGINE_SURFACES: [&str; 33] = [
    "default",
    "bark",
    "brick",
    "carpet",
    "cloth",
    "concrete",
    "dirt",
    "flesh",
    "foliage",
    "glass",
    "grass",
    "gravel",
    "ice",
    "metal",
    "mud",
    "paper",
    "plaster",
    "rock",
    "sand",
    "snow",
    "water",
    "wood",
    "asphalt",
    "ceramic",
    "plastic",
    "rubber",
    "cushion",
    "fruit",
    "paintedmetal",
    "riotshield",
    "slush",
    "player",
    "tallgrass",
];

#[derive(Resource, Default)]
pub struct LaunchOptions {
    pub asset_root: PathBuf,
}

impl LaunchOptions {
    pub fn parse(mut args: impl Iterator<Item = OsString>) -> Self {
        let asset_root = if args.any(|argument| argument == "--asset-root") {
            PathBuf::from(args.next().expect("--asset-root requires a directory"))
        } else {
            // Cargo profiles live below target; ordinary assets live at target/assets.
            let executable = std::env::current_exe().expect("Origins executable path");
            executable
                .ancestors()
                .skip(1)
                .map(|parent| parent.join("assets"))
                .find(|path| path.join("game.json").is_file())
                .expect("Place the assets directory beside the executable or its profile directory")
        };
        Self { asset_root }
    }
}

pub use crate::origins::{GameDefinition, Tuning};
#[derive(Component)]
pub(crate) struct MapRoot;
#[derive(Component)]
pub struct WorldCollider;
#[derive(Component, Clone, Copy)]
pub(crate) struct WorldContents(pub u32);
#[derive(Component)]
pub(crate) struct WorldTerrain;
pub(crate) const RAGDOLL_WORLD_LAYER: u32 = 256;
pub(crate) const DYNENT_WORLD_LAYER: u32 = 512;
pub(crate) fn world_collision_layers(contents: u32) -> CollisionLayers {
    let membership = if contents & 1 != 0 { 1 } else { 0 }
        | if contents & 0x10000 != 0 {
            crate::environment::PLAYER_INGRESS_LAYER
        } else {
            0
        }
        | if contents & 0x20000 != 0 {
            crate::environment::MONSTER_CLIP_LAYER
        } else {
            0
        }
        | if contents & 0x0281ee93 != 0 {
            RAGDOLL_WORLD_LAYER
        } else {
            0
        }
        | if contents & 0x0280ec93 != 0 {
            DYNENT_WORLD_LAYER
        } else {
            0
        };
    CollisionLayers::new(membership, LayerMask::ALL)
}
// T6 appends riotshield after the T5 player/tallgrass entries. The
// exporter's unified table inserts riotshield/slush before those two entries.
fn native_surface_type(surface: usize) -> Result<u32, String> {
    match surface {
        0..=28 => Ok(surface as u32),
        29 => Ok(31),
        31 => Ok(29),
        32 => Ok(30),
        30 => Err("Exported slush surface has no native T6 surface type".into()),
        _ => Err("Invalid exported surface type".into()),
    }
}

pub(crate) fn world_surface_colliders(mesh: &Mesh) -> Result<Vec<(usize, Collider)>, String> {
    use bevy::mesh::{PrimitiveTopology, VertexAttributeValues};
    if mesh.primitive_topology() != PrimitiveTopology::TriangleList {
        return Err("Required collision mesh is not a triangle list".into());
    }
    let Some(VertexAttributeValues::Float32x3(positions)) =
        mesh.attribute(Mesh::ATTRIBUTE_POSITION)
    else {
        return Err("Required collision mesh has no Float32x3 positions".into());
    };
    let Some(VertexAttributeValues::Float32(surfaces)) = mesh.attribute(ATTRIBUTE_SURFACE) else {
        return Err("Required collision mesh has no Float32 _SURFACE attribute".into());
    };
    if surfaces.len() != positions.len() {
        return Err("Required collision mesh contains invalid _SURFACE IDs or vertex count".into());
    }
    let indices: Vec<u32> = if let Some(indices) = mesh.indices() {
        indices
            .iter()
            .map(|index| {
                u32::try_from(index).map_err(|_| "Collision vertex index exceeds u32".to_owned())
            })
            .collect::<Result<_, _>>()?
    } else {
        (0..u32::try_from(positions.len()).map_err(|_| "Collision vertex count exceeds u32")?)
            .collect()
    };
    if indices.is_empty() || indices.len() % 3 != 0 {
        return Err("Required collision mesh contains no complete triangles".into());
    }
    let mut groups = std::collections::BTreeMap::<usize, Vec<u32>>::new();
    for triangle in indices.chunks_exact(3) {
        let ids = triangle
            .iter()
            .map(|index| surfaces.get(*index as usize).copied())
            .collect::<Option<Vec<_>>>()
            .ok_or("Collision triangle references a missing vertex")?;
        if ids[0] != ids[1] || ids[0] != ids[2] {
            return Err("Collision triangle has inconsistent source _SURFACE IDs".into());
        }
        groups
            .entry(ids[0] as usize)
            .or_default()
            .extend_from_slice(triangle);
    }
    groups
        .into_iter()
        .map(|(surface, indices)| {
            let mut remap = std::collections::HashMap::new();
            let mut vertices = Vec::new();
            let triangles = indices
                .chunks_exact(3)
                .map(|triangle| {
                    std::array::from_fn(|corner| {
                        let original = triangle[corner];
                        *remap.entry(original).or_insert_with(|| {
                            let index = vertices.len() as u32;
                            vertices.push(Vec3::from_array(positions[original as usize]));
                            index
                        })
                    })
                })
                .collect();
            Ok((surface, Collider::trimesh(vertices, triangles)))
        })
        .collect()
}

#[cfg(test)]
mod collision_tests {
    use super::*;

    #[test]
    fn native_player_and_monster_clips_block_only_their_actor() {
        let entity = Entity::from_bits(1);
        let player = crate::environment::player_solids([]);
        let zombie = crate::environment::actor_solids([]);
        for (contents, blocks_player, blocks_zombie) in [
            (1, true, true),
            (0x10000, true, false),
            (0x08020000, false, true),
            (0x30000, true, true),
            (0x40000000, false, false),
        ] {
            let layers = world_collision_layers(contents);
            assert_eq!(player.test(entity, layers), blocks_player, "{contents:#x}");
            assert_eq!(zombie.test(entity, layers), blocks_zombie, "{contents:#x}");
            assert_eq!(
                layers.memberships.0 & RAGDOLL_WORLD_LAYER != 0,
                contents & 0x0281ee93 != 0,
            );
            assert_eq!(
                layers.memberships.0 & DYNENT_WORLD_LAYER != 0,
                contents & 0x0280ec93 != 0,
            );
        }
    }
}
#[derive(Component)]
pub(crate) struct StaticPropRoot;

#[derive(serde::Deserialize)]
struct WorldMetadata {
    #[serde(flatten)]
    models: crate::rendering::models::StaticModelDefinition,
    #[serde(flatten)]
    fog: crate::rendering::fog::WorldFogDefinition,
    #[serde(flatten)]
    draws: crate::rendering::world::WorldDrawDefinition,
    #[serde(rename = "lightGrid")]
    light_grid: crate::rendering::light_grid::GridDefinition,
}
pub struct GameAssetsPlugin;
impl Plugin for GameAssetsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::Loading), begin_load)
            .add_observer(scene_ready)
            .add_systems(Update, asset_failures)
            .add_systems(
                Update,
                (poll_definition, finish_load)
                    .chain()
                    .run_if(in_state(GameState::Loading)),
            );
    }
}

#[derive(Resource)]
struct DefinitionTask(
    bevy::tasks::Task<(
        GameDefinition,
        crate::rendering::models::LoadedModels,
        crate::rendering::fog::WorldFogDefinition,
        crate::rendering::world::LoadedWorld,
        crate::rendering::model_lighting::ModelLightingSource,
    )>,
);

pub(crate) fn begin_load(
    mut commands: Commands,
    options: Res<LaunchOptions>,
    roots: Query<
        Entity,
        (
            Without<ChildOf>,
            Or<(
                With<MapRoot>,
                With<crate::rendering::models::NativeModelsRoot>,
                With<crate::rendering::world::NativeWorldRoot>,
            )>,
        ),
    >,
) {
    for root in &roots {
        commands.entity(root).despawn();
    }
    commands.remove_resource::<crate::rendering::reflection::ReflectionWorld>();
    commands.remove_resource::<crate::rendering::fog::WorldFogDefinition>();
    commands.remove_resource::<crate::rendering::model_lighting::ModelLightingSource>();
    commands.remove_resource::<crate::rendering::world::WorldWaterLighting>();
    commands.remove_resource::<crate::rendering::native_effect::NativeOutdoorMap>();
    commands.remove_resource::<DefinitionTask>();
    let root = options.asset_root.clone();
    let task = bevy::tasks::IoTaskPool::get().spawn(async move {
        info!("Origins asset root: {}", root.display());
        let definition = GameDefinition::load_from_root(&root);
        let metadata: WorldMetadata = crate::origins::read(&root.join(&definition.world_metadata));
        metadata.fog.validate();
        let coefficient_path = root.join(&metadata.light_grid.coefficients_file);
        let coefficient_bytes = std::fs::read(&coefficient_path).unwrap_or_else(|error| {
            panic!(
                "Read original model lighting {}: {error}",
                coefficient_path.display()
            )
        });
        assert_eq!(
            coefficient_bytes.len(),
            metadata.light_grid.coeff_count * 54,
            "Original model-lighting coefficient count"
        );
        let lighting = crate::rendering::model_lighting::ModelLightingSource {
            coefficients: std::sync::Arc::new(
                crate::rendering::lighting::LightingCoefficients::from_bytes(&coefficient_bytes),
            ),
            grid: std::sync::Arc::new(metadata.light_grid.load(&root)),
        };
        let models = metadata
            .models
            .load(&root, metadata.draws.reflection_probes());
        (
            definition,
            models,
            metadata.fog,
            metadata.draws.load(&root),
            lighting,
        )
    });
    commands.insert_resource(DefinitionTask(task));
}

fn poll_definition(
    mut commands: Commands,
    pending: Option<ResMut<DefinitionTask>>,
    server: Res<AssetServer>,
    mut geometries: ResMut<Assets<crate::rendering::native::NativeWorldGeometry>>,
    mut materials: ResMut<Assets<crate::rendering::native::NativeWorldMaterial>>,
    mut model_materials: ResMut<Assets<crate::rendering::native_model::NativeModelMaterial>>,
    mut model_vertex_lighting: ResMut<Assets<crate::rendering::native_model::ModelVertexLighting>>,
    mut water_materials: ResMut<Assets<crate::rendering::native_water::NativeWaterMaterial>>,
) {
    let Some(mut pending) = pending else {
        return;
    };
    let Some(definition) = bevy::tasks::futures::check_ready(&mut pending.0) else {
        return;
    };
    commands.remove_resource::<DefinitionTask>();
    let (definition, models, fog, draws, lighting) = definition;
    let sun = fog.initial_sun_light();
    commands.insert_resource(fog);
    commands.insert_resource(lighting);
    draws.spawn(
        &mut commands,
        &server,
        &mut geometries,
        &mut materials,
        &mut water_materials,
        &sun,
        &definition,
    );
    let handle: Handle<WorldAsset> = server
        .load_builder()
        .with_settings(|settings: &mut bevy::gltf::GltfLoaderSettings| {
            settings.load_animations = false;
            settings.load_meshes = bevy::asset::RenderAssetUsages::MAIN_WORLD;
            settings.load_materials = bevy::asset::RenderAssetUsages::empty();
        })
        .load(definition.collision_scene.clone());
    commands.spawn((WorldAssetRoot(handle), MapRoot, Visibility::Hidden));
    models.spawn(
        &mut commands,
        &server,
        &mut geometries,
        &mut model_materials,
        &mut model_vertex_lighting,
    );
    commands.insert_resource(definition);
}

pub(crate) fn native_scene<'a>(game: &'a GameDefinition, model: &str) -> Option<&'a String> {
    let actor = format!("actor.{model}");
    game.content
        .presentations
        .iter()
        .find(|presentation| presentation.id.0 == actor)
        .and_then(|presentation| presentation.scene.as_ref())
}

fn scene_ready(
    event: On<WorldInstanceReady>,
    roots: Query<(), With<MapRoot>>,
    children: Query<&Children>,
    parents: Query<&ChildOf>,
    mesh_entities: Query<&Mesh3d>,
    extras: Query<&bevy::gltf::GltfExtras>,
    meshes: Res<Assets<Mesh>>,
    mut commands: Commands,
) {
    let Ok(()) = roots.get(event.entity) else {
        return;
    };
    let mut colliders = 0;
    for entity in children.iter_descendants(event.entity) {
        let Ok(mesh) = mesh_entities.get(entity) else {
            continue;
        };
        let mut ancestor = entity;
        let metadata = loop {
            if let Ok(extra) = extras.get(ancestor) {
                match serde_json::from_str::<serde_json::Value>(&extra.value) {
                    Ok(value) if value.get("group").is_some() => break Some(value),
                    Err(error) => {
                        panic!("Invalid collision extras on {ancestor:?}: {error}");
                    }
                    _ => {}
                }
            }
            let Ok(parent) = parents.get(ancestor) else {
                break None;
            };
            ancestor = parent.parent();
            if ancestor == event.entity {
                break None;
            }
        };
        let Some(metadata) = metadata else {
            panic!("Collision mesh {entity:?} has no exported group metadata");
        };
        if metadata.get("part").is_some() {
            // The visible part's gameplay owner supplies its moving collider.
            continue;
        }
        let contents = metadata["contents"]
            .as_str()
            .and_then(|bits| u32::from_str_radix(bits.trim_start_matches("0x"), 16).ok())
            .or_else(|| {
                metadata["contents"]
                    .as_u64()
                    .and_then(|bits| u32::try_from(bits).ok())
            })
            .or_else(|| (metadata["group"] == "soup").then_some(1));
        let Some(contents) = contents else {
            panic!(
                "Collision group {:?} has no source contents",
                metadata["group"]
            );
        };
        let groups = match meshes
            .get(&mesh.0)
            .ok_or_else(|| "Required collision mesh asset is missing".to_owned())
            .and_then(world_surface_colliders)
        {
            Ok(groups) => groups,
            Err(error) => {
                panic!("Collision mesh {:?}: {error}", mesh.0.path());
            }
        };
        let source_flags = match metadata.get("surface_flags") {
            None => 0,
            Some(value) => match value
                .as_u64()
                .and_then(|bits| u32::try_from(bits).ok())
                .or_else(|| {
                    value.as_str().and_then(|bits| {
                        u32::from_str_radix(bits.trim_start_matches("0x"), 16).ok()
                    })
                }) {
                Some(bits) => bits,
                None => {
                    panic!(
                        "Collision mesh {:?} has invalid exported surface_flags",
                        mesh.0.path()
                    );
                }
            },
        };
        if let Some(error) = groups
            .iter()
            .find_map(|(surface, _)| native_surface_type(*surface).err())
        {
            panic!("{}", error);
        }
        // Keep the original mesh as the scene pivot; its children contain
        // disjoint original triangle groups, so there is no duplicate collider.
        commands.entity(entity).remove::<(
            Collider,
            RigidBody,
            WorldCollider,
            WorldContents,
            WorldTerrain,
            CollisionLayers,
        )>();
        for (surface, collider) in groups {
            let mut child = commands.spawn((
                ChildOf(entity),
                Transform::default(),
                Visibility::default(),
                Name::new(format!("Collision surface {}", ENGINE_SURFACES[surface])),
                RigidBody::Static,
                collider,
                WorldCollider,
                WorldContents(contents),
                world_collision_layers(contents),
                crate::audio::SurfaceSound(ENGINE_SURFACES[surface].to_owned()),
                crate::movement::physics::SurfaceFlags(
                    (source_flags & !(0x3f << 20))
                        | (native_surface_type(surface).expect("surface type validated") << 20),
                ),
            ));
            if metadata["group"] == "soup" {
                child.insert(WorldTerrain);
            }
            colliders += 1;
        }
    }
    if colliders == 0 {
        panic!("Required collision scene contains no usable collision meshes");
    }
}

fn asset_failures(mut failures: MessageReader<bevy::asset::UntypedAssetLoadFailedEvent>) {
    for failure in failures.read() {
        panic!("Required asset {}: {}", failure.path, failure.error);
    }
}

fn finish_load(
    mut commands: Commands,
    roots: Query<(&WorldAssetRoot, Option<&WorldInstance>), With<MapRoot>>,
    spawner: Res<WorldInstanceSpawner>,
    server: Res<AssetServer>,
    native_worlds: Query<(Entity, &crate::rendering::world::NativeWorldRoot)>,
    native_models: Query<(Entity, &crate::rendering::models::NativeModelsRoot)>,
    mut next: ResMut<NextState<GameState>>,
) {
    if roots.is_empty()
        || native_worlds.is_empty()
        || native_models.is_empty()
        || native_worlds
            .iter()
            .any(|(_, world)| !world.images_ready(&server))
        || native_models
            .iter()
            .any(|(_, models)| !models.images_ready(&server))
        || roots.iter().any(|(scene, instance)| {
            !instance.is_some_and(|instance| spawner.instance_is_ready(**instance))
                || !server.is_loaded_with_dependencies(scene.0.id())
        })
    {
        return;
    }
    for (entity, _) in &native_worlds {
        commands.entity(entity).insert(Visibility::Inherited);
    }
    for (entity, _) in &native_models {
        commands.entity(entity).insert(Visibility::Inherited);
    }
    next.set(GameState::StartingSession);
}

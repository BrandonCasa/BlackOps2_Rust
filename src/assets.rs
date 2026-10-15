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

use crate::GameState;

pub(crate) const SOURCE_UNIT_METERS: f32 = 0.0254;

pub const ATTRIBUTE_SURFACE: bevy::mesh::MeshVertexAttribute =
    bevy::mesh::MeshVertexAttribute::new("Surface", 0x4f52_4947_5355_5246, bevy::render::render_resource::VertexFormat::Float32);

// Canonical IDs written by the unchanged upstream collision exporter.
const ENGINE_SURFACES: [&str; 33] = [
    "default", "bark", "brick", "carpet", "cloth", "concrete", "dirt", "flesh", "foliage", "glass", "grass",
    "gravel", "ice", "metal", "mud", "paper", "plaster", "rock", "sand", "snow", "water", "wood",
    "asphalt", "ceramic", "plastic", "rubber", "cushion", "fruit", "paintedmetal", "riotshield", "slush", "player", "tallgrass",
];

#[derive(Resource, Default)]
pub struct LaunchOptions {
    pub asset_root: PathBuf,
}

impl LaunchOptions {
    pub fn parse(mut args: impl Iterator<Item = OsString>) -> Self {
        let asset_root = args.find(|argument| argument == "--asset-root")
            .and_then(|_| args.next())
            .expect("Launch Origins with --asset-root <absolute directory>");
        Self { asset_root: PathBuf::from(asset_root) }
    }
}

#[derive(Debug)]
pub struct AssetDirectory {
    pub root: PathBuf,
}

impl AssetDirectory {
    pub fn discover(root: &Path) -> Result<Self, String> {
        if !root.is_absolute() || !root.is_dir() {
            return Err(format!(
                "Asset root must be an existing absolute directory: {}",
                root.display()
            ));
        }
        if !root.join("game.json").is_file() {
            return Err(format!(
                "Required gameplay definition is missing: {}",
                root.join("game.json").display()
            ));
        }
        Ok(Self {
            root: root.to_owned(),
        })
    }
}

#[derive(Resource)]
pub struct LoadConfiguration(pub Result<AssetDirectory, String>);

pub use crate::origins::{GameDefinition, Tuning};
#[derive(Component)]
pub(crate) struct MapRoot {
    collision: bool,
}
#[derive(Component)]
pub struct WorldCollider;
#[derive(Component, Clone, Copy)]
pub(crate) struct WorldContents(pub u32);
#[derive(Component)]
pub(crate) struct WorldTerrain;
pub(crate) const RAGDOLL_WORLD_LAYER: u32 = 256;
pub(crate) const DYNENT_WORLD_LAYER: u32 = 512;
pub(crate) fn world_collision_layers(contents: u32) -> CollisionLayers {
    let membership = if contents & 0x10001 != 0 { 1 } else { 0 }
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
#[derive(Resource)]
pub(crate) struct WorldBrushes(Vec<(u32, Vec<Vec4>)>);
impl WorldBrushes {
    fn load(bytes: &[u8]) -> Result<Self, String> {
        #[derive(serde::Deserialize)]
        struct Vector {
            x: f32,
            y: f32,
            z: f32,
        }
        impl Vector {
            fn array(&self) -> [f32; 3] {
                [self.x, self.y, self.z]
            }
        }
        #[derive(serde::Deserialize)]
        struct Bounds {
            #[serde(rename = "midPoint")]
            mid: Vector,
            #[serde(rename = "halfSize")]
            half: Vector,
        }
        #[derive(serde::Deserialize)]
        struct Side {
            normal: [f32; 3],
            dist: f32,
        }
        #[derive(serde::Deserialize)]
        struct Brush {
            contents: u32,
            bounds: Bounds,
            sides: Vec<Side>,
        }
        #[derive(serde::Deserialize)]
        struct Model {
            brushes: Vec<usize>,
        }
        #[derive(serde::Deserialize)]
        struct ClipMap {
            brushes: Vec<Brush>,
            cmodels: Vec<Model>,
        }
        let clip: ClipMap = serde_json::from_slice(bytes)
            .map_err(|error| format!("External clipmap gameplay records are invalid: {error}"))?;
        let movable: std::collections::HashSet<usize> = clip
            .cmodels
            .iter()
            .skip(1)
            .flat_map(|model| model.brushes.iter().copied())
            .collect();
        if movable.iter().any(|index| *index >= clip.brushes.len()) {
            return Err("External clipmap contains an invalid movable brush reference".into());
        }
        // Match the external exporter's documented Y-up metre convention exactly.
        let plane = |normal: [f32; 3], distance: f32| {
            Vec4::new(normal[0], normal[2], -normal[1], distance / 39.37)
        };
        let brushes = clip
            .brushes
            .into_iter()
            .enumerate()
            .filter(|(index, _)| !movable.contains(index))
            .map(|(_, brush)| {
                let midpoint = brush.bounds.mid.array();
                let half = brush.bounds.half.array();
                let mut planes = Vec::with_capacity(6 + brush.sides.len());
                for axis in 0..3 {
                    let mut normal = [0.; 3];
                    normal[axis] = 1.;
                    planes.push(plane(normal, midpoint[axis] + half[axis]));
                    normal[axis] = -1.;
                    planes.push(plane(normal, -midpoint[axis] + half[axis]));
                }
                planes.extend(
                    brush
                        .sides
                        .into_iter()
                        .map(|side| plane(side.normal, side.dist)),
                );
                Ok((brush.contents, planes))
            })
            .collect::<Result<Vec<_>, String>>()?;
        if brushes.is_empty() {
            return Err("External clipmap contains no static brush volumes".into());
        }
        Ok(Self(brushes))
    }
    pub(crate) fn trace(&self, start: Vec3, end: Vec3, mask: u32) -> (bool, f32) {
        let mut start_solid = false;
        let mut fraction = 1f32;
        for (_, planes) in self.0.iter().filter(|(contents, _)| contents & mask != 0) {
            let mut enter = 0f32;
            let mut leave = fraction;
            let mut inside = true;
            let mut rejected = false;
            for plane in planes {
                let first = plane.truncate().dot(start) - plane.w;
                let last = plane.truncate().dot(end) - plane.w;
                if first > 0. {
                    inside = false;
                    if last >= first.min(0.125 * 0.0254) {
                        rejected = true;
                        break;
                    }
                    enter = enter.max((first - 0.125 * 0.0254) / (first - last));
                } else if last > 0. {
                    leave = leave.min(first / (first - last));
                }
                if leave <= enter {
                    rejected = true;
                    break;
                }
            }
            if !rejected {
                if inside {
                    start_solid = true;
                } else {
                    fraction = fraction.min(enter);
                }
            }
        }
        (start_solid, fraction)
    }
}
fn world_mesh_collider(mesh: &Mesh) -> Option<Collider> {
    use bevy::mesh::{Indices, PrimitiveTopology, VertexAttributeValues};
    if mesh.primitive_topology() != PrimitiveTopology::TriangleList {
        return None;
    }
    if let Some(indices) = mesh.indices() {
        return (indices.len() % 3 == 0)
            .then(|| Collider::trimesh_from_mesh(mesh))
            .flatten();
    }
    let VertexAttributeValues::Float32x3(vertices) = mesh.attribute(Mesh::ATTRIBUTE_POSITION)?
    else {
        return None;
    };
    let count = u32::try_from(vertices.len()).ok()?;
    if count == 0 || count % 3 != 0 {
        return None;
    }
    Collider::trimesh_from_mesh(
        &mesh
            .clone()
            .with_inserted_indices(Indices::U32((0..count).collect())),
    )
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

fn world_surface_colliders(mesh: &Mesh) -> Result<Vec<(usize, Collider)>, String> {
    use bevy::mesh::{Indices, PrimitiveTopology, VertexAttributeValues};
    if mesh.primitive_topology() != PrimitiveTopology::TriangleList {
        return Err("Required collision mesh is not a triangle list".into());
    }
    let Some(VertexAttributeValues::Float32x3(positions)) = mesh.attribute(Mesh::ATTRIBUTE_POSITION) else {
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
        (0..u32::try_from(positions.len()).map_err(|_| "Collision vertex count exceeds u32")?).collect()
    };
    if indices.is_empty() || indices.len() % 3 != 0 {
        return Err("Required collision mesh contains no complete triangles".into());
    }
    let mut groups = std::collections::BTreeMap::<usize, Vec<u32>>::new();
    for triangle in indices.chunks_exact(3) {
        let ids = triangle.iter().map(|index| surfaces.get(*index as usize).copied()).collect::<Option<Vec<_>>>()
            .ok_or("Collision triangle references a missing vertex")?;
        if ids[0] != ids[1] || ids[0] != ids[2] {
            return Err("Collision triangle has inconsistent source _SURFACE IDs".into());
        }
        groups.entry(ids[0] as usize).or_default().extend_from_slice(triangle);
    }
    groups
        .into_iter()
        .map(|(surface, indices)| {
            let group = mesh.clone().with_inserted_indices(Indices::U32(indices));
            world_mesh_collider(&group)
                .map(|collider| (surface, collider))
                .ok_or_else(|| {
                    format!(
                        "Cannot construct collision surface {}",
                        ENGINE_SURFACES[surface]
                    )
                })
        })
        .collect()
}

#[cfg(test)]
mod collision_tests {
    use super::*;

    #[test]
    fn nonindexed_collision_keeps_every_ordered_triangle_and_rejects_incomplete_lists() {
        use bevy::asset::RenderAssetUsages;
        use bevy::mesh::{Indices, PrimitiveTopology};
        let vertices = vec![
            [0., 0., 0.],
            [1., 0., 0.],
            [0., 0., 1.],
            [2., 0., 0.],
            [3., 0., 0.],
            [2., 0., 1.],
        ];
        let mesh = Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::MAIN_WORLD,
        )
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, vertices.clone());
        let collider = world_mesh_collider(&mesh).unwrap();
        let triangles = collider.shape().as_trimesh().unwrap();
        assert_eq!(triangles.indices().len(), 2);
        for (index, triangle) in triangles.triangles().enumerate() {
            for (vertex, source) in [triangle.a, triangle.b, triangle.c]
                .iter()
                .zip(&vertices[index * 3..index * 3 + 3])
            {
                assert_eq!([vertex.x, vertex.y, vertex.z], *source);
            }
        }
        assert!(mesh.indices().is_none());
        assert!(
            world_mesh_collider(
                &mesh
                    .clone()
                    .with_inserted_indices(Indices::U32((0..6).collect()))
            )
            .is_some()
        );
        assert!(
            world_mesh_collider(
                &mesh
                    .clone()
                    .with_inserted_indices(Indices::U32(vec![0, 1, 2, 3]))
            )
            .is_none()
        );
        assert!(
            world_mesh_collider(
                &mesh
                    .clone()
                    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, vertices[..5].to_vec())
            )
            .is_none()
        );
        assert!(
            world_mesh_collider(
                &Mesh::new(PrimitiveTopology::LineList, RenderAssetUsages::MAIN_WORLD)
                    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, vertices)
            )
            .is_none()
        );
    }

    #[test]
    fn static_prop_pose_preserves_source_basis_scale_and_units() {
        let prop = StaticProp {
            origin: SourcePosition {
                x: 39.37,
                y: 78.74,
                z: 118.11,
            },
            axis: [[0., 1., 0.], [-1., 0., 0.], [0., 0., 1.]],
            scale: 2.,
            model: "source_prop".into(),
        };
        let pose = prop.transform().unwrap();
        assert!((pose.translation - Vec3::new(1., 3., -2.)).length() < 0.00001);
        assert!((pose.rotation * Vec3::X - Vec3::NEG_Z).length() < 0.00001);
        assert!((pose.rotation * Vec3::Y - Vec3::Y).length() < 0.00001);
        assert!((pose.scale - Vec3::splat(2. / 39.37)).length() < 0.00001);
        let mut invalid = prop;
        invalid.axis[0][0] = f32::NAN;
        assert!(invalid.transform().is_err());
        invalid.axis = [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]];
        invalid.scale = -1.;
        assert!(invalid.transform().is_err());
    }

    fn external_brush(contents: u32) -> serde_json::Value {
        serde_json::json!({
            "contents": contents,
            "bounds": {
                "midPoint": {"x":39.37,"y":78.74,"z":118.11},
                "halfSize": {"x":39.37,"y":39.37,"z":39.37}
            },
            "sides": [{"normal":[1.,0.,0.],"dist":59.055}]
        })
    }

    fn load_external(
        brushes: Vec<serde_json::Value>,
        cmodels: serde_json::Value,
    ) -> Result<WorldBrushes, String> {
        WorldBrushes::load(
            &serde_json::to_vec(&serde_json::json!({
                "brushes": brushes, "cmodels": cmodels
            }))
            .unwrap(),
        )
    }

    #[test]
    fn external_clipmap_preserves_axes_scale_side_planes_and_projectile_masks() {
        let brushes = load_external(
            vec![external_brush(0x10000)],
            serde_json::json!([{"brushes":[]}]),
        )
        .unwrap();
        let center = Vec3::new(1., 3., -2.);
        assert!(brushes.trace(center, center + Vec3::Y * 3., 0x0281ee93).0);
        assert_eq!(
            brushes.trace(center, center + Vec3::Y * 3., 0x0280ec93),
            (false, 1.)
        );
        // Axial bounds and non-axial side data must agree after the same conversion.
        assert!(
            !brushes
                .trace(Vec3::new(1.75, 3., -2.), Vec3::new(2., 3., -2.), 0x10000)
                .0
        );
        assert!(brushes.trace(Vec3::new(1.4, 3., -2.), center, 0x10000).0);
        assert!(
            !brushes
                .trace(Vec3::new(1., 3., 2.), Vec3::new(1., 3., 3.), 0x10000)
                .0
        );
        assert!(
            brushes.0[0]
                .1
                .iter()
                .any(|plane| (*plane - Vec4::new(0., 1., 0., 4.)).length() < 0.00001)
        );
        assert!(
            brushes.0[0]
                .1
                .iter()
                .any(|plane| (*plane - Vec4::new(1., 0., 0., 1.5)).length() < 0.00001)
        );
    }

    #[test]
    fn external_clipmap_excludes_inline_model_owned_brushes() {
        let mut owned = external_brush(1);
        owned["bounds"]["midPoint"]["x"] = serde_json::json!(393.7);
        owned["sides"] = serde_json::json!([]);
        let brushes = load_external(
            vec![external_brush(1), owned],
            serde_json::json!([{"brushes":[]},{"brushes":[1]}]),
        )
        .unwrap();
        assert_eq!(brushes.0.len(), 1);
        assert_eq!(
            brushes.trace(Vec3::new(10., 3., -2.), Vec3::new(10., 4., -2.), 1),
            (false, 1.)
        );
        assert!(
            brushes
                .trace(Vec3::new(1., 3., -2.), Vec3::new(1., 4., -2.), 1)
                .0
        );
    }

    #[test]
    fn external_clipmap_rejects_bad_references_bounds_and_planes() {
        assert!(
            load_external(
                vec![external_brush(1)],
                serde_json::json!([{"brushes":[]},{"brushes":[2]}])
            )
            .is_err()
        );
        let mut brush = external_brush(1);
        brush["bounds"]["halfSize"]["x"] = serde_json::json!(-1.);
        assert!(load_external(vec![brush], serde_json::json!([])).is_err());
        let mut brush = external_brush(1);
        brush["sides"][0]["normal"] = serde_json::json!([2., 0., 0.]);
        assert!(load_external(vec![brush], serde_json::json!([])).is_err());
        // JSON cannot represent infinity; oversized numeric input must fail rather than reach traces.
        assert!(WorldBrushes::load(br#"{"brushes":[{"contents":1,"bounds":{"midPoint":{"x":1e100,"y":0,"z":0},"halfSize":{"x":1,"y":1,"z":1}},"sides":[]}],"cmodels":[]}"#).is_err());
    }

    #[test]
    fn native_world_brush_trace_preserves_contents_and_enter_plane_bias() {
        let mut brushes = WorldBrushes(vec![(
            1,
            vec![
                Vec4::new(1., 0., 0., 1.),
                Vec4::new(-1., 0., 0., 1.),
                Vec4::new(0., 1., 0., 1.),
                Vec4::new(0., -1., 0., 1.),
                Vec4::new(0., 0., 1., 1.),
                Vec4::new(0., 0., -1., 1.),
            ],
        )]);
        let (solid, fraction) = brushes.trace(Vec3::X * -2., Vec3::X * 2., 0x0280ec93);
        assert!(!solid);
        assert!((fraction - (1. - 0.125 * 0.0254) / 4.).abs() < 0.000001);
        assert!(brushes.trace(Vec3::ZERO, Vec3::X * 2., 0x0280ec93).0);
        brushes.0[0].0 = 0x10000;
        assert_eq!(
            brushes.trace(Vec3::ZERO, Vec3::X * 2., 0x0280ec93),
            (false, 1.)
        );
        assert!(brushes.trace(Vec3::ZERO, Vec3::X * 2., 0x0281ee93).0);
    }
}
#[derive(Component)]
pub(crate) struct StaticPropRoot;

#[derive(serde::Deserialize)]
struct WorldMetadata {
    #[serde(rename = "staticModels")]
    static_models: Vec<StaticProp>,
}
#[derive(serde::Deserialize)]
struct StaticProp {
    origin: SourcePosition,
    axis: [[f32; 3]; 3],
    scale: f32,
    model: String,
}
#[derive(serde::Deserialize)]
struct SourcePosition {
    x: f32,
    y: f32,
    z: f32,
}
impl StaticProp {
    fn transform(&self) -> Result<Transform, String> {
        let position = Vec3::new(self.origin.x, self.origin.z, -self.origin.y) / 39.37;
        let source_rotation = Mat3::from_cols_array_2d(&self.axis);
        let frame = Mat3::from_cols(Vec3::X, Vec3::NEG_Z, Vec3::Y);
        let rotation = frame * source_rotation * frame.transpose();
        // OAT GLBs already use Y-up but retain source inches.
        Ok(Transform {
            translation: position,
            rotation: Quat::from_mat3(&rotation),
            scale: Vec3::splat(self.scale / 39.37),
        })
    }
    fn gameplay_owned(&self) -> bool {
        matches!(
            self.model.as_str(),
            "veh_t6_dlc_mkiv_tank"
                | "veh_t6_dlc_zm_robot"
                | "p6_zm_tm_soul_box"
                | "p6_zm_tm_elm_divider"
                | "p6_zm_tm_wind_ceiling_ring"
                | "p6_zm_tm_chamber_mid_piece"
        )
    }
}

#[derive(Resource)]
struct Loading {
    handles: Vec<Handle<WorldAsset>>,
    ready: usize,
    scene_count: usize,
    colliders: usize,
    started: Instant,
}

pub struct GameAssetsPlugin;
impl Plugin for GameAssetsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::Loading), begin_load)
            .add_observer(scene_ready)
            .add_systems(
                Update,
                (poll_definition, finish_load)
                    .chain()
                    .run_if(in_state(GameState::Loading)),
            );
    }
}

type StaticProps = Vec<(String, String, Transform)>;
#[derive(Resource)]
struct DefinitionTask(
    bevy::tasks::Task<Result<(GameDefinition, WorldBrushes, StaticProps), String>>,
);

pub(crate) fn begin_load(
    mut commands: Commands,
    config: Res<LoadConfiguration>,
    roots: Query<Entity, Or<(With<MapRoot>, With<StaticPropRoot>)>>,
) {
    for root in &roots {
        commands.entity(root).despawn();
    }
    commands.remove_resource::<Loading>();
    commands.remove_resource::<WorldBrushes>();
    commands.remove_resource::<DefinitionTask>();
    let root = config.0.as_ref().expect("Invalid Origins asset root").root.clone();
    let task = bevy::tasks::IoTaskPool::get().spawn(async move {
        info!("Origins asset root: {}", root.display());
        let definition =
            GameDefinition::load_from_root(&root).map_err(|error| error.to_string())?;
        let volumes = root.join(&definition.collision_metadata);
        let bytes = std::fs::read(&volumes).map_err(|error| {
            format!(
                "Required external collision metadata {}: {error}",
                volumes.display()
            )
        })?;
        let brushes = WorldBrushes::load(&bytes)?;
        let metadata_path = root.join(&definition.world_metadata);
        let bytes = std::fs::read(&metadata_path).map_err(|error| {
            format!(
                "Required world metadata {}: {error}",
                metadata_path.display()
            )
        })?;
        let metadata: WorldMetadata = serde_json::from_slice(&bytes).map_err(|error| {
            format!(
                "Invalid world metadata {}: {error}",
                metadata_path.display()
            )
        })?;
        let mut props = Vec::new();
        for prop in metadata.static_models {
            let transform = prop.transform()?;
            if prop.gameplay_owned() {
                continue;
            }
            let path = format!("model_export/{}_lod0.glb", prop.model);
            props.push((path, prop.model, transform));
        }
        Ok((definition, brushes, props))
    });
    commands.insert_resource(DefinitionTask(task));
}

fn poll_definition(
    mut commands: Commands,
    pending: Option<ResMut<DefinitionTask>>,
    server: Res<AssetServer>,
) {
    let Some(mut pending) = pending else {
        return;
    };
    let Some(definition) = bevy::tasks::futures::check_ready(&mut pending.0) else {
        return;
    };
    commands.remove_resource::<DefinitionTask>();
    let (definition, brushes, props) = definition.expect("Origins source asset loading failed");
            commands.insert_resource(brushes);
            let mut handles = Vec::new();
            let scene_count = 2 + definition.additional_scenes.len();
            for (path, collision) in std::iter::once((&definition.scene, false))
                .chain(std::iter::once((&definition.collision_scene, true)))
                .chain(
                    definition
                        .additional_scenes
                        .iter()
                        .map(|path| (path, false)),
                )
            {
                let handle: Handle<WorldAsset> = server.load(path.clone());
                commands.spawn((
                    WorldAssetRoot(handle.clone()),
                    MapRoot { collision },
                    Visibility::Hidden,
                ));
                handles.push(handle);
            }
            for (path, model, transform) in props {
                let handle: Handle<WorldAsset> = server.load(format!("{path}#Scene0"));
                handles.push(handle.clone());
                commands.spawn((
                    WorldAssetRoot(handle),
                    StaticPropRoot,
                    transform,
                    Name::new(model),
                ));
            }
            commands.insert_resource(definition);
            commands.insert_resource(Loading {
                handles,
                ready: 0,
                scene_count,
                colliders: 0,
                started: Instant::now(),
            });
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
    roots: Query<&MapRoot>,
    children: Query<&Children>,
    parents: Query<&ChildOf>,
    mesh_entities: Query<&Mesh3d>,
    extras: Query<&bevy::gltf::GltfExtras>,
    definition: Res<GameDefinition>,
    meshes: Res<Assets<Mesh>>,
    loading: Option<ResMut<Loading>>,
    mut commands: Commands,
) {
    let Some(mut loading) = loading else { return };
    let Ok(root) = roots.get(event.entity) else {
        return;
    };
    if !root.collision {
        let mut owners = std::collections::HashMap::new();
        for entity in children.iter_descendants(event.entity) {
            let Ok(extra) = extras.get(entity) else {
                continue;
            };
            let Ok(metadata) = serde_json::from_str::<serde_json::Value>(&extra.value) else {
                panic!("Invalid scene extras on {entity:?}");
            };
            let Some(part) = metadata["part"].as_str() else {
                continue;
            };
            let Some(source) = definition.environment.brush_entities.iter().find(|source| {
                source
                    .source
                    .get("model")
                    .is_some_and(|model| model == part)
            }) else {
                panic!("Exported brush part {part} has no gameplay owner");
            };
            let owner = *owners.entry(part.to_owned()).or_insert_with(|| {
                let owner = commands
                    .spawn((
                        ChildOf(event.entity),
                        source.transform(),
                        Visibility::default(),
                        Name::new(source.id.clone()),
                    ))
                    .id();
                crate::environment::brushes::bind(
                    &mut commands,
                    owner,
                    source,
                    source.visual_meshes,
                    definition
                        .environment
                        .ownership
                        .iter()
                        .find(|item| item.id == source.id),
                );
                owner
            });
            // External parts are already placed in world coordinates. Keep that
            // placement while giving gameplay an ordinary movable scene pivot.
            commands.entity(entity).insert(Transform::from_matrix(
                source.transform().to_matrix().inverse(),
            ));
            commands.entity(owner).add_child(entity);
        }
    } else {
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
            let groups = match meshes.get(&mesh.0).ok_or_else(|| "Required collision mesh asset is missing".to_owned()).and_then(world_surface_colliders) {
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
                    ChildOf(entity), Transform::default(), Visibility::default(),
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
                if metadata["group"] == "soup" { child.insert(WorldTerrain); }
                loading.colliders += 1;
            }
        }
    }
    if root.collision && loading.colliders == 0 {
        panic!("Required collision scene contains no usable collision meshes");
    }
    loading.ready += 1;
}

fn finish_load(
    mut commands: Commands,
    roots: Query<(Entity, &MapRoot)>,
    loading: Option<ResMut<Loading>>,
    config: Res<LoadConfiguration>,
    server: Res<AssetServer>,
    mut next: ResMut<NextState<GameState>>,
) {
    let Some(loading) = loading else {
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
                .map(|c| c.root.display().to_string())
                .unwrap_or_default();
            panic!(
                "Asset root: {root}\nRequired scene {:?}: {error}",
                handle.path()
            );
        }
    }
    if loading.ready == loading.scene_count
        && loading.colliders > 0
        && loading
            .handles
            .iter()
            .all(|handle| server.is_loaded_with_dependencies(handle.id()))
    {
        info!(
            "Origins scene dependencies loaded: {} colliders",
            loading.colliders
        );
        for (entity, root) in &roots {
            if !root.collision {
                commands.entity(entity).insert(Visibility::Inherited);
            }
        }
        next.set(GameState::StartingSession);
    } else if loading.started.elapsed().as_secs() > 120 {
        panic!("Scene loading timed out or collision coverage is incomplete.");
    }
}

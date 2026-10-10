use bevy::{camera::visibility::VisibilityRange, gltf::GltfExtras, prelude::*};

pub(super) struct PropLodPlugin;

impl Plugin for PropLodPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, prepare);
    }
}

#[derive(Component)]
struct LodReady;

fn prepare(
    mut commands: Commands,
    meshes: Query<Entity, (With<Mesh3d>, Without<LodReady>)>,
    extras: Query<&GltfExtras>,
    parents: Query<&ChildOf>,
) {
    for entity in &meshes {
        let mut ancestor = Some(entity);
        while let Some(current) = ancestor {
            if let Some(lod) = extras.get(current).ok().and_then(|extras| {
                serde_json::from_str::<serde_json::Value>(&extras.value)
                    .ok()
                    .and_then(|value| {
                        value
                            .get("mesh_lod")
                            .or_else(|| value.get("prop_lod"))
                            .cloned()
                    })
            }) {
                let level = lod["level"].as_u64().unwrap_or(0);
                let distances: Vec<f32> = lod["distances"].as_array().map_or_else(
                    || match lod["levels"].as_u64().unwrap_or(1) {
                        2 => vec![20.0],
                        3 => vec![20.0, 45.0],
                        _ => Vec::new(),
                    },
                    |values| {
                        values
                            .iter()
                            .filter_map(|v| v.as_f64().map(|d| d as f32))
                            .collect()
                    },
                );
                let margin = |index: usize| {
                    let distance = distances[index];
                    let previous = index.checked_sub(1).map_or(0.0, |i| distances[i]);
                    let next = distances.get(index + 1).copied().unwrap_or(f32::MAX);
                    let width = (distance * 0.025)
                        .min(0.5)
                        .min((distance - previous) * 0.25)
                        .min((next - distance) * 0.25)
                        .max(0.0);
                    (distance - width).max(0.0)..distance + width
                };
                commands.entity(entity).try_insert(VisibilityRange {
                    start_margin: level
                        .checked_sub(1)
                        .map(|index| index as usize)
                        .filter(|index| *index < distances.len())
                        .map_or(0.0..0.0, margin),
                    end_margin: distances
                        .get(level as usize)
                        .map_or(f32::MAX..f32::MAX, |_| margin(level as usize)),
                    use_aabb: false,
                });
                break;
            }
            ancestor = parents.get(current).ok().map(ChildOf::parent);
        }
        commands.entity(entity).try_insert(LodReady);
    }
}

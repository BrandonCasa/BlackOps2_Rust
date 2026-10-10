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
                    .and_then(|value| value.get("prop_lod").cloned())
            }) {
                let level = lod["level"].as_u64().unwrap_or(0);
                let levels = lod["levels"].as_u64().unwrap_or(1);
                commands.entity(entity).try_insert(VisibilityRange {
                    start_margin: match level {
                        0 => 0.0..0.0,
                        1 => 19.0..21.0,
                        _ => 44.0..46.0,
                    },
                    end_margin: if level + 1 == levels {
                        f32::MAX..f32::MAX
                    } else if level == 0 {
                        19.0..21.0
                    } else {
                        44.0..46.0
                    },
                    use_aabb: false,
                });
                break;
            }
            ancestor = parents.get(current).ok().map(ChildOf::parent);
        }
        commands.entity(entity).try_insert(LodReady);
    }
}

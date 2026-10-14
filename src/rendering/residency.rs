use super::opaque::{OpaqueSurfaceStatus, SurfaceLibrary};
use crate::{GameState, assets::WorldCollider};
use bevy::{asset::RenderAssetUsages, prelude::*};

pub(super) struct ResidencyPlugin;
impl Plugin for ResidencyPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PostUpdate, release.before(bevy::asset::AssetEventSystems));
    }
}

fn release(
    mut commands: Commands,
    state: Res<State<GameState>>,
    status: Res<OpaqueSurfaceStatus>,
    lights: Query<(&GlobalTransform, &PointLight, &ViewVisibility)>,
    cameras: Query<&GlobalTransform, With<super::GameCamera>>,
    colliders: Query<(Entity, &Mesh3d), With<WorldCollider>>,
    server: Res<AssetServer>,
    library: Option<Res<SurfaceLibrary>>,
    mut images: ResMut<Assets<Image>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut finished: Local<bool>,
) {
    if **state == GameState::Loading {
        *finished = false;
        return;
    }
    if *finished || !status.ready() {
        return;
    }
    let Some(library) = library else {
        return;
    };
    if let Ok(camera) = cameras.single() {
        info!(
            "Lantern allocation: {} total, {} view-visible, {} affect camera position; camera {:?}",
            lights.iter().count(),
            lights
                .iter()
                .filter(|(_, _, visible)| visible.get())
                .count(),
            lights
                .iter()
                .filter(
                    |(pose, light, _)| pose.translation().distance(camera.translation())
                        < light.range
                )
                .count(),
            camera.translation()
        );
    }
    let mut image_bytes = 0;
    let mut image_count = 0;
    let image_ids: Vec<_> = images
        .iter()
        .filter_map(|(id, image)| {
            (server.get_path(id).is_some()
                && image.texture_descriptor.size.depth_or_array_layers == 1
                && image.data.is_some()
                && image.asset_usage.contains(RenderAssetUsages::MAIN_WORLD))
            .then_some(id)
        })
        .collect();
    for id in image_ids {
        if let Some(mut image) = images.get_mut(id) {
            image_bytes += image.data.as_ref().map_or(0, Vec::len);
            image_count += 1;
            image.asset_usage = RenderAssetUsages::RENDER_WORLD;
        }
    }
    let mut mesh_bytes = 0;
    let mut mesh_count = 0;
    let mesh_ids: Vec<_> = meshes
        .iter()
        .filter_map(|(id, mesh)| {
            (server.get_path(id).is_some()
                && library.prepared_meshes.contains(&id)
                && !library.live_meshes.contains(&id)
                && mesh.asset_usage.contains(RenderAssetUsages::MAIN_WORLD)
                && mesh.attribute(Mesh::ATTRIBUTE_JOINT_INDEX).is_none()
                && mesh.try_attributes().is_ok())
            .then_some(id)
        })
        .collect();
    for id in mesh_ids {
        let Some(mut mesh) = meshes.get_mut(id) else {
            continue;
        };
        mesh_bytes += mesh.try_attributes().map_or(0, |attributes| {
            attributes
                .map(|(_, values)| values.get_bytes().len())
                .sum::<usize>()
        });
        mesh_bytes += mesh
            .try_indices_option()
            .ok()
            .flatten()
            .map_or(0, |indices| match indices {
                bevy::mesh::Indices::U16(values) => values.len() * 2,
                bevy::mesh::Indices::U32(values) => values.len() * 4,
            });
        mesh_count += 1;
        mesh.asset_usage = RenderAssetUsages::RENDER_WORLD;
    }
    for (entity, _) in &colliders {
        commands
            .entity(entity)
            .remove::<(Mesh3d, MeshMaterial3d<StandardMaterial>)>()
            .remove_parent_in_place();
    }
    info!(
        "Released preparation data: {image_count} images ({image_bytes} CPU bytes), {mesh_count} meshes ({mesh_bytes} CPU bytes)"
    );
    *finished = true;
}

use super::opaque::{self, OpaqueSurfaceStatus, SurfaceLibrary};
use crate::{GameState, assets::WorldCollider};
use bevy::{asset::RenderAssetUsages, prelude::*};
use std::collections::HashSet;

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
    active_materials: Query<&MeshMaterial3d<StandardMaterial>, Without<WorldCollider>>,
    server: Res<AssetServer>,
    mut library: Option<ResMut<SurfaceLibrary>>,
    mut images: ResMut<Assets<Image>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut finished: Local<bool>,
) {
    if *finished || **state == GameState::Loading || !status.ready() {
        return;
    }
    let Some(ref mut library) = library else {
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
    opaque::release_preparation(library);
    let active: HashSet<_> = active_materials
        .iter()
        .map(|material| material.0.id())
        .collect();
    let mut source_materials = 0;
    let source_ids: Vec<_> = materials
        .iter()
        .filter_map(|(id, _)| {
            (!active.contains(&id)
                && opaque::converted_source(library, id)
                && server.get_path(id).is_some())
            .then_some(id)
        })
        .collect();
    for id in source_ids {
        if let Some(mut material) = materials.get_mut(id) {
            material.base_color_texture = None;
            material.normal_map_texture = None;
            material.metallic_roughness_texture = None;
            material.emissive_texture = None;
            material.occlusion_texture = None;
            material.clearcoat_texture = None;
            source_materials += 1;
        }
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
        "Released preparation data: {image_count} images ({image_bytes} CPU bytes), {mesh_count} meshes ({mesh_bytes} CPU bytes), {source_materials} source material texture owners"
    );
    *finished = true;
}

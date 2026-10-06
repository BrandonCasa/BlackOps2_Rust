//! Release preparation data while retaining Bevy's GPU assets and Avian colliders.
use super::opaque::{self, OpaqueSurfaceStatus, SurfaceLibrary};
use crate::{GameState, assets::WorldCollider};
use bevy::{asset::RenderAssetUsages, light::GeneratedEnvironmentMapLight, prelude::*};
use std::collections::HashSet;

pub(super) struct ResidencyPlugin;
impl Plugin for ResidencyPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Last, release);
    }
}

fn release(
    mut commands: Commands,
    state: Res<State<GameState>>,
    status: Res<OpaqueSurfaceStatus>,
    generators: Query<(), With<GeneratedEnvironmentMapLight>>,
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
    if *finished || **state == GameState::Loading || !status.ready() || !generators.is_empty() {
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
    let generated_images: HashSet<_> = opaque::release_preparation(library).into_iter().collect();
    let active: HashSet<_> = active_materials
        .iter()
        .map(|material| material.0.id())
        .collect();
    let mut source_materials = 0;
    for (id, material) in materials.iter_mut() {
        if !active.contains(&id) && server.get_path(id).is_some() {
            // Converted extended materials already own the textures they use.
            // Source scene templates must not retain unused duplicate maps.
            material.base_color_texture = None;
            material.normal_map_texture = None;
            material.metallic_roughness_texture = None;
            material.emissive_texture = None;
            material.occlusion_texture = None;
            source_materials += 1;
        }
    }
    let mut image_bytes = 0;
    let mut image_count = 0;
    for (id, image) in images.iter_mut() {
        // Generated font atlases remain CPU-accessible for new HUD glyphs.
        if (server.get_path(id).is_some() || generated_images.contains(&id)) && image.data.is_some()
        {
            image_bytes += image.data.as_ref().map_or(0, Vec::len);
            image_count += 1;
            image.asset_usage = RenderAssetUsages::RENDER_WORLD;
        }
    }
    let mut mesh_bytes = 0;
    let mut mesh_count = 0;
    for (id, mesh) in meshes.iter_mut() {
        // Late scene/weapon meshes may already be loaded without having had
        // their surface attributes adapted. Release only completed providers.
        if server.get_path(id).is_none() || !library.prepared_meshes.contains(&id) {
            continue;
        }
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
        // Let the engine move data during extraction; taking it here would
        // invalidate the pending asset upload associated with this mutation.
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

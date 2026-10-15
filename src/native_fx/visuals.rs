//! Stock Bevy bindings for source FX visual entities.
use crate::assets::GameDefinition;
use bevy::{
    pbr::decal::{ForwardDecal, ForwardDecalMaterial, ForwardDecalMaterialExt},
    prelude::*,
};

pub(super) fn model(
    commands: &mut Commands,
    server: &AssetServer,
    entity: Entity,
    model: &str,
) -> Result<Handle<bevy::world_serialization::WorldAsset>, String> {
    let model = model.trim_start_matches(',');
    if model.is_empty() || model.contains(['/', '\\']) || model.contains("..") {
        return Err(format!(
            "Invalid required source FX model identity: {model}"
        ));
    }
    // The exporter preserves exact source XModel names; dependency failures are
    // checked by the owning particle system through this ordinary scene handle.
    let scene = server.load(format!("model_export/{model}_lod0.glb#Scene0"));
    commands.spawn((
        ChildOf(entity),
        Transform::default(),
        Visibility::default(),
        WorldAssetRoot(scene.clone()),
    ));
    Ok(scene)
}

pub(super) fn light(commands: &mut Commands, entity: Entity) {
    commands.entity(entity).insert(PointLight {
        shadow_maps_enabled: false,
        ..default()
    });
}

pub(super) fn sound(
    commands: &mut Commands,
    game: &GameDefinition,
    name: &str,
    position: Vec3,
) -> Result<(), String> {
    let name = name.trim_start_matches(',');
    if crate::audio::resolve_alias(game, name).is_none() {
        return Err(format!(
            "Required source effect sound alias is missing: {name}"
        ));
    }
    crate::audio::emit(commands, name, crate::audio::SoundSource::World(position));
    Ok(())
}

pub(super) fn decal(
    commands: &mut Commands,
    materials: &mut Assets<ForwardDecalMaterial<StandardMaterial>>,
    entity: Entity,
    material: StandardMaterial,
) -> Handle<ForwardDecalMaterial<StandardMaterial>> {
    let handle = materials.add(ForwardDecalMaterial {
        base: material,
        // No source depth-feather shader is reconstructed.
        extension: ForwardDecalMaterialExt {
            depth_fade_factor: 0.,
        },
    });
    commands
        .entity(entity)
        .insert((ForwardDecal, MeshMaterial3d(handle.clone())));
    handle
}

//! Source FX model dependencies.
use bevy::prelude::*;

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

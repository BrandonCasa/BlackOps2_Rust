//! Source FX model dependencies.
use bevy::prelude::*;

pub(super) fn model(
    commands: &mut Commands,
    server: &AssetServer,
    entity: Entity,
    model: &str,
) -> Result<(), String> {
    let model = model.trim_start_matches(',');
    if model.is_empty() || model.contains(['/', '\\']) || model.contains("..") {
        return Err(format!(
            "Invalid required source FX model identity: {model}"
        ));
    }
    // The scene owns its handle; GameAssetsPlugin reports asset-load failures.
    let scene = server.load(format!("model_export/{model}_lod0.glb#Scene0"));
    commands.spawn((
        ChildOf(entity),
        // The shared model loader owns source-unit normalization.
        Transform::IDENTITY,
        Visibility::default(),
        WorldAssetRoot(scene),
    ));
    Ok(())
}

use bevy::{
    prelude::*,
    world_serialization::{WorldAssetRoot, WorldInstanceReady, WorldInstanceSpawner},
};
use std::collections::HashSet;
#[derive(Component)]
struct SourceModelRoot;
pub(super) fn install(app: &mut App) {
    app.add_observer(normalize);
}
fn normalize(
    event: On<WorldInstanceReady>,
    mut commands: Commands,
    roots: Query<
        &WorldAssetRoot,
        (
            Without<crate::assets::MapRoot>,
            Without<crate::assets::StaticPropRoot>,
        ),
    >,
    parents: Query<&ChildOf>,
    normalized: Query<(), With<SourceModelRoot>>,
    spawner: Res<WorldInstanceSpawner>,
    mut transforms: Query<&mut Transform>,
) {
    let Ok(root) = roots.get(event.entity) else {
        return;
    };
    let Some(path) = root.0.path() else { return };
    if !path.path().starts_with("model_export") {
        return;
    }
    if parents
        .iter_ancestors(event.entity)
        .any(|ancestor| normalized.contains(ancestor))
    {
        return;
    }
    let entities: HashSet<_> = spawner.iter_instance_entities(event.instance_id).collect();
    for entity in &entities {
        if parents
            .get(*entity)
            .is_ok_and(|parent| entities.contains(&parent.parent()))
        {
            continue;
        }
        if normalized.contains(*entity) {
            continue;
        }
        if let Ok(mut transform) = transforms.get_mut(*entity) {
            transform.scale *= crate::assets::SOURCE_UNIT_METERS;
            commands.entity(*entity).insert(SourceModelRoot);
        }
    }
}

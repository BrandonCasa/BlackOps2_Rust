use crate::{
    GameState,
    assets::GameDefinition,
    origins::content::WeaponId,
};
use bevy::prelude::*;
#[derive(Component)]
pub(crate) struct WeaponMaterialContext(pub WeaponId);
#[derive(Component)]
struct Applied(String);
#[derive(Component)]
pub(crate) struct CamoTexture {
    path: String,
    image: Handle<Image>,
}

pub(super) fn install(app: &mut App) {
    app.add_systems(Update, (apply, check_dependencies).chain());
}
fn apply(
    mut commands: Commands,
    game: Option<Res<GameDefinition>>,
    server: Res<AssetServer>,
    contexts: Query<&WeaponMaterialContext>,
    parents: Query<&ChildOf>,
    surfaces: Query<(Entity, &MeshMaterial3d<StandardMaterial>, Option<&Applied>)>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let Some(game) = game else { return };
    for (entity, handle, applied) in &surfaces {
        let mut owner = entity;
        let context = loop {
            if let Ok(context) = contexts.get(owner) {
                break Some(context);
            };
            let Ok(parent) = parents.get(owner) else {
                break None;
            };
            owner = parent.parent();
        };
        let Some(context) = context else { continue };
        let Some(texture) = game.camos.get(&context.0.0) else {
            continue;
        };
        if applied.is_some_and(|a| a.0 == *texture) {
            continue;
        }
        let Some(source) = materials.get(&handle.0) else {
            continue;
        };
        let mut material = source.clone();
        let image = server.load(texture.clone());
        material.base_color_texture = Some(image.clone());
        commands.entity(entity).insert((
            MeshMaterial3d(materials.add(material)),
            Applied(texture.clone()),
            CamoTexture {
                path: texture.clone(),
                image,
            },
        ));
    }
}

pub(crate) fn assets_ready(
    server: &AssetServer,
    textures: &Query<&CamoTexture>,
) -> Result<bool, String> {
    let mut ready = true;
    for texture in textures {
        match server.get_load_state(texture.image.id()) {
            Some(bevy::asset::LoadState::Failed(error)) => {
                return Err(format!("Required camo texture {}: {error}", texture.path));
            }
            Some(bevy::asset::LoadState::Loaded) => {}
            _ => ready = false,
        }
    }
    Ok(ready)
}
fn check_dependencies(
    server: Res<AssetServer>,
    textures: Query<&CamoTexture>,

) {
    if let Err(error) = assets_ready(&server, &textures) {
        panic!("{}", error);
    }
}

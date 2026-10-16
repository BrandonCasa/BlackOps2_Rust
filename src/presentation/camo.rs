use crate::{
    rendering::native::NativeWorldState,
    rendering::native_model::{NativeModelMaterial, NativeModelSurface},
};
use bevy::prelude::*;

/// An eligible original material pair, submitted by the weapon/attachment
/// owner after its native descriptor/model mask and transfer flags resolve.
/// Replacement owns the actual destination constants, delivered weaponParam6
/// and preserved weapon normal. One image path cannot express that contract.
#[derive(Component)]
pub(crate) struct NativeCamoOverride {
    pub source: (Handle<NativeModelMaterial>, NativeWorldState),
    pub replacement: Option<(Handle<NativeModelMaterial>, NativeWorldState)>,
}

#[derive(Component)]
pub(crate) struct CamoTexture {
    images: Vec<Handle<Image>>,
}

pub(super) fn install(app: &mut App) {
    app.add_systems(Update, apply);
}

fn apply(
    mut commands: Commands,
    mut surfaces: Query<
        (Entity, &NativeCamoOverride, &mut NativeModelSurface),
        Changed<NativeCamoOverride>,
    >,
    materials: Res<Assets<NativeModelMaterial>>,
) {
    for (entity, selection, mut surface) in &mut surfaces {
        let Some((replacement, state)) = &selection.replacement else {
            surface.material = selection.source.0.clone();
            surface.state = selection.source.1;
            commands.entity(entity).remove::<CamoTexture>();
            continue;
        };
        let material = materials
            .get(replacement)
            .expect("submit a selected original camo after binding all required material inputs");
        assert!(
            matches!(material, NativeModelMaterial::Camo3Layer { .. }),
            "port the selected original camo program before submitting its replacement"
        );
        surface.material = replacement.clone();
        surface.state = *state;
        commands.entity(entity).insert(CamoTexture {
            images: material.images().cloned().collect(),
        });
    }
}

pub(crate) fn assets_ready(server: &AssetServer, textures: &Query<&CamoTexture>) -> bool {
    textures.iter().all(|camo| {
        camo.images
            .iter()
            .all(|image| server.is_loaded_with_dependencies(image.id()))
    })
}

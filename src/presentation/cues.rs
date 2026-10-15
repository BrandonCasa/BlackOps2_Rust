use bevy::prelude::*;
#[derive(Component, Default)]
pub(crate) struct CueParameters {
    pub script_vectors: [[f32; 4]; 4],
    pub script_available: u8,
    pub luminous_channel: Option<(usize, usize)>,
}
pub(super) fn install(app: &mut App) {
    app.add_systems(PostUpdate, (bind_materials, present).chain());
}

#[derive(Component)]
struct CueMaterial {
    owner: Entity,
    source: StandardMaterial,
    target: Handle<StandardMaterial>,
    eyes: bool,
}
fn bind_materials(
    mut commands: Commands,
    surfaces: Query<
        (
            Entity,
            &MeshMaterial3d<StandardMaterial>,
            Option<&bevy::gltf::GltfMaterialName>,
        ),
        Without<CueMaterial>,
    >,
    owners: Query<(), With<CueParameters>>,
    parents: Query<&ChildOf>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for (entity, handle, name) in &surfaces {
        let mut ancestor = entity;
        let owner = loop {
            if owners.contains(ancestor) {
                break Some(ancestor);
            };
            let Ok(parent) = parents.get(ancestor) else {
                break None;
            };
            ancestor = parent.parent();
        };
        let Some(owner) = owner else { continue };
        let Some(mut source) = materials.get(&handle.0).cloned() else {
            continue;
        };
        let eyes = name.is_some_and(|name| name.0.to_ascii_lowercase().contains("eye"));
        if !eyes && source.emissive == LinearRgba::BLACK {
            continue;
        }
        if eyes && source.emissive == LinearRgba::BLACK {
            source.emissive = source.base_color.to_linear();
            source.emissive_texture = source.base_color_texture.clone();
        }
        let target = materials.add(source.clone());
        commands.entity(entity).insert((
            MeshMaterial3d(target.clone()),
            CueMaterial {
                owner,
                source,
                target,
                eyes,
            },
        ));
    }
}
fn present(
    surfaces: Query<&CueMaterial>,
    owners: Query<&CueParameters>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for surface in &surfaces {
        let Ok(cue) = owners.get(surface.owner) else {
            continue;
        };
        let Some(mut material) = materials.get_mut(&surface.target) else {
            continue;
        };
        material.emissive = surface.source.emissive;
        let Some((slot, component)) = cue.luminous_channel else {
            continue;
        };
        if cue.script_available & (1 << slot) == 0 || (slot == 2 && !surface.eyes) {
            continue;
        }
        let value = cue.script_vectors[slot][component].max(0.);
        material.emissive = surface.source.emissive * if slot == 2 { value * value } else { value };
    }
}

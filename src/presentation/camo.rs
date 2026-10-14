use super::Rig;
use crate::{
    GameState,
    assets::{LoadConfiguration, LoadFailure},
    origins::content::WeaponId,
    rendering::SurfacePreparation,
};
use bevy::{
    gltf::{GltfMaterialExtras, GltfMaterialName},
    prelude::*,
};
use serde::Deserialize;
use serde_json::Value;
use std::collections::{BTreeMap, HashSet};

#[derive(Component)]
pub(crate) struct WeaponMaterialContext(pub WeaponId);

#[derive(Component)]
struct Applied {
    weapon: Option<WeaponId>,
    name: String,
    material: Handle<StandardMaterial>,
    extras: Option<GltfMaterialExtras>,
}

#[derive(Resource, Deserialize)]
struct Library {
    version: u32,
    camo_id: u32,
    surface_version: u32,
    weapons: BTreeMap<String, BTreeMap<String, String>>,
    surfaces: BTreeMap<String, Value>,
    pending_bakes: BTreeMap<String, String>,
}

pub(super) fn install(app: &mut App) {
    app.add_systems(OnEnter(GameState::Loading), reset);
    app.add_systems(Update, load.before(apply))
        .add_systems(Update, apply.before(SurfacePreparation));
}

fn reset(mut commands: Commands, surfaces: Query<(Entity, &Applied)>) {
    commands.remove_resource::<Library>();
    for (entity, applied) in &surfaces {
        let mut target = commands.entity(entity);
        target.insert((
            MeshMaterial3d(applied.material.clone()),
            GltfMaterialName(applied.name.clone()),
        ));
        if let Some(extras) = &applied.extras {
            target.insert(extras.clone());
        } else {
            target.remove::<GltfMaterialExtras>();
        }
        crate::rendering::opaque::reset_binding(&mut target);
        target.remove::<Applied>();
    }
}

fn load(
    mut commands: Commands,
    configuration: Res<LoadConfiguration>,
    art: Option<Res<crate::assets::prepared::PreparedArt>>,
    game: Option<Res<crate::assets::GameDefinition>>,
    library: Option<Res<Library>>,
    rigs: Query<&Rig>,
    contexts: Query<&WeaponMaterialContext>,
    mut failure: ResMut<LoadFailure>,
    mut next: ResMut<NextState<GameState>>,
    mut upgraded_ids: Local<HashSet<WeaponId>>,
) {
    let Some(game) = game else { return };
    if game.is_changed() || upgraded_ids.is_empty() {
        *upgraded_ids = game
            .content
            .weapons
            .iter()
            .filter_map(|weapon| weapon.upgraded.clone())
            .collect();
    }
    let mut required = rigs
        .iter()
        .filter(|rig| !rig.arms && !rig.knife)
        .map(|rig| &rig.weapon)
        .chain(contexts.iter().map(|context| &context.0))
        .filter(|id| upgraded_ids.contains(*id))
        .peekable();
    if required.peek().is_none() {
        return;
    }
    if let Some(library) = library {
        if let Some((id, reason)) =
            required.find_map(|id| library.pending_bakes.get(&id.0).map(|reason| (id, reason)))
        {
            failure.0 = format!("Native weapon camo {} is not prepared: {reason}", id.0);
            error!("{}", failure.0);
            next.set(GameState::LoadError);
        }
        return;
    }
    let Some(art) = art else { return };
    let Ok(pack) = &configuration.0 else { return };
    let result = (|| {
        let uri = art
            .camo_uri()
            .ok_or("Prepared native weapon camo library is missing")?;
        crate::origins::validate_asset_path(uri).map_err(|error| error.to_string())?;
        let bytes = pack.contents.get(uri).map_err(|error| error.to_string())?;
        let library: Library = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
        if library.version != 2 || library.camo_id != 45 || library.surface_version != 3 {
            return Err("Unsupported native Origins weapon camo metadata".into());
        }
        for overrides in library.weapons.values() {
            for identity in overrides.values() {
                let surface = library
                    .surfaces
                    .get(identity)
                    .ok_or_else(|| format!("Missing native weapon camo material {identity}"))?;
                if surface["identity"] != *identity
                    || surface["version"] != 3
                    || surface["brdf"] != "ggx"
                {
                    return Err(format!("Invalid native weapon camo material {identity}"));
                }
            }
        }
        Ok(library)
    })();
    match result {
        Ok(library) => {
            commands.insert_resource(library);
        }
        Err(error) => {
            failure.0 = format!("Native weapon camos: {error}");
            error!("{}", failure.0);
            next.set(GameState::LoadError);
        }
    }
}

fn apply(
    mut commands: Commands,
    library: Option<Res<Library>>,
    entities: Query<
        (
            Entity,
            &MeshMaterial3d<StandardMaterial>,
            &GltfMaterialName,
            Option<&GltfMaterialExtras>,
            Option<&Applied>,
        ),
        Without<crate::assets::WorldCollider>,
    >,
    parents: Query<&ChildOf>,
    rigs: Query<&Rig>,
    contexts: Query<&WeaponMaterialContext>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let Some(library) = library else { return };
    for (entity, handle, name, extras, applied) in &entities {
        let mut ancestor = Some(entity);
        let mut weapon = None;
        while let Some(current) = ancestor {
            if let Ok(rig) = rigs.get(current) {
                if !rig.arms && !rig.knife {
                    weapon = Some(&rig.weapon);
                }
                break;
            }
            if let Ok(context) = contexts.get(current) {
                weapon = Some(&context.0);
                break;
            }
            ancestor = parents.get(current).ok().map(ChildOf::parent);
        }
        if applied.is_some_and(|applied| applied.weapon.as_ref() == weapon) {
            continue;
        }
        let source_name = applied.map_or(name.0.as_str(), |applied| applied.name.as_str());
        let source_extras = applied.map_or(extras, |applied| applied.extras.as_ref());
        let replacement = weapon
            .and_then(|weapon| library.weapons.get(&weapon.0))
            .and_then(|overrides| overrides.get(source_name));
        let staff = source_extras
            .and_then(|e| serde_json::from_str::<Value>(&e.value).ok())
            .is_some_and(|v| v["surface"]["native_weapon"]["kind"] == "staff");
        if replacement.is_none() && !staff {
            if let Some(applied) = applied {
                let mut target = commands.entity(entity);
                target.insert((
                    MeshMaterial3d(applied.material.clone()),
                    GltfMaterialName(applied.name.clone()),
                ));
                if let Some(extras) = &applied.extras {
                    target.insert(extras.clone());
                } else {
                    target.remove::<GltfMaterialExtras>();
                }
                crate::rendering::opaque::reset_binding(&mut target);
                target.remove::<Applied>();
            }
            continue;
        }
        let identity = replacement.map(String::as_str).unwrap_or(source_name);
        let source_material = applied.map_or(&handle.0, |applied| &applied.material);
        let Some(original) = materials.get(source_material).cloned() else {
            continue;
        };
        let mut value = source_extras
            .and_then(|e| serde_json::from_str::<Value>(&e.value).ok())
            .unwrap_or_else(|| serde_json::json!({}));
        if let Some(surface) = library.surfaces.get(identity) {
            value["surface"] = surface.clone();
        }
        let applied = Applied {
            weapon: weapon.cloned(),
            name: source_name.into(),
            material: source_material.clone(),
            extras: source_extras.cloned(),
        };
        let mut target = commands.entity(entity);
        crate::rendering::opaque::reset_binding(&mut target);
        target.try_insert((
            MeshMaterial3d(materials.add(original)),
            GltfMaterialName(identity.into()),
            GltfMaterialExtras {
                value: value.to_string(),
            },
            applied,
        ));
    }
}

use super::Rig;
use crate::{
    GameState,
    assets::{LoadConfiguration, LoadFailure},
    origins::{camo, content::WeaponId},
    rendering::SurfacePreparation,
};
use bevy::{
    gltf::{GltfExtras, GltfMaterialExtras, GltfMaterialName},
    prelude::*,
};
use serde_json::Value;
use std::collections::HashSet;

#[derive(Component)]
pub(crate) struct WeaponMaterialContext(pub WeaponId);

#[derive(Component)]
struct Applied {
    inputs: Value,
    identity: String,
    name: String,
    material: Handle<StandardMaterial>,
    extras: Option<GltfMaterialExtras>,
}

#[derive(Resource)]
struct Library {
    native: camo::Library,
    upgraded: HashSet<WeaponId>,
}

#[derive(Clone, PartialEq)]
struct Selection {
    weapon: WeaponId,
    options: u32,
    packed: u32,
    mode: u8,
    main_index: usize,
    initial_model: String,
}

#[derive(Component)]
struct Submission {
    selection: Selection,
    models: Vec<String>,
    descriptors: Vec<camo::CamoDescriptor>,
}

pub(super) fn install(app: &mut App) {
    app.add_systems(OnEnter(GameState::Loading), reset)
        .add_systems(
            Update,
            (load, submit, apply)
                .chain()
                .after(super::PresentationAnimation)
                .before(SurfacePreparation),
        );
}

fn restore(target: &mut bevy::ecs::system::EntityCommands, applied: &Applied) {
    target.insert((
        MeshMaterial3d(applied.material.clone()),
        GltfMaterialName(applied.name.clone()),
    ));
    if let Some(extras) = &applied.extras {
        target.insert(extras.clone());
    } else {
        target.remove::<GltfMaterialExtras>();
    }
    crate::rendering::opaque::reset_binding(target);
    target.remove::<Applied>();
}

fn reset(
    mut commands: Commands,
    surfaces: Query<(Entity, &Applied)>,
    submissions: Query<Entity, With<Submission>>,
) {
    commands.remove_resource::<Library>();
    for (entity, applied) in &surfaces {
        restore(&mut commands.entity(entity), applied);
    }
    for entity in &submissions {
        commands.entity(entity).remove::<Submission>();
    }
}

fn load(
    mut commands: Commands,
    configuration: Res<LoadConfiguration>,
    art: Option<Res<crate::assets::prepared::PreparedArt>>,
    game: Option<Res<crate::assets::GameDefinition>>,
    library: Option<Res<Library>>,
    mut failure: ResMut<LoadFailure>,
    mut next: ResMut<NextState<GameState>>,
) {
    if library.is_some() {
        return;
    }
    let (Some(art), Some(game), Ok(pack)) = (art, game, &configuration.0) else {
        return;
    };
    let result = (|| {
        let uri = art
            .camo_uri()
            .ok_or("Prepared native camo library is missing")?;
        crate::origins::validate_asset_path(uri).map_err(|error| error.to_string())?;
        let bytes = pack.contents.get(uri).map_err(|error| error.to_string())?;
        let native: camo::Library =
            serde_json::from_slice(&bytes).map_err(|error| error.to_string())?;
        native.validate()?;
        for variant in &native.variants {
            if let Some(surface) = &variant.surface {
                crate::rendering::validate_surface(surface)?;
            }
        }
        Ok::<_, String>(Library {
            native,
            upgraded: game
                .content
                .weapons
                .iter()
                .filter_map(|weapon| weapon.upgraded.clone())
                .collect(),
        })
    })();
    match result {
        Ok(library) => {
            commands.insert_resource(library);
        }
        Err(error) => {
            failure.0 = format!("Native weapon camos: {error}");
            next.set(GameState::LoadError);
        }
    }
}

fn submit(
    mut commands: Commands,
    library: Option<Res<Library>>,
    art: Option<Res<crate::assets::prepared::PreparedArt>>,
    game: Option<Res<crate::assets::GameDefinition>>,
    server: Res<AssetServer>,
    rigs: Query<(Entity, &Rig, Option<&Submission>)>,
    inventories: Query<&crate::weapons::WeaponInventory>,
    mut failure: ResMut<LoadFailure>,
    mut next: ResMut<NextState<GameState>>,
) {
    let (Some(library), Some(art), Some(game)) = (library, art, game) else {
        return;
    };
    for (entity, rig, current) in &rigs {
        if rig.arms || rig.knife || rig.left {
            continue;
        }
        let Some(inventory) = inventories.get(rig.owner).ok() else {
            if current.is_some() {
                commands.entity(entity).remove::<Submission>();
            }
            continue;
        };
        let active = inventory
            .active()
            .is_some_and(|weapon| weapon.definition == rig.weapon);
        let options = if active && library.upgraded.contains(&rig.weapon) {
            45
        } else {
            0
        };
        if options == 0 {
            if current.is_some() {
                commands.entity(entity).remove::<Submission>();
            }
            continue;
        }
        let result: Result<Option<Submission>, String> = (|| {
            if rig.dual {
                return Err(
                    "Native dual-hand camo descriptors require alternate-hand model selection"
                        .into(),
                );
            }
            let weapon =
                library.native.weapons.get(&rig.weapon.0).ok_or_else(|| {
                    format!("Missing native weapon selectors for {}", rig.weapon.0)
                })?;
            let definition = game
                .content
                .weapons
                .iter()
                .find(|weapon| weapon.id == rig.weapon)
                .ok_or("Active weapon definition is missing")?;
            let stock = rigs
                .iter()
                .find(|(_, other, _)| {
                    other.owner == rig.owner
                        && other.weapon == rig.weapon
                        && !other.arms
                        && !other.knife
                        && !other.ads
                        && !other.left
                })
                .ok_or("Native main viewmodel owner is missing")?
                .1;
            let path = server
                .get_path(stock.source.id())
                .ok_or("Native main viewmodel asset path is missing")?;
            let model = art
                .native_model(&path.to_string())
                .ok_or("Native main viewmodel identity is missing")?;
            let indices: Vec<_> = weapon
                .main_models
                .iter()
                .enumerate()
                .filter_map(|(index, name)| (name.as_deref() == Some(model)).then_some(index))
                .collect();
            let main_index = if let Some(index) = definition.native_view_model_index {
                let index = index as usize;
                if !indices.contains(&index) {
                    return Err(
                        "Held native viewmodel index disagrees with its prepared model".into(),
                    );
                }
                index
            } else if indices.len() == 1 {
                indices[0]
            } else {
                return Err("Held native viewmodel index is not uniquely recovered".into());
            };
            let arms = rigs
                .iter()
                .find(|(_, other, _)| {
                    other.owner == rig.owner
                        && other.weapon == rig.weapon
                        && other.arms
                        && !other.left
                })
                .ok_or("Native viewmodel slot-zero owner is missing")?
                .1;
            let path = server
                .get_path(arms.source.id())
                .ok_or("Native viewmodel slot-zero asset path is missing")?;
            let initial_model = art
                .native_model(&path.to_string())
                .ok_or("Native viewmodel slot-zero identity is missing")?
                .to_owned();
            let selectors = if inventory.attachment() == "defaultattachment" {
                [0; 3]
            } else {
                definition.attachments.get(inventory.attachment()).and_then(|attachment| attachment.native_selectors)
                    .ok_or("Active attachment configuration lacks its recovered native selector triple")?
            };
            if selectors.iter().any(|selector| *selector > 63) {
                return Err("Native attachment selector exceeds six bits".into());
            }
            let packed = selectors
                .into_iter()
                .zip([8, 14, 20])
                .fold(0, |packed, (selector, shift)| {
                    packed | ((selector as u32) << shift)
                });
            if !inventory.ads.is_finite() {
                return Err("Native optic zoom fraction is non-finite".into());
            }
            let selection = Selection {
                weapon: rig.weapon.clone(),
                options,
                packed,
                mode: u8::from(inventory.ads >= library.native.optic_switch),
                main_index,
                initial_model,
            };
            if current.is_some_and(|current| current.selection == selection) {
                return Ok(None);
            }
            if weapon.definition44 == 3 {
                return Err(
                    "Native alternate packed-weapon role is not connected to the active owner"
                        .into(),
                );
            }
            let plan = camo::viewmodel_materials(
                weapon,
                None,
                &library.native.attachments,
                &selection.initial_model,
                packed,
                0,
                main_index,
                selection.mode,
            )?;
            Ok(Some(Submission {
                selection,
                models: plan.models,
                descriptors: plan.descriptors,
            }))
        })();
        match result {
            Ok(Some(submission)) => {
                commands.entity(entity).insert(submission);
            }
            Ok(None) => (),
            Err(error) => {
                failure.0 = format!("Native camo owner {}: {error}", rig.weapon.0);
                next.set(GameState::LoadError);
                return;
            }
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
    extras: Query<&GltfExtras>,
    rigs: Query<&Rig>,
    submissions: Query<&Submission>,
    contexts: Query<&WeaponMaterialContext>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut failure: ResMut<LoadFailure>,
    mut next: ResMut<NextState<GameState>>,
) {
    let Some(library) = library else {
        return;
    };
    for (entity, handle, name, material_extras, applied) in &entities {
        let previous = applied;
        let applied = applied.filter(|applied| applied.identity == name.0);
        if previous.is_some() && applied.is_none() {
            commands.entity(entity).remove::<Applied>();
        }
        let original = applied.map_or(name.0.as_str(), |applied| applied.name.as_str());
        if !library.native.materials.contains_key(original) {
            continue;
        }
        let mut ancestor = Some(entity);
        let mut model = None;
        let mut submission = None;
        let mut unconnected = None;
        while let Some(current) = ancestor {
            if model.is_none() {
                model = extras
                    .get(current)
                    .ok()
                    .and_then(|extra| serde_json::from_str::<Value>(&extra.value).ok())
                    .and_then(|value| value["source_model"].as_str().map(str::to_owned));
            }
            if let Ok(state) = submissions.get(current) {
                submission = Some(state);
                break;
            }
            if let Ok(rig) = rigs.get(current) {
                if (rig.left || rig.knife) && library.upgraded.contains(&rig.weapon) {
                    unconnected = Some(
                        "Native knife/alternate-hand model submission is not connected".to_owned(),
                    );
                }
                break;
            }
            if let Ok(context) = contexts.get(current) {
                if library.upgraded.contains(&context.0) {
                    unconnected = Some("Native world-weapon descriptor groups and mapped camera distance are not connected".to_owned());
                }
                break;
            }
            ancestor = parents.get(current).ok().map(ChildOf::parent);
        }
        let result: Result<Option<(Value, Option<Value>)>, String> = (|| {
            if let Some(reason) = unconnected {
                return Err(reason);
            }
            let (queue, index) = if let Some(submission) = submission {
                let model = model
                    .as_deref()
                    .ok_or("Submitted native model identity is missing")?;
                let indices: Vec<_> = submission
                    .models
                    .iter()
                    .enumerate()
                    .filter_map(|(index, name)| (name == model).then_some(index))
                    .collect();
                if indices.len() != 1 {
                    return Err(format!(
                        "Submitted native model {model} does not have one recovered DObj index"
                    ));
                }
                let index = indices[0] as u32;
                let mask = u32::from((1_u32.wrapping_shl(index)) as u16);
                let groups: HashSet<_> = submission
                    .descriptors
                    .iter()
                    .filter(|descriptor| descriptor.mask & mask != 0)
                    .map(|descriptor| descriptor.group)
                    .collect();
                if groups.len() > 1 {
                    return Err("Native camo descriptor groups overlap this model".into());
                }
                let queue = if let Some(group) = groups.into_iter().next() {
                    camo::queue_descriptors(
                        &submission.descriptors,
                        group,
                        Some(&library.native.knife_camo),
                        &library.native.definitions,
                        &library.native.options,
                        submission.selection.options,
                        0.,
                        &library.native.materials,
                    )?
                } else {
                    Vec::new()
                };
                (queue, index)
            } else {
                (Vec::new(), 0)
            };
            let inputs = library.native.bound_inputs(&queue, original, index)?;
            if !library.native.needs_binding(original, &inputs)? {
                return Ok(None);
            }
            if applied.is_some_and(|applied| applied.inputs == inputs) {
                return Ok(Some((inputs, None)));
            }
            let surface = library.native.prepared(&inputs)?;
            crate::rendering::validate_surface(surface)?;
            Ok(Some((inputs.clone(), Some(surface.clone()))))
        })();
        match result {
            Ok(None) => {
                if let Some(applied) = applied {
                    restore(&mut commands.entity(entity), applied);
                }
            }
            Ok(Some((_, None))) => (),
            Ok(Some((inputs, Some(surface)))) => {
                let source_material = applied.map_or(&handle.0, |applied| &applied.material);
                let Some(original_material) = materials.get(source_material).cloned() else {
                    continue;
                };
                let source_extras =
                    applied.map_or(material_extras, |applied| applied.extras.as_ref());
                let mut value = source_extras
                    .and_then(|extra| serde_json::from_str::<Value>(&extra.value).ok())
                    .unwrap_or_else(|| serde_json::json!({}));
                let identity = surface["identity"].as_str().unwrap().to_owned();
                value["surface"] = surface;
                let applied = Applied {
                    inputs,
                    identity: identity.clone(),
                    name: original.into(),
                    material: source_material.clone(),
                    extras: source_extras.cloned(),
                };
                let mut target = commands.entity(entity);
                crate::rendering::opaque::reset_binding(&mut target);
                target.insert((
                    MeshMaterial3d(materials.add(original_material)),
                    GltfMaterialName(identity),
                    GltfMaterialExtras {
                        value: value.to_string(),
                    },
                    applied,
                ));
            }
            Err(error) => {
                failure.0 = format!("Native camo surface {original}: {error}");
                next.set(GameState::LoadError);
                return;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::asset::AssetApp;
    use serde_json::json;
    use std::collections::BTreeMap;

    fn library() -> Library {
        let a = json!({"name":"a","techniques":[{"index":4,"passes":[{"arguments":[{"type":5,"value":16777397_u32}]}]}],
            "textures":[{"semantic":2,"image":"color_a"},{"semantic":5,"image":"normal_a"},{"semantic":8,"image":"spec_a"}]});
        let b = json!({"name":"b","techniques":[],
            "textures":[{"semantic":2,"image":"color_b"},{"semantic":13,"image":"normal_b"},{"semantic":14,"image":"spec_b"}]});
        let mut sets = vec![Vec::new(); 13];
        sets[12].push(camo::CamoRecord {
            flags: 7,
            pairs: vec![(Some("a".into()), Some("b".into()))],
            constants: [2., 3., 0.5, 0., 0., 0., 0., 0.],
        });
        let camo = camo::CamoDefinition {
            images: [None, None],
            patterns: Vec::new(),
            material_sets: sets,
        };
        let mut options = vec![None; 128];
        options[45] = Some(camo::CamoOption {
            selection: 13,
            material: true,
        });
        let mut native = camo::Library {
            version: 3,
            definitions: BTreeMap::from([("paint".into(), camo.clone()), ("knife".into(), camo)]),
            options,
            weapons: BTreeMap::new(),
            attachments: BTreeMap::new(),
            knife_camo: "knife".into(),
            optic_switch: 1.,
            materials: BTreeMap::from([("a".into(), a), ("b".into(), b)]),
            variants: Vec::new(),
        };
        let surface = |identity| json!({"version":3,"identity":identity,"channels":[],"standard":{},"presentation":{"alpha":"opaque","cull":"back"}});
        let baseline = native.bound_inputs(&[], "a", 1).unwrap();
        let queue = camo::queue_descriptors(
            &[camo::CamoDescriptor {
                group: 0,
                camo: Some("paint".into()),
                mask: 2,
            }],
            0,
            Some("knife"),
            &native.definitions,
            &native.options,
            45,
            0.,
            &native.materials,
        )
        .unwrap();
        let selected = native.bound_inputs(&queue, "a", 1).unwrap();
        assert_eq!(selected["code_constants"]["185"], json!([2., 3., 0.5, 10.]));
        native.variants = vec![
            camo::PreparedVariant {
                inputs: baseline,
                surface: Some(surface("state.base")),
                pending: None,
            },
            camo::PreparedVariant {
                inputs: selected,
                surface: Some(surface("state.paint")),
                pending: None,
            },
        ];
        native.validate().unwrap();
        Library {
            native,
            upgraded: HashSet::new(),
        }
    }

    fn submission(options: u32) -> Submission {
        Submission {
            selection: Selection {
                weapon: WeaponId("weapon.controlled".into()),
                options,
                packed: 0,
                mode: 0,
                main_index: 0,
                initial_model: "cached".into(),
            },
            models: vec!["cached".into(), "model".into()],
            descriptors: vec![camo::CamoDescriptor {
                group: 0,
                camo: Some("paint".into()),
                mask: 2,
            }],
        }
    }

    #[test]
    fn native_camo_rebinds_per_object_restores_defaults_and_resets_the_session() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, bevy::asset::AssetPlugin::default()))
            .init_asset::<Image>()
            .init_asset::<StandardMaterial>()
            .init_resource::<LoadFailure>()
            .insert_resource(NextState::<GameState>::Unchanged)
            .insert_resource(library())
            .add_systems(Update, apply);
        let original = app
            .world_mut()
            .resource_mut::<Assets<StandardMaterial>>()
            .add(StandardMaterial::default());
        let mut objects = Vec::new();
        for options in [45, 0] {
            let root = app
                .world_mut()
                .spawn((
                    submission(options),
                    GltfExtras {
                        value: json!({"source_model":"model"}).to_string(),
                    },
                ))
                .id();
            let entity = app
                .world_mut()
                .spawn((
                    ChildOf(root),
                    MeshMaterial3d(original.clone()),
                    GltfMaterialName("a".into()),
                    GltfMaterialExtras {
                        value: json!({"source":"original"}).to_string(),
                    },
                ))
                .id();
            objects.push((root, entity));
        }
        app.update();
        assert!(app.world().resource::<LoadFailure>().0.is_empty());
        assert_eq!(
            app.world().get::<GltfMaterialName>(objects[0].1).unwrap().0,
            "state.paint"
        );
        assert_eq!(
            app.world().get::<GltfMaterialName>(objects[1].1).unwrap().0,
            "state.base"
        );
        let handle_a = app
            .world()
            .get::<MeshMaterial3d<StandardMaterial>>(objects[0].1)
            .unwrap()
            .0
            .clone();
        let handle_b = app
            .world()
            .get::<MeshMaterial3d<StandardMaterial>>(objects[1].1)
            .unwrap()
            .0
            .clone();
        assert_ne!(handle_a, handle_b);
        assert_ne!(handle_a, original);
        assert_ne!(handle_b, original);
        app.world_mut()
            .entity_mut(objects[0].0)
            .insert(submission(0));
        app.update();
        assert_eq!(
            app.world().get::<GltfMaterialName>(objects[0].1).unwrap().0,
            "state.base"
        );
        assert!(
            app.world().get::<Applied>(objects[0].1).unwrap().inputs["code_constants"]["185"]
                .is_null()
        );
        assert_eq!(
            app.world().get::<Applied>(objects[0].1).unwrap().inputs["code_constants"]["181"],
            json!([0.,0.,0.,0.])
        );
        app.add_systems(Update, reset.before(apply));
        app.update();
        assert!(!app.world().contains_resource::<Library>());
        for (root, entity) in objects {
            assert!(app.world().get::<Submission>(root).is_none());
            assert!(app.world().get::<Applied>(entity).is_none());
            assert_eq!(
                app.world()
                    .get::<MeshMaterial3d<StandardMaterial>>(entity)
                    .unwrap()
                    .0,
                original
            );
            assert_eq!(app.world().get::<GltfMaterialName>(entity).unwrap().0, "a");
            assert_eq!(
                app.world().get::<GltfMaterialExtras>(entity).unwrap().value,
                json!({"source":"original"}).to_string()
            );
        }
    }

    #[test]
    fn missing_camo_variants_preserve_the_specific_source_gap() {
        let mut library = library();
        let inputs = library.native.variants[1].inputs.clone();
        library.native.variants[1].surface = None;
        library.native.variants[1].pending =
            Some("Native camo parallax operation is not converted".into());
        library.native.validate().unwrap();
        assert_eq!(
            library.native.prepared(&inputs).unwrap_err(),
            "Native camo parallax operation is not converted"
        );
    }
}

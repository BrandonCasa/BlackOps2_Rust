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
use std::collections::{BTreeMap, HashSet};

#[derive(Component)]
pub(crate) struct WeaponMaterialContext(pub WeaponId);

#[derive(Component)]
struct Applied {
    inputs: Value,
    identity: String,
    name: String,
    material: Handle<StandardMaterial>,
    extras: Option<GltfMaterialExtras>,
    binding_extras: String,
}

#[derive(Resource)]
struct Library {
    native: camo::Library,
    upgraded: HashSet<WeaponId>,
}

#[derive(Clone, PartialEq)]
struct Selection {
    owner: Entity,
    left: bool,
    weapon: WeaponId,
    options: u32,
    packed: u32,
    mode: u8,
    main_index: Option<usize>,
    main_model: String,
    initial_model: String,
}

#[derive(Component)]
struct Submission {
    selection: Selection,
    models: Vec<String>,
    descriptors: Vec<camo::CamoDescriptor>,
    queues: BTreeMap<u32, Vec<camo::CamoOverride>>,
}

impl Submission {
    fn new(
        selection: Selection,
        plan: camo::ViewmodelMaterials,
        native: &camo::Library,
    ) -> Result<Self, String> {
        let mut queues = BTreeMap::new();
        for group in plan.descriptors.iter().map(|descriptor| descriptor.group) {
            if queues.contains_key(&group) {
                continue;
            }
            queues.insert(
                group,
                camo::queue_descriptors(
                    &plan.descriptors,
                    group,
                    Some(&native.knife_camo),
                    &native.definitions,
                    &native.options,
                    selection.options,
                    0.,
                    &native.materials,
                )?,
            );
        }
        Ok(Self {
            selection,
            models: plan.models,
            descriptors: plan.descriptors,
            queues,
        })
    }
}

fn equivalent_viewmodel_plans(
    indices: &[usize],
    mut build: impl FnMut(usize) -> Result<camo::ViewmodelMaterials, String>,
) -> Result<camo::ViewmodelMaterials, String> {
    let (&first, remaining) = indices
        .split_first()
        .ok_or("Held native viewmodel has no matching source slot")?;
    let plan = build(first)?;
    for &index in remaining {
        if build(index)? != plan {
            return Err("Held native viewmodel source slots have different material plans".into());
        }
    }
    Ok(plan)
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
        if rig.arms || rig.knife {
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
                Some(index)
            } else if indices.len() == 1 {
                Some(indices[0])
            } else {
                None
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
                owner: rig.owner,
                left: rig.left,
                weapon: rig.weapon.clone(),
                options,
                packed,
                mode: u8::from(inventory.ads >= library.native.optic_switch),
                main_index,
                main_model: model.to_owned(),
                initial_model,
            };
            if weapon.definition44 == 3 {
                return Err(
                    "Native alternate packed-weapon role is not connected to the active owner"
                        .into(),
                );
            }
            let secondary = library.native.secondary_weapon(weapon, packed)?;
            if rig.dual && secondary.is_none() {
                return Err(
                    "Presented dual weapon has no recovered native secondary selection".into(),
                );
            }
            if let Some(secondary) = secondary {
                let left = rigs
                    .iter()
                    .find(|(_, other, _)| {
                        other.owner == rig.owner
                            && other.weapon == rig.weapon
                            && other.left
                            && !other.arms
                            && !other.knife
                    })
                    .ok_or("Native secondary viewmodel owner is missing")?
                    .1;
                let path = server
                    .get_path(left.source.id())
                    .ok_or("Native secondary viewmodel asset path is missing")?;
                let model = art
                    .native_model(&path.to_string())
                    .ok_or("Native secondary viewmodel identity is missing")?;
                if secondary.main_models.first().and_then(Option::as_deref) != Some(model) {
                    return Err(
                        "Presented secondary viewmodel disagrees with the authored native selector"
                            .into(),
                    );
                }
            }
            if current.is_some_and(|current| current.selection == selection) {
                return Ok(None);
            }
            let plan = equivalent_viewmodel_plans(
                if main_index.is_some() {
                    main_index.as_slice()
                } else {
                    &indices
                },
                |index| {
                    camo::viewmodel_materials(
                        weapon,
                        None,
                        &library.native.attachments,
                        &selection.initial_model,
                        packed,
                        0,
                        index,
                        selection.mode,
                        secondary.map(|secondary| (secondary, packed)),
                        [None; 2],
                    )
                },
            )?;
            Ok(Some(Submission::new(selection, plan, &library.native)?))
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
    inventories: Query<&crate::weapons::WeaponInventory>,
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
        let applied = applied.filter(|applied| {
            applied.identity == name.0
                && material_extras.is_some_and(|extras| extras.value == applied.binding_extras)
        });
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
                if (rig.left || rig.knife)
                    && library.upgraded.contains(&rig.weapon)
                    && inventories.get(rig.owner).is_ok_and(|inventory| {
                        inventory
                            .active()
                            .is_some_and(|weapon| weapon.definition == rig.weapon)
                    })
                {
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
                let queue = groups
                    .into_iter()
                    .next()
                    .and_then(|group| submission.queues.get(&group))
                    .map_or(&[][..], Vec::as_slice);
                (queue, index)
            } else {
                (&[][..], 0)
            };
            let inputs = library.native.bound_inputs(queue, original, index)?;
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
                let binding = materials.add(original_material);
                let binding_extras = value.to_string();
                let applied = Applied {
                    inputs,
                    identity: identity.clone(),
                    name: original.into(),
                    material: source_material.clone(),
                    extras: source_extras.cloned(),
                    binding_extras: binding_extras.clone(),
                };
                let mut target = commands.entity(entity);
                crate::rendering::opaque::reset_binding(&mut target);
                target.insert((
                    MeshMaterial3d(binding),
                    GltfMaterialName(identity),
                    GltfMaterialExtras {
                        value: binding_extras,
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
        Submission::new(
            Selection {
                owner: Entity::PLACEHOLDER,
                left: false,
                weapon: WeaponId("weapon.controlled".into()),
                options,
                packed: 0,
                mode: 0,
                main_index: Some(0),
                main_model: "model".into(),
                initial_model: "cached".into(),
            },
            camo::ViewmodelMaterials {
                models: vec!["cached".into(), "model".into()],
                descriptors: vec![camo::CamoDescriptor {
                    group: 0,
                    camo: Some("paint".into()),
                    mask: 2,
                }],
            },
            &library().native,
        )
        .unwrap()
    }

    fn native_weapon(model: &str) -> camo::WeaponMaterials {
        let mut weapon = camo::WeaponMaterials {
            camo: Some("paint".into()),
            main_models: vec![None; 16],
            variant_models: std::array::from_fn(|_| None),
            attachments: vec![None; 95],
            definition28: 0,
            definition44: 0,
            shared_ammo_related: 0,
            alternate_weapon_index: 0,
            alternate_weapon_name: None,
            base_attachment_flags: vec![None; 63],
        };
        weapon.main_models[0] = Some(model.into());
        weapon
    }

    #[test]
    fn equivalent_main_slots_preserve_complete_material_plans_without_an_index() {
        let mut primary = native_weapon("right");
        primary.main_models[7] = Some("right".into());
        primary.variant_models[0] = Some("right_detail".into());
        let secondary = native_weapon("left");
        let build = |index| {
            camo::viewmodel_materials(
                &primary,
                None,
                &BTreeMap::new(),
                "arms",
                0,
                0,
                index,
                0,
                Some((&secondary, 0)),
                [Some("mask"), Some("clip")],
            )
        };
        let plan = equivalent_viewmodel_plans(&[0, 7], build).unwrap();
        assert_eq!(
            plan.models,
            ["arms", "right", "right_detail", "left", "mask", "clip"]
        );
        assert_eq!(
            plan.descriptors
                .iter()
                .map(|entry| entry.mask)
                .collect::<Vec<_>>(),
            [6, 8]
        );
        assert!(
            equivalent_viewmodel_plans(&[], build)
                .unwrap_err()
                .contains("no matching")
        );
        let mut different = plan.clone();
        different.descriptors[1].group = 2;
        assert!(
            equivalent_viewmodel_plans(&[0, 7], |index| Ok(if index == 0 {
                plan.clone()
            } else {
                different.clone()
            }))
            .unwrap_err()
            .contains("different material plans")
        );
        let mut selected = submission(45).selection;
        selected.main_index = None;
        let mut changed = selected.clone();
        changed.main_model = "another_model".into();
        assert!(selected != changed);
        assert_eq!(selected.main_index, None);
    }

    #[test]
    fn authored_dual_selection_preserves_descriptor_groups_model_indices_and_reset() {
        let mut library = library();
        let mut primary = native_weapon("right");
        primary.shared_ammo_related = 1;
        primary.alternate_weapon_name = Some("weapon.left".into());
        primary.variant_models[0] = Some("right_detail".into());
        let mut left = native_weapon("left");
        left.variant_models[0] = Some("left_detail".into());
        left.camo = None;
        library
            .native
            .weapons
            .insert("weapon.right".into(), primary.clone());
        library.native.weapons.insert("weapon.left".into(), left);
        library.native.validate().unwrap();
        let secondary = library
            .native
            .secondary_weapon(&primary, 0)
            .unwrap()
            .unwrap();
        assert_eq!(secondary.main_models[0].as_deref(), Some("left"));
        let mut attachment_enabled = primary.clone();
        attachment_enabled.shared_ammo_related = 0;
        attachment_enabled.base_attachment_flags[1] = Some(0);
        attachment_enabled.base_attachment_flags[2] = Some(1);
        assert!(
            library
                .native
                .secondary_weapon(&attachment_enabled, (1 << 8) | (2 << 14))
                .unwrap()
                .is_some()
        );
        assert!(
            library
                .native
                .secondary_weapon(&attachment_enabled, 2 << 14)
                .unwrap()
                .is_none()
        );
        assert!(
            library
                .native
                .secondary_weapon(&attachment_enabled, 63 << 8)
                .unwrap_err()
                .contains("selector 63")
        );
        let plan = camo::viewmodel_materials(
            &primary,
            None,
            &library.native.attachments,
            "arms",
            0,
            0,
            0,
            0,
            Some((secondary, 0)),
            [None; 2],
        )
        .unwrap();
        assert_eq!(
            plan.models,
            ["arms", "right", "right_detail", "left", "left_detail"]
        );
        assert_eq!(
            plan.descriptors
                .iter()
                .map(|descriptor| descriptor.mask)
                .collect::<Vec<_>>(),
            [6, 24]
        );
        let selection = Selection {
            owner: Entity::PLACEHOLDER,
            left: false,
            weapon: WeaponId("weapon.right".into()),
            options: 45,
            packed: 0,
            mode: 0,
            main_index: Some(0),
            main_model: "right".into(),
            initial_model: "arms".into(),
        };
        let selected = Submission::new(selection.clone(), plan.clone(), &library.native).unwrap();
        assert_eq!(selected.queues[&0].len(), 5);
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, bevy::asset::AssetPlugin::default()))
            .init_asset::<Image>()
            .init_asset::<StandardMaterial>()
            .init_resource::<LoadFailure>()
            .insert_resource(NextState::<GameState>::Unchanged)
            .insert_resource(library)
            .add_systems(Update, apply);
        let original = app
            .world_mut()
            .resource_mut::<Assets<StandardMaterial>>()
            .add(StandardMaterial::default());
        let root = app.world_mut().spawn(selected).id();
        let mut objects = Vec::new();
        for model in ["right", "right_detail", "left", "left_detail"] {
            let entity = app
                .world_mut()
                .spawn((
                    ChildOf(root),
                    GltfExtras {
                        value: json!({"source_model":model}).to_string(),
                    },
                    MeshMaterial3d(original.clone()),
                    GltfMaterialName("a".into()),
                    GltfMaterialExtras {
                        value: json!({"source":"original"}).to_string(),
                    },
                ))
                .id();
            objects.push(entity);
        }
        app.update();
        assert!(app.world().resource::<LoadFailure>().0.is_empty());
        for (index, entity) in objects.iter().enumerate() {
            assert_eq!(
                app.world().get::<GltfMaterialName>(*entity).unwrap().0,
                if index < 2 {
                    "state.paint"
                } else {
                    "state.base"
                }
            );
        }
        let selected_handle = app
            .world()
            .get::<MeshMaterial3d<StandardMaterial>>(objects[0])
            .unwrap()
            .0
            .clone();
        let cloned = app
            .world()
            .resource::<Assets<StandardMaterial>>()
            .get(&selected_handle)
            .unwrap()
            .clone();
        let cloned_handle = app
            .world_mut()
            .resource_mut::<Assets<StandardMaterial>>()
            .add(cloned);
        app.world_mut()
            .entity_mut(objects[0])
            .insert(MeshMaterial3d(cloned_handle));
        app.update();
        assert!(app.world().get::<Applied>(objects[0]).is_some());
        let mut selection = selection;
        selection.options = 0;
        let cleared =
            Submission::new(selection, plan, &app.world().resource::<Library>().native).unwrap();
        app.world_mut().entity_mut(root).insert(cleared);
        app.update();
        for entity in &objects {
            assert_eq!(
                app.world().get::<GltfMaterialName>(*entity).unwrap().0,
                "state.base"
            );
            assert!(
                app.world().get::<Applied>(*entity).unwrap().inputs["code_constants"]["185"]
                    .is_null()
            );
        }
        app.add_systems(Update, reset.before(apply));
        app.update();
        for entity in objects {
            assert!(app.world().get::<Applied>(entity).is_none());
            assert_eq!(
                app.world()
                    .get::<MeshMaterial3d<StandardMaterial>>(entity)
                    .unwrap()
                    .0,
                original
            );
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
            json!([0., 0., 0., 0.])
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

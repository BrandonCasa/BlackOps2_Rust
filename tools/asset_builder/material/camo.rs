use super::*;

use crate::camo::{CamoDefinition, CamoOption, CamoPattern, CamoRecord};

fn camo_asset(
    catalog: &Catalog<'_>,
    zone: usize,
    field: Address,
    kind: u32,
) -> Result<Option<String>> {
    if catalog.zones[zone].1.streams.u32(field)? == 0 {
        return Ok(None);
    }
    let (_, owner, root) = catalog.pointer(zone, field, kind)?;
    let streams = &catalog.zones[owner].1.streams;
    let name = streams.text(Address::decode(
        streams.u32(root.add(if kind == 8 { 72 } else { 0 })?)?,
    )?)?;
    Ok(Some(name.trim_start_matches(',').to_ascii_lowercase()))
}

pub fn read_camos(catalog: &Catalog<'_>) -> Result<BTreeMap<String, CamoDefinition>> {
    let mut result = BTreeMap::new();
    for (name, zone, root) in catalog.definitions(31) {
        let streams = &catalog.zones[zone].1.streams;
        let array = |field: Address, count: usize, stride: usize| -> Result<Option<Address>> {
            if count == 0 {
                return Ok(None);
            }
            let pointer = streams.u32(field)?;
            if pointer == 0 {
                return Err(format!("Camo {name} has a null positive-count array").into());
            }
            let address = Address::decode(pointer)?;
            streams.read(
                address,
                count
                    .checked_mul(stride)
                    .ok_or("Camo array size overflow")?,
            )?;
            Ok(Some(address))
        };
        let scalar = |field: Address| -> Result<f32> {
            let value = f32::from_bits(streams.u32(field)?);
            if !value.is_finite() {
                return Err(format!("Camo {name} has a non-finite authored constant").into());
            }
            Ok(value)
        };
        let images = [
            camo_asset(catalog, zone, root.add(4)?, 8)?,
            camo_asset(catalog, zone, root.add(8)?, 8)?,
        ];
        let count = streams.u32(root.add(16)?)? as usize;
        let mut patterns = Vec::new();
        if let Some(table) = array(root.add(12)?, count, 20)? {
            for index in 0..count {
                let record = table.add(index * 20)?;
                patterns.push(CamoPattern {
                    images: [
                        camo_asset(catalog, zone, record, 8)?,
                        camo_asset(catalog, zone, record.add(4)?, 8)?,
                    ],
                    offset: [scalar(record.add(8)?)?, scalar(record.add(12)?)?],
                    scale: scalar(record.add(16)?)?,
                });
            }
        }
        let count = streams.u32(root.add(24)?)? as usize;
        let mut material_sets = Vec::new();
        if let Some(table) = array(root.add(20)?, count, 8)? {
            for index in 0..count {
                let set = table.add(index * 8)?;
                let count = streams.u32(set)? as usize;
                let mut records = Vec::new();
                if let Some(table) = array(set.add(4)?, count, 44)? {
                    for index in 0..count {
                        let record = table.add(index * 44)?;
                        let count = streams.u16(record.add(2)?)? as usize;
                        let from = array(record.add(4)?, count, 4)?;
                        let to = array(record.add(8)?, count, 4)?;
                        let mut pairs = Vec::new();
                        for index in 0..count {
                            pairs.push((
                                camo_asset(
                                    catalog,
                                    zone,
                                    from.ok_or("Missing source camo pair array")?
                                        .add(index * 4)?,
                                    6,
                                )?,
                                camo_asset(
                                    catalog,
                                    zone,
                                    to.ok_or("Missing destination camo pair array")?
                                        .add(index * 4)?,
                                    6,
                                )?,
                            ));
                        }
                        let constants = (0..8)
                            .map(|index| scalar(record.add(12 + index * 4)?))
                            .collect::<Result<Vec<_>>>()?
                            .try_into()
                            .map_err(|_| "Camo record does not contain eight constants")?;
                        records.push(CamoRecord {
                            flags: streams.u16(record)?,
                            pairs,
                            constants,
                        });
                    }
                }
                material_sets.push(records);
            }
        }
        result.insert(
            name.to_ascii_lowercase(),
            CamoDefinition {
                images,
                patterns,
                material_sets,
            },
        );
    }
    if result.is_empty() {
        return Err("Required native camo definitions are missing".into());
    }
    Ok(result)
}

pub fn read_camo_options(catalog: &Catalog<'_>) -> Result<Vec<Option<CamoOption>>> {
    let (zone, root) = catalog.resolve(42, "mp/weaponoptions_zm.csv")?;
    let streams = &catalog.zones[zone].1.streams;
    let columns = streams.u32(root.add(4)?)? as usize;
    let rows = streams.u32(root.add(8)?)? as usize;
    if columns < 5 {
        return Err("Native Zombies weapon-options table has fewer than five columns".into());
    }
    let cells = Address::decode(streams.u32(root.add(12)?)?)?;
    let cell = |row: usize, column: usize| -> Result<String> {
        let pointer = streams.u32(cells.add((row * columns + column) * 8)?)?;
        if pointer == 0 {
            Ok(String::new())
        } else {
            Ok(streams.text(Address::decode(pointer)?)?)
        }
    };
    let mut options = vec![None; 128];
    for row in 0..rows {
        if cell(row, 1)? != "camo" {
            continue;
        }
        let id: usize = cell(row, 0)?.parse()?;
        let selection = cell(row, 4)?;
        if selection.is_empty() {
            continue;
        }
        let mode = cell(row, 3)?;
        *options
            .get_mut(id)
            .ok_or("Native camo option ID exceeds 127")? = Some(CamoOption {
            selection: selection.parse()?,
            material: !mode.is_empty() && mode.parse::<i32>()? != 0,
        });
    }
    Ok(options)
}

pub fn read_material_selection(
    catalog: &Catalog<'_>,
) -> Result<(
    BTreeMap<String, crate::camo::WeaponMaterials>,
    BTreeMap<String, crate::camo::AttachmentMaterials>,
)> {
    let weapon_name = |zone: usize, field: Address| -> Result<Option<String>> {
        let streams = &catalog.zones[zone].1.streams;
        let pointer = streams.u32(field)?;
        if pointer == 0 {
            return Ok(None);
        }
        let name = streams.text(Address::decode(pointer)?)?;
        let name = name.trim_start_matches(',');
        Ok((!name.is_empty()).then(|| format!("weapon.{}", name.to_ascii_lowercase())))
    };
    let mut attachments = BTreeMap::new();
    for (name, zone, root) in catalog.definitions(30) {
        let streams = &catalog.zones[zone].1.streams;
        attachments.insert(
            name.to_ascii_lowercase(),
            crate::camo::AttachmentMaterials {
                camo: camo_asset(catalog, zone, root.add(164)?, 31)?,
                attachment_type: streams.u32(root.add(4)?)?,
                chain_next: streams.u32(root.add(8)?)?,
                chain_head: streams.u32(root.add(12)?)?,
                combination_mask: streams.u32(root.add(16)?)?,
                alternate_weapon_index: streams.u32(root.add(0x20)?)?,
                alternate_weapon_name: weapon_name(zone, root.add(0x1c)?)?,
                suppress168: streams.u8(root.add(168)?)?,
                suppress169: streams.u8(root.add(169)?)?,
                models: [
                    camo_asset(catalog, zone, root.add(40)?, 5)?,
                    camo_asset(catalog, zone, root.add(44)?, 5)?,
                    camo_asset(catalog, zone, root.add(48)?, 5)?,
                ],
            },
        );
    }
    let mut weapons = BTreeMap::new();
    for (name, zone, root) in catalog.definitions(25) {
        let streams = &catalog.zones[zone].1.streams;
        let definition = Address::decode(streams.u32(root.add(8)?)?)?;
        let array = |field: Address, count: usize, kind: u32| -> Result<Vec<Option<String>>> {
            let pointer = streams.u32(field)?;
            if pointer == 0 {
                return Ok(vec![None; count]);
            }
            let table = Address::decode(pointer)?;
            (0..count)
                .map(|index| camo_asset(catalog, zone, table.add(index * 4)?, kind))
                .collect()
        };
        let base_attachment_flags = array(root.add(0x18)?, 63, 29)?
            .into_iter()
            .map(|name| -> Result<Option<u8>> {
                let Some(name) = name else {
                    return Ok(None);
                };
                let (owner, root) = catalog.resolve(29, &name)?;
                Ok(Some(catalog.zones[owner].1.streams.u8(root.add(0x31)?)?))
            })
            .collect::<Result<Vec<_>>>()?;
        weapons.insert(
            format!("weapon.{}", name.to_ascii_lowercase()),
            crate::camo::WeaponMaterials {
                camo: camo_asset(catalog, zone, definition.add(2420)?, 31)?,
                main_models: array(definition.add(4)?, 16, 5)?,
                variant_models: array(root.add(40)?, 8, 5)?
                    .try_into()
                    .map_err(|_| "Native viewmodel variant count is not eight")?,
                attachments: array(root.add(28)?, 95, 30)?,
                definition28: streams.u32(definition.add(28)?)?,
                definition44: streams.u32(definition.add(44)?)?,
                shared_ammo_related: streams.u8(definition.add(0x64f)?)?,
                alternate_weapon_index: streams.u32(definition.add(0x67c)?)?,
                alternate_weapon_name: weapon_name(zone, definition.add(0x678)?)?,
                base_attachment_flags,
            },
        );
    }
    Ok((weapons, attachments))
}

pub fn camo_variants(library: &mut crate::camo::Library) -> Result<BTreeMap<String, Value>> {
    let mut inputs = BTreeMap::new();
    let crate::camo::Library {
        variants,
        materials,
        definitions,
        options,
        ..
    } = library;
    let mut add = |bound: Value| {
        if variants.iter().any(|variant| variant.inputs == bound) {
            return;
        }
        let key = format!("camo.{}", variants.len());
        variants.push(crate::camo::PreparedVariant {
            inputs: bound.clone(),
            surface: None,
            pending: Some("Native camo state has not been converted".into()),
        });
        inputs.insert(key, bound);
    };
    let initial = BTreeMap::from([(181, [0.; 4])]);
    for material in materials.values() {
        let active = material["techniques"]
            .as_array()
            .ok_or("Missing camo material techniques")?
            .iter()
            .filter(|technique| technique["index"] == 4 || technique["index"] == 6)
            .flat_map(|technique| technique["passes"].as_array().into_iter().flatten())
            .flat_map(|pass| pass["arguments"].as_array().into_iter().flatten())
            .any(|argument| {
                argument["type"] == 5
                    && matches!(argument["value"].as_u64(), Some(16777397 | 16777401))
            });
        if active {
            add(crate::camo::camo_inputs(&[], material, 1, &initial)?.0);
        }
    }
    for camo in definitions.values() {
        let mut queue = Vec::new();
        crate::camo::queue_camo(camo, options, 45, 0., 2, materials, &mut queue)?;
        let originals: std::collections::BTreeSet<_> = queue
            .iter()
            .filter_map(|command| {
                if let crate::camo::CamoOverride::Material { from, .. } = command {
                    Some(from.as_str())
                } else {
                    None
                }
            })
            .collect();
        for original in originals {
            let selected = crate::camo::camo_material(&queue, original, 1);
            let material = materials
                .get(selected)
                .ok_or_else(|| format!("Missing selected camo material {selected}"))?;
            add(crate::camo::camo_inputs(&queue, material, 1, &initial)?.0);
        }
    }
    Ok(inputs)
}

pub fn finish_camos(
    library: &mut crate::camo::Library,
    converted: &BTreeMap<String, Value>,
    pack: &mut Pack,
) -> Result<Vec<String>> {
    let mut pending = Vec::new();
    for (index, variant) in library.variants.iter_mut().enumerate() {
        let key = format!("camo.{index}");
        let surface = converted
            .get(&key)
            .map(|material| &material["extras"]["surface"]);
        let reason = surface.and_then(|surface| surface["pending_runtime"].as_str()).map(str::to_owned)
            .or_else(|| surface.is_none().then(|| format!("Native camo material {} needs conversion for technique {} with its scoped source inputs",
                variant.inputs["name"],variant.inputs["technique_set"])));
        variant.pending = reason.clone();
        variant.surface = reason.is_none().then(|| surface.unwrap().clone());
        if let Some(reason) = reason {
            pending.push(format!("{key}: {reason}"));
        }
    }
    library.validate()?;
    if pack
        .entries
        .insert(
            "prepared/camo.json".into(),
            Arc::new(serde_json::to_vec(library)?),
        )
        .is_some()
    {
        return Err("Duplicate native camo library".into());
    }
    Ok(pending)
}

pub fn prepare_library(
    catalog: &Catalog<'_>,
    materials: &mut BTreeMap<String, Value>,
) -> Result<(crate::camo::Library, BTreeMap<String, Value>)> {
    let camos = read_camos(catalog)?;
    let camo_options = read_camo_options(catalog)?;
    let (weapon_materials, attachment_materials) = read_material_selection(catalog)?;
    for name in weapon_materials
        .values()
        .filter_map(|weapon| weapon.camo.as_ref())
        .chain(
            attachment_materials
                .values()
                .filter_map(|attachment| attachment.camo.as_ref()),
        )
    {
        if !camos.contains_key(name) {
            return Err(
                format!("Native weapon/attachment camo definition {name} is missing").into(),
            );
        }
    }
    println!(
        "Recovered material selectors for {} native weapons and {} attachment definitions; packed selector delivery and live owner submissions remain required",
        weapon_materials.len(),
        attachment_materials.len()
    );
    if camo_options[45].is_none() {
        return Err(
            "Required Origins upgraded camo option is missing from the installed source table"
                .into(),
        );
    }
    let camo_dependencies = crate::camo::camo_dependencies(&camos, &camo_options, 45);
    let extra: std::collections::BTreeSet<_> = camo_dependencies
        .into_iter()
        .filter(|name| !materials.contains_key(name))
        .collect();
    materials.extend(read_used(catalog, &extra)?);
    println!(
        "Resolved {} native camo definitions and {} additional source/replacement material dependencies for Origins upgraded camo",
        camos.len(),
        extra.len()
    );
    let mut camo_library = crate::camo::Library {
        version: 3,
        definitions: camos,
        options: camo_options,
        weapons: weapon_materials,
        attachments: attachment_materials,
        knife_camo: "camo_knife".into(),
        optic_switch: 1.,
        materials: materials.clone(),
        variants: Vec::new(),
    };
    camo_library.resolve_knife()?;
    camo_library.validate()?;
    let camo_variants = camo_variants(&mut camo_library)?;
    Ok((camo_library, camo_variants))
}

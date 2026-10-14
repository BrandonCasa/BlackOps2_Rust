use serde_json::{Value, json};
use std::collections::BTreeMap;
type Result<T> = std::result::Result<T, String>;

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct CamoPattern {
    pub images: [Option<String>; 2],
    pub offset: [f32; 2],
    pub scale: f32,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct CamoRecord {
    pub flags: u16,
    pub pairs: Vec<(Option<String>, Option<String>)>,
    pub constants: [f32; 8],
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct CamoDefinition {
    pub images: [Option<String>; 2],
    pub patterns: Vec<CamoPattern>,
    pub material_sets: Vec<Vec<CamoRecord>>,
}

#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
pub struct CamoOption {
    pub selection: i32,
    pub material: bool,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct WeaponMaterials {
    pub camo: Option<String>,
    pub main_models: Vec<Option<String>>,
    pub variant_models: [Option<String>; 8],
    pub attachments: Vec<Option<String>>,
    pub definition28: u32,
    pub definition44: u32,
    pub shared_ammo_related: u8,
    pub alternate_weapon_index: u32,
    pub alternate_weapon_name: Option<String>,
    pub base_attachment_flags: Vec<Option<u8>>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct AttachmentMaterials {
    pub camo: Option<String>,
    pub attachment_type: u32,
    pub chain_next: u32,
    pub chain_head: u32,
    pub combination_mask: u32,
    pub suppress168: u8,
    pub suppress169: u8,
    pub alternate_weapon_index: u32,
    pub alternate_weapon_name: Option<String>,
    pub models: [Option<String>; 3],
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct CamoDescriptor {
    pub group: u32,
    pub camo: Option<String>,
    pub mask: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ViewmodelMaterials {
    pub models: Vec<String>,
    pub descriptors: Vec<CamoDescriptor>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct PreparedVariant {
    pub inputs: Value,
    pub surface: Option<Value>,
    pub pending: Option<String>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Library {
    pub version: u32,
    pub definitions: BTreeMap<String, CamoDefinition>,
    pub options: Vec<Option<CamoOption>>,
    pub weapons: BTreeMap<String, WeaponMaterials>,
    pub attachments: BTreeMap<String, AttachmentMaterials>,
    pub knife_camo: String,
    pub optic_switch: f32,
    pub materials: BTreeMap<String, Value>,
    pub variants: Vec<PreparedVariant>,
}

impl Library {
    pub fn validate(&self) -> Result<()> {
        if self.version != 3
            || self.options.len() != 128
            || !self.optic_switch.is_finite()
            || !(0. ..=1.).contains(&self.optic_switch)
            || !self.definitions.contains_key(&self.knife_camo)
        {
            return Err("Invalid native camo library controls".into());
        }
        for weapon in self.weapons.values() {
            if weapon.main_models.len() != 16
                || weapon.attachments.len() != 95
                || weapon.base_attachment_flags.len() != 63
            {
                return Err("Native weapon material selector table has the wrong size".into());
            }
            for name in weapon.attachments.iter().flatten() {
                if !self.attachments.contains_key(name) {
                    return Err(format!("Missing native attachment selector {name}"));
                }
            }
        }
        for name in self
            .weapons
            .values()
            .filter_map(|weapon| weapon.alternate_weapon_name.as_ref())
            .chain(
                self.attachments
                    .values()
                    .filter_map(|attachment| attachment.alternate_weapon_name.as_ref()),
            )
        {
            if !self.weapons.contains_key(name) {
                return Err(format!("Missing authored native alternate weapon {name}"));
            }
        }
        for name in self
            .weapons
            .values()
            .filter_map(|weapon| weapon.camo.as_ref())
            .chain(
                self.attachments
                    .values()
                    .filter_map(|attachment| attachment.camo.as_ref()),
            )
        {
            if !self.definitions.contains_key(name) {
                return Err(format!("Missing selected native camo {name}"));
            }
        }
        for variant in &self.variants {
            let name = variant.inputs["name"]
                .as_str()
                .ok_or("Camo variant has no native material")?;
            if !self.materials.contains_key(name)
                || variant.surface.is_some() == variant.pending.is_some()
            {
                return Err(format!("Invalid prepared camo variant {name}"));
            }
            if let Some(surface) = &variant.surface {
                if surface["version"] != 3
                    || surface["identity"].as_str().is_none_or(str::is_empty)
                    || surface["pending_runtime"].as_str().is_some()
                {
                    return Err(format!("Unresolved prepared camo variant {name}"));
                }
            }
        }
        Ok(())
    }

    pub fn resolve_knife(&mut self) -> Result<()> {
        if self.definitions.contains_key(&self.knife_camo) {
            return Ok(());
        }
        if self.knife_camo != "camo_knife" {
            return Err(format!(
                "Unreviewed missing knife camo selector {}",
                self.knife_camo
            ));
        }
        let template = self
            .definitions
            .get("defaultcamo")
            .ok_or("Native knife camo lookup requires the missing installed defaultcamo template")?
            .clone();
        self.definitions.insert(self.knife_camo.clone(), template);
        Ok(())
    }

    pub fn bound_inputs(
        &self,
        queue: &[CamoOverride],
        original: &str,
        model_index: u32,
    ) -> Result<Value> {
        let selected = camo_material(queue, original, model_index);
        let material = self
            .materials
            .get(selected)
            .ok_or_else(|| format!("Selected native camo material {selected} is missing"))?;
        camo_inputs(
            queue,
            material,
            model_index,
            &BTreeMap::from([(181, [0.; 4])]),
        )
        .map(|(bound, _)| bound)
    }

    pub fn prepared(&self, inputs: &Value) -> Result<&Value> {
        let name = inputs["name"]
            .as_str()
            .ok_or("Bound camo input has no native material")?;
        let variant = self
            .variants
            .iter()
            .find(|variant| variant.inputs == *inputs)
            .ok_or_else(|| format!("Native camo input state is not prepared: {name}"))?;
        variant.surface.as_ref().ok_or_else(|| {
            variant
                .pending
                .clone()
                .unwrap_or_else(|| format!("Native camo variant is incomplete: {name}"))
        })
    }

    pub fn needs_binding(&self, original: &str, inputs: &Value) -> Result<bool> {
        let material = self
            .materials
            .get(original)
            .ok_or_else(|| format!("Native material {original} is missing"))?;
        let baseline = camo_inputs(&[], material, 0, &BTreeMap::from([(181, [0.; 4])]))?.0;
        let parameter = material["techniques"]
            .as_array()
            .ok_or("Missing native material techniques")?
            .iter()
            .filter(|technique| technique["index"] == 4 || technique["index"] == 6)
            .flat_map(|technique| technique["passes"].as_array().into_iter().flatten())
            .flat_map(|pass| pass["arguments"].as_array().into_iter().flatten())
            .any(|argument| {
                argument["type"] == 5
                    && matches!(argument["value"].as_u64(), Some(16777397 | 16777401))
            });
        Ok(parameter || baseline != *inputs)
    }

    pub fn secondary_weapon(
        &self,
        weapon: &WeaponMaterials,
        packed: u32,
    ) -> Result<Option<&WeaponMaterials>> {
        let mut enabled = weapon.shared_ammo_related != 0;
        if !enabled {
            for selector in [8, 14, 20].map(|shift| packed >> shift & 63) {
                let Some(flag) = (selector != 0)
                    .then(|| weapon.base_attachment_flags.get(selector as usize))
                    .flatten()
                    .copied()
                    .flatten()
                else {
                    if selector >= weapon.base_attachment_flags.len() as u32 {
                        return Err(format!(
                            "Native base attachment selector {selector} exceeds its table"
                        ));
                    }
                    break;
                };
                if flag != 0 {
                    enabled = true;
                    break;
                }
            }
        }
        if !enabled {
            return Ok(None);
        }
        if weapon.definition44 != 3 && weapon.attachments.first().is_some_and(Option::is_some) {
            for name in selected_attachments(weapon, packed)?
                .into_iter()
                .take_while(Option::is_some)
                .flatten()
            {
                let attachment = self
                    .attachments
                    .get(name)
                    .ok_or_else(|| format!("Missing native alternate attachment {name}"))?;
                if attachment.alternate_weapon_name.is_some()
                    || attachment.alternate_weapon_index != 0
                {
                    return Err("Native attachment-selected secondary requires registered child table inheritance and backlink resolution".into());
                }
            }
        }
        let name = weapon
            .alternate_weapon_name
            .as_ref()
            .ok_or("Native secondary weapon index has no recovered authored-name mapping")?;
        self.weapons
            .get(name)
            .map(Some)
            .ok_or_else(|| format!("Selected authored native secondary weapon {name} is missing"))
    }
}

fn selected_attachments(weapon: &WeaponMaterials, packed: u32) -> Result<[Option<&str>; 3]> {
    let at = |index: u32| {
        weapon
            .attachments
            .get(index as usize)
            .map(|value| value.as_deref())
            .ok_or_else(|| format!("Native attachment selector {index} exceeds its table"))
    };
    let [a, b, c] = [8, 14, 20].map(|shift| packed >> shift & 63);
    let default = at(0)?;
    let mut out = [None, None, None];
    if c != 0 {
        out = [default, default, at(c)?];
    }
    if b != 0 {
        out[1] = at(b)?.or(default);
        out[0] = default;
    }
    if a != 0 {
        out[0] = at(a)?.or(default);
    }
    Ok(out)
}

fn combined_attachment<'a>(
    weapon: &'a WeaponMaterials,
    attachments: &'a BTreeMap<String, AttachmentMaterials>,
    selected: &AttachmentMaterials,
    packed: u32,
) -> Result<Option<&'a str>> {
    let requested = selected.attachment_type;
    let at = |index: u32| -> Result<Option<&str>> {
        weapon
            .attachments
            .get(index as usize)
            .map(|value| value.as_deref())
            .ok_or_else(|| format!("Native combined attachment {index} exceeds its table"))
    };
    if requested == 0 {
        return at(0);
    }
    let original =
        at(requested)?.ok_or_else(|| format!("Native attachment {requested} is missing"))?;
    let definition = |name: &str| {
        attachments
            .get(name)
            .ok_or_else(|| format!("Native attachment definition {name} is missing"))
    };
    let selectors = [8, 14, 20].map(|shift| packed >> shift & 63);
    let bits = selectors
        .into_iter()
        .fold(0, |bits, selector| bits | 1_u32.wrapping_shl(selector))
        & !1_u32.wrapping_shl(selected.attachment_type)
        & !1;
    let mut next = definition(original)?.chain_head;
    let mut visited = std::collections::BTreeSet::new();
    while next != 0 {
        if !visited.insert(next) {
            return Err("Native attachment combination chain is cyclic".into());
        }
        let Some(name) = at(next)? else {
            return Ok(Some(original));
        };
        let candidate = definition(name)?;
        if candidate.combination_mask & bits != 0 {
            return Ok(Some(name));
        }
        next = candidate.chain_next;
    }
    Ok(Some(original))
}

pub fn viewmodel_materials(
    weapon: &WeaponMaterials,
    variant_weapon: Option<&WeaponMaterials>,
    attachments: &BTreeMap<String, AttachmentMaterials>,
    initial_model: &str,
    packed_weapon: u32,
    alternate_packed_weapon: u32,
    main_model_index: usize,
    mode: u8,
    secondary: Option<(&WeaponMaterials, u32)>,
    cached_models: [Option<&str>; 2],
) -> Result<ViewmodelMaterials> {
    if mode > 1 {
        return Err("Native viewmodel mode is outside its source domain".into());
    }
    let main = weapon
        .main_models
        .get(main_model_index)
        .and_then(Option::as_ref)
        .ok_or_else(|| format!("Selected native viewmodel {main_model_index} is missing"))?;
    let mut output = ViewmodelMaterials {
        models: vec![initial_model.into(), main.clone()],
        descriptors: vec![CamoDescriptor {
            group: if weapon.definition28 == 7 { 2 } else { 0 },
            camo: weapon.camo.clone(),
            mask: 2,
        }],
    };
    append_viewmodel_parts(
        &mut output,
        weapon,
        variant_weapon,
        attachments,
        packed_weapon,
        alternate_packed_weapon,
        mode,
    )?;
    if let Some((secondary, packed)) = secondary {
        let model = secondary
            .main_models
            .first()
            .and_then(Option::as_ref)
            .ok_or("Selected native secondary viewmodel zero is missing")?;
        let mask = 1_u32.wrapping_shl(output.models.len() as u32);
        output.models.push(model.clone());
        if output.descriptors.len() < 8 {
            output.descriptors.push(CamoDescriptor {
                group: 0,
                camo: secondary.camo.clone(),
                mask,
            });
        }
        append_viewmodel_parts(
            &mut output,
            secondary,
            variant_weapon,
            attachments,
            packed,
            alternate_packed_weapon,
            mode,
        )?;
    }
    output
        .models
        .extend(cached_models.into_iter().flatten().map(str::to_owned));
    Ok(output)
}

fn append_viewmodel_parts(
    output: &mut ViewmodelMaterials,
    weapon: &WeaponMaterials,
    variant_weapon: Option<&WeaponMaterials>,
    attachments: &BTreeMap<String, AttachmentMaterials>,
    packed_weapon: u32,
    alternate_packed_weapon: u32,
    mode: u8,
) -> Result<()> {
    let original_selected = selected_attachments(weapon, packed_weapon)?;
    let suppress = |field: fn(&AttachmentMaterials) -> u8| -> Result<bool> {
        for name in original_selected
            .into_iter()
            .take_while(Option::is_some)
            .flatten()
        {
            let attachment = attachments
                .get(name)
                .ok_or_else(|| format!("Native attachment {name} is missing"))?;
            if field(attachment) != 0 {
                return Ok(true);
            }
        }
        Ok(false)
    };
    let suppress168 = suppress(|attachment| attachment.suppress168)?;
    let suppress169 = suppress(|attachment| attachment.suppress169)?;
    let (variants, packed) = if weapon.definition44 == 3 {
        (
            variant_weapon.ok_or("Native alternate weapon selection is missing")?,
            alternate_packed_weapon,
        )
    } else {
        (weapon, packed_weapon)
    };
    for (slot, model) in variants.variant_models.iter().enumerate() {
        if (matches!(slot, 0 | 7) && suppress168)
            || (slot == 6 && suppress169)
            || (slot == 7 && mode == 0)
            || (slot == 0 && mode == 1 && variants.variant_models[7].is_some())
        {
            continue;
        }
        if let Some(model) = model {
            let index = output.models.len() as u32;
            output.descriptors.last_mut().unwrap().mask |= 1_u32.rotate_left(index);
            output.models.push(model.clone());
        }
    }
    let default = variants.attachments.first().and_then(Option::as_deref);
    for name in selected_attachments(variants, packed)?
        .into_iter()
        .flatten()
        .filter(|name| Some(*name) != default)
    {
        let original = attachments
            .get(name)
            .ok_or_else(|| format!("Native attachment {name} is missing"))?;
        let Some(displayed) = combined_attachment(variants, attachments, original, packed_weapon)?
        else {
            continue;
        };
        let selected = &attachments[displayed];
        let Some(model) = (mode == 1)
            .then_some(selected.models[2].as_ref())
            .flatten()
            .or(selected.models[0].as_ref())
        else {
            continue;
        };
        let index = output.models.len() as u32;
        let mask = (if selected.models[1].is_some() {
            3_u32
        } else {
            1_u32
        })
        .wrapping_shl(index);
        output.models.push(model.clone());
        if let Some(second) = &selected.models[1] {
            output.models.push(second.clone());
        }
        if output.descriptors.len() < 8 {
            output.descriptors.push(CamoDescriptor {
                group: if selected.attachment_type == 28 { 2 } else { 0 },
                camo: original.camo.clone(),
                mask,
            });
        }
    }
    Ok(())
}

pub fn queue_descriptors(
    descriptors: &[CamoDescriptor],
    requested_group: u32,
    knife_camo: Option<&str>,
    definitions: &BTreeMap<String, CamoDefinition>,
    options: &[Option<CamoOption>],
    packed_options: u32,
    mapped_distance: f32,
    materials: &BTreeMap<String, Value>,
) -> Result<Vec<CamoOverride>> {
    if descriptors.len() > 8 {
        return Err("Native camo descriptor capacity exceeded".into());
    }
    let mut queue = Vec::new();
    for descriptor in descriptors
        .iter()
        .filter(|descriptor| descriptor.group == requested_group)
    {
        let name = if requested_group == 2 {
            knife_camo
        } else {
            descriptor.camo.as_deref()
        };
        let Some(name) = name else { continue };
        let camo = definitions
            .get(name)
            .ok_or_else(|| format!("Selected native camo {name} is missing"))?;
        queue_camo(
            camo,
            options,
            packed_options,
            mapped_distance,
            descriptor.mask,
            materials,
            &mut queue,
        )?;
    }
    Ok(queue)
}

pub fn camo_dependencies(
    camos: &BTreeMap<String, CamoDefinition>,
    options: &[Option<CamoOption>],
    packed_options: u32,
) -> std::collections::BTreeSet<String> {
    let mut names = std::collections::BTreeSet::new();
    let id = (packed_options & 127) as usize;
    let Some(option) = options
        .get(id)
        .copied()
        .flatten()
        .filter(|option| id != 0 && option.material)
    else {
        return names;
    };
    let Some(index) = option
        .selection
        .checked_sub(1)
        .and_then(|value| usize::try_from(value).ok())
    else {
        return names;
    };
    for camo in camos.values() {
        for record in camo.material_sets.get(index).into_iter().flatten() {
            for (from, to) in &record.pairs {
                if let (Some(from), Some(to)) = (from, to)
                    && from != to
                {
                    names.extend([from.clone(), to.clone()]);
                }
            }
        }
    }
    names
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub enum CamoOverride {
    Image {
        mask: u32,
        scope: Option<String>,
        from: String,
        to: String,
    },
    Material {
        mask: u32,
        from: String,
        to: String,
    },
    Constant {
        mask: u32,
        scope: Option<String>,
        index: u32,
        value: [f32; 4],
    },
}

fn queue_camo_image(
    queue: &mut Vec<CamoOverride>,
    mask: u32,
    scope: Option<&str>,
    from: Option<&str>,
    to: Option<&str>,
) -> bool {
    let (Some(from), Some(to)) = (from, to) else {
        return false;
    };
    if from == to || queue.len() >= 1024 {
        return false;
    }
    queue.push(CamoOverride::Image {
        mask,
        scope: scope.map(str::to_owned),
        from: from.into(),
        to: to.into(),
    });
    true
}

fn queue_camo_constant(
    queue: &mut Vec<CamoOverride>,
    mask: u32,
    scope: Option<&str>,
    index: u32,
    value: [f32; 4],
) {
    if queue.len() < 1024 {
        queue.push(CamoOverride::Constant {
            mask,
            scope: scope.map(str::to_owned),
            index,
            value,
        });
    }
}

pub fn queue_camo(
    camo: &CamoDefinition,
    options: &[Option<CamoOption>],
    packed_options: u32,
    mapped_distance: f32,
    mask: u32,
    materials: &BTreeMap<String, Value>,
    queue: &mut Vec<CamoOverride>,
) -> Result<()> {
    let id = (packed_options & 127) as usize;
    let Some(option) = options
        .get(id)
        .copied()
        .flatten()
        .filter(|_| id != 0 && mapped_distance < 1250.)
    else {
        return Ok(());
    };
    let Some(index) = option
        .selection
        .checked_sub(1)
        .and_then(|value| usize::try_from(value).ok())
    else {
        return Ok(());
    };
    if !option.material {
        if let Some(pattern) = camo.patterns.get(index)
            && queue_camo_image(
                queue,
                mask,
                None,
                camo.images[0].as_deref(),
                pattern.images[0].as_deref(),
            )
            && queue_camo_image(
                queue,
                mask,
                None,
                camo.images[1].as_deref(),
                pattern.images[1].as_deref(),
            )
        {
            queue_camo_constant(
                queue,
                mask,
                None,
                181,
                [pattern.offset[0], pattern.offset[1], pattern.scale, 1.],
            );
        }
        return Ok(());
    }
    for record in camo.material_sets.get(index).into_iter().flatten() {
        for (from, to) in &record.pairs {
            let (Some(from), Some(to)) = (from, to) else {
                continue;
            };
            if from == to || queue.len() >= 1024 {
                continue;
            }
            let source = materials
                .get(from)
                .ok_or_else(|| format!("Camo source material {from} is missing"))?;
            let destination = materials
                .get(to)
                .ok_or_else(|| format!("Camo destination material {to} is missing"))?;
            queue.push(CamoOverride::Material {
                mask,
                from: from.clone(),
                to: to.clone(),
            });
            queue_camo_constant(
                queue,
                mask,
                Some(to),
                185,
                [
                    record.constants[0],
                    record.constants[1],
                    record.constants[2],
                    record.constants[3] + if record.flags & 2 != 0 { 10. } else { 0. },
                ],
            );
            for (flag, source_semantic, destination_semantic) in [(1, 2, 2), (2, 5, 13), (4, 8, 14)]
            {
                if record.flags & flag == 0 {
                    continue;
                }
                for a in source["textures"]
                    .as_array()
                    .ok_or("Missing camo source texture table")?
                    .iter()
                    .filter(|texture| texture["semantic"] == source_semantic)
                {
                    for b in destination["textures"]
                        .as_array()
                        .ok_or("Missing camo destination texture table")?
                        .iter()
                        .filter(|texture| texture["semantic"] == destination_semantic)
                    {
                        queue_camo_image(
                            queue,
                            mask,
                            Some(to),
                            b["image"].as_str(),
                            a["image"].as_str(),
                        );
                    }
                }
            }
        }
    }
    Ok(())
}

pub fn camo_material<'a>(
    queue: &'a [CamoOverride],
    original: &'a str,
    model_index: u32,
) -> &'a str {
    let bit = u32::from((1_u32 << (model_index & 31)) as u16);
    queue
        .iter()
        .rev()
        .find_map(|command| {
            if let CamoOverride::Material { mask, from, to } = command {
                (*mask & bit != 0 && from == original).then_some(to.as_str())
            } else {
                None
            }
        })
        .unwrap_or(original)
}

pub fn camo_inputs(
    queue: &[CamoOverride],
    material: &Value,
    model_index: u32,
    initial: &BTreeMap<u32, [f32; 4]>,
) -> Result<(Value, BTreeMap<u32, [f32; 4]>)> {
    let mut result = material.clone();
    let name = material["name"]
        .as_str()
        .ok_or("Camo material identity is missing")?;
    let bit = u32::from((1_u32 << (model_index & 31)) as u16);
    let mut constants = initial.clone();
    for texture in result["textures"]
        .as_array_mut()
        .ok_or("Missing selected camo texture table")?
    {
        texture["image"] = json!(
            texture["image"]
                .as_str()
                .ok_or("Missing selected camo image identity")?
                .to_ascii_lowercase()
        );
    }
    for command in queue.iter().rev() {
        match command {
            CamoOverride::Image {
                mask,
                scope,
                from,
                to,
            } if mask & bit != 0 && scope.as_deref().is_none_or(|scope| scope == name) => {
                for texture in result["textures"]
                    .as_array_mut()
                    .ok_or("Missing selected camo texture table")?
                {
                    if texture["image"] == *from {
                        texture["image"] = json!(to);
                    }
                }
            }
            CamoOverride::Constant {
                mask,
                scope,
                index,
                value,
            } if mask & bit != 0 && scope.as_deref().is_none_or(|scope| scope == name) => {
                constants.insert(*index, *value);
            }
            _ => (),
        }
    }
    result["code_constants"] = json!(constants);
    Ok((result, constants))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn combined_viewmodel_uses_selected_descriptor_camo_and_actual_append_indices() {
        let mut weapon = WeaponMaterials {
            camo: Some("main_camo".into()),
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
        weapon.main_models[0] = Some("main".into());
        weapon.variant_models[0] = Some("hip".into());
        weapon.variant_models[7] = Some("ads".into());
        weapon.attachments[0] = Some("none".into());
        weapon.attachments[1] = Some("optic".into());
        weapon.attachments[12] = Some("grip".into());
        weapon.attachments[63] = Some("combined".into());
        let empty = AttachmentMaterials {
            camo: None,
            attachment_type: 0,
            chain_next: 0,
            chain_head: 0,
            combination_mask: 0,
            suppress168: 0,
            suppress169: 0,
            alternate_weapon_index: 0,
            alternate_weapon_name: None,
            models: [None, None, None],
        };
        let mut attachments = BTreeMap::from([
            ("none".into(), empty.clone()),
            (
                "optic".into(),
                AttachmentMaterials {
                    camo: Some("original_optic_camo".into()),
                    attachment_type: 1,
                    chain_head: 63,
                    ..empty.clone()
                },
            ),
            (
                "grip".into(),
                AttachmentMaterials {
                    attachment_type: 12,
                    models: [Some("grip_model".into()), None, None],
                    ..empty.clone()
                },
            ),
            (
                "combined".into(),
                AttachmentMaterials {
                    camo: Some("displayed_definition_camo".into()),
                    attachment_type: 63,
                    combination_mask: 1 << 12,
                    models: [
                        Some("combined_hip".into()),
                        Some("secondary".into()),
                        Some("combined_ads".into()),
                    ],
                    ..empty
                },
            ),
        ]);
        let packed = (1 << 8) | (12 << 14);
        for (mode, variant, optic) in [(0, "hip", "combined_hip"), (1, "ads", "combined_ads")] {
            let result = viewmodel_materials(
                &weapon,
                None,
                &attachments,
                "cached",
                packed,
                0,
                0,
                mode,
                None,
                [None; 2],
            )
            .unwrap();
            assert_eq!(
                result.models,
                ["cached", "main", variant, optic, "secondary", "grip_model"]
            );
            assert_eq!(result.descriptors[0].mask, 6);
            assert_eq!(result.descriptors[1].mask, 24);
            assert_eq!(
                result.descriptors[1].camo.as_deref(),
                Some("original_optic_camo")
            );
            assert_eq!(result.descriptors[2].mask, 32);
        }
        attachments.get_mut("optic").unwrap().suppress168 = 1;
        let result = viewmodel_materials(
            &weapon,
            None,
            &attachments,
            "cached",
            packed,
            0,
            0,
            1,
            None,
            [None; 2],
        )
        .unwrap();
        assert_eq!(
            result.models,
            ["cached", "main", "combined_ads", "secondary", "grip_model"]
        );
        assert_eq!(result.descriptors[0].mask, 2);
        assert_eq!(result.descriptors[1].mask, 12);

        attachments.get_mut("optic").unwrap().suppress168 = 0;
        let mut alternate = weapon.clone();
        alternate.variant_models[0] = Some("alternate_hip".into());
        alternate.variant_models[7] = None;
        alternate.attachments[63] = Some("alternate_combined".into());
        let mut combined = attachments["combined"].clone();
        combined.models = [Some("alternate_combined_model".into()), None, None];
        attachments.insert("alternate_combined".into(), combined);
        weapon.definition44 = 3;
        let result = viewmodel_materials(
            &weapon,
            Some(&alternate),
            &attachments,
            "cached",
            packed,
            packed,
            0,
            0,
            None,
            [None; 2],
        )
        .unwrap();
        assert_eq!(
            result.models,
            [
                "cached",
                "main",
                "alternate_hip",
                "alternate_combined_model",
                "grip_model"
            ]
        );
        assert_eq!(
            result.descriptors[1].camo.as_deref(),
            Some("original_optic_camo")
        );

        weapon.definition44 = 0;
        weapon.definition28 = 7;
        let mut secondary = weapon.clone();
        secondary.main_models[0] = Some("secondary_main".into());
        secondary.variant_models[0] = Some("secondary_hip".into());
        secondary.camo = Some("secondary_camo".into());
        let result = viewmodel_materials(
            &weapon,
            None,
            &attachments,
            "cached",
            0,
            0,
            0,
            0,
            Some((&secondary, packed)),
            [Some("gasmask"), Some("clip")],
        )
        .unwrap();
        assert_eq!(
            result.models,
            [
                "cached",
                "main",
                "hip",
                "secondary_main",
                "secondary_hip",
                "combined_hip",
                "secondary",
                "grip_model",
                "gasmask",
                "clip"
            ]
        );
        assert_eq!(
            result
                .descriptors
                .iter()
                .map(|descriptor| (descriptor.group, descriptor.mask))
                .collect::<Vec<_>>(),
            [(2, 6), (0, 24), (0, 96), (0, 128)]
        );
        assert_eq!(
            result.descriptors[1].camo.as_deref(),
            Some("secondary_camo")
        );
        assert!(
            result
                .descriptors
                .iter()
                .all(|descriptor| descriptor.mask & ((1 << 8) | (1 << 9)) == 0)
        );

        weapon.attachments[63] = None;
        attachments.get_mut("optic").unwrap().models[0] = Some("uncombined_optic".into());
        let result = viewmodel_materials(
            &weapon,
            None,
            &attachments,
            "cached",
            packed,
            0,
            0,
            0,
            None,
            [None; 2],
        )
        .unwrap();
        assert!(
            result
                .models
                .iter()
                .any(|model| model == "uncombined_optic")
        );
    }

    #[test]
    fn camo_pattern_requires_both_image_swaps_and_keeps_partial_failure() {
        let mut options = vec![None; 128];
        options[1] = Some(CamoOption {
            selection: 1,
            material: false,
        });
        let mut camo = CamoDefinition {
            images: [Some("solid".into()), Some("pattern".into())],
            patterns: vec![CamoPattern {
                images: [Some("red".into()), Some("stripes".into())],
                offset: [0.25, -0.5],
                scale: 3.,
            }],
            material_sets: Vec::new(),
        };
        let mut queue = Vec::new();
        queue_camo(&camo, &options, 129, 0., 2, &BTreeMap::new(), &mut queue).unwrap();
        let material = json!({"name":"weapon","textures":[{"image":"solid"},{"image":"pattern"}]});
        let initial = BTreeMap::from([(181, [0.; 4]), (185, [0.; 4])]);
        let (bound, constants) = camo_inputs(&queue, &material, 1, &initial).unwrap();
        assert_eq!(bound["textures"][0]["image"], "red");
        assert_eq!(bound["textures"][1]["image"], "stripes");
        assert_eq!(constants[&181], [0.25, -0.5, 3., 1.]);
        assert_eq!(
            camo_inputs(&queue, &material, 0, &initial).unwrap().1[&181],
            [0.; 4]
        );
        camo.patterns[0].images[1] = Some("pattern".into());
        queue.clear();
        queue_camo(&camo, &options, 1, 0., 2, &BTreeMap::new(), &mut queue).unwrap();
        assert_eq!(queue.len(), 1);
        let (bound, constants) = camo_inputs(&queue, &material, 1, &initial).unwrap();
        assert_eq!(bound["textures"][0]["image"], "red");
        assert_eq!(bound["textures"][1]["image"], "pattern");
        assert_eq!(constants[&181], [0.; 4]);
        camo.patterns[0].images[1] = Some("stripes".into());
        queue = vec![
            CamoOverride::Constant {
                mask: 0,
                scope: None,
                index: 181,
                value: [0.; 4]
            };
            1023
        ];
        queue_camo(&camo, &options, 1, 0., 2, &BTreeMap::new(), &mut queue).unwrap();
        assert_eq!(queue.len(), 1024);
        assert_eq!(
            camo_inputs(&queue, &material, 1, &initial).unwrap().1[&181],
            [0.; 4]
        );
        for (id, distance) in [(0, 0.), (1, 1250.), (1, f32::NAN)] {
            queue.clear();
            queue_camo(
                &camo,
                &options,
                id,
                distance,
                2,
                &BTreeMap::new(),
                &mut queue,
            )
            .unwrap();
            assert!(queue.is_empty());
        }
    }

    #[test]
    fn camo_material_preserves_scope_semantics_and_native_consumer_order() {
        let mut options = vec![None; 128];
        options[1] = Some(CamoOption {
            selection: 1,
            material: true,
        });
        let camo = CamoDefinition {
            images: [None, None],
            patterns: Vec::new(),
            material_sets: vec![vec![CamoRecord {
                flags: 7,
                pairs: vec![(Some("a".into()), Some("b".into()))],
                constants: [6., 6., 0.55, 0., 1., 2., 3., 4.],
            }]],
        };
        let materials = BTreeMap::from([
            (
                "a".into(),
                json!({"name":"a","textures":[{"semantic":2,"image":"basecolor"},{"semantic":5,"image":"basenormal"},{"semantic":8,"image":"basespec"}]}),
            ),
            (
                "b".into(),
                json!({"name":"b","textures":[{"semantic":2,"image":"camocolor"},{"semantic":13,"image":"camonormal"},{"semantic":14,"image":"camospec"}]}),
            ),
        ]);
        let mut queue = Vec::new();
        queue_camo(&camo, &options, 1, 0., 2, &materials, &mut queue).unwrap();
        assert_eq!(queue.len(), 5);
        assert_eq!(camo_material(&queue, "a", 1), "b");
        assert_eq!(camo_material(&queue, "a", 0), "a");
        assert_eq!(camo_material(&queue, "a", 17), "a");
        assert_eq!(camo_material(&queue, "a", 33), "b");
        let initial = BTreeMap::from([(181, [0.; 4]), (185, [0.; 4])]);
        let (bound, constants) = camo_inputs(&queue, &materials["b"], 1, &initial).unwrap();
        assert_eq!(constants[&185], [6., 6., 0.55, 10.]);
        assert_eq!(constants[&181], [0.; 4]);
        assert_eq!(
            bound["textures"]
                .as_array()
                .unwrap()
                .iter()
                .map(|texture| texture["image"].as_str().unwrap())
                .collect::<Vec<_>>(),
            ["basecolor", "basenormal", "basespec"]
        );
        assert_eq!(
            camo_inputs(&queue, &materials["b"], 17, &initial)
                .unwrap()
                .1[&185],
            [0.; 4]
        );
        let other = json!({"name":"other","textures":[{"image":"camonormal"}]});
        assert_eq!(
            camo_inputs(&queue, &other, 1, &initial).unwrap().0["textures"][0]["image"],
            "camonormal"
        );
        queue.push(CamoOverride::Material {
            mask: 2,
            from: "b".into(),
            to: "c".into(),
        });
        assert_eq!(camo_material(&queue, "a", 1), "b");
        queue.push(CamoOverride::Material {
            mask: 2,
            from: "a".into(),
            to: "d".into(),
        });
        assert_eq!(camo_material(&queue, "a", 1), "d");
        queue.clear();
        queue_camo_constant(&mut queue, 2, Some("b"), 185, [1.; 4]);
        queue_camo_constant(&mut queue, 2, Some("b"), 185, [2.; 4]);
        queue_camo_image(&mut queue, 2, Some("b"), Some("camocolor"), Some("red"));
        queue_camo_image(&mut queue, 2, Some("b"), Some("red"), Some("blue"));
        let (bound, constants) = camo_inputs(&queue, &materials["b"], 1, &initial).unwrap();
        assert_eq!(constants[&185], [1.; 4]);
        assert_eq!(bound["textures"][0]["image"], "red");
        let empty = CamoDefinition {
            material_sets: vec![vec![CamoRecord {
                pairs: Vec::new(),
                flags: 7,
                constants: [9.; 8],
            }]],
            ..camo
        };
        queue.clear();
        queue_camo(&empty, &options, 1, 0., 2, &materials, &mut queue).unwrap();
        assert!(queue.is_empty());
    }
}

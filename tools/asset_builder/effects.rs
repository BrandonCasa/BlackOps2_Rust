use crate::{
    catalog::Catalog,
    core::*,
    fastfile::{Address, Streams},
};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Default)]
pub struct Effects {
    pub definitions: BTreeMap<String, Value>,
    pub models: BTreeSet<String>,
    pub materials: BTreeSet<String>,
    pub sounds: BTreeSet<String>,
}

fn float(streams: &Streams, at: Address) -> Result<f32> {
    let value = f32::from_bits(streams.u32(at)?);
    if !value.is_finite() {
        return Err("Non-finite native FX component".into());
    }
    Ok(value)
}

fn floats(streams: &Streams, at: Address, count: usize) -> Result<Vec<f32>> {
    (0..count)
        .map(|index| float(streams, at.add(index * 4)?))
        .collect()
}

fn text(streams: &Streams, at: Address) -> Result<String> {
    let pointer = streams.u32(at)?;
    if pointer == 0 {
        return Ok(String::new());
    }
    Ok(streams
        .text(Address::decode(pointer)?)?
        .trim_start_matches(',')
        .to_owned())
}

fn velocity(streams: &Streams, at: Address) -> Result<Value> {
    Ok(
        json!({"velocity":[floats(streams,at,3)?,floats(streams,at.add(12)?,3)?],
        "delta":[floats(streams,at.add(24)?,3)?,floats(streams,at.add(36)?,3)?]}),
    )
}

fn state(streams: &Streams, at: Address) -> Result<Value> {
    Ok(
        json!({"color":streams.read(at,4)?.iter().map(|byte| *byte as f32).collect::<Vec<_>>(),
        "rotation_delta":float(streams,at.add(4)?)?,"rotation_total":float(streams,at.add(8)?)?,
        "size":floats(streams,at.add(12)?,2)?,"scale":float(streams,at.add(20)?)?}),
    )
}

fn material_inputs<'a>(name: &str, native: &'a Value) -> Result<(&'a Value, &'static str, u64)> {
    let family = native["technique_set"]
        .as_str()
        .ok_or("Native FX material family is absent")?;
    let index = if family.starts_with("mc_lit_") || family.starts_with("wc_lit_") {
        4
    } else {
        3
    };
    let pass = crate::material::contract::pass(native, index)?;
    let binding = crate::material::contract::texture_binding(native, 2695565377)?;
    if !pass["arguments"]
        .as_array()
        .ok_or("Native FX shader arguments are absent")?
        .iter()
        .any(|argument| argument["type"] == 2 && argument["value"] == 2695565377_u32)
    {
        return Err(format!(
            "Required native FX material {name} ({family}) has no supported color-map consumer"
        )
        .into());
    }
    let state_index = native["state_entries"][index]
        .as_u64()
        .ok_or("Native FX state entry is absent")? as usize;
    let state = native["state_bits"][state_index][0]
        .as_u64()
        .ok_or("Native FX state bits are absent")?;
    let alpha = match state & 0x7ff {
        0x12 => "opaque",
        0x122 | 0x125 | 0x12a => "add",
        0x162 => "premultiplied",
        0x165 => "blend",
        0x131 => "multiply",
        blend => {
            return Err(format!(
                "Required FX material {name} has unsupported native blend state {blend:#x}"
            )
            .into());
        }
    };
    if binding["sampler"]
        .as_u64()
        .is_none_or(|sampler| sampler > 255)
    {
        return Err(format!("Required FX material {name} has invalid native sampler").into());
    }
    Ok((binding, alpha, state))
}

fn atlas_dimensions(catalog: &Catalog<'_>, name: &str) -> Result<[u8; 2]> {
    let (zone, root) = catalog.resolve(6, name)?;
    let streams = &catalog.zones[zone].1.streams;
    let columns = streams.u8(root.add(11)?)?;
    let rows = streams.u8(root.add(10)?)?;
    if columns == 0 || rows == 0 {
        return Err(format!("Required FX material {name} has zero native atlas dimensions").into());
    }
    Ok([columns, rows])
}

impl Effects {
    pub fn inputs(&self, catalog: &Catalog<'_>) -> Result<BTreeSet<String>> {
        let mut images = BTreeSet::new();
        for name in &self.materials {
            let material = crate::material::read(catalog, name)?;
            material_inputs(name, &material)?;
            atlas_dimensions(catalog, name)?;
            for binding in material["textures"]
                .as_array()
                .ok_or("Native FX material texture list is absent")?
            {
                let name = binding["image"]
                    .as_str()
                    .ok_or("Native FX material image is absent")?;
                catalog.resolve(8, name)?;
                images.insert(name.to_ascii_lowercase());
            }
        }
        Ok(images)
    }

    pub fn write(
        &self,
        catalog: &Catalog<'_>,
        models: &[Value],
        images: &[Value],
        initial: bool,
        pack: &mut crate::pack::Pack,
    ) -> Result<()> {
        let mut materials = BTreeMap::new();
        for name in &self.materials {
            let native = crate::material::read(catalog, name)?;
            let (binding, alpha, _) = material_inputs(name, &native)?;
            let [columns, rows] = atlas_dimensions(catalog, name)?;
            let image = images
                .iter()
                .find(|image| {
                    image["name"].as_str().is_some_and(|name| {
                        binding["image"]
                            .as_str()
                            .is_some_and(|source| source.eq_ignore_ascii_case(name))
                    })
                })
                .ok_or_else(|| {
                    format!(
                        "Required FX material {name} image {} was not converted",
                        binding["image"]
                    )
                })?;
            let uri = image["uris"]["srgb"]
                .as_str()
                .ok_or("Required FX color-map URI is absent")?;
            pack.get(uri)?;
            let reason = "Native FX source pixel equations, depth feathering, destination-alpha composition and source framebuffer transfer use the existing color-map PBR approximation";
            let mut material = json!({"texture":uri,"sampler":binding["sampler"],"presentation":{"alpha":alpha},"columns":columns,"rows":rows,"native":native});
            if initial {
                material["approximation_scope"] = json!("first_playable_solo");
                material["approximations"] = json!([reason]);
                println!("Initial visual approximation: FX material {name}: {reason}");
            } else {
                material["pending_runtime"] = json!(
                    "Native FX source pixel equations, depth feathering, destination-alpha composition and source framebuffer transfer require matching consumers"
                );
            }
            materials.insert(name, material);
        }
        let models = self
            .models
            .iter()
            .map(|name| {
                let model = models
                    .iter()
                    .find(|model| {
                        model["name"]
                            .as_str()
                            .is_some_and(|model| model.eq_ignore_ascii_case(name))
                    })
                    .ok_or_else(|| format!("Required FX model {name} was not exported"))?;
                let uri = model["uri"]
                    .as_str()
                    .ok_or("FX model scene URI is absent")?;
                pack.get(uri)?;
                Ok((name, format!("{uri}#Scene0")))
            })
            .collect::<Result<BTreeMap<_, _>>>()?;
        let mut document =
            json!({"effects":self.definitions,"materials":materials,"models":models});
        if initial {
            let mut approximations = Vec::new();
            for (name, effect) in &self.definitions {
                for (index, element) in effect["elements"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .enumerate()
                {
                    let kind = element["type"]
                        .as_u64()
                        .ok_or("Required FX element type is absent")?;
                    match kind {
                        5 | 6 => approximations.push(format!("{name}/{index}: native trail/cloud geometry uses the original texture and sampled curves")),
                        9 | 11 => approximations.push(format!("{name}/{index}: native {} is retained but not rendered", if kind == 9 { "spotlight" } else { "projected decal" })),
                        _ => (),
                    }
                    for field in ["impact", "emitted", "attached"] {
                        if let Some(child) =
                            element[field].as_str().filter(|child| !child.is_empty())
                        {
                            approximations.push(format!("{name}/{index}: native {field} effect {child} is retained; source collision/emission-distance consumer is unconnected"));
                        }
                    }
                }
            }
            document["approximation_scope"] = json!("first_playable_solo");
            document["approximations"] = json!(approximations);
        }
        pack.entries.insert(
            "prepared/effects.nativefx".into(),
            std::sync::Arc::new(serde_json::to_vec(&document)?),
        );
        Ok(())
    }

    pub fn read(catalog: &Catalog<'_>, roots: BTreeSet<String>) -> Result<Self> {
        let mut result = Self::default();
        let mut pending = roots;
        while let Some(name) = pending.pop_first() {
            let name = name.trim_start_matches(',').to_ascii_lowercase();
            if result.definitions.contains_key(&name) {
                continue;
            }
            let (zone, root) = catalog.resolve(33, &name)?;
            let streams = &catalog.zones[zone].1.streams;
            let counts = [8, 10, 12].map(|offset| {
                streams
                    .u16(root.add(offset).unwrap())
                    .map(|value| value as i16)
            });
            let counts = counts
                .into_iter()
                .collect::<std::result::Result<Vec<_>, _>>()?;
            if counts.iter().any(|count| *count < 0) {
                return Err(format!("Native FX {name} has negative element counts").into());
            }
            let count = counts.iter().map(|count| *count as usize).sum::<usize>();
            let array = if count == 0 {
                root
            } else {
                Address::decode(streams.u32(root.add(28)?)?)?
            };
            let mut elements = Vec::new();
            for index in 0..count {
                let at = array.add(index * 292)?;
                let kind = streams.u8(at.add(184)?)?;
                if kind > 12 {
                    return Err(format!(
                        "Native FX {name}/{index} has unknown element type {kind}"
                    )
                    .into());
                }
                let visual_count = streams.u8(at.add(185)?)? as usize;
                let field = at.add(196)?;
                let visual_array = if kind == 11 || visual_count > 1 {
                    (streams.u32(field)? != 0)
                        .then(|| Address::decode(streams.u32(field).unwrap()))
                        .transpose()?
                } else {
                    Some(field)
                };
                let mut visuals = Vec::new();
                let mut marks = Vec::new();
                for visual in 0..visual_count {
                    let array = visual_array.ok_or_else(|| {
                        format!("Native FX {name}/{index} has missing visual array")
                    })?;
                    if kind == 11 {
                        let mut pair = Vec::new();
                        for layer in 0..2 {
                            let field = array.add(visual * 8 + layer * 4)?;
                            let material = if streams.u32(field)? == 0 {
                                String::new()
                            } else {
                                catalog.pointer(zone, field, 6)?.0
                            };
                            if !material.is_empty() {
                                result.materials.insert(material.to_ascii_lowercase());
                            }
                            pair.push(material);
                        }
                        marks.push(pair);
                        continue;
                    }
                    let field = array.add(visual * 4)?;
                    if streams.u32(field)? == 0 {
                        continue;
                    }
                    let visual = match kind {
                        7 => {
                            let model = catalog.pointer(zone, field, 5)?.0;
                            result.models.insert(model.to_ascii_lowercase());
                            model
                        }
                        9 => catalog.pointer(zone, field, 18)?.0,
                        10 | 12 => text(streams, field)?,
                        8 => continue,
                        _ => {
                            let material = catalog.pointer(zone, field, 6)?.0;
                            result.materials.insert(material.to_ascii_lowercase());
                            material
                        }
                    };
                    if kind == 12 {
                        pending.insert(visual.clone());
                    }
                    if kind == 10 {
                        result.sounds.insert(visual.to_ascii_lowercase());
                    }
                    visuals.push(visual);
                }
                let mut children = BTreeMap::new();
                for (key, offset) in [
                    ("impact", 224),
                    ("death", 228),
                    ("emitted", 232),
                    ("attached", 252),
                ] {
                    let child = text(streams, at.add(offset)?)?;
                    if !child.is_empty() {
                        pending.insert(child.clone());
                    }
                    children.insert(key, child);
                }
                let sound = text(streams, at.add(280)?)?;
                if !sound.is_empty() {
                    result.sounds.insert(sound.to_ascii_lowercase());
                }
                let velocity_count = streams.u8(at.add(186)?)? as usize + 1;
                let state_count = streams.u8(at.add(187)?)? as usize + 1;
                let velocity_pointer = streams.u32(at.add(188)?)?;
                let state_pointer = streams.u32(at.add(192)?)?;
                let mut velocities = Vec::new();
                if velocity_pointer != 0 {
                    let array = Address::decode(velocity_pointer)?;
                    for index in 0..velocity_count {
                        let sample = array.add(index * 96)?;
                        velocities.push(json!({"local":velocity(streams,sample)?,"world":velocity(streams,sample.add(48)?)?}));
                    }
                }
                let mut states = Vec::new();
                if state_pointer != 0 {
                    let array = Address::decode(state_pointer)?;
                    for index in 0..state_count {
                        let sample = array.add(index * 48)?;
                        states.push(json!({"base":state(streams,sample)?,"amplitude":state(streams,sample.add(24)?)?}));
                    }
                }
                let ranges = |offset: usize| -> Result<Vec<Vec<f32>>> {
                    (0..3)
                        .map(|axis| floats(streams, at.add(offset + axis * 8)?, 2))
                        .collect()
                };
                let ints = |offset: usize| -> Result<Vec<i32>> {
                    (0..2)
                        .map(|index| Ok(streams.u32(at.add(offset + index * 4)?)? as i32))
                        .collect()
                };
                let atlas = (0..6)
                    .map(|index| streams.u8(at.add(172 + index)?).map(u32::from))
                    .chain(std::iter::once(streams.u16(at.add(178)?).map(u32::from)))
                    .collect::<std::result::Result<Vec<_>, _>>()?;
                let extended = streams.u32(at.add(256)?)?;
                let extended = if extended == 0 {
                    Value::Null
                } else {
                    let extra = Address::decode(extended)?;
                    match kind {
                        5 => {
                            let vertices = streams.u32(extra.add(12)?)? as usize;
                            let indices = streams.u32(extra.add(20)?)? as usize;
                            let vertex_array = Address::decode(streams.u32(extra.add(16)?)?)?;
                            let index_array = Address::decode(streams.u32(extra.add(24)?)?)?;
                            json!({"scroll_ms":streams.u32(extra)? as i32,"repeat_distance":streams.u32(extra.add(4)?)? as i32,
                                "split_distance":streams.u32(extra.add(8)?)? as i32,
                                "vertices":(0..vertices).map(|index|floats(streams,vertex_array.add(index*20)?,5)).collect::<Result<Vec<_>>>()?,
                                "indices":(0..indices).map(|index|streams.u16(index_array.add(index*2).unwrap())).collect::<std::result::Result<Vec<_>,_>>()?})
                        }
                        9 => json!({"spot":floats(streams,extra,3)?}),
                        _ => json!(streams.u8(extra)?),
                    }
                };
                let mut element = json!({"type":kind,"flags":streams.u32(at)? as i32,"looping":index < counts[0] as usize,
                    "emission":index >= (counts[0]+counts[1]) as usize,"spawn":ints(4)?,"spawn_range":floats(streams,at.add(12)?,2)?,
                    "fade_in":floats(streams,at.add(20)?,2)?,"fade_out":floats(streams,at.add(28)?,2)?,"cull_radius":float(streams,at.add(36)?)?,
                    "delay":ints(40)?,"life":ints(48)?,"origin":ranges(56)?,"radius":floats(streams,at.add(80)?,2)?,"height":floats(streams,at.add(88)?,2)?,
                    "angles":ranges(96)?,"angular_velocity":ranges(120)?,"rotation":floats(streams,at.add(144)?,2)?,"rotation_axis":streams.u32(at.add(152)?)?,
                    "gravity":floats(streams,at.add(156)?,2)?,"reflection":floats(streams,at.add(164)?,2)?,"atlas":atlas});
                let extra = json!({"wind":float(streams,at.add(180)?)?,"velocities":velocities,"states":states,"visuals":visuals,"marks":marks,"extended":extended,
                    "collision_bounds":[floats(streams,at.add(200)?,3)?,floats(streams,at.add(212)?,3)?],"impact":children["impact"],"death":children["death"],
                    "emitted":children["emitted"],"emit_distance":floats(streams,at.add(236)?,2)?,"emit_variance":floats(streams,at.add(244)?,2)?,
                    "attached":children["attached"],"sound":sound,"pivot":floats(streams,at.add(284)?,2)?,
                    "sort_order":streams.u8(at.add(260)?)?,"lighting_fraction":streams.u8(at.add(261)?)?,"alpha_fade_ms":streams.u16(at.add(264)?)?,
                    "max_wind_strength":streams.u16(at.add(266)?)?,"wind_interval":streams.u16(at.add(268)?)?,"wind_life":streams.u16(at.add(270)?)?,"shape":streams.read(at.add(272)?,8)?});
                element
                    .as_object_mut()
                    .unwrap()
                    .extend(extra.as_object().unwrap().clone());
                elements.push(element);
            }
            result.definitions.insert(name,json!({"elements":elements,"loop_life":streams.u32(root.add(20)?)? as i32,
                "oneshot_life":streams.u32(root.add(24)?)? as i32,"flags":streams.u16(root.add(4)?)?,"priority":streams.u8(root.add(6)?)?,
                "occlusion_query":{"depth_bias":float(streams,root.add(0x38)?)?,
                    "fade_in_ms":streams.u32(root.add(0x3c)?)? as i32,"fade_out_ms":streams.u32(root.add(0x40)?)? as i32,
                    "scale_range":floats(streams,root.add(0x44)?,2)?}}));
        }
        Ok(result)
    }
}

pub fn initial_roots(catalog: &Catalog<'_>, game: &Value) -> Result<BTreeSet<String>> {
    let mut roots = [
        "impacts/fx_flesh_hit",
        "misc/fx_zombie_bloodsplat",
        "misc/fx_zombie_bloodspurt",
        "weapon/bullet/fx_flesh_gib_fatal_01",
        "trail/fx_trail_blood_streak",
        "misc/fx_zombie_eye_single",
        "misc/fx_zombie_cola_revive_on",
        "maps/zombie/fx_zmb_cola_revive_flicker",
        "misc/fx_zombie_couch_effect",
        "maps/zombie_tomb/fx_tomb_capture_progression",
        "maps/zombie_tomb/fx_tomb_capture_complete",
        "maps/zombie_tomb/fx_tomb_capture_exhaust_back",
        "maps/zombie_tomb/fx_tomb_emergence_spawn",
        "maps/zombie_alcatraz/fx_alcatraz_afterlife_zmb_tport",
        "maps/zombie_tomb/fx_tomb_screecher_vortex",
        "maps/zombie_tomb/fx_tomb_crusader_torso_loop",
        "maps/zombie/fx_zombie_crusader_eyes",
        "maps/zombie_tomb/fx_tomb_capture_light_red",
        "maps/zombie_tomb/fx_tomb_challenge_fire",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect::<BTreeSet<_>>();
    if game["delivery_scope"] != "first_playable_solo" {
        roots.insert("maps/zombie_tomb/fx_tomb_capture_light_green".into());
    }
    for weapon in game["content"]["weapons"].as_array().into_iter().flatten() {
        let name = weapon["id"]
            .as_str()
            .and_then(|name| name.strip_prefix("weapon."))
            .ok_or("Selected weapon lacks native ID")?;
        let (zone, root) = catalog.resolve(25, name)?;
        let streams = &catalog.zones[zone].1.streams;
        let def = Address::decode(streams.u32(root.add(8)?)?)?;
        for offset in [
            108, 112, 116, 464, 468, 472, 476, 1776, 1784, 1792, 1800, 1808, 1816, 1888, 1916,
        ] {
            if streams.u32(def.add(offset)?)? != 0 {
                roots.insert(
                    catalog
                        .pointer(zone, def.add(offset)?, 33)
                        .map_err(|error| format!("Initial FX {name} WeaponDef+{offset}: {error}"))?
                        .0,
                );
            }
        }
    }
    let (zone, root) = catalog.resolve(59, "zmcore_BasicWoodBarrier")?;
    let streams = &catalog.zones[zone].1.streams;
    for index in 0..streams.u32(root.add(52)?)? as usize {
        for offset in [20, 24] {
            let field = root.add(80 + index * 80 + offset)?;
            if streams.u32(field)? != 0 {
                roots.insert(catalog.pointer(zone, field, 33)?.0);
            }
        }
    }
    Ok(roots)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn incomplete_effects_cannot_pass_pack_check() {
        let mut pack = crate::pack::Pack::default();
        pack.entries.insert("prepared/effects.nativefx".into(),std::sync::Arc::new(serde_json::to_vec(&json!({"materials":{"source_material":{"pending_runtime":"required native dissolve consumer is missing"}}})).unwrap()));
        assert!(
            crate::validate_entries(&pack)
                .unwrap_err()
                .to_string()
                .contains("required native dissolve consumer is missing")
        );
    }

    #[test]
    #[ignore = "requires the authorized Steam installation"]
    fn installed_initial_effect_dependencies() {
        let zones = crate::source::read_zones(&crate::source::installation().unwrap()).unwrap();
        let catalog = Catalog::new(&zones);
        let entities = crate::entities::read(&catalog).unwrap();
        let (game, _) = crate::game::read(&catalog, &entities).unwrap();
        let roots = initial_roots(&catalog, &game).unwrap();
        let effects = Effects::read(&catalog, roots.clone()).unwrap();
        let mut kinds = BTreeMap::<u64, usize>::new();
        for (name, effect) in &effects.definitions {
            for (index, element) in effect["elements"].as_array().unwrap().iter().enumerate() {
                *kinds.entry(element["type"].as_u64().unwrap()).or_default() += 1;
                let superflare =
                    element["visuals"]
                        .as_array()
                        .into_iter()
                        .flatten()
                        .any(|visual| {
                            visual.as_str().is_some_and(|name| {
                                name.eq_ignore_ascii_case(
                                    "gfx_fxt_superflare_zmb_tomb_elec_bolt_lg",
                                )
                            })
                        });
                if matches!(element["type"].as_u64(), Some(9 | 11))
                    || superflare
                    || ["impact", "emitted", "attached"].iter().any(|field| {
                        element[*field]
                            .as_str()
                            .is_some_and(|value| !value.is_empty())
                    })
                {
                    println!(
                        "Required FX {name}/{index}: type {}, visuals {}, marks {}, extended {}, impact {}, death {}, emitted {}, attached {}, emit_distance {}, emit_variance {}",
                        element["type"],
                        element["visuals"],
                        element["marks"],
                        element["extended"],
                        element["impact"],
                        element["death"],
                        element["emitted"],
                        element["attached"],
                        element["emit_distance"],
                        element["emit_variance"]
                    );
                    if superflare {
                        let colors: Vec<_> = element["states"]
                            .as_array()
                            .unwrap()
                            .iter()
                            .map(|sample| {
                                json!([sample["base"]["color"], sample["amplitude"]["color"]])
                            })
                            .collect();
                        println!(
                            "Required superflare {name}/{index}: flags {}; native color samples {colors:?}; atlas {}",
                            element["flags"], element["atlas"]
                        );
                        println!(
                            "Required superflare visibility {name}/{index}: fade_in {}, fade_out {}, lighting_fraction {}, effect flags {}, occlusion_query {}",
                            element["fade_in"],
                            element["fade_out"],
                            element["lighting_fraction"],
                            effect["flags"],
                            effect["occlusion_query"]
                        );
                        let bucket: Vec<_> = effect["elements"].as_array().unwrap().iter().enumerate()
                            .filter(|(source_index, _)| (*source_index).min(7) == index.min(7))
                            .map(|(source_index, definition)| json!({"index":source_index,"type":definition["type"],
                                "flags":definition["flags"],"looping":definition["looping"],"emission":definition["emission"],
                                "spawn":definition["spawn"],"delay":definition["delay"],"life":definition["life"],"sort_order":definition["sort_order"]})).collect();
                        println!(
                            "Required superflare bucket {name}/{index}: effect flags {}, loop_life {}, oneshot_life {}, definitions {bucket:?}",
                            effect["flags"], effect["loop_life"], effect["oneshot_life"]
                        );
                    }
                }
            }
        }
        let mut material_gaps = Vec::new();
        let mut blend_states = BTreeMap::<u64, (String, usize)>::new();
        let mut atlas_sizes = BTreeMap::<[u8; 2], usize>::new();
        assert!(
            effects
                .materials
                .contains("gfx_fxt_env_dust_mote_pcloud_add")
        );
        assert!(effects.materials.contains("wc/gfx_impact_blood_spatter03"));
        for name in &effects.materials {
            let mut native = crate::material::read(&catalog, name).unwrap();
            if name == "gfx_fxt_superflare_zmb_tomb_elec_bolt_lg" {
                println!(
                    "Required superflare active native pass: {}",
                    crate::material::contract::pass(&native, 3).unwrap()
                );
            }
            match atlas_dimensions(&catalog, name) {
                Ok(dimensions) => *atlas_sizes.entry(dimensions).or_default() += 1,
                Err(error) => material_gaps.push(error.to_string()),
            }
            match material_inputs(name, &native) {
                Ok((_, alpha, state)) => {
                    blend_states
                        .entry(state & 0x7ff)
                        .or_insert((alpha.into(), 0))
                        .1 += 1;
                    if name == "gfx_fxt_env_dust_mote_pcloud_add" {
                        assert_eq!(state & 0x7ff, 0x125);
                        assert_eq!(alpha, "add");
                        let state_index = native["state_entries"][3].as_u64().unwrap() as usize;
                        native["state_bits"][state_index][0] = json!((state & !0x7ff) | 0x225);
                        assert!(
                            material_inputs(name, &native)
                                .unwrap_err()
                                .to_string()
                                .contains("unsupported native blend state 0x225")
                        );
                    }
                    if name == "wc/gfx_impact_blood_spatter03" {
                        assert_eq!(state & 0x7ff, 0x162);
                        assert_eq!(alpha, "premultiplied");
                    }
                }
                Err(error) => material_gaps.push(error.to_string()),
            }
        }
        println!("Required FX native RGB blend states: {blend_states:?}");
        println!("Required FX native atlas dimensions: {atlas_sizes:?}");
        assert!(
            material_gaps.is_empty(),
            "Required FX material compatibility gaps:\n{}",
            material_gaps.join("\n")
        );
        assert!(
            roots
                .iter()
                .all(|root| effects.definitions.contains_key(&root.to_ascii_lowercase()))
        );
        assert!(
            effects
                .definitions
                .contains_key("impacts/fx_large_impact_core")
        );
        assert!(effects.models.contains("fx_pistol_shell"));
        assert!(effects.models.contains("fx_wood_splinter01"));
        assert!(!effects.inputs(&catalog).unwrap().is_empty());
        println!(
            "Required FX: {} definitions; {} models {:?}; {} materials; {} sounds {:?}; kinds {:?}",
            effects.definitions.len(),
            effects.models.len(),
            effects.models,
            effects.materials.len(),
            effects.sounds.len(),
            effects.sounds,
            kinds
        );
    }
}

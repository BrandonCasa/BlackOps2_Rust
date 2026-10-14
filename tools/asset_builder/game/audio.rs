use super::*;
use std::collections::{BTreeMap, BTreeSet};

fn text(catalog: &Catalog<'_>, zone: usize, field: Address) -> Result<Option<String>> {
    let streams = &catalog.zones[zone].1.streams;
    let pointer = streams.u32(field)?;
    if pointer == 0 {
        return Ok(None);
    }
    let value = streams.text(Address::decode(pointer)?)?;
    Ok((!value.is_empty()).then_some(value))
}

fn word(catalog: &Catalog<'_>, zone: usize, field: Address) -> Result<Option<String>> {
    let streams = &catalog.zones[zone].1.streams;
    let index = streams.u16(field)? as usize;
    if index == 0 {
        return Ok(None);
    }
    let address = streams
        .script_strings
        .get(index)
        .copied()
        .flatten()
        .ok_or("Native weapon sound-map script string is absent")?;
    Ok(Some(streams.text(address)?))
}

pub(super) fn read(catalog: &Catalog<'_>, game: &mut Value, gaps: &mut Vec<String>) -> Result<()> {
    let available: BTreeSet<_> = crate::sound::alias_hashes(catalog)?
        .into_values()
        .map(|name| name.to_ascii_lowercase())
        .collect();
    let mut bindings = game["audio"]["bindings"]
        .as_object()
        .cloned()
        .unwrap_or_default();
    let mut required: BTreeSet<_> = bindings
        .values()
        .filter_map(Value::as_str)
        .map(str::to_ascii_lowercase)
        .collect();
    let mut timelines = serde_json::Map::new();
    for name in super::equipment::INITIAL_WEAPONS {
        let (zone, root) = catalog.resolve(25, name)?;
        let streams = &catalog.zones[zone].1.streams;
        let definition = Address::decode(streams.u32(root.add(8)?)?)?;
        for (event, offset) in [
            ("shot", 180),
            ("dry_fire", 236),
            ("prime", 172),
            ("melee", 252),
            ("hit", 256),
            ("equip", 388),
            ("unequip", 412),
            ("projectile_loop", 164),
            ("explosion", 1820),
        ] {
            if let Some(alias) = text(catalog, zone, definition.add(offset)?)? {
                println!("Initial native weapon {name} sound field {offset} ({event}): {alias}");
                required.insert(alias.to_ascii_lowercase());
                bindings.insert(format!("weapon.{name}.{event}"), json!(alias));
            }
        }
        let mut notes = BTreeMap::new();
        if streams.u32(definition.add(16)?)? != 0 && streams.u32(definition.add(20)?)? != 0 {
            let keys = Address::decode(streams.u32(definition.add(16)?)?)?;
            let values = Address::decode(streams.u32(definition.add(20)?)?)?;
            for index in 0..20 {
                if let (Some(key), Some(value)) = (
                    word(catalog, zone, keys.add(index * 2)?)?,
                    word(catalog, zone, values.add(index * 2)?)?,
                ) {
                    notes.insert(key, value);
                }
            }
        }
        let actions = game["content"]["weapons"]
            .as_array()
            .into_iter()
            .flatten()
            .find(|weapon| weapon["id"] == format!("weapon.{name}"))
            .and_then(|weapon| weapon["animation_actions"].as_object())
            .ok_or_else(|| format!("Initial weapon {name} animation actions are missing"))?;
        for id in actions.values() {
            let animation = game["content"]["animations"]
                .as_array()
                .unwrap()
                .iter()
                .find(|animation| animation["id"] == *id)
                .ok_or("Initial weapon sound timeline lacks its animation")?;
            let sounds = animation["notifications"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|note| {
                    notes
                        .get(note["event"].as_str()?)
                        .map(|alias| (note, alias))
                })
                .map(|(note, alias)| {
                    required.insert(alias.to_ascii_lowercase());
                    json!({"seconds":note["seconds"],"alias":alias})
                })
                .collect::<Vec<_>>();
            if !sounds.is_empty() {
                let key = id
                    .as_str()
                    .ok_or("Initial weapon animation ID is invalid")?;
                if timelines
                    .get(key)
                    .is_some_and(|existing| existing != &json!(sounds))
                {
                    return Err(format!(
                        "Shared initial weapon clip {key} has conflicting sound maps"
                    )
                    .into());
                }
                timelines.insert(key.into(), json!(sounds));
            }
        }
    }
    for alias in [
        "zmb_cha_ching",
        "zmb_no_cha_ching",
        "mus_zombie_round_start",
        "mus_zombie_round_over",
        "mus_zombie_game_over",
        "zmb_attack_whoosh",
        "zmb_vocals_zombie_attack",
        "zmb_vocals_zombie_teardown",
        "zmb_vocals_zombie_sprint",
        "zmb_vocals_zombie_ambience",
        "zmb_vocals_zombie_death",
        "chr_breathing_hurt",
        "chr_breathing_hurt_start",
        "chr_breathing_better",
        "chr_revive_gasp",
        "mus_perks_revive_sting",
        "mus_perks_revive_jingle",
        "zmb_capturezone_donut_start",
        "zmb_capturezone_rise",
        "zmb_capturezone_donut_stop",
        "zmb_capturezone_success",
        "zmb_zone_plate_down",
        "zmb_zone_plate_up",
        "zmb_zone_plate_loop",
        "zmb_vocals_capzomb_spawn",
        "zmb_vocals_capzomb_attack",
        "zmb_vocals_capzomb_ambience",
        "zmb_vocals_capzomb_sprint",
        "zmb_vocals_capzomb_death",
    ] {
        if available.contains(alias) {
            required.insert(alias.into());
        } else {
            gaps.push(format!(
                "Initial sound consumer {alias} has no installed native alias binding"
            ));
        }
    }
    for alias in &available {
        if [
            "walk_plr_",
            "sprint_plr_",
            "crouch_walk_plr_",
            "walk_npc_",
            "run_npc_",
            "sprint_npc_",
        ]
        .iter()
        .any(|suffix| alias.starts_with(&format!("fly_step_{suffix}")))
            || alias.starts_with("fly_prone_")
            || (0..4).any(|character| {
                [
                    "exert_pain_high",
                    "exert_pain_low",
                    "ammo_out",
                    "ammo_low",
                    "kill_headshot",
                    "kill_explo",
                    "kill_close",
                    "heal_revived",
                    "perk_revive",
                ]
                .iter()
                .any(|event| alias.starts_with(&format!("vox_plr_{character}_{event}_")))
            })
        {
            required.insert(alias.clone());
        }
    }
    let rows = crate::sound::definitions(catalog, &required, gaps)?;
    let globals = catalog.definitions(32).collect::<Vec<_>>();
    if globals.len() != 1 {
        return Err(format!(
            "Initial native audio requires one active sound driver globals definition, found {}",
            globals.len()
        )
        .into());
    }
    let (_, zone, root) = globals[0];
    let streams = &catalog.zones[zone].1.streams;
    let groups = Address::decode(streams.u32(root.add(8)?)?)?;
    let group_count = streams.u32(root.add(4)?)? as usize;
    let curves = Address::decode(streams.u32(root.add(16)?)?)?;
    let curve_count = streams.u32(root.add(12)?)? as usize;
    let curve = |index: usize| -> Result<(String, Vec<[f32; 2]>)> {
        if index >= curve_count {
            return Err(format!("Native sound falloff curve {index} is out of range").into());
        }
        let curve = curves.add(index * 100)?;
        let name = streams.text(curve)?;
        let points = (0..8)
            .map(|point| {
                Ok([
                    f32::from_bits(streams.u32(curve.add(36 + point * 8)?)?),
                    f32::from_bits(streams.u32(curve.add(40 + point * 8)?)?),
                ])
            })
            .collect::<Result<Vec<_>>>()?;
        Ok((name, points))
    };
    let mut sounds = Vec::new();
    let mut aliases = serde_json::Map::new();
    for row in &rows {
        let name = row["name"]
            .as_str()
            .ok_or("Native sound alias name is missing")?;
        let index = row["index"].as_u64().unwrap();
        let parameters: Vec<u8> = serde_json::from_value(row["parameters"].clone())?;
        let raw = |offset: usize| u16_at(&parameters, offset - 28);
        let flags = row["flags"].as_u64().unwrap() as u32;
        let flags1 = row["flags1"].as_u64().unwrap() as u32;
        let group_index = (flags >> 17 & 31) as usize;
        if group_index >= group_count {
            return Err(
                format!("Native alias {name} volume group {group_index} is invalid").into(),
            );
        }
        let mut attenuation = 1_f32;
        let mut parent = group_index;
        let mut seen = BTreeSet::new();
        while parent != u32::MAX as usize {
            if parent >= group_count || !seen.insert(parent) {
                return Err(
                    format!("Native alias {name} has invalid volume-group ancestry").into(),
                );
            }
            let group = groups.add(parent * 80)?;
            attenuation *= streams.u16(group.add(78)?)? as f32 * 1.5259022e-05;
            parent = streams.u32(group.add(68)?)? as usize;
        }
        let category = streams.u32(groups.add(group_index * 80 + 72)?)?;
        let channel = match category {
            0 | 3 => json!("effects"),
            1 => json!("dialogue"),
            2 => json!("music"),
            _ => {
                gaps.push(format!("Initial sound {name} native menu category {category} requires its channel consumer"));
                Value::Null
            }
        };
        let mut unsupported = Vec::<String>::new();
        if row["asset_id"] == 0 {
            unsupported.push("silent variant with secondary/subtitle semantics".into());
        }
        if raw(60) != raw(62) {
            unsupported.push(format!("random volume range {}/{}", raw(60), raw(62)));
        }
        if flags >> 29 != 0 {
            unsupported.push(format!("entity randomization flags {}", flags >> 29));
        }
        if row["context_type"] != 0 || row["context_value"] != 0 {
            unsupported.push(format!(
                "context {}/{}",
                row["context_type"], row["context_value"]
            ));
        }
        if row["start_delay"] != 0 {
            unsupported.push(format!("start delay {}ms", row["start_delay"]));
        }
        if row["probability"] != 0 && row["probability"] != 255 {
            unsupported.push(format!("probability {}", row["probability"]));
        }
        if parameters[90 - 28] != parameters[91 - 28] {
            unsupported.push("distance-dependent priority".into());
        }
        if parameters[93 - 28] == 0 {
            unsupported.push("unbounded native alias voice limit".into());
        }
        if parameters[94 - 28] != 0 {
            unsupported.push(format!("entity voice limit {}", parameters[94 - 28]));
        }
        let policy = if channel == "dialogue"
            && ["vox_plr_", "vox_sam_", "vox_maxi_"]
                .iter()
                .any(|prefix| name.starts_with(prefix))
        {
            2
        } else {
            1
        };
        if flags >> 25 & 3 != policy {
            unsupported.push(format!("voice limit policy {}", flags >> 25 & 3));
        }
        if raw(70) == 0 {
            unsupported
                .push("native zero maximum distance requires nonspatial sound validation".into());
        }
        for (offset, label) in [(32, "duck"), (44, "stop-on-play"), (48, "futz patch")] {
            if u32_at(&parameters, offset - 28) != 0 {
                unsupported.push(format!("{label} {}", u32_at(&parameters, offset - 28)));
            }
        }
        if flags >> 22 & 7 != 0 || raw(52) != 0 {
            unsupported.push("flux modulation".into());
        }
        if flags & (1 << 3) != 0 {
            unsupported.push("Doppler pitch".into());
        }
        if raw(80) != 0 || raw(82) != 0 {
            unsupported.push("native fade-in/fade-out".into());
        }
        if flags & 2 != 0 {
            for (curve_index, minimum, label) in [
                (flags1 >> 2 & 63, false, "volume"),
                (flags1 >> 14 & 63, true, "minimum volume"),
            ] {
                let (name, points) = curve(curve_index as usize)?;
                if points.iter().any(|[x, y]| {
                    !x.is_finite()
                        || !y.is_finite()
                        || (*y - if minimum { 1. } else { 1. - *x }).abs() > 0.00001
                }) {
                    unsupported.push(format!("{label} falloff curve {name} ({curve_index})"));
                }
            }
        }
        if raw(56) != 0 {
            unsupported.push(format!(
                "native room reverb send {} and falloff curves {}/{}",
                raw(56),
                flags1 >> 8 & 63,
                flags1 >> 20 & 63
            ));
        }
        if flags1 & 1 != 0 {
            unsupported.push("never-play-twice variant rule".into());
        }
        if !unsupported.is_empty() {
            gaps.push(format!(
                "Initial sound {name}/{index} requires {}",
                unsupported.join(", ")
            ));
        }
        let id = format!("sound.native.{name}.{index}");
        bindings.insert(id.clone(), json!(name));
        sounds.push(json!({"id":id,"source_alias":name,"source_index":index,"path":null,
            "volume":if raw(60)==raw(62) {json!(raw(60) as f32*1.5259022e-05*attenuation)}else{Value::Null},
            "pitch":[raw(64) as f32*3.051851e-05,raw(66) as f32*3.051851e-05],
            "distance_min":raw(68) as f32*2.*0.0254,"distance":raw(70) as f32*2.*0.0254,"looping":flags&1!=0}));
        let alias = aliases.entry(name.to_owned()).or_insert_with(||json!({"sounds":[],"spatial":flags&2!=0,"channel":channel,
            "subtitle":row["subtitle"],"limit":parameters[93-28],"priority":parameters[90-28],"seconds":null,
            "secondary":row["secondary"].as_str().filter(|name|!name.is_empty()),"reverb_send":raw(56) as f32*1.5259022e-05*100.,"wet_distance_min":raw(68) as f32*2.*0.0254,"wet_distance_max":raw(72) as f32*2.*0.0254}));
        for (field, expected) in [
            ("spatial", json!(flags & 2 != 0)),
            ("channel", json!(channel)),
            ("subtitle", row["subtitle"].clone()),
            ("limit", json!(parameters[93 - 28])),
            ("priority", json!(parameters[90 - 28])),
            (
                "secondary",
                json!(row["secondary"].as_str().filter(|name| !name.is_empty())),
            ),
        ] {
            if alias[field] != expected {
                gaps.push(format!("Initial sound alias {name} has variant-specific {field} requiring a runtime consumer"));
            }
        }
        alias["sounds"].as_array_mut().unwrap().push(json!(id));
    }
    for (name, alias) in &aliases {
        if alias["sounds"].as_array().unwrap().len() > 2 {
            gaps.push(format!("Initial sound alias {name} requires native random variant selection and source previous-variant handling"));
        }
    }
    game["content"]["sounds"] = json!(sounds);
    game["audio"]["aliases"] = json!(aliases);
    game["audio"]["bindings"] = json!(bindings);
    game["audio"]["timelines"] = json!(timelines);
    println!(
        "Converted {} initial sound aliases, {} native variants and {} weapon sound timelines",
        game["audio"]["aliases"].as_object().unwrap().len(),
        rows.len(),
        game["audio"]["timelines"].as_object().unwrap().len()
    );
    Ok(())
}

pub fn bind(game: &mut Value, converted: &[Value]) -> Result<()> {
    let mut durations = BTreeMap::<String, Vec<f64>>::new();
    for sound in game["content"]["sounds"]
        .as_array_mut()
        .ok_or("Required initial sounds are absent")?
    {
        let name = sound["source_alias"]
            .as_str()
            .ok_or("Native sound source alias is absent")?
            .to_owned();
        let index = sound["source_index"].clone();
        let row = converted
            .iter()
            .find(|row| {
                row["name"]
                    .as_str()
                    .is_some_and(|alias| alias.eq_ignore_ascii_case(&name))
                    && row["index"] == index
            })
            .ok_or_else(|| {
                format!("Required initial sound payload {name}/{index} was not converted")
            })?;
        let path = row["uri"].as_str().ok_or_else(|| {
            format!("Required initial sound {name}/{index} has no converted payload")
        })?;
        let seconds = row["seconds"]
            .as_f64()
            .filter(|seconds| seconds.is_finite() && *seconds > 0.)
            .ok_or("Converted native sound duration is invalid")?;
        sound["path"] = json!(path);
        sound.as_object_mut().unwrap().remove("source_alias");
        sound.as_object_mut().unwrap().remove("source_index");
        durations.entry(name).or_default().push(seconds);
    }
    for (name, seconds) in durations {
        if seconds.iter().any(|duration| *duration != seconds[0]) {
            return Err(format!("Initial native sound alias {name} requires variant-specific playback duration consumption").into());
        }
        game["audio"]["aliases"][name]["seconds"] = json!(seconds[0]);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[ignore = "requires the authorized Steam installation"]
    fn installed_initial_sound_metadata() {
        let zones = crate::source::read_zones(&crate::source::installation().unwrap()).unwrap();
        let catalog = Catalog::new(&zones);
        let entities = crate::entities::read(&catalog).unwrap();
        let (game, gaps) = super::super::read(&catalog, &entities).unwrap();
        assert!(!game["content"]["sounds"].as_array().unwrap().is_empty());
        for gap in gaps.iter().filter(|gap| gap.starts_with("Initial sound")) {
            println!("{gap}");
        }
    }
}

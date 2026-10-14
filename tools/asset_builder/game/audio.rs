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
    let alias_ids = crate::sound::alias_hashes(catalog)?;
    let available: BTreeSet<_> = alias_ids
        .values()
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
    let (zone, table) = catalog.resolve(57, "default_ai")?;
    let streams = &catalog.zones[zone].1.streams;
    let footsteps = (0..32)
        .map(|surface| {
            (0..7)
                .map(|movement| {
                    let id = streams.u32(table.add(4 + (surface * 7 + movement) * 4)?)?;
                    let alias = alias_ids.get(&id).map(|alias| alias.to_ascii_lowercase());
                    if let Some(alias) = &alias {
                        required.insert(alias.clone());
                    }
                    Ok(alias)
                })
                .collect::<Result<Vec<_>>>()
        })
        .collect::<Result<Vec<_>>>()?;
    game["audio"]["footsteps"] = json!(footsteps);
    game["audio"]["water"] = crate::collision::water(catalog)?;
    let room_contexts = [
        ("tomb_outdoor", 3434048311_u32),
        ("tomb_tunnel", 1609388568),
        ("tomb_cave", 1609388568),
        ("tomb_spiral_staircase_bot", 1609388568),
        ("tomb_robot_head", 1609388568),
        ("tomb_large_chamber", 1609388568),
        ("tomb_bunker_sml", 1609388568),
        ("tomb_bunker_sml_prtl", 1609388568),
        ("tomb_bunker_med", 1609388568),
        ("tomb_bunker_med_prtl", 1609388568),
        ("tomb_bunker_lrg", 1609388568),
        ("tomb_bunker_lrg_prtl", 1609388568),
        ("tomb_overhang", 1609388568),
        ("tomb_overhang_prtl", 3434048311),
        ("tomb_spiral_staircase", 1609388568),
        ("tomb_church", 1609388568),
        ("tomb_church_prtl", 3434048311),
    ]
    .into_iter()
    .map(|(room, value)| (room.to_owned(), json!({"2155298321":value})))
    .collect::<serde_json::Map<_, _>>();
    game["audio"]["room_contexts"] = json!(room_contexts);
    game["audio"]["contexts"] = json!({"822242733":2546536281_u32});
    let room_names = [
        ("tomb_outdoor", "tomb_outdoor"),
        ("tomb_tunnel", "tomb_tunnel"),
        ("tomb_cave", "tomb_cave"),
        ("tomb_spiral_staircase_bot", "tomb_stairs_big"),
        ("tomb_robot_head", "tomb_bot_head"),
        ("tomb_large_chamber", "tomb_cave_2"),
        ("tomb_bunker_sml", "tomb_bunker_smallroom"),
        ("tomb_bunker_sml_prtl", "tomb_bunker_smallroom"),
        ("tomb_bunker_med", "tomb_bunker_mediumroom"),
        ("tomb_bunker_med_prtl", "tomb_bunker_mediumroom"),
        ("tomb_bunker_lrg", "tomb_bunker_largeroom"),
        ("tomb_bunker_lrg_prtl", "tomb_bunker_largeroom"),
        ("tomb_overhang", "tomb_overhang"),
        ("tomb_overhang_prtl", "tomb_overhang"),
        ("tomb_spiral_staircase", "tomb_stairs"),
        ("tomb_church", "tomb_church"),
        ("tomb_church_prtl", "tomb_church"),
    ];
    let profiles = crate::sound::rooms(
        catalog,
        &room_names
            .iter()
            .map(|(_, name)| (*name).to_owned())
            .collect(),
    )?;
    game["audio"]["rooms"] = json!(
        room_names
            .into_iter()
            .map(|(room, name)| (room.to_owned(), profiles[name].clone()))
            .collect::<BTreeMap<_, _>>()
    );
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
                    let event = note["event"].as_str()?;
                    event
                        .strip_prefix("sndnt#")
                        .filter(|alias| !alias.is_empty())
                        .or_else(|| notes.get(event).map(String::as_str))
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
    for animation in game["content"]["animations"].as_array().unwrap() {
        let id = animation["id"]
            .as_str()
            .ok_or("Selected sound animation lacks its ID")?;
        for note in animation["notifications"].as_array().into_iter().flatten() {
            let Some(alias) = note["event"]
                .as_str()
                .and_then(|event| event.strip_prefix("sndnt#"))
                .filter(|alias| !alias.is_empty())
            else {
                continue;
            };
            let alias = alias.to_ascii_lowercase();
            required.insert(alias.clone());
            let sound = json!({"seconds":note["seconds"],"alias":alias});
            let timeline = timelines
                .entry(id.to_owned())
                .or_insert_with(|| json!([]))
                .as_array_mut()
                .unwrap();
            if !timeline.contains(&sound) {
                timeline.push(sound);
            }
        }
    }
    for timeline in timelines.values_mut() {
        timeline.as_array_mut().unwrap().sort_by(|a, b| {
            a["seconds"]
                .as_f64()
                .unwrap()
                .total_cmp(&b["seconds"].as_f64().unwrap())
        });
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
        "zombie_head_gib",
        "zmb_death_gibs",
        "zmb_spawn_powerup",
        "zmb_spawn_powerup_loop",
        "zmb_powerup_grabbed",
        "zmb_footprintbox_glow_lp",
        "vox_zmba_powerup_doublepoints_0",
        "mpl_player_heartbeat",
        "revive_gasp",
        "chr_heart_beat",
        "evt_laststand_loop",
        "evt_laststand_in",
        "mus_perks_revive_sting",
        "mus_perks_revive_jingle",
        "zmb_perks_bump_bottle",
        "zmb_perks_power_on",
        "vox_maxi_generator_initiate_0",
        "vox_maxi_generator_process_complete_0",
        "vox_maxi_generator_1_activated_0",
        "mus_event_generator_1",
        "zmb_capturezone_donut_start",
        "zmb_capturezone_rise",
        "zmb_capturezone_donut_stop",
        "zmb_capturezone_success",
        "zmb_capturezone_portal_start",
        "zmb_capturezone_portal_stop",
        "zmb_capturezone_portal_loop",
        "zmb_capturezone_vat_loop",
        "zmb_capturezone_zombie_loop",
        "zmb_zone_plate_down",
        "zmb_zone_plate_up",
        "zmb_zone_plate_loop",
        "zmb_vocals_capzomb_spawn",
        "zmb_vocals_capzomb_attack",
        "zmb_vocals_capzomb_ambience",
        "zmb_vocals_capzomb_sprint",
        "zmb_vocals_capzomb_death",
        "zmb_vocals_capzomb_ambience_loud",
        "zmb_vocals_capzomb_sprint_loud",
    ] {
        required.insert(alias.into());
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
                    "capture_zombies",
                    "powerup_doublepoints",
                ]
                .iter()
                .any(|event| alias.starts_with(&format!("vox_plr_{character}_{event}_")))
            })
        {
            required.insert(alias.clone());
        }
        if alias.starts_with("zmb_vocals_zombie_")
            && ["ambience_loud", "sprint_loud", "crawler", "crawler_loud"]
                .iter()
                .any(|event| alias == &format!("zmb_vocals_zombie_{event}"))
        {
            required.insert(alias.clone());
        }
    }
    let mut absent = BTreeSet::new();
    let rows = crate::sound::definitions(catalog, &required, &mut absent)?;
    let duck_ids = rows
        .iter()
        .filter_map(|row| {
            let bytes = serde_json::from_value::<Vec<u8>>(row["parameters"].clone()).ok()?;
            let id = u32_at(&bytes, 4);
            (id != 0).then_some(id)
        })
        .collect::<BTreeSet<_>>();
    let ducks = crate::sound::ducks(catalog, &duck_ids)?;
    for (id, profile) in &ducks {
        let filters: Vec<f32> = serde_json::from_value(profile["filter"].clone())?;
        let attenuation: Vec<f32> = serde_json::from_value(profile["attenuation"].clone())?;
        println!(
            "Initial native duck {id} {}: fades {}, delay {}, length {}, distance {}, attenuation {:?}, filter {:?}",
            profile["name"],
            profile["fades"],
            profile["delay"],
            profile["length"],
            profile["distance"],
            attenuation,
            filters
        );
    }
    game["audio"]["ducks"] = json!(ducks);
    bindings.retain(|_, name| {
        name.as_str()
            .is_some_and(|name| !absent.contains(&name.to_ascii_lowercase()))
    });
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
    let pans = Address::decode(streams.u32(root.add(24)?)?)?;
    let pan_count = streams.u32(root.add(20)?)? as usize;
    let mut default_pan = None;
    for index in 0..pan_count {
        if streams.text(pans.add(index * 60)?)? == "default" {
            default_pan = Some(index);
        }
    }
    let default_pan = default_pan.ok_or("Native default pan is absent")?;
    let masters = Address::decode(streams.u32(root.add(48)?)?)?;
    let master_count = streams.u32(root.add(44)?)? as usize;
    let mut master = None;
    for index in 0..master_count {
        let entry = masters.add(index * 240)?;
        let name = streams.text(entry)?;
        if !matches!(name.as_str(), "default" | "treyarch_mix") {
            continue;
        }
        let scalar =
            |offset| -> Result<f32> { Ok(f32::from_bits(streams.u32(entry.add(offset)?)?)) };
        println!(
            "Initial native master {name}: {:?}",
            (36..240)
                .step_by(4)
                .map(|offset| streams.u32(entry.add(offset)?))
                .collect::<std::result::Result<Vec<_>, _>>()?
        );
        if name != "default" {
            continue;
        }
        let mut buses = Vec::new();
        for bus in 0..8 {
            buses.push(if streams.u32(entry.add(196 + bus * 4)?)? != 0 {
                scalar(160 + bus * 4)?
            } else {
                0.
            });
        }
        buses.push(1.);
        master = Some(json!({"buses":buses,"gain":scalar(100)?}));
        for (label, enabled, gain) in [
            ("low shelf", 36, 40),
            ("peak1", 52, 56),
            ("peak2", 68, 72),
            ("high shelf", 84, 88),
        ] {
            if streams.u32(entry.add(enabled)?)? != 0 && scalar(gain)?.abs() > 1.52879e-05 {
                gaps.push(format!("Initial native default master {label} gain {} requires its source biquad consumer006f6010",scalar(gain)?));
            }
        }
        for (label, enabled, offset) in [("compressor", 104, 108), ("limiter", 132, 136)] {
            if streams.u32(entry.add(enabled)?)? != 0 {
                let parameters = (0..6)
                    .map(|index| scalar(offset + index * 4))
                    .collect::<Result<Vec<_>>>()?;
                gaps.push(format!("Initial native default master {label} parameters {parameters:?} require source consumer00835603"));
            }
        }
    }
    game["audio"]["master"] = master.ok_or("Native default master fallback is absent")?;
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
    let mut unsupported_consumers = BTreeMap::<String, BTreeSet<String>>::new();
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
        let pan_index = parameters[92 - 28] as usize;
        if pan_index >= pan_count {
            return Err(format!("Initial sound {name} pan {pan_index} is out of range").into());
        }
        let pan = pans.add(pan_index * 60)?;
        let pan_scalar =
            |offset| -> Result<f32> { Ok(f32::from_bits(streams.u32(pan.add(offset)?)?)) };
        let pan_sum = pan_scalar(36)? + pan_scalar(40)?;
        let pan_coefficients = [
            [
                pan_scalar(52)? * pan_sum.clamp(0., 1.),
                pan_scalar(56)? * pan_sum.clamp(0., 1.),
            ],
            [
                pan_scalar(52)? * pan_sum * 0.5,
                pan_scalar(56)? * pan_sum * 0.5,
            ],
        ];
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
        if row["context_type"] != 0
            && row["context_type"] != 2155298321_u32
            && row["context_type"] != 822242733_u32
        {
            unsupported.push(format!(
                "initial context setter {}/{}",
                row["context_type"], row["context_value"]
            ));
        }
        for (offset, label) in [(44, "stop-on-play"), (48, "futz patch")] {
            if u32_at(&parameters, offset - 28) != 0 {
                unsupported.push(format!("{label} {}", u32_at(&parameters, offset - 28)));
            }
        }
        if flags & 2 != 0 && flags >> 22 & 7 != 0 && !matches!(flags >> 22 & 7, 1 | 3) {
            unsupported.push(format!(
                "native sound motion type {} with fluxTime {}ms",
                flags >> 22 & 7,
                raw(52)
            ));
        }
        if flags & (1 << 3) != 0 {
            unsupported.push("Doppler pitch".into());
        }
        if flags & 2 != 0 && parameters[89 - 28] != 0 {
            unsupported.push(format!(
                "native obstruction blend {} in consumers00829650/00838790",
                parameters[89 - 28]
            ));
        }
        for reason in unsupported {
            unsupported_consumers
                .entry(reason)
                .or_default()
                .insert(name.to_owned());
        }
        let id = format!("sound.native.{name}.{index}");
        let sound = (row["asset_id"] != 0).then_some(id.clone());
        if sound.is_some() {
            bindings.insert(id.clone(), json!(name));
            sounds.push(json!({"id":id,"source_alias":name,"source_index":index,"path":null,
                "volume":raw(60) as f32*1.5259022e-05*attenuation,
                "pitch":[raw(64) as f32*3.051851e-05,raw(66) as f32*3.051851e-05],
                "distance_min":raw(68) as f32*2.*0.0254,"distance":raw(70) as f32*2.*0.0254,"looping":flags&1!=0}));
        }
        let secondary = row["secondary"]
            .as_str()
            .filter(|name| !name.is_empty() && !absent.contains(&name.to_ascii_lowercase()));
        let alias=aliases.entry(name.to_owned()).or_insert_with(||json!({"sounds":[],"spatial":flags&2!=0,"channel":channel,
            "subtitle":row["subtitle"],"limit":parameters[93-28],"priority":parameters[90-28],"seconds":null,
            "secondary":secondary,"reverb_send":raw(56) as f32*1.5259022e-05*100.,"wet_distance_min":raw(68) as f32*2.*0.0254,
            "wet_distance_max":raw(72) as f32*2.*0.0254,"native":[]}));
        if sound.is_some() {
            alias["sounds"].as_array_mut().unwrap().push(json!(id));
        }
        alias["native"].as_array_mut().unwrap().push(json!({"id":row["alias_id"],"source_index":index,"sound":sound,"seconds":null,
            "volume_max":raw(62) as f32*1.5259022e-05*attenuation,"spatial":flags&2!=0,"channel":channel,"subtitle":row["subtitle"],"secondary":secondary,
            "context":[row["context_type"],row["context_value"]],"probability":row["probability"],"randomize":flags>>29,
            "never_play_twice":flags1&1!=0,"ignore_max_distance":flags&(1<<10)!=0,"stop_on_death":flags&(1<<7)!=0,"loaded":flags>>15&3==1,
            "voice_limit":flags&(1<<9)!=0,"announcer":streams.text(groups.add(group_index*80)?)?=="grp_announcer","pauseable":flags&(1<<5)!=0,
            "motion":flags>>22&7,"motion_seconds":raw(52) as f32*0.001,
            "bus":flags>>11&15,"distance_low_pass":flags&(1<<2)!=0,"large":flags&(1<<4)!=0,
            "panning":flags&2!=0 && pan_index==default_pan,"pan":pan_coefficients,
            "duck":ducks.contains_key(&u32_at(&parameters,4)).then_some(u32_at(&parameters,4)),"duck_group":parameters[95-28],
            "priority":[parameters[90-28],parameters[91-28]],"priority_threshold":[parameters[86-28] as f32*0.003921569,parameters[87-28] as f32*0.003921569],
            "limit":{"count":parameters[93-28],"policy":flags>>25&3},"entity_limit":{"count":parameters[94-28],"policy":flags>>27&3},
            "delay":raw(54) as f32*0.001,"fades":[raw(80) as i16 as f32*0.001,raw(82) as i16 as f32*0.001],
            "curves":[curve((flags1>>2&63) as usize)?.1,curve((flags1>>14&63) as usize)?.1],
            "reverb_send":raw(56) as f32*1.5259022e-05*100.,"wet_distance":[raw(68) as f32*2.*0.0254,raw(72) as f32*2.*0.0254],
            "reverb_curves":[curve((flags1>>8&63) as usize)?.1,curve((flags1>>20&63) as usize)?.1]}));
    }
    for alias in aliases.values_mut() {
        if alias["sounds"].as_array().unwrap().is_empty() {
            alias["seconds"] = json!(0.);
        }
    }
    for (consumer, aliases) in unsupported_consumers {
        gaps.push(format!(
            "Initial sound requires {consumer} for {} selected aliases, including {}",
            aliases.len(),
            aliases
                .iter()
                .take(4)
                .cloned()
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    game["content"]["sounds"] = json!(sounds);
    game["audio"]["aliases"] = json!(aliases);
    game["audio"]["bindings"] = json!(bindings);
    game["audio"]["timelines"] = json!(timelines);
    game["audio"]["absent"] = json!(absent);
    println!(
        "Converted {} initial sound aliases, {} native variants and {} animation sound timelines",
        game["audio"]["aliases"].as_object().unwrap().len(),
        rows.len(),
        game["audio"]["timelines"].as_object().unwrap().len()
    );
    Ok(())
}

pub fn bind(game: &mut Value, converted: &[Value]) -> Result<()> {
    let mut durations = BTreeMap::<(String, u64), f64>::new();
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
        durations.insert(
            (
                name,
                index
                    .as_u64()
                    .ok_or("Native sound variant index is invalid")?,
            ),
            seconds,
        );
    }
    for (name, alias) in game["audio"]["aliases"].as_object_mut().unwrap() {
        let mut first = None;
        for native in alias["native"].as_array_mut().unwrap() {
            let index = native
                .as_object_mut()
                .unwrap()
                .remove("source_index")
                .and_then(|index| index.as_u64())
                .ok_or("Native sound variant index is absent")?;
            if !native["sound"].is_null() {
                let seconds = *durations
                    .get(&(name.clone(), index))
                    .ok_or("Native variant has no converted duration")?;
                native["seconds"] = json!(seconds);
                first.get_or_insert(seconds);
            }
        }
        alias["seconds"] = json!(first.unwrap_or(0.));
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

    #[test]
    #[ignore = "requires the authorized Steam installation"]
    fn installed_initial_footstep_bindings() {
        let zones = crate::source::read_zones(&crate::source::installation().unwrap()).unwrap();
        let catalog = Catalog::new(&zones);
        for name in [
            "aitype/clientscripts/zm_tomb_basic_german.csc",
            "aitype/clientscripts/zm_tomb_basic_german2.csc",
        ] {
            let (zone, root) = catalog.resolve(48, name).unwrap();
            let streams = &catalog.zones[zone].1.streams;
            let length = streams.u32(root.add(4).unwrap()).unwrap() as usize;
            let data = Address::decode(streams.u32(root.add(8).unwrap()).unwrap()).unwrap();
            let data = streams.read(data, length).unwrap();
            assert_eq!(u32_at(data, 0), 0x43534780);
            assert_eq!(u32_at(data, 4), 0x06000a0d);
            let string = |offset: usize| {
                let bytes = &data[offset..];
                std::str::from_utf8(&bytes[..bytes.iter().position(|byte| *byte == 0).unwrap()])
                    .unwrap()
                    .to_owned()
            };
            let mut strings = BTreeMap::new();
            let mut offset = u32_at(data, 0x18) as usize;
            for _ in 0..u16_at(data, 0x32) {
                let value = string(u16_at(data, offset) as usize);
                let count = data[offset + 2] as usize;
                for index in 0..count {
                    strings.insert(u32_at(data, offset + 4 + index * 4) as usize, value.clone());
                }
                offset += 4 + count * 4;
            }
            let includes = u32_at(data, 0xc) as usize;
            for index in 0..data[0x3c] as usize {
                println!(
                    "Native initial actor script {name} includes {}",
                    string(u32_at(data, includes + index * 4) as usize)
                );
            }
            let mut offset = u32_at(data, 0x20) as usize;
            for _ in 0..u16_at(data, 0x36) {
                let function = string(u16_at(data, offset) as usize);
                let namespace = string(u16_at(data, offset + 2) as usize);
                let count = u16_at(data, offset + 4) as usize;
                println!(
                    "Native initial actor script {name} imports {namespace}::{function} with {} arguments and {count} calls",
                    data[offset + 6]
                );
                if function.contains("footstep") {
                    for index in 0..count {
                        let call = u32_at(data, offset + 8 + index * 4) as usize;
                        let arguments = strings
                            .range(call.saturating_sub(32)..call)
                            .map(|(reference, value)| {
                                (call - *reference, data[reference - 1], value)
                            })
                            .collect::<Vec<_>>();
                        println!(
                            "Native initial actor footstep call {name} {function} at {call:#x}: preceding string references {arguments:?}"
                        );
                    }
                }
                offset += 8 + count * 4;
            }
        }
    }
}

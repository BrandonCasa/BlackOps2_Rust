use super::*;

pub fn check_triggers(catalog: &Catalog<'_>, entities: &[crate::entities::Entity]) -> Result<()> {
    let mut scripts = 0;
    let mut staff_references = Vec::new();
    let mut selected_references = std::collections::BTreeMap::<String, Vec<String>>::new();
    for (name, zone, root) in catalog.definitions(48) {
        let streams = &catalog.zones[zone].1.streams;
        let size = streams.u32(root.add(4)?)? as usize;
        let bytes = streams.read(Address::decode(streams.u32(root.add(8)?)?)?, size)?;
        scripts += 1;
        for literal in [
            "staff_craftable_trigger",
            "staff_air_craftable_trigger",
            "staff_fire_craftable_trigger",
            "staff_lightning_craftable_trigger",
            "staff_water_craftable_trigger",
            "craftable_trigger_think",
            "setup_unitrigger_craftable",
            "setup_unitrigger_craftable_array",
            "craftable_trigger_think_array",
            "vehicle_craftable_trigger_think",
            "_craftable_trigger",
            "staff_",
        ] {
            if bytes
                .windows(literal.len() + 1)
                .enumerate()
                .any(|(offset, window)| {
                    (offset == 0 || bytes[offset - 1] == 0)
                        && window[..literal.len()] == *literal.as_bytes()
                        && window[literal.len()] == 0
                })
            {
                selected_references
                    .entry(literal.into())
                    .or_default()
                    .push(format!("{}/{name}", catalog.zones[zone].0));
                if literal == "staff_craftable_trigger" {
                    staff_references.push(name.to_owned());
                }
            }
        }
        if name == "codescripts/delete.gsc" {
            if bytes.len() < 64 || u32_at(bytes, 0) != 0x43534780 || u32_at(bytes, 4) != 0x06000a0d
            {
                return Err("Installed native delete script is not supported T6 bytecode".into());
            }
            let text = |offset: usize| -> Result<&str> {
                let value = bytes
                    .get(offset..)
                    .ok_or("Native script string offset exceeds bytecode")?;
                let end = value
                    .iter()
                    .position(|byte| *byte == 0)
                    .ok_or("Native script string is unterminated")?;
                Ok(std::str::from_utf8(&value[..end])?)
            };
            let exports = u32_at(bytes, 28) as usize;
            if u16_at(bytes, 52) != 1 || exports + 12 > bytes.len() {
                return Err("Native delete script does not have its sole main export".into());
            }
            let main = u32_at(bytes, exports + 4) as usize;
            if text(u16_at(bytes, exports + 8) as usize)? != "main" || bytes[exports + 10] != 0 {
                return Err("Native delete script main export signature is unsupported".into());
            }
            for index in 0..u16_at(bytes, 52) as usize {
                let export = exports + index * 12;
                let address = u32_at(bytes, export + 4) as usize;
                println!(
                    "Native delete script selected export: name={} address={address} params={} flags={}",
                    text(u16_at(bytes, export + 8) as usize)?,
                    bytes[export + 10],
                    bytes[export + 11]
                );
            }
            let mut imports = u32_at(bytes, 32) as usize;
            let mut delete_call = None;
            for _ in 0..u16_at(bytes, 54) as usize {
                if imports + 8 > bytes.len() {
                    return Err("Truncated native delete script import".into());
                }
                let references = u16_at(bytes, imports + 4) as usize;
                if imports + 8 + references * 4 > bytes.len() {
                    return Err("Truncated native delete script import references".into());
                }
                let addresses: Vec<_> = (0..references)
                    .map(|index| u32_at(bytes, imports + 8 + index * 4))
                    .collect();
                if text(u16_at(bytes, imports) as usize)? == "delete" && bytes[imports + 6] == 0 {
                    if addresses.len() != 1 || delete_call.replace(addresses[0] as usize).is_some()
                    {
                        return Err("Native delete script method binding is ambiguous".into());
                    }
                }
                println!(
                    "Native delete script selected import: name={} namespace={} params={} flags={} references={}",
                    text(u16_at(bytes, imports) as usize)?,
                    text(u16_at(bytes, imports + 2) as usize)?,
                    bytes[imports + 6],
                    bytes[imports + 7],
                    json!(addresses)
                );
                imports += 8 + references * 4;
            }
            let code = u32_at(bytes, 20) as usize;
            let end = code
                .checked_add(u32_at(bytes, 44) as usize)
                .filter(|end| *end <= bytes.len())
                .ok_or("Native delete script code range exceeds its buffer")?;
            if !(code..end).contains(&main) {
                return Err("Native delete script main entry is outside bytecode".into());
            }
            let waits: Vec<_> = bytes[code..end]
                .windows(2)
                .enumerate()
                .filter(|(_, window)| *window == [3, 0x2b])
                .map(|(offset, _)| code + offset)
                .collect();
            let defined: Vec<_> = bytes[code..end]
                .windows(2)
                .enumerate()
                .filter(|(_, window)| *window == [0xf, 0x5f])
                .map(|(offset, _)| code + offset)
                .collect();
            println!(
                "Native delete script selected semantics: wait_zero_offsets={} self_isdefined_offsets={}",
                json!(waits),
                json!(defined)
            );
            let wait = bytes[main..end]
                .windows(4)
                .position(|window| window == [3, 0x2b, 0xf, 0x5f])
                .map(|offset| main + offset)
                .ok_or(
                    "Native delete main does not perform wait zero followed by a live-self check",
                )?;
            let delete = delete_call
                .filter(|address| (wait + 4..end).contains(address))
                .ok_or(
                    "Native delete main has no subsequent zero-argument delete method binding",
                )?;
            println!(
                "Native delete main semantic scalar check: entry={main} wait_zero={wait} live_self={} branch_opcode={} delete_import={delete} delete_opcode={} operand_byte={} code_end={end}",
                wait + 2,
                bytes[wait + 4],
                bytes[delete],
                bytes.get(delete + 1).copied().unwrap_or(0)
            );
        }
    }
    println!(
        "Native active script binding references across {scripts} ScriptParseTree definitions: {}",
        json!(selected_references)
    );
    println!(
        "Native staff template exact literal references: {}",
        staff_references.len()
    );
    let (mut game, mut missing) = read(catalog, entities)?;
    let mut pack = Pack::default();
    let collision = crate::collision::convert(catalog, &mut pack)?;
    let world = json!({"brush_entities":crate::scenes::brush_entities(entities)?});
    let (rows, classifiers) = brushes(&world, &collision)?;
    missing.extend(classifiers);
    environment(&mut game, rows, &mut missing)?;
    link_owners(&mut game, entities)?;
    zone_graph(&mut game, entities, &mut missing)?;
    let mut inventory = std::collections::BTreeMap::<(String, String), usize>::new();
    let mut ids = std::collections::BTreeSet::new();
    let mut convex_count = 0;
    let mut triangle_count = 0;
    for trigger in game["environment"]["triggers"]
        .as_array()
        .into_iter()
        .flatten()
    {
        let source = &trigger["source"];
        let id = trigger["id"]
            .as_str()
            .ok_or("Converted trigger ID is missing")?;
        if !ids.insert(id) {
            return Err(format!("Duplicate converted trigger {id}").into());
        }
        let position: [f64; 3] = serde_json::from_value(trigger["position"].clone())?;
        if !position.into_iter().all(f64::is_finite) {
            return Err(format!("Converted trigger {id} has a nonfinite origin").into());
        }
        #[cfg(feature = "game")]
        {
            let runtime: origins::environment::triggers::TriggerDefinition =
                serde_json::from_value(trigger.clone()).map_err(|error| {
                    format!("Converted trigger {id} does not match the runtime schema: {error}")
                })?;
            if runtime.collider().is_none() {
                return Err(format!(
                    "Converted trigger {id} cannot form its runtime Avian collider"
                )
                .into());
            }
            let pose = runtime.transform();
            if !pose.translation.is_finite()
                || !pose.rotation.is_finite()
                || !pose.scale.is_finite()
            {
                return Err(
                    format!("Converted trigger {id} has a nonfinite runtime transform").into(),
                );
            }
        }
        let owners: Vec<_> = game["environment"]["ownership"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|owner| owner["id"] == id)
            .collect();
        if owners.len() != 1 {
            return Err(format!("Converted trigger {id} resolves {} owners", owners.len()).into());
        }
        if owners[0]["owner"] == "deferred" {
            return Err(format!("Native trigger {id} has no recovered source owner").into());
        }
        let shape = &trigger["shape"];
        if shape["kind"] == "cylinder" {
            for dimension in ["radius", "height"] {
                if !shape[dimension]
                    .as_f64()
                    .is_some_and(|value| value.is_finite() && value > 0.)
                {
                    return Err(format!("Converted trigger {id} has invalid {dimension}").into());
                }
            }
        } else {
            let volumes = shape["volumes"]
                .as_array()
                .ok_or_else(|| format!("Converted trigger {id} has no convex geometry index"))?;
            let triangles = shape["triangle_shapes"]
                .as_array()
                .map(Vec::as_slice)
                .unwrap_or_default();
            if volumes.is_empty() && triangles.is_empty() {
                return Err(format!("Converted trigger {id} has no geometry").into());
            }
            for volume in volumes {
                let vertices: Vec<[f64; 3]> = serde_json::from_value(volume["vertices"].clone())?;
                if !(4..=128).contains(&vertices.len())
                    || !vertices.iter().flatten().all(|value| value.is_finite())
                {
                    return Err(format!(
                        "Converted trigger {id} has an unsupported native convex volume {}",
                        volume["id"]
                    )
                    .into());
                }
                if shape["kind"] == "brush"
                    && (volume["contents"].as_u64().is_none()
                        || volume["surface_flags"].as_array().is_none())
                {
                    return Err(format!(
                        "Converted trigger {id} dropped native convex contents or surface flags"
                    )
                    .into());
                }
                convex_count += 1;
            }
            for triangle in triangles {
                let vertices: Vec<[f64; 3]> = serde_json::from_value(triangle["vertices"].clone())?;
                let indices: Vec<[usize; 3]> = serde_json::from_value(triangle["indices"].clone())?;
                if !vertices.iter().flatten().all(|value| value.is_finite())
                    || indices
                        .iter()
                        .flatten()
                        .any(|&index| index >= vertices.len())
                    || triangle["contents"].as_u64().is_none()
                    || triangle["surface_flags"].as_u64().is_none()
                {
                    return Err(format!(
                        "Converted trigger {id} has invalid native triangle geometry/flags"
                    )
                    .into());
                }
                triangle_count += indices.len();
            }
        }
        let key = (
            source["classname"].as_str().unwrap_or("").to_owned(),
            trigger["shape"]["kind"].as_str().unwrap_or("").to_owned(),
        );
        *inventory.entry(key).or_default() += 1;
        if matches!(
            source["targetname"].as_str(),
            Some("sndBG" | "sndCaptureZone" | "sndMaelstrom" | "pack_a_punch_intro_trigger")
        ) || source["script_noteworthy"] == "sndMusicDoor"
            || source["model"] == "*164"
            || matches!(
                source["targetname"].as_str(),
                Some(
                    "trig_use_tank"
                        | "vol_on_tank_watch"
                        | "staff_craftable_trigger"
                        | "zone_nml_3a"
                )
            )
        {
            let selected: std::collections::BTreeMap<_, _> = [
                "classname",
                "targetname",
                "origin",
                "angles",
                "angle",
                "model",
                "spawnflags",
                "script_noteworthy",
                "script_sound",
                "script_int",
                "target",
                "wait",
                "height",
                "width",
                "length",
                "link_owner_origin",
                "link_owner_angles",
            ]
            .into_iter()
            .filter_map(|key| source.get(key).map(|value| (key, value)))
            .collect();
            println!(
                "Trigger selected source {}: {}",
                trigger["id"],
                serde_json::to_string(&selected)?
            );
            if source["model"] == "*164" {
                println!(
                    "Trigger *164 geometry: position={} rotation={} convex={} triangle_shapes={} triangle_count={}",
                    trigger["position"],
                    trigger["shape"]["rotation"],
                    trigger["shape"]["volumes"].as_array().map_or(0, Vec::len),
                    trigger["shape"]["triangle_shapes"]
                        .as_array()
                        .map_or(0, Vec::len),
                    trigger["shape"]["triangle_shapes"]
                        .as_array()
                        .into_iter()
                        .flatten()
                        .map(|shape| shape["indices"].as_array().map_or(0, Vec::len))
                        .sum::<usize>()
                );
            }
            if source["targetname"] == "pack_a_punch_intro_trigger" {
                let target = source["target"]
                    .as_str()
                    .ok_or("Pack a Punch discovery trigger has no target")?;
                for (index, entity) in entities.iter().enumerate().filter(|(_, entity)| {
                    entity.get("targetname").is_some_and(|name| name == target)
                }) {
                    let selected: std::collections::BTreeMap<_, _> =
                        ["classname", "targetname", "origin", "angles"]
                            .into_iter()
                            .filter_map(|key| entity.get(key).map(|value| (key, value)))
                            .collect();
                    println!(
                        "Trigger target source {index}: {}",
                        serde_json::to_string(&selected)?
                    );
                }
            }
        }
        if matches!(
            source["targetname"].as_str(),
            Some("zombie_door" | "zombie_debris")
        ) {
            let selected: std::collections::BTreeMap<_, _> = source
                .as_object()
                .ok_or("Native door source fields are missing")?
                .iter()
                .filter(|(key, _)| {
                    matches!(
                        key.as_str(),
                        "classname"
                            | "targetname"
                            | "target"
                            | "model"
                            | "origin"
                            | "angles"
                            | "spawnflags"
                            | "zombie_cost"
                            | "script_noteworthy"
                            | "script_int"
                            | "script_string"
                            | "script_linkto"
                    ) || key.starts_with("script_flag")
                })
                .map(|(key, value)| (key, value))
                .collect();
            println!(
                "Door trigger selected source {id}: {}",
                serde_json::to_string(&selected)?
            );
            if let Some(flags) = source["script_flag"].as_str() {
                let matched: Vec<_> = flags.split(',').flat_map(|flag| game["access"]["flag_links"].as_array().into_iter().flatten()
                    .filter(move |link| link["flag"] == flag.trim()).map(|link| json!({"flag":link["flag"],"connections":link["connections"]}))).collect();
                println!("Door trigger source connections {id}: {}", json!(matched));
            }
        }
    }
    for entity in entities.iter().filter(|entity| {
        entity
            .get("targetname")
            .is_some_and(|target| target == "tank")
    }) {
        let selected: std::collections::BTreeMap<_, _> =
            ["classname", "targetname", "origin", "angles", "angle"]
                .into_iter()
                .filter_map(|key| entity.get(key).map(|value| (key, value)))
                .collect();
        println!(
            "Tank selected source: {}",
            serde_json::to_string(&selected)?
        );
    }
    let expected: std::collections::BTreeSet<_> = entities
        .iter()
        .enumerate()
        .filter(|(_, entity)| {
            entity
                .get("classname")
                .is_some_and(|class| class.starts_with("trigger_") || class == "info_volume")
        })
        .map(|(index, entity)| {
            format!(
                "{}.native.{index}",
                if entity
                    .get("model")
                    .is_some_and(|model| model.starts_with('*'))
                {
                    "brush"
                } else {
                    "trigger"
                }
            )
        })
        .collect();
    let converted: std::collections::BTreeSet<_> = ids.iter().map(|id| (*id).to_owned()).collect();
    if expected != converted {
        return Err(format!(
            "Native trigger geometry coverage is incomplete: missing {:?}, unexpected {:?}",
            expected.difference(&converted).collect::<Vec<_>>(),
            converted.difference(&expected).collect::<Vec<_>>()
        )
        .into());
    }
    for ((class, shape), count) in inventory {
        println!("Trigger inventory: {class}/{shape}: {count}");
    }
    println!(
        "Trigger source/geometry checks passed: {} trigger IDs, {convex_count} convex volumes, {triangle_count} native triangles; one source owner per trigger",
        ids.len()
    );
    #[cfg(feature = "game")]
    println!(
        "Installed native trigger runtime schemas and Avian colliders checked: {}",
        ids.len()
    );
    println!(
        "Converted native volume consumers: {} mud volumes, {} player-zone volumes",
        game["environment"]["mud"]["volumes"]
            .as_array()
            .map_or(0, Vec::len),
        game["access"]["volumes"].as_array().map_or(0, Vec::len)
    );
    println!(
        "Converted access trigger activations: {}",
        game["access"]["trigger_activations"]
    );
    let trigger_gaps: Vec<_> = missing
        .iter()
        .filter(|gap| {
            gap.starts_with("environment ")
                || gap.starts_with("movement consumer ")
                || gap.starts_with("boundary consumer ")
        })
        .collect();
    if !trigger_gaps.is_empty() {
        return Err(format!(
            "Native trigger source conversion remains incomplete: {trigger_gaps:?}"
        )
        .into());
    }
    println!("Phase 2 trigger source gaps: 0");
    for gap in missing {
        println!("Remaining conversion: {gap}");
    }
    Ok(())
}

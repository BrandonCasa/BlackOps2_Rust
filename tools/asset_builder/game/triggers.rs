use super::*;

pub(super) fn zone_graph(
    game: &mut Value,
    entities: &[crate::entities::Entity],
    missing: &mut Vec<String>,
) -> Result<()> {
    use std::collections::{BTreeMap, BTreeSet};
    let mut zone_gaps = Vec::new();
    let path = std::path::PathBuf::from(
        std::env::var_os("USERPROFILE").ok_or("USERPROFILE is unavailable")?,
    )
    .join("Documents/T6ZMResearch/t6zm-master/source/server-tomb/maps/mp/zm_tomb.gsc");
    let source = std::fs::read_to_string(path)?;
    if !source.contains("init_zones[0] = \"zone_start\";") {
        return Err("Original initial-zone source expression is unsupported".into());
    }
    let body = source
        .split_once("working_zone_init()")
        .ok_or("Original working_zone_init source is missing")?
        .1;
    let body = body
        .trim_start()
        .strip_prefix('{')
        .ok_or("Original working_zone_init body is missing")?;
    let body = body
        .split_once('}')
        .ok_or("Original working_zone_init body is unterminated")?
        .0;
    let literals = |line: &str, call: &str| -> Result<Vec<String>> {
        let args = line
            .strip_prefix(call)
            .ok_or("Unsupported zone source call")?
            .trim()
            .strip_prefix('(')
            .and_then(|args| args.strip_suffix(");"))
            .ok_or_else(|| format!("Unsupported literal zone call syntax: {line}"))?;
        args.split(',')
            .map(|arg| {
                let value = arg
                    .trim()
                    .strip_prefix('"')
                    .and_then(|arg| arg.strip_suffix('"'))
                    .ok_or_else(|| format!("Nonliteral zone call: {line}"))?;
                if value.is_empty()
                    || !value
                        .bytes()
                        .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
                {
                    return Err(format!("Unsupported zone source literal: {value}").into());
                }
                Ok(value.to_owned())
            })
            .collect()
    };
    let mut zones = BTreeSet::new();
    let mut edges = Vec::<(String, String, BTreeSet<String>)>::new();
    let mut edge_indices = BTreeMap::new();
    let mut activations = Vec::new();
    let mut calls = 0;
    for line in body.lines().map(str::trim).filter(|line| !line.is_empty()) {
        if line.starts_with("add_adjacent_zone") {
            let args = literals(line, "add_adjacent_zone")?;
            let [a, b, flag]: [String; 3] = args
                .try_into()
                .map_err(|_| "Original adjacent zone call requires three literals")?;
            zones.insert(a.clone());
            zones.insert(b.clone());
            calls += 1;
            let key = if a <= b {
                (a.clone(), b.clone())
            } else {
                (b.clone(), a.clone())
            };
            let index = *edge_indices.entry(key).or_insert_with(|| {
                edges.push((a, b, BTreeSet::new()));
                edges.len() - 1
            });
            edges[index].2.insert(flag);
        } else if line.starts_with("level thread activate_zone_trig") {
            let args = literals(line, "level thread activate_zone_trig")?;
            if !(2..=3).contains(&args.len()) {
                return Err("Original zone activation call requires two or three literals".into());
            }
            activations.push(args);
        } else if line == "flag_init( \"always_on\" );" || line == "flag_set( \"always_on\" );" {
        } else {
            return Err(format!("Unsupported working_zone_init statement: {line}").into());
        }
    }
    for entity in entities {
        if entity
            .get("script_noteworthy")
            .is_some_and(|value| value == "player_volume")
        {
            zones.insert(
                entity
                    .get("targetname")
                    .ok_or("Native player_volume lacks its source zone targetname")?
                    .clone(),
            );
        }
    }
    let zone_id = |name: &str| format!("zone.{}", name.strip_prefix("zone_").unwrap_or(name));
    let mut links = BTreeMap::<String, Vec<String>>::new();
    let connections: Vec<_> = edges.iter().enumerate().map(|(index,(a,b,flags))| {
        let id = format!("connection.native.{index}");
        for flag in flags { links.entry(flag.clone()).or_default().push(id.clone()); }
        json!({"id":id,"from":zone_id(a),"to":zone_id(b),"initially_open":flags.contains("always_on")})
    }).collect();
    game["content"]["zones"] = json!(
        zones
            .iter()
            .map(|name| json!({"id":zone_id(name),"initially_open":name == "zone_start"}))
            .collect::<Vec<_>>()
    );
    game["content"]["zone_connections"] = json!(connections);
    if let Some(volumes) = game["access"]["volumes"].as_array_mut() {
        let registered: BTreeSet<_> = zones.iter().map(|name| zone_id(name)).collect();
        let count = volumes.len();
        volumes.retain(|volume| {
            volume["zone"]
                .as_str()
                .is_some_and(|zone| registered.contains(zone))
        });
        if volumes.len() != count {
            println!(
                "Retained {} source-registered player-zone volumes; {} unregistered native volume pieces remain passive query geometry",
                volumes.len(),
                count - volumes.len()
            );
        }
    }
    let mut triggers = Vec::new();
    let mut owners = game["environment"]["ownership"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    for args in activations {
        let matching: Vec<_> = entities
            .iter()
            .enumerate()
            .filter(|(_, entity)| entity.get("targetname") == Some(&args[0]))
            .collect();
        if matching.len() != 1 {
            zone_gaps.push(format!(
                "access activation source {} resolves {} native entities",
                args[0],
                matching.len()
            ));
            continue;
        }
        let (index, entity) = matching[0];
        let brush = entity
            .get("model")
            .is_some_and(|model| model.starts_with('*'));
        let id = format!("{}.native.{index}", if brush { "brush" } else { "trigger" });
        let class = entity.get("classname").map(String::as_str).unwrap_or("");
        if !matches!(class, "trigger_box" | "trigger_multiple" | "trigger_radius") {
            zone_gaps.push(format!("access activation source {} requires native representation integration: {entity:?}",args[0]));
            continue;
        }
        if !game["environment"]["triggers"]
            .as_array()
            .is_some_and(|rows| rows.iter().any(|row| row["id"] == id))
        {
            zone_gaps.push(format!(
                "access activation source {} has no converted trigger geometry",
                args[0]
            ));
            continue;
        }
        let flags: u32 = entity
            .get("spawnflags")
            .map(|flags| flags.parse())
            .transpose()?
            .unwrap_or(0);
        let contents = if flags & 8 == 0 { 0x40000000 } else { 0 }
            | if flags & 1 != 0 { 0x40000 } else { 0 }
            | if flags & 2 != 0 { 0x80000 } else { 0 }
            | if flags & 4 != 0 { 0x100000 } else { 0 }
            | if flags & 16 != 0 { 8 } else { 0 }
            | if class == "trigger_multiple" {
                0
            } else {
                0x200000
            };
        missing.retain(|gap| {
            !gap.starts_with(&format!(
                "environment unresolved script ownership/activation/shootability for {id}:"
            )) && gap != &format!("environment script ownership for {id} ({class})")
        });
        owners.retain(|owner| owner["id"] != id);
        owners.push(json!({"id":id,"owner":"access","shootable":false}));
        triggers.push(json!({"trigger":id,"flags":&args[1..],"contents":contents}));
    }
    if !owners.is_empty() {
        game["environment"]["ownership"] = json!(owners);
    }
    if game["access"].is_null() {
        game["access"] = json!({});
    }
    game["access"]["flag_links"] = json!(
        links
            .into_iter()
            .map(|(flag, connections)| json!({"flag":flag,"connections":connections}))
            .collect::<Vec<_>>()
    );
    game["access"]["trigger_activations"] = json!(triggers);
    zone_gaps.push("access source zone events, spawn enablement, barriers and remaining flag producers require conversion".into());
    println!(
        "Converted {calls} source zone links into {} undirected connections and {} zone definitions",
        edges.len(),
        zones.len()
    );
    for gap in &zone_gaps {
        println!("Pending native zone conversion: {gap}");
    }
    missing.extend(zone_gaps);
    Ok(())
}

pub(super) fn owner(source: &Value) -> Option<(&'static str, bool)> {
    let name = source["targetname"].as_str().unwrap_or("");
    let noteworthy = source["script_noteworthy"].as_str().unwrap_or("");
    if name == "player_slow_area" {
        Some(("movement", false))
    } else if name == "staff_craftable_trigger" && source["classname"] == "trigger_box" {
        Some(("native", false))
    } else if noteworthy == "player_volume" {
        Some(("boundary", false))
    } else if source["classname"] == "info_volume" && name.starts_with("zone_") {
        Some(("access", false))
    } else if matches!(
        name,
        "trig_use_tank" | "trig_tank_station_call" | "vol_on_tank_watch"
    ) {
        Some(("vehicle", false))
    } else if matches!(name, "zombie_debris" | "zombie_door") {
        Some(("access", false))
    } else if name.starts_with("trig_stomp_kill_left_")
        || name.starts_with("trig_stomp_kill_right_")
    {
        Some(("robot", false))
    } else if noteworthy == "flamethrower_trigger" {
        Some(("enemy", false))
    } else if matches!(
        name,
        "sndCaptureZone"
            | "sndBG"
            | "sndMaelstrom"
            | "ambient_package"
            | "pack_a_punch_intro_trigger"
    ) || noteworthy == "sndMusicDoor"
    {
        Some(("ambience", false))
    } else if name == "wagon_damage_trigger" {
        Some(("quest", true))
    } else if matches!(
        name,
        "fire_link_damage"
            | "fire_sacrifice_volume"
            | "oneinchpunch_church_volume"
            | "oneinchpunch_bunker_volume"
            | "trigger_oneinchpunch_church_birdbath"
            | "trigger_oneinchpunch_bunker_table"
    ) || noteworthy == "foot_box_volume"
    {
        Some(("quest", false))
    } else {
        None
    }
}

pub(super) fn native_staff_trigger(
    catalog: &Catalog<'_>,
    entities: &[crate::entities::Entity],
) -> Result<usize> {
    let targets = [
        "staff_craftable_trigger",
        "staff_air_craftable_trigger",
        "staff_fire_craftable_trigger",
        "staff_lightning_craftable_trigger",
        "staff_water_craftable_trigger",
    ];
    let mut references = [0_usize; 5];
    let mut scripts = 0;
    for (_, zone, root) in catalog.definitions(48) {
        let streams = &catalog.zones[zone].1.streams;
        let bytes = streams.read(
            Address::decode(streams.u32(root.add(8)?)?)?,
            streams.u32(root.add(4)?)? as usize,
        )?;
        scripts += 1;
        for (index, target) in targets.iter().enumerate() {
            references[index] += usize::from(bytes.windows(target.len() + 1).enumerate().any(
                |(offset, window)| {
                    (offset == 0 || bytes[offset - 1] == 0)
                        && window[..target.len()] == *target.as_bytes()
                        && window[target.len()] == 0
                },
            ));
        }
    }
    if references[0] != 0 || references[1..].contains(&0) {
        return Err(format!("Native staff template requires renewed source binding recovery: unbound/element target reference counts {references:?}").into());
    }
    let templates: Vec<_> = entities
        .iter()
        .filter(|entity| {
            entity
                .get("targetname")
                .is_some_and(|name| name == targets[0])
        })
        .collect();
    if templates.len() != 1
        || templates[0].get("classname").map(String::as_str) != Some("trigger_box")
        || templates[0].keys().any(|key| {
            !matches!(
                key.as_str(),
                "classname"
                    | "targetname"
                    | "guid"
                    | "origin"
                    | "angles"
                    | "height"
                    | "width"
                    | "length"
            )
        })
        || entities.iter().any(|entity| {
            entity
                .get("target")
                .is_some_and(|target| target == targets[0])
        })
    {
        let keys: Vec<_> = templates
            .iter()
            .flat_map(|entity| entity.keys().map(String::as_str))
            .collect();
        let links: Vec<_> = entities
            .iter()
            .enumerate()
            .filter(|(_, entity)| {
                entity
                    .get("target")
                    .is_some_and(|target| target == targets[0])
            })
            .map(|(index, entity)| {
                let fields: std::collections::BTreeMap<_, _> = [
                    "classname",
                    "targetname",
                    "target",
                    "script_noteworthy",
                    "origin",
                ]
                .into_iter()
                .filter_map(|key| entity.get(key).map(|value| (key, value)))
                .collect();
                json!({"entity":index,"fields":fields})
            })
            .collect();
        return Err(format!("Native staff template has a source activation/link field that requires binding recovery: keys {keys:?}, incoming {}",json!(links)).into());
    }
    Ok(scripts)
}

pub(super) fn link_owners(game: &mut Value, entities: &[crate::entities::Entity]) -> Result<()> {
    for trigger in game["environment"]["triggers"]
        .as_array_mut()
        .into_iter()
        .flatten()
    {
        if trigger["source"]["targetname"] == "pack_a_punch_intro_trigger" {
            let target = trigger["source"]["target"]
                .as_str()
                .ok_or("Pack a Punch discovery trigger has no source look target")?;
            let mut matches = entities.iter().filter(|entity| {
                entity.get("targetname").is_some_and(|name| name == target)
                    && entity
                        .get("classname")
                        .is_some_and(|class| class == "script_struct")
            });
            let look = matches.next().ok_or_else(|| {
                format!("Pack a Punch discovery target {target} has no installed script_struct")
            })?;
            if matches.next().is_some() {
                return Err(format!(
                    "Pack a Punch discovery target {target} has ambiguous installed script_structs"
                )
                .into());
            }
            let [x, y, z] = crate::entities::vector(look, "origin")?.ok_or_else(|| {
                format!("Pack a Punch discovery target {target} lacks a source origin")
            })?;
            trigger["target_position"] = json!([x * 0.0254, z * 0.0254, -y * 0.0254]);
        }
        if !matches!(
            trigger["source"]["targetname"].as_str(),
            Some("trig_use_tank" | "vol_on_tank_watch")
        ) {
            continue;
        }
        let mut matches = entities.iter().filter(|entity| {
            entity
                .get("targetname")
                .is_some_and(|target| target == "tank")
        });
        let tank = matches
            .next()
            .ok_or("Native tank trigger has no source link owner named tank")?;
        if matches.next().is_some() {
            return Err("Native tank trigger has ambiguous source link owners named tank".into());
        }
        let origin = tank
            .get("origin")
            .ok_or("Native tank source link owner lacks its origin")?;
        crate::entities::vector(tank, "origin")?;
        let angles = if let Some(angles) = tank.get("angles") {
            angles.clone()
        } else {
            format!(
                "0 {} 0",
                tank.get("angle").map(String::as_str).unwrap_or("0")
            )
        };
        trigger["source"]["link_owner_origin"] = json!(origin);
        trigger["source"]["link_owner_angles"] = json!(angles);
    }
    Ok(())
}

pub(super) fn brush_transform(value: &Value) -> Result<([f64; 3], [f64; 4])> {
    let matrix = value
        .as_array()
        .ok_or("Native brush transform is missing")?;
    let matrix: Vec<_> = matrix
        .iter()
        .map(|value| {
            value
                .as_f64()
                .filter(|value| value.is_finite())
                .ok_or_else(|| "Native brush transform is non-finite".into())
        })
        .collect::<Result<_>>()?;
    let m: [f64; 16] = matrix
        .try_into()
        .map_err(|_| "Native brush transform is not a 4x4 matrix")?;
    if [m[3], m[7], m[11], m[15]] != [0., 0., 0., 1.] {
        return Err("Native brush transform is not affine".into());
    }
    for a in 0..3 {
        for b in 0..3 {
            let dot: f64 = (0..3).map(|row| m[a * 4 + row] * m[b * 4 + row]).sum();
            if (dot - if a == b { 1. } else { 0. }).abs() > 0.0001 {
                return Err("Native brush transform contains unsupported scale or shear".into());
            }
        }
    }
    let determinant = m[0] * (m[5] * m[10] - m[9] * m[6]) - m[4] * (m[1] * m[10] - m[9] * m[2])
        + m[8] * (m[1] * m[6] - m[5] * m[2]);
    if (determinant - 1.).abs() > 0.0001 {
        return Err("Native brush transform reverses handedness".into());
    }
    let mut rotation = if m[0] + m[5] + m[10] > 0. {
        let s = (m[0] + m[5] + m[10] + 1.).sqrt() * 2.;
        [
            (m[6] - m[9]) / s,
            (m[8] - m[2]) / s,
            (m[1] - m[4]) / s,
            s * 0.25,
        ]
    } else if m[0] > m[5] && m[0] > m[10] {
        let s = (1. + m[0] - m[5] - m[10]).sqrt() * 2.;
        [
            s * 0.25,
            (m[4] + m[1]) / s,
            (m[8] + m[2]) / s,
            (m[6] - m[9]) / s,
        ]
    } else if m[5] > m[10] {
        let s = (1. + m[5] - m[0] - m[10]).sqrt() * 2.;
        [
            (m[4] + m[1]) / s,
            s * 0.25,
            (m[9] + m[6]) / s,
            (m[8] - m[2]) / s,
        ]
    } else {
        let s = (1. + m[10] - m[0] - m[5]).sqrt() * 2.;
        [
            (m[8] + m[2]) / s,
            (m[9] + m[6]) / s,
            s * 0.25,
            (m[1] - m[4]) / s,
        ]
    };
    let length = rotation
        .iter()
        .map(|value| value * value)
        .sum::<f64>()
        .sqrt();
    if !length.is_finite() || length == 0. {
        return Err("Native brush rotation is invalid".into());
    }
    for value in &mut rotation {
        *value /= length;
    }
    Ok(([m[12], m[13], m[14]], rotation))
}

pub(super) fn brushes(world: &Value, collision: &Value) -> Result<(Vec<Value>, Vec<String>)> {
    let models = collision["brush_models"]
        .as_array()
        .ok_or("Native collision brush-model index is missing")?;
    let entities = world["brush_entities"]
        .as_array()
        .ok_or("Native world brush-entity index is missing")?;
    let mut rows = Vec::new();
    let mut classifiers = Vec::new();
    let mut ids = std::collections::BTreeSet::new();
    for entity in entities {
        let id = entity["id"]
            .as_str()
            .ok_or("Native brush entity ID is missing")?;
        if !ids.insert(id) {
            return Err(format!("Duplicate native brush entity: {id}").into());
        }
        let index = entity["native_brush_model"]
            .as_u64()
            .ok_or("Native brush model identity is missing")?;
        let mut matching = models
            .iter()
            .filter(|model| model["native_brush_model"] == index);
        let model = matching
            .next()
            .ok_or_else(|| format!("Native brush {id} has no collision submodel *{index}"))?;
        if matching.next().is_some() {
            return Err(
                format!("Native brush {id} collision submodel *{index} is ambiguous").into(),
            );
        }
        let (position, rotation) = brush_transform(&entity["matrix"])?;
        let source = entity["source"]
            .as_object()
            .ok_or("Native brush entity source fields are missing")?;
        let class = source
            .get("classname")
            .and_then(Value::as_str)
            .ok_or("Native brush classname is missing")?;
        if !matches!(
            class,
            "script_brushmodel"
                | "heli_height_lock"
                | "trigger_multiple"
                | "trigger_use"
                | "trigger_damage"
                | "trigger_box"
                | "trigger_lookat"
                | "info_volume"
        ) {
            classifiers.push(format!(
                "environment native classifier for {id} ({class}, *{index})"
            ));
        }
        let volumes = model["volumes"].as_array().ok_or("Native brush convex volumes are missing")?.iter().map(|volume| {
            let native = volume["native_brush"].as_u64().ok_or("Native convex brush identity is missing")?;
            let vertices = volume["vertices"].as_array().ok_or("Native convex brush vertices are missing")?;
            Ok(json!({"id":format!("{id}.convex.{native}"),"position":[0.,0.,0.],"vertices":vertices,
                "contents":volume["contents"],"surface_flags":volume["surface_flags"]}))
        }).collect::<Result<Vec<_>>>()?;
        let triangles = model["triangle_shapes"].as_array().ok_or_else(|| format!("Native brush *{index} terrain-shape conversion is missing"))?
            .iter().enumerate().map(|(shape_index, shape)| {
                for key in ["contents","surface_flags","vertices","indices"] {
                    if shape.get(key).is_none() { return Err(format!("Native brush *{index} terrain shape is missing {key}").into()); }
                }
                Ok(json!({"id":format!("{id}.triangle.{shape_index}"),"contents":shape["contents"],
                    "surface_flags":shape["surface_flags"],"vertices":shape["vertices"],"indices":shape["indices"]}))
            }).collect::<Result<Vec<_>>>()?;
        let bounds = model["bounds"]
            .as_array()
            .filter(|bounds| bounds.len() == 2)
            .ok_or("Native brush bounds are missing")?;
        let visual_meshes = entity["visual_meshes"]
            .as_u64()
            .ok_or("Native brush visual mesh count is missing")?;
        rows.push(
            json!({"id":id,"position":position,"rotation":rotation,"bounds":bounds,"source":source,
            "volumes":volumes,"triangle_shapes":triangles,"visual_meshes":visual_meshes}),
        );
    }
    println!(
        "Joined {} native brush entities to collision submodels; {} source classifications remain unresolved",
        rows.len(),
        classifiers.len()
    );
    Ok((rows, classifiers))
}

pub(super) fn rotated_volumes(row: &Value) -> Result<Vec<Value>> {
    let [x, y, z, w]: [f64; 4] = serde_json::from_value(row["rotation"].clone())?;
    let rotate = |value: &Value| -> Result<[f64; 3]> {
        let [a, b, c]: [f64; 3] = serde_json::from_value(value.clone())?;
        let [tx, ty, tz] = [
            2. * (y * c - z * b),
            2. * (z * a - x * c),
            2. * (x * b - y * a),
        ];
        Ok([
            a + w * tx + y * tz - z * ty,
            b + w * ty + z * tx - x * tz,
            c + w * tz + x * ty - y * tx,
        ])
    };
    row["volumes"].as_array().ok_or("Native trigger convex geometry is missing")?.iter().map(|volume| {
        let vertices = volume["vertices"].as_array().ok_or("Native trigger vertices are missing")?.iter().map(rotate).collect::<Result<Vec<_>>>()?;
        Ok(json!({"id":volume["id"],"position":rotate(&volume["position"])? ,"vertices":vertices,
            "contents":volume["contents"],"surface_flags":volume["surface_flags"]}))
    }).collect()
}

pub(super) fn environment(
    game: &mut Value,
    rows: Vec<Value>,
    missing: &mut Vec<String>,
) -> Result<()> {
    let mut brushes = Vec::new();
    let mut triggers = game["environment"]["triggers"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    let mut owners = game["environment"]["ownership"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    let mut mud = Vec::new();
    let mut zones = Vec::new();
    for row in rows {
        let source = &row["source"];
        let class = source["classname"]
            .as_str()
            .ok_or("Native environment classname is missing")?;
        let id = row["id"]
            .as_str()
            .ok_or("Native environment identity is missing")?;
        let movement = source["targetname"] == "player_slow_area";
        let boundary = source["script_noteworthy"] == "player_volume"
            || (class == "info_volume"
                && source["targetname"]
                    .as_str()
                    .is_some_and(|name| name.starts_with("zone_")));
        let convex_trigger = matches!(
            class,
            "trigger_multiple"
                | "trigger_use"
                | "trigger_damage"
                | "trigger_box"
                | "trigger_lookat"
                | "info_volume"
        );
        if convex_trigger
            && (row["triangle_shapes"]
                .as_array()
                .is_some_and(|shapes| !shapes.is_empty())
                || row["volumes"]
                    .as_array()
                    .is_some_and(|volumes| !volumes.is_empty()))
        {
            if movement || boundary {
                let volumes = rotated_volumes(&row)?;
                let position: [f64; 3] = serde_json::from_value(row["position"].clone())?;
                for volume in &volumes {
                    let offset: [f64; 3] = serde_json::from_value(volume["position"].clone())?;
                    let mut world_volume = volume.clone();
                    world_volume["position"] = json!(std::array::from_fn::<_, 3, _>(
                        |axis| position[axis] + offset[axis]
                    ));
                    if movement {
                        world_volume["id"] = json!(format!(
                            "mud.{}",
                            volume["id"]
                                .as_str()
                                .ok_or("Mud volume identity is missing")?
                        ));
                        mud.push(world_volume);
                    } else {
                        let name = source["targetname"]
                            .as_str()
                            .ok_or("Native player_volume has no source zone targetname")?;
                        zones.push(json!({"zone":format!("zone.{}",name.strip_prefix("zone_").unwrap_or(name)),"volume":world_volume}));
                    }
                }
                if row["triangle_shapes"]
                    .as_array()
                    .is_some_and(|shapes| !shapes.is_empty())
                {
                    missing.push(format!(
                        "{} consumer for native triangle volume geometry {id}",
                        if movement { "movement" } else { "boundary" }
                    ));
                }
            }
            triggers.push(json!({"id":id,"position":row["position"],"source":source,"shape":{"kind":"brush",
                "rotation":row["rotation"],"volumes":row["volumes"],"triangle_shapes":row["triangle_shapes"]}}));
        } else {
            if convex_trigger {
                missing.push(format!("environment trigger shape for {id} ({class}) has no native convex or triangle geometry"));
            }
            brushes.push(row.clone());
        }
        if let Some((owner, shootable)) = owner(source) {
            owners.push(json!({"id":id,"owner":owner,"shootable":shootable}));
        } else {
            owners.push(json!({"id":id,"owner":"deferred","shootable":false}));
            if convex_trigger {
                if !source["targetname"]
                    .as_str()
                    .is_some_and(|name| name.starts_with("trig_zone_"))
                {
                    missing.push(format!("environment source consumer for {id} ({class}, targetname {}) is absent from recovered scripts",source["targetname"]));
                }
            } else {
                missing.push(format!("presentation source brush lifecycle for {id} ({class}) requires scene integration"));
            }
        }
    }
    if game["environment"].is_null() {
        game["environment"] = json!({});
    }
    game["environment"]["brush_entities"] = json!(brushes);
    if !triggers.is_empty() {
        game["environment"]["triggers"] = json!(triggers);
    }
    if !owners.is_empty() {
        game["environment"]["ownership"] = json!(owners);
    }
    if !mud.is_empty() {
        game["environment"]["mud"] = json!({"scale":0.6,"stamin_scale":0.7,"step":0.4/4.,"stamin_step":0.3/6.,"interval":0.1,"volumes":mud});
    }
    if !zones.is_empty() {
        if game["access"].is_null() {
            game["access"] = json!({});
        }
        game["access"]["volumes"] = json!(zones);
    }
    Ok(())
}

pub(super) fn initial_brush_lifecycles(
    game: &mut Value,
    entities: &[crate::entities::Entity],
    missing: &mut Vec<String>,
) -> Result<()> {
    if game["delivery_scope"] != "first_playable_solo" {
        return Ok(());
    }
    let mut ownership = game["environment"]["ownership"]
        .as_array()
        .cloned()
        .ok_or("Initial brush ownership is missing")?;
    let mut resolved = 0;
    for brush in game["environment"]["brush_entities"]
        .as_array_mut()
        .ok_or("Initial brush definitions are missing")?
    {
        let source = &brush["source"];
        let id = brush["id"]
            .as_str()
            .ok_or("Initial brush identity is missing")?
            .to_owned();
        let class = source["classname"]
            .as_str()
            .ok_or("Initial brush classname is missing")?
            .to_owned();
        let name = source["targetname"].as_str().unwrap_or("");
        let noteworthy = source["script_noteworthy"].as_str().unwrap_or("");
        let target_owner = entities.iter().find(|entity| {
            entity.get("target").is_some_and(|target| target == name)
                && entity
                    .get("targetname")
                    .is_some_and(|name| matches!(name.as_str(), "zombie_door" | "zombie_debris"))
        });
        let smoke_detector = entities.iter().any(|entity| {
            entity.get("target").is_some_and(|target| target == name)
                && entity
                    .get("targetname")
                    .is_some_and(|target| target == "puzzle_smoke_origin")
        });
        let collidable = brush["volumes"]
            .as_array()
            .into_iter()
            .flatten()
            .any(|volume| {
                volume["contents"]
                    .as_u64()
                    .is_some_and(|contents| contents & 0x10001 != 0)
            })
            || brush["triangle_shapes"]
                .as_array()
                .into_iter()
                .flatten()
                .any(|shape| {
                    shape["contents"]
                        .as_u64()
                        .is_some_and(|contents| contents & 0x10001 != 0)
                });
        let (owner, visible, solid, removed) = if class == "heli_height_lock" {
            ("deferred", false, false, true)
        } else if target_owner.is_some()
            || matches!(
                name,
                "chamber_entrance" | "junk_nml_chamber_clip" | "traversal_blocker"
            )
        {
            ("access", true, collidable, false)
        } else if name == "zombie_blood_visible" {
            ("deferred", false, collidable, false)
        } else if name == "tank_path_blocker" {
            ("deferred", false, false, true)
        } else if name.starts_with("clip_foot_left_") || name.starts_with("clip_foot_right_") {
            ("deferred", false, false, true)
        } else if matches!(name, "clip_foot_bottom_tank" | "clip_foot_bottom_wind") {
            ("deferred", true, false, false)
        } else if smoke_detector
            || noteworthy == "ice_ceiling_tile"
            || name == "ice_chamber_digit"
            || noteworthy == "fire_torch_ternary"
        {
            ("deferred", false, false, true)
        } else if matches!(name, "easter_mechzombie_spawn" | "hole_poster") {
            ("deferred", true, collidable, false)
        } else {
            continue;
        };
        brush["initial_state"] = json!({"visible":visible,"solid":solid,"removed":removed});
        ownership.retain(|entry| entry["id"] != id);
        ownership.push(json!({"id":id,"owner":owner,"shootable":false}));
        missing.retain(|gap| {
            gap != &format!(
                "presentation source brush lifecycle for {id} ({class}) requires scene integration"
            )
        });
        resolved += 1;
    }
    game["environment"]["ownership"] = json!(ownership);
    println!(
        "Bound {resolved} source brush initial lifecycles for the scoped session; deferred owners cannot activate"
    );
    Ok(())
}

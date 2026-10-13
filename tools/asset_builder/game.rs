use crate::{convert::Catalog, core::*, fastfile::Address, pack::Pack};
use serde_json::{Value, json};
use std::sync::Arc;

fn variable(catalog: &Catalog<'_>, name: &str, column: usize, fallback: Value) -> Result<Value> {
    let (zone, root) = catalog.resolve(42, "mp/zombiemode.csv")?;
    let streams = &catalog.zones[zone].1.streams;
    let columns = streams.u32(root.add(4)?)? as usize;
    let rows = streams.u32(root.add(8)?)? as usize;
    if column >= columns { return Err("Native zombie difficulty table lacks the required column".into()); }
    let cells = Address::decode(streams.u32(root.add(12)?)?)?;
    let cell = |row: usize, column: usize| -> Result<String> {
        let pointer = streams.u32(cells.add((row * columns + column) * 8)?)?;
        if pointer == 0 { return Ok(String::new()); }
        Ok(streams.text(Address::decode(pointer)?)?)
    };
    for row in 0..rows {
        if cell(row, 0)? != name { continue; }
        let text = cell(row, column)?;
        if text.is_empty() { return Ok(fallback); }
        let value: f64 = text.parse()?;
        if !value.is_finite() { return Err(format!("Native zombie variable is non-finite: {name}").into()); }
        if (fallback.is_i64() || fallback.is_u64()) && !(i32::MIN as f64..=i32::MAX as f64).contains(&value.trunc()) {
            return Err(format!("Native zombie integer variable exceeds the source range: {name}").into());
        }
        return Ok(if fallback.is_i64() || fallback.is_u64() { json!(value.trunc() as i64) } else { json!(value) });
    }
    Ok(fallback)
}

fn zone_graph(game: &mut Value, entities: &[crate::entities::Entity], missing: &mut Vec<String>) -> Result<()> {
    use std::collections::{BTreeMap,BTreeSet};
    let mut zone_gaps = Vec::new();
    let path = std::path::PathBuf::from(std::env::var_os("USERPROFILE").ok_or("USERPROFILE is unavailable")?)
        .join("Documents/T6ZMResearch/t6zm-master/source/server-tomb/maps/mp/zm_tomb.gsc");
    let source = std::fs::read_to_string(path)?;
    if !source.contains("init_zones[0] = \"zone_start\";") { return Err("Original initial-zone source expression is unsupported".into()); }
    let body = source.split_once("working_zone_init()").ok_or("Original working_zone_init source is missing")?.1;
    let body = body.trim_start().strip_prefix('{').ok_or("Original working_zone_init body is missing")?;
    let body = body.split_once('}').ok_or("Original working_zone_init body is unterminated")?.0;
    let literals = |line: &str, call: &str| -> Result<Vec<String>> {
        let args = line.strip_prefix(call).ok_or("Unsupported zone source call")?.trim()
            .strip_prefix('(').and_then(|args| args.strip_suffix(");")).ok_or_else(|| format!("Unsupported literal zone call syntax: {line}"))?;
        args.split(',').map(|arg| {
            let value = arg.trim().strip_prefix('"').and_then(|arg| arg.strip_suffix('"')).ok_or_else(|| format!("Nonliteral zone call: {line}"))?;
            if value.is_empty() || !value.bytes().all(|byte| byte.is_ascii_alphanumeric() || byte == b'_') { return Err(format!("Unsupported zone source literal: {value}").into()); }
            Ok(value.to_owned())
        }).collect()
    };
    let mut zones = BTreeSet::new();
    let mut edges = Vec::<(String,String,BTreeSet<String>)>::new();
    let mut edge_indices = BTreeMap::new();
    let mut activations = Vec::new();
    let mut calls = 0;
    for line in body.lines().map(str::trim).filter(|line| !line.is_empty()) {
        if line.starts_with("add_adjacent_zone") {
            let args = literals(line,"add_adjacent_zone")?;
            let [a,b,flag]: [String;3] = args.try_into().map_err(|_| "Original adjacent zone call requires three literals")?;
            zones.insert(a.clone()); zones.insert(b.clone()); calls += 1;
            let key = if a <= b { (a.clone(),b.clone()) } else { (b.clone(),a.clone()) };
            let index = *edge_indices.entry(key).or_insert_with(|| { edges.push((a,b,BTreeSet::new())); edges.len()-1 });
            edges[index].2.insert(flag);
        } else if line.starts_with("level thread activate_zone_trig") {
            let args = literals(line,"level thread activate_zone_trig")?;
            if !(2..=3).contains(&args.len()) { return Err("Original zone activation call requires two or three literals".into()); }
            activations.push(args);
        } else if line == "flag_init( \"always_on\" );" || line == "flag_set( \"always_on\" );" {
        } else { return Err(format!("Unsupported working_zone_init statement: {line}").into()); }
    }
    for entity in entities {
        if entity.get("script_noteworthy").is_some_and(|value| value == "player_volume") {
            zones.insert(entity.get("targetname").ok_or("Native player_volume lacks its source zone targetname")?.clone());
        }
    }
    let zone_id = |name: &str| format!("zone.{}",name.strip_prefix("zone_").unwrap_or(name));
    let mut links = BTreeMap::<String,Vec<String>>::new();
    let connections: Vec<_> = edges.iter().enumerate().map(|(index,(a,b,flags))| {
        let id = format!("connection.native.{index}");
        for flag in flags { links.entry(flag.clone()).or_default().push(id.clone()); }
        json!({"id":id,"from":zone_id(a),"to":zone_id(b),"initially_open":flags.contains("always_on")})
    }).collect();
    game["content"]["zones"] = json!(zones.iter().map(|name| json!({"id":zone_id(name),"initially_open":name == "zone_start"})).collect::<Vec<_>>());
    game["content"]["zone_connections"] = json!(connections);
    let mut triggers = Vec::new();
    let mut owners = game["environment"]["ownership"].as_array().cloned().unwrap_or_default();
    for args in activations {
        let matching: Vec<_> = entities.iter().enumerate().filter(|(_,entity)| entity.get("targetname") == Some(&args[0])).collect();
        if matching.len() != 1 { zone_gaps.push(format!("access activation source {} resolves {} native entities",args[0],matching.len())); continue; }
        let (index,entity) = matching[0];
        let id = format!("trigger.native.{index}");
        if entity.get("classname").map(String::as_str) != Some("trigger_box") || entity.get("model").is_some_and(|model| model.starts_with('*')) {
            zone_gaps.push(format!("access activation source {} requires native representation integration: {entity:?}",args[0])); continue;
        }
        if !game["environment"]["triggers"].as_array().is_some_and(|rows| rows.iter().any(|row| row["id"] == id)) {
            zone_gaps.push(format!("access activation source {} has no converted trigger geometry",args[0])); continue;
        }
        let flags: u32 = entity.get("spawnflags").ok_or("Native access trigger spawnflags are missing")?.parse()?;
        let contents = if flags&8 == 0 {0x40000000} else {0} | if flags&1 != 0 {0x40000} else {0}
            | if flags&2 != 0 {0x80000} else {0} | if flags&4 != 0 {0x100000} else {0} | if flags&16 != 0 {8} else {0} | 0x200000;
        if contents&0x40000000 == 0 || contents&(8|0x400000|0x1c0000) != 0 {
            zone_gaps.push(format!("access activation {id} requires unsupported native contact categories for contents {contents:#x}")); continue;
        }
        missing.retain(|gap| !gap.starts_with(&format!("environment unresolved script ownership/activation/shootability for {id}:")));
        owners.push(json!({"id":id,"owner":"access","shootable":false}));
        triggers.push(json!({"trigger":id,"flags":&args[1..],"contents":contents}));
    }
    if !owners.is_empty() { game["environment"]["ownership"] = json!(owners); }
    game["access"] = json!({"flag_links":links.into_iter().map(|(flag,connections)| json!({"flag":flag,"connections":connections})).collect::<Vec<_>>(),"trigger_activations":triggers});
    zone_gaps.push("access native client-mode synchronization, source zone events, spawn enablement, barriers and remaining flag producers require conversion".into());
    println!("Converted {calls} source zone links into {} undirected connections and {} zone definitions",edges.len(),zones.len());
    for gap in &zone_gaps { println!("Pending native zone conversion: {gap}"); }
    missing.extend(zone_gaps);
    Ok(())
}

pub fn read(catalog: &Catalog<'_>, entities: &[crate::entities::Entity]) -> Result<(Value, Vec<String>)> {
    let mut native_gaps = Vec::new();
    let mut native_triggers = Vec::new();
    let mut native_owners = Vec::new();
    let mut crafting_benches = Vec::new();
    let mut primitive_counts = [0_usize;2];
    for (index, entity) in entities.iter().enumerate() {
        let Some(class) = entity.get("classname") else { continue; };
        if entity.get("model").is_some_and(|model| model.starts_with('*')) { continue; }
        if !matches!(class.as_str(),"trigger_radius" | "trigger_box") {
            if class.starts_with("trigger_") || class == "info_volume" {
                native_gaps.push(format!("environment native non-brush trigger geometry for trigger.native.{index} ({class})"));
            }
            continue;
        }
        primitive_counts[usize::from(class == "trigger_box")] += 1;
        let id = format!("trigger.native.{index}");
        let origin = crate::entities::vector(entity,"origin").map_err(|error| format!("Native primitive trigger {id}: {error}"))?;
        let scalar = |key: &str| -> Result<Option<f32>> {
            let Some(text) = entity.get(key) else { return Ok(None); };
            let value: f32 = text.parse().map_err(|error| format!("Native primitive trigger {id} has invalid {key} {text:?}: {error}"))?;
            Ok((value.is_finite() && value > 0.).then_some(value*0.0254))
        };
        let height = scalar("height")?;
        let shape = if class == "trigger_radius" {
            scalar("radius")?.zip(height).map(|(radius,height)| json!({"kind":"cylinder","radius":radius,"height":height}))
        } else {
            let transform = crate::scenes::entity_matrix(entity)?;
            scalar("width")?.zip(scalar("length")?).zip(height).map(|((width,length),height)| {
                let half = [width*0.5,height*0.5,length*0.5];
                let vertices: Vec<_> = (0..8).map(|corner| {
                    let local: [f32;3] = std::array::from_fn(|axis| if corner & (1<<axis) == 0 { -half[axis] } else { half[axis] });
                    std::array::from_fn::<_,3,_>(|axis| (0..3).map(|column| transform[column*4+axis]*local[column]).sum::<f32>())
                }).collect();
                json!({"kind":"convex","volumes":[{"id":format!("{id}.volume"),"position":[0.,0.,0.],"vertices":vertices}]})
            })
        };
        if let (Some([x,y,z]),Some(shape)) = (origin,shape) {
            native_triggers.push(json!({"id":id,"position":[x*0.0254,z*0.0254,-y*0.0254],"source":entity,
                "shape":shape}));
            let name = entity.get("targetname").map(String::as_str).unwrap_or("");
            let crafting = match name {
                "open_craftable_trigger" => Some(("open_table","","OPEN_CRAFTABLE",false)),
                "quadrotor_zm_craftable_trigger" => Some(("equip_dieseldrone_zm","equip_dieseldrone_zm","ZM_TOMB_TQ",true)),
                "riotshield_zm_craftable_trigger" => Some(("tomb_shield_zm","tomb_shield_zm","ZOMBIE_GRAB_RIOTSHIELD",true)),
                "staff_air_craftable_trigger" => Some(("elemental_staff_air","staff_air_zm","ZM_TOMB_PUAS",true)),
                "staff_fire_craftable_trigger" => Some(("elemental_staff_fire","staff_fire_zm","ZM_TOMB_PUFS",true)),
                "staff_lightning_craftable_trigger" => Some(("elemental_staff_lightning","staff_lightning_zm","ZM_TOMB_PULS",true)),
                "staff_water_craftable_trigger" => Some(("elemental_staff_water","staff_water_zm","ZM_TOMB_PUIS",true)),
                "gramophone_craftable_trigger" => Some(("gramophone","gramophone","ZOMBIE_GRAB_GRAMOPHONE",true)),
                _ => None,
            };
            if let Some((equipname,weaponname,hint,persistent)) = crafting {
                let script_angles = crate::entities::vector(entity,"script_angles")?;
                let offset_angles = script_angles.unwrap_or([0.;3]);
                let angles = if let Some(angles) = script_angles.or(crate::entities::vector(entity,"angles")?) { angles }
                    else { [0.,entity.get("angle").map(|value| value.parse::<f32>()).transpose()?.unwrap_or(0.),0.] };
                let mut offset_entity = entity.clone();
                offset_entity.insert("angles".into(),format!("{} {} {}",offset_angles[0],offset_angles[1],offset_angles[2]));
                let transform = crate::scenes::entity_matrix(&offset_entity)?;
                let origin = [x*0.0254,z*0.0254,-y*0.0254];
                let position: [f32;3] = std::array::from_fn(|axis| origin[axis]-transform[8+axis]*(6.*0.0254));
                let dimension = |key: &str, fallback: f32| -> Result<f32> {
                    if entity.contains_key(key) { scalar(key)?.ok_or_else(|| format!("Crafting source {id} has invalid {key}").into()) }
                    else { Ok(fallback*0.0254) }
                };
                let dimensions = [dimension("script_width",100.)?,dimension("script_length",32.)?,dimension("script_height",64.)?];
                crafting_benches.push(json!({"id":format!("crafting.native.{index}"),"source_trigger":id,"targetname":name,
                    "equipname":equipname,"weaponname":weaponname,"hint":hint,"position":position,"angles":angles,"dimensions":dimensions,
                    "use_seconds":3.,"persistent":persistent,"delete_source":true,"require_look_at":true}));
                native_owners.push(json!({"id":id,"owner":"crafting","shootable":false}));
                if equipname == "gramophone" { native_gaps.push(format!("crafting gramophone recipe/purchase consumer for {id} requires source lifecycle integration")); }
                continue;
            }
            let owner = if name == "player_slow_area" { "movement" }
                else if entity.get("script_noteworthy").is_some_and(|name| name == "player_volume") { "boundary" }
                else { "unresolved script" };
            native_gaps.push(format!("environment {owner} ownership/activation/shootability for {id}: native contents and script lifecycle require integration (targetname {:?}, noteworthy {:?})",entity.get("targetname"),entity.get("script_noteworthy")));
            if owner == "movement" { native_gaps.push(format!("environment mud primitive trigger for {id} requires source-selected volume integration")); }
        } else {
            native_gaps.push(format!("environment native {class} {id} requires explicit source origin and positive finite dimensions; fields: {entity:?}"));
        }
    }
    if !crafting_benches.is_empty() { native_gaps.push("crafting native per-player unitrigger look/shape/persistence lifecycle conversion".into()); }
    println!("Converted {} of {} native non-brush primitive triggers ({} radius, {} boxes); {} precise native-trigger gaps remain",native_triggers.len(),primitive_counts.iter().sum::<usize>(),primitive_counts[0],primitive_counts[1],native_gaps.len());
    println!("Converted {} source crafting bench bindings; native per-player unitrigger selection/persistence remains incomplete",crafting_benches.len());
    for gap in &native_gaps { println!("Pending native trigger conversion: {gap}"); }
    let mut localized = Vec::new();
    for (name, zone, root) in catalog.definitions(24) {
        let streams = &catalog.zones[zone].1.streams;
        let text = streams.text(Address::decode(streams.u32(root)?)?)?;
        let id = format!("text.{}", name.to_ascii_lowercase());
        if !id.bytes().all(|byte| byte.is_ascii_alphanumeric() || b"._-".contains(&byte)) {
            return Err(format!("Native localization key cannot form a runtime content ID: {name}").into());
        }
        localized.push(json!({"id":id,"key":name,"text":text}));
    }
    if localized.is_empty() { return Err("Required native localization definitions are missing".into()); }
    let maximum = variable(catalog, "zombie_max_ai", 2, json!(24))?;
    let extra = variable(catalog, "zombie_ai_per_player", 2, json!(6))?.as_f64().ok_or("Native population increment is not numeric")? * 0.5;
    let ranks = ["m1911_zm", "c96_zm", "cz75_zm", "cz75dw_zm", "kard_zm", "fiveseven_zm", "beretta93r_zm", "beretta93r_extclip_zm",
        "fivesevendw_zm", "rnma_zm", "python_zm", "judge_zm", "cz75_upgraded_zm", "cz75dw_upgraded_zm", "kard_upgraded_zm", "fiveseven_upgraded_zm",
        "m1911_upgraded_zm", "c96_upgraded_zm", "beretta93r_upgraded_zm", "beretta93r_extclip_upgraded_zm", "fivesevendw_upgraded_zm", "rnma_upgraded_zm",
        "python_upgraded_zm", "judge_upgraded_zm", "ray_gun_zm", "raygun_mark2_zm", "freezegun_zm", "ray_gun_upgraded_zm", "raygun_mark2_upgraded_zm",
        "freezegun_upgraded_zm", "microwavegundw_zm", "microwavegundw_upgraded_zm"].into_iter().enumerate()
        .map(|(rank, name)| json!({"weapon":format!("weapon.{name}"),"rank":rank})).collect::<Vec<_>>();
    let rounds = json!({"health_start":variable(catalog,"zombie_health_start",2,json!(150))?,
        "health_increase":variable(catalog,"zombie_health_increase",2,json!(100))?,
        "health_multiplier":variable(catalog,"zombie_health_increase_multiplier",2,json!(0.1))?,"exponential_round":10,
        "population_base":maximum,"solo_population_increment":extra,"population_round_divisor":5,
        "population_acceleration_round":10,"population_acceleration":0.15,"early_population":[0.25,0.3,0.5,0.7,0.9],
        "ordinary_cap":maximum,"actor_cap":32,"spawn_seconds":variable(catalog,"zombie_spawn_delay",2,json!(2.0))?,"spawn_decay":0.95,
        "minimum_spawn_seconds":0.08,"first_spawn_seconds":2.5,"between_round_seconds":variable(catalog,"zombie_between_round_time",1,json!(10))?,
        "speed_multiplier":variable(catalog,"zombie_move_speed_multiplier",2,json!(8))?,"speed_random_band":35,"gait_thresholds":[35,70]});
    let solo = json!({"perk":"perk.quick_revive","seconds":10.,"bleedout_seconds":45.,"bleedout_poll_seconds":1.,
        "maximum_purchases":3,"downed_weapon":"weapon.c96_upgraded_zm","reserve_clips":2,"pistol_ranks":ranks,"pistol_replace_below":15});
    println!("Converted {} native localized strings, player health and recovered round/solo-revive parameters", localized.len());
    let mut game = json!({"version":2,"player_health":{
        "maximum":100,"regular_delay":2.4,"regular_step_fraction":1.,
        "critical_delay":5.,"critical_ratio":0.2,"critical_step_seconds":0.05,"critical_step_fraction":0.1,
        "worthy_damage_ratio":0.1,"immunity_regular":0.35,"immunity_critical_enter":0.5,
        "immunity_critical_repeat":0.3,"feedback_seconds":0.8},
        "rounds":rounds,"solo_revive":solo,"content":{"localized_strings":localized}});
    if !native_triggers.is_empty() { game["environment"] = json!({"triggers":native_triggers,"ownership":native_owners,"crafting_benches":crafting_benches}); }
    zone_graph(&mut game,entities,&mut native_gaps)?;
    Ok((game,native_gaps))
}

fn brush_transform(value: &Value) -> Result<([f64; 3], [f64; 4])> {
    let matrix = value.as_array().ok_or("Native brush transform is missing")?;
    let matrix: Vec<_> = matrix.iter().map(|value| value.as_f64().filter(|value| value.is_finite())
        .ok_or_else(|| "Native brush transform is non-finite".into())).collect::<Result<_>>()?;
    let m: [f64; 16] = matrix.try_into().map_err(|_| "Native brush transform is not a 4x4 matrix")?;
    if [m[3],m[7],m[11],m[15]] != [0.,0.,0.,1.] { return Err("Native brush transform is not affine".into()); }
    for a in 0..3 { for b in 0..3 {
        let dot: f64 = (0..3).map(|row| m[a*4+row] * m[b*4+row]).sum();
        if (dot - if a == b { 1. } else { 0. }).abs() > 0.0001 {
            return Err("Native brush transform contains unsupported scale or shear".into());
        }
    }}
    let determinant = m[0]*(m[5]*m[10]-m[9]*m[6])-m[4]*(m[1]*m[10]-m[9]*m[2])+m[8]*(m[1]*m[6]-m[5]*m[2]);
    if (determinant - 1.).abs() > 0.0001 { return Err("Native brush transform reverses handedness".into()); }
    let mut rotation = if m[0]+m[5]+m[10] > 0. {
        let s = (m[0]+m[5]+m[10]+1.).sqrt()*2.;
        [(m[6]-m[9])/s,(m[8]-m[2])/s,(m[1]-m[4])/s,s*0.25]
    } else if m[0] > m[5] && m[0] > m[10] {
        let s = (1.+m[0]-m[5]-m[10]).sqrt()*2.;
        [s*0.25,(m[4]+m[1])/s,(m[8]+m[2])/s,(m[6]-m[9])/s]
    } else if m[5] > m[10] {
        let s = (1.+m[5]-m[0]-m[10]).sqrt()*2.;
        [(m[4]+m[1])/s,s*0.25,(m[9]+m[6])/s,(m[8]-m[2])/s]
    } else {
        let s = (1.+m[10]-m[0]-m[5]).sqrt()*2.;
        [(m[8]+m[2])/s,(m[9]+m[6])/s,s*0.25,(m[1]-m[4])/s]
    };
    let length = rotation.iter().map(|value| value*value).sum::<f64>().sqrt();
    if !length.is_finite() || length == 0. { return Err("Native brush rotation is invalid".into()); }
    for value in &mut rotation { *value /= length; }
    Ok(([m[12],m[13],m[14]],rotation))
}

fn brushes(world: &Value, collision: &Value) -> Result<(Vec<Value>, Vec<String>)> {
    let models = collision["brush_models"].as_array().ok_or("Native collision brush-model index is missing")?;
    let entities = world["brush_entities"].as_array().ok_or("Native world brush-entity index is missing")?;
    let mut rows = Vec::new();
    let mut classifiers = Vec::new();
    let mut ids = std::collections::BTreeSet::new();
    for entity in entities {
        let id = entity["id"].as_str().ok_or("Native brush entity ID is missing")?;
        if !ids.insert(id) { return Err(format!("Duplicate native brush entity: {id}").into()); }
        let index = entity["native_brush_model"].as_u64().ok_or("Native brush model identity is missing")?;
        let mut matching = models.iter().filter(|model| model["native_brush_model"] == index);
        let model = matching.next().ok_or_else(|| format!("Native brush {id} has no collision submodel *{index}"))?;
        if matching.next().is_some() { return Err(format!("Native brush {id} collision submodel *{index} is ambiguous").into()); }
        let (position, rotation) = brush_transform(&entity["matrix"])?;
        let source = entity["source"].as_object().ok_or("Native brush entity source fields are missing")?;
        let class = source.get("classname").and_then(Value::as_str).ok_or("Native brush classname is missing")?;
        if !matches!(class,"script_brushmodel" | "heli_height_lock" | "trigger_multiple" | "trigger_use" | "trigger_damage" | "trigger_box" | "trigger_lookat" | "info_volume") {
            classifiers.push(format!("environment native classifier for {id} ({class}, *{index})"));
        }
        let volumes = model["volumes"].as_array().ok_or("Native brush convex volumes are missing")?.iter().map(|volume| {
            let native = volume["native_brush"].as_u64().ok_or("Native convex brush identity is missing")?;
            let vertices = volume["vertices"].as_array().ok_or("Native convex brush vertices are missing")?;
            Ok(json!({"id":format!("{id}.convex.{native}"),"position":[0.,0.,0.],"vertices":vertices}))
        }).collect::<Result<Vec<_>>>()?;
        let triangles = model["triangle_shapes"].as_array().ok_or_else(|| format!("Native brush *{index} terrain-shape conversion is missing"))?
            .iter().enumerate().map(|(shape_index, shape)| {
                for key in ["contents","surface_flags","vertices","indices"] {
                    if shape.get(key).is_none() { return Err(format!("Native brush *{index} terrain shape is missing {key}").into()); }
                }
                Ok(json!({"id":format!("{id}.triangle.{shape_index}"),"contents":shape["contents"],
                    "surface_flags":shape["surface_flags"],"vertices":shape["vertices"],"indices":shape["indices"]}))
            }).collect::<Result<Vec<_>>>()?;
        let bounds = model["bounds"].as_array().filter(|bounds| bounds.len() == 2).ok_or("Native brush bounds are missing")?;
        let visual_meshes = entity["visual_meshes"].as_u64().ok_or("Native brush visual mesh count is missing")?;
        rows.push(json!({"id":id,"position":position,"rotation":rotation,"bounds":bounds,"source":source,
            "volumes":volumes,"triangle_shapes":triangles,"visual_meshes":visual_meshes}));
    }
    println!("Joined {} native brush entities to collision submodels; {} source classifications remain unresolved", rows.len(), classifiers.len());
    Ok((rows,classifiers))
}

fn rotated_volumes(row: &Value) -> Result<Vec<Value>> {
    let [x,y,z,w]: [f64;4] = serde_json::from_value(row["rotation"].clone())?;
    let rotate = |value: &Value| -> Result<[f64;3]> {
        let [a,b,c]: [f64;3] = serde_json::from_value(value.clone())?;
        let [tx,ty,tz] = [2.*(y*c-z*b),2.*(z*a-x*c),2.*(x*b-y*a)];
        Ok([a+w*tx+y*tz-z*ty,b+w*ty+z*tx-x*tz,c+w*tz+x*ty-y*tx])
    };
    row["volumes"].as_array().ok_or("Native trigger convex geometry is missing")?.iter().map(|volume| {
        let vertices = volume["vertices"].as_array().ok_or("Native trigger vertices are missing")?.iter().map(rotate).collect::<Result<Vec<_>>>()?;
        Ok(json!({"id":volume["id"],"position":rotate(&volume["position"])? ,"vertices":vertices}))
    }).collect()
}

fn environment(game: &mut Value, rows: Vec<Value>, missing: &mut Vec<String>) -> Result<()> {
    let mut brushes = Vec::new();
    let mut triggers = game["environment"]["triggers"].as_array().cloned().unwrap_or_default();
    let mut owners = game["environment"]["ownership"].as_array().cloned().unwrap_or_default();
    let mut mud = Vec::new();
    for row in rows {
        let source = &row["source"];
        let class = source["classname"].as_str().ok_or("Native environment classname is missing")?;
        let id = row["id"].as_str().ok_or("Native environment identity is missing")?;
        let movement = source["targetname"] == "player_slow_area";
        let boundary = source["script_noteworthy"] == "player_volume";
        let convex_trigger = matches!(class,"trigger_multiple" | "trigger_use" | "trigger_damage" | "trigger_box" | "trigger_lookat" | "info_volume");
        if convex_trigger && row["triangle_shapes"].as_array().is_some_and(Vec::is_empty)
            && row["volumes"].as_array().is_some_and(|volumes| !volumes.is_empty()) {
            let volumes = rotated_volumes(&row)?;
            if movement {
                let position: [f64;3] = serde_json::from_value(row["position"].clone())?;
                for volume in &volumes {
                    let offset: [f64;3] = serde_json::from_value(volume["position"].clone())?;
                    mud.push(json!({"id":format!("mud.{}",volume["id"].as_str().ok_or("Mud volume identity is missing")?),
                        "position":std::array::from_fn::<_,3,_>(|axis| position[axis]+offset[axis]),"vertices":volume["vertices"]}));
                }
            }
            triggers.push(json!({"id":id,"position":row["position"],"source":source,"shape":{"kind":"convex","volumes":volumes}}));
        } else {
            if convex_trigger { missing.push(format!("environment trigger shape for {id} ({class}) requires unsupported terrain or missing convex source geometry")); }
            brushes.push(row.clone());
        }
        if (movement || boundary) && matches!(class,"trigger_multiple" | "info_volume") {
            owners.push(json!({"id":id,"owner":if movement {"movement"} else {"boundary"},"shootable":false}));
        } else {
            missing.push(format!("environment script ownership for {id} ({class})"));
        }
    }
    if game["environment"].is_null() { game["environment"] = json!({}); }
    game["environment"]["brush_entities"] = json!(brushes);
    if !triggers.is_empty() { game["environment"]["triggers"] = json!(triggers); }
    if !owners.is_empty() { game["environment"]["ownership"] = json!(owners); }
    if !mud.is_empty() { game["environment"]["mud"] = json!({"scale":0.6,"stamin_scale":0.7,"step":0.4/4.,"stamin_step":0.3/6.,"interval":0.1,"volumes":mud}); }
    missing.push("environment script-spawned trigger lifecycle conversion".into());
    Ok(())
}

pub fn write((mut game, mut native_gaps): (Value, Vec<String>), worlds: &[Value], collision: &Value, pack: &mut Pack) -> Result<()> {
    let world = worlds.iter().find(|world| world["map"] == "zm_tomb").ok_or("Required Origins world scene is missing")?;
    for (target, source) in [("scene", "uri"), ("collision_scene", "collision_uri")] {
        let uri = world[source].as_str().ok_or_else(|| format!("Prepared world is missing {source}"))?;
        pack.get(uri)?;
        game[target] = json!(format!("{uri}#Scene0"));
    }
    for key in ["source_surfaces", "render_meshes", "collision_meshes", "static_model_instances"] {
        let count = world[key].as_u64().ok_or_else(|| format!("Prepared world is missing {key}"))?;
        game[key] = json!(count);
    }
    let (brushes, mut missing) = brushes(world,collision)?;
    missing.append(&mut native_gaps);
    environment(&mut game,brushes,&mut missing)?;
    for key in ["mud", "triggers", "ownership", "digging", "parts", "crafting_benches"] {
        if game["environment"].get(key).is_none() { missing.push(format!("environment.{key}")); }
    }
    for key in ["doors", "volumes", "spawns"] {
        if game["access"].get(key).is_none() { missing.push(format!("access.{key}")); }
    }
    for key in ["spawn", "yaw", "spawn_points", "waypoints", "survey_views", "rounds", "solo_revive", "environment", "access", "audio", "tuning"] {
        if game.get(key).is_none() { missing.push(key.to_owned()); }
    }
    for (profile, fields) in [("rounds", &["gait_speeds"][..]), ("solo_revive", &["downed_height", "downed_speed"][..])] {
        for field in fields {
            if game[profile].get(field).is_none() { missing.push(format!("{profile}.{field}")); }
        }
    }
    for key in ["barriers", "weapons", "enemies", "perks", "powerups", "zones", "definitions", "placements", "interactions",
        "zone_connections", "navigation_nodes", "navigation_links", "generators", "robot_routes", "tank_routes", "weather", "digging",
        "recipes", "staff_puzzles", "quests", "quest_transitions", "presentations", "sounds", "animations", "ui_assets", "cinematics"] {
        if !game["content"][key].is_array() { missing.push(format!("content.{key}")); }
    }
    if !missing.is_empty() {
        return Err(format!("Required game.json source conversion is incomplete: {}. No runtime pack was written.", missing.join(", ")).into());
    }
    pack.entries.insert("game.json".into(), Arc::new(serde_json::to_vec(&game)?));
    Ok(())
}

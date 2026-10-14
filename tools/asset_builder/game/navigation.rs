use super::*;
use std::collections::{BTreeMap, BTreeSet};

fn position([x, y, z]: [f32; 3]) -> [f32; 3] {
    [x * 0.0254, z * 0.0254, -y * 0.0254]
}

fn entity_position(entity: &crate::entities::Entity) -> Result<[f32; 3]> {
    let values = entity
        .get("origin")
        .ok_or("Native spawn struct has no origin")?
        .split_whitespace()
        .map(str::parse)
        .collect::<std::result::Result<Vec<f32>, _>>()?;
    Ok(position(
        values
            .try_into()
            .map_err(|_| "Native spawn origin is not a vec3")?,
    ))
}

fn target_origin(
    entity: &crate::entities::Entity,
    entities: &[crate::entities::Entity],
    nodes: &[Value],
) -> Result<Option<[f32; 3]>> {
    let Some(target) = entity.get("target").filter(|target| !target.is_empty()) else {
        return Ok(None);
    };
    for structs in [false, true] {
        let matching = entities
            .iter()
            .filter(|entity| {
                entity.get("targetname") == Some(target)
                    && entity
                        .get("classname")
                        .is_some_and(|class| (class == "script_struct") == structs)
            })
            .collect::<Vec<_>>();
        if matching.len() > 1 {
            return Err(
                format!("Initial spawn target {target} resolves multiple source entities").into(),
            );
        }
        if let Some(entity) = matching.first() {
            return entity_position(entity).map(Some);
        }
    }
    let matching = nodes
        .iter()
        .filter(|node| node["targetname"] == *target)
        .collect::<Vec<_>>();
    if matching.len() != 1 {
        return Err(format!(
            "Initial spawn target {target} resolves {} native nodes",
            matching.len()
        )
        .into());
    }
    Ok(Some(serde_json::from_value(
        matching[0]["position"].clone(),
    )?))
}

fn entrance_choices(
    goals: &[Value],
    origin: [f32; 3],
    desired: Option<[f32; 3]>,
) -> Result<Vec<usize>> {
    let center = desired.unwrap_or(origin);
    let mut indices = (0..goals.len()).collect::<Vec<_>>();
    let distance = |index: usize, point: [f32; 3]| -> Result<f32> {
        let goal: [f32; 3] = serde_json::from_value(goals[index]["position"].clone())?;
        let delta = difference(goal, point);
        Ok(dot(delta, delta).sqrt())
    };
    let distances = indices
        .iter()
        .map(|&index| distance(index, center))
        .collect::<Result<Vec<_>>>()?;
    indices.sort_by(|a, b| distances[*a].total_cmp(&distances[*b]));
    indices.truncate(if desired.is_some() { 1 } else { 3 });
    if desired.is_none()
        && let Some(&first) = indices.first()
    {
        let mut previous = distance(first, origin)?;
        let mut count = 1;
        for &index in indices.iter().skip(1) {
            let current = distance(index, origin)?;
            if current - previous > 500. * 0.0254 {
                break;
            }
            previous = current;
            count += 1;
        }
        indices.truncate(count);
    }
    Ok(indices)
}

struct ZoneVolume {
    zone: String,
    minimum: [f32; 3],
    maximum: [f32; 3],
    planes: Vec<([f32; 3], f32)>,
}

fn difference(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    std::array::from_fn(|axis| a[axis] - b[axis])
}

fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    a.into_iter().zip(b).map(|(a, b)| a * b).sum()
}

impl ZoneVolume {
    fn read(row: &Value) -> Result<Self> {
        let zone = row["zone"]
            .as_str()
            .ok_or("Native zone volume has no zone")?
            .to_owned();
        let offset: [f32; 3] = serde_json::from_value(row["volume"]["position"].clone())?;
        let vertices: Vec<[f32; 3]> = serde_json::from_value(row["volume"]["vertices"].clone())?;
        let vertices: Vec<[f32; 3]> = vertices
            .into_iter()
            .map(|point| std::array::from_fn(|axis| point[axis] + offset[axis]))
            .collect();
        let minimum = std::array::from_fn(|axis| {
            vertices
                .iter()
                .map(|p| p[axis])
                .fold(f32::INFINITY, f32::min)
        });
        let maximum = std::array::from_fn(|axis| {
            vertices
                .iter()
                .map(|p| p[axis])
                .fold(f32::NEG_INFINITY, f32::max)
        });
        let mut planes = Vec::<([f32; 3], f32)>::new();
        for a in 0..vertices.len() {
            for b in a + 1..vertices.len() {
                for c in b + 1..vertices.len() {
                    let u = difference(vertices[b], vertices[a]);
                    let v = difference(vertices[c], vertices[a]);
                    let mut normal = [
                        u[1] * v[2] - u[2] * v[1],
                        u[2] * v[0] - u[0] * v[2],
                        u[0] * v[1] - u[1] * v[0],
                    ];
                    let length = dot(normal, normal).sqrt();
                    if length < 0.00001 {
                        continue;
                    }
                    normal.iter_mut().for_each(|value| *value /= length);
                    let mut distance = dot(normal, vertices[a]);
                    if vertices
                        .iter()
                        .all(|point| dot(normal, *point) >= distance - 0.0001)
                    {
                        normal.iter_mut().for_each(|value| *value = -*value);
                        distance = -distance;
                    } else if vertices
                        .iter()
                        .any(|point| dot(normal, *point) > distance + 0.0001)
                    {
                        continue;
                    }
                    if !planes
                        .iter()
                        .any(|(n, d)| dot(*n, normal) > 0.99999 && (*d - distance).abs() < 0.0001)
                    {
                        planes.push((normal, distance));
                    }
                }
            }
        }
        if planes.len() < 4 {
            return Err("Native navigation zone has degenerate convex geometry".into());
        }
        Ok(Self {
            zone,
            minimum,
            maximum,
            planes,
        })
    }

    fn contains(&self, point: [f32; 3]) -> bool {
        (0..3).all(|axis| {
            point[axis] >= self.minimum[axis] - 0.0001 && point[axis] <= self.maximum[axis] + 0.0001
        }) && self
            .planes
            .iter()
            .all(|(normal, distance)| dot(*normal, point) <= distance + 0.0001)
    }
}

pub(super) fn zone_at(game: &Value, position: [f32; 3]) -> Result<Option<String>> {
    for row in game["access"]["volumes"]
        .as_array()
        .ok_or("Native zone volumes are missing")?
    {
        let volume = ZoneVolume::read(row)?;
        if volume.contains(position) {
            return Ok(Some(volume.zone));
        }
    }
    Ok(None)
}

pub(super) fn read(catalog: &Catalog<'_>, game: &mut Value) -> Result<()> {
    let worlds: Vec<_> = catalog.definitions(15).collect();
    if worlds.len() != 1 {
        return Err(format!(
            "Initial navigation requires one native GameWorldMP, found {}",
            worlds.len()
        )
        .into());
    }
    let (_, zone, root) = worlds[0];
    let streams = &catalog.zones[zone].1.streams;
    let path = root.add(4)?;
    let count = streams.u32(path)? as usize;
    if count == 0 || count >= u16::MAX as usize {
        return Err("Native Origins path node count is outside the supported source range".into());
    }
    let nodes = Address::decode(streams.u32(path.add(8)?)?)?;
    let mut records = Vec::with_capacity(count);
    for index in 0..count {
        let at = nodes.add(index * 144)?;
        let scalar = |offset| -> Result<f32> { Ok(f32::from_bits(streams.u32(at.add(offset)?)?)) };
        let script = |offset| -> Result<String> {
            let index = streams.u16(at.add(offset)?)? as usize;
            if index == 0 {
                return Ok(String::new());
            }
            let address = streams
                .script_strings
                .get(index)
                .copied()
                .flatten()
                .ok_or_else(|| {
                    format!("Native path node {index} has an unresolved script string")
                })?;
            Ok(streams.text(address)?)
        };
        let kind = streams.u32(at)?;
        let point = position([scalar(24)?, scalar(28)?, scalar(32)?]);
        let yaw = scalar(36)?.to_radians();
        if !point.iter().all(|x| x.is_finite()) || !yaw.is_finite() || kind > 21 {
            return Err(format!("Native path node {index} has invalid source constants").into());
        }
        let links_count = streams.u16(at.add(60)?)? as usize;
        let mut links = Vec::with_capacity(links_count);
        if links_count > 0 {
            let links_at = Address::decode(streams.u32(at.add(64)?)?)?;
            for ordinal in 0..links_count {
                let link = links_at.add(ordinal * 16)?;
                let to = streams.u16(link.add(4)?)? as usize;
                let cost = f32::from_bits(streams.u32(link)?);
                if to >= count || !cost.is_finite() || cost <= 0. {
                    return Err(format!(
                        "Native path link {index}:{ordinal} has invalid target/cost"
                    )
                    .into());
                }
                links.push(json!({"to":to,"cost":cost * 0.0254,
                    "disconnect_count":streams.u8(link.add(6)?)?,
                    "negotiation":streams.u8(link.add(7)?)? != 0,
                    "flags":streams.u8(link.add(8)?)?}));
            }
        }
        records.push(json!({"index":index,"type":kind,"spawn_flags":streams.u32(at.add(4)?)? as i32,
            "position":point,"yaw":yaw,"forward":[scalar(40)?,0.,-scalar(44)?],
            "radius":scalar(48)? * 0.0254,"minimum_use_distance_squared":scalar(52)? * 0.0254_f32.powi(2),
            "overlap":[streams.u16(at.add(56)?)? as i16,streams.u16(at.add(58)?)? as i16],
            "targetname":script(8)?,"link_name":script(10)?,"noteworthy":script(12)?,
            "target":script(14)?,"animscript":script(16)?,"links":links}));
    }
    game["_native_navigation"] = json!(records);
    Ok(())
}

pub(super) fn prepare(
    game: &mut Value,
    entities: &[crate::entities::Entity],
    missing: &mut Vec<String>,
) -> Result<()> {
    let nodes = game["_native_navigation"]
        .as_array()
        .ok_or("Native path source was not read")?
        .clone();
    let volumes = game["access"]["volumes"]
        .as_array()
        .ok_or("Native navigation requires source zone volumes")?
        .iter()
        .map(ZoneVolume::read)
        .collect::<Result<Vec<_>>>()?;
    let mut enabled: BTreeSet<String> = game["content"]["zones"]
        .as_array()
        .ok_or("Source zone catalog is absent")?
        .iter()
        .filter(|zone| zone["initially_open"] == true)
        .map(|zone| {
            zone["id"]
                .as_str()
                .ok_or("Source initial zone lacks identity")
                .map(str::to_owned)
        })
        .collect::<std::result::Result<_, _>>()?;
    let connections = game["content"]["zone_connections"]
        .as_array()
        .ok_or("Source zone connections are absent")?
        .clone();
    loop {
        let count = enabled.len();
        for connection in &connections {
            if connection["initially_open"] != true {
                continue;
            }
            let from = connection["from"]
                .as_str()
                .ok_or("Source connection lacks from zone")?;
            let to = connection["to"]
                .as_str()
                .ok_or("Source connection lacks to zone")?;
            if enabled.contains(from) || enabled.contains(to) {
                enabled.insert(from.to_owned());
                enabled.insert(to.to_owned());
            }
        }
        if count == enabled.len() {
            break;
        }
    }
    if enabled
        != BTreeSet::from([
            "zone.start".to_owned(),
            "zone.start_a".to_owned(),
            "zone.start_b".to_owned(),
        ])
    {
        return Err(format!("Recovered initial-zone closure changed: {enabled:?}").into());
    }
    let research = std::path::PathBuf::from(
        std::env::var_os("USERPROFILE").ok_or("USERPROFILE is unavailable")?,
    )
    .join("Documents/T6ZMResearch/t6zm-master/source");
    let manager =
        std::fs::read_to_string(research.join("server-patch/maps/mp/zombies/_zm_zonemgr.gsc"))?;
    for expression in [
        "spots = getstructarray( zone.volumes[0].target, \"targetname\" );",
        "tokens = strtok( spots[i].script_noteworthy, \" \" );",
        "if ( zone.is_enabled && zone.is_active && zone.is_spawning_allowed )",
        "if ( zone.spawn_locations[i].is_enabled )",
    ] {
        if !manager.contains(expression) {
            return Err(
                format!("Recovered initial spawn eligibility changed: {expression}").into(),
            );
        }
    }
    let mut spawns = Vec::new();
    for zone in &enabled {
        let source_zone = format!(
            "zone_{}",
            zone.strip_prefix("zone.")
                .ok_or("Native zone identity is unsupported")?
        );
        let volume = entities
            .iter()
            .find(|entity| {
                entity
                    .get("classname")
                    .is_some_and(|class| class == "info_volume")
                    && entity.get("targetname") == Some(&source_zone)
            })
            .ok_or_else(|| format!("Initial source zone has no info_volume: {source_zone}"))?;
        let target = volume
            .get("target")
            .ok_or_else(|| format!("Initial source volume has no spawn target: {source_zone}"))?;
        for (index, entity) in entities.iter().enumerate().filter(|(_, entity)| {
            entity
                .get("classname")
                .is_some_and(|class| class == "script_struct")
                && entity.get("targetname") == Some(target)
        }) {
            if entity.get("is_blocked").is_some_and(|value| value != "0") {
                continue;
            }
            let tokens: Vec<_> = entity
                .get("script_noteworthy")
                .ok_or("Initial spawn has no source category")?
                .split_whitespace()
                .collect();
            let special = [
                "dog_location",
                "screecher_location",
                "avogadro_location",
                "inert_location",
                "quad_location",
                "leaper_location",
                "brutus_location",
                "mechz_location",
                "astro_location",
                "napalm_location",
            ];
            let ordinary: Vec<_> = tokens
                .iter()
                .filter(|token| !special.contains(token))
                .collect();
            if ordinary.is_empty() {
                continue;
            }
            if ordinary.len() != 1
                || !matches!(*ordinary[0], "spawn_location" | "riser_location")
                || entity.contains_key("script_string")
                || entity.contains_key("script_int")
            {
                return Err(format!(
                    "Initial spawn {index} has unsupported authored entry conditions: {entity:?}"
                )
                .into());
            }
            spawns.push(json!({"id":format!("spawn.native.{index}"),"zone":zone,"position":entity_position(entity)?,"rising":*ordinary[0] == "riser_location","yaw":super::player::pose(entity)?.1,"desired_origin":target_origin(entity,entities,&nodes)?}));
        }
    }
    if spawns.is_empty() {
        return Err("Source initial zone closure contains no ordinary spawn locations".into());
    }
    let spawners = entities.iter().enumerate().filter(|(_,entity)| {
        entity.get("script_noteworthy").is_some_and(|name| name=="zombie_spawner")
            && entity.get("script_int").is_some_and(|number| number=="1")
    }).map(|(index,entity)| {
        if entity.get("script_forcespawn").is_none_or(|value| value != "1") {
            return Err(format!("Initial ordinary spawner requires its native force-spawn branch: {entity:?}").into());
        }
        let (position,yaw)=super::player::pose(entity)?;
        Ok(json!({"id":format!("spawner.native.{index}"),"archetype":entity.get("classname").ok_or("Native ordinary spawner has no class")?,"position":position,"yaw":yaw,"desired_origin":target_origin(entity,entities,&nodes)?}))
    }).collect::<Result<Vec<_>>>()?;
    if spawners.is_empty() {
        return Err("Initial ordinary source spawner pool is empty".into());
    }
    let positions: Vec<[f32; 3]> = nodes
        .iter()
        .map(|node| serde_json::from_value(node["position"].clone()))
        .collect::<std::result::Result<_, _>>()?;
    let membership: Vec<Option<String>> = positions
        .iter()
        .map(|point| {
            volumes
                .iter()
                .find(|volume| volume.contains(*point))
                .map(|volume| volume.zone.clone())
        })
        .collect();
    let mut selected: BTreeMap<usize, String> = membership
        .iter()
        .enumerate()
        .filter_map(|(index, zone)| {
            zone.as_ref()
                .filter(|zone| enabled.contains(*zone))
                .map(|zone| (index, zone.clone()))
        })
        .collect();
    let adjacency: Vec<Vec<usize>> = nodes
        .iter()
        .map(|node| {
            node["links"]
                .as_array()
                .ok_or("Native node links are absent")?
                .iter()
                .filter(|link| link["disconnect_count"] == 0)
                .map(|link| {
                    link["to"]
                        .as_u64()
                        .map(|to| to as usize)
                        .ok_or("Native path link target is absent")
                })
                .collect::<std::result::Result<_, _>>()
        })
        .collect::<std::result::Result<_, _>>()?;
    let mut incoming = vec![Vec::new(); nodes.len()];
    for (from, edges) in adjacency.iter().enumerate() {
        for &to in edges {
            incoming[to].push(from);
        }
    }
    let mut reaches_initial = vec![false; nodes.len()];
    let mut work: Vec<_> = selected.keys().copied().collect();
    for &node in &work {
        reaches_initial[node] = true;
    }
    while let Some(node) = work.pop() {
        for &from in &incoming[node] {
            if !reaches_initial[from]
                && membership[from]
                    .as_ref()
                    .is_none_or(|zone| enabled.contains(zone))
            {
                reaches_initial[from] = true;
                work.push(from);
            }
        }
    }
    for spawn in &spawns {
        let point: [f32; 3] = serde_json::from_value(spawn["position"].clone())?;
        let seed = positions
            .iter()
            .enumerate()
            .filter(|(index, _)| nodes[*index]["type"] == 1)
            .min_by(|(_, a), (_, b)| {
                dot(difference(**a, point), difference(**a, point))
                    .total_cmp(&dot(difference(**b, point), difference(**b, point)))
            })
            .map(|(index, _)| index)
            .ok_or("Source spawn cannot resolve a native path node")?;
        if !reaches_initial[seed] {
            return Err(format!(
                "Source {} cannot reach initial player volumes through native path links",
                spawn["id"]
            )
            .into());
        }
        let mut visited = BTreeSet::new();
        let mut work = vec![seed];
        while let Some(node) = work.pop() {
            if !visited.insert(node) || !reaches_initial[node] {
                continue;
            }
            if membership[node]
                .as_ref()
                .is_some_and(|zone| enabled.contains(zone))
            {
                continue;
            }
            selected
                .entry(node)
                .or_insert_with(|| spawn["zone"].as_str().unwrap().to_owned());
            work.extend(adjacency[node].iter().copied());
        }
    }
    let goals = entities
        .iter()
        .filter(|entity| {
            entity
                .get("targetname")
                .is_some_and(|name| name == "exterior_goal")
        })
        .map(|entity| {
            Ok(json!({"position":entity_position(entity)?,"target":entity.get("target")}))
        })
        .collect::<Result<Vec<_>>>()?;
    let mut required_goals = BTreeSet::new();
    for spawn in &spawns {
        let origin: [f32; 3] = serde_json::from_value(spawn["position"].clone())?;
        for spawner in &spawners {
            let desired = if spawn["desired_origin"].is_null() {
                &spawner["desired_origin"]
            } else {
                &spawn["desired_origin"]
            };
            let desired: Option<[f32; 3]> = serde_json::from_value(desired.clone())?;
            required_goals.extend(entrance_choices(&goals, origin, desired)?);
        }
    }
    let entrances = required_goals.into_iter().map(|goal| {
        let target=goals[goal]["target"].as_str().ok_or("Initial exterior goal has no native barrier target")?;
        let matching=nodes.iter().enumerate().filter(|(_,node)|node["type"]==17 && node["targetname"]==target).collect::<Vec<_>>();
        let [(index,_)] = matching.as_slice() else {return Err(format!("Initial exterior goal {target} has {} native Begin endpoints",matching.len()).into());};
        if !selected.contains_key(index) {return Err(format!("Source initial spawn selects exterior goal {target} outside the retained native navigation closure").into());}
        Ok(json!({"node":format!("node.native.{index}"),"position":goals[goal]["position"]}))
    }).collect::<Result<Vec<_>>>()?;
    if entrances.is_empty() {
        return Err("Initial ordinary source entrance pool is empty".into());
    }
    game["access"]["spawners"] = json!(spawners);
    game["access"]["entrances"] = json!(entrances);
    let mut links = Vec::new();
    let mut aliases = BTreeSet::new();
    for (&from, _) in &selected {
        for (ordinal, link) in nodes[from]["links"]
            .as_array()
            .ok_or("Native node links are absent")?
            .iter()
            .enumerate()
        {
            let to = link["to"]
                .as_u64()
                .ok_or("Native path link target is absent")? as usize;
            if !selected.contains_key(&to) || link["disconnect_count"] != 0 {
                continue;
            }
            let mut row = json!({"id":format!("link.native.{from}.{ordinal}"),"from":format!("node.native.{from}"),"to":format!("node.native.{to}"),
                "traversal":"walk","cost":link["cost"],"connection":null});
            if link["negotiation"] == true {
                if nodes[from]["type"] != 17 || nodes[to]["type"] != 18 {
                    return Err(format!(
                        "Native negotiation {from}:{ordinal} does not join Begin to End"
                    )
                    .into());
                }
                let script = nodes[from]["animscript"]
                    .as_str()
                    .ok_or("Native negotiation animscript is absent")?;
                if !script
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
                    || script.is_empty()
                {
                    return Err(
                        format!("Native negotiation script name is unsupported: {script}").into(),
                    );
                }
                let script_path = format!("maps/mp/animscripts/traverse/{script}.gsc");
                let path = [
                    research.join("server-tomb").join(&script_path),
                    research.join("server-patch").join(&script_path),
                ]
                .into_iter()
                .find(|path| path.is_file())
                .ok_or_else(|| format!("Recovered native traversal script is missing: {script}"))?;
                let source = std::fs::read_to_string(path)?;
                let alias = if script == "zm_mantle_over_40"
                    && ["barrier_walk", "barrier_run", "barrier_sprint"]
                        .iter()
                        .all(|alias| source.contains(&format!("\"{alias}\"")))
                {
                    script.to_owned()
                } else {
                    let call = source
                        .split_once("dosimpletraverse(")
                        .ok_or_else(|| {
                            format!("Native traversal {script} requires explicit source conversion")
                        })?
                        .1
                        .trim_start();
                    let call = call
                        .strip_prefix('"')
                        .ok_or("Native traversal alias is not literal")?;
                    call.split_once('"')
                        .ok_or("Native traversal alias is unterminated")?
                        .0
                        .to_owned()
                };
                aliases.insert(alias.clone());
                let endpoint = |index: usize| {
                    let mut node = nodes[index].clone();
                    node.as_object_mut().unwrap().remove("links");
                    node.as_object_mut().unwrap().remove("animscript");
                    node
                };
                row["traversal"] = json!(if alias.starts_with("jump_") {
                    "jump"
                } else {
                    "climb"
                });
                row["source_animation"] = json!(alias);
                row["negotiation"] = json!({"cost":link["cost"],"flags":link["flags"],"disconnect_count":link["disconnect_count"],"start":endpoint(from),"end":endpoint(to)});
            }
            links.push(row);
        }
    }
    game["content"]["navigation_nodes"] = json!(selected.iter().map(|(index,zone)|json!({"id":format!("node.native.{index}"),"position":positions[*index],"zone":zone,"native":{"type":nodes[*index]["type"],"spawn_flags":nodes[*index]["spawn_flags"]}})).collect::<Vec<_>>());
    game["content"]["navigation_links"] = json!(links);
    game["access"]["spawns"] = json!(spawns);
    game["content"]["zone_connections"] = json!(
        connections
            .into_iter()
            .filter(|connection| connection["from"]
                .as_str()
                .is_some_and(|zone| enabled.contains(zone))
                && connection["to"]
                    .as_str()
                    .is_some_and(|zone| enabled.contains(zone)))
            .collect::<Vec<_>>()
    );
    let connection_ids: BTreeSet<_> = game["content"]["zone_connections"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|connection| connection["id"].as_str())
        .map(str::to_owned)
        .collect();
    for link in game["access"]["flag_links"]
        .as_array_mut()
        .into_iter()
        .flatten()
    {
        link["connections"]
            .as_array_mut()
            .ok_or("Source flag connection list is absent")?
            .retain(|id| id.as_str().is_some_and(|id| connection_ids.contains(id)));
    }
    game["access"]["flag_links"]
        .as_array_mut()
        .ok_or("Source flag links are absent")?
        .retain(|link| {
            link["connections"]
                .as_array()
                .is_some_and(|connections| !connections.is_empty())
        });
    let flags: BTreeSet<_> = game["access"]["flag_links"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|link| link["flag"].as_str())
        .map(str::to_owned)
        .collect();
    game["access"]["trigger_activations"]
        .as_array_mut()
        .ok_or("Source trigger activation list is absent")?
        .retain(|activation| {
            activation["flags"].as_array().is_some_and(|values| {
                values
                    .iter()
                    .all(|flag| flag.as_str().is_some_and(|flag| flags.contains(flag)))
            })
        });
    game.as_object_mut()
        .ok_or("Native game data is not an object")?
        .remove("_native_navigation");
    missing.retain(|gap| gap != "access source zone events, spawn enablement, barriers and remaining flag producers require conversion");
    println!(
        "Converted initial source navigation: {} authored nodes, {} directed links, {} spawn structs, {} enabled zones; traversal actions {aliases:?}",
        selected.len(),
        game["content"]["navigation_links"]
            .as_array()
            .unwrap()
            .len(),
        game["access"]["spawns"].as_array().unwrap().len(),
        enabled.len()
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn convex_zone_membership_preserves_authored_sloped_faces() {
        let volume = ZoneVolume::read(&json!({"zone":"zone.start","volume":{"position":[5.,0.,0.],"vertices":[[0.,0.,0.],[1.,0.,0.],[0.,1.,0.],[0.,0.,1.]]}})).unwrap();
        assert!(volume.contains([5.2, 0.2, 0.2]));
        assert!(!volume.contains([5.8, 0.8, 0.8]));
        assert!(!volume.contains([0.2, 0.2, 0.2]));
    }

    #[test]
    #[ignore = "requires the authorized Steam installation"]
    fn installed_initial_navigation_source() {
        let zones = crate::source::read_zones(&crate::source::installation().unwrap()).unwrap();
        let catalog = Catalog::new(&zones);
        let entities = crate::entities::read(&catalog).unwrap();
        let mut game = json!({});
        read(&catalog, &mut game).unwrap();
        let native = game["_native_navigation"].as_array().unwrap().clone();
        let collision = crate::collision::convert(&catalog, &mut Pack::default()).unwrap();
        let world = json!({"brush_entities":crate::scenes::brush_entities(&entities).unwrap()});
        let (brushes, mut missing) = super::super::triggers::brushes(&world, &collision).unwrap();
        super::super::triggers::environment(&mut game, brushes, &mut missing).unwrap();
        super::super::triggers::zone_graph(&mut game, &entities, &mut missing).unwrap();
        prepare(&mut game, &entities, &mut missing).unwrap();
        assert!(game.get("_native_navigation").is_none());
        let spawns = game["access"]["spawns"].as_array().unwrap();
        assert_eq!(spawns.len(), 12);
        assert_eq!(
            spawns
                .iter()
                .filter(|spawn| spawn["rising"] == true)
                .count(),
            3
        );
        for spawn in spawns {
            let index = spawn["id"]
                .as_str()
                .unwrap()
                .strip_prefix("spawn.native.")
                .unwrap()
                .parse::<usize>()
                .unwrap();
            assert_eq!(
                spawn["position"],
                json!(entity_position(&entities[index]).unwrap())
            );
        }
        let nodes = game["content"]["navigation_nodes"].as_array().unwrap();
        let links = game["content"]["navigation_links"].as_array().unwrap();
        assert_eq!(nodes.len(), 80);
        assert_eq!(links.len(), 562);
        for node in nodes {
            let index: usize = node["id"]
                .as_str()
                .unwrap()
                .rsplit('.')
                .next()
                .unwrap()
                .parse()
                .unwrap();
            assert_eq!(node["position"], native[index]["position"]);
            assert_eq!(node["native"]["type"], native[index]["type"]);
            assert_eq!(node["native"]["spawn_flags"], native[index]["spawn_flags"]);
        }
        assert!(nodes.iter().all(|node| matches!(
            node["zone"].as_str(),
            Some("zone.start" | "zone.start_a" | "zone.start_b")
        )));
        let ids: BTreeSet<_> = nodes
            .iter()
            .map(|node| node["id"].as_str().unwrap())
            .collect();
        for link in links {
            assert!(ids.contains(link["from"].as_str().unwrap()));
            assert!(ids.contains(link["to"].as_str().unwrap()));
            let parts: Vec<_> = link["id"].as_str().unwrap().split('.').collect();
            let from: usize = parts[2].parse().unwrap();
            let ordinal: usize = parts[3].parse().unwrap();
            assert_eq!(link["cost"], native[from]["links"][ordinal]["cost"]);
        }
        assert_eq!(
            game["content"]["zone_connections"]
                .as_array()
                .unwrap()
                .len(),
            3
        );
        let negotiations: Vec<_> = links
            .iter()
            .filter(|link| link["negotiation"].is_object())
            .collect();
        assert_eq!(negotiations.len(), 4);
        for link in &negotiations {
            assert_eq!(link["source_animation"], "zm_mantle_over_40");
            assert_eq!(
                link["negotiation"]["start"]["target"],
                link["negotiation"]["end"]["targetname"]
            );
            for endpoint in ["start", "end"] {
                let node = &link["negotiation"][endpoint];
                let yaw = node["yaw"].as_f64().unwrap();
                let forward: [f64; 3] = serde_json::from_value(node["forward"].clone()).unwrap();
                assert!(
                    (forward[0] - yaw.cos()).abs() < 0.0001
                        && (forward[2] + yaw.sin()).abs() < 0.0001,
                    "Native {} forward is inconsistent with source yaw: {node}",
                    link["id"]
                );
            }
        }
        println!(
            "Initial source conversion retains {} nodes and {} links with {} negotiations",
            nodes.len(),
            links.len(),
            negotiations.len()
        );
    }
}

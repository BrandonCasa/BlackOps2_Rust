use super::*;
use std::collections::{BTreeMap, BTreeSet};

fn state_source(catalog: &Catalog<'_>) -> Result<String> {
    raw_source(catalog, "animstatedefs/zm_tomb_basic.asd")
}

fn raw_source(catalog: &Catalog<'_>, name: &str) -> Result<String> {
    let (zone, root) = catalog.resolve(41, name)?;
    let streams = &catalog.zones[zone].1.streams;
    let length = streams.u32(root.add(4)?)? as usize;
    let data = Address::decode(streams.u32(root.add(8)?)?)?;
    Ok(std::str::from_utf8(streams.read(data, length)?)?.to_owned())
}

pub(super) fn gib_piece(catalog: &Catalog<'_>, name: &str, joint: &str) -> Result<Value> {
    let (zone, root) = catalog.resolve(5, name)?;
    let streams = &catalog.zones[zone].1.streams;
    let number = |streams: &crate::fastfile::Streams, field: Address| -> Result<f32> {
        let value = f32::from_bits(streams.u32(field)?);
        if !value.is_finite() {
            return Err("Native gib physics contains a nonfinite float".into());
        }
        Ok(value)
    };
    let vector = |streams: &crate::fastfile::Streams, field: Address| -> Result<[f32; 3]> {
        Ok([
            number(streams, field)?,
            number(streams, field.add(8)?)?,
            -number(streams, field.add(4)?)?,
        ]
        .map(|value| value * 0.0254))
    };
    let (_, preset_zone, preset) = catalog.pointer(zone, root.add(216)?, 1)?;
    let preset_streams = &catalog.zones[preset_zone].1.streams;
    let count = streams.u8(root.add(220)?)?;
    if count == 0 {
        return Err(format!("Native gib {name} has no source collision map").into());
    }
    let maps = Address::decode(streams.u32(root.add(224)?)?)?;
    let map = Address::decode(streams.u32(maps)?)?;
    let count = streams.u32(map)? as usize;
    let array = Address::decode(streams.u32(map.add(4)?)?)?;
    let mut shapes = Vec::new();
    for index in 0..count {
        let geometry = array.add(index * 68)?;
        let brush = streams.u32(geometry)?;
        if brush == 0 {
            return Err(format!(
                "Native gib {name} requires non-brush physics geometry conversion"
            )
            .into());
        }
        let brush = Address::decode(brush)?;
        let count = streams.u32(brush.add(84)?)? as usize;
        let points = if streams.u32(brush.add(28)?)? == 0 {
            let low = vector(streams, brush)?;
            let high = vector(streams, brush.add(16)?)?;
            (0..8)
                .map(|corner| {
                    std::array::from_fn(|axis| {
                        if corner & (1 << axis) == 0 {
                            low[axis].min(high[axis])
                        } else {
                            low[axis].max(high[axis])
                        }
                    })
                })
                .collect::<Vec<[f32; 3]>>()
        } else {
            let vertices = Address::decode(streams.u32(brush.add(88)?)?)?;
            (0..count)
                .map(|index| vector(streams, vertices.add(index * 12)?))
                .collect::<Result<Vec<_>>>()?
        };
        if points.len() < 4 {
            return Err(format!("Native gib {name} has insufficient collision vertices").into());
        }
        shapes.push(points);
    }
    if shapes.is_empty() {
        return Err(format!("Native gib {name} has empty collision geometry").into());
    }
    let mut low = [f32::INFINITY; 3];
    let mut high = [f32::NEG_INFINITY; 3];
    for point in shapes.iter().flatten() {
        for axis in 0..3 {
            low[axis] = low[axis].min(point[axis]);
            high[axis] = high[axis].max(point[axis]);
        }
    }
    let offset = vector(preset_streams, preset.add(48)?)?;
    let center: [f32; 3] =
        std::array::from_fn(|axis| (low[axis] + high[axis]) * 0.5 + offset[axis]);
    Ok(
        json!({"presentation":format!("actor.{name}"),"joint":joint,"physics":{
        "mass":number(preset_streams,preset.add(8)?)?,"bounce":number(preset_streams,preset.add(12)?)?,
        "friction":number(preset_streams,preset.add(16)?)?,"bullet_force_scale":number(preset_streams,preset.add(20)?)?,"explosive_force_scale":number(preset_streams,preset.add(24)?)?,
        "gravity_scale":number(preset_streams,preset.add(44)?)?,"center":center,"shapes":shapes}}),
    )
}

fn parse_states(source: &str) -> Result<BTreeMap<String, Vec<(String, String)>>> {
    let mut states = BTreeMap::new();
    let mut current: Option<(String, bool, Vec<(String, String)>)> = None;
    let mut opened = false;
    for line in source.lines() {
        let line = line.split_once("//").map_or(line, |(line, _)| line).trim();
        if line.is_empty() {
            continue;
        }
        if line == "{" {
            if current.is_none() || opened {
                return Err("Native ordinary ASD has an unexpected opening brace".into());
            }
            opened = true;
        } else if line == "}" {
            if !opened {
                return Err("Native ordinary ASD has an unexpected closing brace".into());
            }
            let (name, _, clips) = current
                .take()
                .ok_or("Native ordinary ASD has no state header")?;
            if clips.is_empty() || states.insert(name, clips).is_some() {
                return Err("Native ordinary ASD has an empty or duplicate state".into());
            }
            opened = false;
        } else if opened {
            let (_, aliased, clips) = current
                .as_mut()
                .ok_or("Native ordinary ASD lost its state")?;
            let words = line.split_whitespace().collect::<Vec<_>>();
            let expected = if *aliased { 2 } else { 1 };
            if words.len() != expected {
                return Err(format!(
                    "Native ordinary ASD state has {line:?} instead of {expected} substate fields"
                )
                .into());
            }
            clips.push((
                if *aliased {
                    words[0].to_owned()
                } else {
                    String::new()
                },
                words[expected - 1].to_owned(),
            ));
        } else {
            if current.is_some() {
                return Err("Native ordinary ASD state header lacks an opening brace".into());
            }
            let words = line.split_whitespace().collect::<Vec<_>>();
            let name = words
                .first()
                .ok_or("Native ordinary ASD state name is missing")?;
            if words.get(1).is_some_and(|word| *word != ":") {
                return Err(
                    "Native ordinary ASD state has an unsupported property declaration".into(),
                );
            }
            let mut flags = words.iter().skip(2);
            while let Some(flag) = flags.next() {
                match *flag {
                    "restart" | "aliased" | "missing_legs" => {}
                    "notify" => {
                        if flags.next().is_none() {
                            return Err(
                                "Native ordinary ASD notify property has no event name".into()
                            );
                        }
                    }
                    _ => {
                        return Err(format!(
                            "Native ordinary ASD has an unknown source property {flag}"
                        )
                        .into());
                    }
                }
            }
            current = Some((name.to_string(), words.contains(&"aliased"), Vec::new()));
        }
    }
    if opened || current.is_some() {
        return Err("Native ordinary ASD ends inside a state".into());
    }
    Ok(states)
}

pub(super) fn state_clips(catalog: &Catalog<'_>, state: &str) -> Result<Vec<(String, String)>> {
    parse_states(&state_source(catalog)?)?
        .remove(state)
        .ok_or_else(|| format!("Required ordinary native ASD state is missing: {state}").into())
}

pub(super) fn ragdoll_profile(catalog: &Catalog<'_>) -> Result<Value> {
    let source = raw_source(catalog, "ragdoll.cfg")?;
    let empty = || json!({"bones":[],"joints":[],"baselerp":[],"self_pairs":[]});
    let mut profiles = [empty(), empty()];
    for line in source.trim_end_matches('\0').lines() {
        let line = line.split_once("//").map_or(line, |(line, _)| line).trim();
        if line.is_empty() {
            continue;
        }
        let words = line.split_whitespace().collect::<Vec<_>>();
        let command = words[0];
        let profile: usize = words
            .get(1)
            .ok_or("Native ragdoll command lacks profile")?
            .parse()?;
        let row = profiles
            .get_mut(profile)
            .ok_or("Native ragdoll profile exceeds source limit")?;
        let number = |index: usize| -> Result<f32> {
            let value: f32 = words
                .get(index)
                .ok_or("Native ragdoll numeric field is missing")?
                .parse()?;
            if !value.is_finite() {
                return Err("Native ragdoll numeric field is not finite".into());
            }
            Ok(value)
        };
        let index = |field: usize| -> Result<usize> {
            Ok(words
                .get(field)
                .ok_or("Native ragdoll index field is missing")?
                .parse()?)
        };
        match command {
            "ragdoll_clear" if words.len() == 2 => *row = empty(),
            "ragdoll_bone" if words.len() == 12 => {
                if !words[10].starts_with("cap") {
                    return Err(format!(
                        "Native ragdoll geometry requires conversion: {}",
                        words[10]
                    )
                    .into());
                }
                number(5)?;
                number(11)?;
                let parent: i32 = words[8].parse()?;
                let parent = (parent >= 0).then_some(parent as usize);
                let bones = row["bones"].as_array_mut().unwrap();
                if bones.len() >= 14 || parent.is_some_and(|parent| parent >= bones.len()) {
                    return Err("Native ragdoll bone parent/order exceeds source limits".into());
                }
                bones.push(json!({"joint":words[2],"end_joint":words[3],"radius":number(4)?*0.0254,
                    "mass":number(6)?*0.001,"friction":number(7)?,"parent":parent,"mirror":number(9)?!=0.}));
            }
            "ragdoll_joint" if words.len() == 4 => {
                let bone = index(2)?;
                if bone == 0
                    || bone >= row["bones"].as_array().unwrap().len()
                    || !matches!(words[3], "hinge" | "swivel")
                {
                    return Err("Native ragdoll joint has unsupported source bone/type".into());
                }
                let joints = row["joints"].as_array_mut().unwrap();
                if joints.len() >= 28 {
                    return Err("Native ragdoll joint count exceeds source limit".into());
                }
                joints.push(json!({"bone":bone,"kind":words[3],"limits":[]}));
            }
            "ragdoll_limit" if words.len() == 7 => {
                let axis = match words[3] {
                    "x" => [1., 0., 0.],
                    "y" => [0., 0., -1.],
                    "z" => [0., 1., 0.],
                    _ => return Err("Native ragdoll angular axis requires conversion".into()),
                };
                let joint = row["joints"]
                    .as_array_mut()
                    .unwrap()
                    .get_mut(index(2)?)
                    .ok_or("Native ragdoll limit references missing joint")?;
                let limits = joint["limits"].as_array_mut().unwrap();
                if limits.len() >= 3 {
                    return Err("Native ragdoll angular axes exceed source limit".into());
                }
                let radians = |field| {
                    number(field).map(|value| {
                        value
                            .to_radians()
                            .clamp(-std::f32::consts::PI, std::f32::consts::PI)
                    })
                };
                limits.push(json!({"axis":axis,"friction":number(4)?,"minimum":radians(5)?,"maximum":radians(6)?}));
            }
            "ragdoll_baselerp_bone" if (3..=5).contains(&words.len()) => {
                let seconds = if words.len() > 3 {
                    number(3)?.clamp(100., 6000.) / 1000.
                } else {
                    1.
                };
                let bone = if words.len() > 4 { index(4)? } else { 0 };
                if bone >= row["bones"].as_array().unwrap().len() {
                    return Err("Native ragdoll baselerp parent is missing".into());
                }
                row["baselerp"]
                    .as_array_mut()
                    .unwrap()
                    .push(json!({"joint":words[2],"bone":bone,"seconds":seconds}));
            }
            "ragdoll_selfpair" if words.len() == 4 => {
                let pair = [index(2)?, index(3)?];
                if pair
                    .iter()
                    .any(|bone| *bone >= row["bones"].as_array().unwrap().len())
                {
                    return Err("Native ragdoll collision pair references missing bone".into());
                }
                row["self_pairs"].as_array_mut().unwrap().push(json!(pair));
            }
            _ => return Err(format!("Native ragdoll command requires conversion: {line}").into()),
        }
    }
    let profile = profiles.into_iter().nth(1).unwrap();
    if profile["bones"].as_array().unwrap().len() != 12
        || profile["joints"].as_array().unwrap().len() != 11
    {
        return Err(
            "Native ordinary ragdoll profile1 has unexpected source body/joint counts".into(),
        );
    }
    Ok(profile)
}

pub(super) fn ragdoll_model(catalog: &Catalog<'_>, model: &str, profile: &Value) -> Result<Value> {
    let (zone, root) = catalog.resolve(5, model)?;
    let joints = crate::geometry::skeleton(catalog, model, zone, root)?;
    let named = joints
        .iter()
        .filter_map(|joint| joint.name.as_deref().map(|name| (name, joint)))
        .collect::<BTreeMap<_, _>>();
    let mut profile = profile.clone();
    for (index, bone) in profile["bones"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .enumerate()
    {
        let first = bone["joint"].as_str().unwrap();
        let end = bone["end_joint"].as_str().unwrap();
        let first = named.get(first).ok_or_else(|| {
            format!("Native ragdoll {model} is missing capsule base joint {first}")
        })?;
        let end = named.get(end).ok_or_else(|| {
            format!("Native ragdoll {model} is missing capsule base end joint {end}")
        })?;
        let length = (12..15)
            .map(|axis| (first.bind[axis] - end.bind[axis]).powi(2))
            .sum::<f32>()
            .sqrt();
        bone["half_length"] = json!(length * 0.5 + if index == 3 { 2. * 0.0254 } else { 0. });
        bone["base"] = json!(first.bind);
    }
    let bones = profile["bones"].as_array().unwrap().clone();
    for bone in profile["baselerp"].as_array_mut().unwrap() {
        let joint = bone["joint"].as_str().unwrap();
        let parent = bones[bone["bone"].as_u64().unwrap() as usize]["joint"]
            .as_str()
            .unwrap();
        let joint = named
            .get(joint)
            .ok_or_else(|| format!("Native ragdoll {model} is missing base-lerp joint {joint}"))?;
        let parent = named.get(parent).ok_or_else(|| {
            format!("Native ragdoll {model} is missing base-lerp parent {parent}")
        })?;
        bone["base"] = json!(crate::geometry::compose(parent.inverse_bind, joint.bind));
    }
    Ok(profile)
}

pub(crate) fn root_motion(decoded: &crate::animation::DecodedClip) -> Value {
    if !decoded.has_root_delta {
        return Value::Null;
    }
    let clip = &decoded.clip;
    let seconds = clip.frames as f32 / clip.frame_rate;
    let translation = decoded
        .root_translation
        .as_ref()
        .map(|track| {
            let mut samples = track
                .frames
                .iter()
                .zip(&track.values)
                .map(|(frame, value)| {
                    [
                        *frame as f32 / clip.frame_rate,
                        value[0],
                        value[1],
                        value[2],
                    ]
                })
                .collect::<Vec<_>>();
            if samples.len() == 1 {
                samples.push([seconds, samples[0][1], samples[0][2], samples[0][3]]);
            }
            samples
        })
        .unwrap_or_default();
    let rotation = decoded
        .root_rotation
        .as_ref()
        .map(|track| {
            let mut samples = track
                .frames
                .iter()
                .zip(&track.values)
                .map(|(frame, value)| {
                    [
                        *frame as f32 / clip.frame_rate,
                        value[0],
                        value[1],
                        value[2],
                        value[3],
                    ]
                })
                .collect::<Vec<_>>();
            if samples.len() == 1 {
                samples.push([
                    seconds,
                    samples[0][1],
                    samples[0][2],
                    samples[0][3],
                    samples[0][4],
                ]);
            }
            samples
        })
        .unwrap_or_default();
    json!({"seconds":seconds,"translation":translation,"rotation":rotation})
}

pub(super) fn hit_volumes(catalog: &Catalog<'_>, models: &[&str]) -> Result<Vec<Value>> {
    let locations = [
        "none",
        "helmet",
        "head",
        "neck",
        "upper_torso",
        "mid_torso",
        "lower_torso",
        "right_upper_arm",
        "left_upper_arm",
        "right_lower_arm",
        "left_lower_arm",
        "right_hand",
        "left_hand",
        "right_upper_leg",
        "left_upper_leg",
        "right_lower_leg",
        "left_lower_leg",
        "right_foot",
        "left_foot",
        "gun",
        "shield",
    ];
    let mut shared = BTreeMap::<String, u8>::new();
    let mut volumes = Vec::new();
    for &model in models {
        let (zone, root) = catalog.resolve(5, model)?;
        let streams = &catalog.zones[zone].1.streams;
        let joints = crate::geometry::skeleton(catalog, model, zone, root)?;
        let classes = Address::decode(streams.u32(root.add(24)?)?)?;
        let information = Address::decode(streams.u32(root.add(164)?)?)?;
        let mut resolved = Vec::<u8>::with_capacity(joints.len());
        for (index, joint) in joints.iter().enumerate() {
            let name = joint.name.as_deref().ok_or_else(|| {
                format!("Ordinary source model {model} has unnamed joint {index}")
            })?;
            let mut location = streams.u8(classes.add(index)?)?;
            if location as usize >= locations.len() {
                return Err(format!(
                    "Ordinary source model {model}/{name} has unknown hit location {location}"
                )
                .into());
            }
            if location == 0 {
                location = shared
                    .get(name)
                    .copied()
                    .unwrap_or_else(|| joint.parent.map_or(0, |parent| resolved[parent]));
            }
            resolved.push(location);
            shared.entry(name.to_owned()).or_insert(location);
            let bytes = streams.read(information.add(index * 44)?, 44)?;
            let values: [f32; 10] =
                std::array::from_fn(|index| f32::from_bits(u32_at(bytes, index * 4)));
            let radius = values[9];
            if !radius.is_finite() || radius < 0. {
                return Err(format!(
                    "Ordinary source model {model}/{name} has invalid bone collision radius"
                )
                .into());
            }
            if radius < 4. || location == 0 || location == 19 {
                continue;
            }
            if location == 20 {
                return Err(format!(
                    "Ordinary source model {model}/{name} requires native shield surface collision"
                )
                .into());
            }
            let minimum = [values[0], values[1], values[2]];
            let maximum = [values[3], values[4], values[5]];
            let center = [values[6], values[7], values[8]];
            if minimum
                .iter()
                .chain(&maximum)
                .chain(&center)
                .any(|value| !value.is_finite())
                || (0..3).any(|axis| minimum[axis] >= maximum[axis])
            {
                return Err(format!(
                    "Ordinary source model {model}/{name} has invalid authored bone bounds"
                )
                .into());
            }
            let minimum = [minimum[0], minimum[2], -maximum[1]].map(|value| value * 0.0254);
            let maximum = [maximum[0], maximum[2], -values[1]].map(|value| value * 0.0254);
            let center = [center[0], center[2], -center[1]].map(|value| value * 0.0254);
            volumes.push(json!({"id":format!("{model}.{name}"),"region":locations[location as usize],"presentation":format!("actor.{model}"),"joint":name,"bounds":[minimum,maximum],"sphere":[center[0],center[1],center[2],radius*0.0254*0.0254]}));
        }
    }
    if volumes.is_empty() {
        return Err("Ordinary source character has no authored collision bones".into());
    }
    Ok(volumes)
}

pub(super) fn combat_clip(model: &str, decoded: &crate::animation::DecodedClip) -> Value {
    let clip = &decoded.clip;
    let duration = clip.frames as f32 / clip.frame_rate;
    let seconds = clip
        .notifies
        .iter()
        .find(|(name, _)| matches!(name.as_deref(), Some("end" | "finish" | "undefined")))
        .map_or(duration, |(_, time)| time * duration);
    let hits = clip
        .notifies
        .iter()
        .filter(|(name, _)| name.as_deref() == Some("fire"))
        .map(|(_, time)| time * duration)
        .filter(|time| *time <= seconds)
        .collect::<Vec<_>>();
    let stops = clip
        .notifies
        .iter()
        .filter(|(name, _)| name.as_deref() == Some("stop"))
        .map(|(_, time)| time * duration)
        .filter(|time| *time <= seconds)
        .collect::<Vec<_>>();
    json!({"animation":format!("animation.{model}.{}",clip.name),"seconds":seconds,"duration":duration,"hits":hits,"stops":stops})
}

fn traversal_clip(model: &str, decoded: &crate::animation::DecodedClip) -> Result<Value> {
    let name = &decoded.clip.name;
    let motion = root_motion(decoded);
    let mut translation: Vec<[f32; 4]> = serde_json::from_value(motion["translation"].clone())
        .map_err(|_| {
            format!("Required ordinary traversal {name} has no native root translation")
        })?;
    let seconds = combat_clip(model, decoded)["seconds"]
        .as_f64()
        .ok_or_else(|| format!("Required ordinary traversal {name} has no native finish time"))?
        as f32;
    let mut finish = *translation.last().ok_or_else(|| {
        format!("Required ordinary traversal {name} has empty native root translation")
    })?;
    for pair in translation.windows(2) {
        if seconds <= pair[1][0] {
            let fraction = ((seconds - pair[0][0]) / (pair[1][0] - pair[0][0])).clamp(0., 1.);
            finish = std::array::from_fn(|axis| {
                pair[0][axis] + (pair[1][axis] - pair[0][axis]) * fraction
            });
            break;
        }
    }
    finish[0] = seconds;
    translation.retain(|sample| sample[0] < seconds);
    translation.push(finish);
    if translation.len() < 2 {
        return Err(format!(
            "Required ordinary traversal {name} finishes before its native root motion"
        )
        .into());
    }
    Ok(
        json!({"animation":format!("animation.{model}.{name}"),"seconds":seconds,"motion":translation}),
    )
}

fn global_hit_multipliers(catalog: &Catalog<'_>) -> Result<[f32; 21]> {
    let locations = [
        "none",
        "helmet",
        "head",
        "neck",
        "torso_upper",
        "torso_mid",
        "torso_lower",
        "right_arm_upper",
        "left_arm_upper",
        "right_arm_lower",
        "left_arm_lower",
        "right_hand",
        "left_hand",
        "right_leg_upper",
        "left_leg_upper",
        "right_leg_lower",
        "left_leg_lower",
        "right_foot",
        "left_foot",
        "gun",
        "shield",
    ];
    let (zone, root) = catalog.resolve(41, "info/mp_lochit_dmgtable")?;
    let streams = &catalog.zones[zone].1.streams;
    let length = streams.u32(root.add(4)?)? as usize;
    let buffer = Address::decode(streams.u32(root.add(8)?)?)?;
    let text = std::str::from_utf8(streams.read(buffer, length)?)?;
    let values = text
        .strip_prefix("LOCDMGTABLE")
        .ok_or("Native location damage table has invalid magic")?
        .trim()
        .trim_matches('\0')
        .split('\\')
        .collect::<Vec<_>>();
    let mut multipliers = [1.; 21];
    multipliers[19] = 0.;
    for (index, location) in locations.into_iter().enumerate() {
        let rows = values
            .windows(2)
            .filter(|row| row[0] == location)
            .collect::<Vec<_>>();
        if rows.is_empty() {
            continue;
        }
        if rows.len() != 1 {
            return Err(format!("Native location damage table repeats {location}").into());
        }
        let multiplier: f32 = rows[0][1].parse()?;
        if !multiplier.is_finite() || multiplier < 0. {
            return Err(format!("Native location damage table has invalid {location}").into());
        }
        multipliers[index] = multiplier;
    }
    Ok(multipliers)
}

fn destructible_spawn_models(catalog: &Catalog<'_>) -> Result<BTreeSet<String>> {
    let mut models = BTreeSet::new();
    for (_, zone, root) in catalog.definitions(3) {
        let streams = &catalog.zones[zone].1.streams;
        let count = streams.u32(root.add(12)?)? as usize;
        if count == 0 {
            continue;
        }
        let pieces = Address::decode(streams.u32(root.add(16)?)?)?;
        for piece in 0..count {
            for stage in 0..5 {
                for offset in [32, 36, 40] {
                    let field = pieces.add(piece * 312 + stage * 48 + offset)?;
                    if streams.u32(field)? != 0 {
                        models.insert(catalog.pointer(zone, field, 5)?.0);
                    }
                }
            }
        }
    }
    Ok(models)
}

fn reload_clip_model(catalog: &Catalog<'_>, weapon: &str) -> Result<Option<String>> {
    let (zone, root) = catalog.resolve(25, weapon)?;
    let streams = &catalog.zones[zone].1.streams;
    let array = streams.u32(root.add(44)?)?;
    if array == 0 {
        return Ok(None);
    }
    let field = Address::decode(array)?.add(24)?;
    if streams.u32(field)? == 0 {
        Ok(None)
    } else {
        Ok(Some(catalog.pointer(zone, field, 5)?.0))
    }
}

pub(super) fn read(
    catalog: &Catalog<'_>,
    game: &mut Value,
    entities: &[crate::entities::Entity],
    gaps: &mut Vec<String>,
) -> Result<()> {
    game["tuning"]["global_hit_multipliers"] = json!(global_hit_multipliers(catalog)?);
    game["rounds"]["corpse_limit"] = json!(8);
    game["rounds"]["actor_think_seconds"] = json!(0.05);
    let ragdoll = ragdoll_profile(catalog)?;
    let source = parse_states(&state_source(catalog)?)?;
    let locomotion = [
        ("walk", "zm_move_walk"),
        ("run", "zm_move_run"),
        ("sprint", "zm_move_sprint"),
        ("walk_crawl", "zm_move_walk_crawl"),
        ("run_crawl", "zm_move_run_crawl"),
        ("sprint_crawl", "zm_move_sprint_crawl"),
        ("stumpy", "zm_move_stumpy"),
    ];
    let combat = [
        ("walk_melee", "zm_walk_melee"),
        ("run_melee", "zm_run_melee"),
        ("walk_melee_crawl", "zm_walk_melee_crawl"),
        ("run_melee_crawl", "zm_run_melee_crawl"),
        ("stumpy_melee", "zm_stumpy_melee"),
        ("death", "zm_death"),
        ("death_crawl", "zm_death_crawl"),
    ];
    let selected_states = locomotion
        .iter()
        .chain(combat.iter())
        .map(|(_, state)| *state)
        .chain([
            "zm_idle",
            "zm_idle_crawl",
            "zm_rise",
            "zm_traverse_barrier",
            "zm_traverse_barrier_crawl",
            "zm_zbarrier_board_tear_in",
            "zm_zbarrier_board_tear_out",
            "zm_zbarrier_board_tear_in_crawl",
            "zm_zbarrier_board_tear_out_crawl",
        ])
        .collect::<BTreeSet<_>>();
    let mut required = BTreeSet::new();
    for state in selected_states {
        let clips = source
            .get(state)
            .ok_or_else(|| format!("Required ordinary native ASD state is missing: {state}"))?;
        required.extend(clips.iter().map(|(_, clip)| clip.clone()));
    }
    let decoded = crate::animation::read(catalog, Some(&required))?;
    let clips = decoded
        .iter()
        .map(|decoded| (decoded.clip.name.as_str(), decoded))
        .collect::<BTreeMap<_, _>>();
    let archetypes = entities
        .iter()
        .filter(|entity| {
            entity
                .get("script_noteworthy")
                .is_some_and(|name| name == "zombie_spawner")
                && entity.get("script_int").is_some_and(|group| group == "1")
        })
        .map(|entity| {
            entity
                .get("classname")
                .cloned()
                .ok_or("Ordinary native spawner has no classname")
        })
        .collect::<std::result::Result<BTreeSet<_>, _>>()?;
    if archetypes.is_empty() {
        return Err("Initial ordinary native spawner group1 is missing".into());
    }
    if entities.iter().any(|entity| {
        entity
            .get("script_noteworthy")
            .is_some_and(|name| name == "zombie_spawner")
            && entity.get("script_int").is_some_and(|group| group == "1")
            && (entity.contains_key("script_char_index")
                || entity.contains_key("script_char_group"))
    }) {
        return Err("Initial ordinary native spawner has an unsupported explicit character index/group override".into());
    }
    let german = [
        ("c_zom_tomb_german_body_1a", Some("c_zom_tomb_german_hat_1")),
        ("c_zom_tomb_german_body_1a_nohat", None),
        ("c_zom_tomb_german_body_1b", Some("c_zom_tomb_german_hat_2")),
        ("c_zom_tomb_german_body_1a", Some("c_zom_tomb_german_hat_1")),
        ("c_zom_tomb_german_body_1a", None),
        ("c_zom_tomb_german_body_1a", None),
    ];
    let german2 = [
        ("c_zom_tomb_german_body_1b_nohat", None),
        ("c_zom_tomb_german_body_3c", Some("c_zom_tomb_german_hat_3")),
        ("c_zom_tomb_german_body_3c_nohat", None),
        ("c_zom_tomb_german_body_1a", Some("c_zom_tomb_german_hat_1")),
        ("c_zom_tomb_german_body_1a", None),
        ("c_zom_tomb_german_body_1a", None),
    ];
    let mut models = BTreeMap::<String, BTreeSet<String>>::new();
    let mut enemies = Vec::new();
    for archetype in archetypes {
        let characters = match archetype.as_str() {
            "actor_zm_tomb_basic_german" => german,
            "actor_zm_tomb_basic_german2" => german2,
            _ => return Err(format!(
                "Initial ordinary native archetype requires source character recovery: {archetype}"
            )
            .into()),
        };
        for (index, (model, hat)) in characters.into_iter().enumerate() {
            let mut pieces = Vec::new();
            for (piece, joint) in [
                ("rarm", "j_elbow_ri"),
                ("larm", "j_elbow_le"),
                ("rleg", "j_knee_ri"),
                ("lleg", "j_knee_le"),
            ] {
                let name = format!("c_zom_buried_g_{piece}spawn");
                pieces.push(gib_piece(catalog, &name, joint)?);
                models.entry(name).or_default();
            }
            let hat_piece = hat
                .map(|hat| gib_piece(catalog, hat, "j_head"))
                .transpose()?;
            let variant = match model {
                "c_zom_tomb_german_body_1a" | "c_zom_tomb_german_body_1a_nohat" => "1a",
                "c_zom_tomb_german_body_1b" | "c_zom_tomb_german_body_1b_nohat" => "1b",
                "c_zom_tomb_german_body_3c" | "c_zom_tomb_german_body_3c_nohat" => "3c",
                _ => return Err(format!("Native gib character variant is missing: {model}").into()),
            };
            let replacement = |name: &str| format!("c_zom_tomb_german_body_g_{name}_{variant}");
            let beheaded = "c_zom_tomb_german_body_g_behead";
            let mut gib_models = serde_json::Map::new();
            let mut gib_sources = BTreeSet::from([beheaded.to_owned()]);
            for (kind, torso, legs) in [
                ("right_arm", "rarm", "lowclean"),
                ("left_arm", "larm", "lowclean"),
                ("right_leg", "upclean", "rleg"),
                ("left_leg", "upclean", "lleg"),
                ("no_legs", "upclean", "legsoff"),
            ] {
                let body = replacement(torso);
                let legs = replacement(legs);
                gib_sources.extend([body.clone(), legs.clone()]);
                gib_models.insert(
                    kind.into(),
                    json!({"body":format!("actor.{body}"),"legs":format!("actor.{legs}")}),
                );
            }
            for gib in &gib_sources {
                catalog.resolve(5, gib)?;
                models
                    .entry(gib.clone())
                    .or_default()
                    .extend(required.iter().cloned());
            }
            catalog.resolve(5, model)?;
            models
                .entry(model.into())
                .or_default()
                .extend(required.iter().cloned());
            let presentation = format!("actor.{model}");
            let model_ragdoll = ragdoll_model(catalog, model, &ragdoll)?;
            let animation_id = |clip: &str| format!("animation.{model}.{clip}");
            let mut actions = serde_json::Map::new();
            for (action, state) in [
                ("idle", "zm_idle"),
                ("walk", "zm_move_walk"),
                ("run", "zm_move_run"),
                ("sprint", "zm_move_sprint"),
                ("attack", "zm_walk_melee"),
                ("death", "zm_death"),
            ] {
                actions.insert(action.into(), json!(animation_id(&source[state][0].1)));
            }
            let mut locomotion_states = serde_json::Map::new();
            for (action, state) in locomotion {
                locomotion_states.insert(
                    action.into(),
                    json!(
                        source[state]
                            .iter()
                            .map(|(_, clip)| animation_id(clip))
                            .collect::<Vec<_>>()
                    ),
                );
            }
            let mut combat_states = serde_json::Map::new();
            for (action, state) in combat {
                let values = source[state]
                    .iter()
                    .map(|(_, name)| combat_clip(model, clips[name.as_str()]))
                    .collect::<Vec<_>>();
                combat_states.insert(action.into(), json!(values));
            }
            let mut traversals = serde_json::Map::new();
            for state in [
                "zm_traverse_barrier",
                "zm_traverse_barrier_crawl",
                "zm_rise",
            ] {
                for (alias, name) in &source[state] {
                    let alias = if state == "zm_rise" { "rise" } else { alias };
                    let clip = clips[name.as_str()];
                    let value = traversal_clip(model, clip)?;
                    traversals
                        .entry(alias.to_owned())
                        .or_insert_with(|| json!([]))
                        .as_array_mut()
                        .unwrap()
                        .push(value);
                }
            }
            let attack = &clips[source["zm_walk_melee"][0].1.as_str()].clip;
            let death = &clips[source["zm_death"][0].1.as_str()].clip;
            for head in 1..=3 {
                let head_model = format!("c_zom_tomb_german_head{head}");
                catalog.resolve(5, &head_model)?;
                models.entry(head_model.clone()).or_default();
                let mut parts = vec![
                    json!({"presentation":format!("actor.{head_model}"),"socket":"j_head","root_joint":"j_head"}),
                ];
                if let Some(hat) = hat {
                    catalog.resolve(5, hat)?;
                    models.entry(hat.into()).or_default();
                    parts.push(json!({"presentation":format!("actor.{hat}"),"socket":"j_head","root_joint":"j_head"}));
                }
                let mut hit_models = vec![model, head_model.as_str()];
                if let Some(hat) = hat {
                    hit_models.push(hat);
                }
                let mut hit_regions = hit_volumes(catalog, &hit_models)?;
                let mut ids = hit_regions
                    .iter()
                    .map(|hit| hit["id"].as_str().unwrap().to_owned())
                    .collect::<BTreeSet<_>>();
                for gib in &gib_sources {
                    for hit in hit_volumes(catalog, &[model, gib.as_str()])? {
                        if ids.insert(hit["id"].as_str().unwrap().to_owned()) {
                            hit_regions.push(hit);
                        }
                    }
                }
                enemies.push(json!({"id":format!("enemy.{archetype}.{index}.{head}"),"health":game["rounds"]["health_start"],"damage":60,
                    "physics":{"radius":15.*0.0254,"height":72.*0.0254,"crawler_height":24.*0.0254},
                    "family":"german","archetype":archetype,"character_group":"tomb.auto","character_index":index,
                    "presentation":presentation,"parts":parts,"hit_regions":hit_regions,"animation_actions":actions,"locomotion_states":locomotion_states,"combat_states":combat_states,"traversals":traversals,
                    "gibs":{"models":gib_models,"beheaded":format!("actor.{beheaded}"),"pieces":pieces,"hat":hat_piece},
                    "ragdoll":model_ragdoll,
                    "attack_seconds":attack.frames as f32/attack.frame_rate,"attack_hits":attack.notifies.iter().filter(|(name,_)|name.as_deref()==Some("fire")).map(|(_,time)|time*attack.frames as f32/attack.frame_rate).collect::<Vec<_>>(),
                    "death_seconds":death.frames as f32/death.frame_rate}));
            }
        }
    }
    let mut presentations = Vec::new();
    let mut animations = Vec::new();
    for (model, required) in models {
        let skeleton = format!("actor.{model}");
        let mut ids = Vec::new();
        for name in required {
            let decoded = clips[name.as_str()];
            let clip = &decoded.clip;
            let id = format!("animation.{model}.{name}");
            ids.push(id.clone());
            let seconds = clip.frames as f32 / clip.frame_rate;
            animations.push(json!({"id":id,"source_clip":name,"path":null,"skeleton":skeleton,"seconds":seconds,
                "notifications":clip.notifies.iter().filter_map(|(name,time)|name.as_ref().map(|name|json!({"seconds":time*seconds,"event":name}))).collect::<Vec<_>>(),"root_motion":root_motion(decoded),"looping":clip.source_flags[0]!=0}));
        }
        presentations.push(
            json!({"id":skeleton,"source_model":model,"scene":null,"animations":ids,"sounds":[]}),
        );
    }
    game["content"]["enemies"] = json!(enemies);
    game["content"]["presentations"]
        .as_array_mut()
        .ok_or("Initial presentations are missing before ordinary enemy conversion")?
        .extend(presentations);
    game["content"]["animations"]
        .as_array_mut()
        .ok_or("Initial animations are missing before ordinary enemy conversion")?
        .extend(animations);
    let destructibles = destructible_spawn_models(catalog)?;
    if !destructibles.is_empty() {
        gaps.push(format!("Initial source destructible allocations sharing the detached gib pool require consumers: {}",destructibles.into_iter().collect::<Vec<_>>().join(", ")));
    }
    if let Some(model) = reload_clip_model(catalog, "c96_zm")? {
        gaps.push(format!("Initial C96 dropped reload clip {model} shares the detached gib pool and requires a source consumer"));
    }
    println!(
        "Converted {} source ordinary character/head selections and {} required native animation clips with root-motion samples",
        game["content"]["enemies"].as_array().unwrap().len(),
        clips.len()
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ordinary_traversal_stops_at_source_finish_between_root_keys() {
        let decoded = crate::animation::DecodedClip {
            clip: crate::animation::Clip {
                name: "rise".into(),
                frame_rate: 10.,
                frames: 10,
                source_flags: [0; 4],
                streamed_file_size: 0,
                loop_entry_time: 0.,
                notifies: vec![(Some("finish".into()), 0.75)],
                bones: Vec::new(),
            },
            has_root_delta: true,
            root_translation: Some(crate::animation::Track {
                frames: vec![0, 5, 10],
                values: vec![[0., 0., 0.], [1., 0., 0.], [5., 0., 0.]],
            }),
            root_rotation: None,
        };
        let converted = traversal_clip("body", &decoded).unwrap();
        assert_eq!(converted["seconds"], 0.75);
        assert_eq!(
            converted["motion"],
            json!([[0., 0., 0., 0.], [0.5, 1., 0., 0.], [0.75, 3., 0., 0.]])
        );
    }

    #[test]
    #[ignore = "requires the authorized Steam installation"]
    fn installed_ordinary_ragdoll_source() {
        let zones = crate::source::read_zones(&crate::source::installation().unwrap()).unwrap();
        let catalog = Catalog::new(&zones);
        let profile = ragdoll_profile(&catalog).unwrap();
        assert_eq!(profile["bones"].as_array().unwrap().len(), 12);
        assert_eq!(profile["joints"].as_array().unwrap().len(), 11);
        assert_eq!(profile["bones"][0]["joint"], "j_mainroot");
        assert_eq!(profile["bones"][0]["end_joint"], "j_spinelower");
        assert_eq!(profile["bones"][6]["mirror"], true);
        let limits = profile["joints"][3]["limits"].as_array().unwrap();
        assert_eq!(limits.len(), 3);
        assert_eq!(limits[1]["axis"], json!([0., 0., -1.]));
        assert!(
            (limits[1]["minimum"].as_f64().unwrap() as f32 - (-90f32).to_radians()).abs() < 0.00001
        );
        println!(
            "Parsed native ordinary ragdoll profile1: {} bodies, {} joints, {} baselerp bones, {} collision pairs",
            profile["bones"].as_array().unwrap().len(),
            profile["joints"].as_array().unwrap().len(),
            profile["baselerp"].as_array().unwrap().len(),
            profile["self_pairs"].as_array().unwrap().len()
        );
        for name in [
            "c_zom_buried_g_rarmspawn",
            "c_zom_buried_g_larmspawn",
            "c_zom_buried_g_rlegspawn",
            "c_zom_buried_g_llegspawn",
            "c_zom_tomb_german_hat_1",
            "c_zom_tomb_german_hat_2",
            "c_zom_tomb_german_hat_3",
        ] {
            let piece = gib_piece(&catalog, name, "j_head").unwrap();
            assert!((piece["physics"]["mass"].as_f64().unwrap() - 0.003).abs() < 0.000001);
            assert_eq!(piece["physics"]["bullet_force_scale"], 1.);
            assert_eq!(piece["physics"]["explosive_force_scale"], 1.);
            assert!(!piece["physics"]["shapes"].as_array().unwrap().is_empty());
        }
        let model = ragdoll_model(&catalog, "c_zom_tomb_german_body_1a", &profile).unwrap();
        assert!(
            model["bones"]
                .as_array()
                .unwrap()
                .iter()
                .all(|bone| bone["half_length"].as_f64().unwrap() > 0.)
        );
        assert!(
            model["baselerp"]
                .as_array()
                .unwrap()
                .iter()
                .all(|bone| bone["base"].as_array().unwrap().len() == 16)
        );
        for weapon in ["c96_zm", "c96_upgraded_zm"] {
            let model = reload_clip_model(&catalog, weapon).unwrap();
            println!("Native source {weapon} dropped reload clip model: {model:?}");
            assert!(model.is_none());
        }
        let destructibles = catalog
            .definitions(3)
            .map(|(name, _, _)| name)
            .collect::<Vec<_>>();
        println!("Native destructible definitions: {destructibles:?}");
        let spawned = destructible_spawn_models(&catalog).unwrap();
        println!("Native destructible physical spawn models: {spawned:?}");
        assert!(spawned.is_empty());
        let entities = crate::entities::read(&catalog).unwrap();
        for entity in entities.iter().filter(|entity| {
            entity.iter().any(|(key, value)| {
                key.contains("destruct") || destructibles.iter().any(|name| *name == value)
            })
        }) {
            println!("Native authored destructible entity: {entity:?}");
        }
    }

    #[test]
    #[ignore = "requires the authorized Steam installation"]
    fn installed_ordinary_animation_source() {
        let zones = crate::source::read_zones(&crate::source::installation().unwrap()).unwrap();
        let catalog = Catalog::new(&zones);
        let source = parse_states(&state_source(&catalog).unwrap()).unwrap();
        assert_eq!(source["zm_move_walk"].len(), 8);
        assert_eq!(source["zm_move_walk"][6], source["zm_move_walk"][7]);
        assert_eq!(source["zm_walk_melee"].len(), 10);
        let entities = crate::entities::read(&catalog).unwrap();
        let mut game =
            json!({"rounds":{"health_start":150},"content":{"presentations":[],"animations":[]}});
        let mut gaps = Vec::new();
        read(&catalog, &mut game, &entities, &mut gaps).unwrap();
        let enemies = game["content"]["enemies"].as_array().unwrap();
        assert_eq!(enemies.len(), 36);
        assert!(enemies.iter().all(|enemy| enemy["damage"] == 60
            && enemy["health"] == 150
            && enemy.get("corpse_seconds").is_none()));
        let first = &enemies[0];
        let volumes = first["hit_regions"].as_array().unwrap();
        assert!(
            volumes.iter().any(
                |volume| volume["joint"] == "j_knee_le" && volume["region"] == "left_lower_leg"
            )
        );
        assert!(
            volumes
                .iter()
                .any(|volume| volume["joint"] == "j_knee_ri"
                    && volume["region"] == "right_lower_leg")
        );
        assert!(
            volumes
                .iter()
                .any(|volume| volume["joint"] == "j_elbow_le"
                    && volume["region"] == "left_lower_arm")
        );
        assert!(
            volumes
                .iter()
                .any(|volume| volume["joint"] == "j_head" && volume["region"] == "head")
        );
        assert!(
            enemies
                .iter()
                .all(|enemy| !enemy["hit_regions"].as_array().unwrap().is_empty())
        );
        assert_eq!(
            first["locomotion_states"]["walk"].as_array().unwrap().len(),
            8
        );
        assert_eq!(
            first["traversals"]["barrier_walk"]
                .as_array()
                .unwrap()
                .len(),
            2
        );
        assert_eq!(
            first["traversals"]["barrier_sprint"]
                .as_array()
                .unwrap()
                .len(),
            2
        );
        assert!(
            first["combat_states"]["walk_melee"]
                .as_array()
                .unwrap()
                .iter()
                .all(|clip| !clip["hits"].as_array().unwrap().is_empty())
        );
        let animations = game["content"]["animations"].as_array().unwrap();
        println!(
            "Ordinary native selected notify events: {:?}",
            animations
                .iter()
                .flat_map(|animation| animation["notifications"].as_array().unwrap())
                .map(|note| note["event"].as_str().unwrap())
                .collect::<BTreeSet<_>>()
        );
        assert!(
            animations
                .iter()
                .filter(|animation| animation["source_clip"] == "ai_zombie_walk_v1")
                .all(|animation| animation["root_motion"]["translation"]
                    .as_array()
                    .unwrap()
                    .len()
                    == 2)
        );
        assert!(gaps.is_empty(), "{gaps:?}");
        println!(
            "Ordinary source closure retained {} exact pending consumers",
            gaps.len()
        );
    }
}

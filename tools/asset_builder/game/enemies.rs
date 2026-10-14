use super::*;
use std::collections::{BTreeMap, BTreeSet};

fn state_source(catalog: &Catalog<'_>) -> Result<String> {
    let (zone, root) = catalog.resolve(41, "animstatedefs/zm_tomb_basic.asd")?;
    let streams = &catalog.zones[zone].1.streams;
    let length = streams.u32(root.add(4)?)? as usize;
    let data = Address::decode(streams.u32(root.add(8)?)?)?;
    Ok(std::str::from_utf8(streams.read(data, length)?)?.to_owned())
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

fn hit_volumes(catalog: &Catalog<'_>, models: &[&str]) -> Result<Vec<Value>> {
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

pub(super) fn read(
    catalog: &Catalog<'_>,
    game: &mut Value,
    entities: &[crate::entities::Entity],
    gaps: &mut Vec<String>,
) -> Result<()> {
    game["tuning"]["global_hit_multipliers"] = json!(global_hit_multipliers(catalog)?);
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
            catalog.resolve(5, model)?;
            models
                .entry(model.into())
                .or_default()
                .extend(required.iter().cloned());
            let presentation = format!("actor.{model}");
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
                    .map(|(_, name)| {
                        let clip = &clips[name.as_str()].clip;
                        let duration = clip.frames as f32 / clip.frame_rate;
                        let seconds = clip.notifies.iter().find(|(name,_)|name.as_deref()==Some("end")).map_or(duration,|(_,time)|time*duration);
                        let hits = clip
                            .notifies
                            .iter()
                            .filter(|(name, _)| name.as_deref() == Some("fire"))
                            .map(|(_, time)| time * duration)
                            .filter(|time|*time <= seconds)
                            .collect::<Vec<_>>();
                        let stops=clip.notifies.iter().filter(|(name,_)|name.as_deref()==Some("stop")).map(|(_,time)|time*duration).filter(|time|*time <= seconds).collect::<Vec<_>>();
                        json!({"animation":animation_id(name),"seconds":seconds,"hits":hits,"stops":stops})
                    })
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
                    let motion = root_motion(clip);
                    let translation = motion["translation"].as_array().ok_or_else(|| {
                        format!("Required ordinary traversal {name} has no native root translation")
                    })?;
                    let value = json!({"animation":animation_id(name),"seconds":motion["seconds"],"motion":translation});
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
                let hit_regions = hit_volumes(catalog, &hit_models)?;
                enemies.push(json!({"id":format!("enemy.{archetype}.{index}.{head}"),"health":game["rounds"]["health_start"],"damage":60,
                    "family":"german","archetype":archetype,"character_group":"tomb.auto","character_index":index,
                    "presentation":presentation,"parts":parts,"hit_regions":hit_regions,"animation_actions":actions,"locomotion_states":locomotion_states,"combat_states":combat_states,"traversals":traversals,
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
            animations.push(json!({"id":id,"source_clip":name,"path":null,"skeleton":skeleton,
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
    gaps.push("ordinary enemy limb replacement/gib model bindings and damage-category behavior require conversion".into());
    gaps.push(
        "ordinary enemy server corpse allocation/actor retirement and physical ragdoll require conversion"
            .into(),
    );
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
        assert!(!gaps.is_empty());
        println!(
            "Ordinary source closure retained {} exact pending consumers",
            gaps.len()
        );
    }
}

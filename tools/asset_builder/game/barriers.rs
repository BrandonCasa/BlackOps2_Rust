use super::*;
use std::collections::BTreeSet;

fn append(game: &mut Value, key: &str, value: Value) -> Result<()> {
    if game["content"][key].is_null() {
        game["content"][key] = json!([]);
    }
    let rows = game["content"][key]
        .as_array_mut()
        .ok_or("Initial barrier content is not an array")?;
    if let Some(existing) = rows.iter().find(|row| row["id"] == value["id"]) {
        if existing != &value {
            return Err(format!("Conflicting source barrier binding {}", value["id"]).into());
        }
    } else {
        rows.push(value);
    }
    Ok(())
}

fn motion(decoded: &crate::animation::DecodedClip) -> Result<Value> {
    let clip = &decoded.clip;
    let track = decoded
        .root_translation
        .as_ref()
        .or_else(|| {
            clip.bones
                .iter()
                .find(|bone| bone.name.as_deref() == Some("tag_origin"))
                .and_then(|bone| bone.translation.as_ref())
        })
        .ok_or_else(|| {
            format!(
                "Barrier actor clip {} has no source root translation",
                clip.name
            )
        })?;
    let mut result = track
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
    if result.len() == 1 && result[0][0] == 0. {
        let mut last = result[0];
        last[0] = clip.frames as f32 / clip.frame_rate;
        result.push(last);
    }
    if result.first().is_none_or(|point| point[0] != 0.)
        || result
            .last()
            .is_none_or(|point| (point[0] - clip.frames as f32 / clip.frame_rate).abs() > 0.001)
    {
        return Err(format!(
            "Barrier actor clip {} root motion does not span its authored duration",
            clip.name
        )
        .into());
    }
    let mut rotation = decoded
        .root_rotation
        .as_ref()
        .map(|track| {
            track
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
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    if rotation.len() == 1 && rotation[0][0] == 0. {
        let mut last = rotation[0];
        last[0] = clip.frames as f32 / clip.frame_rate;
        rotation.push(last);
    }
    Ok(
        json!({"seconds":clip.frames as f32/clip.frame_rate,"translation":result,"rotation":rotation}),
    )
}

pub(super) fn read(
    catalog: &Catalog<'_>,
    game: &mut Value,
    entities: &[crate::entities::Entity],
    missing: &mut Vec<String>,
) -> Result<()> {
    let wood: Vec<_> = entities
        .iter()
        .enumerate()
        .filter(|(_, entity)| {
            entity
                .get("classname")
                .is_some_and(|class| class.starts_with("zbarrier_"))
                && entity
                    .get("type")
                    .is_some_and(|kind| kind.eq_ignore_ascii_case("zmcore_BasicWoodBarrier"))
        })
        .collect();
    if wood.is_empty() {
        return Err("Required Origins ordinary barrier entities are absent".into());
    }
    let (zone, root) = catalog.resolve(59, "zmcore_BasicWoodBarrier")?;
    let streams = &catalog.zones[zone].1.streams;
    let count = streams.u32(root.add(52)?)?;
    let slots = streams.u32(root.add(72)?)?;
    if count != 6 || slots != 3 {
        return Err(format!(
            "Ordinary native barrier board/attack-slot layout changed: {count}/{slots}"
        )
        .into());
    }
    for (flag, state, behavior) in [(60, 68, "taunt"), (64, 70, "reach-through attack")] {
        if streams.u32(root.add(flag)?)? != 0 {
            let word = streams.u16(root.add(state)?)? as usize;
            let name = streams
                .script_strings
                .get(word)
                .copied()
                .flatten()
                .map(|address| streams.text(address))
                .transpose()?
                .unwrap_or_default();
            missing.push(format!("Initial ordinary barrier native {behavior} state {name:?} requires source actor consumer"));
        }
    }
    if streams.u32(root.add(28)?)? != 0 {
        missing.push("Initial ordinary barrier native repair earthquake requires source scale/duration/radius playback binding".into());
    }
    let mut source = Vec::new();
    let mut required = BTreeSet::new();
    let mut repair_effects = BTreeSet::new();
    let mut board_audio = Vec::new();
    let aliases = crate::sound::alias_hashes(catalog)?;
    let mut audio_bindings = serde_json::Map::new();
    let mut resolve_sound = |hash: u32| -> Result<Value> {
        if hash == 0 {
            return Ok(Value::Null);
        }
        let name = aliases.get(&hash).ok_or_else(|| format!("Initial ordinary barrier native sound alias {hash:#010x} is absent from installed sound banks"))?;
        audio_bindings.insert(format!("sound.barrier.native.{hash:08x}"), json!(name));
        Ok(json!(name))
    };
    for index in 0..count as usize {
        let board = root.add(80 + index * 80)?;
        let model = catalog.pointer(zone, board, 5)?.0;
        let repair_sound = streams.u32(board.add(52)?)?;
        let hover_sound = streams.u32(board.add(56)?)?;
        let pause_repeat = streams.u32(board.add(60)?)?;
        let min_pause = f32::from_bits(streams.u32(board.add(64)?)?);
        let max_pause = f32::from_bits(streams.u32(board.add(68)?)?);
        if pause_repeat != 0
            && (!min_pause.is_finite()
                || !max_pause.is_finite()
                || min_pause < 0.
                || max_pause < min_pause)
        {
            return Err(format!(
                "Native barrier board {index} repair sound pause range is invalid"
            )
            .into());
        }
        board_audio.push(
            json!({"opening":resolve_sound(repair_sound)?,"hover":resolve_sound(hover_sound)?,
            "repeat_pause":if pause_repeat != 0 {json!([min_pause,max_pause])} else {Value::Null}}),
        );
        for offset in [20, 24] {
            if streams.u32(board.add(offset)?)? != 0 {
                repair_effects.insert(catalog.pointer(zone, board.add(offset)?, 33)?.0);
            }
        }
        let text = |offset| -> Result<String> {
            Ok(streams.text(Address::decode(streams.u32(board.add(offset)?)?)?)?)
        };
        let script = |offset| -> Result<String> {
            let index = streams.u16(board.add(offset)?)? as usize;
            let address = streams
                .script_strings
                .get(index)
                .copied()
                .flatten()
                .ok_or("Native barrier state script string is missing")?;
            Ok(streams.text(address)?)
        };
        let pull = text(12)?;
        let repair = text(16)?;
        let state = script(72)?;
        let piece = script(74)?;
        let select = |slot: usize, suffix: &str| -> Result<String> {
            let alias = format!("spot_{slot}_piece_{piece}");
            let state = format!("{state}_{suffix}");
            let mut clips = super::enemies::state_clips(catalog, &state)?
                .into_iter()
                .filter(|(substate, _)| substate == &alias);
            let clip = clips
                .next()
                .ok_or_else(|| format!("Native barrier ASD state {state} lacks {alias}"))?
                .1;
            if clips.next().is_some() {
                return Err(
                    format!("Native barrier ASD state {state}/{alias} is ambiguous").into(),
                );
            }
            Ok(clip)
        };
        let mut attacks = Vec::new();
        for slot in 0..slots as usize {
            let grab = select(slot, "in")?;
            let tear = select(slot, "out")?;
            required.extend([grab.clone(), tear.clone()]);
            attacks.push((grab, tear));
        }
        required.extend([repair.clone(), pull.clone()]);
        source.push((model, repair, pull, attacks));
    }
    let decoded = crate::animation::read(catalog, Some(&required))?;
    let clip = |name: &str| {
        decoded
            .iter()
            .find(|decoded| decoded.clip.name.eq_ignore_ascii_case(name))
            .ok_or_else(|| format!("Native barrier animation {name} did not decode"))
    };
    let mut boards = Vec::new();
    for (index, (model, repair, pull, attacks)) in source.into_iter().enumerate() {
        let presentation = format!("actor.{model}");
        let ids = [&repair, &pull].map(|clip| format!("animation.{model}.{clip}"));
        for (name, id) in [&repair, &pull].into_iter().zip(&ids) {
            let decoded = clip(name)?;
            let duration = decoded.clip.frames as f32 / decoded.clip.frame_rate;
            let notifications = decoded
                .clip
                .notifies
                .iter()
                .filter_map(|(name, fraction)| {
                    name.as_ref()
                        .map(|name| json!({"event":name,"seconds":fraction*duration}))
                })
                .collect::<Vec<_>>();
            append(
                game,
                "animations",
                json!({"id":id,"source_clip":name,"path":null,"skeleton":presentation,"notifications":notifications,"looping":decoded.clip.source_flags[0]!=0,"root_motion":super::enemies::root_motion(decoded)}),
            )?;
        }
        append(
            game,
            "presentations",
            json!({"id":presentation,"source_model":model,"scene":null,"animations":ids,"sounds":[]}),
        )?;
        let mut variants = Vec::new();
        for (grab, tear) in attacks {
            let grab_decoded = clip(&grab)?;
            let tear_decoded = clip(&tear)?;
            let tear_seconds = tear_decoded.clip.frames as f32 / tear_decoded.clip.frame_rate;
            let destroy: Vec<_> = tear_decoded
                .clip
                .notifies
                .iter()
                .filter(|(name, _)| {
                    name.as_deref()
                        .is_some_and(|name| matches!(name, "board" | "destroy_piece" | "bar"))
                })
                .map(|(_, fraction)| fraction * tear_seconds)
                .collect();
            if destroy.len() != 1 {
                return Err(format!(
                    "Barrier board {index} actor tear {tear} has {} source destroy notifies",
                    destroy.len()
                )
                .into());
            }
            variants.push(json!({"grab_seconds":grab_decoded.clip.frames as f32/grab_decoded.clip.frame_rate,"tear_seconds":tear_seconds,
            "destroy_seconds":destroy[0],"grab_clip":grab,"tear_clip":tear,"grab_motion":motion(grab_decoded)?,"tear_motion":motion(tear_decoded)?}));
        }
        boards.push(json!({"presentation":presentation,"repair":ids[0],"pull":ids[1],
            "repair_seconds":clip(&repair)?.clip.frames as f32/clip(&repair)?.clip.frame_rate,
            "pull_seconds":clip(&pull)?.clip.frames as f32/clip(&pull)?.clip.frame_rate,"attacks":variants,"audio":board_audio[index]}));
    }
    for (index, entity) in &wood {
        for (board_index, board) in boards.iter().enumerate() {
            for (key, expected) in [
                (
                    format!("zbarrierboardmodel{}", board_index + 1),
                    board["presentation"]
                        .as_str()
                        .unwrap()
                        .strip_prefix("actor.")
                        .unwrap(),
                ),
                (
                    format!("zbarrierboardanim{}", board_index + 1),
                    board["repair"]
                        .as_str()
                        .unwrap()
                        .rsplit('.')
                        .next()
                        .unwrap(),
                ),
                (
                    format!("zbarriertearanim{}", board_index + 1),
                    board["pull"].as_str().unwrap().rsplit('.').next().unwrap(),
                ),
            ] {
                if entity.get(&key).is_none_or(|value| value != expected) {
                    return Err(format!("Initial barrier source entity {index} overrides required {key}; conversion must preserve it").into());
                }
            }
        }
    }
    let (collision_zone, collision_root) =
        catalog.resolve(5, "p6_anim_zm_barricade_board_collision")?;
    let collision_streams = &catalog.zones[collision_zone].1.streams;
    let contents = collision_streams.u32(collision_root.add(160)?)? | 0x2080;
    let joints = crate::geometry::skeleton(
        catalog,
        "p6_anim_zm_barricade_board_collision",
        collision_zone,
        collision_root,
    )?;
    let mut collision_triangles = 0;
    for (index, board) in boards.iter_mut().enumerate() {
        let name = format!("tag_board_{}", index + 1);
        let (bone, joint) = joints
            .iter()
            .enumerate()
            .find(|(_, joint)| joint.name.as_deref() == Some(&name))
            .ok_or_else(|| format!("Native barrier collision model lacks {name}"))?;
        let batches = crate::collision::model_triangles(
            catalog,
            collision_zone,
            collision_root,
            u32::MAX,
            Some(bone),
        )?;
        let mut shapes = Vec::new();
        for (flags, triangles) in batches {
            let vertices = triangles
                .iter()
                .flatten()
                .map(|point| {
                    let point = [
                        point[0] as f32 * 0.0254,
                        point[2] as f32 * 0.0254,
                        -point[1] as f32 * 0.0254,
                    ];
                    std::array::from_fn::<_, 3, _>(|axis| {
                        joint.bind[12 + axis]
                            + (0..3)
                                .map(|column| joint.bind[column * 4 + axis] * point[column])
                                .sum::<f32>()
                    })
                })
                .collect::<Vec<_>>();
            let indices = (0..vertices.len() / 3)
                .map(|triangle| [triangle * 3, triangle * 3 + 1, triangle * 3 + 2])
                .collect::<Vec<_>>();
            collision_triangles += indices.len();
            shapes.push(json!({"id":format!("barrier.board.{index}.{flags}"),"vertices":vertices,"indices":indices,"contents":contents,"surface_flags":flags}));
        }
        if shapes.is_empty() {
            return Err(format!(
                "Native barrier collision model has no triangle surfaces for {name}"
            )
            .into());
        }
        board["collision"] = json!(shapes);
    }
    println!(
        "Native ordinary barrier collision: contents {contents:#x}, {} triangles",
        collision_triangles
    );
    game["_native_barrier_boards"] = json!({"boards":boards,"attack_slots":slots,"attack_spacing":f32::from_bits(streams.u32(root.add(76)?)?)*0.0254});
    if game["audio"].is_null() {
        game["audio"] = json!({});
    }
    if game["audio"]["bindings"].is_null() {
        game["audio"]["bindings"] = json!({});
    }
    let general = [streams.u32(root.add(4)?)?, streams.u32(root.add(8)?)?];
    let delay = streams.u32(root.add(20)?)?;
    let delay_seconds = f32::from_bits(streams.u32(root.add(24)?)?);
    if delay != 0 && (!delay_seconds.is_finite() || delay_seconds < 0.) {
        return Err("Native ordinary barrier general repair sound delay is invalid".into());
    }
    game["_native_barrier_audio"] = json!({"repair":[resolve_sound(general[0])?,resolve_sound(general[1])?],
        "repair_delay":if delay != 0 {json!(delay_seconds)}else{Value::Null}});
    game["audio"]["bindings"]
        .as_object_mut()
        .ok_or("Source audio bindings are not an object")?
        .extend(audio_bindings);
    for effect in repair_effects {
        missing.push(format!(
            "Initial ordinary barrier repair effect {effect} requires native runtime timing/binding"
        ));
    }
    println!(
        "Converted native ordinary barrier model/animation/timing roots: {} authored wood barriers, six board models, {} selected clips",
        wood.len(),
        required.len()
    );
    Ok(())
}

pub(super) fn prepare(
    game: &mut Value,
    entities: &[crate::entities::Entity],
    missing: &mut Vec<String>,
) -> Result<()> {
    super::triggers::initial_brush_lifecycles(game, entities, missing)?;
    let boards = game
        .as_object_mut()
        .ok_or("Initial game source is not an object")?
        .remove("_native_barrier_boards")
        .ok_or("Native ordinary barrier source was not read")?;
    let audio = game
        .as_object_mut()
        .unwrap()
        .remove("_native_barrier_audio")
        .ok_or("Native ordinary barrier audio was not read")?;
    let links = game["content"]["navigation_links"]
        .as_array()
        .ok_or("Initial barriers require native navigation links")?
        .clone();
    let mut barriers = Vec::new();
    for link in links.iter().filter(|link| !link["negotiation"].is_null()) {
        let target = link["negotiation"]["start"]["targetname"]
            .as_str()
            .ok_or("Barrier negotiation lacks source targetname")?;
        let matching: Vec<_> = entities
            .iter()
            .enumerate()
            .filter(|(_, entity)| {
                entity.get("targetname").is_some_and(|name| name == target)
                    && entity
                        .get("type")
                        .is_some_and(|kind| kind.eq_ignore_ascii_case("zmcore_BasicWoodBarrier"))
            })
            .collect();
        if matching.is_empty() {
            continue;
        }
        if matching.len() != 1 {
            return Err(
                format!("Initial negotiation {target} resolves ambiguous wood barriers").into(),
            );
        }
        let (index, entity) = matching[0];
        let id = format!("barrier.native.{index}");
        if barriers.iter().any(|barrier: &Value| barrier["id"] == id) {
            continue;
        }
        let goal = entities
            .iter()
            .find(|goal| {
                goal.get("targetname")
                    .is_some_and(|name| name == "exterior_goal")
                    && goal.get("target").is_some_and(|name| name == target)
            })
            .ok_or_else(|| format!("Initial barrier {target} lacks its source exterior_goal"))?;
        if goal.contains_key("script_delay") {
            return Err(format!(
                "Initial barrier {target} requires authored repair trigger overrides"
            )
            .into());
        }
        let repair_locations = entities
            .iter()
            .filter(|entity| {
                entity
                    .get("classname")
                    .is_some_and(|class| class == "script_struct")
                    && entity.get("targetname").is_some_and(|name| name == target)
            })
            .collect::<Vec<_>>();
        if repair_locations.len() > 1 {
            return Err(format!(
                "Initial barrier {target} has ambiguous authored repair locations"
            )
            .into());
        }
        let repair_location = repair_locations.first().copied().unwrap_or(goal);
        let [rx, ry, rz] = crate::entities::vector(repair_location, "origin")?
            .ok_or("Initial barrier repair location lacks origin")?;
        let [repair_pitch, repair_yaw, repair_roll] =
            crate::entities::vector(repair_location, "angles")?.unwrap_or([0.; 3]);
        if repair_pitch != 0. || repair_roll != 0. {
            return Err(
                format!("Initial barrier {target} requires a tilted repair trigger").into(),
            );
        }
        let repair_radius = repair_location
            .get("radius")
            .map(|value| value.parse::<f32>())
            .transpose()?
            .unwrap_or(94.21)
            * 0.0254;
        let repair_points = goal
            .get("zombie_cost")
            .map(|value| value.parse::<u32>())
            .transpose()?
            .unwrap_or(10);
        let [x, y, z] = crate::entities::vector(entity, "origin")?
            .ok_or("Initial barrier lacks native origin")?;
        let [pitch, yaw, roll] = crate::entities::vector(entity, "angles")?
            .ok_or("Initial barrier lacks native angles")?;
        if pitch != 0. || roll != 0. {
            return Err(format!("Initial barrier {target} requires tilted board placement").into());
        }
        let exit = &link["to"];
        let zone = game["content"]["navigation_nodes"]
            .as_array()
            .unwrap()
            .iter()
            .find(|node| node["id"] == *exit)
            .ok_or("Initial barrier exit node was omitted")?["zone"]
            .clone();
        let placement = format!("placement.barrier.native.{index}");
        append(
            game,
            "placements",
            json!({"id":placement,"definition":id,"position":[x*0.0254,z*0.0254,-y*0.0254],"yaw":(yaw-90.).to_radians(),"zone":zone}),
        )?;
        append(
            game,
            "interactions",
            json!({"id":format!("interaction.barrier.native.{index}"),"placement":placement,"action":{"kind":"barrier","barrier":id},"reach":repair_radius,"hold_seconds":0.25,"hidden_prompt":false}),
        )?;
        barriers.push(json!({"id":id,"placement":placement,"entry":link["from"],"exit":exit,"boards":boards["boards"],"audio":audio,
            "repair_origin":[rx*0.0254,rz*0.0254,-ry*0.0254],"repair_yaw":(repair_yaw-90.).to_radians(),"repair_radius":repair_radius,"repair_height":64_f32*0.0254,"repair_interval":1.,"repair_points":repair_points}));
    }
    if barriers.is_empty() {
        return Err("Initial native path routes have no authored wood barrier binding".into());
    }
    println!(
        "Bound {} initial authored barriers to exact native entry/exit negotiations",
        barriers.len()
    );
    game["content"]["barriers"] = json!(barriers);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "requires the authorized Steam installation"]
    fn installed_starting_barriers() {
        let zones = crate::source::read_zones(&crate::source::installation().unwrap()).unwrap();
        let catalog = Catalog::new(&zones);
        let entities = crate::entities::read(&catalog).unwrap();
        let mut game = json!({"content":{}});
        let mut missing = Vec::new();
        read(&catalog, &mut game, &entities, &mut missing).unwrap();
        assert_eq!(
            game["_native_barrier_boards"]["boards"]
                .as_array()
                .unwrap()
                .len(),
            6
        );
        assert_eq!(game["_native_barrier_boards"]["attack_slots"], 3);
        println!(
            "Native ordinary barrier audio: {}",
            game["_native_barrier_audio"]
        );
        assert!(!missing.iter().any(|gap| gap.contains("sound hashes")));
        for board in game["_native_barrier_boards"]["boards"].as_array().unwrap() {
            println!("Native board audio: {}", board["audio"]);
            assert!(!board["collision"].as_array().unwrap().is_empty());
            let attacks = board["attacks"].as_array().unwrap();
            assert_eq!(attacks.len(), 3);
            for attack in attacks {
                assert!(
                    attack["destroy_seconds"].as_f64().unwrap()
                        < attack["tear_seconds"].as_f64().unwrap()
                );
            }
        }
    }
}

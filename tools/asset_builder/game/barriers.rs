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

fn bind_actor_clips(
    game: &mut Value,
    decoded: &[crate::animation::DecodedClip],
    required: &BTreeSet<String>,
) -> Result<()> {
    let models = game["content"]["enemies"]
        .as_array()
        .ok_or("Native barrier actor profiles are missing")?
        .iter()
        .flat_map(|enemy| {
            std::iter::once(enemy["presentation"].as_str())
                .chain(std::iter::once(enemy["gibs"]["beheaded"].as_str()))
                .chain(
                    enemy["gibs"]["models"]
                        .as_object()
                        .into_iter()
                        .flat_map(|models| {
                            models
                                .values()
                                .flat_map(|model| [model["body"].as_str(), model["legs"].as_str()])
                        }),
                )
                .flatten()
                .map(str::to_owned)
        })
        .collect::<BTreeSet<_>>();
    for presentation in models {
        let model = presentation
            .strip_prefix("actor.")
            .ok_or("Native barrier actor presentation has no model")?;
        for decoded in decoded
            .iter()
            .filter(|decoded| required.contains(&decoded.clip.name))
        {
            let seconds = decoded.clip.frames as f32 / decoded.clip.frame_rate;
            let id = format!("animation.{model}.{}", decoded.clip.name);
            append(
                game,
                "animations",
                json!({"id":id,"source_clip":decoded.clip.name,"path":null,"skeleton":presentation,"seconds":seconds,"notifications":decoded.clip.notifies.iter().filter_map(|(name,time)|name.as_ref().map(|name|json!({"event":name,"seconds":time*seconds}))).collect::<Vec<_>>(),"looping":decoded.clip.source_flags[0]!=0,"root_motion":super::enemies::root_motion(decoded)}),
            )?;
            let row = game["content"]["presentations"]
                .as_array_mut()
                .and_then(|rows| rows.iter_mut().find(|row| row["id"] == presentation))
                .ok_or("Native barrier actor presentation is absent")?;
            let animations = row["animations"]
                .as_array_mut()
                .ok_or("Native barrier actor animation list is invalid")?;
            if !animations.iter().any(|animation| animation == &json!(id)) {
                animations.push(json!(id));
            }
        }
    }
    Ok(())
}

pub(super) fn read(
    catalog: &Catalog<'_>,
    game: &mut Value,
    entities: &[crate::entities::Entity],
    _missing: &mut Vec<String>,
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
    let mut actor_actions = serde_json::Map::new();
    let spawner = super::player::source("source/server-patch/maps/mp/zombies/_zm_spawner.gsc")?;
    for expression in [
        "setdvar( \"zombie_taunt_freq\", \"5\" );",
        "setdvar( \"zombie_reachin_freq\", \"50\" );",
        "level.attack_player_thru_boards_range = 109.8;",
        "_trigger_dist_sq = 2601;",
        "self.attacking_spot_index - 1",
    ] {
        if !spawner.contains(expression) {
            return Err(
                format!("Native barrier actor source expression changed: {expression}").into(),
            );
        }
    }
    for (flag, state, behavior, frequency, range) in [
        (60, 68, "taunt", 5, 0.),
        (64, 70, "reach_through", 50, 109.8_f32 * 0.0254),
    ] {
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
            if name.is_empty() {
                return Err(
                    format!("Native barrier {behavior} enabled with no source state").into(),
                );
            }
            let selected = super::enemies::state_clips(catalog, &name)?;
            let clips = selected
                .iter()
                .map(|(_, clip)| clip.clone())
                .collect::<BTreeSet<_>>();
            let decoded = crate::animation::read(catalog, Some(&clips))?;
            let mut actions = Vec::new();
            for (_, clip) in selected {
                let decoded = decoded
                    .iter()
                    .find(|decoded| decoded.clip.name == clip)
                    .ok_or("Native barrier action clip did not decode")?;
                let duration = decoded.clip.frames as f32 / decoded.clip.frame_rate;
                let seconds = decoded
                    .clip
                    .notifies
                    .iter()
                    .find(|(name, _)| name.as_deref() == Some("end"))
                    .map(|(_, time)| time * duration)
                    .ok_or_else(|| {
                        format!("Native barrier {behavior} clip {clip} has no source end notetrack")
                    })?;
                let hits = decoded
                    .clip
                    .notifies
                    .iter()
                    .filter(|(name, _)| name.as_deref() == Some("fire"))
                    .map(|(_, time)| time * duration)
                    .collect::<Vec<_>>();
                actions.push(json!({"name":clip,"seconds":seconds,"duration":duration,"hits":hits,"motion":super::enemies::root_motion(decoded)}));
            }
            bind_actor_clips(game, &decoded, &clips)?;
            actor_actions.insert(
                behavior.into(),
                json!({"state":name,"frequency":frequency,"range":range,"clips":actions}),
            );
        }
    }
    game["_native_barrier_actions"] = json!(actor_actions);
    let mut entry = serde_json::Map::new();
    for (kind, state) in [
        ("standing", "zm_barricade_enter"),
        ("crawler", "zm_barricade_enter_crawl"),
    ] {
        let selected = super::enemies::state_clips(catalog, state)?;
        let required = selected
            .iter()
            .map(|(_, clip)| clip.clone())
            .collect::<BTreeSet<_>>();
        let decoded = crate::animation::read(catalog, Some(&required))?;
        let mut clips = serde_json::Map::new();
        for (alias, name) in selected {
            let decoded = decoded
                .iter()
                .find(|decoded| decoded.clip.name == name)
                .ok_or("Native barricade entry clip did not decode")?;
            let duration = decoded.clip.frames as f32 / decoded.clip.frame_rate;
            let seconds = decoded
                .clip
                .notifies
                .iter()
                .find(|(name, _)| name.as_deref() == Some("end"))
                .map(|(_, time)| time * duration)
                .ok_or_else(|| {
                    format!("Native barricade entry {name} has no source end notetrack")
                })?;
            clips.entry(alias).or_insert_with(||json!([])).as_array_mut().unwrap().push(json!({"name":name,"seconds":seconds,"duration":duration,"hits":[],"motion":motion(decoded)?}));
        }
        bind_actor_clips(game, &decoded, &required)?;
        entry.insert(kind.into(), json!(clips));
    }
    game["_native_barrier_entry"] = json!(entry);
    let repair_quake = if streams.u32(root.add(28)?)? != 0 {
        let values = [32, 36, 40, 44, 48]
            .map(|offset| streams.u32(root.add(offset)?).map(f32::from_bits))
            .into_iter()
            .collect::<std::result::Result<Vec<_>, _>>()?;
        if values.iter().any(|value| !value.is_finite())
            || values[0] < 0.
            || values[1] < values[0]
            || values[2] <= 0.
            || values[3] < values[2]
            || values[4] < 0.
        {
            return Err("Native barrier repair earthquake range is invalid".into());
        }
        json!({"scale":[values[0],values[1]],"duration":[values[2],values[3]],"radius":values[4]*0.0254})
    } else {
        Value::Null
    };
    let mut source = Vec::new();
    let mut required = BTreeSet::new();
    let mut actor_required = BTreeSet::new();
    let mut board_effects = Vec::new();
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
        if !min_pause.is_finite()
            || !max_pause.is_finite()
            || min_pause < 0.
            || max_pause < min_pause
        {
            return Err(format!(
                "Native barrier board {index} repair sound pause range is invalid"
            )
            .into());
        }
        board_audio.push(
            json!({"opening":resolve_sound(repair_sound)?,"hover":resolve_sound(hover_sound)?,
            "repeat_pause":if pause_repeat != 0 {json!([min_pause,max_pause])} else {Value::Null},"effect_delay":[min_pause,max_pause]}),
        );
        let effects = [20, 24]
            .into_iter()
            .zip([28, 40])
            .map(|(pointer, offset)| -> Result<Value> {
                if streams.u32(board.add(pointer)?)? == 0 {
                    return Ok(Value::Null);
                }
                let name = catalog.pointer(zone, board.add(pointer)?, 33)?.0;
                let [x, y, z] = [0, 4, 8]
                    .map(|axis| streams.u32(board.add(offset + axis)?).map(f32::from_bits))
                    .into_iter()
                    .collect::<std::result::Result<Vec<_>, _>>()?
                    .try_into()
                    .unwrap();
                if [x, y, z].into_iter().any(|value| !value.is_finite()) {
                    return Err(format!(
                        "Native barrier board {index} repair effect offset is invalid"
                    )
                    .into());
                }
                Ok(json!({"name":name,"offset":[x*0.0254,z*0.0254,-y*0.0254]}))
            })
            .collect::<Result<Vec<_>>>()?;
        board_effects.push(json!(effects));
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
        let select = |slot: usize, suffix: &str, crawler: bool| -> Result<String> {
            let alias = format!("spot_{slot}_piece_{piece}");
            let state = format!("{state}_{suffix}");
            let mut clips = super::enemies::state_clips(catalog, &state)?
                .into_iter()
                .enumerate()
                .filter(|(_, (substate, _))| substate == &alias);
            let clip = clips
                .next()
                .ok_or_else(|| format!("Native barrier ASD state {state} lacks {alias}"))?;
            if clips.next().is_some() {
                return Err(
                    format!("Native barrier ASD state {state}/{alias} is ambiguous").into(),
                );
            }
            if crawler {
                Ok(super::enemies::state_clips(catalog,&format!("{state}_crawl"))?.get(clip.0).ok_or_else(||format!("Native crawler barrier state {state}_crawl lacks standing substate index {}",clip.0))?.1.clone())
            } else {
                Ok(clip.1.1)
            }
        };
        let mut attacks = Vec::new();
        for slot in 0..slots as usize {
            let grab = select(slot, "in", false)?;
            let tear = select(slot, "out", false)?;
            let crawl_grab = select(slot, "in", true)?;
            let crawl_tear = select(slot, "out", true)?;
            actor_required.extend([
                grab.clone(),
                tear.clone(),
                crawl_grab.clone(),
                crawl_tear.clone(),
            ]);
            required.extend([
                grab.clone(),
                tear.clone(),
                crawl_grab.clone(),
                crawl_tear.clone(),
            ]);
            attacks.push((grab, tear, crawl_grab, crawl_tear));
        }
        required.extend([repair.clone(), pull.clone()]);
        source.push((model, repair, pull, attacks));
    }
    let decoded = crate::animation::read(catalog, Some(&required))?;
    bind_actor_clips(game, &decoded, &actor_required)?;
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
                json!({"id":id,"source_clip":name,"path":null,"skeleton":presentation,"seconds":duration,"notifications":notifications,"looping":decoded.clip.source_flags[0]!=0,"root_motion":super::enemies::root_motion(decoded)}),
            )?;
        }
        append(
            game,
            "presentations",
            json!({"id":presentation,"source_model":model,"scene":null,"animations":ids,"sounds":[]}),
        )?;
        let mut variants = Vec::new();
        let attack = |grab: &str, tear: &str| -> Result<Value> {
            let grab_decoded = clip(&grab)?;
            let tear_decoded = clip(&tear)?;
            let end = |decoded: &crate::animation::DecodedClip| {
                decoded
                    .clip
                    .notifies
                    .iter()
                    .find(|(name, _)| name.as_deref() == Some("end"))
                    .map(|(_, fraction)| {
                        fraction * decoded.clip.frames as f32 / decoded.clip.frame_rate
                    })
                    .ok_or_else(|| {
                        format!(
                            "Native barrier actor clip {} lacks source end notetrack",
                            decoded.clip.name
                        )
                    })
            };
            let grab_seconds = end(grab_decoded)?;
            let tear_seconds = end(tear_decoded)?;
            let tear_duration = tear_decoded.clip.frames as f32 / tear_decoded.clip.frame_rate;
            let destroy: Vec<_> = tear_decoded
                .clip
                .notifies
                .iter()
                .filter(|(name, _)| {
                    name.as_deref()
                        .is_some_and(|name| matches!(name, "board" | "destroy_piece" | "bar"))
                })
                .map(|(_, fraction)| fraction * tear_duration)
                .collect();
            if destroy.len() != 1 {
                return Err(format!(
                    "Barrier board {index} actor tear {tear} has {} source destroy notifies",
                    destroy.len()
                )
                .into());
            }
            Ok(
                json!({"grab_seconds":grab_seconds,"tear_seconds":tear_seconds,
            "destroy_seconds":destroy[0],"grab_clip":grab,"tear_clip":tear,"grab_motion":motion(grab_decoded)?,"tear_motion":motion(tear_decoded)?}),
            )
        };
        for (grab, tear, crawl_grab, crawl_tear) in attacks {
            let mut variant = attack(&grab, &tear)?;
            variant["crawler"] = attack(&crawl_grab, &crawl_tear)?;
            variants.push(variant);
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
        for ((contents, flags), triangles) in batches {
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
        board["repair_effects"] = board_effects[index].clone();
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
    game["_native_barrier_quake"] = repair_quake;
    game["_native_barrier_audio"] = json!({"repair":[resolve_sound(general[0])?,resolve_sound(general[1])?],
        "repair_delay":if delay != 0 {json!(delay_seconds)}else{Value::Null}});
    game["audio"]["bindings"]
        .as_object_mut()
        .ok_or("Source audio bindings are not an object")?
        .extend(audio_bindings);
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
    let source =
        super::player::source("source/server-tomb/maps/mp/zm_tomb.gsc")?.replace("\r\n", "\n");
    let drop_barriers = source
        .split_once("\ndrop_all_barriers()\n")
        .and_then(|(_, body)| {
            body.split_once("\nget_all_zone_zbarriers(")
                .map(|(body, _)| body)
        })
        .ok_or("Recovered Origins initial barrier state function is absent")?;
    for expression in ["level thread drop_all_barriers();"] {
        if !source.contains(expression) {
            return Err(format!(
                "Recovered Origins initial barrier state requires conversion: {expression}"
            )
            .into());
        }
    }
    for expression in [
        "zbarriers = get_all_zone_zbarriers( zkeys[z] );",
        "zbarrier hidezbarrierpiece( i );",
        "zbarrier setzbarrierpiecestate( i, \"open\" );",
    ] {
        if !drop_barriers.contains(expression) {
            return Err(format!(
                "Recovered Origins initial barrier state requires conversion: {expression}"
            )
            .into());
        }
    }
    let boards = game
        .as_object_mut()
        .ok_or("Initial game source is not an object")?
        .remove("_native_barrier_boards")
        .ok_or("Native ordinary barrier source was not read")?;
    let attack_spacing = boards["attack_spacing"]
        .as_f64()
        .filter(|spacing| spacing.is_finite() && *spacing > 0.)
        .ok_or("Native ordinary barrier attack spacing is invalid")?;
    let audio = game
        .as_object_mut()
        .unwrap()
        .remove("_native_barrier_audio")
        .ok_or("Native ordinary barrier audio was not read")?;
    let repair_quake = game
        .as_object_mut()
        .unwrap()
        .remove("_native_barrier_quake")
        .ok_or("Native ordinary barrier earthquake was not read")?;
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
        let initially_open = entity.get("script_string").is_some_and(|source_zone| {
            let zone = format!(
                "zone.{}",
                source_zone.strip_prefix("zone_").unwrap_or(source_zone)
            );
            game["content"]["zones"]
                .as_array()
                .is_some_and(|zones| zones.iter().any(|row| row["id"] == zone))
                && entities.iter().any(|volume| {
                    volume
                        .get("classname")
                        .is_some_and(|class| class == "info_volume")
                        && volume.get("targetname") == Some(source_zone)
                        && volume.contains_key("target")
                })
        });
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
        let [attack_pitch, attack_yaw, attack_roll] =
            crate::entities::vector(goal, "angles")?.unwrap_or([0.; 3]);
        if attack_pitch != 0. || attack_roll != 0. || !attack_yaw.is_finite() {
            return Err(
                format!("Initial barrier {target} requires tilted attack slot placement").into(),
            );
        }
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
            json!({"id":placement,"definition":id,"position":[x*0.0254,z*0.0254,-y*0.0254],"yaw":yaw.to_radians(),"zone":zone}),
        )?;
        append(
            game,
            "interactions",
            json!({"id":format!("interaction.barrier.native.{index}"),"placement":placement,"action":{"kind":"barrier","barrier":id},"reach":repair_radius,"hold_seconds":0.25,"hidden_prompt":false}),
        )?;
        barriers.push(json!({"id":id,"placement":placement,"entry":link["from"],"exit":exit,"initially_open":initially_open,"attack_spacing":attack_spacing,"attack_yaw":attack_yaw.to_radians(),"boards":boards["boards"],"audio":audio,"repair_quake":repair_quake,
            "repair_origin":[rx*0.0254,rz*0.0254,-ry*0.0254],"repair_yaw":(repair_yaw-90.).to_radians(),"repair_radius":repair_radius,"repair_height":64_f32*0.0254,"repair_interval":1.,"repair_points":repair_points}));
    }
    if barriers.is_empty() {
        return Err("Initial native path routes have no authored wood barrier binding".into());
    }
    let actions = game
        .as_object_mut()
        .ok_or("Native barrier game is invalid")?
        .remove("_native_barrier_actions")
        .ok_or("Native barrier action source is missing")?;
    let enter = game
        .as_object_mut()
        .ok_or("Native barrier game is invalid")?
        .remove("_native_barrier_entry")
        .ok_or("Native barricade entry source is missing")?;
    for barrier in &mut barriers {
        barrier["taunt"] = actions["taunt"].clone();
        barrier["reach_through"] = actions["reach_through"].clone();
        barrier["enter"] = enter.clone();
        barrier["repair_yaw"] = json!(
            barrier["repair_yaw"]
                .as_f64()
                .ok_or("Native barrier repair yaw is invalid")?
                + std::f64::consts::FRAC_PI_2
        );
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
        let (zone, root) = catalog.resolve(59, "zmcore_BasicWoodBarrier").unwrap();
        let streams = &catalog.zones[zone].1.streams;
        println!(
            "Native barrier earthquake fields32/36/40/44/48: {:?}",
            std::array::from_fn::<_, 5, _>(|index| f32::from_bits(
                streams.u32(root.add(32 + index * 4).unwrap()).unwrap()
            ))
        );
        let (game, missing) = super::super::read(&catalog, &entities).unwrap();
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
                assert!(attack["crawler"].is_object());
                assert!(
                    attack["crawler"]["destroy_seconds"].as_f64().unwrap()
                        < attack["crawler"]["tear_seconds"].as_f64().unwrap()
                );
                assert!(
                    attack["destroy_seconds"].as_f64().unwrap()
                        < attack["tear_seconds"].as_f64().unwrap()
                );
            }
        }
    }
}

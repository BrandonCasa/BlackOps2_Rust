use super::player::{append, pose, source};
use super::*;
use std::collections::{BTreeMap, BTreeSet};

const GENERATOR: &str = "generator_start_bunker";
const CAPTURE_ENEMY: &str = "enemy.actor_zm_tomb_basic_zone_capture.0";

fn character_field(source: &str, field: &str) -> Result<String> {
    source
        .split_once(&format!("self.{field} = \""))
        .and_then(|(_, tail)| tail.split_once('"'))
        .map(|(value, _)| value.to_owned())
        .ok_or_else(|| format!("Initial capture character field is missing: {field}").into())
}

pub(super) fn read(
    catalog: &Catalog<'_>,
    game: &mut Value,
    entities: &[crate::entities::Entity],
    _gaps: &mut Vec<String>,
) -> Result<()> {
    let capture = source("source/server-tomb/maps/mp/zm_tomb_capture_zones.gsc")?;
    let unitrigger = source("source/server-patch/maps/mp/zombies/_zm_unitrigger.gsc")?;
    if !unitrigger.contains("box_radius = length( ( unitrigger_stub.script_width / 2, unitrigger_stub.script_length / 2, unitrigger_stub.script_height / 2 ) );") || !unitrigger.contains("level._unitriggers.largest_radius = min( 113.0, unitrigger_stub.radius );") {
        return Err("Initial native unitrigger global use radius source changed".into());
    }
    if entities
        .iter()
        .any(|entity| entity.contains_key("script_unitrigger_type"))
    {
        return Err(
            "Initial native unitrigger global radius requires authored stub recovery".into(),
        );
    }
    game["tuning"]["use_radius"] = json!((3_f32).sqrt() * 64. * 0.0254);
    let character = source(
        "evidence/scripts-complete-20261012/decompiled/zm_tomb/character/c_zom_tomb_crusader_zc.gsc",
    )?;
    for expression in [
        "s_unitrigger_stub.script_length = 128;",
        "s_unitrigger_stub.script_width = 128;",
        "s_unitrigger_stub.script_height = 128;",
        "return 200 * get_players().size;",
        "level.zone_capture.rate_capture_solo = get_update_rate( 12 );",
        "level.zone_capture.rate_decay = get_update_rate( 20 ) * -1;",
        "distance2dsquared( player.origin, self.origin ) < 48400",
        "s_spawn_point = random( a_valid_spawn_points );",
    ] {
        if !capture.contains(expression) {
            return Err(format!(
                "Initial generator source expression is unsupported: {expression}"
            )
            .into());
        }
    }
    let authored = entities
        .iter()
        .enumerate()
        .filter(|(_, entity)| {
            entity
                .get("classname")
                .is_some_and(|class| class == "script_struct")
                && entity
                    .get("targetname")
                    .is_some_and(|name| name == "s_generator")
                && entity
                    .get("script_noteworthy")
                    .is_some_and(|name| name == GENERATOR)
        })
        .collect::<Vec<_>>();
    let [(index, generator)] = authored.as_slice() else {
        return Err("Initial native generator owner is missing or ambiguous".into());
    };
    if generator.get("script_int").map(String::as_str) != Some("1") {
        return Err("Initial native generator does not have source objective1".into());
    }
    let target = generator
        .get("target")
        .ok_or("Initial native generator has no emergence target")?;
    let emergence = entities
        .iter()
        .filter(|entity| {
            entity
                .get("classname")
                .is_some_and(|class| class == "script_struct")
                && entity.get("targetname") == Some(target)
        })
        .map(|entity| {
            let (position, yaw) = pose(entity)?;
            let angles = crate::entities::vector(entity, "angles")?.unwrap_or([0.; 3]);
            if angles[0] != 0. || angles[2] != 0. {
                return Err(
                    "Initial capture emergence requires unsupported tilted source angles".into(),
                );
            }
            Ok(json!({"position":position,"yaw":yaw}))
        })
        .collect::<Result<Vec<_>>>()?;
    if emergence.is_empty() {
        return Err("Initial native generator has no linked emergence structs".into());
    }
    let models = entities
        .iter()
        .filter(|entity| {
            entity
                .get("targetname")
                .is_some_and(|name| name == GENERATOR)
                && entity.get("model").is_some()
        })
        .collect::<Vec<_>>();
    let [model_entity] = models.as_slice() else {
        return Err("Initial native generator visual model is missing or ambiguous".into());
    };
    let model = model_entity
        .get("model")
        .ok_or("Initial native generator model name is absent")?;
    let spawners = entities
        .iter()
        .filter(|entity| {
            entity
                .get("targetname")
                .is_some_and(|name| name == "capture_zombie_spawner")
        })
        .collect::<Vec<_>>();
    let [spawner] = spawners.as_slice() else {
        return Err("Initial native capture actor spawner is missing or ambiguous".into());
    };
    if spawner.get("classname").map(String::as_str) != Some("actor_zm_tomb_basic_zone_capture") {
        return Err(
            format!("Initial capture requires source archetype recovery for {spawner:?}").into(),
        );
    }
    if spawner.get("script_noteworthy").map(String::as_str) != Some("capture_zombie")
        || spawner.contains_key("target")
    {
        return Err("Initial capture actor source scoring/rise target binding changed".into());
    }
    let pump = [
        "fxanim_zom_tomb_generator_start_anim",
        "fxanim_zom_tomb_generator_up_idle_anim",
        "fxanim_zom_tomb_generator_down_idle_anim",
        "fxanim_zom_tomb_generator_end_anim",
        "fxanim_zom_tomb_generator_fluid_up_anim",
        "fxanim_zom_tomb_generator_fluid_down_anim",
        "fxanim_zom_tomb_generator_fluid_rotate_up_anim",
        "fxanim_zom_tomb_generator_fluid_rotate_down_anim",
    ];
    let challenges = source("source/server-tomb/maps/mp/zombies/_zm_challenges.gsc")?;
    let utility = source("source/server-tomb/maps/mp/zm_tomb_utility.gsc")?;
    let powerups = source("source/server-patch/maps/mp/zombies/_zm_powerups.gsc")?;
    if !utility.contains("reward_powerup_double_points, -1")
        || !challenges.contains("s_unitrigger_stub.script_length = 64;")
        || !challenges.contains("s_unitrigger_stub.radius = 64;")
        || !powerups.contains("set_zombie_var( \"zombie_powerup_point_doubler_time\", 30,")
        || !powerups.contains("add_zombie_powerup( \"double_points\", \"zombie_x2_icon\"")
    {
        return Err("Initial generator chest source binding changed".into());
    }
    let chests = entities
        .iter()
        .filter(|entity| {
            entity
                .get("targetname")
                .is_some_and(|name| name == "challenge_box")
                && entity
                    .get("script_string")
                    .is_some_and(|name| name == "start_bunker")
        })
        .collect::<Vec<_>>();
    let [chest] = chests.as_slice() else {
        return Err("Initial native generator challenge chest is missing or ambiguous".into());
    };
    if chest.get("model").map(String::as_str) != Some("p6_zm_tm_challenge_box") {
        return Err("Initial challenge chest native model differs from source".into());
    }
    let (chest_position, chest_yaw) = pose(chest)?;
    let chest_clips = [
        "o_zombie_dlc4_challenge_box_open",
        "o_zombie_dlc4_challenge_box_close",
    ];
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
        ("generator_melee", "zm_generator_melee"),
    ];
    let mut states = BTreeMap::new();
    for state in locomotion
        .iter()
        .chain(combat.iter())
        .map(|(_, state)| *state)
        .chain([
            "zm_idle",
            "zm_dug_rise",
            "zm_traverse_barrier",
            "zm_traverse_barrier_crawl",
        ])
    {
        states.insert(state, super::enemies::state_clips(catalog, state)?);
    }
    let mut required = states
        .values()
        .flat_map(|clips| clips.iter().map(|(_, clip)| clip.clone()))
        .collect::<BTreeSet<_>>();
    required.extend(pump.into_iter().map(str::to_owned));
    required.extend(chest_clips.into_iter().map(str::to_owned));
    let decoded = crate::animation::read(catalog, Some(&required))?;
    let clips = decoded
        .iter()
        .map(|clip| (clip.clip.name.as_str(), clip))
        .collect::<BTreeMap<_, _>>();
    let body = "c_zom_tomb_crusader_body_zc";
    let head = "c_zom_tomb_crusader_headz";
    let hat = "c_zom_tomb_crusader_hat_3";
    let mut pieces = Vec::new();
    let mut gib_sources = BTreeSet::new();
    for index in 1..=4 {
        let model = character_field(&character, &format!("gibspawn{index}"))?;
        let joint =
            character_field(&character, &format!("gibspawntag{index}"))?.to_ascii_lowercase();
        pieces.push(super::enemies::gib_piece(catalog, &model, &joint)?);
        append(
            game,
            "presentations",
            json!({"id":format!("actor.{model}"),"source_model":model,"scene":null,"animations":[],"sounds":[]}),
        )?;
    }
    let mut gib_models = serde_json::Map::new();
    for (kind, torso, legs) in [
        ("right_arm", "torsodmg2", "legdmg1"),
        ("left_arm", "torsodmg3", "legdmg1"),
        ("right_leg", "torsodmg1", "legdmg2"),
        ("left_leg", "torsodmg1", "legdmg3"),
        ("no_legs", "torsodmg1", "legdmg4"),
    ] {
        let torso = character_field(&character, torso)?;
        let legs = character_field(&character, legs)?;
        gib_sources.extend([torso.clone(), legs.clone()]);
        gib_models.insert(
            kind.into(),
            json!({"body":format!("actor.{torso}"),"legs":format!("actor.{legs}")}),
        );
    }
    let beheaded = character_field(&character, "torsodmg5")?;
    gib_sources.insert(beheaded.clone());
    let hat_source = character_field(&character, "gibspawn5")?;
    let hat_joint = character_field(&character, "gibspawntag5")?.to_ascii_lowercase();
    if hat_source != hat {
        return Err("Initial capture source hat fragment differs from attached hat".into());
    }
    let hat_piece = super::enemies::gib_piece(catalog, &hat_source, &hat_joint)?;
    let ragdoll =
        super::enemies::ragdoll_model(catalog, body, &super::enemies::ragdoll_profile(catalog)?)?;
    let mut presentation_models = vec![
        (
            model.as_str(),
            pump.iter()
                .map(|name| name.to_string())
                .collect::<BTreeSet<_>>(),
        ),
        (
            body,
            states
                .values()
                .flat_map(|clips| clips.iter().map(|(_, clip)| clip.clone()))
                .collect(),
        ),
        (head, BTreeSet::new()),
        (hat, BTreeSet::new()),
        (
            "p6_zm_tm_challenge_box",
            chest_clips.into_iter().map(str::to_owned).collect(),
        ),
        ("zombie_x2_icon", BTreeSet::new()),
        ("p6_zm_tm_zone_capture_hole", BTreeSet::new()),
    ];
    for gib in &gib_sources {
        presentation_models.push((
            gib.as_str(),
            states
                .values()
                .flat_map(|clips| clips.iter().map(|(_, clip)| clip.clone()))
                .collect(),
        ));
    }
    for (name, names) in presentation_models {
        catalog.resolve(5, name)?;
        let mut animations = Vec::new();
        for clip in names {
            let decoded = clips[clip.as_str()];
            let seconds = decoded.clip.frames as f32 / decoded.clip.frame_rate;
            let id = format!("animation.{name}.{clip}");
            animations.push(id.clone());
            append(
                game,
                "animations",
                json!({"id":id,"source_clip":clip,"path":null,"skeleton":format!("actor.{name}"),"seconds":seconds,"notifications":decoded.clip.notifies.iter().filter_map(|(event,time)|event.as_ref().map(|event|json!({"seconds":time*seconds,"event":event}))).collect::<Vec<_>>(),"root_motion":super::enemies::root_motion(decoded),"looping":decoded.clip.source_flags[0]!=0}),
            )?;
        }
        if name == model {
            animations = pump
                .iter()
                .map(|clip| format!("animation.{name}.{clip}"))
                .collect();
        }
        if let Some(existing) = game["content"]["presentations"]
            .as_array_mut()
            .and_then(|rows| {
                rows.iter_mut()
                    .find(|row| row["id"] == format!("actor.{name}"))
            })
        {
            let mut merged = existing["animations"]
                .as_array()
                .ok_or("Initial capture shared presentation animations are invalid")?
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_owned)
                .collect::<BTreeSet<_>>();
            merged.extend(animations);
            existing["animations"] = json!(merged);
        } else {
            append(
                game,
                "presentations",
                json!({"id":format!("actor.{name}"),"source_model":name,"scene":null,"animations":animations,"sounds":[]}),
            )?;
        }
    }
    append(
        game,
        "powerups",
        json!({"id":"powerup.double_points","duration":30.,"presentation":"actor.zombie_x2_icon"}),
    )?;
    for name in [body, head, hat] {
        if !character.contains(&format!("\"{name}\"")) {
            return Err(format!("Initial capture character source has no {name}").into());
        }
    }
    let animation_id = |clip: &str| format!("animation.{body}.{clip}");
    let actions = [
        ("idle", "zm_idle"),
        ("walk", "zm_move_walk"),
        ("run", "zm_move_run"),
        ("sprint", "zm_move_sprint"),
        ("attack", "zm_walk_melee"),
        ("death", "zm_death"),
    ]
    .into_iter()
    .map(|(action, state)| (action.to_owned(), json!(animation_id(&states[state][0].1))))
    .collect::<serde_json::Map<_, _>>();
    let locomotion = locomotion
        .into_iter()
        .map(|(action, state)| {
            (
                action.to_owned(),
                json!(
                    states[state]
                        .iter()
                        .map(|(_, clip)| animation_id(clip))
                        .collect::<Vec<_>>()
                ),
            )
        })
        .collect::<serde_json::Map<_, _>>();
    let combat = combat
        .into_iter()
        .map(|(action, state)| {
            (
                action.to_owned(),
                json!(
                    states[state]
                        .iter()
                        .map(|(_, clip)| super::enemies::combat_clip(body, clips[clip.as_str()]))
                        .collect::<Vec<_>>()
                ),
            )
        })
        .collect::<serde_json::Map<_, _>>();
    let mut traversals = serde_json::Map::new();
    for state in [
        "zm_dug_rise",
        "zm_traverse_barrier",
        "zm_traverse_barrier_crawl",
    ] {
        for (alias, name) in &states[state] {
            let alias = if state == "zm_dug_rise" {
                "rise"
            } else {
                alias
            };
            traversals
                .entry(alias.to_owned())
                .or_insert_with(|| json!([]))
                .as_array_mut()
                .unwrap()
                .push(super::enemies::traversal_clip(body, clips[name.as_str()])?);
        }
    }
    let attack = &clips[states["zm_walk_melee"][0].1.as_str()].clip;
    let death = &clips[states["zm_death"][0].1.as_str()].clip;
    let mut hit_regions = super::enemies::hit_volumes(catalog, &[body, head, hat])?;
    let mut hit_ids = hit_regions
        .iter()
        .map(|hit| hit["id"].as_str().unwrap().to_owned())
        .collect::<BTreeSet<_>>();
    for gib in &gib_sources {
        for hit in super::enemies::hit_volumes(catalog, &[body, gib])? {
            if hit_ids.insert(hit["id"].as_str().unwrap().to_owned()) {
                hit_regions.push(hit);
            }
        }
    }
    let health = game["rounds"]["health_start"].clone();
    append(
        game,
        "enemies",
        json!({"id":CAPTURE_ENEMY,"health":health,"damage":60,"family":"capture","archetype":"actor_zm_tomb_basic_zone_capture","character_group":"tomb.capture","character_index":0,"presentation":format!("actor.{body}"),"parts":[{"presentation":format!("actor.{head}"),"socket":"j_head","root_joint":"j_head"},{"presentation":format!("actor.{hat}"),"socket":"j_head","root_joint":"j_head"}],"hit_regions":hit_regions,"animation_actions":actions,"locomotion_states":locomotion,"combat_states":combat,"traversals":traversals,"gibs":{"models":gib_models,"beheaded":format!("actor.{beheaded}"),"pieces":pieces,"hat":hat_piece},"ragdoll":ragdoll,"attack_seconds":attack.frames as f32/attack.frame_rate,"attack_hits":attack.notifies.iter().filter(|(event,_)|event.as_deref()==Some("fire")).map(|(_,time)|time*attack.frames as f32/attack.frame_rate).collect::<Vec<_>>(),"death_seconds":death.frames as f32/death.frame_rate,"physics":{"radius":15.*0.0254,"height":72.*0.0254,"crawler_height":24.*0.0254}}),
    )?;
    let (position, yaw) = pose(generator)?;
    let (model_position, model_yaw) = pose(model_entity)?;
    if model_yaw != yaw {
        return Err("Initial generator visual/use source angles differ".into());
    }
    let clip_seconds = pump
        .iter()
        .map(|name| {
            let clip = &clips[*name].clip;
            clip.frames as f32 / clip.frame_rate
        })
        .collect::<Vec<_>>();
    game["_initial_generator"] = json!({"index":index,"source":generator,"position":position,"yaw":yaw,"model_position":model_position,"model":model,"emergence":emergence,"clip_seconds":clip_seconds,"reward":{"position":chest_position,"yaw":chest_yaw,"presentation":"actor.p6_zm_tm_challenge_box","powerup":"powerup.double_points","source_trigger":"trigger.generator_start_bunker.reward"}});
    Ok(())
}

pub(super) fn prepare(
    game: &mut Value,
    entities: &[crate::entities::Entity],
    _missing: &mut Vec<String>,
) -> Result<()> {
    let native = game
        .as_object_mut()
        .ok_or("Native generator source is invalid")?
        .remove("_initial_generator")
        .ok_or("Initial generator source was not read")?;
    let position: [f32; 3] = serde_json::from_value(native["position"].clone())?;
    let zone = super::navigation::zone_at(game, position)?
        .ok_or("Initial generator is outside source zones")?;
    let source_index = native["index"]
        .as_u64()
        .ok_or("Native generator entity index is invalid")? as usize;
    let matrix = crate::scenes::entity_matrix(
        entities
            .get(source_index)
            .ok_or("Native generator entity index is missing")?,
    )?;
    let forward = [matrix[0], matrix[1], matrix[2]];
    let right = [matrix[8], matrix[9], matrix[10]];
    let directions = [
        forward,
        right,
        forward.map(|value| -value),
        right.map(|value| -value),
    ];
    let attack_points = (0..12)
        .map(|index| {
            let lateral = match index / 4 {
                1 => 170. * 0.0254,
                2 => -170. * 0.0254,
                _ => 0.,
            };
            let face =
                std::array::from_fn::<_, 3, _>(|axis| position[axis] + right[axis] * lateral);
            let distance = if index < 4 {
                52. * 0.0254
            } else {
                32. * 0.0254
            };
            let point = std::array::from_fn::<_, 3, _>(|axis| {
                face[axis] + directions[index % 4][axis] * distance
            });
            json!({"position":point,"face":face,"inaccessible":matches!(index,5|11)})
        })
        .collect::<Vec<_>>();
    let size = 128. * 0.0254;
    let vertices = (0..8)
        .map(|corner| {
            let local = [
                if corner & 1 == 0 {
                    -size * 0.5
                } else {
                    size * 0.5
                },
                if corner & 2 == 0 { 0. } else { size },
                if corner & 4 == 0 {
                    -size * 0.5
                } else {
                    size * 0.5
                },
            ];
            std::array::from_fn::<_, 3, _>(|axis| {
                (0..3)
                    .map(|column| matrix[column * 4 + axis] * local[column])
                    .sum::<f32>()
            })
        })
        .collect::<Vec<_>>();
    let trigger = "trigger.generator_start_bunker.use";
    game["environment"]["triggers"].as_array_mut().ok_or("Native initial triggers are missing")?.push(json!({"id":trigger,"position":position,"source":{"classname":"trigger_box_use","targetname":"s_generator","script_noteworthy":GENERATOR},"shape":{"kind":"convex","volumes":[{"id":format!("{trigger}.volume"),"position":[0.,0.,0.],"vertices":vertices}]}}));
    game["environment"]["ownership"]
        .as_array_mut()
        .ok_or("Native initial ownership is missing")?
        .push(json!({"id":trigger,"owner":"native","shootable":false}));
    let reward = &native["reward"];
    let reward_yaw = reward["yaw"]
        .as_f64()
        .ok_or("Initial reward yaw is missing")? as f32;
    let (sin, cos) = reward_yaw.sin_cos();
    let vertices = (0..8)
        .map(|corner| {
            let x = if corner & 1 == 0 {
                -32. * 0.0254
            } else {
                32. * 0.0254
            };
            let y = if corner & 2 == 0 { 0. } else { 64. * 0.0254 };
            let z = if corner & 4 == 0 {
                -32. * 0.0254
            } else {
                32. * 0.0254
            };
            [x * cos + z * sin, y, -x * sin + z * cos]
        })
        .collect::<Vec<_>>();
    game["environment"]["triggers"].as_array_mut().ok_or("Native initial triggers are missing")?.push(json!({"id":reward["source_trigger"],"position":reward["position"],"source":{"classname":"trigger_box_use","targetname":"challenge_box","script_string":"start_bunker"},"shape":{"kind":"convex","volumes":[{"id":"trigger.generator_start_bunker.reward.volume","position":[0.,0.,0.],"vertices":vertices}]}}));
    game["environment"]["ownership"]
        .as_array_mut()
        .ok_or("Native initial ownership is missing")?
        .push(json!({"id":reward["source_trigger"],"owner":"native","shootable":false}));
    append(
        game,
        "placements",
        json!({"id":"placement.generator_start_bunker","definition":format!("generator.{GENERATOR}"),"position":position,"yaw":native["yaw"],"zone":zone}),
    )?;
    append(
        game,
        "generators",
        json!({"id":format!("generator.{GENERATOR}"),"placement":"placement.generator_start_bunker","zones":[zone],"price":200,"capture_seconds":12.,"radius":220.*0.0254,"number":1,"decay_seconds":20.,"shutdown_seconds":40.,"recovery_seconds":10.,"presentation":format!("actor.{}",native["model"].as_str().ok_or("Initial generator model is invalid")?),"model_position":native["model_position"],"emergence":native["emergence"],"clip_seconds":native["clip_seconds"],"capture_enemy":CAPTURE_ENEMY,"attack_points":attack_points,"capture_sound":"sound.native.zmb_capturezone_rise.0","active_sound":"sound.native.zmb_capturezone_rise.0","initial_reward":reward}),
    )?;
    append(
        game,
        "interactions",
        json!({"id":"interaction.generator_start_bunker","placement":"placement.generator_start_bunker","action":{"kind":"generator","generator":format!("generator.{GENERATOR}")},"reach":72.*0.0254,"hold_seconds":0.,"hidden_prompt":false,"source_trigger":trigger}),
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "requires the authorized Steam installation"]
    fn installed_initial_generator_source() {
        let zones = crate::source::read_zones(&crate::source::installation().unwrap()).unwrap();
        let catalog = Catalog::new(&zones);
        let entities = crate::entities::read(&catalog).unwrap();
        let generators = entities
            .iter()
            .enumerate()
            .filter(|(_, entity)| {
                entity
                    .get("targetname")
                    .is_some_and(|name| name == "s_generator")
                    && entity
                        .get("script_noteworthy")
                        .is_some_and(|name| name == "generator_start_bunker")
            })
            .collect::<Vec<_>>();
        assert_eq!(generators.len(), 1);
        for (index, entity) in &generators {
            println!("Initial native generator struct {index}: {}", json!(entity));
        }
        let target = generators[0].1.get("target").unwrap();
        for (index, entity) in entities.iter().enumerate().filter(|(_, entity)| {
            entity.get("targetname").is_some_and(|name| {
                matches!(
                    name.as_str(),
                    "generator_start_bunker" | "capture_zombie_spawner" | "challenge_box"
                ) || name == target
            })
        }) {
            println!(
                "Initial native generator dependency entity {index}: {}",
                json!(entity)
            );
        }
        for state in ["zm_dug_rise", "zm_generator_melee"] {
            println!(
                "Initial native generator ASD {state}: {}",
                json!(super::super::enemies::state_clips(&catalog, state).unwrap())
            );
        }
        for model in [
            "c_zom_tomb_crusader_body_zc",
            "c_zom_tomb_crusader_headz",
            "c_zom_tomb_crusader_hat_3",
            "zm_collision_perks1",
        ] {
            let (zone, root) = catalog.resolve(5, model).unwrap();
            let streams = &catalog.zones[zone].1.streams;
            println!(
                "Initial native generator model {model}: collision_surfaces={} contents={:#x}",
                streams.u32(root.add(156).unwrap()).unwrap(),
                streams.u32(root.add(160).unwrap()).unwrap()
            );
            let maps = streams.u8(root.add(220).unwrap()).unwrap() as usize;
            println!("Initial native generator model {model}: physics_maps={maps}");
            if maps > 0 {
                let table = Address::decode(streams.u32(root.add(224).unwrap()).unwrap()).unwrap();
                for index in 0..maps {
                    let map = Address::decode(streams.u32(table.add(index * 4).unwrap()).unwrap())
                        .unwrap();
                    let count = streams.u32(map).unwrap() as usize;
                    let contents = streams.u32(map.add(8).unwrap()).unwrap();
                    let geometries =
                        Address::decode(streams.u32(map.add(4).unwrap()).unwrap()).unwrap();
                    for geometry in 0..count {
                        let entry = geometries.add(geometry * 68).unwrap();
                        let bounds = std::array::from_fn::<_, 3, _>(|axis| {
                            f32::from_bits(streams.u32(entry.add(56 + axis * 4).unwrap()).unwrap())
                        });
                        let brush = streams.u32(entry).unwrap();
                        let vertices = if brush == 0 {
                            0
                        } else {
                            streams
                                .u32(Address::decode(brush).unwrap().add(84).unwrap())
                                .unwrap()
                        };
                        println!(
                            "Initial native {model} physics {index}:{geometry}: type={} contents={contents:#x} brush_vertices={vertices} half_lengths={bounds:?}",
                            streams.u32(entry.add(4).unwrap()).unwrap()
                        );
                    }
                }
            }
        }
        let mut radius = (3_f32).sqrt() * 64.;
        let mut stubs = 0;
        for entity in &entities {
            let Some(kind) = entity.get("script_unitrigger_type") else {
                continue;
            };
            let scalar = |key: &str, default: f32| {
                entity
                    .get(key)
                    .map(|value| value.parse::<f32>().unwrap())
                    .unwrap_or(default)
            };
            let candidate = match kind.as_str() {
                "unitrigger_radius" | "unitrigger_radius_use" => scalar("radius", 32.),
                "unitrigger_box" | "unitrigger_box_use" => {
                    let width = scalar("script_width", 64.);
                    let length = scalar("script_length", 64.);
                    let height = scalar("script_height", 64.);
                    scalar("radius", 0.)
                        .max((width * width + length * length + height * height).sqrt() * 0.5)
                }
                _ => continue,
            };
            stubs += 1;
            radius = radius.max(candidate.min(113.));
        }
        println!(
            "Initial native registered authored unitrigger count={stubs} source global use radius={radius} inches"
        );
        let (game, _) = super::super::read(&catalog, &entities).unwrap();
        assert_eq!(
            game["_initial_generator"]["emergence"]
                .as_array()
                .unwrap()
                .len(),
            4
        );
        assert_eq!(
            game["_initial_generator"]["model"],
            "fxanim_zom_tomb_generator_pump_mod"
        );
        let enemy = game["content"]["enemies"]
            .as_array()
            .unwrap()
            .iter()
            .find(|enemy| enemy["id"] == CAPTURE_ENEMY)
            .unwrap();
        assert_eq!(enemy["archetype"], "actor_zm_tomb_basic_zone_capture");
        assert_eq!(enemy["character_index"], 0);
        assert_eq!(enemy["damage"], 60);
        assert_eq!(
            enemy["combat_states"]["generator_melee"]
                .as_array()
                .unwrap()
                .len(),
            4
        );
        assert_eq!(enemy["traversals"]["rise"].as_array().unwrap().len(), 1);
        for alias in [
            "barrier_walk",
            "barrier_run",
            "barrier_sprint",
            "barrier_crawl",
        ] {
            assert!(
                enemy["traversals"][alias]
                    .as_array()
                    .is_some_and(|clips| !clips.is_empty())
            );
        }
    }
}

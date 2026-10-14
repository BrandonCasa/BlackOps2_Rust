use crate::{catalog::Catalog, core::*, fastfile::Address, pack::Pack};
use serde_json::{Value, json};
use std::sync::Arc;

mod audio;
mod barriers;
mod enemies;
mod equipment;
mod generator;
mod navigation;
mod player;
mod presentations;
mod triggers;
mod verification;
pub use audio::bind as bind_audio;
pub(crate) use enemies::root_motion;
use presentations::{initial_presentations, variable};
use triggers::{brushes, environment, link_owners, native_staff_trigger, owner, zone_graph};
pub use verification::check_triggers;

pub fn read(
    catalog: &Catalog<'_>,
    entities: &[crate::entities::Entity],
) -> Result<(Value, Vec<String>)> {
    if entities.iter().any(|entity| {
        entity
            .get("targetname")
            .is_some_and(|name| name == "staff_craftable_trigger")
    }) {
        let scripts = native_staff_trigger(catalog, entities)?;
        println!(
            "Preserved the unbound native staff box contact producer and map/session lifetime; {scripts} installed script definitions bind only the four element staff triggers"
        );
    }
    let mut native_gaps = Vec::new();
    let mut native_triggers = Vec::new();
    let mut native_owners = Vec::new();
    let mut crafting_benches = Vec::new();
    let mut primitive_counts = [0_usize; 2];
    for (index, entity) in entities.iter().enumerate() {
        let Some(class) = entity.get("classname") else {
            continue;
        };
        if entity
            .get("model")
            .is_some_and(|model| model.starts_with('*'))
        {
            continue;
        }
        if !matches!(class.as_str(), "trigger_radius" | "trigger_box") {
            if class.starts_with("trigger_") || class == "info_volume" {
                native_gaps.push(format!("environment native non-brush trigger geometry for trigger.native.{index} ({class})"));
            }
            continue;
        }
        primitive_counts[usize::from(class == "trigger_box")] += 1;
        let id = format!("trigger.native.{index}");
        let origin = crate::entities::vector(entity, "origin")
            .map_err(|error| format!("Native primitive trigger {id}: {error}"))?;
        let scalar = |key: &str| -> Result<Option<f32>> {
            let Some(text) = entity.get(key) else {
                return Ok(None);
            };
            let value: f32 = text.parse().map_err(|error| {
                format!("Native primitive trigger {id} has invalid {key} {text:?}: {error}")
            })?;
            Ok((value.is_finite() && value > 0.).then_some(value * 0.0254))
        };
        let height = scalar("height")?;
        let shape = if class == "trigger_radius" {
            scalar("radius")?
                .zip(height)
                .map(|(radius, height)| json!({"kind":"cylinder","radius":radius,"height":height}))
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
        if let (Some([x, y, z]), Some(shape)) = (origin, shape) {
            native_triggers.push(
                json!({"id":id,"position":[x*0.0254,z*0.0254,-y*0.0254],"source":entity,
                "shape":shape}),
            );
            let name = entity.get("targetname").map(String::as_str).unwrap_or("");
            let crafting = match name {
                "open_craftable_trigger" => Some(("open_table", "", "OPEN_CRAFTABLE", false)),
                "quadrotor_zm_craftable_trigger" => Some((
                    "equip_dieseldrone_zm",
                    "equip_dieseldrone_zm",
                    "ZM_TOMB_TQ",
                    true,
                )),
                "riotshield_zm_craftable_trigger" => Some((
                    "tomb_shield_zm",
                    "tomb_shield_zm",
                    "ZOMBIE_GRAB_RIOTSHIELD",
                    true,
                )),
                "staff_air_craftable_trigger" => {
                    Some(("elemental_staff_air", "staff_air_zm", "ZM_TOMB_PUAS", true))
                }
                "staff_fire_craftable_trigger" => Some((
                    "elemental_staff_fire",
                    "staff_fire_zm",
                    "ZM_TOMB_PUFS",
                    true,
                )),
                "staff_lightning_craftable_trigger" => Some((
                    "elemental_staff_lightning",
                    "staff_lightning_zm",
                    "ZM_TOMB_PULS",
                    true,
                )),
                "staff_water_craftable_trigger" => Some((
                    "elemental_staff_water",
                    "staff_water_zm",
                    "ZM_TOMB_PUIS",
                    true,
                )),
                "gramophone_craftable_trigger" => {
                    Some(("gramophone", "gramophone", "ZOMBIE_GRAB_GRAMOPHONE", true))
                }
                _ => None,
            };
            if let Some((equipname, weaponname, hint, persistent)) = crafting {
                let script_angles = crate::entities::vector(entity, "script_angles")?;
                let offset_angles = script_angles.unwrap_or([0.; 3]);
                let angles = if let Some(angles) =
                    script_angles.or(crate::entities::vector(entity, "angles")?)
                {
                    angles
                } else {
                    [
                        0.,
                        entity
                            .get("angle")
                            .map(|value| value.parse::<f32>())
                            .transpose()?
                            .unwrap_or(0.),
                        0.,
                    ]
                };
                let mut offset_entity = entity.clone();
                offset_entity.insert(
                    "angles".into(),
                    format!(
                        "{} {} {}",
                        offset_angles[0], offset_angles[1], offset_angles[2]
                    ),
                );
                let transform = crate::scenes::entity_matrix(&offset_entity)?;
                let origin = [x * 0.0254, z * 0.0254, -y * 0.0254];
                let position: [f32; 3] =
                    std::array::from_fn(|axis| origin[axis] - transform[8 + axis] * (6. * 0.0254));
                let dimension = |key: &str, fallback: f32| -> Result<f32> {
                    if entity.contains_key(key) {
                        scalar(key)?
                            .ok_or_else(|| format!("Crafting source {id} has invalid {key}").into())
                    } else {
                        Ok(fallback * 0.0254)
                    }
                };
                let dimensions = [
                    dimension("script_width", 100.)?,
                    dimension("script_length", 32.)?,
                    dimension("script_height", 64.)?,
                ];
                crafting_benches.push(json!({"id":format!("crafting.native.{index}"),"source_trigger":id,"targetname":name,
                    "equipname":equipname,"weaponname":weaponname,"hint":hint,"position":position,"angles":angles,"dimensions":dimensions,
                    "use_seconds":3.,"persistent":persistent,"delete_source":true,"require_look_at":true}));
                native_owners.push(json!({"id":id,"owner":"crafting","shootable":false}));
                if equipname == "gramophone" {
                    native_gaps.push(format!("crafting gramophone recipe/purchase consumer for {id} requires source lifecycle integration"));
                }
                continue;
            }
            if let Some((owner, shootable)) = owner(&json!(entity)) {
                native_owners.push(json!({"id":id,"owner":owner,"shootable":shootable}));
            } else if !name.starts_with("trig_zone_") {
                native_owners.push(json!({"id":id,"owner":"deferred","shootable":false}));
                native_gaps.push(format!("environment source consumer for {id} ({class}, targetname {name:?}) is absent from recovered scripts"));
            }
        } else {
            native_gaps.push(format!("environment native {class} {id} requires explicit source origin and positive finite dimensions; fields: {entity:?}"));
        }
    }
    if !crafting_benches.is_empty() {
        native_gaps.push(
            "crafting native per-player unitrigger look/shape/persistence lifecycle conversion"
                .into(),
        );
    }
    println!(
        "Converted {} of {} native non-brush primitive triggers ({} radius, {} boxes); {} precise native-trigger gaps remain",
        native_triggers.len(),
        primitive_counts.iter().sum::<usize>(),
        primitive_counts[0],
        primitive_counts[1],
        native_gaps.len()
    );
    println!(
        "Converted {} source crafting bench bindings; native per-player unitrigger selection/persistence remains incomplete",
        crafting_benches.len()
    );
    for gap in &native_gaps {
        println!("Pending native trigger conversion: {gap}");
    }
    let mut localized = Vec::new();
    for (name, zone, root) in catalog.definitions(24) {
        let streams = &catalog.zones[zone].1.streams;
        let text = streams.text(Address::decode(streams.u32(root)?)?)?;
        let id = format!("text.{}", name.to_ascii_lowercase());
        if !id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"._-".contains(&byte))
        {
            return Err(format!(
                "Native localization key cannot form a runtime content ID: {name}"
            )
            .into());
        }
        localized.push(json!({"id":id,"key":name,"text":text}));
    }
    if localized.is_empty() {
        return Err("Required native localization definitions are missing".into());
    }
    let maximum = variable(catalog, "zombie_max_ai", 2, json!(24))?;
    let extra = variable(catalog, "zombie_ai_per_player", 2, json!(6))?
        .as_f64()
        .ok_or("Native population increment is not numeric")?
        * 0.5;
    let ranks = [
        "m1911_zm",
        "c96_zm",
        "cz75_zm",
        "cz75dw_zm",
        "kard_zm",
        "fiveseven_zm",
        "beretta93r_zm",
        "beretta93r_extclip_zm",
        "fivesevendw_zm",
        "rnma_zm",
        "python_zm",
        "judge_zm",
        "cz75_upgraded_zm",
        "cz75dw_upgraded_zm",
        "kard_upgraded_zm",
        "fiveseven_upgraded_zm",
        "m1911_upgraded_zm",
        "c96_upgraded_zm",
        "beretta93r_upgraded_zm",
        "beretta93r_extclip_upgraded_zm",
        "fivesevendw_upgraded_zm",
        "rnma_upgraded_zm",
        "python_upgraded_zm",
        "judge_upgraded_zm",
        "ray_gun_zm",
        "raygun_mark2_zm",
        "freezegun_zm",
        "ray_gun_upgraded_zm",
        "raygun_mark2_upgraded_zm",
        "freezegun_upgraded_zm",
        "microwavegundw_zm",
        "microwavegundw_upgraded_zm",
    ]
    .into_iter()
    .enumerate()
    .map(|(rank, name)| json!({"weapon":format!("weapon.{name}"),"rank":rank}))
    .collect::<Vec<_>>();
    let rounds = json!({"health_start":variable(catalog,"zombie_health_start",2,json!(150))?,
        "health_increase":variable(catalog,"zombie_health_increase",2,json!(100))?,
        "health_multiplier":variable(catalog,"zombie_health_increase_multiplier",2,json!(0.1))?,"exponential_round":10,
        "population_base":maximum,"solo_population_increment":extra,"population_round_divisor":5,
        "population_acceleration_round":10,"population_acceleration":0.15,"early_population":[0.25,0.3,0.5,0.7,0.9],
        "ordinary_cap":maximum,"actor_cap":32,"spawn_seconds":variable(catalog,"zombie_spawn_delay",2,json!(2.0))?,"spawn_decay":0.95,
        "minimum_spawn_seconds":0.08,"initial_spawn_seconds":8.25,"first_spawn_seconds":2.5,"between_round_seconds":variable(catalog,"zombie_between_round_time",1,json!(10))?,
        "speed_multiplier":variable(catalog,"zombie_move_speed_multiplier",2,json!(8))?,"speed_random_band":35,"gait_thresholds":[35,70]});
    let solo = json!({"perk":"perk.quick_revive","seconds":10.,"bleedout_seconds":45.,"bleedout_poll_seconds":1.,
        "maximum_purchases":3,"downed_weapon":"weapon.c96_upgraded_zm","reserve_clips":2,"pistol_ranks":ranks,"pistol_replace_below":15,
        "downed_height":30_f32*0.0254,"downed_eye":22_f32*0.0254,"downed_speed":190_f32*0.15*0.0254});
    println!(
        "Converted {} native localized strings, player health and recovered round/solo-revive parameters",
        localized.len()
    );
    let mut game = json!({"version":2,"delivery_scope":"first_playable_solo",
        "rounds":rounds,"solo_revive":solo,"content":{"localized_strings":localized}});
    if !native_triggers.is_empty() {
        game["environment"] = json!({"triggers":native_triggers,"ownership":native_owners,"crafting_benches":crafting_benches});
    }
    initial_presentations(catalog, &mut game, entities)?;
    equipment::read(catalog, &mut game, &mut native_gaps)?;
    player::read(catalog, &mut game, entities, &mut native_gaps)?;
    enemies::read(catalog, &mut game, entities, &mut native_gaps)?;
    generator::read(catalog, &mut game, entities, &mut native_gaps)?;
    barriers::read(catalog, &mut game, entities, &mut native_gaps)?;
    navigation::read(catalog, &mut game)?;
    let effects = crate::effects::Effects::read(catalog, crate::effects::initial_roots(catalog, &game)?)?;
    if game["audio"]["bindings"].is_null() {
        game["audio"]["bindings"] = json!({});
    }
    for alias in &effects.sounds {
        game["audio"]["bindings"][format!("sound.fx.{alias}")] = json!(alias);
    }
    audio::read(catalog, &mut game, &mut native_gaps)?;
    Ok((game, native_gaps))
}

pub fn prepare(
    (game, native_gaps): &mut (Value, Vec<String>),
    entities: &[crate::entities::Entity],
    collision: &Value,
) -> Result<()> {
    let initial = game["delivery_scope"] == "first_playable_solo";
    if initial {
        game["environment"]
            .as_object_mut()
            .ok_or("Initial environment source is missing")?
            .remove("crafting_benches");
        for owner in game["environment"]["ownership"]
            .as_array_mut()
            .into_iter()
            .flatten()
        {
            if owner["owner"] == "crafting" {
                owner["owner"] = json!("deferred");
            }
        }
        native_gaps.retain(|gap| !gap.starts_with("crafting "));
    }
    let world = json!({"brush_entities":crate::scenes::brush_entities(entities)?});
    let (brushes, mut missing) = brushes(&world, collision)?;
    missing.extend(native_gaps.iter().cloned());
    environment(game, brushes, &mut missing)?;
    link_owners(game, entities)?;
    zone_graph(game, entities, &mut missing)?;
    navigation::prepare(game, entities, &mut missing)?;
    generator::prepare(game, entities, &mut missing)?;
    player::prepare(game, entities, &mut missing)?;
    barriers::prepare(game, entities, &mut missing)?;
    for key in [
        "mud",
        "triggers",
        "ownership",
        "digging",
        "parts",
        "crafting_benches",
    ] {
        if initial && matches!(key, "digging" | "parts" | "crafting_benches") {
            continue;
        }
        if game["environment"].get(key).is_none() {
            missing.push(format!("environment.{key}"));
        }
    }
    for key in ["doors", "volumes", "spawns"] {
        if initial && key == "doors" {
            continue;
        }
        if game["access"].get(key).is_none() {
            missing.push(format!("access.{key}"));
        }
    }
    for key in [
        "spawn",
        "yaw",
        "spawn_points",
        "waypoints",
        "survey_views",
        "rounds",
        "solo_revive",
        "environment",
        "access",
        "audio",
        "tuning",
    ] {
        if initial && matches!(key, "waypoints" | "survey_views") {
            continue;
        }
        if game.get(key).is_none() {
            missing.push(key.to_owned());
        }
    }
    for (profile, fields) in [(
        "solo_revive",
        &["downed_height", "downed_eye", "downed_speed"][..],
    )] {
        for field in fields {
            if game[profile].get(field).is_none() {
                missing.push(format!("{profile}.{field}"));
            }
        }
    }
    for key in [
        "barriers",
        "weapons",
        "enemies",
        "perks",
        "powerups",
        "zones",
        "definitions",
        "placements",
        "interactions",
        "zone_connections",
        "navigation_nodes",
        "navigation_links",
        "generators",
        "robot_routes",
        "tank_routes",
        "weather",
        "digging",
        "recipes",
        "staff_puzzles",
        "quests",
        "quest_transitions",
        "presentations",
        "sounds",
        "animations",
        "ui_assets",
        "cinematics",
    ] {
        if initial
            && matches!(
                key,
                "powerups"
                    | "generators"
                    | "robot_routes"
                    | "tank_routes"
                    | "weather"
                    | "digging"
                    | "recipes"
                    | "staff_puzzles"
                    | "quests"
                    | "quest_transitions"
                    | "cinematics"
            )
        {
            continue;
        }
        if !game["content"][key].is_array() {
            missing.push(format!("content.{key}"));
        }
    }
    if !missing.is_empty() {
        return Err(format!(
            "Required game.json source conversion is incomplete: {}. No runtime pack was written.",
            missing.join(", ")
        )
        .into());
    }
    Ok(())
}

pub fn write(mut game: Value, worlds: &[Value], pack: &mut Pack) -> Result<()> {
    let world = worlds
        .iter()
        .find(|world| world["map"] == "zm_tomb")
        .ok_or("Required Origins world scene is missing")?;
    for (target, source) in [("scene", "uri"), ("collision_scene", "collision_uri")] {
        let uri = world[source]
            .as_str()
            .ok_or_else(|| format!("Prepared world is missing {source}"))?;
        pack.get(uri)?;
        game[target] = json!(format!("{uri}#Scene0"));
    }
    for key in [
        "source_surfaces",
        "render_meshes",
        "collision_meshes",
        "static_model_instances",
    ] {
        let count = world[key]
            .as_u64()
            .ok_or_else(|| format!("Prepared world is missing {key}"))?;
        game[key] = json!(count);
    }
    for brush in game["environment"]["brush_entities"]
        .as_array_mut()
        .into_iter()
        .flatten()
    {
        let source = world["brush_entities"]
            .as_array()
            .into_iter()
            .flatten()
            .find(|source| source["id"] == brush["id"])
            .ok_or_else(|| {
                format!(
                    "Prepared world has no native brush binding: {}",
                    brush["id"]
                )
            })?;
        let count = source["visual_meshes"]
            .as_u64()
            .ok_or("Prepared native brush visual mesh count is missing")?;
        brush["visual_meshes"] = json!(count);
    }
    pack.entries
        .insert("game.json".into(), Arc::new(serde_json::to_vec(&game)?));
    Ok(())
}

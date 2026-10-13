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

pub fn read(catalog: &Catalog<'_>) -> Result<Value> {
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
        "ordinary_cap":maximum,"spawn_seconds":variable(catalog,"zombie_spawn_delay",2,json!(2.0))?,"spawn_decay":0.95,
        "minimum_spawn_seconds":0.08,"first_spawn_seconds":2.5,"between_round_seconds":variable(catalog,"zombie_between_round_time",1,json!(10))?,
        "speed_multiplier":variable(catalog,"zombie_move_speed_multiplier",2,json!(8))?,"speed_random_band":35,"gait_thresholds":[35,70]});
    let solo = json!({"perk":"perk.quick_revive","seconds":10.,"bleedout_seconds":45.,"bleedout_poll_seconds":1.,
        "maximum_purchases":3,"downed_weapon":"weapon.c96_upgraded_zm","reserve_clips":2,"pistol_ranks":ranks,"pistol_replace_below":15});
    println!("Converted {} native localized strings, player health and recovered round/solo-revive parameters", localized.len());
    Ok(json!({"version":2,"player_health":{
        "maximum":100,"regular_delay":2.4,"regular_step_fraction":1.,
        "critical_delay":5.,"critical_ratio":0.2,"critical_step_seconds":0.05,"critical_step_fraction":0.1,
        "worthy_damage_ratio":0.1,"immunity_regular":0.35,"immunity_critical_enter":0.5,
        "immunity_critical_repeat":0.3,"feedback_seconds":0.8},
        "rounds":rounds,"solo_revive":solo,"content":{"localized_strings":localized}}))
}

pub fn write(mut game: Value, worlds: &[Value], pack: &mut Pack) -> Result<()> {
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
    let mut missing = Vec::new();
    for key in ["spawn", "yaw", "spawn_points", "waypoints", "survey_views", "rounds", "solo_revive", "environment", "access", "audio", "tuning"] {
        if game.get(key).is_none() { missing.push(key.to_owned()); }
    }
    for (profile, fields) in [("rounds", &["actor_cap", "gait_speeds"][..]), ("solo_revive", &["downed_height", "downed_speed"][..])] {
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

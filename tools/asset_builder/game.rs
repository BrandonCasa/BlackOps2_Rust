use crate::{convert::Catalog, core::*, fastfile::Address, pack::Pack};
use serde_json::{Value, json};
use std::sync::Arc;

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
    println!("Converted {} native localized strings and original normal-difficulty player health profile", localized.len());
    Ok(json!({"version":2,"player_health":{
        "maximum":100,"regular_delay":2.4,"regular_step_fraction":1.,
        "critical_delay":5.,"critical_ratio":0.2,"critical_step_seconds":0.05,"critical_step_fraction":0.1,
        "worthy_damage_ratio":0.1,"immunity_regular":0.35,"immunity_critical_enter":0.5,
        "immunity_critical_repeat":0.3,"feedback_seconds":0.8},
        "content":{"localized_strings":localized}}))
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

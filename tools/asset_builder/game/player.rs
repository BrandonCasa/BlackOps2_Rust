use super::*;

pub(super) fn source(path: &str) -> Result<String> {
    Ok(std::fs::read_to_string(
        std::path::PathBuf::from(
            std::env::var_os("USERPROFILE").ok_or("USERPROFILE is unavailable")?,
        )
        .join("Documents/T6ZMResearch/t6zm-master")
        .join(path),
    )?)
}

fn dvar(source: &str, name: &str, integer: bool) -> Result<f32> {
    let registration = format!(
        "FUN_{}(\"{name}\",",
        if integer { "00757cf0" } else { "00757df0" }
    );
    let value = source
        .split_once(&registration)
        .and_then(|(_, tail)| tail.split_once(','))
        .map(|(value, _)| value.trim())
        .ok_or_else(|| format!("Native player tuning registration is missing: {name}"))?;
    let value = if let Some(hex) = value.strip_prefix("0x") {
        let bits = u32::from_str_radix(hex, 16)?;
        if integer {
            bits as i32 as f32
        } else {
            f32::from_bits(bits)
        }
    } else if integer {
        value.parse::<i32>()? as f32
    } else {
        f32::from_bits(value.parse::<u32>()?)
    };
    if !value.is_finite() {
        return Err(format!("Native player tuning is non-finite: {name}").into());
    }
    Ok(value)
}

fn script_number(source: &str, assignment: &str) -> Result<f32> {
    let value = source
        .split_once(&format!("{assignment} = "))
        .and_then(|(_, tail)| tail.split_once(';'))
        .ok_or_else(|| format!("Native player script assignment is missing: {assignment}"))?
        .0
        .trim()
        .parse::<f32>()?;
    if !value.is_finite() {
        return Err(format!("Native player script assignment is non-finite: {assignment}").into());
    }
    Ok(value)
}

pub(super) fn append(game: &mut Value, key: &str, row: Value) -> Result<()> {
    if game["content"][key].is_null() {
        game["content"][key] = json!([]);
    }
    let rows = game["content"][key]
        .as_array_mut()
        .ok_or("Native initial player content is not an array")?;
    if let Some(existing) = rows.iter().find(|existing| existing["id"] == row["id"]) {
        if existing != &row {
            return Err(format!("Conflicting initial player content {}", row["id"]).into());
        }
    } else {
        rows.push(row);
    }
    Ok(())
}

pub(super) fn pose(entity: &crate::entities::Entity) -> Result<([f32; 3], f32)> {
    let [x, y, z] = crate::entities::vector(entity, "origin")?
        .ok_or("Native initial player interaction has no origin")?;
    let yaw = if let Some(angles) = crate::entities::vector(entity, "angles")? {
        angles[1]
    } else {
        entity
            .get("angle")
            .map(|angle| angle.parse())
            .transpose()?
            .unwrap_or(0.)
    };
    Ok(([x * 0.0254, z * 0.0254, -y * 0.0254], yaw.to_radians()))
}

fn revive_collision(catalog: &Catalog<'_>) -> Result<Vec<Value>> {
    let (zone, root) = catalog.resolve(5, "zm_collision_perks1")?;
    let streams = &catalog.zones[zone].1.streams;
    let count = streams.u8(root.add(220)?)? as usize;
    if count != 1 {
        return Err(format!(
            "Native Quick Revive collision requires one PhysCollmap, found {count}"
        )
        .into());
    }
    let maps = Address::decode(streams.u32(root.add(224)?)?)?;
    let map = Address::decode(streams.u32(maps)?)?;
    let count = streams.u32(map)? as usize;
    let contents = streams.u32(map.add(8)?)?;
    let geometries = Address::decode(streams.u32(map.add(4)?)?)?;
    let mut shapes = Vec::new();
    let mut brush_union = 0;
    for index in 0..count {
        let geometry = geometries.add(index * 68)?;
        let kind = streams.u32(geometry.add(4)?)?;
        if kind != 2 {
            return Err(format!(
                "Native Quick Revive PhysCollmap requires geometry type {kind} conversion"
            )
            .into());
        }
        let brush = Address::decode(streams.u32(geometry)?)?;
        let brush_contents = streams.u32(brush.add(12)?)?;
        if brush_contents & 0x30000 != 0x30000 {
            return Err(
                "Native Quick Revive brush lacks source player and actor clip contents".into(),
            );
        }
        brush_union |= brush_contents;
        let count = streams.u32(brush.add(84)?)? as usize;
        if !(4..=128).contains(&count) {
            return Err("Native Quick Revive PhysCollmap vertex count is unsupported".into());
        }
        let vertices = Address::decode(streams.u32(brush.add(88)?)?)?;
        let mut points = Vec::with_capacity(count);
        for index in 0..count {
            let vertex = vertices.add(index * 12)?;
            let x = f32::from_bits(streams.u32(vertex)?);
            let y = f32::from_bits(streams.u32(vertex.add(4)?)?);
            let z = f32::from_bits(streams.u32(vertex.add(8)?)?);
            if ![x, y, z].iter().all(|value| value.is_finite()) {
                return Err("Native Quick Revive PhysCollmap contains nonfinite vertices".into());
            }
            points.push([x * 0.0254, z * 0.0254, -y * 0.0254]);
        }
        shapes.push(json!({"id":format!("quick_revive.clip.{index}"),"position":[0.,0.,0.],"vertices":points,"contents":brush_contents}));
    }
    if shapes.is_empty() {
        return Err("Native Quick Revive PhysCollmap has no geometry".into());
    }
    if brush_union != contents {
        return Err(
            "Native Quick Revive PhysCollmap broadphase contents differ from brush union".into(),
        );
    }
    Ok(shapes)
}

fn revive_path_bounds_radius(catalog: &Catalog<'_>) -> Result<f32> {
    let (zone, root) = catalog.resolve(5, "zm_collision_perks1")?;
    let streams = &catalog.zones[zone].1.streams;
    let mut squared = 0.;
    for (axis, (minimum, maximum)) in [(-4_f32, 4_f32), (-4., 4.), (0., 10.)]
        .into_iter()
        .enumerate()
    {
        let low = f32::from_bits(streams.u32(root.add(172 + axis * 4)?)?);
        let high = f32::from_bits(streams.u32(root.add(184 + axis * 4)?)?);
        if !low.is_finite() || !high.is_finite() || low > high {
            return Err("Native Quick Revive XModel path bounds are invalid".into());
        }
        squared += low.min(minimum).abs().max(high.max(maximum).abs()).powi(2);
    }
    Ok((squared.sqrt() + 1.) * 0.0254)
}

pub(super) fn read(
    catalog: &Catalog<'_>,
    game: &mut Value,
    entities: &[crate::entities::Entity],
    gaps: &mut Vec<String>,
) -> Result<()> {
    let movement = source("exports/functions/008b/008b64d0.c")?;
    let server = source("exports/functions/005f/005fced0.c")?;
    let jump = source("exports/functions/008b/008b3540.c")?;
    let dive = source("exports/functions/008a/008ad170.c")?;
    let tomb = source("source/server-tomb/maps/mp/zm_tomb.gsc")?;
    let slow = source("source/server-tomb/maps/mp/zm_tomb_utility.gsc")?;
    let laststand = source("source/server-patch/maps/mp/zombies/_zm.gsc")?;
    let health = source("source/server-patch/maps/mp/zombies/_zm_playerhealth.gsc")?;
    let perks = source("source/server-patch/maps/mp/zombies/_zm_perks.gsc")?;
    for (body, expression) in [
        (&tomb, "self setsprintduration( 4 );"),
        (&tomb, "self setsprintcooldown( 0 );"),
        (&slow, "n_movescale_delta_no_perk = 0.4 / 4.0;"),
        (&slow, "n_movescale_delta_staminup = 0.3 / 6.0;"),
        (&slow, "n_new_move_scale = 0.6;"),
        (&slow, "n_new_move_scale = 0.7;"),
        (&slow, "wait 0.1;"),
        (&laststand, "solo_revive_time = 10.0;"),
        (
            &laststand,
            "setdvar( \"player_lastStandBleedoutTime\", \"45\" );",
        ),
    ] {
        if !body.contains(expression) {
            return Err(format!(
                "Original initial player source expression is unsupported: {expression}"
            )
            .into());
        }
    }
    let units = 0.0254_f32;
    let speed = dvar(&server, "g_speed", true)? * units;
    let gravity = dvar(&movement, "bg_gravity", false)? * units;
    let jump_height = dvar(&jump, "jump_height", false)? * units;
    let reboot = perks
        .split_once("else if ( perk == \"health_reboot\" )")
        .ok_or("Native player health reboot branch is missing")?
        .1;
    let maximum = script_number(reboot, "max_total_health")?;
    if maximum.fract() != 0. || maximum <= 0. || maximum > i32::MAX as f32 {
        return Err("Native player maximum health is invalid".into());
    }
    game["player_health"] = json!({
        "maximum":maximum as u32,
        "regular_delay":script_number(&health,"level.playerhealth_regularregendelay")? * 0.001,
        "regular_step_fraction":1.,
        "critical_delay":script_number(&health,"level.longregentime")? * 0.001,
        "critical_ratio":script_number(&health,"level.healthoverlaycutoff")?,
        "critical_step_seconds":0.05,
        "critical_step_fraction":script_number(&health,"regenrate")?,
        "worthy_damage_ratio":script_number(&health,"level.worthydamageratio")?,
        "immunity_regular":script_number(&health,"level.invultime_preshield")?,
        "immunity_critical_enter":script_number(&health,"level.invultime_onshield")?,
        "immunity_critical_repeat":script_number(&health,"level.invultime_postshield")?,
        "feedback_seconds":0.8
    });
    for expression in [
        "wait 0.05;",
        "newhealth = 1;",
        "self startfadingblur( 3, 0.8 );",
    ] {
        if !health.contains(expression) {
            return Err(
                format!("Native player health expression is unsupported: {expression}").into(),
            );
        }
    }
    let pistol = game["content"]["weapons"]
        .as_array()
        .and_then(|rows| rows.iter().find(|row| row["id"] == "weapon.c96_zm"))
        .ok_or("Native initial pistol profile is missing for player tuning")?;
    let knife = game["content"]["weapons"]
        .as_array()
        .and_then(|rows| rows.iter().find(|row| row["id"] == "weapon.knife_zm"))
        .ok_or("Native initial knife profile is missing for player tuning")?;
    game["tuning"] = json!({
        "walk_speed":speed,"sprint_speed":speed*dvar(&movement,"player_sprintSpeedScale",false)?,
        "sprint_seconds":4.,"sprint_minimum":dvar(&movement,"player_sprintMinTime",false)?,
        "sprint_recovery_rate":1.,"sprint_recovery_delay":0.,
        "crouch_speed":speed*0.65,"gravity":gravity,"jump_speed":(2.*jump_height*gravity).sqrt(),
        "step_height":dvar(&jump,"jump_stepSize",false)?*units,
        "radius":15.*units,"standing_height":70.*units,"crouch_height":50.*units,
        "eye_height":dvar(&movement,"player_standingViewHeight",true)?*units,"crouch_eye":40.*units,
        "pistol_damage":pistol["damage"],"melee_damage":knife["damage"],"magazine":pistol["magazine"],
        "reserve":pistol["mechanics"]["starting_reserve"],"fire_interval":pistol["fire_interval"],
        "reload_seconds":pistol["reload_seconds"],"combat_points":[10,50,50,10,80],"combat_neck_points":20
    });
    game["tuning"]["physics"] = json!({"ground_acceleration":9.,"crouch_acceleration":12.,"prone_acceleration":19.,
            "air_acceleration":1.,"friction":dvar(&movement,"friction",false)?,
            "stop_speed":dvar(&movement,"stopspeed",false)?*units,
            "strafe_scale":dvar(&movement,"player_strafeSpeedScale",false)?,
            "backward_scale":dvar(&movement,"player_backSpeedScale",false)?,
            "sprint_strafe_scale":dvar(&movement,"player_sprintStrafeSpeedScale",false)?,
            "ground_probe":units,"step_down":9.*units,"skin_width":0.25*units,
            "step_slowdown_height":0.5*units,"step_speed_min":0.2});
    game["tuning"]["mobility"] = json!({"prone_height":30.*units,"prone_eye":11.*units,"prone_speed":speed*0.15,
            "prone_step":10.*units,"jump_recovery":0.5,"dive_jump_height":jump_height,
            "dive_launch_multiplier":dvar(&dive,"dtp_new_trajectory_multiplier",false)?,
            "dive_apex_seconds":dvar(&dive,"dtp_max_apex_duration",true)?*0.001,
            "dive_recovery":dvar(&dive,"dtp_post_move_pause",false)?*0.001,
            "dive_startup":dvar(&dive,"dtp_startup_delay",false)?*0.001,
            "dive_minimum_speed":dvar(&dive,"dtp_min_speed",false)?*units,
            "dive_exhaustion":dvar(&dive,"dtp_exhaustion_window",false)?*0.001,
            "dive_slide":dvar(&dive,"dtp_max_slide_duration",false)?*0.001,
            "dive_fall_minimum":dvar(&dive,"dtp_fall_damage_min_height",false)?*units,
            "dive_fall_fatal":dvar(&dive,"dtp_fall_damage_max_height",false)?*units,
            "fall_minimum":dvar(&movement,"bg_fallDamageMinHeight",false)?*units,
            "fall_fatal":dvar(&movement,"bg_fallDamageMaxHeight",false)?*units
    });
    let curves = [
        json!([
            [0., 1.],
            [1., 0.975],
            [4., 0.925],
            [30., 0.8],
            [80., 0.2],
            [90., 0.075],
            [95., 0.025],
            [100., 0.]
        ]),
        json!([
            [0., 0.],
            [5., 0.025],
            [10., 0.075],
            [20., 0.2],
            [70., 0.8],
            [96., 0.925],
            [99., 0.975],
            [100., 1.]
        ]),
        json!([
            [0., 1.],
            [11., 0.931],
            [22., 0.759],
            [34., 0.483],
            [45., 0.172],
            [50., 0.138],
            [55., 0.172],
            [70., 0.241],
            [90., 0.207],
            [100., 0.]
        ]),
        json!([
            [0., 0.],
            [5., -0.034],
            [30., 0.345],
            [50., 0.483],
            [67., 0.69],
            [83., 0.79],
            [100., 1.]
        ]),
    ];
    game["tuning"]["mobility"]["stance"] =
        json!({"seconds":[0.2,0.2,0.6,0.4],"other_speed":180.*units,"curves":curves});
    game["solo_revive"]["seconds"] = json!(script_number(&laststand, "solo_revive_time")?);
    game["solo_revive"]["bleedout_seconds"] = json!(45.);
    game["solo_revive"]["downed_height"] = json!(30. * units);
    game["solo_revive"]["downed_eye"] = json!(22. * units);
    game["solo_revive"]["downed_speed"] = json!(speed * 0.15);
    let enabled_weapons = game["content"]["weapons"]
        .as_array()
        .ok_or("Native initial weapons are missing")?
        .iter()
        .filter_map(|weapon| weapon["id"].as_str().map(str::to_owned))
        .collect::<std::collections::BTreeSet<_>>();
    game["solo_revive"]["pistol_ranks"]
        .as_array_mut()
        .ok_or("Native initial solo pistol ranking is missing")?
        .retain(|pistol| {
            pistol["weapon"]
                .as_str()
                .is_some_and(|id| enabled_weapons.contains(id))
        });
    let capture = source("source/server-tomb/maps/mp/zm_tomb_capture_zones.gsc")?;
    for expression in [
        "cost = 500;",
        "self.lives = 1;",
        "if ( level.solo_lives_given >= 3 )",
        "self waittill_notify_or_timeout( \"burp\", 0.5 );",
    ] {
        if !perks.contains(expression) {
            return Err(format!(
                "Original Quick Revive source expression is unsupported: {expression}"
            )
            .into());
        }
    }
    if !capture.contains("register_perk_machine_for_zone( \"generator_start_bunker\", \"revive\", \"vending_revive\", ::revive_perk_fx_think );")
        || !capture.contains("b_machine_valid = level.zone_capture.zones[self.str_zone_name] ent_flag( \"player_controlled\" );") {
        return Err("Original initial Quick Revive generator prerequisite is unsupported".into());
    }
    if !tomb.contains("level.default_start_location = \"tomb\";") {
        return Err("Original Origins perk starting-location selection is unsupported".into());
    }
    for expression in [
        "structs = getstructarray( \"zm_perk_machine\", \"targetname\" );",
        "use_trigger = spawn( \"trigger_radius_use\", pos[i].origin + vectorscale( ( 0, 0, 1 ), 30.0 ), 0, 40, 70 );",
        "collision setmodel( \"zm_collision_perks1\" );",
        "bump_trigger = spawn( \"trigger_radius\", pos[i].origin, 0, 35, 64 );",
    ] {
        if !perks.contains(expression) {
            return Err(format!(
                "Original Quick Revive creation expression is unsupported: {expression}"
            )
            .into());
        }
    }
    let machines = entities
        .iter()
        .enumerate()
        .filter(|(_, entity)| {
            entity
                .get("script_noteworthy")
                .is_some_and(|name| name == "specialty_quickrevive")
                && entity
                    .get("targetname")
                    .is_some_and(|name| name == "zm_perk_machine")
                && entity
                    .get("classname")
                    .is_some_and(|class| class == "script_struct")
                && entity.get("script_string").is_none_or(|tokens| {
                    tokens
                        .split_whitespace()
                        .any(|token| token == "zclassic_perks_tomb")
                })
        })
        .collect::<Vec<_>>();
    let [(trigger_index, machine)] = machines.as_slice() else {
        let candidates = entities
            .iter()
            .enumerate()
            .filter(|(_, entity)| {
                entity
                    .get("script_noteworthy")
                    .is_some_and(|name| name == "specialty_quickrevive")
            })
            .collect::<Vec<_>>();
        return Err(format!(
            "Initial native Quick Revive requires one eligible authored perk struct, found {}; native candidates {candidates:?}",
            machines.len()
        )
        .into());
    };
    let (position, yaw) = pose(machine)?;
    let model = "p6_zm_tm_vending_revive_on";
    catalog.resolve(5, model)?;
    append(
        game,
        "presentations",
        json!({"id":format!("actor.{model}"),"source_model":model,"scene":null,"animations":[],"sounds":[]}),
    )?;
    let off_model = "p6_zm_tm_vending_revive";
    catalog.resolve(5, off_model)?;
    append(
        game,
        "presentations",
        json!({"id":format!("actor.{off_model}"),"source_model":off_model,"scene":null,"animations":[],"sounds":[]}),
    )?;
    let collision = revive_collision(catalog)?;
    let path_bounds_radius = revive_path_bounds_radius(catalog)?;
    let mut icon = Value::Null;
    let material = "specialty_quickrevive_zombies";
    match catalog.resolve(6, material) {
        Ok((zone, root)) => {
            let streams = &catalog.zones[zone].1.streams;
            if streams.u8(root.add(84)?)? == 1 {
                let textures = Address::decode(streams.u32(root.add(96)?)?)?;
                let image = catalog.pointer(zone, textures.add(12)?, 8)?.0;
                let id = "ui.perk.quick_revive";
                append(
                    game,
                    "ui_assets",
                    json!({"id":id,"source_image":image,"path":null}),
                )?;
                icon = json!(id);
            } else {
                gaps.push(format!("Initial Quick Revive icon {material} requires its native multi-texture UI binding"));
            }
        }
        Err(_) => gaps.push(format!(
            "Initial Quick Revive icon material {material} is absent from installed assets"
        )),
    }
    let bottle = game["content"]["weapons"]
        .as_array()
        .and_then(|rows| {
            rows.iter()
                .find(|row| row["id"] == "weapon.zombie_perk_bottle_revive")
        })
        .ok_or("Initial Quick Revive bottle profile is missing")?;
    let drink = json!({"bottle":"weapon.zombie_perk_bottle_revive","seconds":bottle["mechanics"]["equip_seconds"],"lower_seconds":bottle["mechanics"]["unequip_seconds"],"activation_delay":0.5});
    append(
        game,
        "perks",
        json!({"id":"perk.quick_revive","name":"Quick Revive","price":500,"icon":icon,"presentation":format!("actor.{model}"),"off_presentation":format!("actor.{off_model}"),"collision":collision,"path_bounds_radius":path_bounds_radius,"bump_trigger":format!("trigger.quick_revive.bump.native.{trigger_index}"),"generator":"generator.generator_start_bunker","drink":drink}),
    )?;
    game["solo_revive"]["maximum_purchases"] = json!(3);
    game["_initial_quick_revive"] = json!({"trigger":format!("trigger.quick_revive.native.{trigger_index}"),"position":position,"yaw":yaw,"source":machine});
    Ok(())
}

pub(super) fn prepare(
    game: &mut Value,
    entities: &[crate::entities::Entity],
    missing: &mut Vec<String>,
) -> Result<()> {
    let source = game
        .as_object_mut()
        .ok_or("Native game source is not an object")?
        .remove("_initial_quick_revive")
        .ok_or("Initial Quick Revive source was not read")?;
    let trigger = source["trigger"]
        .as_str()
        .ok_or("Initial Quick Revive trigger identity is missing")?;
    let position: [f32; 3] = serde_json::from_value(source["position"].clone())?;
    let mut trigger_source = source["source"]
        .as_object()
        .ok_or("Initial Quick Revive source struct is invalid")?
        .clone();
    trigger_source.insert("classname".into(), json!("trigger_radius_use"));
    trigger_source.insert("targetname".into(), json!("zombie_vending"));
    trigger_source.insert("target".into(), json!("vending_revive"));
    trigger_source.remove("model");
    trigger_source.remove("spawnflags");
    let trigger_position = [position[0], position[1] + 30. * 0.0254, position[2]];
    game["environment"]["triggers"].as_array_mut().ok_or("Native initial triggers are missing")?.push(json!({"id":trigger,"position":trigger_position,"source":trigger_source,"shape":{"kind":"cylinder","radius":40.*0.0254,"height":70.*0.0254}}));
    game["environment"]["ownership"]
        .as_array_mut()
        .ok_or("Native initial ownership is missing")?
        .push(json!({"id":trigger,"owner":"native","shootable":false}));
    let bump = game["content"]["perks"]
        .as_array()
        .and_then(|rows| rows.iter().find(|row| row["id"] == "perk.quick_revive"))
        .and_then(|row| row["bump_trigger"].as_str())
        .ok_or("Initial Quick Revive bump trigger is missing")?
        .to_owned();
    game["environment"]["triggers"].as_array_mut().ok_or("Native initial triggers are missing")?.push(json!({"id":bump,"position":position,"source":{"classname":"trigger_radius","targetname":"audio_bump_trigger","script_activated":"1","script_sound":"zmb_perks_bump_bottle","script_string":"revive_perk"},"shape":{"kind":"cylinder","radius":35.*0.0254,"height":64.*0.0254}}));
    game["environment"]["ownership"]
        .as_array_mut()
        .ok_or("Native initial ownership is missing")?
        .push(json!({"id":bump,"owner":"native","shootable":false}));
    let generators = entities
        .iter()
        .filter(|entity| {
            entity
                .get("classname")
                .is_some_and(|class| class == "script_struct")
                && entity
                    .get("targetname")
                    .is_some_and(|name| name == "s_generator")
                && entity
                    .get("script_noteworthy")
                    .is_some_and(|name| name == "generator_start_bunker")
                && entity.get("script_int").is_some_and(|number| number == "1")
        })
        .collect::<Vec<_>>();
    let [generator] = generators.as_slice() else {
        return Err(format!(
            "Initial Quick Revive requires one authored generator_start_bunker owner, found {}",
            generators.len()
        )
        .into());
    };
    let (generator_position, _) = pose(generator)?;
    let zone = super::navigation::zone_at(game, generator_position)?
        .ok_or_else(|| format!("Initial Quick Revive generator owner at {generator_position:?} is outside source player zones"))?;
    println!(
        "Bound native Quick Revive at {position:?} to generator_start_bunker at {generator_position:?} in {zone}"
    );
    append(
        game,
        "placements",
        json!({"id":"placement.quick_revive","definition":"perk.quick_revive","position":position,"yaw":source["yaw"],"zone":zone}),
    )?;
    append(
        game,
        "interactions",
        json!({"id":"interaction.quick_revive","placement":"placement.quick_revive","action":{"kind":"perk","perk":"perk.quick_revive"},"reach":72.*0.0254,"hold_seconds":0.,"hidden_prompt":false,"source_trigger":trigger,"generator":"generator.generator_start_bunker"}),
    )?;
    if !game["content"]["generators"]
        .as_array()
        .is_some_and(|rows| {
            rows.iter()
                .any(|row| row["id"] == "generator.generator_start_bunker")
        })
    {
        missing.push("Initial solo Quick Revive requires source generator_start_bunker capture conversion and admission before purchase is usable".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_player_script_scalars_reject_missing_or_unsupported_expressions() {
        let source = "level.delay = 2400; regenrate = 0.1;";
        assert_eq!(script_number(source, "level.delay").unwrap(), 2400.);
        assert_eq!(script_number(source, "regenrate").unwrap(), 0.1);
        assert!(script_number(source, "missing").is_err());
        assert!(script_number("delay = undefined;", "delay").is_err());
        assert!(script_number("delay = NaN;", "delay").is_err());
    }

    #[test]
    fn native_player_dvars_preserve_integer_and_float_registration_types() {
        let source =
            "FUN_00757df0(\"speed\",0x433e0000,0,0,0,0); FUN_00757cf0(\"height\",0x3c,0,0,0,0);";
        assert_eq!(dvar(source, "speed", false).unwrap(), 190.);
        assert_eq!(dvar(source, "height", true).unwrap(), 60.);
        assert!(dvar(source, "speed", true).is_err());
        assert!(dvar(source, "missing", false).is_err());
    }

    #[test]
    #[ignore = "requires the authorized Steam installation"]
    fn installed_initial_player_profiles_and_revive_source() {
        let zones = crate::source::read_zones(&crate::source::installation().unwrap()).unwrap();
        let catalog = Catalog::new(&zones);
        let entities = crate::entities::read(&catalog).unwrap();
        let (game, _) = super::super::read(&catalog, &entities).unwrap();
        let tuning = &game["tuning"];
        let scalar = |value: &Value| value.as_f64().unwrap();
        assert!((scalar(&tuning["walk_speed"]) - 190. * 0.0254).abs() < 0.000001);
        assert!((scalar(&tuning["radius"]) - 15. * 0.0254).abs() < 0.000001);
        assert!((scalar(&tuning["crouch_eye"]) - 40. * 0.0254).abs() < 0.000001);
        assert!((scalar(&tuning["mobility"]["prone_height"]) - 30. * 0.0254).abs() < 0.000001);
        assert!((scalar(&tuning["mobility"]["prone_eye"]) - 11. * 0.0254).abs() < 0.000001);
        assert_eq!(tuning["physics"]["friction"], 5.5);
        assert_eq!(tuning["mobility"]["jump_recovery"], 0.5);
        assert_eq!(
            tuning["mobility"]["stance"]["seconds"],
            json!([0.2, 0.2, 0.6, 0.4])
        );
        assert_eq!(game["solo_revive"]["maximum_purchases"], 3);
        assert_eq!(
            game["solo_revive"]["pistol_ranks"],
            json!([
                {"weapon":"weapon.c96_zm","rank":1},
                {"weapon":"weapon.c96_upgraded_zm","rank":17}
            ])
        );
        assert_eq!(game["player_health"]["maximum"], 100);
        assert!((scalar(&game["player_health"]["regular_delay"]) - 2.4).abs() < 0.000001);
        assert_eq!(game["player_health"]["critical_delay"], 5.);
        assert_eq!(game["player_health"]["immunity_critical_enter"], json!(0.5_f32));
        let perk = game["content"]["perks"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["id"] == "perk.quick_revive")
            .unwrap();
        assert_eq!(perk["price"], 500);
        assert_eq!(perk["generator"], "generator.generator_start_bunker");
        assert_eq!(perk["drink"]["bottle"], "weapon.zombie_perk_bottle_revive");
        assert!(
            perk["path_bounds_radius"]
                .as_f64()
                .is_some_and(|radius| radius > 0.)
        );
        assert_eq!(
            game["_initial_quick_revive"]["source"]["script_noteworthy"],
            "specialty_quickrevive"
        );
        println!(
            "Native initial Quick Revive source trigger {}",
            game["_initial_quick_revive"]["trigger"]
        );
        println!(
            "Native Quick Revive path radius {} and authored blocker {}",
            perk["path_bounds_radius"], game["_initial_quick_revive"]["source"]["blocker_model"]
        );
    }
}

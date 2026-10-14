use super::*;

pub(super) const INITIAL_WEAPONS: [&str; 5] = [
    "c96_zm",
    "c96_upgraded_zm",
    "knife_zm",
    "frag_grenade_zm",
    "zombie_perk_bottle_revive",
];

pub(super) fn read(catalog: &Catalog<'_>, game: &mut Value, gaps: &mut Vec<String>) -> Result<()> {
    let mut weapons = Vec::new();
    let mut ammunition = std::collections::BTreeSet::new();
    let mut ui = Vec::new();
    for name in INITIAL_WEAPONS {
        let (zone, root) = catalog.resolve(25, name)?;
        let streams = &catalog.zones[zone].1.streams;
        let definition = Address::decode(streams.u32(root.add(8)?)?)?;
        let integer = |base: Address, offset| -> Result<u32> {
            let value = streams.u32(base.add(offset)?)?;
            if value > i32::MAX as u32 {
                return Err(format!(
                    "Initial weapon {name} has negative native integer at {offset}"
                )
                .into());
            }
            Ok(value)
        };
        let scalar = |base: Address, offset| -> Result<f32> {
            let value = f32::from_bits(streams.u32(base.add(offset)?)?);
            if !value.is_finite() {
                return Err(format!(
                    "Initial weapon {name} has nonfinite native float at {offset}"
                )
                .into());
            }
            Ok(value)
        };
        let text = |base: Address, offset| -> Result<Option<String>> {
            let pointer = streams.u32(base.add(offset)?)?;
            if pointer == 0 {
                return Ok(None);
            }
            let value = streams.text(Address::decode(pointer)?)?;
            Ok((!value.is_empty()).then_some(value))
        };
        let seconds = |base, offset| -> Result<f32> { Ok(integer(base, offset)? as f32 * 0.001) };
        let native_type = integer(definition, 28)?;
        let class = integer(definition, 32)?;
        let delivery =
            match native_type {
                0 if class == 4 => "hitscan",
                1 if class == 5 => "equipment",
                2 if class == 4 => "projectile",
                7 if class == 11 => "melee",
                0..=8 if class == 10 => "equipment",
                _ => return Err(format!(
                    "Initial weapon {name} has unsupported native type/class {native_type}/{class}"
                )
                .into()),
            };
        let magazine = integer(root, 476)?;
        let relative = streams.u8(definition.add(993)?)? != 0;
        let ammo = |offset| -> Result<u32> {
            integer(definition, offset)?
                .checked_mul(if relative { magazine } else { 1 })
                .ok_or_else(|| format!("Initial weapon {name} native ammunition overflows").into())
        };
        let group = text(root, 512)?.map(|source| {
            format!(
                "ammo.{}",
                source
                    .chars()
                    .map(|value| {
                        if value.is_ascii_alphanumeric() || matches!(value, '.' | '_' | '-') {
                            value.to_ascii_lowercase()
                        } else {
                            '.'
                        }
                    })
                    .collect::<String>()
            )
        });
        if let Some(group) = &group {
            ammunition.insert(group.clone());
        }
        let gun_models = Address::decode(streams.u32(definition.add(4)?)?)?;
        let (model, _, _) = catalog.pointer(zone, gun_models, 5)?;
        let presentation = format!("actor.{model}");
        let mut actions = serde_json::Map::new();
        let animations = Address::decode(streams.u32(root.add(32)?)?)?;
        for &(index, action) in super::presentations::WEAPON_ACTIONS {
            if let Some(clip) = text(animations, index * 4)? {
                catalog.resolve(4, &clip)?;
                let binding = game["content"]["animations"].as_array()
                    .ok_or("Initial source animation bindings are missing")?
                    .iter().filter(|row|row["source_clip"].as_str().is_some_and(|name|name.eq_ignore_ascii_case(&clip)))
                    .filter(|row|row["skeleton"]==presentation || matches!(row["skeleton"].as_str(),
                        Some("actor.c_zom_dempsey_viewhands"|"actor.c_zom_nikolai_viewhands"|"actor.c_zom_richtofen_viewhands"|"actor.c_zom_takeo_viewhands")))
                    .min_by_key(|row|u8::from(row["skeleton"]!=presentation))
                    .ok_or_else(||format!("Initial weapon {name}/{action} has no source-mapped hand or weapon animation binding for {clip}"))?;
                actions.insert(action.into(), binding["id"].clone());
            }
        }
        let mut pickup = None;
        if streams.u32(definition.add(916)?)? != 0 {
            let array = Address::decode(streams.u32(definition.add(916)?)?)?;
            if streams.u32(array)? != 0 {
                pickup = Some(format!("actor.{}", catalog.pointer(zone, array, 5)?.0));
            }
        }
        let mut reticle = |offset, role: &str| -> Result<Option<String>> {
            if streams.u32(definition.add(offset)?)? == 0 {
                return Ok(None);
            }
            let (material, material_zone, material_root) =
                catalog.pointer(zone, definition.add(offset)?, 6)?;
            let material_streams = &catalog.zones[material_zone].1.streams;
            if material_streams.u8(material_root.add(84)?)? != 1 {
                gaps.push(format!("Initial weapon {name} {role} reticle material {material} requires its native multi-texture UI binding"));
                return Ok(None);
            }
            let table = Address::decode(material_streams.u32(material_root.add(96)?)?)?;
            let image = catalog.pointer(material_zone, table.add(12)?, 8)?.0;
            let id = format!("ui.weapon.{name}.{role}");
            ui.push(json!({"id":id,"source_image":image,"path":null}));
            Ok(Some(id))
        };
        let hud = json!({"center":reticle(528,"center")?,"side":reticle(532,"side")?,
            "center_size":integer(definition,536)?,"side_size":integer(definition,540)?,
            "minimum_offset":integer(definition,544)?,"side_position":scalar(definition,1504)?,
            "keep_when_ads":streams.u8(definition.add(1626)?)? != 0,
            "low_ammo_fraction":scalar(definition,1860)?});
        let fire = match integer(definition, 48)? {
            0 => json!({"mode":"automatic"}),
            1 => json!({"mode":"semi"}),
            shots @ 2..=5 => json!({"mode":"burst","shots":shots}),
            value => {
                return Err(format!(
                    "Initial weapon {name} has unsupported native firing mode {value}"
                )
                .into());
            }
        };
        let mut mechanics = json!({"delivery":delivery,"fire":fire,
            "weapon_class":match class {4=>"pistol",5=>"grenade",10=>"item",11=>"melee",_=>unreachable!()},
            "range":if native_type == 7 {55_f32*0.0254} else {8192_f32*0.0254},
            "melee_extent":[10_f32*0.0254,10_f32*0.0254],
            "starting_reserve":ammo(968)?,"movement":[scalar(definition,1416)?,scalar(definition,1420)?],
            "melee_seconds":[seconds(definition,1076)?,seconds(definition,1152)?],
            "equip_seconds":seconds(definition,1212)?,"unequip_seconds":seconds(definition,1208)?,
            "sprint_seconds":[seconds(definition,1240)?,seconds(definition,1244)?,seconds(definition,1248)?],
            "dive_seconds":[seconds(definition,1276)?,seconds(definition,1280)?,seconds(definition,1284)?],
            "crawl_seconds":[seconds(definition,1288)?,seconds(definition,1292)?,seconds(definition,1296)?,
                seconds(definition,1300)?,seconds(definition,1304)?,seconds(definition,1308)?,seconds(definition,1312)?],
            "ads_enabled":streams.u8(definition.add(1592)?)? != 0,
            "ads_seconds":[seconds(root,496)?,seconds(root,500)?],
            "ads_fov":scalar(root,548)?,"ads_zoom_fraction":[scalar(root,560)?,scalar(root,564)?],
            "empty_reload":seconds(root,484)?,"burst_delay":seconds(definition,1156)?,
            "shell_reload":streams.u8(definition.add(1642)?)? != 0,
            "shell_seconds":seconds(definition,1180)?,"reload_start_seconds":seconds(definition,1196)?,
            "reload_start_insert":seconds(definition,1200)?,"reload_end_seconds":seconds(definition,1204)?,
            "dual_wield":streams.u8(definition.add(1615)?)? != 0,
            "projectile_speed":integer(definition,1732)? as f32*0.0254,
            "projectile_up_speed":integer(definition,1736)? as f32*0.0254,
            "projectile_lifetime":scalar(definition,1756)?,"projectile_bounce":streams.u8(definition.add(1623)?)? != 0,
            "explosion_radius":integer(definition,1708)? as f32*0.0254,
            "explosion_damage":[integer(definition,1720)?,integer(definition,1724)?],"fuse_seconds":seconds(definition,1356)?});
        let damage = if native_type == 7 {
            integer(definition, 1060)?
        } else {
            integer(definition, 996)?
        };
        if native_type <= 2 {
            let multipliers = Address::decode(streams.u32(definition.add(2300)?)?)?;
            let locations = (0..21)
                .map(|index| scalar(multipliers, index * 4))
                .collect::<Result<Vec<_>>>()?;
            mechanics["hit_multipliers"] = json!(locations);
        }
        if matches!(delivery, "hitscan" | "projectile") {
            for index in 1..5 {
                if integer(definition, 996 + index * 4)? != 0
                    || scalar(definition, 1020 + index * 4)? != 0.
                {
                    gaps.push(format!("Initial weapon {name} requires native intermediate damage curve point {index}"));
                }
            }
            mechanics["minimum_damage"] = json!(integer(definition, 1016)?);
            mechanics["damage_range"] = json!([
                scalar(definition, 1020)? * 0.0254,
                scalar(definition, 1040)? * 0.0254
            ]);
            mechanics["pellets"] = json!(1);
            mechanics["spread"] = json!([
                scalar(definition, 1456)?,
                scalar(definition, 1460)?,
                scalar(definition, 1464)?,
                scalar(definition, 2016)?
            ]);
            mechanics["recoil_pitch"] =
                json!([scalar(definition, 2060)?, scalar(definition, 2064)?]);
            mechanics["recoil_yaw"] = json!([scalar(definition, 2072)?, scalar(definition, 2076)?]);
            mechanics["recoil_recovery"] = json!(scalar(root, 544)?);
            mechanics["ads_recoil_recovery"] = json!(scalar(root, 540)?);
            mechanics["player_damage"] =
                json!([integer(definition, 1056)?, integer(definition, 1044)?]);
            if integer(definition, 36)? == 0 {
                mechanics["penetration"] = Value::Null;
            } else {
                mechanics["penetration"] = json!(penetration(catalog, integer(definition, 36)?)?);
            }
            if mechanics["ads_enabled"] == true {
                gaps.push(format!(
                    "Initial weapon {name} requires source ADS sight/alignment binding"
                ));
            }
        }
        for (offset, key) in [(1888, "projectile_trail"), (1776, "projectile_effect")] {
            if streams.u32(definition.add(offset)?)? != 0 {
                mechanics[key] = json!(catalog.pointer(zone, definition.add(offset)?, 33)?.0);
            }
        }
        if let Some(alias) = text(definition, 1920)? {
            mechanics["projectile_loop"] = json!(alias);
        }
        let throwable = if name == "frag_grenade_zm" {
            let bounce = |offset| -> Result<Vec<f32>> {
                let table = Address::decode(streams.u32(definition.add(offset)?)?)?;
                (0..32).map(|index| scalar(table, index * 4)).collect()
            };
            let sticky = integer(definition, 1840)?;
            if sticky != 0 {
                return Err(format!(
                    "Initial frag grenade has unexpected native stickiness {sticky}"
                )
                .into());
            }
            gaps.push(format!("Initial weapon {name} requires native rolling/friction trajectory, source launch origin/velocity and tick-boundary prime/throw timing"));
            Some(
                json!({"slot":"lethal","initial":0,"capacity":magazine,"round_award":2,
                "prime_seconds":seconds(definition,1144)?,"throw_seconds":seconds(definition,1128)?,
                "release_delay":seconds(definition,1072)?,"fuse_seconds":seconds(definition,1356)?,
                "speed":integer(definition,1732)? as f32*0.0254,"up_speed":integer(definition,1736)? as f32*0.0254,
                "radius":integer(definition,1708)? as f32*0.0254,"inner_radius":integer(definition,1712)? as f32*0.0254,
                "inner_damage":integer(definition,1720)?,"outer_damage":integer(definition,1724)?,
                "tangent_bounce":bounce(1880)?,"normal_bounce":bounce(1884)?,
                "collision_radius":1.5_f32*0.0254,"collision_half_extents":([1.5_f32*0.0254;3]),
                "cook_in_hand":streams.u8(definition.add(1604)?)? != 0,"sticky":false}),
            )
        } else {
            None
        };
        let display = text(root, 12)?;
        let label = display
            .as_ref()
            .and_then(|key| {
                game["content"]["localized_strings"]
                    .as_array()?
                    .iter()
                    .find(|row| row["key"] == *key)
                    .and_then(|row| row["text"].as_str())
                    .map(str::to_owned)
            })
            .unwrap_or_else(|| name.to_owned());
        weapons.push(json!({"id":format!("weapon.{name}"),"name":label,"localization_key":display,
            "damage":damage,"category":match native_type { 0=>"bullet",7=>"melee",_=>"explosive" },
            "magazine":magazine,"reserve":ammo(972)?,"fire_interval":seconds(definition,1128)?,
            "reload_seconds":seconds(root,480)?,"upgraded":if name=="c96_zm" { Some("weapon.c96_upgraded_zm") } else {None},
            "presentation":presentation,"pickup":pickup,"ammo_group":group,"animation_actions":actions,
            "hud":hud,"mechanics":mechanics,"throwable":throwable}));
    }
    game["content"]["weapons"] = json!(weapons);
    game["content"]["definitions"] = json!(ammunition);
    game["content"]["ui_assets"] = json!(ui);
    if let Some(ranks) = game["solo_revive"]["pistol_ranks"].as_array_mut() {
        ranks.retain(|rank| {
            matches!(
                rank["weapon"].as_str(),
                Some("weapon.c96_zm" | "weapon.c96_upgraded_zm")
            )
        });
    }
    println!(
        "Converted {} initial weapon profiles and {} source weapon HUD images",
        INITIAL_WEAPONS.len(),
        game["content"]["ui_assets"].as_array().map_or(0, Vec::len)
    );
    Ok(())
}

fn penetration(catalog: &Catalog<'_>, class: u32) -> Result<[f32; 32]> {
    let prefix = match class {
        1 => "small",
        2 => "medium",
        3 => "large",
        _ => return Err(format!("Unsupported native bullet penetration class {class}").into()),
    };
    let surfaces = [
        "default",
        "bark",
        "brick",
        "carpet",
        "cloth",
        "concrete",
        "dirt",
        "flesh",
        "foliage",
        "glass",
        "grass",
        "gravel",
        "ice",
        "metal",
        "mud",
        "paper",
        "plaster",
        "rock",
        "sand",
        "snow",
        "water",
        "wood",
        "asphalt",
        "ceramic",
        "plastic",
        "rubber",
        "cushion",
        "fruit",
        "paintedmetal",
        "player",
        "tallgrass",
        "riotshield",
    ];
    let (zone, root) = catalog.resolve(41, "info/bullet_penetration_mp")?;
    let streams = &catalog.zones[zone].1.streams;
    let length = streams.u32(root.add(4)?)? as usize;
    let buffer = Address::decode(streams.u32(root.add(8)?)?)?;
    let text = std::str::from_utf8(streams.read(buffer, length)?)?;
    let values: Vec<_> = text
        .strip_prefix("BULLET_PEN_TABLE")
        .ok_or("Native bullet penetration table has invalid magic")?
        .trim()
        .trim_matches('\0')
        .split('\\')
        .collect();
    let mut depths = [0.; 32];
    for (index, surface) in surfaces.into_iter().enumerate().skip(1) {
        let key = format!("{prefix}_{surface}");
        let entries: Vec<_> = values.windows(2).filter(|row| row[0] == key).collect();
        if entries.is_empty() {
            continue;
        }
        if entries.len() != 1 {
            return Err(format!("Native bullet penetration table repeats {key}").into());
        }
        let depth: f32 = entries[0][1].parse()?;
        if !depth.is_finite() || depth < 0. {
            return Err(format!("Native bullet penetration table has invalid {key}").into());
        }
        depths[index] = depth * 0.0254;
    }
    Ok(depths)
}

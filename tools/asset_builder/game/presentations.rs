use super::*;

pub(super) fn variable(
    catalog: &Catalog<'_>,
    name: &str,
    column: usize,
    fallback: Value,
) -> Result<Value> {
    let (zone, root) = catalog.resolve(42, "mp/zombiemode.csv")?;
    let streams = &catalog.zones[zone].1.streams;
    let columns = streams.u32(root.add(4)?)? as usize;
    let rows = streams.u32(root.add(8)?)? as usize;
    if column >= columns {
        return Err("Native zombie difficulty table lacks the required column".into());
    }
    let cells = Address::decode(streams.u32(root.add(12)?)?)?;
    let cell = |row: usize, column: usize| -> Result<String> {
        let pointer = streams.u32(cells.add((row * columns + column) * 8)?)?;
        if pointer == 0 {
            return Ok(String::new());
        }
        Ok(streams.text(Address::decode(pointer)?)?)
    };
    for row in 0..rows {
        if cell(row, 0)? != name {
            continue;
        }
        let text = cell(row, column)?;
        if text.is_empty() {
            return Ok(fallback);
        }
        let value: f64 = text.parse()?;
        if !value.is_finite() {
            return Err(format!("Native zombie variable is non-finite: {name}").into());
        }
        if (fallback.is_i64() || fallback.is_u64())
            && !(i32::MIN as f64..=i32::MAX as f64).contains(&value.trunc())
        {
            return Err(
                format!("Native zombie integer variable exceeds the source range: {name}").into(),
            );
        }
        return Ok(if fallback.is_i64() || fallback.is_u64() {
            json!(value.trunc() as i64)
        } else {
            json!(value)
        });
    }
    Ok(fallback)
}

pub(super) const WEAPON_ACTIONS: &[(usize, &str)] = &[
    (1, "idle"),
    (2, "idle_empty"),
    (4, "fire"),
    (5, "prime"),
    (6, "last_shot"),
    (7, "rechamber"),
    (9, "melee"),
    (16, "reload"),
    (18, "reload_empty"),
    (19, "reload_start"),
    (20, "reload_end"),
    (23, "equip"),
    (25, "unequip"),
    (32, "sprint_enter"),
    (33, "sprint_loop"),
    (34, "sprint_leave"),
    (44, "prone_enter"),
    (45, "prone_loop"),
    (46, "prone_back"),
    (47, "prone_right"),
    (48, "prone_left"),
    (49, "prone_leave"),
    (50, "prone_enter_empty"),
    (51, "prone_loop_empty"),
    (52, "prone_back_empty"),
    (53, "prone_right_empty"),
    (54, "prone_left_empty"),
    (55, "prone_leave_empty"),
    (61, "ads_fire"),
    (62, "ads_last_shot"),
    (64, "ads_rechamber"),
    (65, "dive_enter"),
    (66, "dive_loop"),
    (67, "dive_leave"),
    (68, "dive_enter_empty"),
    (69, "dive_loop_empty"),
    (70, "dive_leave_empty"),
    (85, "ads_enter"),
    (86, "ads_leave"),
];

pub(super) fn initial_presentations(
    catalog: &Catalog<'_>,
    game: &mut Value,
    entities: &[crate::entities::Entity],
) -> Result<()> {
    use std::collections::{BTreeMap, BTreeSet};
    let mut models = BTreeMap::<String, BTreeSet<String>>::new();
    let mut first_person_clips = BTreeSet::new();

    for name in super::equipment::INITIAL_WEAPONS {
        let (zone, root) = catalog.resolve(25, name)?;
        let streams = &catalog.zones[zone].1.streams;
        let definition = Address::decode(streams.u32(root.add(8)?)?)?;
        let gun_models = Address::decode(streams.u32(definition.add(4)?)?)?;
        let (view, _, _) = catalog.pointer(zone, gun_models, 5)?;
        let animations = Address::decode(streams.u32(root.add(32)?)?)?;
        for &(index, _) in WEAPON_ACTIONS {
            let pointer = streams.u32(animations.add(index * 4)?)?;
            if pointer == 0 {
                continue;
            }
            let clip = streams.text(Address::decode(pointer)?)?;
            if clip.is_empty() {
                continue;
            }
            catalog.resolve(4, &clip)?;
            first_person_clips.insert(clip.clone());
            models.entry(view.clone()).or_default().insert(clip);
        }
        for (offset, count, variant) in [
            (4, 16, false),
            (916, 16, false),
            (40, 8, true),
            (44, 8, true),
        ] {
            let field = if variant {
                root.add(offset)?
            } else {
                definition.add(offset)?
            };
            let pointer = streams.u32(field)?;
            if pointer == 0 {
                continue;
            }
            let array = Address::decode(pointer)?;
            for index in 0..count {
                let field = array.add(index * 4)?;
                if streams.u32(field)? != 0 {
                    models
                        .entry(catalog.pointer(zone, field, 5)?.0)
                        .or_default();
                }
            }
        }
        for offset in [8, 920, 924, 928, 932, 1768] {
            let field = definition.add(offset)?;
            if streams.u32(field)? != 0 {
                models
                    .entry(catalog.pointer(zone, field, 5)?.0)
                    .or_default();
            }
        }
    }
    for character in ["dempsey", "nikolai", "richtofen", "takeo"] {
        for (model, clips) in [
            (
                format!("c_zom_{character}_viewhands"),
                first_person_clips.clone(),
            ),
            (format!("c_zom_tomb_{character}_fb"), BTreeSet::new()),
        ] {
            catalog.resolve(5, &model)?;
            models.entry(model).or_default().extend(clips);
        }
    }
    let mut animations = Vec::new();
    let mut presentations = Vec::new();
    for (model, clips) in models {
        let skeleton = format!("actor.{model}");
        let (model_zone, model_root) = catalog.resolve(5, &model)?;
        let joint_names = crate::geometry::skeleton(catalog, &model, model_zone, model_root)?
            .into_iter()
            .filter_map(|joint| joint.name.map(|name| name.to_ascii_lowercase()))
            .collect::<BTreeSet<_>>();
        let mut ids = Vec::new();
        for clip in clips {
            let (zone, root) = catalog.resolve(4, &clip)?;
            let streams = &catalog.zones[zone].1.streams;
            let count = streams.u8(root.add(33)?)? as usize;
            let mapped = if count == 0 {
                false
            } else {
                let names = Address::decode(streams.u32(root.add(64)?)?)?;
                (0..count)
                    .map(|index| -> Result<bool> {
                        let word = streams.u16(names.add(index * 2)?)? as usize;
                        let address = streams
                            .script_strings
                            .get(word)
                            .copied()
                            .flatten()
                            .ok_or("Initial source animation bone name is missing")?;
                        Ok(joint_names.contains(&streams.text(address)?.to_ascii_lowercase()))
                    })
                    .collect::<Result<Vec<_>>>()?
                    .into_iter()
                    .any(|mapped| mapped)
            };
            if !mapped {
                continue;
            }
            let frames = streams.u16(root.add(14)?)? as f32;
            let rate = f32::from_bits(streams.u32(root.add(48)?)?);
            if !rate.is_finite() || rate <= 0. {
                return Err(
                    format!("Initial animation {clip} has invalid native frame rate").into(),
                );
            }
            let mut notifications = Vec::new();
            let count = streams.u8(root.add(34)?)? as usize;
            if count > 0 {
                let array = Address::decode(streams.u32(root.add(96)?)?)?;
                for index in 0..count {
                    let entry = array.add(index * 8)?;
                    let word = streams.u16(entry)? as usize;
                    let name = streams
                        .script_strings
                        .get(word)
                        .copied()
                        .flatten()
                        .ok_or_else(|| {
                            format!(
                                "Initial animation {clip} notification has no native script string"
                            )
                        })?;
                    let fraction = f32::from_bits(streams.u32(entry.add(4)?)?);
                    if !fraction.is_finite() || !(0. ..=1.).contains(&fraction) {
                        return Err(format!(
                            "Initial animation {clip} notification exceeds native time domain"
                        )
                        .into());
                    }
                    notifications
                        .push(json!({"seconds":fraction*frames/rate,"event":streams.text(name)?}));
                }
            }
            let id = format!("animation.{model}.{clip}");
            ids.push(id.clone());
            animations.push(json!({"id":id,"source_clip":clip,"path":null,"skeleton":skeleton,"seconds":frames/rate,"notifications":notifications,"looping":streams.u8(root.add(16)?)? != 0}));
        }
        presentations.push(
            json!({"id":skeleton,"source_model":model,"scene":null,"animations":ids,"sounds":[]}),
        );
    }
    game["content"]["presentations"] = json!(presentations);
    game["content"]["animations"] = json!(animations);
    let mut spawns = Vec::new();
    for entity in entities.iter().filter(|entity| {
        entity
            .get("targetname")
            .is_some_and(|name| name == "initial_spawn_points")
    }) {
        if entity.get("classname").map(String::as_str) != Some("script_struct")
            || entity.get("script_string").is_some()
        {
            return Err(
                "Initial Origins spawn eligibility requires unsupported source entity conditions"
                    .into(),
            );
        }
        let [x, y, z] = crate::entities::vector(entity, "origin")?
            .ok_or("Initial native spawn lacks origin")?;
        let [pitch, yaw, _] = crate::entities::vector(entity, "angles")?
            .ok_or("Initial native spawn lacks angles")?;
        let side: u8 = entity
            .get("script_int")
            .ok_or("Initial native spawn lacks source side")?
            .parse()?;
        if ![1, 2].contains(&side) {
            return Err("Initial native spawn side is outside the recovered source branch".into());
        }
        let pitch = (pitch + 180.).rem_euclid(360.) - 180.;
        spawns.push(json!({"position":[x*0.0254,z*0.0254,-y*0.0254],"yaw":(yaw-90.).to_radians(),"pitch":-pitch.to_radians(),"excluded_side":side}));
    }
    if spawns.len() != 8 {
        return Err(format!(
            "Origins initial spawn selection requires its eight native structs, found {}",
            spawns.len()
        )
        .into());
    }
    game["spawn"] = spawns[0]["position"].clone();
    game["yaw"] = spawns[0]["yaw"].clone();
    game["initial_spawns"] = json!(spawns);
    game["spawn_points"] = json!(
        spawns
            .iter()
            .map(|spawn| spawn["position"].clone())
            .collect::<Vec<_>>()
    );
    Ok(())
}

pub(super) fn initial_ui(catalog: &Catalog<'_>, game: &mut Value) -> Result<()> {
    let mut assets = Vec::new();
    for name in [
        "overlay_low_health",
        "scorebar_zom_1",
        "hud_zm_tomb_dpad_bar",
        "hud_zm_tomb_dpad",
        "hud_us_grenade",
        "damage_feedback",
        "menu_zm_map_tomb",
        "hud_zm_num_tomb_slash",
    ]
    .into_iter()
    .map(str::to_owned)
    .chain((0..=9).map(|number| format!("hud_zm_num_tomb_{number}")))
    .chain((1..=5).map(|number| format!("chalkmarks_hellcatraz_{number}")))
    {
        let (image, _, _) = ui_image(catalog, &name)?;
        assets.push(json!({"id":format!("ui.{name}"),"source_image":image,"path":null}));
    }
    for (style, name) in [
        ("small", "fonts/720/smallFont"),
        ("extraSmall", "fonts/720/extraSmallFont"),
        ("extraBig", "fonts/720/extraBigFont"),
    ] {
        let (zone, root) = catalog.resolve(20, name)?;
        let streams = &catalog.zones[zone].1.streams;
        let height = streams.u32(root.add(4)?)? as i32;
        if height <= 0 {
            return Err(format!("Initial font {name} has invalid native height {height}").into());
        }
        let (material, _, _) = catalog.pointer(zone, root.add(20)?, 6)?;
        let (image, image_zone, image_root) = ui_image(catalog, &material)?;
        let image_streams = &catalog.zones[image_zone].1.streams;
        let width = image_streams.u16(image_root.add(20)?)? as f32;
        let atlas_height = image_streams.u16(image_root.add(22)?)? as f32;
        if width <= 0. || atlas_height <= 0. {
            return Err(format!("Initial font {name} has an empty native atlas").into());
        }
        let count = streams.u32(root.add(12)?)? as usize;
        if count == 0 {
            return Err(format!("Initial font {name} has no native glyphs").into());
        }
        let array = Address::decode(streams.u32(root.add(28)?)?)?;
        let mut glyphs = Vec::with_capacity(count);
        for index in 0..count {
            let glyph = array.add(index * 24)?;
            let scalar = |offset| -> Result<f32> {
                let value = f32::from_bits(streams.u32(glyph.add(offset)?)?);
                if !value.is_finite() || !(0. ..=1.).contains(&value) {
                    return Err(format!(
                        "Initial font {name} glyph {index} has invalid native atlas coordinates"
                    )
                    .into());
                }
                Ok(value)
            };
            let rect = [
                scalar(8)? * width,
                scalar(12)? * atlas_height,
                scalar(16)? * width,
                scalar(20)? * atlas_height,
            ];
            if rect[0] > rect[2] || rect[1] > rect[3] {
                return Err(
                    format!("Initial font {name} glyph {index} has reversed native UVs").into(),
                );
            }
            glyphs.push(json!({
                "codepoint":streams.u16(glyph)?,
                "offset":[streams.u8(glyph.add(2)?)? as i8,streams.u8(glyph.add(3)?)? as i8],
                "advance":streams.u8(glyph.add(4)?)?,
                "size":[streams.u8(glyph.add(5)?)?,streams.u8(glyph.add(6)?)?],
                "rect":rect
            }));
        }
        assets.push(
            json!({"id":format!("ui.font.{style}"),"source_image":image,"path":null,
            "font":{"height":height,"atlas":[width,atlas_height],"glyphs":glyphs}}),
        );
    }
    let ui = game["content"]["ui_assets"]
        .as_array_mut()
        .ok_or("Initial equipment UI bindings are missing")?;
    for asset in assets {
        if ui.iter().any(|existing| existing["id"] == asset["id"]) {
            return Err(format!("Initial UI binding is duplicated: {}", asset["id"]).into());
        }
        ui.push(asset);
    }
    println!(
        "Converted {} initial native UI images and bitmap fonts",
        ui.len()
    );
    Ok(())
}

pub(super) fn ui_image(catalog: &Catalog<'_>, name: &str) -> Result<(String, usize, Address)> {
    if !catalog
        .definitions(6)
        .any(|(material, _, _)| material.eq_ignore_ascii_case(name))
    {
        return Err(format!("Required initial UI material definition is missing: {name}").into());
    }
    let (zone, root) = catalog.resolve(6, name)?;
    let streams = &catalog.zones[zone].1.streams;
    let count = streams.u8(root.add(84)?)?;
    if count != 1 {
        return Err(format!(
            "Required initial UI material {name} needs its native {count}-texture binding"
        )
        .into());
    }
    let table = Address::decode(streams.u32(root.add(96)?)?)?;
    catalog.pointer(zone, table.add(12)?, 8)
}

#[test]
#[ignore = "requires the authorized Steam installation"]
fn installed_initial_equipment_profiles() {
    let zones = super::read_zones(&super::installation().unwrap()).unwrap();
    let catalog = crate::catalog::Catalog::new(&zones);
    let entities = crate::entities::read(&catalog).unwrap();
    let (game, gaps) = crate::game::read(&catalog, &entities).unwrap();
    let weapons = game["content"]["weapons"].as_array().unwrap();
    let pistol = weapons.iter().find(|w| w["id"] == "weapon.c96_zm").unwrap();
    assert_eq!(pistol["magazine"], 8);
    assert_eq!(pistol["mechanics"]["starting_reserve"], 32);
    assert_eq!(pistol["reserve"], 80);
    assert_eq!(pistol["damage"], 20);
    assert_eq!(pistol["mechanics"]["fire"]["mode"], "semi");
    let depths = pistol["mechanics"]["penetration"].as_array().unwrap();
    assert_eq!(depths.len(), 32);
    for (index, inches) in [
        (0, 0.),
        (5, 6.),
        (7, 32.),
        (9, 72.),
        (13, 12.),
        (21, 12.),
        (29, 0.),
    ] {
        assert!((depths[index].as_f64().unwrap() - inches * 0.0254).abs() < 0.00001);
    }
    let knife = weapons
        .iter()
        .find(|w| w["id"] == "weapon.knife_zm")
        .unwrap();
    assert_eq!(knife["damage"], 150);
    assert_eq!(knife["mechanics"]["delivery"], "melee");
    assert_eq!(
        game["solo_revive"]["pistol_ranks"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    for weapon in weapons {
        for clip in weapon["animation_actions"].as_object().unwrap().values() {
            assert!(
                game["content"]["animations"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|row| row["id"] == *clip)
            );
        }
        assert!(
            game["content"]["presentations"]
                .as_array()
                .unwrap()
                .iter()
                .any(|row| row["id"] == weapon["presentation"])
        );
    }
    for gap in gaps.iter().filter(|gap| gap.starts_with("Initial weapon")) {
        println!("Equipment gap: {gap}");
    }
    assert!((knife["mechanics"]["range"].as_f64().unwrap() - 55. * 0.0254).abs() < 0.00001);
    let grenade = weapons
        .iter()
        .find(|w| w["id"] == "weapon.frag_grenade_zm")
        .unwrap();
    assert_eq!(grenade["magazine"], 4);
    assert_eq!(
        grenade["mechanics"]["explosion_damage"],
        serde_json::json!([300, 75])
    );
    assert!((grenade["mechanics"]["fuse_seconds"].as_f64().unwrap() - 3.5).abs() < 0.00001);
    let (zone, root) = catalog.resolve(25, "frag_grenade_zm").unwrap();
    let streams = &catalog.zones[zone].1.streams;
    let definition =
        crate::fastfile::Address::decode(streams.u32(root.add(8).unwrap()).unwrap()).unwrap();
    println!(
        "Native frag trajectory flags rolling/stickiness/rotateType/rotate/keepRolling: {:?}",
        [1688, 1840, 1844, 1852, 1853].map(|offset| if offset < 1852 {
            streams.u32(definition.add(offset).unwrap()).unwrap()
        } else {
            streams.u8(definition.add(offset).unwrap()).unwrap() as u32
        })
    );
    for name in ["c96_zm", "c96_upgraded_zm"] {
        let (zone, root) = catalog.resolve(25, name).unwrap();
        let streams = &catalog.zones[zone].1.streams;
        let animations =
            crate::fastfile::Address::decode(streams.u32(root.add(32).unwrap()).unwrap()).unwrap();
        let mut names = std::collections::BTreeSet::new();
        for index in [85, 86] {
            let pointer = streams.u32(animations.add(index * 4).unwrap()).unwrap();
            if pointer != 0 {
                names.insert(
                    streams
                        .text(crate::fastfile::Address::decode(pointer).unwrap())
                        .unwrap(),
                );
            }
        }
        for clip in crate::animation::read(&catalog, Some(&names)).unwrap() {
            assert_eq!(clip.clip.source_flags, [0; 4]);
            assert_eq!(clip.clip.bones.len(), 1);
            assert_eq!(clip.clip.bones[0].name.as_deref(), Some("tag_torso"));
            assert!(clip.clip.bones[0].translation.is_some());
        }
        assert!(
            weapons
                .iter()
                .find(|weapon| weapon["id"] == format!("weapon.{name}"))
                .unwrap()["mechanics"]["native_ads_pitch"]
                .is_number()
        );
    }
}

#[test]
#[ignore = "requires the authorized Steam installation"]
fn installed_initial_animation_bindings() {
    use std::collections::{BTreeMap, BTreeSet};
    let zones = super::read_zones(&super::installation().unwrap()).unwrap();
    let catalog = crate::catalog::Catalog::new(&zones);
    let entities = crate::entities::read(&catalog).unwrap();
    let (game, _) = crate::game::read(&catalog, &entities).unwrap();
    let definitions = game["content"]["animations"].as_array().unwrap();
    let required = definitions
        .iter()
        .map(|row| row["source_clip"].as_str().unwrap().to_ascii_lowercase())
        .collect::<BTreeSet<_>>();
    let decoded = crate::animation::read(&catalog, Some(&required)).unwrap();
    for clip in decoded
        .iter()
        .filter(|clip| clip.clip.loop_entry_time != 0.)
    {
        println!(
            "Selected nonzero loop-entry clip {}: {}",
            clip.clip.name, clip.clip.loop_entry_time
        );
    }
    let index = decoded
        .iter()
        .map(|clip| (clip.clip.name.to_ascii_lowercase(), clip))
        .collect::<BTreeMap<_, _>>();
    let mut gaps = BTreeSet::new();
    let mut bindings = 0;
    for presentation in game["content"]["presentations"].as_array().unwrap() {
        let name = presentation["source_model"].as_str().unwrap();
        let (zone, root) = catalog.resolve(5, name).unwrap();
        let joints = crate::geometry::skeleton(&catalog, name, zone, root).unwrap();
        let mut scene = crate::gltf::Scene::new(name);
        let owner = scene.node(serde_json::json!({"name":name}));
        scene.skin(owner, &joints).unwrap();
        for definition in definitions
            .iter()
            .filter(|row| row["skeleton"] == presentation["id"])
        {
            let clip = index[&definition["source_clip"]
                .as_str()
                .unwrap()
                .to_ascii_lowercase()];
            if clip.has_root_delta && definition["root_motion"] != crate::game::root_motion(clip) {
                gaps.insert(format!(
                    "{} has unmatched runtime root-motion samples",
                    clip.clip.name
                ));
            }
            match scene.animation(owner, &joints, &clip.clip) {
                Ok(_) => bindings += 1,
                Err(error) => {
                    gaps.insert(error.to_string());
                }
            }
        }
    }
    assert!(
        gaps.is_empty(),
        "{} selected animation export gaps:\n{}",
        gaps.len(),
        gaps.iter().take(20).cloned().collect::<Vec<_>>().join("\n")
    );
    println!(
        "Validated {bindings} exact initial model/clip bindings from {} native clips",
        decoded.len()
    );
}

#[test]
#[ignore = "requires the authorized Steam installation"]
fn installed_initial_material_dependencies() {
    let zones = super::read_zones(&super::installation().unwrap()).unwrap();
    let catalog = crate::catalog::Catalog::new(&zones);
    let entities = crate::entities::read(&catalog).unwrap();
    let (game, _) = crate::game::read(&catalog, &entities).unwrap();
    assert_eq!(game["delivery_scope"], "first_playable_solo");
    let models = crate::scenes::required_models(&catalog, &game).unwrap();
    assert!(models.contains("t6_wpn_zmb_mc96_view"));
    assert!(models.len() < catalog.definitions(5).count());
    let materials = crate::geometry::material_names(&catalog, Some(&models)).unwrap();
    let all = crate::geometry::material_names(&catalog, None).unwrap();
    assert!(materials.is_subset(&all) && materials.len() < all.len());
    let native = crate::material::read_used(&catalog, &materials).unwrap();
    let gaps = crate::material::unimplemented(&native);
    let watched = |name: &str| {
        [
            "generator_115",
            "soul_box_glow",
            "crystal",
            "plinth_glow",
            "elm_spawn_glow",
            "portal_vortex",
            "rock_hex_glyph",
            "elm_ltng_keys",
            "rock_lava_molten",
            "wallbuy",
        ]
        .iter()
        .any(|part| name.contains(part))
    };
    for (name, material) in &native {
        if watched(name)
            || material["technique_set"]
                .as_str()
                .is_some_and(|family| family.contains("char_cloth_reveal2"))
        {
            println!(
                "Initial material binding {name}: family {}",
                material["technique_set"]
            );
        }
    }
    for (name, zone, root) in catalog
        .definitions(5)
        .filter(|(name, _, _)| models.contains(&name.to_ascii_lowercase()))
    {
        let streams = &catalog.zones[zone].1.streams;
        let bindings =
            crate::fastfile::Address::decode(streams.u32(root.add(36).unwrap()).unwrap()).unwrap();
        let materials = (0..streams.u8(root.add(6).unwrap()).unwrap() as usize)
            .map(|index| {
                catalog
                    .pointer(zone, bindings.add(index * 4).unwrap(), 6)
                    .unwrap()
                    .0
                    .to_ascii_lowercase()
            })
            .filter(|material| watched(material))
            .collect::<std::collections::BTreeSet<_>>();
        if materials.is_empty() {
            continue;
        }
        println!("Initial script material model {name}: {materials:?}");
        for (index, entity) in entities.iter().enumerate().filter(|(_, entity)| {
            entity
                .get("model")
                .is_some_and(|model| model.eq_ignore_ascii_case(name))
        }) {
            println!(
                "Initial script model entity {index}: class {:?}, target {:?}, noteworthy {:?}, origin {:?}, angles {:?}, materials {materials:?}",
                entity.get("classname"),
                entity.get("targetname"),
                entity.get("script_noteworthy"),
                entity.get("origin"),
                entity.get("angles")
            );
        }
    }
    for (_, zone, root) in catalog.definitions(17) {
        let streams = &catalog.zones[zone].1.streams;
        let surfaces =
            crate::fastfile::Address::decode(streams.u32(root.add(872).unwrap()).unwrap()).unwrap();
        let brushes =
            crate::fastfile::Address::decode(streams.u32(root.add(540).unwrap()).unwrap()).unwrap();
        for brush in 0..streams.u32(root.add(536).unwrap()).unwrap() as usize {
            let model = brushes.add(brush * 64).unwrap();
            let first = streams.u32(model.add(60).unwrap()).unwrap() as usize;
            let count = streams.u32(model.add(56).unwrap()).unwrap() as usize;
            let materials = (first..first + count)
                .filter(|index| streams.u16(surfaces.add(index * 80 + 42).unwrap()).unwrap() != 0)
                .map(|index| {
                    catalog
                        .pointer(zone, surfaces.add(index * 80 + 48).unwrap(), 6)
                        .unwrap()
                        .0
                        .to_ascii_lowercase()
                })
                .filter(|name| watched(name))
                .collect::<std::collections::BTreeSet<_>>();
            if materials.is_empty() {
                continue;
            }
            println!("Initial script material brush *{brush}: {materials:?}");
            for (index, entity) in entities.iter().enumerate().filter(|(_, entity)| {
                entity
                    .get("model")
                    .is_some_and(|model| model == &format!("*{brush}"))
            }) {
                println!(
                    "Initial script brush entity {index}: target {:?}, noteworthy {:?}, materials {materials:?}",
                    entity.get("targetname"),
                    entity.get("script_noteworthy")
                );
            }
        }
    }
    assert!(
        gaps.is_empty(),
        "Initial material families remain:\n{}",
        gaps.join("\n")
    );
    println!(
        "Initial world/static props plus current source-selected presentation roots: {} of {} models, {} of {} materials; {} missing families before remaining actor/content closure:\n{}",
        models.len(),
        catalog.definitions(5).count(),
        materials.len(),
        all.len(),
        gaps.len(),
        gaps.join("\n")
    );
    for image in crate::atmosphere::inputs(&catalog).unwrap() {
        catalog.resolve(8, &image).unwrap();
    }
}

#[test]
#[ignore = "requires the authorized Steam installation"]
fn installed_last_material_families() {
    use serde_json::{Value, json};
    use std::collections::{BTreeMap, BTreeSet};
    let families = [
        "mc_sw4_3d_char_cloth_4z8fq5wu",
        "mc_sw4_3d_char_eye_cornea_2eww29wu",
        "mc_sw4_3d_char_skin_j92387z3",
        "mc_sw4_3d_optic_lens_4w47028j",
        "mc_sw4_3d_unlit_3layer_normal_script_q0zez5ju",
        "mc_sw4_3d_unlit_4layer_jw0z1973",
        "mc_sw4_3d_viewmodel_transparent_z9z0z75f",
        "mc_sw4_3d_zmb_smoke_pillar_cheap_w6w7e738",
        "mc_treecanopy_3z378q8f",
        "mc_treecanopy_sm_68zw237j",
        "mc_treecanopy_sm_j9ff996z",
        "mlv_sw4_3d_unlit_3layer_nofog_falloff_zfeather_z0zw1090",
        "mlv_sw4_3d_unlit_3layer_normal_script_766fz50u",
        "wpc_cod7water_we876z2f",
        "wpc_default",
        "wpc_shadowcaster_wj6w5j60",
        "wpc_sw4_3d_burning_embers_nuketown_74j6971w",
        "wpc_sw4_3d_unlit_3layer_normal_script_q1z1f6w1",
    ];
    let install = super::installation().unwrap();
    let zones = super::read_zones(&install).unwrap();
    let catalog = crate::catalog::Catalog::new(&zones);
    let names = crate::geometry::material_names(&catalog, None).unwrap();
    let mut native = crate::material::read_used(&catalog, &names).unwrap();
    native.retain(|_, material| families.contains(&material["technique_set"].as_str().unwrap()));
    assert_eq!(native.len(), 82);
    assert!(crate::material::unimplemented(&native).is_empty());
    let required = crate::material::inputs(&native, &catalog).unwrap();
    let lava = BTreeMap::from([(
        "wpc/zm_tm_rock_lava_molten".into(),
        native["wpc/zm_tm_rock_lava_molten"].clone(),
    )]);
    let inactive = native["wpc/zm_tm_rock_lava_molten"]["textures"]
        .as_array()
        .unwrap()
        .iter()
        .find(|binding| binding["hash"] == 1507003663_u32)
        .unwrap()["image"]
        .as_str()
        .unwrap();
    assert!(
        !crate::material::inputs(&lava, &catalog)
            .unwrap()
            .contains(&inactive.to_ascii_lowercase())
    );
    let mut seen = BTreeSet::new();
    for (name, material) in &native {
        let family = material["technique_set"].as_str().unwrap();
        if !seen.insert(family) {
            continue;
        }
        let index = crate::material::selected_technique(family).unwrap();
        let mut changed = material.clone();
        let pass = changed["techniques"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|technique| technique["index"] == index)
            .unwrap()["passes"][0]
            .as_object_mut()
            .unwrap();
        let active = pass["arguments"]
            .as_array()
            .unwrap()
            .iter()
            .position(|argument| argument["type"] == 2)
            .unwrap();
        pass["arguments"][active]["value"] = json!(0_u32);
        assert!(
            crate::material::inputs(&BTreeMap::from([(name.clone(), changed)]), &catalog).is_err(),
            "{name}: changed active binding accepted"
        );
    }
    assert_eq!(seen.len(), 18);
    let mut pack = crate::pack::Pack::default();
    let (_, images) =
        crate::images::convert(&install, &catalog, &mut pack, Some(&required)).unwrap();
    let converted = crate::material::convert_known(&native, &images, &mut pack).unwrap();
    assert_eq!(converted.len(), native.len());
    fn links(value: &Value, pack: &crate::pack::Pack) {
        match value {
            Value::Object(fields) => {
                for (key, value) in fields {
                    if key == "uri" || key.ends_with("_uri") {
                        if let Some(uri) = value.as_str() {
                            pack.get(uri)
                                .unwrap_or_else(|error| panic!("{uri}: {error}"));
                        }
                    }
                    links(value, pack);
                }
            }
            Value::Array(values) => {
                for value in values {
                    links(value, pack);
                }
            }
            _ => (),
        }
    }
    for (name, material) in &converted {
        let surface = &material["extras"]["surface"];
        assert_eq!(surface["version"], 3, "{name}");
        assert_eq!(surface["identity"], name.as_str());
        assert_eq!(surface["native"], native[name]);
        assert_eq!(
            surface["presentation"]["visible"],
            native[name]["technique_set"] != "wpc_shadowcaster_wj6w5j60",
            "{name}"
        );
        links(surface, &pack);
    }
    for (uri, bytes) in &pack.entries {
        if uri.ends_with(".ktx2") {
            crate::texture::validate(bytes).unwrap();
        }
    }
    let pending = crate::material::missing(&native, &converted);
    assert!(pending.iter().all(|gap| {
        native
            .keys()
            .any(|name| gap.starts_with(&format!("{name}: ")))
    }));
    assert!(
        !pending.is_empty(),
        "Missing live scene/object consumers must remain explicit"
    );
    let mut controlled = BTreeMap::new();
    let mut seen = BTreeSet::new();
    for (name, material) in &native {
        let family = material["technique_set"].as_str().unwrap();
        if !family.contains("normal_script") || !seen.insert(family) {
            continue;
        }
        assert_eq!(
            converted[name]["extras"]["surface"]["source_script_layers"]["composed_color_available"],
            false
        );
        let mut material = material.clone();
        material["code_constants"]["169"] = json!([0.5, 0.25, 0., 0.]);
        controlled.insert(name.clone(), material);
    }
    assert_eq!(controlled.len(), 3);
    let mut controlled_pack = crate::pack::Pack::default();
    let controlled =
        crate::material::convert_known(&controlled, &images, &mut controlled_pack).unwrap();
    for (name, material) in &controlled {
        let surface = &material["extras"]["surface"];
        assert_eq!(
            surface["source_script_layers"]["composed_color_available"], true,
            "{name}"
        );
        assert!(surface["pending_runtime"].is_string());
        assert!(!surface["channels"].as_array().unwrap().is_empty());
        links(surface, &controlled_pack);
    }
    for (uri, bytes) in &controlled_pack.entries {
        if uri.ends_with(".ktx2") {
            crate::texture::validate(bytes).unwrap();
        }
    }
    println!(
        "Converted all {} installed materials in eighteen families and three explicit normal-script states; {} active images and {} packed entries resolve; {} live runtime bindings remain explicit",
        native.len(),
        required.len(),
        pack.entries.len(),
        pending.len()
    );
}

#[test]
#[ignore = "requires the authorized Steam installation"]
fn installed_zone_activation_geometry() {
    let zones = super::read_zones(&super::installation().unwrap()).unwrap();
    let catalog = crate::catalog::Catalog::new(&zones);
    let entities = crate::entities::read(&catalog).unwrap();
    let collision = crate::collision::convert(&catalog, &mut crate::pack::Pack::default()).unwrap();
    for name in [
        "trig_zone_bunker_1",
        "trig_zone_bunker_2",
        "trig_zone_bunker_4",
        "trig_zone_bunker_6",
    ] {
        let matching: Vec<_> = entities
            .iter()
            .enumerate()
            .filter(|(_, entity)| {
                entity
                    .get("targetname")
                    .is_some_and(|target| target == name)
            })
            .collect();
        assert_eq!(matching.len(), 1, "Source zone activation {name}");
        let (index, entity) = matching[0];
        if let Some(model) = entity
            .get("model")
            .and_then(|model| model.strip_prefix('*'))
        {
            let model: u64 = model.parse().unwrap();
            let shape = collision["brush_models"]
                .as_array()
                .unwrap()
                .iter()
                .find(|shape| shape["native_brush_model"] == model)
                .unwrap();
            assert!(
                !shape["volumes"].as_array().unwrap().is_empty()
                    || !shape["triangle_shapes"].as_array().unwrap().is_empty()
            );
            println!(
                "Source activation {name}, entity {index}, fields {entity:?}, brush {model}, {} convex volumes, {} triangle shapes; contents {:?}",
                shape["volumes"].as_array().unwrap().len(),
                shape["triangle_shapes"].as_array().unwrap().len(),
                shape["volumes"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|volume| &volume["contents"])
                    .collect::<Vec<_>>()
            );
        } else {
            println!("Source activation {name}, entity {index}, fields {entity:?}");
        }
    }
}

#[test]
#[ignore = "requires the authorized Steam installation"]
fn installed_ordinary_animation_streams() {
    let zones = super::read_zones(&super::installation().unwrap()).unwrap();
    let catalog = crate::catalog::Catalog::new(&zones);
    crate::animation::read(&catalog, None).unwrap();
}

#[test]
#[ignore = "requires the authorized Steam installation"]
fn installed_collision_brush_models() {
    let zones = super::read_zones(&super::installation().unwrap()).unwrap();
    let catalog = crate::catalog::Catalog::new(&zones);
    crate::collision::convert(&catalog, &mut crate::pack::Pack::default()).unwrap();
}

#[test]
#[ignore = "requires the authorized Steam installation"]
fn installed_multiply_materials() {
    installed_material_families(
        &[
            "lit_sm_r0c0n0_m1c1",
            "lit_sm_r0c0n0_m1c1_m2c2",
            "lit_sm_r0c0n0x0_m1c1",
        ],
        15,
    );
}

#[test]
#[ignore = "requires the authorized Steam installation"]
fn installed_additive_materials() {
    installed_material_families(&["lit_sm_r0c0n0x0_a1c1"], 3);
}

#[test]
#[ignore = "requires the authorized Steam installation"]
fn installed_color_blend_materials() {
    installed_material_families(&["lit_sm_r0c0n0x0_b1c1", "lit_sm_r0c0n0x0_b1c1_b2c2"], 11);
}

#[test]
#[ignore = "requires the authorized Steam installation"]
fn installed_diffuse_reveal_materials() {
    installed_material_families(&["lit_sm_r0c0n0x0_b1c1n1v1"], 5);
}

#[test]
#[ignore = "requires the authorized Steam installation"]
fn installed_ordinary_materials_batch() {
    let install = super::installation().unwrap();
    let zones = super::read_zones(&install).unwrap();
    let catalog = crate::catalog::Catalog::new(&zones);
    let used = crate::geometry::validate(&catalog, None).unwrap();
    let mut materials = crate::material::read_used(&catalog, &used).unwrap();
    let families = [
        "mc_lit_sm_r0c0s0_986ezzjq",
        "wpc_lit_sm_r0c0n0_80fe30wz",
        "mc_lit_sm_t0c0_90wz6fe2",
        "wpc_lit_sm_t0c0_2w3887e6",
        "wpc_lit_sm_t0c0n0_9ew0jz24",
    ];
    materials.retain(|_, material| {
        material["technique_set"]
            .as_str()
            .is_some_and(|family| families.contains(&family))
    });
    assert_eq!(materials.len(), 38);
    let required = materials
        .values()
        .flat_map(|material| material["textures"].as_array().unwrap())
        .map(|binding| binding["image"].as_str().unwrap().to_ascii_lowercase())
        .collect();
    let mut pack = crate::pack::Pack::default();
    let (_, images) =
        crate::images::convert(&install, &catalog, &mut pack, Some(&required)).unwrap();
    let converted = crate::material::convert_known(&materials, &images, &mut pack).unwrap();
    assert!(crate::material::missing(&materials, &converted).is_empty());
    let (mut masked, mut normals, mut responses) = (0, 0, 0);
    for material in converted.values() {
        let surface = &material["extras"]["surface"];
        masked += usize::from(surface["presentation"]["alpha"] == "mask");
        for channel in surface["channels"].as_array().unwrap() {
            normals += usize::from(channel["role"] == "normal");
            responses += usize::from(channel["role"] == "response");
            crate::texture::validate(pack.get(channel["uri"].as_str().unwrap()).unwrap()).unwrap();
        }
    }
    assert_eq!((masked, normals, responses), (24, 8, 8));
    let mut scenes = 0;
    let mut triangles = 0;
    for (_, zone, root) in catalog.definitions(17) {
        let surfaces = crate::geometry::world(&catalog, zone, root).unwrap();
        let selected: Vec<_> = surfaces
            .iter()
            .filter(|surface| converted.contains_key(&surface.material.to_ascii_lowercase()))
            .collect();
        if selected.is_empty() {
            continue;
        }
        let mut scene = crate::gltf::Scene::new("ordinary-world");
        let indices = scene
            .materials(selected.iter().copied(), &converted)
            .unwrap();
        let mesh = scene
            .mesh("ordinary-world", selected.iter().copied(), &indices, false)
            .unwrap();
        scene.node(serde_json::json!({"mesh":mesh}));
        triangles += selected
            .iter()
            .map(|surface| surface.indices.len() / 3)
            .sum::<usize>();
        scene
            .write(&format!("prepared/ordinary/{scenes}.gltf"), &mut pack)
            .unwrap();
        scenes += 1;
    }
    for (name, zone, root) in catalog.definitions(5) {
        for surfaces in crate::geometry::model(&catalog, name, zone, root).unwrap() {
            let selected: Vec<_> = surfaces
                .iter()
                .filter(|surface| converted.contains_key(&surface.material.to_ascii_lowercase()))
                .collect();
            if selected.is_empty() {
                continue;
            }
            let mut scene = crate::gltf::Scene::new(name);
            let indices = scene
                .materials(selected.iter().copied(), &converted)
                .unwrap();
            let mesh = scene
                .mesh(name, selected.iter().copied(), &indices, true)
                .unwrap();
            let owner = scene.node(serde_json::json!({"name":name,"mesh":mesh}));
            scene
                .skin(
                    owner,
                    &crate::geometry::skeleton(&catalog, name, zone, root).unwrap(),
                )
                .unwrap();
            triangles += selected
                .iter()
                .map(|surface| surface.indices.len() / 3)
                .sum::<usize>();
            scene
                .write(&format!("prepared/ordinary/{scenes}.gltf"), &mut pack)
                .unwrap();
            scenes += 1;
        }
    }
    assert!(scenes > 1 && triangles > 0);
    for (uri, bytes) in &pack.entries {
        if !uri.ends_with(".gltf") {
            continue;
        }
        let doc: serde_json::Value = serde_json::from_slice(bytes).unwrap();
        for buffer in doc["buffers"].as_array().unwrap() {
            crate::dependency(
                &pack,
                std::path::Path::new(uri).parent().unwrap(),
                buffer["uri"].as_str().unwrap(),
            )
            .unwrap();
        }
    }
    println!(
        "Converted five ordinary families: {} materials, {masked} masked, {normals} normal maps, {responses} response maps; exported {scenes} world/model LOD scenes and {triangles} triangles into {} in-memory pack entries",
        converted.len(),
        pack.entries.len()
    );
}

#[test]
#[ignore = "requires the authorized Steam installation"]
fn installed_ordinary_materials_increment() {
    let install = super::installation().unwrap();
    let zones = super::read_zones(&install).unwrap();
    let catalog = crate::catalog::Catalog::new(&zones);
    let used = crate::geometry::material_names(&catalog, None).unwrap();
    let all = crate::material::read_used(&catalog, &used).unwrap();
    assert_eq!(all.len(), 1034);
    let gaps = crate::material::unimplemented(&all);
    let camos = crate::material::read_camos(&catalog).unwrap();
    let options = crate::material::read_camo_options(&catalog).unwrap();
    let (weapons, attachments) = crate::material::read_material_selection(&catalog).unwrap();
    assert_eq!(weapons.len(), 130);
    assert_eq!(attachments.len(), 56);
    for (name, alternate) in [
        ("weapon.fivesevendw_zm", "weapon.fivesevenlh_zm"),
        ("weapon.fivesevenlh_zm", "weapon.fivesevendw_zm"),
        (
            "weapon.fivesevendw_upgraded_zm",
            "weapon.fivesevenlh_upgraded_zm",
        ),
        (
            "weapon.fivesevenlh_upgraded_zm",
            "weapon.fivesevendw_upgraded_zm",
        ),
    ] {
        let weapon = &weapons[name];
        assert_eq!(weapon.alternate_weapon_name.as_deref(), Some(alternate));
        assert_ne!(weapon.shared_ammo_related, 0);
        assert_eq!(weapon.alternate_weapon_index, 0);
        assert!(weapons.contains_key(alternate));
    }
    assert!(
        weapons
            .values()
            .all(|weapon| weapon.base_attachment_flags.len() == 63)
    );
    for name in weapons
        .values()
        .filter_map(|weapon| weapon.camo.as_ref())
        .chain(
            attachments
                .values()
                .filter_map(|attachment| attachment.camo.as_ref()),
        )
    {
        assert!(camos.contains_key(name), "Selected source camo {name}");
    }
    for (name, a, b) in [
        ("weapon.hamr_upgraded_zm", 1, 12),
        ("weapon.type95_upgraded_zm", 11, 16),
        ("weapon.type95_upgraded_zm", 19, 11),
    ] {
        let weapon = &weapons[name];
        let index = weapon.main_models.iter().position(Option::is_some).unwrap();
        let selected = weapon.attachments[a].as_ref().unwrap();
        let original_camo = &attachments[selected].camo;
        for mode in [0, 1] {
            let output = crate::camo::viewmodel_materials(
                weapon,
                None,
                &attachments,
                "controlled.slot0",
                ((a as u32) << 8) | ((b as u32) << 14),
                0,
                index,
                mode,
                None,
                [None; 2],
            )
            .unwrap();
            assert_eq!(output.models[0], "controlled.slot0");
            assert_eq!(
                output.models[1],
                *weapon.main_models[index].as_ref().unwrap()
            );
            assert!(output.descriptors.len() <= 8);
            assert!(
                output
                    .descriptors
                    .iter()
                    .all(|descriptor| descriptor.mask & 1 == 0)
            );
            assert!(
                output.descriptors.iter().any(
                    |descriptor| descriptor.camo == *original_camo && descriptor.mask & !3 != 0
                )
            );
        }
    }
    println!(
        "Resolved all {} weapon and {} attachment selector inputs and six controlled native combined-attachment descriptor plans; this is not active owner delivery",
        weapons.len(),
        attachments.len()
    );
    assert_eq!(camos.len(), 35);
    assert!(options[45].unwrap().material);
    assert_eq!(options[45].unwrap().selection, 13);
    let records: Vec<_> = camos
        .values()
        .flat_map(|camo| camo.material_sets.get(12).into_iter().flatten())
        .collect();
    assert_eq!(records.len(), 68);
    assert_eq!(
        records
            .iter()
            .filter(|record| record.pairs.is_empty())
            .count(),
        7
    );
    let dependencies = crate::camo::camo_dependencies(&camos, &options, 45);
    let camo_materials = crate::material::read_used(&catalog, &dependencies).unwrap();
    let mut native_camo_materials = all.clone();
    native_camo_materials.extend(camo_materials.clone());
    let mut camo_library = crate::camo::Library {
        version: 3,
        definitions: camos.clone(),
        options: options.clone(),
        weapons,
        attachments,
        knife_camo: "camo_knife".into(),
        optic_switch: 1.,
        materials: native_camo_materials,
        variants: Vec::new(),
    };
    camo_library.resolve_knife().unwrap();
    camo_library.validate().unwrap();
    assert_eq!(
        serde_json::to_value(&camo_library.definitions["camo_knife"]).unwrap(),
        serde_json::to_value(&camos["defaultcamo"]).unwrap()
    );
    let variant_inputs = crate::material::camo_variants(&mut camo_library).unwrap();
    assert!(!variant_inputs.is_empty());
    let initial = std::collections::BTreeMap::from([(181, [0.; 4]), (185, [0.; 4])]);
    let mut camo_bindings = 0;
    for camo in camos.values() {
        let mut queue = Vec::new();
        crate::camo::queue_camo(camo, &options, 45, 0., 2, &camo_materials, &mut queue).unwrap();
        assert!(queue.len() <= 1024);
        for original in camo_materials.keys() {
            let selected = crate::camo::camo_material(&queue, original, 1);
            if selected == original.as_str() {
                continue;
            }
            let (bound, constants) =
                crate::camo::camo_inputs(&queue, &camo_materials[selected], 1, &initial).unwrap();
            for texture in bound["textures"].as_array().unwrap() {
                catalog
                    .resolve(8, texture["image"].as_str().unwrap())
                    .unwrap();
            }
            assert_eq!(constants[&181], [0.; 4]);
            assert!(constants[&185].iter().all(|value| value.is_finite()));
            camo_bindings += 1;
        }
    }
    assert!(camo_bindings > 0);
    println!(
        "Read {} installed camo definitions, {} material-set-13 records and {} source/replacement dependencies; resolved {camo_bindings} controlled model-index binding plans, not runtime-active descriptor selection",
        camos.len(),
        records.len(),
        dependencies.len()
    );
    let prior = [
        "mc_lit_sm_r0c0s0_986ezzjq",
        "wpc_lit_sm_r0c0n0_80fe30wz",
        "mc_lit_sm_t0c0_90wz6fe2",
        "wpc_lit_sm_t0c0_2w3887e6",
        "wpc_lit_sm_t0c0n0_9ew0jz24",
    ];
    let families = [
        "mc_lit_sm_t0c0n0_9qf6e4qj",
        "wpc_lit_sm_t0c0n0s0_weef63f3",
        "mc_lit_sm_b0c0_fw0jf955",
        "mc_lit_sm_b0c0s0_805w75j2",
        "wpc_lit_sm_b0c0n0_19f3588f",
        "wpc_lit_sm_b0c0_6j30z67q",
        "mc_lit_sm_b0c0n0_67fqj7jf",
        "wpc_lit_sm_r0c0s0_qfqz7850",
        "wpc_lit_sm_r0c0x0_5j63fj97",
        "wpc_lit_sm_b0c0s0_3f3q946z",
        "mc_unlit_replace_3j970129",
        "mc_unlit_add_ww82zw88",
        "mc_unlit_blend_588ewzff",
        "wpc_unlit_add_8ez3wzw3",
        "wpc_unlit_replace_4688792e",
        "mc_lit_sm_r0c0n0s0_ze50j7jz",
        "mc_lit_sm_r0c0n0s0_7j414q0z",
        "mc_lit_sm_r0c0n0s0_4w3wqfe5",
        "mc_lit_sm_r0c0n0x0_53144qq1",
        "mc_lit_sm_r0c0_5137qq99",
        "mc_lit_sm_r0c0d0n0_33ffej1u",
        "mc_lit_sm_r0c0d0n0s0_qwq4q449",
        "mc_lit_sm_t0c0d0n0s0_68eww765",
        "mc_lit_sm_r0c0n0s0o0_3z86zq2z",
        "mc_lit_sm_r0c0n0s0o0_909j993q",
        "mc_lit_sm_b0c0n0s0o0_jf00fe4q",
        "mc_lit_sm_t0c0n0s0_f933q84u",
        "mc_lit_sm_b0c0n0s0_68f25z0u",
        "mc_lit_sm_b0c0s0_q94f27qw",
        "mc_lit_sm_b0c0n0v0_3559e8jj",
        "wpc_lit_sm_b0c0n0v0_ew347fe3",
        "mlv_lit_sm_r0c0n0x0_619829wj",
        "mlv_lit_sm_r0c0_j60qqj0e",
        "mlv_lit_sm_r0c0n0s0_wqqj839w",
        "mlv_lit_sm_r0c0n0_3qf192ew",
        "mlv_lit_sm_t0c0n0s0_9jfj95q5",
        "mlv_lit_sm_b0c0_50077370",
        "mlv_lit_sm_b0c0n0_322zqw73",
        "mlv_lit_sm_b0c0s0_41731e52",
        "mlv_lit_sm_r0c0_475q06w3",
        "wpc_lit_sm_b0c0n0s0v0_efz5w56f",
        "mc_lit_sm_flag_t0c0d0n0s0_w75eqzzj",
        "mc_lit_sm_flag_b0c0s0_6406ez95",
        "mlv_lit_sm_r0c0d0n0_w65788j0",
        "mlv_lit_sm_r0c0n0x0_8f47740f",
        "mc_unlit_multiply_97923qz1",
        "mc_unlitdecalblend_add_fzw1z66f",
        "mc_unlitdecalblend_multiply_34w7q3jw",
        "wpc_unlitdecalblend_add_j26wq580",
        "wpc_unlitdecalblend_multiply_35079164",
        "wpc_sw4_3d_unlit_1layer_8691j77q",
        "mc_sw4_3d_unlit_1layer_nofog_91f00f80",
        "mc_sw4_3d_viewmodel_unlit_j299w86e",
        "mc_sw4_3d_viewmodel_unlit_8216051u",
        "mc_sw4_3d_viewmodel_unlit_60466j88",
        "mc_sw4_3d_unlit_3layer_nofog_falloff_w840w777",
        "mc_sw4_3d_unlit_3layer_nofog_falloff_5z943qe4",
        "mc_sw4_3d_unlit_3layer_nofog_falloff_5z909874",
        "mc_sw4_3d_unlit_3layer_nofog_falloff_zfeather_wq4jwzz6",
        "mlv_sw4_3d_unlit_3layer_nofog_falloff_fjz6ze0u",
        "mc_sw4_3d_model_unlit_cheap_zombie_eyes_jq3e7eqw",
        "mc_sw4_3d_viewmodel_unlit_alpha_85z6j5w5",
        "mc_sw4_3d_unlit_3layer_nofog_qw71w785",
        "mc_sw4_3d_unlit_4layer_nofog_eee768jw",
        "mlv_sw4_3d_unlit_4layer_nofog_2j0ezz67",
    ];
    let selected: std::collections::BTreeMap<_, _> = all
        .iter()
        .filter(|(_, material)| {
            let family = material["technique_set"].as_str().unwrap();
            families.contains(&family) || prior.contains(&family)
        })
        .map(|(key, material)| (key.clone(), material.clone()))
        .collect();
    assert_eq!(selected.len(), 315);
    let required = crate::material::inputs(&selected, &catalog).unwrap();
    assert!(!required.is_empty());
    let names = [
        "mc/mtl_p6_zm_tm_foliage_trunk_splinter",
        "mc/mtl_p6_zm_wood_fence",
        "wpc/decal_trim_metal_grate01",
        "mc/mtl_p6_zm_tm_science_glass",
        "mc/me_glassdirt",
        "mc/mtl_t6_veh_drone_quad_rotor_blades",
        "wpc/zm_tm_decal_damage_crack_06",
        "wpc/zm_tm_stencil_numbers",
        "mc/zm_tm_wall_concrete_blend",
        "wpc/mtl_p6_zm_tm_science_metal",
        "wpc/com_black2",
        "wpc/me_glassdirt",
        "mc/mtl_p6_zm_tm_robot_map",
        "mc/mtl_t6_attach_trinium_sight_green",
        "mc/mat_veh_t6_dlc_zm_drone_water",
        "wpc/pb_decal_wall_fillet_2",
        "wpc/global_color_black_unlit",
        "mc/mtl_t6_wpn_zmb_monkey",
        "mc/mtl_t6_attach_fastmag",
        "mc/mtl_veh_t6_dlc_mkiv_tank_treads",
        "mc/mtl_p6_zm_tm_sandbags",
        "mc/mtl_p6_zm_tm_sandbags_cheap",
        "mc/mtl_p6_wood_plank_rustic01",
        "mc/mtl_p6_wood_plank_rustic01_2x8x108",
        "mc/mtl_p6_zm_tm_packapunch_body",
        "mc/mtl_fx_char_gib_chunks",
        "mc/mtl_t6_attach_gl_ammo",
        "mc/mtl_t5_weapon_camera_spike",
        "mc/mtl_weapon_m14_scope_rail",
        "mc/mtl_veh_t6_dlc_mkiv_tank_catwalk",
        "mc/mtl_t6_wpn_zmb_raygun2_3_world",
        "mc/mtl_c_zom_mech_alpha",
        "mc/mtl_t6_wpn_zmb_monkey_fur",
        "mc/mtl_fxuse_sphere_fireball_cold",
        "mc/mtl_fxuse_sphere_fireball_cold_reveal",
        "mc/zm_tm_foliage_ivy_blend",
        "wpc/grass_dry_brown_1024_blend",
        "wpc/zm_tm_brick02_dmg_blend",
        "mlv/ch_woodwall01",
        "mlv/mtl_p6_zm_tm_crate_ammo_cheap",
        "mlv/mtl_p6_zm_tm_bunker_shelves",
        "mlv/mtl_p6_zm_tm_bunkbed",
        "mlv/mtl_p6_zm_tm_corrugated_wall",
        "mlv/mtl_p6_zm_tm_bunker_arrows",
        "mlv/zm_tm_wall_concrete_blend",
        "mlv/mtl_p6_zm_tm_pump_vent_glassdirt",
        "mlv/mtl_p6_zm_tm_sandbags_cheap",
        "wpc/zm_tm_frost_powder_blend",
        "mc/mtl_fxanim_gp_streamer_red",
        "mc/mtl_fxanim_gp_spiderweb_lrg_sq",
        "mlv/mtl_p6_wood_plank_rustic01",
        "mlv/mtl_p6_zm_tm_sandbags",
        "mc/mtl_fxanim_gp_tarp_tattered_brown",
        "mc/mtl_fxanim_zom_tomb_wall_note",
        "mc/mtl_zm_tm_decal_damage_wall_fillet",
        "mc/mtl_p6_zm_tm_challenge_glow",
        "mc/zm_tm_decal_quaternary",
        "wpc/zm_tm_decal_quaternary_ice",
        "wpc/zm_tm_decal_damage_wall_fillet",
        "wpc/zm_tm_decal_grunge_darkstain_06",
        "wpc/zm_tm_wallbuy_01",
        "mc/mtl_p6_zm_tm_runes",
        "mc/mtl_t6_attach_tritium_grn_glo",
        "mc/mat_veh_t6_dlc_zm_drone_water_vm",
        "mc/mat_veh_t6_dlc_zm_drone_unlit_vm",
        "mc/mtl_t6_attach_optic_acog_crosshair",
        "mc/mtl_p6_zm_tm_elm_spawn_glow",
        "mc/mtl_p6_zm_tm_pap_glow_effect",
        "mc/mtl_p6_zm_tm_soul_box_glow",
        "mc/mtl_p6_zm_tm_generator_115",
        "mc/mtl_p6_zm_tm_generator_115_vat_opaque",
        "mc/mtl_p6_zm_tm_generator_115_vat",
        "mc/mtl_p6_zm_tm_elm_plinth_glow_blue",
        "mc/mtl_p6_zm_tm_elm_plinth_glow_yellow",
        "mlv/mtl_p6_zm_tm_elm_spawn_glow",
        "mc/mtl_c_zom_mech_head_unlit",
        "mc/mtl_t6_attach_optic_acog_shade",
        "mc/mtl_t6_attach_zmb_mc96_reticle",
        "mc/mtl_p6_zm_tm_generator_115_vat_spiral",
        "mc/mtl_p6_zm_tm_rock_hex_glyph",
        "mc/mtl_p6_zm_tm_rock_hex_glyph_2",
        "mc/mtl_p6_zm_tm_rock_hex_glyph_3",
        "mc/mtl_zm_tm_elm_portal_vortex",
        "mlv/mtl_zm_tm_elm_portal_vortex",
    ];
    let mut samples: std::collections::BTreeMap<String, _> = names
        .iter()
        .map(|&name| (name.into(), selected[name].clone()))
        .collect();
    for (index, name) in [
        "mc/mtl_p6_zm_tm_elm_spawn_glow",
        "mc/mtl_p6_zm_tm_generator_115_vat_opaque",
        "mc/mtl_p6_zm_tm_generator_115_vat",
    ]
    .into_iter()
    .enumerate()
    {
        for (state, vector) in [[0., 0., 0., 0.], [0.25, 0.5, 0.75, 0.], [-1., 2., 0.5, 0.]]
            .into_iter()
            .enumerate()
        {
            let mut material = selected[name].clone();
            material["code_constants"]["169"] = serde_json::json!(vector);
            samples.insert(format!("explicit_falloff_{index}_{state}"), material);
        }
    }
    for (index, luminance) in [0., 0.25, 1.].into_iter().enumerate() {
        let mut material = selected["mc/mtl_c_zom_mech_head_unlit"].clone();
        material["code_constants"]["171"] = serde_json::json!([0., luminance, 0., 0.]);
        samples.insert(format!("explicit_eye_luminance_{index}"), material);
    }
    let (bound, constants) = crate::camo::camo_inputs(
        &[],
        &selected["mc/mtl_t6_attach_fastmag"],
        1,
        &std::collections::BTreeMap::from([(181, [0.25, -0.5, 3., 1.])]),
    )
    .unwrap();
    assert_eq!(constants[&181], [0.25, -0.5, 3., 1.]);
    samples.insert("explicit_camo_state".into(), bound);
    for (key, name) in [
        ("explicit_occlusion_camo_state", "mc/mtl_t6_attach_gl_ammo"),
        (
            "explicit_color_matrix_camo_state",
            "mc/mtl_weapon_m14_scope_rail",
        ),
    ] {
        let (bound, _) = crate::camo::camo_inputs(
            &[],
            &selected[name],
            1,
            &std::collections::BTreeMap::from([(181, [0.25, -0.5, 3., 1.])]),
        )
        .unwrap();
        samples.insert(key.into(), bound);
    }
    let mut baseline_keys = Vec::new();
    for name in [
        "mc/mtl_t6_attach_fastmag",
        "mc/mtl_t6_attach_gl_ammo",
        "mc/mtl_weapon_m14_scope_rail",
    ] {
        let (key, inputs) = variant_inputs
            .iter()
            .find(|(_, inputs)| {
                inputs["name"] == name
                    && inputs["code_constants"]["181"] == serde_json::json!([0., 0., 0., 0.])
                    && inputs["code_constants"]["185"].is_null()
            })
            .unwrap();
        samples.insert(key.clone(), inputs.clone());
        baseline_keys.push(key.clone());
    }
    let special_families = [
        "mc_sw4_3d_cod7_tile_blend_spec_8w40z341",
        "mlv_sw4_3d_cod7_tile_blend_spec_w8wezqe6",
        "mc_sw4_3d_cod7_tile_blend_q2q2zej6",
        "mlv_sw4_3d_cod7_tile_blend_26zq1w22",
        "mc_sw4_3d_water_on_surface_reveal_77320z68",
        "mc_sw4_3d_phong_emissive_q0751958",
        "mc_sw4_3d_phong_emissive_alcatraz_90892wq5",
        "mc_sw4_3d_phong_emissive_alcatraz_8q551153",
        "mlv_sw4_3d_phong_emissive_alcatraz_w1q1w09e",
        "mc_sw4_3d_phong_simple_emissive_flicker_9j5f25q3",
        "mc_sw4_3d_phong_emissive_fritz_76q5wqjw",
        "mc_sw4_3d_phong_emissive_fritz_61728967",
        "mc_sw4_3d_phong_emissive_script_multi_72q2fj2u",
        "mc_sw4_3d_phong_emissive_script_multi_j05zz52w",
        "mc_sw4_3d_emissive_tomb_qww536q2",
    ];
    let special_all: std::collections::BTreeMap<_, _> = all
        .iter()
        .filter(|(_, material)| {
            special_families.contains(&material["technique_set"].as_str().unwrap())
        })
        .map(|(name, material)| (name.clone(), material.clone()))
        .collect();
    crate::material::inputs(&special_all, &catalog).unwrap();
    let special_checked = special_all.len();
    assert_eq!(special_checked, 67);
    let special_names = [
        "mc/mtl_p6_zm_tm_chamber_btm_piece",
        "mc/mtl_p6_zm_tm_incense_burner",
        "mc/mtl_p6_zm_tm_elm_plinth_top",
        "mc/mtl_p6_zm_tm_templar_statue_head_1",
        "mc/mtl_p6_rocks_cluster_03b_nospec",
        "mlv/mtl_p6_zm_tm_concrete_pillar_cap",
        "mlv/mtl_p6_zm_tm_templar_statue_top",
        "mc/mtl_p6_zm_tm_note_rock_01_anim",
        "mc/mtl_c_zom_german_soldier_upper1",
        "mc/mtl_p6_zm_al_power_station_panels",
        "mc/mtl_p6_zm_lighting_fixture_on",
        "mc/mtl_p6_zm_tm_robot_plaque",
        "mc/mtl_p6_zm_tm_dig_mound_blood",
        "mc/mtl_p6_zm_tm_gas_lamp_glass_on",
        "mlv/mtl_p6_zm_tm_wind_rings",
        "mc/mtl_p6_zm_tm_shovel",
        "mc/mtl_p6_zm_tm_challenge_box",
        "mc/mtl_p6_zm_al_vending_three_gun_logo_on",
        "mc/mtl_p6_zm_al_vending_three_gun_on",
        "mc/mtl_p6_zm_tm_monolith_ring",
        "mc/mtl_p6_zm_vending_diesel_magic_acc",
        "mc/mtl_p6_zm_vending_diesel_magic_ball",
        "mc/mtl_p6_zm_vending_diesel_magic_logo",
        "mc/mtl_p6_zm_tm_chamber_mid_piece_lt",
        "mc/mtl_p6_zm_tm_soul_box",
    ];
    let special: std::collections::BTreeMap<_, _> = special_names
        .into_iter()
        .map(|name| (name.to_owned(), all[name].clone()))
        .collect();
    let mut sample_images = crate::material::inputs(&samples, &catalog).unwrap();
    sample_images.extend(crate::material::inputs(&special, &catalog).unwrap());
    let mut pack = crate::pack::Pack::default();
    let (rows, images) =
        crate::images::convert(&install, &catalog, &mut pack, Some(&sample_images)).unwrap();
    let mut shared = 0;
    for row in rows {
        let a = &pack.entries[row["uris"]["linear"].as_str().unwrap()];
        let b = &pack.entries[row["uris"]["srgb"].as_str().unwrap()];
        if std::sync::Arc::ptr_eq(a, b) {
            shared += 1;
        }
        crate::texture::validate(a).unwrap();
        crate::texture::validate(b).unwrap();
    }
    assert!(shared > 0);
    let converted = crate::material::convert_known(&samples, &images, &mut pack).unwrap();
    assert_eq!(converted.len(), names.len() + 18);
    let special_pending =
        crate::bake::check_special_response(&catalog, &special, &images, &mut pack).unwrap();
    assert_eq!(special_pending.len(), 18);
    println!(
        "Checked {special_checked} special materials across fifteen families and {} representative materials plus controlled water/flicker states; {} source/runtime gaps retained",
        special.len(),
        special_pending.len()
    );
    for material in converted.values() {
        for channel in material["extras"]["surface"]["channels"]
            .as_array()
            .unwrap()
        {
            crate::texture::validate(pack.get(channel["uri"].as_str().unwrap()).unwrap()).unwrap();
        }
    }
    assert_eq!(
        converted
            .values()
            .filter(|material| material["extras"]["surface"]["presentation"]["alpha"] == "mask")
            .count(),
        9
    );
    assert_eq!(
        converted
            .values()
            .filter(
                |material| material["extras"]["surface"]["presentation"]["alpha"]
                    == "premultiplied"
            )
            .count(),
        20
    );
    assert_eq!(
        converted
            .values()
            .filter(|material| material["extras"]["surface"]["standard"]["unlit"] == true)
            .count(),
        47
    );
    let pending = crate::material::missing(&samples, &converted);
    assert_eq!(pending.len(), 82);
    assert_eq!(
        pending
            .iter()
            .filter(|reason| reason.contains("Native ONE/ONE addition"))
            .count(),
        2
    );
    let flag_materials: std::collections::BTreeSet<_> = names
        .iter()
        .copied()
        .filter(|name| converted[*name]["extras"]["surface"]["source_flag_motion"].is_object())
        .collect();
    let mut flag_models = std::collections::BTreeSet::new();
    let mut flag_primitives = 0;
    for (name, zone, root) in catalog.definitions(5) {
        let streams = &catalog.zones[zone].1.streams;
        let materials =
            crate::fastfile::Address::decode(streams.u32(root.add(36).unwrap()).unwrap()).unwrap();
        let relevant = (0..streams.u8(root.add(6).unwrap()).unwrap() as usize).any(|index| {
            let material = catalog
                .pointer(zone, materials.add(index * 4).unwrap(), 6)
                .unwrap()
                .0;
            flag_materials.contains(material.as_str())
        });
        if !relevant {
            continue;
        }
        flag_models.insert(name.to_owned());
        for surface in crate::geometry::model(&catalog, name, zone, root)
            .unwrap()
            .iter()
            .flatten()
            .filter(|surface| flag_materials.contains(surface.material.as_str()))
        {
            let source = surface.select_triangles(&[0]).unwrap();
            let mut scene = crate::gltf::Scene::new(name);
            let indices = scene.materials([&source], &converted).unwrap();
            let mesh = scene.mesh(name, [&source], &indices, false).unwrap();
            scene.node(serde_json::json!({"name":name,"mesh":mesh}));
            let uri = format!("prepared/flag-check/{flag_primitives}.gltf");
            scene.write(&uri, &mut pack).unwrap();
            let doc: serde_json::Value = serde_json::from_slice(pack.get(&uri).unwrap()).unwrap();
            for field in ["_ORIGINS_NORMAL", "_ORIGINS_TANGENT"] {
                let index = doc["meshes"][0]["primitives"][0]["attributes"][field]
                    .as_u64()
                    .unwrap() as usize;
                assert_eq!(doc["accessors"][index]["type"], "VEC3");
                assert_eq!(doc["accessors"][index]["count"], 3);
            }
            let index = doc["meshes"][0]["primitives"][0]["attributes"]["_ORIGINS_PACKED_BASIS"]
                .as_u64()
                .unwrap() as usize;
            let accessor = &doc["accessors"][index];
            assert_eq!(accessor["type"], "VEC2");
            assert_eq!(accessor["componentType"], 5125);
            assert_eq!(accessor["count"], 3);
            let view = accessor["bufferView"].as_u64().unwrap() as usize;
            let start = doc["bufferViews"][view]["byteOffset"].as_u64().unwrap() as usize;
            let bytes = pack.get(&uri.replace(".gltf", ".bin")).unwrap();
            let actual: Vec<[u32; 2]> = (0..3)
                .map(|vertex| {
                    [
                        crate::core::u32_at(bytes, start + vertex * 8),
                        crate::core::u32_at(bytes, start + vertex * 8 + 4),
                    ]
                })
                .collect();
            assert_eq!(actual, source.packed_basis);
            flag_primitives += 1;
        }
    }
    assert!(flag_primitives > 0);
    let mut flag_instances = 0;
    for (_, zone, root) in catalog.definitions(17) {
        let streams = &catalog.zones[zone].1.streams;
        let instances =
            crate::fastfile::Address::decode(streams.u32(root.add(876).unwrap()).unwrap()).unwrap();
        for index in 0..streams.u32(root.add(784).unwrap()).unwrap() as usize {
            let name = catalog
                .pointer(zone, instances.add(index * 152 + 56).unwrap(), 5)
                .unwrap()
                .0;
            if flag_models.contains(&name) {
                flag_instances += 1;
            }
        }
    }
    println!(
        "Retained exact raw flag bases in {flag_primitives} bounded representative primitives across {} native models; {flag_instances} installed static instances use the rigid scene path, while standalone model scenes remain skinned",
        flag_models.len()
    );
    let camo = &converted["explicit_camo_state"]["extras"]["surface"];
    assert_eq!(
        camo["source_camo"]["parameters"],
        serde_json::json!([0.25, -0.5, 3., 1.])
    );
    for role in ["source_color", "source_camo"] {
        crate::texture::validate(
            pack.get(camo["source_camo"]["inputs"][role]["uri"].as_str().unwrap())
                .unwrap(),
        )
        .unwrap();
    }
    assert_ne!(
        camo["channels"],
        converted["mc/mtl_t6_attach_fastmag"]["extras"]["surface"]["channels"]
    );
    assert!(
        converted["mc/mtl_t6_attach_fastmag"]["extras"]["surface"]["source_camo"]["parameters"]
            .is_null()
    );
    assert_eq!(
        converted["mc/mtl_p6_zm_tm_sandbags"]["extras"]["surface"]["presentation"]["alpha"],
        "opaque"
    );
    assert_eq!(
        converted["mc/mtl_p6_zm_tm_sandbags_cheap"]["extras"]["surface"]["standard"]["reflectance"],
        0.
    );
    for name in [
        "mc/mtl_p6_wood_plank_rustic01",
        "mc/mtl_p6_wood_plank_rustic01_2x8x108",
        "mc/mtl_p6_zm_tm_packapunch_body",
        "mc/mtl_fx_char_gib_chunks",
    ] {
        let detail = &converted[name]["extras"]["surface"]["source_detail_normal"];
        assert_eq!(detail["operation"], "scaled_slope_add_before_normalization");
        assert_eq!(detail["input"]["transfer"], "linear");
        crate::texture::validate(pack.get(detail["input"]["uri"].as_str().unwrap()).unwrap())
            .unwrap();
    }
    for name in [
        "explicit_occlusion_camo_state",
        "explicit_color_matrix_camo_state",
        "mc/mtl_veh_t6_dlc_mkiv_tank_catwalk",
    ] {
        let surface = &converted[name]["extras"]["surface"];
        let occlusion = &surface["source_occlusion"];
        assert_eq!(occlusion["indirect_specular"], "green");
        assert_eq!(occlusion["direct_specular"], "red");
        crate::texture::validate(
            pack.get(occlusion["input"]["uri"].as_str().unwrap())
                .unwrap(),
        )
        .unwrap();
    }
    assert_eq!(
        converted["explicit_occlusion_camo_state"]["extras"]["surface"]["source_camo"]["color_bias"],
        "occlusion_blue"
    );
    assert_eq!(
        converted["explicit_color_matrix_camo_state"]["extras"]["surface"]["source_color_matrix"]["rows"],
        serde_json::json!([114, 115, 116])
    );
    assert_eq!(
        converted["mc/mtl_veh_t6_dlc_mkiv_tank_catwalk"]["extras"]["surface"]["source_fragment_alpha"],
        "sqrt_texture_vertex_coverage"
    );
    for name in [
        "mc/mtl_fxuse_sphere_fireball_cold",
        "mc/mtl_fxuse_sphere_fireball_cold_reveal",
        "mc/zm_tm_foliage_ivy_blend",
        "wpc/grass_dry_brown_1024_blend",
        "wpc/zm_tm_brick02_dmg_blend",
    ] {
        let surface = &converted[name]["extras"]["surface"];
        let reveal = &surface["source_reveal"];
        assert_eq!(reveal["vertex_channel"], "alpha");
        assert_eq!(surface["source_fragment_alpha"], "sqrt_reveal_coverage");
        assert!(reveal["parameters"][0].as_f64().unwrap() > 0.);
        crate::texture::validate(pack.get(reveal["input"]["uri"].as_str().unwrap()).unwrap())
            .unwrap();
    }
    assert_eq!(
        converted["mc/mtl_fxuse_sphere_fireball_cold"]["extras"]["surface"]["source_reveal"]["parameters"]
            [1],
        0.
    );
    assert_eq!(
        converted["mc/zm_tm_foliage_ivy_blend"]["extras"]["surface"]["source_reveal"]["parameters"]
            [1],
        0.5
    );
    assert_eq!(
        converted["mc/mtl_t6_wpn_zmb_raygun2_3_world"]["extras"]["surface"]["source_framebuffer_alpha"],
        "texture_vertex_coverage_after_cutoff"
    );
    for name in ["mc/mtl_c_zom_mech_alpha", "mc/mtl_t6_wpn_zmb_monkey_fur"] {
        let surface = &converted[name]["extras"]["surface"];
        assert_eq!(
            surface["source_color_matrix"]["stage"],
            "coverage_scaled_linear_lighting_before_fog"
        );
        assert_eq!(
            surface["source_fragment_alpha"],
            "sqrt_texture_vertex_coverage"
        );
    }
    for name in names.iter().filter(|name| {
        selected[**name]["technique_set"]
            .as_str()
            .unwrap()
            .starts_with("mlv_lit_sm_")
    }) {
        let surface = &converted[*name]["extras"]["surface"];
        assert_eq!(surface["source_vertex_lighting"]["source"], 5);
        assert_eq!(
            surface["source_vertex_lighting"]["alpha"],
            "sun_shadow_fallback"
        );
        assert!(surface["pending_runtime"].as_str().is_some());
    }
    let falloff = &converted["mlv/mtl_p6_zm_tm_elm_spawn_glow"]["extras"]["surface"];
    assert_eq!(
        falloff["source_vertex_color_transfer"],
        "unused_primary_color"
    );
    assert_eq!(
        falloff["source_reflection"]["secondary_vertex_alpha_red_bias"],
        serde_json::json!(0.00001)
    );
    assert!(
        falloff["pending_runtime"]
            .as_str()
            .unwrap()
            .contains("reflection/SH")
    );
    assert_eq!(
        converted["mlv/mtl_p6_zm_tm_crate_ammo_cheap"]["extras"]["surface"]["standard"]["reflectance"],
        0.
    );
    assert_eq!(
        converted["mlv/ch_woodwall01"]["extras"]["surface"]["native_specular"]["gloss_channel"],
        "alpha"
    );
    let eye_uri =
        &converted["mc/mtl_c_zom_mech_head_unlit"]["extras"]["surface"]["channels"][0]["uri"];
    for (key, luminance) in [
        ("mc/mtl_c_zom_mech_head_unlit", 0_f32),
        ("explicit_eye_luminance_0", 0.),
        ("explicit_eye_luminance_1", 0.25),
        ("explicit_eye_luminance_2", 1.),
    ] {
        let surface = &converted[key]["extras"]["surface"];
        assert_eq!(
            surface["source_eye_luminance"]["lod"],
            serde_json::json!(0.)
        );
        assert_eq!(&surface["channels"][0]["uri"], eye_uri);
        assert_eq!(
            surface["standard"]["base_color"],
            serde_json::json!([
                luminance * luminance,
                luminance * luminance,
                luminance * luminance,
                1.
            ])
        );
        let uri = surface["channels"][0]["uri"].as_str().unwrap();
        assert_eq!(crate::core::u32_at(pack.get(uri).unwrap(), 40), 1);
    }
    let camo_pending =
        crate::material::finish_camos(&mut camo_library, &converted, &mut pack).unwrap();
    assert_eq!(camo_pending.len() + 1, camo_library.variants.len());
    let decoded: crate::camo::Library =
        serde_json::from_slice(pack.get("prepared/camo.json").unwrap()).unwrap();
    decoded.validate().unwrap();
    assert_eq!(decoded.variants.len(), variant_inputs.len());
    assert_eq!(
        decoded.prepared(&samples[&baseline_keys[0]]).unwrap()["identity"],
        baseline_keys[0]
    );
    assert!(decoded.prepared(&samples[&baseline_keys[1]]).is_err());
    assert_eq!(
        decoded
            .bound_inputs(&[], "mc/mtl_t6_attach_fastmag", 1)
            .unwrap()["code_constants"]["181"],
        serde_json::json!([0., 0., 0., 0.])
    );
    let mut check_pack = crate::pack::Pack {
        entries: pack.entries.clone(),
    };
    check_pack
        .entries
        .insert("game.json".into(), std::sync::Arc::new(b"{}".to_vec()));
    check_pack.entries.insert(
        "prepared/runtime.json".into(),
        std::sync::Arc::new(
            serde_json::to_vec(&serde_json::json!({
        "schema_version":1,"camo_uri":"prepared/camo.json"}))
            .unwrap(),
        ),
    );
    let error = crate::validate_pack(&check_pack).unwrap_err().to_string();
    assert!(error.contains("Native camo state") && error.contains("is incomplete"));
    let mut prepared_only = decoded.clone();
    prepared_only
        .variants
        .retain(|variant| variant.surface.is_some());
    check_pack.entries.insert(
        "prepared/camo.json".into(),
        std::sync::Arc::new(serde_json::to_vec(&prepared_only).unwrap()),
    );
    assert!(
        crate::validate_pack(&check_pack)
            .unwrap_err()
            .to_string()
            .contains("Unresolved prepared material")
    );
    check_pack
        .entries
        .retain(|uri, _| !uri.starts_with("prepared/flag-check/"));
    crate::validate_pack(&check_pack).unwrap();
    assert_eq!(
        converted["mlv/mtl_p6_zm_tm_sandbags_cheap"]["extras"]["surface"]["source_vertex_alpha"],
        "indirect_diffuse"
    );
    assert_eq!(
        converted["mlv/mtl_p6_zm_tm_sandbags_cheap"]["extras"]["surface"]["presentation"]["alpha"],
        "opaque"
    );
    println!(
        "Checked exact programs, routes, active inputs and states for {} materials in seventy families; {} image definitions resolve; converted {} representative materials/states; reused {shared} identical image encodings; {} dispatch gaps and eighty-two sampled runtime gaps remain; serialized {} native camo variants with one prepared baseline and {} explicit pending variants",
        selected.len(),
        required.len(),
        converted.len(),
        gaps.len(),
        decoded.variants.len(),
        camo_pending.len()
    );
}

#[test]
#[ignore = "requires the authorized Steam installation and a hardware GPU"]
fn installed_layered_response_batch() {
    let install = super::installation().unwrap();
    let zones = super::read_zones(&install).unwrap();
    let catalog = crate::catalog::Catalog::new(&zones);
    let used = crate::geometry::material_names(&catalog, None).unwrap();
    let all = crate::material::read_used(&catalog, &used).unwrap();
    assert_eq!(all.len(), 1034);
    let unsupported = crate::material::unimplemented(&all).len();
    let families = [
        "lit_sm_r0c0n0s0_b1c1n1",
        "lit_sm_r0c0n0s0_b1c1n1s1v1",
        "lit_sm_r0c0n0x0_b1c1n1s1",
        "lit_sm_r0c0n0x0_b1c1n1s1v1_m2c2",
        "lit_sm_r0c0s0_b1c1",
        "lit_sm_r0c0n0x0_a1c1_m2c2",
        "lit_sm_r0c0n0x0_b1c1n1_a2c2",
        "lit_sm_r0c0n0x0_b1c1n1s1v1_a2c2",
        "lit_sm_r0c0n0x0_b1c1_b2c2n2",
        "lit_sm_r0c0n0x0_b1c1n1_b2c2n2",
        "lit_sm_r0c0n0x0_b1c1n1s1v1_b2c2n2s2v2",
        "lit_sm_r0c0n0s0_b1c1n1_b2c2n2s2v2",
        "lit_sm_r0c0n0s0_b1c1n1s1v1_b2c2n2s2v2",
        "lit_sm_r0c0n0x0_b1c1_b2c2n2s2v2",
        "lit_sm_r0c0n0x0_b1c1n1s1v1_b2c2n2v2",
        "lit_sm_r0c0n0x0_b1c1n1_b2c2n2s2v2",
        "lit_sm_r0c0n0x0_b1c1_b2c2n2s2",
        "lit_sm_r0c0n0x0_b1c1n1s1v1_a2c2_m3c3",
        "lit_sm_r0c0n0x0_t1c1n1",
    ];
    let mut selected = std::collections::BTreeMap::new();
    let mut seen = std::collections::BTreeSet::new();
    let mut checked = 0;
    for (name, material) in &all {
        let family = material["technique_set"].as_str().unwrap();
        if !families.contains(&family) {
            continue;
        }
        crate::bake::validate_native(material).unwrap_or_else(|error| panic!("{name}: {error}"));
        for binding in material["textures"].as_array().unwrap() {
            catalog
                .resolve(8, binding["image"].as_str().unwrap())
                .unwrap();
        }
        checked += 1;
        let clamped = material["textures"]
            .as_array()
            .unwrap()
            .iter()
            .any(|binding| binding["hash"] == 3054311507_u32 && binding["sampler"] == 74);
        if seen.insert(family) || clamped {
            selected.insert(name.clone(), material.clone());
        }
    }
    assert_eq!(checked, 58);
    assert_eq!(selected.len(), 20);
    let ordinary = std::collections::BTreeMap::from([(
        "mc/dec_glass02".into(),
        all["mc/dec_glass02"].clone(),
    )]);
    let mut required = crate::material::inputs(&ordinary, &catalog).unwrap();
    required.extend(
        selected
            .values()
            .flat_map(|material| material["textures"].as_array().unwrap())
            .map(|binding| binding["image"].as_str().unwrap().to_ascii_lowercase()),
    );
    let mut pack = crate::pack::Pack::default();
    let (_, images) =
        crate::images::convert(&install, &catalog, &mut pack, Some(&required)).unwrap();
    let converted = crate::material::convert_known(&ordinary, &images, &mut pack).unwrap();
    assert_eq!(converted.len(), 1);
    let ordinary_pending = crate::material::missing(&ordinary, &converted);
    assert_eq!(ordinary_pending.len(), 1);
    assert!(ordinary_pending[0].contains("sqrt coverage"));
    let pending =
        crate::bake::check_response_geometry(&catalog, &selected, &images, &mut pack).unwrap();
    assert_eq!(pending.len(), 1);
    assert!(pending[0].contains("opaque layer selection"));
    println!(
        "Checked all {checked} layered materials in nineteen families and one ordinary glass material; representative bakes for {} layered materials including wrap, clamp-V, nine-image, UV3 and layer-selection edges; {unsupported} dispatch gaps and two sampled framebuffer-composition runtime gaps remain",
        selected.len()
    );
}

fn installed_material_families(families: &[&str], expected: usize) {
    let install = super::installation().unwrap();
    let zones = super::read_zones(&install).unwrap();
    let catalog = crate::catalog::Catalog::new(&zones);
    let used = crate::geometry::validate(&catalog, None).unwrap();
    let mut materials = crate::material::read_used(&catalog, &used).unwrap();
    materials.retain(|_, material| {
        material["technique_set"]
            .as_str()
            .is_some_and(|family| families.contains(&family))
    });
    assert_eq!(materials.len(), expected);
    let required = materials
        .values()
        .flat_map(|material| material["textures"].as_array().unwrap())
        .map(|binding| binding["image"].as_str().unwrap().to_ascii_lowercase())
        .collect();
    let mut pack = crate::pack::Pack::default();
    let (_, images) =
        crate::images::convert(&install, &catalog, &mut pack, Some(&required)).unwrap();
    let mut converted = std::collections::BTreeMap::new();
    let baked =
        crate::bake::world(&catalog, &materials, &images, &mut converted, &mut pack).unwrap();
    assert!(crate::material::missing(&materials, &converted).is_empty());
    assert!(!baked.is_empty());
    for material in converted.values() {
        for channel in material["extras"]["surface"]["channels"]
            .as_array()
            .unwrap()
        {
            crate::texture::validate(pack.get(channel["uri"].as_str().unwrap()).unwrap()).unwrap();
        }
    }
    let mut scene = crate::gltf::Scene::new("multiply");
    let indices = scene
        .materials(baked.values().flatten(), &converted)
        .unwrap();
    let mesh = scene
        .mesh("multiply", baked.values().flatten(), &indices, false)
        .unwrap();
    scene.node(serde_json::json!({"name":"multiply","mesh":mesh}));
    scene.write("prepared/material.gltf", &mut pack).unwrap();
    println!(
        "Converted {} installed materials, {} surfaces, {} triangles, {} packed entries; exported glTF bindings and KTX2 mip layouts resolved",
        materials.len(),
        baked.len(),
        baked
            .values()
            .flatten()
            .map(|surface| surface.indices.len() / 3)
            .sum::<usize>(),
        pack.entries.len()
    );
}

#[test]
#[ignore = "requires the authorized Steam installation"]
fn installed_geometry_bindings() {
    let zones = super::read_zones(&super::installation().unwrap()).unwrap();
    let catalog = crate::catalog::Catalog::new(&zones);
    let used = crate::geometry::validate(&catalog, None).unwrap();
    let materials = crate::material::read_used(&catalog, &used).unwrap();
    let gaps = crate::material::unimplemented(&materials);
    println!(
        "Installed material dispatch: {} unimplemented families",
        gaps.len()
    );
    for gap in &gaps {
        println!("Required material conversion: {gap}");
    }
    let mut mixed_frames = 0;
    for (_, zone, root) in catalog.definitions(17) {
        for surface in crate::geometry::world(&catalog, zone, root).unwrap() {
            if !crate::bake::supported(
                materials[&surface.material.to_ascii_lowercase()]["technique_set"]
                    .as_str()
                    .unwrap(),
            ) {
                continue;
            }
            for triangle in surface.indices.chunks_exact(3) {
                let signs: Vec<_> = triangle
                    .iter()
                    .map(|&index| surface.tangents[index as usize][3])
                    .collect();
                assert!(
                    signs.iter().all(|&sign| matches!(sign, -1. | 1.)),
                    "Invalid source handedness in {}",
                    surface.name
                );
                if signs[1] != signs[0] || signs[2] != signs[0] {
                    mixed_frames += 1;
                }
            }
        }
    }
    println!(
        "Validated source handedness for implemented layered materials: {mixed_frames} mixed-frame triangles"
    );
    let mut bones = 0;
    let mut models = 0;
    for (name, zone, root) in catalog.definitions(5) {
        let joints = crate::geometry::skeleton(&catalog, name, zone, root).unwrap();
        let mut scene = crate::gltf::Scene::new(name);
        let owner = scene.node(serde_json::json!({"name":name}));
        scene
            .skin(owner, &joints)
            .unwrap_or_else(|error| panic!("Model {name}: {error}"));
        bones += joints.len();
        models += 1;
    }
    println!("Validated glTF TRS bind hierarchies: {models} models, {bones} joints");
}
#[test]
#[ignore = "requires the authorized Steam installation"]
fn installed_initial_required_material_profiles() {
    let zones = super::read_zones(&super::installation().unwrap()).unwrap();
    let catalog = crate::catalog::Catalog::new(&zones);
    for name in [
        "mc/mtl_fx_decal_character_blood",
        "mc/mtl_c_zom_tomb_crusader_body_3",
        "mc/mtl_c_zom_tomb_crusader_skirt_3",
        "mc/mtl_c_gen_gore_alpha",
        "mc/mtl_c_gen_limb_gore",
        "mc/mtl_c_gen_limb_gore_alpha",
        "mc/mtl_fxuse_disc_electric",
    ] {
        let material = crate::material::read(&catalog, name).unwrap();
        println!(
            "Required material {name} family {}, states {}, entries {}",
            material["technique_set"], material["state_bits"], material["state_entries"]
        );
        assert!(
            crate::material::selected_technique(material["technique_set"].as_str().unwrap())
                .is_some()
        );
        let inputs = crate::material::inputs(
            &std::collections::BTreeMap::from([(name.to_owned(), material.clone())]),
            &catalog,
        )
        .unwrap_or_else(|error| panic!("Required material {name}: {error}"));
        assert!(!inputs.is_empty());
        assert!(!inputs.contains("c_gen_arm_rim_mask_c"));
        println!(
            "Validated required material {name}: {} active images",
            inputs.len()
        );
    }
}

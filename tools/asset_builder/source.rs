use super::core::*;
use std::path::{Path, PathBuf};
use crate::{convert, fastfile, native, pack};

pub fn installation() -> Result<PathBuf> {
    Ok(PathBuf::from(std::env::var_os("ProgramFiles(x86)").ok_or("ProgramFiles(x86) is unavailable")?)
        .join("Steam/steamapps/common/Call of Duty Black Ops II"))
}

fn read_zones(install: &Path) -> Result<Vec<(String,native::Zone)>> {
    let authorized = installation()?.canonicalize()?;
    if install.canonicalize()? != authorized {
        return Err("Assets must be read directly from the authorized Steam installation".into());
    }
    let mut zones = Vec::new();
    for (language, name) in [
        ("all", "code_pre_gfx_zm"), ("all", "code_post_gfx_zm"),
        ("english", "en_code_post_gfx_zm"), ("all", "common_zm"),
        ("english", "en_common_zm"), ("all", "zm_tomb"), ("english", "en_zm_tomb"),
        ("all", "patch_zm"), ("english", "en_patch_zm"), ("all", "zm_tomb_patch"),
    ] {
        let path = authorized.join("zone").join(language).join(format!("{name}.ff"));
        let bytes = std::fs::read(&path).map_err(|error| format!("Required original zone {}: {error}", path.display()))?;
        let file = fastfile::decode(&bytes, name)?;
        let size = file.bytes.len();
        let blocks = file.compressed_blocks;
        println!("Decoded {name}: {blocks} compressed blocks, {size} bytes");
        let zone = native::Zone::load(file).map_err(|error| format!("Zone {name}: {error}"))?;
        println!("Traversed {name}: {} registered native assets", zone.assets.len());
        zones.push((name.to_owned(), zone));
    }
    Ok(zones)
}

pub fn build(install: &Path, output: &Path, preview: Option<&str>) -> Result<()> {
    let authorized = installation()?.canonicalize()?;
    let zones = read_zones(install)?;
    let catalog = convert::Catalog::new(&zones);
    let references: std::collections::BTreeSet<_> = zones.iter().flat_map(|(_, zone)| zone.assets.iter())
        .filter(|asset| asset.reference).map(|asset| (asset.kind, asset.name.as_str())).collect();
    for (kind, name) in &references { catalog.resolve(*kind, name)?; }
    println!("Resolved {} distinct native references", references.len());
    let entities = crate::entities::read(&catalog)?;
    crate::scenes::validate_placements(&catalog, &entities)?;
    let used_materials = crate::geometry::validate(&catalog)?;
    let mut materials = crate::material::read_used(&catalog, &used_materials)?;
    let research = PathBuf::from(std::env::var_os("USERPROFILE").ok_or("USERPROFILE is unavailable")?)
        .join("Documents/T6ZMResearch/t6zm-master");
    if !research.is_dir() {
        return Err("The authorized t6zm-master research workspace is missing".into());
    }
    if let Some(name) = preview {
        let key = name.to_ascii_lowercase();
        if let Some((model_name, zone, root)) = catalog.definitions(5).find(|(model, _, _)| model.eq_ignore_ascii_case(name)) {
            let lods = crate::geometry::model(&catalog, model_name, zone, root)?;
            let surfaces = lods.first().ok_or("Native model has no LODs")?;
            let selected = surfaces.iter().map(|surface| {
                let key = surface.material.to_ascii_lowercase();
                materials.get(&key).cloned().map(|material| (key, material))
                    .ok_or_else(|| format!("Native model material is missing: {}", surface.material).into())
            }).collect::<Result<std::collections::BTreeMap<_, _>>>()?;
            let required = selected.values().flat_map(|material| material["textures"].as_array().into_iter().flatten())
                .map(|binding| binding["image"].as_str().map(str::to_ascii_lowercase)
                    .ok_or_else(|| "Native model image name is missing".into())).collect::<Result<std::collections::BTreeSet<_>>>()?;
            let mut pack = pack::Pack::default();
            let (images, _) = convert::images(&authorized, &catalog, &mut pack, Some(&required))?;
            let converted = selected.iter().map(|(key, material)| {
                let mut channels = Vec::new();
                for binding in material["textures"].as_array().into_iter().flatten() {
                    let name = binding["image"].as_str().ok_or("Native model image name is missing")?;
                    let image = images.iter().find(|image| image["name"].as_str().is_some_and(|value| value.eq_ignore_ascii_case(name)))
                        .ok_or_else(|| format!("Native model image is missing: {name}"))?;
                    let mut channel = binding.clone();
                    channel["uri"] = image["uris"]["linear"].clone();
                    channels.push(channel);
                }
                Ok((key.clone(), serde_json::json!({"name":key,"extras":{"surface":{
                    "native":material,"source_vertex_color_transfer":"gamma2","channels":channels}}})))
            }).collect::<Result<std::collections::BTreeMap<_, _>>>()?;
            let mut scene = crate::gltf::Scene::new(model_name);
            let indices = scene.materials(surfaces, &converted)?;
            let mesh = scene.mesh(model_name, surfaces, &indices, true)?;
            let owner = scene.node(serde_json::json!({"name":model_name,"mesh":mesh}));
            scene.skin(owner,&crate::geometry::skeleton(&catalog,model_name,zone,root)?)?;
            scene.write("prepared/material.gltf", &mut pack)?;
            pack.entries.retain(|uri, _| !uri.ends_with("/srgb.ktx2"));
            pack.write(std::io::BufWriter::new(std::fs::File::create(output)?))?;
            println!("Packed native model {model_name}: {} surfaces into {}", surfaces.len(), output.display());
            return Ok(());
        }
        let material = materials.get(&key).ok_or_else(|| format!("Native material/model is missing: {name}; matching models: {:?}", catalog.definitions(5).filter(|(model, _, _)| model.to_ascii_lowercase().contains(&key)).map(|(model, _, _)| model).collect::<Vec<_>>()))?;
        let selected = std::collections::BTreeMap::from([(key.clone(), material.clone())]);
        let required = material["textures"].as_array().ok_or("Native material textures are missing")?
            .iter().map(|binding| binding["image"].as_str().map(str::to_ascii_lowercase)
                .ok_or_else(|| "Native material image name is missing".into())).collect::<Result<std::collections::BTreeSet<_>>>()?;
        let mut pack = pack::Pack::default();
        let (_, images) = convert::images(&authorized, &catalog, &mut pack, Some(&required))?;
        let mut converted = std::collections::BTreeMap::new();
        let baked = crate::bake::world(&catalog, &selected, &images, &mut converted, &mut pack)?;
        if !converted.contains_key(&key) { return Err("Material-only output requires a supported layered world material".into()); }
        let mut scene = crate::gltf::Scene::new(name);
        let indices = scene.materials(baked.values().flatten(), &converted)?;
        let mesh = scene.mesh(name, baked.values().flatten(), &indices, false)?;
        scene.node(serde_json::json!({"name":name,"mesh":mesh}));
        scene.write("prepared/material.gltf", &mut pack)?;
        pack.entries.retain(|uri, _| !uri.starts_with("prepared/images/"));
        pack.write(std::io::BufWriter::new(std::fs::File::create(output)?))?;
        println!("Packed {} baked native surfaces into {}", baked.len(), output.display());
        return Ok(());
    }
    let camos = crate::material::read_camos(&catalog)?;
    let camo_options = crate::material::read_camo_options(&catalog)?;
    if camo_options[45].is_none() { return Err("Required Origins upgraded camo option is missing from the installed source table".into()); }
    let camo_dependencies = crate::material::camo_dependencies(&camos,&camo_options,45);
    let extra: std::collections::BTreeSet<_> = camo_dependencies.into_iter().filter(|name| !materials.contains_key(name)).collect();
    materials.extend(crate::material::read_used(&catalog,&extra)?);
    println!("Resolved {} native camo definitions and {} additional source/replacement material dependencies for Origins upgraded camo",camos.len(),extra.len());
    let game = crate::game::read(&catalog,&entities)?;
    let animations = crate::animation_decode::read(&catalog)?;
    let wide = animations.iter().filter(|entry| entry.clip.frames >= 256).count();
    let modes: std::collections::BTreeSet<_> = animations.iter().map(|entry| entry.clip.source_flags).collect();
    let jumpcuts = animations.iter().filter(|entry| entry.clip.notifies.iter().any(|(name,_)| name.as_deref().is_some_and(|name| name.eq_ignore_ascii_case("jumpcut")))).count();
    println!("Validated {} native ordinary animation streams ({wide} wide); {} separate root-delta tracks, {} native loop/delta/IK flag patterns and {jumpcuts} jumpcut clips require playback/pose selection before export", animations.len(), animations.iter().filter(|entry| entry.has_root_delta).count(), modes.len());
    let unsupported = crate::material::unimplemented(&materials);
    if !unsupported.is_empty() {
        return Err(format!("Required material conversion has {} unimplemented families:\n{}", unsupported.len(), unsupported.join("\n")).into());
    }
    let mut pack = pack::Pack::default();
    let collision = crate::collision::convert(&catalog, &mut pack)?;
    let sounds = crate::sound::convert(&authorized, &catalog, &mut pack)?;
    let (images, image_sources) = convert::images(&authorized, &catalog, &mut pack, None)?;
    let mut converted_materials = std::collections::BTreeMap::new();
    let baked = crate::bake::world(&catalog, &materials, &image_sources, &mut converted_materials, &mut pack)?;
    converted_materials.extend(crate::material::convert_known(&materials, &image_sources, &mut pack)?);
    let material_gaps = crate::material::missing(&materials, &converted_materials);
    let atmosphere = crate::atmosphere::read(&catalog, &image_sources, &mut pack)?;
    if !material_gaps.is_empty() {
        return Err(format!("Required material conversion has {} unresolved materials:\n{}", material_gaps.len(), material_gaps.join("\n")).into());
    }
    let (models, mut worlds) = crate::scenes::convert(&catalog, &entities, &converted_materials, &baked, &mut pack)?;
    let atmosphere_uri = "prepared/worlds/zm_tomb.atmosphere.json";
    pack.entries.insert(atmosphere_uri.into(), std::sync::Arc::new(serde_json::to_vec(&atmosphere)?));
    for world in &mut worlds {
        if world["map"] == "zm_tomb" {
            world["collision_uri"] = collision["uri"].clone();
            world["collision_meshes"] = collision["meshes"].clone();
            world["atmosphere_uri"] = serde_json::json!(atmosphere_uri);
        }
    }
    crate::game::write(game, &worlds, &collision, &mut pack)?;
    let runtime = runtime(&pack, models, worlds, images, sounds)?;
    pack.entries.insert("prepared/runtime.json".into(), std::sync::Arc::new(serde_json::to_vec(&runtime)?));
    crate::validate_pack(&pack)?;
    pack.write(std::io::BufWriter::new(std::fs::File::create(output)?))?;
    println!("Packed {} runtime entries into {}", pack.entries.len(), output.display());
    Ok(())
}

fn runtime(pack: &pack::Pack, models: Vec<serde_json::Value>, worlds: Vec<serde_json::Value>, images: Vec<serde_json::Value>, sounds: Vec<serde_json::Value>) -> Result<serde_json::Value> {
    use serde_json::{Value, json};
    let game: Value = serde_json::from_slice(pack.get("game.json")
        .map_err(|_| "Required game.json conversion is missing: native gameplay profiles, content tables, placements, presentation/animation bindings and UI/font definitions must be converted before writing origins.pack")?)?;
    if game["version"] != 2 { return Err("Required game.json must use runtime content version 2".into()); }
    let content = game["content"].as_object().ok_or("Required gameplay content catalog is missing")?;
    let presentations = content.get("presentations").and_then(Value::as_array)
        .ok_or("Required gameplay presentations are missing")?.iter().filter(|row| !row["scene"].is_null()).map(|row| {
            let id = row["id"].as_str().ok_or("Gameplay presentation ID is missing")?;
            let scene = row["scene"].as_str().ok_or("Gameplay presentation scene is missing")?;
            let uri = scene.split('#').next().ok_or("Gameplay presentation URI is missing")?;
            let model = models.iter().find(|model| model["uri"] == uri || model["lods"].as_array().into_iter().flatten().any(|lod| lod["uri"] == uri))
                .ok_or_else(|| format!("Required gameplay presentation has no exact exported model binding: {id} ({scene})"))?;
            pack.get(uri)?;
            let animations = model["animations"].as_object().ok_or_else(|| format!("Native model animation bindings are missing: {id}"))?;
            Ok(json!({"id":id,"uri":uri,"animations":animations,"name":model["name"],"zone":model["zone"]}))
        }).collect::<Result<Vec<_>>>()?;
    for animation in content.get("animations").and_then(Value::as_array).ok_or("Required gameplay animations are missing")? {
        let id = animation["id"].as_str().ok_or("Gameplay animation ID is missing")?;
        let owner = presentations.iter().find(|row| row["id"] == animation["skeleton"])
            .ok_or_else(|| format!("Required gameplay animation has no exported skeleton: {id}"))?;
        let matches: Vec<_> = owner["animations"].as_object().unwrap().iter()
            .filter(|(name, _)| id.to_ascii_lowercase().ends_with(&format!(".{}", name.to_ascii_lowercase()))).collect();
        if matches.len() != 1 || matches[0].1.as_u64().is_none() {
            return Err(format!("Required gameplay animation has no unique exported clip binding: {id}").into());
        }
    }
    let ui_assets = content.get("ui_assets").and_then(Value::as_array).ok_or("Required gameplay UI/font definitions are missing")?
        .iter().map(|row| {
            let id = row["id"].as_str().ok_or("Gameplay UI ID is missing")?;
            let uri = row["path"].as_str().ok_or("Gameplay UI image URI is missing")?;
            let image = images.iter().find(|image| image["uris"]["srgb"] == uri)
                .ok_or_else(|| format!("Required gameplay UI image has no exact exported image binding: {id} ({uri})"))?;
            pack.get(uri)?;
            Ok(json!({"id":id,"name":image["name"],"zone":image["zone"],"uri":uri}))
        }).collect::<Result<Vec<_>>>()?;
    let effects_uri = "prepared/effects.nativefx";
    let camo_uri = "prepared/camo.json";
    pack.get(effects_uri).map_err(|_| "Required native FX conversion is missing; no runtime pack was written")?;
    pack.get(camo_uri).map_err(|_| "Required native camo conversion is missing; no runtime pack was written")?;
    if worlds.iter().all(|world| world["map"] != "zm_tomb") { return Err("Required Origins world scene is missing".into()); }
    Ok(json!({"schema_version":1,"models":models,"worlds":worlds,"images":images,"sounds":sounds,
        "presentations":presentations,"ui_assets":ui_assets,"effects_uri":effects_uri,"camo_uri":camo_uri}))
}

#[cfg(test)]
mod tests {
    #[test]
    #[ignore = "requires the authorized Steam installation"]
    fn installed_ordinary_animation_streams() {
        let zones = super::read_zones(&super::installation().unwrap()).unwrap();
        let catalog = crate::convert::Catalog::new(&zones);
        crate::animation_decode::read(&catalog).unwrap();
    }

    #[test]
    #[ignore = "requires the authorized Steam installation"]
    fn installed_collision_brush_models() {
        let zones = super::read_zones(&super::installation().unwrap()).unwrap();
        let catalog = crate::convert::Catalog::new(&zones);
        crate::collision::convert(&catalog, &mut crate::pack::Pack::default()).unwrap();
    }

    #[test]
    #[ignore = "requires the authorized Steam installation"]
    fn installed_multiply_materials() {
        installed_material_families(&["lit_sm_r0c0n0_m1c1","lit_sm_r0c0n0_m1c1_m2c2","lit_sm_r0c0n0x0_m1c1"],15);
    }

    #[test]
    #[ignore = "requires the authorized Steam installation"]
    fn installed_additive_materials() {
        installed_material_families(&["lit_sm_r0c0n0x0_a1c1"],3);
    }

    #[test]
    #[ignore = "requires the authorized Steam installation"]
    fn installed_color_blend_materials() {
        installed_material_families(&["lit_sm_r0c0n0x0_b1c1","lit_sm_r0c0n0x0_b1c1_b2c2"],11);
    }

    #[test]
    #[ignore = "requires the authorized Steam installation"]
    fn installed_diffuse_reveal_materials() {
        installed_material_families(&["lit_sm_r0c0n0x0_b1c1n1v1"],5);
    }

    #[test]
    #[ignore = "requires the authorized Steam installation"]
    fn installed_ordinary_materials_batch() {
        let install = super::installation().unwrap();
        let zones = super::read_zones(&install).unwrap();
        let catalog = crate::convert::Catalog::new(&zones);
        let used = crate::geometry::validate(&catalog).unwrap();
        let mut materials = crate::material::read_used(&catalog,&used).unwrap();
        let families = ["mc_lit_sm_r0c0s0_986ezzjq","wpc_lit_sm_r0c0n0_80fe30wz","mc_lit_sm_t0c0_90wz6fe2",
            "wpc_lit_sm_t0c0_2w3887e6","wpc_lit_sm_t0c0n0_9ew0jz24"];
        materials.retain(|_,material| material["technique_set"].as_str().is_some_and(|family| families.contains(&family)));
        assert_eq!(materials.len(),38);
        let required = materials.values().flat_map(|material| material["textures"].as_array().unwrap())
            .map(|binding| binding["image"].as_str().unwrap().to_ascii_lowercase()).collect();
        let mut pack = crate::pack::Pack::default();
        let (_,images) = crate::convert::images(&install,&catalog,&mut pack,Some(&required)).unwrap();
        let converted = crate::material::convert_known(&materials,&images,&mut pack).unwrap();
        assert!(crate::material::missing(&materials,&converted).is_empty());
        let (mut masked,mut normals,mut responses) = (0,0,0);
        for material in converted.values() {
            let surface = &material["extras"]["surface"];
            masked+=usize::from(surface["presentation"]["alpha"] == "mask");
            for channel in surface["channels"].as_array().unwrap() {
                normals+=usize::from(channel["role"] == "normal");
                responses+=usize::from(channel["role"] == "response");
                crate::texture::validate(pack.get(channel["uri"].as_str().unwrap()).unwrap()).unwrap();
            }
        }
        assert_eq!((masked,normals,responses),(24,8,8));
        let mut scenes = 0;
        let mut triangles = 0;
        for (_,zone,root) in catalog.definitions(17) {
            let surfaces = crate::geometry::world(&catalog,zone,root).unwrap();
            let selected: Vec<_> = surfaces.iter().filter(|surface| converted.contains_key(&surface.material.to_ascii_lowercase())).collect();
            if selected.is_empty() { continue; }
            let mut scene = crate::gltf::Scene::new("ordinary-world");
            let indices = scene.materials(selected.iter().copied(),&converted).unwrap();
            let mesh = scene.mesh("ordinary-world",selected.iter().copied(),&indices,false).unwrap();
            scene.node(serde_json::json!({"mesh":mesh}));
            triangles+=selected.iter().map(|surface| surface.indices.len()/3).sum::<usize>();
            scene.write(&format!("prepared/ordinary/{scenes}.gltf"),&mut pack).unwrap();
            scenes+=1;
        }
        for (name,zone,root) in catalog.definitions(5) {
            for surfaces in crate::geometry::model(&catalog,name,zone,root).unwrap() {
                let selected: Vec<_> = surfaces.iter().filter(|surface| converted.contains_key(&surface.material.to_ascii_lowercase())).collect();
                if selected.is_empty() { continue; }
                let mut scene = crate::gltf::Scene::new(name);
                let indices = scene.materials(selected.iter().copied(),&converted).unwrap();
                let mesh = scene.mesh(name,selected.iter().copied(),&indices,true).unwrap();
                let owner = scene.node(serde_json::json!({"name":name,"mesh":mesh}));
                scene.skin(owner,&crate::geometry::skeleton(&catalog,name,zone,root).unwrap()).unwrap();
                triangles+=selected.iter().map(|surface| surface.indices.len()/3).sum::<usize>();
                scene.write(&format!("prepared/ordinary/{scenes}.gltf"),&mut pack).unwrap();
                scenes+=1;
            }
        }
        assert!(scenes>1 && triangles>0);
        for (uri,bytes) in &pack.entries {
            if !uri.ends_with(".gltf") { continue; }
            let doc: serde_json::Value = serde_json::from_slice(bytes).unwrap();
            for buffer in doc["buffers"].as_array().unwrap() {
                crate::dependency(&pack,std::path::Path::new(uri).parent().unwrap(),buffer["uri"].as_str().unwrap()).unwrap();
            }
        }
        println!("Converted five ordinary families: {} materials, {masked} masked, {normals} normal maps, {responses} response maps; exported {scenes} world/model LOD scenes and {triangles} triangles into {} in-memory pack entries",converted.len(),pack.entries.len());
    }

    #[test]
    #[ignore = "requires the authorized Steam installation"]
    fn installed_ordinary_materials_increment() {
        let install = super::installation().unwrap();
        let zones = super::read_zones(&install).unwrap();
        let catalog = crate::convert::Catalog::new(&zones);
        let used = crate::geometry::material_names(&catalog).unwrap();
        let all = crate::material::read_used(&catalog,&used).unwrap();
        assert_eq!(all.len(),1034);
        let gaps = crate::material::unimplemented(&all);
        assert_eq!(gaps.len(),107);
        let camos = crate::material::read_camos(&catalog).unwrap();
        let options = crate::material::read_camo_options(&catalog).unwrap();
        assert_eq!(camos.len(),35);
        assert!(options[45].unwrap().material);
        assert_eq!(options[45].unwrap().selection,13);
        let records: Vec<_> = camos.values().flat_map(|camo| camo.material_sets.get(12).into_iter().flatten()).collect();
        assert_eq!(records.len(),68);
        assert_eq!(records.iter().filter(|record| record.pairs.is_empty()).count(),7);
        let dependencies = crate::material::camo_dependencies(&camos,&options,45);
        let camo_materials = crate::material::read_used(&catalog,&dependencies).unwrap();
        let initial = std::collections::BTreeMap::from([(181,[0.;4]),(185,[0.;4])]);
        let mut camo_bindings = 0;
        for camo in camos.values() {
            let mut queue = Vec::new();
            crate::material::queue_camo(camo,&options,45,0.,2,&camo_materials,&mut queue).unwrap();
            assert!(queue.len()<=1024);
            for original in camo_materials.keys() {
                let selected = crate::material::camo_material(&queue,original,1);
                if selected == original.as_str() { continue; }
                let (bound,constants) = crate::material::camo_inputs(&queue,&camo_materials[selected],1,&initial).unwrap();
                for texture in bound["textures"].as_array().unwrap() { catalog.resolve(8,texture["image"].as_str().unwrap()).unwrap(); }
                assert_eq!(constants[&181],[0.;4]);
                assert!(constants[&185].iter().all(|value| value.is_finite()));
                camo_bindings+=1;
            }
        }
        assert!(camo_bindings>0);
        println!("Read {} installed camo definitions, {} material-set-13 records and {} source/replacement dependencies; resolved {camo_bindings} controlled model-index binding plans, not runtime-active descriptor selection",camos.len(),records.len(),dependencies.len());
        let prior = ["mc_lit_sm_r0c0s0_986ezzjq","wpc_lit_sm_r0c0n0_80fe30wz","mc_lit_sm_t0c0_90wz6fe2",
            "wpc_lit_sm_t0c0_2w3887e6","wpc_lit_sm_t0c0n0_9ew0jz24"];
        let families = ["mc_lit_sm_t0c0n0_9qf6e4qj","wpc_lit_sm_t0c0n0s0_weef63f3","mc_lit_sm_b0c0_fw0jf955",
            "mc_lit_sm_b0c0s0_805w75j2","wpc_lit_sm_b0c0n0_19f3588f","wpc_lit_sm_b0c0_6j30z67q",
            "mc_lit_sm_b0c0n0_67fqj7jf","wpc_lit_sm_r0c0s0_qfqz7850","wpc_lit_sm_r0c0x0_5j63fj97","wpc_lit_sm_b0c0s0_3f3q946z",
            "mc_unlit_replace_3j970129","mc_unlit_add_ww82zw88","mc_unlit_blend_588ewzff","wpc_unlit_add_8ez3wzw3","wpc_unlit_replace_4688792e",
            "mc_lit_sm_r0c0n0s0_ze50j7jz","mc_lit_sm_r0c0n0s0_7j414q0z","mc_lit_sm_r0c0n0s0_4w3wqfe5",
            "mc_lit_sm_r0c0n0x0_53144qq1","mc_lit_sm_r0c0_5137qq99","mc_lit_sm_r0c0d0n0_33ffej1u",
            "mc_lit_sm_r0c0d0n0s0_qwq4q449","mc_lit_sm_t0c0d0n0s0_68eww765","mc_lit_sm_r0c0n0s0o0_3z86zq2z",
            "mc_lit_sm_r0c0n0s0o0_909j993q","mc_lit_sm_b0c0n0s0o0_jf00fe4q"];
        let selected: std::collections::BTreeMap<_,_> = all.iter().filter(|(_,material)| {
            let family = material["technique_set"].as_str().unwrap(); families.contains(&family) || prior.contains(&family)
        }).map(|(key,material)| (key.clone(),material.clone())).collect();
        assert_eq!(selected.len(),194);
        let required = crate::material::inputs(&selected,&catalog).unwrap();
        assert!(!required.is_empty());
        let names = ["mc/mtl_p6_zm_tm_foliage_trunk_splinter","mc/mtl_p6_zm_wood_fence","wpc/decal_trim_metal_grate01",
            "mc/mtl_p6_zm_tm_science_glass","mc/me_glassdirt","mc/mtl_t6_veh_drone_quad_rotor_blades","wpc/zm_tm_decal_damage_crack_06",
            "wpc/zm_tm_stencil_numbers","mc/zm_tm_wall_concrete_blend","wpc/mtl_p6_zm_tm_science_metal","wpc/com_black2","wpc/me_glassdirt",
            "mc/mtl_p6_zm_tm_robot_map","mc/mtl_t6_attach_trinium_sight_green","mc/mat_veh_t6_dlc_zm_drone_water",
            "wpc/pb_decal_wall_fillet_2","wpc/global_color_black_unlit","mc/mtl_t6_wpn_zmb_monkey","mc/mtl_t6_attach_fastmag",
            "mc/mtl_veh_t6_dlc_mkiv_tank_treads","mc/mtl_p6_zm_tm_sandbags","mc/mtl_p6_zm_tm_sandbags_cheap",
            "mc/mtl_p6_wood_plank_rustic01","mc/mtl_p6_wood_plank_rustic01_2x8x108","mc/mtl_p6_zm_tm_packapunch_body",
            "mc/mtl_fx_char_gib_chunks","mc/mtl_t6_attach_gl_ammo","mc/mtl_t5_weapon_camera_spike",
            "mc/mtl_weapon_m14_scope_rail","mc/mtl_veh_t6_dlc_mkiv_tank_catwalk"];
        let mut samples: std::collections::BTreeMap<String,_> = names.iter().map(|&name| (name.into(),selected[name].clone())).collect();
        let (bound,constants) = crate::material::camo_inputs(&[],&selected["mc/mtl_t6_attach_fastmag"],1,
            &std::collections::BTreeMap::from([(181,[0.25,-0.5,3.,1.])])).unwrap();
        assert_eq!(constants[&181],[0.25,-0.5,3.,1.]);
        samples.insert("explicit_camo_state".into(),bound);
        for (key,name) in [("explicit_occlusion_camo_state","mc/mtl_t6_attach_gl_ammo"),("explicit_color_matrix_camo_state","mc/mtl_weapon_m14_scope_rail")] {
            let (bound,_) = crate::material::camo_inputs(&[],&selected[name],1,
                &std::collections::BTreeMap::from([(181,[0.25,-0.5,3.,1.])])).unwrap();
            samples.insert(key.into(),bound);
        }
        let sample_images = crate::material::inputs(&samples,&catalog).unwrap();
        let mut pack = crate::pack::Pack::default();
        let (rows,images) = crate::convert::images(&install,&catalog,&mut pack,Some(&sample_images)).unwrap();
        let mut shared = 0;
        for row in rows {
            let a = &pack.entries[row["uris"]["linear"].as_str().unwrap()];
            let b = &pack.entries[row["uris"]["srgb"].as_str().unwrap()];
            if std::sync::Arc::ptr_eq(a,b) { shared+=1; }
            crate::texture::validate(a).unwrap();
            crate::texture::validate(b).unwrap();
        }
        assert!(shared>0);
        let converted = crate::material::convert_known(&samples,&images,&mut pack).unwrap();
        assert_eq!(converted.len(),names.len()+3);
        for material in converted.values() {
            for channel in material["extras"]["surface"]["channels"].as_array().unwrap() {
                crate::texture::validate(pack.get(channel["uri"].as_str().unwrap()).unwrap()).unwrap();
            }
        }
        assert_eq!(converted.values().filter(|material| material["extras"]["surface"]["presentation"]["alpha"] == "mask").count(),4);
        assert_eq!(converted.values().filter(|material| material["extras"]["surface"]["presentation"]["alpha"] == "premultiplied").count(),8);
        assert_eq!(converted.values().filter(|material| material["extras"]["surface"]["standard"]["unlit"] == true).count(),5);
        let pending = crate::material::missing(&samples,&converted);
        assert_eq!(pending.len(),15);
        assert_eq!(pending.iter().filter(|reason| reason.contains("Native ONE/ONE addition")).count(),2);
        let camo = &converted["explicit_camo_state"]["extras"]["surface"];
        assert_eq!(camo["source_camo"]["parameters"],serde_json::json!([0.25,-0.5,3.,1.]));
        for role in ["source_color","source_camo"] {
            crate::texture::validate(pack.get(camo["source_camo"]["inputs"][role]["uri"].as_str().unwrap()).unwrap()).unwrap();
        }
        assert_ne!(camo["channels"],converted["mc/mtl_t6_attach_fastmag"]["extras"]["surface"]["channels"]);
        assert!(converted["mc/mtl_t6_attach_fastmag"]["extras"]["surface"]["source_camo"]["parameters"].is_null());
        assert_eq!(converted["mc/mtl_p6_zm_tm_sandbags"]["extras"]["surface"]["presentation"]["alpha"],"opaque");
        assert_eq!(converted["mc/mtl_p6_zm_tm_sandbags_cheap"]["extras"]["surface"]["standard"]["reflectance"],0.);
        for name in ["mc/mtl_p6_wood_plank_rustic01","mc/mtl_p6_wood_plank_rustic01_2x8x108","mc/mtl_p6_zm_tm_packapunch_body","mc/mtl_fx_char_gib_chunks"] {
            let detail = &converted[name]["extras"]["surface"]["source_detail_normal"];
            assert_eq!(detail["operation"],"scaled_slope_add_before_normalization");
            assert_eq!(detail["input"]["transfer"],"linear");
            crate::texture::validate(pack.get(detail["input"]["uri"].as_str().unwrap()).unwrap()).unwrap();
        }
        for name in ["explicit_occlusion_camo_state","explicit_color_matrix_camo_state","mc/mtl_veh_t6_dlc_mkiv_tank_catwalk"] {
            let surface = &converted[name]["extras"]["surface"];
            let occlusion = &surface["source_occlusion"];
            assert_eq!(occlusion["indirect_specular"],"green");
            assert_eq!(occlusion["direct_specular"],"red");
            crate::texture::validate(pack.get(occlusion["input"]["uri"].as_str().unwrap()).unwrap()).unwrap();
        }
        assert_eq!(converted["explicit_occlusion_camo_state"]["extras"]["surface"]["source_camo"]["color_bias"],"occlusion_blue");
        assert_eq!(converted["explicit_color_matrix_camo_state"]["extras"]["surface"]["source_color_matrix"]["rows"],serde_json::json!([114,115,116]));
        assert_eq!(converted["mc/mtl_veh_t6_dlc_mkiv_tank_catwalk"]["extras"]["surface"]["source_fragment_alpha"],"sqrt_texture_vertex_coverage");
        println!("Checked exact programs, routes, active inputs and states for {} materials in thirty-one families; {} image definitions resolve; converted {} representative materials/states; reused {shared} identical image encodings; {} dispatch gaps and fifteen sampled runtime gaps remain",selected.len(),required.len(),converted.len(),gaps.len());
    }

    #[test]
    #[ignore = "requires the authorized Steam installation and a hardware GPU"]
    fn installed_layered_response_batch() {
        let install = super::installation().unwrap();
        let zones = super::read_zones(&install).unwrap();
        let catalog = crate::convert::Catalog::new(&zones);
        let used = crate::geometry::material_names(&catalog).unwrap();
        let all = crate::material::read_used(&catalog,&used).unwrap();
        assert_eq!(all.len(),1034);
        assert_eq!(crate::material::unimplemented(&all).len(),107);
        let families = ["lit_sm_r0c0n0s0_b1c1n1","lit_sm_r0c0n0s0_b1c1n1s1v1","lit_sm_r0c0n0x0_b1c1n1s1",
            "lit_sm_r0c0n0x0_b1c1n1s1v1_m2c2","lit_sm_r0c0s0_b1c1","lit_sm_r0c0n0x0_a1c1_m2c2",
            "lit_sm_r0c0n0x0_b1c1n1_a2c2","lit_sm_r0c0n0x0_b1c1n1s1v1_a2c2","lit_sm_r0c0n0x0_b1c1_b2c2n2","lit_sm_r0c0n0x0_b1c1n1_b2c2n2",
            "lit_sm_r0c0n0x0_b1c1n1s1v1_b2c2n2s2v2","lit_sm_r0c0n0s0_b1c1n1_b2c2n2s2v2",
            "lit_sm_r0c0n0s0_b1c1n1s1v1_b2c2n2s2v2","lit_sm_r0c0n0x0_b1c1_b2c2n2s2v2","lit_sm_r0c0n0x0_b1c1n1s1v1_b2c2n2v2",
            "lit_sm_r0c0n0x0_b1c1n1_b2c2n2s2v2","lit_sm_r0c0n0x0_b1c1_b2c2n2s2","lit_sm_r0c0n0x0_b1c1n1s1v1_a2c2_m3c3","lit_sm_r0c0n0x0_t1c1n1"];
        let mut selected = std::collections::BTreeMap::new();
        let mut seen = std::collections::BTreeSet::new();
        let mut checked = 0;
        for (name,material) in &all {
            let family = material["technique_set"].as_str().unwrap();
            if !families.contains(&family) { continue; }
            crate::bake::validate_native(material).unwrap_or_else(|error| panic!("{name}: {error}"));
            for binding in material["textures"].as_array().unwrap() { catalog.resolve(8,binding["image"].as_str().unwrap()).unwrap(); }
            checked+=1;
            let clamped = material["textures"].as_array().unwrap().iter().any(|binding| binding["hash"] == 3054311507_u32 && binding["sampler"] == 74);
            if seen.insert(family) || clamped { selected.insert(name.clone(),material.clone()); }
        }
        assert_eq!(checked,58);
        assert_eq!(selected.len(),20);
        let ordinary = std::collections::BTreeMap::from([("mc/dec_glass02".into(),all["mc/dec_glass02"].clone())]);
        let mut required = crate::material::inputs(&ordinary,&catalog).unwrap();
        required.extend(selected.values().flat_map(|material| material["textures"].as_array().unwrap())
            .map(|binding| binding["image"].as_str().unwrap().to_ascii_lowercase()));
        let mut pack = crate::pack::Pack::default();
        let (_,images) = crate::convert::images(&install,&catalog,&mut pack,Some(&required)).unwrap();
        let converted = crate::material::convert_known(&ordinary,&images,&mut pack).unwrap();
        assert_eq!(converted.len(),1);
        let ordinary_pending = crate::material::missing(&ordinary,&converted);
        assert_eq!(ordinary_pending.len(),1);
        assert!(ordinary_pending[0].contains("sqrt coverage"));
        let pending = crate::bake::check_response_geometry(&catalog,&selected,&images,&mut pack).unwrap();
        assert_eq!(pending.len(),1);
        assert!(pending[0].contains("opaque layer selection"));
        println!("Checked all {checked} layered materials in nineteen families and one ordinary glass material; representative bakes for {} layered materials including wrap, clamp-V, nine-image, UV3 and layer-selection edges; 107 baseline dispatch gaps and two sampled framebuffer-composition runtime gaps remain",selected.len());
    }

    fn installed_material_families(families: &[&str], expected: usize) {
        let install = super::installation().unwrap();
        let zones = super::read_zones(&install).unwrap();
        let catalog = crate::convert::Catalog::new(&zones);
        let used = crate::geometry::validate(&catalog).unwrap();
        let mut materials = crate::material::read_used(&catalog,&used).unwrap();
        materials.retain(|_,material| material["technique_set"].as_str().is_some_and(|family| families.contains(&family)));
        assert_eq!(materials.len(),expected);
        let required = materials.values().flat_map(|material| material["textures"].as_array().unwrap())
            .map(|binding| binding["image"].as_str().unwrap().to_ascii_lowercase()).collect();
        let mut pack = crate::pack::Pack::default();
        let (_,images) = crate::convert::images(&install,&catalog,&mut pack,Some(&required)).unwrap();
        let mut converted = std::collections::BTreeMap::new();
        let baked = crate::bake::world(&catalog,&materials,&images,&mut converted,&mut pack).unwrap();
        assert!(crate::material::missing(&materials,&converted).is_empty());
        assert!(!baked.is_empty());
        for material in converted.values() {
            for channel in material["extras"]["surface"]["channels"].as_array().unwrap() {
                crate::texture::validate(pack.get(channel["uri"].as_str().unwrap()).unwrap()).unwrap();
            }
        }
        let mut scene = crate::gltf::Scene::new("multiply");
        let indices = scene.materials(baked.values().flatten(),&converted).unwrap();
        let mesh = scene.mesh("multiply",baked.values().flatten(),&indices,false).unwrap();
        scene.node(serde_json::json!({"name":"multiply","mesh":mesh}));
        scene.write("prepared/material.gltf",&mut pack).unwrap();
        println!("Converted {} installed materials, {} surfaces, {} triangles, {} packed entries; exported glTF bindings and KTX2 mip layouts resolved",materials.len(),baked.len(),baked.values().flatten().map(|surface| surface.indices.len()/3).sum::<usize>(),pack.entries.len());
    }

    #[test]
    #[ignore = "requires the authorized Steam installation"]
    fn installed_geometry_bindings() {
        let zones = super::read_zones(&super::installation().unwrap()).unwrap();
        let catalog = crate::convert::Catalog::new(&zones);
        let used = crate::geometry::validate(&catalog).unwrap();
        let materials = crate::material::read_used(&catalog,&used).unwrap();
        let gaps = crate::material::unimplemented(&materials);
        println!("Installed material dispatch: {} unimplemented families",gaps.len());
        for gap in &gaps { println!("Required material conversion: {gap}"); }
        let mut mixed_frames = 0;
        for (_,zone,root) in catalog.definitions(17) {
            for surface in crate::geometry::world(&catalog,zone,root).unwrap() {
                if !crate::bake::supported(materials[&surface.material.to_ascii_lowercase()]["technique_set"].as_str().unwrap()) { continue; }
                for triangle in surface.indices.chunks_exact(3) {
                    let signs: Vec<_> = triangle.iter().map(|&index| surface.tangents[index as usize][3]).collect();
                    assert!(signs.iter().all(|&sign| matches!(sign,-1. | 1.)),"Invalid source handedness in {}",surface.name);
                    if signs[1] != signs[0] || signs[2] != signs[0] { mixed_frames+=1; }
                }
            }
        }
        println!("Validated source handedness for implemented layered materials: {mixed_frames} mixed-frame triangles");
        let mut bones = 0;
        let mut models = 0;
        for (name,zone,root) in catalog.definitions(5) {
            let joints = crate::geometry::skeleton(&catalog,name,zone,root).unwrap();
            let mut scene = crate::gltf::Scene::new(name);
            let owner = scene.node(serde_json::json!({"name":name}));
            scene.skin(owner,&joints).unwrap_or_else(|error| panic!("Model {name}: {error}"));
            bones+=joints.len();
            models+=1;
        }
        println!("Validated glTF TRS bind hierarchies: {models} models, {bones} joints");
    }
}

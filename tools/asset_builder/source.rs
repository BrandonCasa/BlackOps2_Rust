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
    let materials = crate::material::read_used(&catalog, &used_materials)?;
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

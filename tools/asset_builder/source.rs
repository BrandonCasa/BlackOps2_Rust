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

pub fn build(install: &Path, preview: Option<(&str, &Path)>) -> Result<()> {
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
    if let Some((name, output)) = preview {
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
    if let Some(gap) = crate::material::unimplemented(&materials).first() {
        return Err(format!("Required material conversion is unimplemented: {gap}").into());
    }
    let mut pack = pack::Pack::default();
    let _collision = crate::collision::convert(&catalog, &mut pack)?;
    let _sounds = crate::sound::convert(&authorized, &catalog, &mut pack)?;
    let (_images, image_sources) = convert::images(&authorized, &catalog, &mut pack, None)?;
    let mut converted_materials = std::collections::BTreeMap::new();
    let baked = crate::bake::world(&catalog, &materials, &image_sources, &mut converted_materials, &mut pack)?;
    converted_materials.extend(crate::material::convert_known(&materials, &image_sources, &mut pack)?);
    let material_gaps = crate::material::missing(&materials, &converted_materials);
    let _atmosphere = crate::atmosphere::read(&catalog, &image_sources, &mut pack)?;
    if let Some(gap) = material_gaps.first() {
        return Err(format!("Required material conversion is unresolved: {gap}").into());
    }
    let (_models, _worlds) = crate::scenes::convert(&catalog, &entities, &converted_materials, &baked, &mut pack)?;
    Err("Native asset traversal and runtime conversion are not complete. No runtime pack was written.".into())
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
        if let Some(gap) = gaps.first() { println!("Next required material conversion: {gap}"); }
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

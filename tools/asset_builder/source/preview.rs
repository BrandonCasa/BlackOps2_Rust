use super::*;

pub(super) fn build(
    install: &Path,
    catalog: &catalog::Catalog<'_>,
    name: &str,
    output: &Path,
) -> Result<()> {
    let model = catalog
        .definitions(5)
        .find(|(model, _, _)| model.eq_ignore_ascii_case(name));
    let groups = if let Some((model_name, zone, root)) = model {
        crate::geometry::model(catalog, model_name, zone, root)?
    } else {
        catalog.resolve(6, name)?;
        let mut surfaces = Vec::new();
        for (_, zone, root) in catalog.definitions(17) {
            surfaces.extend(
                crate::geometry::world(catalog, zone, root)?
                    .into_iter()
                    .filter(|surface| surface.material.eq_ignore_ascii_case(name)),
            );
        }
        for (model_name, zone, root) in catalog.definitions(5) {
            let streams = &catalog.zones[zone].1.streams;
            let bindings = fastfile::Address::decode(streams.u32(root.add(36)?)?)?;
            let mut matching = false;
            for index in 0..streams.u8(root.add(6)?)? as usize {
                if catalog.pointer(zone, bindings.add(index * 4)?, 6)
                    .is_ok_and(|(binding, _, _)| binding.eq_ignore_ascii_case(name))
                {
                    matching = true;
                    break;
                }
            }
            if matching {
                surfaces.extend(
                    crate::geometry::model(catalog, model_name, zone, root)?
                        .into_iter()
                        .flatten()
                        .filter(|surface| surface.material.eq_ignore_ascii_case(name)),
                );
            }
        }
        vec![surfaces]
    };
    if groups.is_empty() || groups.iter().any(Vec::is_empty) {
        return Err(format!("Native preview {name} has an empty geometry selection").into());
    }
    let required_materials = groups
        .iter()
        .flatten()
        .map(|surface| surface.material.to_ascii_lowercase())
        .collect();
    let selected = crate::material::read_used(catalog, &required_materials)?;
    let required = crate::material::inputs(&selected, catalog)?;
    let mut pack = pack::Pack::streaming(output)?;
    let (_, images) = images::convert(install, catalog, &mut pack, Some(&required))?;
    let mut converted = crate::material::convert_known(&selected, &images, &mut pack)?;
    let baked = crate::bake::surfaces(
        &selected,
        groups.iter().flatten().cloned().collect(),
        &images,
        &mut converted,
        &mut pack,
    )?;
    let gaps = crate::material::missing(&selected, &converted);
    if !gaps.is_empty() {
        return Err(format!(
            "Native preview has {} unresolved material bindings:\n{}",
            gaps.len(),
            gaps.join("\n")
        )
        .into());
    }
    let skeleton = model
        .map(|(model_name, zone, root)| crate::geometry::skeleton(catalog, model_name, zone, root))
        .transpose()?;
    for (lod, source) in groups.iter().enumerate() {
        let surfaces = crate::scenes::prepared_surfaces(source, &baked)?;
        let mut scene = crate::gltf::Scene::new(name);
        let indices = scene.materials(&surfaces, &converted)?;
        let mesh = scene.mesh(name, &surfaces, &indices, skeleton.is_some())?;
        let owner = scene.node(serde_json::json!({"name":name,"mesh":mesh,
        "extras":if model.is_some() {serde_json::json!({"source_model":name,"source_lod":lod})} else {serde_json::json!({})}}));
        if let Some(skeleton) = &skeleton {
            scene.skin(owner, skeleton)?;
        }
        let uri = if lod == 0 {
            "prepared/material.gltf".to_owned()
        } else {
            format!("prepared/lods/{lod}.gltf")
        };
        scene.write(&uri, &mut pack)?;
    }
    crate::validate_entries(&pack)?;
    pack.write_to(output)?;
    println!(
        "Packed converted native preview {name}: {} source geometry groups into {}",
        groups.len(),
        output.display()
    );
    Ok(())
}

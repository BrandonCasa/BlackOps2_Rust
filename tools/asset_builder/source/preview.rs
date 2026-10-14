use super::*;

pub(super) fn build(
    install: &Path,
    catalog: &catalog::Catalog<'_>,
    materials: &std::collections::BTreeMap<String, serde_json::Value>,
    name: &str,
    output: &Path,
) -> Result<()> {
    let key = name.to_ascii_lowercase();
    let model = catalog
        .definitions(5)
        .find(|(model, _, _)| model.eq_ignore_ascii_case(name));
    let groups = if let Some((model_name, zone, root)) = model {
        crate::geometry::model(catalog, model_name, zone, root)?
    } else {
        if !materials.contains_key(&key) {
            return Err(format!("Native material/model is missing: {name}").into());
        }
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
                if catalog
                    .pointer(zone, bindings.add(index * 4)?, 6)?
                    .0
                    .eq_ignore_ascii_case(name)
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
    let selected = groups
        .iter()
        .flatten()
        .map(|surface| {
            let key = surface.material.to_ascii_lowercase();
            materials
                .get(&key)
                .cloned()
                .map(|material| (key, material))
                .ok_or_else(|| {
                    format!("Native preview material is missing: {}", surface.material).into()
                })
        })
        .collect::<Result<std::collections::BTreeMap<_, _>>>()?;
    let required = crate::material::inputs(&selected, catalog)?;
    let mut pack = pack::Pack::default();
    let (_, images) = images::convert(install, catalog, &mut pack, Some(&required))?;
    let mut converted = std::collections::BTreeMap::new();
    let baked = crate::bake::surfaces(
        &selected,
        groups.iter().flatten().cloned().collect(),
        &images,
        &mut converted,
        &mut pack,
    )?;
    converted.extend(crate::material::convert_known(
        &selected, &images, &mut pack,
    )?);
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
    pack.write(std::io::BufWriter::new(std::fs::File::create(output)?))?;
    println!(
        "Packed converted native preview {name}: {} source geometry groups into {}",
        groups.len(),
        output.display()
    );
    Ok(())
}

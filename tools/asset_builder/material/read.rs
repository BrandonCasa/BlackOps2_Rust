use super::*;

pub fn read(catalog: &Catalog<'_>, name: &str) -> Result<Value> {
    let (zone, root) = catalog.resolve(6, name)?;
    let streams = &catalog.zones[zone].1.streams;
    let (_, technique_zone, technique) = catalog.pointer(zone, root.add(92)?, 7)?;
    let technique_streams = &catalog.zones[technique_zone].1.streams;
    let family = technique_streams.text(Address::decode(technique_streams.u32(technique)?)?)?;
    let mut techniques = Vec::new();
    for index in [2, 3, 4, 6] {
        let value = technique_streams.u32(technique.add(8 + index * 4)?)?;
        if value == 0 {
            continue;
        }
        let native = Address::decode(value)?;
        let mut passes = Vec::new();
        for pass in 0..technique_streams.u16(native.add(6)?)? as usize {
            let pass = native.add(8 + pass * 24)?;
            let vertex = Address::decode(technique_streams.u32(pass.add(4)?)?)?;
            let vertex_shader =
                technique_streams.text(Address::decode(technique_streams.u32(vertex)?)?)?;
            let declaration = Address::decode(technique_streams.u32(pass)?)?;
            let stream_count = technique_streams.u8(declaration)? as usize;
            if stream_count > 16 {
                return Err(
                    format!("Native vertex declaration exceeds sixteen routes: {name}").into(),
                );
            }
            let routes = (0..stream_count).map(|index| -> Result<Value> {
                let route = declaration.add(4 + index * 2)?;
                Ok(json!({"source":technique_streams.u8(route)?,"destination":technique_streams.u8(route.add(1)?)?}))
            }).collect::<Result<Vec<_>>>()?;
            let pixel = Address::decode(technique_streams.u32(pass.add(8)?)?)?;
            let shader = technique_streams.text(Address::decode(technique_streams.u32(pixel)?)?)?;
            let count = (12..15)
                .map(|offset| -> Result<usize> {
                    Ok(technique_streams.u8(pass.add(offset)?)? as usize)
                })
                .collect::<Result<Vec<_>>>()?
                .into_iter()
                .sum::<usize>();
            let mut arguments = Vec::new();
            if count != 0 {
                let list = Address::decode(technique_streams.u32(pass.add(20)?)?)?;
                for argument in 0..count {
                    let item = list.add(argument * 12)?;
                    let kind = technique_streams.u16(item)?;
                    let value = technique_streams.u32(item.add(8)?)?;
                    let mut argument = json!({"type":kind,"location":technique_streams.u16(item.add(2)?)?,
                        "size":technique_streams.u16(item.add(4)?)?,"buffer":technique_streams.u16(item.add(6)?)?,"value":value});
                    if matches!(kind, 1 | 7) {
                        let literal = technique_streams.read(Address::decode(value)?, 16)?;
                        let literal: [f32; 4] =
                            std::array::from_fn(|index| f32::from_bits(u32_at(literal, index * 4)));
                        if !literal.iter().all(|value| value.is_finite()) {
                            return Err(format!("Non-finite native shader literal: {name}").into());
                        }
                        argument["literal"] = json!(literal);
                    }
                    arguments.push(argument);
                }
            }
            passes.push(json!({"vertex_shader":vertex_shader,"vertex_routes":routes,"pixel_shader":shader,"arguments":arguments}));
        }
        techniques.push(json!({"index":index,"passes":passes}));
    }
    let mut textures = Vec::new();
    let texture_count = streams.u8(root.add(84)?)? as usize;
    if texture_count != 0 {
        let table = Address::decode(streams.u32(root.add(96)?)?)?;
        for index in 0..texture_count {
            let item = table.add(index * 16)?;
            let (image, _, _) = catalog.pointer(zone, item.add(12)?, 8)?;
            textures.push(
                json!({"hash":streams.u32(item)?,"sampler":streams.u8(item.add(6)?)?,
                "semantic":streams.u8(item.add(7)?)?,"image":image}),
            );
        }
    }
    let mut constants = Vec::new();
    let constant_count = streams.u8(root.add(85)?)? as usize;
    if constant_count != 0 {
        let table = Address::decode(streams.u32(root.add(100)?)?)?;
        for index in 0..constant_count {
            let item = streams.read(table.add(index * 32)?, 32)?;
            let literal: [f32; 4] =
                std::array::from_fn(|i| f32::from_bits(u32_at(item, 16 + i * 4)));
            if !literal.iter().all(|value| value.is_finite()) {
                return Err(format!("Non-finite native material constant: {name}").into());
            }
            let short_name = std::str::from_utf8(item[4..16].split(|b| *b == 0).next().unwrap())?;
            constants.push(json!({"hash":u32_at(item, 0),"name":short_name,"literal":literal}));
        }
    }
    let mut states = Vec::new();
    let state_count = streams.u8(root.add(86)?)? as usize;
    if state_count != 0 {
        let table = Address::decode(streams.u32(root.add(104)?)?)?;
        for index in 0..state_count {
            states.push(json!([
                streams.u32(table.add(index * 20)?)?,
                streams.u32(table.add(index * 20 + 4)?)?
            ]));
        }
    }
    Ok(
        json!({"name":name,"technique_set":family,"techniques":techniques,"textures":textures,"constants":constants,
        "state_entries":streams.read(root.add(48)?, 36)?,"state_bits":states}),
    )
}

pub fn read_used(
    catalog: &Catalog<'_>,
    names: &std::collections::BTreeSet<String>,
) -> Result<BTreeMap<String, Value>> {
    let mut materials = BTreeMap::new();
    let mut families = BTreeMap::<String, usize>::new();
    for name in names {
        let material = read(catalog, name)?;
        *families
            .entry(
                material["technique_set"]
                    .as_str()
                    .ok_or("Native material technique name is missing")?
                    .into(),
            )
            .or_default() += 1;
        materials.insert(name.to_ascii_lowercase(), material);
    }
    println!(
        "Read {} used native materials in {} technique families",
        materials.len(),
        families.len()
    );
    Ok(materials)
}

use crate::{
    catalog::Catalog,
    core::*,
    fastfile::{Address, Streams},
    material,
    pack::Pack,
    texture,
};
use serde_json::{Value, json};
use std::{collections::BTreeMap, sync::Arc};

fn floats<const N: usize>(streams: &Streams, address: Address) -> Result<[f32; N]> {
    let mut values = [0.; N];
    for (index, value) in values.iter_mut().enumerate() {
        *value = f32::from_bits(streams.u32(address.add(index * 4)?)?);
        if !value.is_finite() {
            return Err("Non-finite native atmosphere component".into());
        }
    }
    Ok(values)
}

fn fog(streams: &Streams, root: Address) -> Result<Value> {
    Ok(
        json!({"distance":floats::<4>(streams, root)?.map(|v| v * 0.0254),
        "sun_angles":floats::<4>(streams, root.add(16)?)?,"color":floats::<3>(streams, root.add(32)?)?,
        "opacity":floats::<1>(streams, root.add(44)?)?[0],"sun_color":floats::<3>(streams, root.add(48)?)?,
        "sun_opacity":floats::<1>(streams, root.add(60)?)?[0]}),
    )
}

fn transition(streams: &Streams, address: Address) -> Result<f32> {
    let value = floats::<1>(streams, address)?[0];
    if value < 0. {
        return Err("Negative native fog transition rate".into());
    }
    Ok(if value == 0. { 0. } else { 0.001 / value })
}

fn vision(catalog: &Catalog<'_>, name: &str) -> Result<BTreeMap<String, [f32; 4]>> {
    let (zone, root) = catalog.resolve(41, name)?;
    let streams = &catalog.zones[zone].1.streams;
    let pointer = Address::decode(streams.u32(root.add(8)?)?)?;
    let bytes = streams.read(pointer, streams.u32(root.add(4)?)? as usize)?;
    let text = std::str::from_utf8(bytes)?
        .trim_start_matches('\u{feff}')
        .trim_end_matches('\0');
    let mut values = BTreeMap::new();
    for line in text.lines() {
        let line = line.trim();
        let Some(end) = line.find(char::is_whitespace) else {
            continue;
        };
        let key = &line[..end];
        if !key.starts_with("vc_") {
            continue;
        }
        let quoted = line[end..]
            .trim()
            .split('"')
            .nth(1)
            .ok_or_else(|| format!("Malformed native vision value: {name}/{key}"))?;
        let components: Vec<f32> = quoted
            .split_whitespace()
            .map(str::parse)
            .collect::<std::result::Result<_, _>>()?;
        let components: [f32; 4] = components.try_into().map_err(|_| {
            format!("Native vision value has unsupported vector length: {name}/{key}")
        })?;
        if !components.iter().all(|v| v.is_finite()) {
            return Err(format!("Non-finite native vision value: {name}/{key}").into());
        }
        if values.insert(key.into(), components).is_some() {
            return Err(format!("Duplicate native vision value: {name}/{key}").into());
        }
    }
    for key in [
        "vc_RS", "vc_RE", "vc_FSM", "vc_SMR", "vc_MMR", "vc_HMR", "vc_SMG", "vc_MMG", "vc_HMG",
        "vc_SMB", "vc_MMB", "vc_HMB", "vc_FGM", "vc_FBM",
    ] {
        if !values.contains_key(key) {
            return Err(format!("Native vision value is missing: {name}/{key}").into());
        }
    }
    Ok(values)
}

fn sky(
    catalog: &Catalog<'_>,
    zone: usize,
    world: Address,
    images: &BTreeMap<String, texture::Image>,
    pack: &mut Pack,
) -> Result<Value> {
    let streams = &catalog.zones[zone].1.streams;
    let model_name = streams.text(Address::decode(streams.u32(world.add(36)?)?)?)?;
    let (owner, model) = catalog.resolve(5, &model_name)?;
    let model_streams = &catalog.zones[owner].1.streams;
    let count = model_streams.u8(model.add(6)?)? as usize;
    let table = Address::decode(model_streams.u32(model.add(36)?)?)?;
    let mut names = std::collections::BTreeSet::new();
    for index in 0..count {
        names.insert(catalog.pointer(owner, table.add(index * 4)?, 6)?.0);
    }
    if names.len() != 1 {
        return Err(format!(
            "Native sky model {model_name} requires {} materials",
            names.len()
        )
        .into());
    }
    let native = material::read(catalog, names.first().unwrap())?;
    if native["technique_set"] != "mc_skycubemaphdr_1f117w7u" {
        return Err("Unreviewed native sky technique".into());
    }
    let constant = |hash: u32| -> Result<[f32; 4]> {
        let value = native["constants"]
            .as_array()
            .unwrap()
            .iter()
            .find(|value| value["hash"] == hash)
            .ok_or_else(|| format!("Native sky constant {hash} is missing"))?;
        Ok(serde_json::from_value(value["literal"].clone())?)
    };
    let color = constant(343012833)?;
    let rotation = constant(3638790757)?;
    if rotation[3] <= 0.
        || (rotation[0] + rotation[2]).abs() > 1e-5
        || (rotation[0] * rotation[0] + rotation[1] * rotation[1] - 1.).abs() > 1e-5
    {
        return Err("Native sky rotation is outside the reviewed installed convention".into());
    }
    let binding = native["textures"]
        .as_array()
        .unwrap()
        .iter()
        .find(|value| value["hash"] == 2695565377_u32 && value["semantic"] == 2)
        .ok_or("Native sky cube binding is missing")?;
    let image_name = binding["image"]
        .as_str()
        .ok_or("Native sky cube name is missing")?;
    let source = images
        .get(&image_name.to_ascii_lowercase())
        .ok_or("Native sky cube image is missing")?;
    if source.faces != 6 || source.depth != 1 || source.width != source.height {
        return Err("Native sky image is not a square cube".into());
    }
    let source_uri = "prepared/worlds/zm_tomb.sky-source.ktx2";
    let bytes = texture::ktx2(source, false)?;
    texture::validate(&bytes)?;
    pack.entries.insert(source_uri.into(), Arc::new(bytes));
    let mut baked = source.clone();
    baked.format = 19;
    baked.levels.clear();
    for level in 0..source.levels.len() {
        let side = (source.width >> level).max(1);
        let pixels = texture::pixels(source, level)?;
        let mut bytes = Vec::with_capacity(pixels.len() * 8);
        for face in 0..6 {
            let source_face = match face {
                4 => 5,
                5 => 4,
                _ => face,
            };
            for y in 0..side {
                for x in 0..side {
                    let (sx, sy) = if matches!(face, 2 | 3) {
                        (x, side - 1 - y)
                    } else {
                        (side - 1 - x, y)
                    };
                    let pixel = pixels[(source_face * side + sy) * side + sx];
                    let rgb: [f32; 3] = std::array::from_fn(|channel| {
                        if color[0] > 0. {
                            pixel[channel] / (pixel[3] + f32::from_bits(0x358637bd))
                        } else {
                            pixel[channel] * pixel[channel]
                        }
                    });
                    for value in rgb.into_iter().chain([1.]) {
                        let value = half::f16::from_f32(value);
                        if !value.is_finite() {
                            return Err("Baked sky HDR component exceeds binary16 range".into());
                        }
                        bytes.extend_from_slice(&value.to_bits().to_le_bytes());
                    }
                }
            }
        }
        baked.levels.push(bytes);
    }
    let uri = "prepared/worlds/zm_tomb.sky.ktx2";
    let bytes = texture::ktx2(&baked, false)?;
    texture::validate(&bytes)?;
    pack.entries.insert(uri.into(), Arc::new(bytes));
    Ok(
        json!({"uri":uri,"source_uri":source_uri,"source_model":model_name,"native_material":native,
        "color_parameters":color,"rotation_parameters":rotation,
        "rotation_columns":[[rotation[1],0.,-rotation[2]],[rotation[0],0.,-rotation[1]],[0.,1.,0.]],
        "approximations":["HDR cube decoded at source mip texels before filtering for upstream Bevy Skybox and environment lighting"]}),
    )
}

pub fn inputs(catalog: &Catalog<'_>) -> Result<std::collections::BTreeSet<String>> {
    let mut images = std::collections::BTreeSet::new();
    for (_, zone, world) in catalog.definitions(17) {
        let streams = &catalog.zones[zone].1.streams;
        let model_name = streams.text(Address::decode(streams.u32(world.add(36)?)?)?)?;
        let (owner, model) = catalog.resolve(5, &model_name)?;
        let model_streams = &catalog.zones[owner].1.streams;
        let materials = Address::decode(model_streams.u32(model.add(36)?)?)?;
        for index in 0..model_streams.u8(model.add(6)?)? as usize {
            let name = catalog.pointer(owner, materials.add(index * 4)?, 6)?.0;
            let material = material::read(catalog, &name)?;
            let binding = material["textures"]
                .as_array()
                .ok_or("Native sky textures are missing")?
                .iter()
                .find(|binding| binding["hash"] == 2695565377_u32 && binding["semantic"] == 2)
                .ok_or("Native sky cube binding is missing")?;
            images.insert(
                binding["image"]
                    .as_str()
                    .ok_or("Native sky cube name is missing")?
                    .to_ascii_lowercase(),
            );
        }
        let lut = material::read(catalog, &catalog.pointer(zone, world.add(984)?, 6)?.0)?;
        let binding = lut["textures"]
            .as_array()
            .ok_or("Native grade textures are missing")?
            .iter()
            .find(|binding| binding["hash"] == 2008031959_u32 && binding["semantic"] == 26)
            .ok_or("Native grade image binding is missing")?;
        images.insert(
            binding["image"]
                .as_str()
                .ok_or("Native grade image name is missing")?
                .to_ascii_lowercase(),
        );
    }
    Ok(images)
}

pub fn read(
    catalog: &Catalog<'_>,
    images: &BTreeMap<String, texture::Image>,
    pack: &mut Pack,
) -> Result<Value> {
    let (_, zone, root) = catalog
        .definitions(17)
        .next()
        .ok_or("Native Origins world is missing")?;
    let streams = &catalog.zones[zone].1.streams;
    for (offset, kind) in [(292, "exposure"), (340, "LUT")] {
        let count = streams.u32(root.add(offset)?)?;
        if count != 0 {
            return Err(format!(
                "Native world has {count} {kind} volumes requiring runtime support"
            )
            .into());
        }
    }
    let count = streams.u32(root.add(308)?)? as usize;
    let plane_count = streams.u32(root.add(316)?)? as usize;
    let native_volumes = Address::decode(streams.u32(root.add(312)?)?)?;
    let native_planes = Address::decode(streams.u32(root.add(320)?)?)?;
    let mut volumes = Vec::new();
    for order in 0..count {
        let volume = native_volumes.add(order * 100)?;
        let control = streams.u32(volume.add(12)?)?;
        let first = (control >> 16) as usize;
        let count = ((control >> 8 & 255) + 1) as usize;
        if first + count > plane_count {
            return Err("Native fog volume exceeds plane array".into());
        }
        let mut planes = Vec::new();
        for index in first..first + count {
            let [x, y, z, d] = floats::<4>(streams, native_planes.add(index * 16)?)?;
            planes.push([x, z, -y, d * 0.0254]);
        }
        volumes.push(json!({"order":order,"bank":control & 255,"script_id":streams.u32(volume.add(32)?)? & 65535,
            "transition_seconds":transition(streams, volume.add(28)?)?,"planes":planes,"fog":fog(streams, volume.add(36)?)?}));
    }
    let modifier_count = streams.u32(root.add(324)?)? as usize;
    let mut modifiers = Vec::new();
    if modifier_count != 0 {
        let native_modifiers = Address::decode(streams.u32(root.add(328)?)?)?;
        let plane_count = streams.u32(root.add(332)?)? as usize;
        let native_planes = Address::decode(streams.u32(root.add(336)?)?)?;
        for order in 0..modifier_count {
            let modifier = native_modifiers.add(order * 48)?;
            let control = streams.u32(modifier)?;
            let first = (control >> 16) as usize;
            let count = ((control >> 8 & 255) + 1) as usize;
            if first + count > plane_count {
                return Err("Native fog modifier exceeds plane array".into());
            }
            let mut planes = Vec::new();
            for index in first..first + count {
                let [x, y, z, d] = floats::<4>(streams, native_planes.add(index * 16)?)?;
                planes.push([x, z, -y, d * 0.0254]);
            }
            let bounds = (0..6)
                .map(|index| -> Result<u16> { Ok(streams.u16(modifier.add(4 + index * 2)?)?) })
                .collect::<Result<Vec<_>>>()?;
            modifiers.push(json!({"order":order,"control":control,"control_ex":streams.u32(modifier.add(16)?)?,"packed_bounds":bounds,
                "transition_rate":floats::<1>(streams, modifier.add(20)?)?[0],"depth_scale":floats::<1>(streams, modifier.add(24)?)?[0],
                "height_scale":floats::<1>(streams, modifier.add(28)?)?[0],"color_adjust":floats::<4>(streams, modifier.add(32)?)?,"planes":planes}));
        }
    }
    let lut_name = catalog.pointer(zone, root.add(984)?, 6)?.0;
    let lut = material::read(catalog, &lut_name)?;
    let binding = lut["textures"]
        .as_array()
        .unwrap()
        .iter()
        .find(|binding| binding["hash"] == 2008031959_u32 && binding["semantic"] == 26)
        .ok_or_else(|| {
            format!(
                "Native world LUT material {lut_name} has no reviewed lookup binding; bindings: {}",
                lut["textures"]
            )
        })?;
    let image_name = binding["image"]
        .as_str()
        .ok_or("Native LUT image name is missing")?;
    let image = images
        .get(&image_name.to_ascii_lowercase())
        .ok_or("Native LUT image source is missing")?;
    if (image.width, image.height, image.depth, image.faces) != (1024, 64, 1, 1) {
        return Err("Native world LUT has an unsupported layout".into());
    }
    let mut grade = image.clone();
    grade.format = 1;
    grade.levels = (0..image.levels.len())
        .map(|level| -> Result<Vec<u8>> {
            texture::pixels(image, level)?
                .into_iter()
                .flatten()
                .map(|value| {
                    if !value.is_finite() || !(0. ..=1.).contains(&value) {
                        return Err("Native LUT channel exceeds UNORM range".into());
                    }
                    Ok((value * 255.).round() as u8)
                })
                .collect()
        })
        .collect::<Result<_>>()?;
    let grade_uri = "prepared/worlds/zm_tomb.grade.ktx2";
    let grade_bytes = texture::ktx2(&grade, false)?;
    texture::validate(&grade_bytes)?;
    pack.entries.insert(grade_uri.into(), Arc::new(grade_bytes));
    let mut visions = BTreeMap::new();
    for (bank, suffix) in [("clear", ""), ("_rain", "_rain"), ("_snow", "_snow")] {
        visions.insert(
            bank,
            vision(catalog, &format!("vision/zm_tomb{suffix}.vision"))?,
        );
    }
    let sun_color = floats::<4>(streams, root.add(136)?)?;
    if sun_color[3] <= 0. {
        return Err("Native world sunlight brightness is not positive".into());
    }
    let sky = sky(catalog, zone, root, images, pack)?;
    println!(
        "Converted native atmosphere: {} fog volumes, {} vision banks, original 1024x64 grade",
        volumes.len(),
        visions.len()
    );
    Ok(
        json!({"sun_angles":floats::<3>(streams, root.add(108)?)?,"sun_color":&sun_color[..3],"sun_brightness":sun_color[3],
        "snow_sun_color":[0.769943,0.894102,1.],"exposure":floats::<1>(streams, root.add(184)?)?[0],
        "fog":fog(streams, root.add(192)?)?,"fog_transition_seconds":transition(streams, root.add(188)?)?,
        "volumes":volumes,"modifiers":modifiers,"softclip":0.25,"vision":visions,"grade_uri":grade_uri,
        "grade_banks":image.height / 32,"sky_uri":sky["uri"],"sky_rotation_columns":sky["rotation_columns"],"sky":sky}),
    )
}

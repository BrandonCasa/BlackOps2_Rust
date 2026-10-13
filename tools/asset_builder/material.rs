use crate::{convert::Catalog, core::*, fastfile::Address};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use crate::{pack::Pack, texture};
use std::sync::Arc;

fn store_image(source: &texture::Image, uri: String, srgb: bool, role: &str, pack: &mut Pack) -> Result<String> {
    let bytes = texture::encode(source, srgb, role)?;
    texture::validate(&bytes)?;
    if pack.entries.insert(uri.clone(), Arc::new(bytes)).is_some() { return Err(format!("Duplicate converted material texture: {uri}").into()); }
    Ok(uri)
}

fn bake(source: &texture::Image, role: &str) -> Result<texture::Image> {
    if source.depth != 1 || source.faces != 1 { return Err(format!("Ordinary lit {role} channel is not a 2D image").into()); }
    let mut result = source.clone();
    result.format = 1;
    result.levels.clear();
    for level in 0..source.levels.len() {
        let mut bytes = Vec::new();
        for pixel in texture::pixels(source, level)? {
            let mut value = pixel;
            match role {
                "color" => {
                    for channel in &mut value[..3] {
                        let linear = *channel * *channel;
                        *channel = if linear <= 0.0031308 { linear * 12.92 } else { 1.055 * linear.powf(1. / 2.4) - 0.055 };
                    }
                }
                "normal" => {
                    let x = pixel[0] * f32::from_bits(0x40808102) + f32::from_bits(0xc0010204);
                    let y = pixel[1] * f32::from_bits(0x40808102) + f32::from_bits(0xc0010204);
                    let length = (x*x + y*y + 1.).sqrt();
                    value = [x / length * 0.5 + 0.5, y / length * 0.5 + 0.5, 1. / length * 0.5 + 0.5, 1.];
                }
                "response" => {
                    let roughness = (2. / (2_f32.powf(13. * pixel[3]) + 2.)).powf(0.25);
                    value = [1., roughness, 0., 1.];
                }
                _ => return Err(format!("Unknown material bake channel {role}").into()),
            }
            if !value.iter().all(|v| v.is_finite()) { return Err("Non-finite converted material texel".into()); }
            if value.iter().any(|v| !(0. ..=1.).contains(v)) { return Err("Converted material channel exceeds UNORM range".into()); }
            bytes.extend(value.map(|v| (v * 255.).round() as u8));
        }
        result.levels.push(bytes);
    }
    Ok(result)
}

pub fn pbr_pixel(diffuse: [f32; 3], specular: [f32; 3], gloss: f32) -> ([f32; 3], f32, f32) {
    let brightness = |rgb: [f32; 3]| (rgb[0]*rgb[0]*0.299+rgb[1]*rgb[1]*0.587+rgb[2]*rgb[2]*0.114).sqrt();
    let energy = 1.-specular.into_iter().fold(0.,f32::max);
    let specular_brightness = brightness(specular);
    let metallic = if specular_brightness < 0.04 { 0. } else {
        let b = brightness(diffuse)*energy/0.96+specular_brightness-0.08;
        ((-b+(b*b-0.16*(0.04-specular_brightness)).max(0.).sqrt())/0.08).clamp(0.,1.)
    };
    let base = std::array::from_fn(|c| {
        let from_diffuse = diffuse[c]*energy/0.96/(1.-metallic).max(1e-6);
        let from_specular = (specular[c]-0.04*(1.-metallic))/metallic.max(1e-6);
        (from_diffuse+(from_specular-from_diffuse)*metallic*metallic).clamp(0.,1.)
    });
    (base,metallic,(2./(2_f32.powf(13.*gloss)+2.)).powf(0.25))
}

fn metallic_workflow(color: &texture::Image, specular: &texture::Image, samplers: [u8; 2]) -> Result<(texture::Image,texture::Image)> {
    if [color,specular].iter().any(|image| image.faces != 1 || image.depth != 1) { return Err("Specular-workflow input is not a 2D image".into()); }
    let (width,height) = (color.width.max(specular.width),color.height.max(specular.height));
    let mut base = texture::Image { width,height,depth:1,faces:1,format:1,flags:0,levels:Vec::new() };
    let mut response = base.clone();
    let sources = [color,specular];
    let decoded = sources.iter().map(|image| (0..image.levels.len()).map(|level| texture::pixels(image,level)).collect::<Result<Vec<_>>>()).collect::<Result<Vec<_>>>()?;
    for level in 0..color.levels.len().max(specular.levels.len()) {
        let (w,h) = ((width>>level).max(1),(height>>level).max(1));
        let mut base_bytes = Vec::with_capacity(w*h*4);
        let mut response_bytes = Vec::with_capacity(w*h*4);
        for y in 0..h { for x in 0..w {
            let uv = [(x as f32+0.5)/w as f32,(y as f32+0.5)/h as f32];
            let samples: [[f32; 4]; 2] = std::array::from_fn(|index| {
                let image = sources[index];
                let bias = width.max(height).ilog2()-image.width.max(image.height).ilog2();
                let source_level = level.saturating_sub(bias as usize).min(image.levels.len()-1);
                texture::sample(&decoded[index][source_level],(image.width>>source_level).max(1),(image.height>>source_level).max(1),uv,samplers[index])
            });
            let (rgb,metallic,roughness) = pbr_pixel(std::array::from_fn(|c| samples[0][c]*samples[0][c]),std::array::from_fn(|c| samples[1][c]*samples[1][c]),samples[1][3]);
            for value in rgb { let srgb = if value <= 0.0031308 {value*12.92} else {1.055*value.powf(1./2.4)-0.055}; base_bytes.push((srgb*255.).round() as u8); }
            base_bytes.push((samples[0][3]*255.).round() as u8);
            response_bytes.extend([255,(roughness*255.).round() as u8,(metallic*255.).round() as u8,255]);
        }}
        base.levels.push(base_bytes);
        response.levels.push(response_bytes);
    }
    Ok((base,response))
}

fn ordinary_recipe(family: &str) -> Option<(&'static str,bool,u8,u8)> {
    Some(match family {
        "mc_lit_sm_b0c0n0s0_91j5z9q9" => ("pimp_shader_lprobe_8bb4c6fc.hlsl", true, 1, 1),
        "wpc_lit_sm_r0c0n0s0_1zzj1138" => ("pimp_shader_lmap_353f0d80.hlsl", true, 1, 0),
        "mc_lit_sm_r0c0n0s0_zqq1fze7" => ("pimp_shader_lprobe_4bceb0cc.hlsl", true, 1, 0),
        "mc_lit_sm_r0c0n0x0_q361191u" => ("pimp_shader_lprobe_f9c3b462.hlsl", true, 2, 0),
        "wpc_lit_sm_r0c0n0x0_f488034e" => ("pimp_shader_lmap_9a71a1e7.hlsl", true, 2, 0),
        "mc_lit_sm_r0c0_77e21qq8" => ("pimp_shader_lprobe_9c5921d1.hlsl", false, 0, 0),
        "mc_lit_sm_r0c0n0_2z223015" => ("pimp_shader_lprobe_a06c1664.hlsl", true, 0, 0),
        "mc_lit_sm_t0c0n0s0_47jq982j" => ("pimp_shader_lprobe_b646e86e.hlsl", true, 1, 2),
        _ => return None,
    })
}

pub fn convert_known(native: &BTreeMap<String, Value>, images: &BTreeMap<String, texture::Image>, pack: &mut Pack) -> Result<BTreeMap<String, Value>> {
    let mut converted = BTreeMap::new();
    let mut missing = BTreeMap::<String, Vec<String>>::new();
    let mut shared = BTreeMap::<(String, String), String>::new();
    for (key, material) in native {
        let family = material["technique_set"].as_str().ok_or("Missing native material technique")?;
        let Some((expected, normal, specular, coverage)) = ordinary_recipe(family) else {
            missing.entry(family.into()).or_default().push(key.clone()); continue;
        };
        let techniques = material["techniques"].as_array().ok_or("Missing native material techniques")?;
        let lit = techniques.iter().find(|technique| technique["index"] == 4).ok_or("Ordinary lit technique is missing")?;
        let passes = lit["passes"].as_array().ok_or("Native technique pass array is missing")?;
        if passes.len() != 1 { return Err(format!("Ordinary lit material {key} has unsupported pass count").into()); }
        let shader = passes[0]["pixel_shader"].as_str().ok_or("Native pixel shader name is missing")?;
        if shader != expected { return Err(format!("Native technique {family} has an unreviewed pixel shader: {shader}").into()); }
        let mut channels = Vec::new();
        let mut bindings = vec![(2695565377_u32, if specular == 1 { Vec::new() } else { vec!["color"] })];
        if normal { bindings.push((1507003663, vec!["normal"])); }
        if specular == 1 { bindings.push((887934131, Vec::new())); }
        if specular == 2 { bindings.push((2695565377, vec!["response"])); }
        let mut native_specular = Value::Null;
        for (hash, roles) in bindings {
            let binding = material["textures"].as_array().unwrap().iter().find(|binding| binding["hash"] == hash)
                .ok_or_else(|| format!("Ordinary lit material {key} has no source binding {hash}"))?;
            if !passes[0]["arguments"].as_array().unwrap().iter().any(|argument| argument["type"] == 2 && argument["value"] == hash) {
                return Err(format!("Reviewed material {key} has no selected shader binding for {hash}").into());
            }
            let image_name = binding["image"].as_str().ok_or("Native image binding name is missing")?;
            let source = images.get(&image_name.to_ascii_lowercase()).ok_or_else(|| format!("Material {key} image {image_name} is missing"))?;
            for role in roles {
                let channel_key = (image_name.to_ascii_lowercase(), role.to_owned());
                let srgb = role == "color";
                let uri = if let Some(uri) = shared.get(&channel_key) { uri.clone() } else {
                    let image = bake(source, role)?;
                    let uri = store_image(&image, format!("prepared/material-channels/{}/{role}.ktx2", shared.len()), srgb, role, pack)?;
                    shared.insert(channel_key, uri.clone());
                    uri
                };
                channels.push(json!({"uri":uri,"role":role,"layer":0,"color_space":if srgb { "srgb" } else { "linear" },"sampler":binding["sampler"]}));
            }
            if hash == 887934131 || specular == 2 && hash == 2695565377 {
                let channel_key = (image_name.to_ascii_lowercase(), "native-specular".into());
                let uri = if let Some(uri) = shared.get(&channel_key) { uri.clone() } else {
                    let uri = store_image(source, format!("prepared/material-channels/{}/native-specular.ktx2", shared.len()), false, "specular_gloss", pack)?;
                    shared.insert(channel_key, uri.clone());
                    uri
                };
                native_specular = json!({"uri":uri,"gloss_channel":"alpha","specular_rgb":if specular == 2 { "constant_0.04" } else { "gamma2" }});
            }
        }
        if specular == 1 {
            let bindings = material["textures"].as_array().unwrap();
            let color = bindings.iter().find(|value| value["hash"] == 2695565377_u32).unwrap();
            let specular = bindings.iter().find(|value| value["hash"] == 887934131_u32).unwrap();
            let names = [color["image"].as_str().unwrap().to_ascii_lowercase(),specular["image"].as_str().unwrap().to_ascii_lowercase()];
            let samplers = [color["sampler"].as_u64().unwrap() as u8,specular["sampler"].as_u64().unwrap() as u8];
            let pair = format!("{}:{}:{}:{}",names[0],names[1],samplers[0],samplers[1]);
            if !shared.contains_key(&(pair.clone(),"color".into())) {
                let (base,response) = metallic_workflow(&images[&names[0]],&images[&names[1]],samplers)?;
                for (role,image) in [("color",base),("response",response)] {
                    let path = format!("prepared/material-channels/{}/{role}.ktx2",shared.len());
                    let path = store_image(&image,path,role=="color",role,pack)?;
                    shared.insert((pair.clone(),role.into()),path);
                }
            }
            for role in ["color","response"] {
                channels.push(json!({"uri":shared[&(pair.clone(),role.into())],"role":role,"layer":0,
                    "color_space":if role == "color" {"srgb"} else {"linear"},"sampler":samplers[0]}));
            }
        }
        let state_index = material["state_entries"][4].as_u64().ok_or("Missing native lit state index")? as usize;
        let state = material["state_bits"][state_index][0].as_u64().ok_or("Native lit state index exceeds state table")? as u32;
        let (source, destination, operation) = (state & 15, state >> 4 & 15, state >> 8 & 7);
        let alpha = if coverage == 2 && operation == 0 { "mask" } else if operation == 0 { "opaque" } else if (source, destination, operation) == (2, 6, 1) { "premultiplied" }
            else { return Err(format!("Unreviewed ordinary lit blend state for {key}: {state:#010x}").into()); };
        let cull = match state >> 14 & 3 { 1 => "none", 2 => "back", 3 => "front", _ => return Err(format!("Unknown native cull state for {key}").into()) };
        converted.insert(key.clone(), json!({"name":key,"pbrMetallicRoughness":{"baseColorFactor":[1.,1.,1.,1.],"metallicFactor":0.},
            "extras":{"surface":{"version":3,"identity":key,"channels":channels,"native":material,
                "standard":{"roughness":1.,"metallic":if specular == 0 { 0. } else { 1. },"reflectance":if specular == 0 { 0. } else { 0.5 },"specular_tint":[1.,1.,1.],"base_color":[1.,1.,1.,1.]},
                "presentation":{"alpha":alpha,"cutoff":128./255.,"cull":cull,"fog":true,"visible":state & (1 << 27) == 0},
                "source_vertex_color_transfer":"gamma2","source_coverage":match coverage { 1 => "gamma2_premultiplied", 2 => "mask", _ => "opaque" },
                "native_specular":native_specular,
                "approximations":["Khronos specular-glossiness to metallic-roughness fit for shared upstream PBR and Solari inputs", "Native Blinn-Phong gloss fitted to GGX perceptual roughness; original specular and gloss retained", "Slope reconstruction baked before filtering", "Native lighting-term multipliers retained for runtime integration", "Vertex gamma2 transfer evaluated before interpolation"]}}}));
        if converted.len() % 25 == 0 { println!("Converted {} reviewed materials with {} shared texture channels",converted.len(),shared.len()); }
    }
    println!("Converted {} source-reviewed lit materials; {} other technique families remain", converted.len(), missing.len());
    Ok(converted)
}

fn gaps(native: &BTreeMap<String, Value>, covered: impl Fn(&str,&Value) -> bool) -> Vec<String> {
    let mut families = BTreeMap::<&str, Vec<&str>>::new();
    for (name, material) in native {
        if !covered(name,material) { families.entry(material["technique_set"].as_str().unwrap()).or_default().push(name); }
    }
    families.into_iter().map(|(family, names)| format!("{family}: {} used materials, first {}", names.len(), names[0])).collect()
}

pub fn missing(native: &BTreeMap<String, Value>, converted: &BTreeMap<String, Value>) -> Vec<String> {
    gaps(native,|name,_| converted.contains_key(name))
}

pub fn unimplemented(native: &BTreeMap<String, Value>) -> Vec<String> {
    gaps(native,|_,material| {
        let family = material["technique_set"].as_str().unwrap();
        ordinary_recipe(family).is_some() || crate::bake::supported(family)
    })
}

pub fn read(catalog: &Catalog<'_>, name: &str) -> Result<Value> {
    let (zone, root) = catalog.resolve(6, name)?;
    let streams = &catalog.zones[zone].1.streams;
    let (_, technique_zone, technique) = catalog.pointer(zone, root.add(92)?, 7)?;
    let technique_streams = &catalog.zones[technique_zone].1.streams;
    let family = technique_streams.text(Address::decode(technique_streams.u32(technique)?)?)?;
    let mut techniques = Vec::new();
    for index in [2, 4, 6] {
        let value = technique_streams.u32(technique.add(8 + index * 4)?)?;
        if value == 0 { continue; }
        let native = Address::decode(value)?;
        let mut passes = Vec::new();
        for pass in 0..technique_streams.u16(native.add(6)?)? as usize {
            let pass = native.add(8 + pass * 24)?;
            let vertex = Address::decode(technique_streams.u32(pass.add(4)?)?)?;
            let vertex_shader = technique_streams.text(Address::decode(technique_streams.u32(vertex)?)?)?;
            let declaration = Address::decode(technique_streams.u32(pass)?)?;
            let stream_count = technique_streams.u8(declaration)? as usize;
            if stream_count > 16 { return Err(format!("Native vertex declaration exceeds sixteen routes: {name}").into()); }
            let routes = (0..stream_count).map(|index| -> Result<Value> {
                let route = declaration.add(4 + index * 2)?;
                Ok(json!({"source":technique_streams.u8(route)?,"destination":technique_streams.u8(route.add(1)?)?}))
            }).collect::<Result<Vec<_>>>()?;
            let pixel = Address::decode(technique_streams.u32(pass.add(8)?)?)?;
            let shader = technique_streams.text(Address::decode(technique_streams.u32(pixel)?)?)?;
            let count = (12..15).map(|offset| -> Result<usize> { Ok(technique_streams.u8(pass.add(offset)?)? as usize) }).collect::<Result<Vec<_>>>()?.into_iter().sum::<usize>();
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
                        let literal: [f32; 4] = std::array::from_fn(|index| f32::from_bits(u32_at(literal, index * 4)));
                        if !literal.iter().all(|value| value.is_finite()) { return Err(format!("Non-finite native shader literal: {name}").into()); }
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
            textures.push(json!({"hash":streams.u32(item)?,"sampler":streams.u8(item.add(6)?)?,
                "semantic":streams.u8(item.add(7)?)?,"image":image}));
        }
    }
    let mut constants = Vec::new();
    let constant_count = streams.u8(root.add(85)?)? as usize;
    if constant_count != 0 {
        let table = Address::decode(streams.u32(root.add(100)?)?)?;
        for index in 0..constant_count {
            let item = streams.read(table.add(index * 32)?, 32)?;
            let literal: [f32; 4] = std::array::from_fn(|i| f32::from_bits(u32_at(item, 16 + i * 4)));
            if !literal.iter().all(|value| value.is_finite()) { return Err(format!("Non-finite native material constant: {name}").into()); }
            let short_name = std::str::from_utf8(item[4..16].split(|b| *b == 0).next().unwrap())?;
            constants.push(json!({"hash":u32_at(item, 0),"name":short_name,"literal":literal}));
        }
    }
    let mut states = Vec::new();
    let state_count = streams.u8(root.add(86)?)? as usize;
    if state_count != 0 {
        let table = Address::decode(streams.u32(root.add(104)?)?)?;
        for index in 0..state_count {
            states.push(json!([streams.u32(table.add(index * 20)?)?, streams.u32(table.add(index * 20 + 4)?)?]));
        }
    }
    Ok(json!({"name":name,"technique_set":family,"techniques":techniques,"textures":textures,"constants":constants,
        "state_entries":streams.read(root.add(48)?, 36)?,"state_bits":states}))
}

pub fn read_used(catalog: &Catalog<'_>, names: &std::collections::BTreeSet<String>) -> Result<BTreeMap<String, Value>> {
    let mut materials = BTreeMap::new();
    let mut families = BTreeMap::<String, usize>::new();
    for name in names {
        let material = read(catalog, name)?;
        *families.entry(material["technique_set"].as_str().ok_or("Native material technique name is missing")?.into()).or_default() += 1;
        materials.insert(name.to_ascii_lowercase(), material);
    }
    println!("Read {} used native materials in {} technique families", materials.len(), families.len());
    Ok(materials)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn implemented_dispatch_does_not_satisfy_converted_material_closure() {
        let native = BTreeMap::from([
            ("ordinary".into(),json!({"technique_set":"mc_lit_sm_r0c0n0_2z223015"})),
            ("layered".into(),json!({"technique_set":"lit_sm_r0c0n0_b1c1n1v1"})),
            ("unknown".into(),json!({"technique_set":"unreviewed_native_family"})),
        ]);
        assert_eq!(unimplemented(&native),["unreviewed_native_family: 1 used materials, first unknown"]);
        assert_eq!(missing(&native,&BTreeMap::new()).len(),3);
    }
}

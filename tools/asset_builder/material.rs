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

struct OrdinaryRecipe {
    expected: &'static str,
    normal: bool,
    specular: u8,
    coverage: u8,
    programs: Option<[&'static str;3]>,
}

fn ordinary_recipe(family: &str) -> Option<OrdinaryRecipe> {
    let (expected,normal,specular,coverage) = match family {
        "mc_lit_sm_b0c0n0s0_21fz2e61" => ("pimp_shader_lprobe_6741885c.hlsl", true, 1, 1),
        "mc_lit_sm_b0c0n0s0_91j5z9q9" => ("pimp_shader_lprobe_8bb4c6fc.hlsl", true, 1, 1),
        "wpc_lit_sm_r0c0n0s0_1zzj1138" => ("pimp_shader_lmap_353f0d80.hlsl", true, 1, 0),
        "mc_lit_sm_r0c0n0s0_zqq1fze7" => ("pimp_shader_lprobe_4bceb0cc.hlsl", true, 1, 0),
        "mc_lit_sm_r0c0n0x0_q361191u" => ("pimp_shader_lprobe_f9c3b462.hlsl", true, 2, 0),
        "wpc_lit_sm_r0c0n0x0_f488034e" => ("pimp_shader_lmap_9a71a1e7.hlsl", true, 2, 0),
        "mc_lit_sm_r0c0_77e21qq8" => ("pimp_shader_lprobe_9c5921d1.hlsl", false, 0, 0),
        "mc_lit_sm_r0c0n0_2z223015" => ("pimp_shader_lprobe_a06c1664.hlsl", true, 0, 0),
        "mc_lit_sm_t0c0n0s0_47jq982j" => ("pimp_shader_lprobe_b646e86e.hlsl", true, 1, 2),

        "mc_lit_sm_r0c0s0_986ezzjq" => ("pimp_shader_lprobe_77fb8779.hlsl", false, 1, 0),
        "wpc_lit_sm_r0c0n0_80fe30wz" => ("pimp_shader_lmap_c6b996da.hlsl", true, 0, 0),
        "mc_lit_sm_t0c0_90wz6fe2" => ("pimp_shader_lprobe_61d17973.hlsl", false, 0, 2),
        "wpc_lit_sm_t0c0_2w3887e6" => ("pimp_shader_lmap_2d3e6f96.hlsl", false, 0, 2),
        "wpc_lit_sm_t0c0n0_9ew0jz24" => ("pimp_shader_lmap_439f5633.hlsl", true, 0, 2),
        "mc_lit_sm_t0c0n0_9qf6e4qj" => ("pimp_shader_lprobe_5de44ec6.hlsl", true, 0, 2),
        "wpc_lit_sm_t0c0n0s0_weef63f3" => ("pimp_shader_lmap_b019cd69.hlsl", true, 1, 2),
        "mc_lit_sm_b0c0_fw0jf955" => ("pimp_shader_lprobe_5c2357e1.hlsl", false, 0, 1),
        "mc_lit_sm_b0c0s0_805w75j2" => ("pimp_shader_lprobe_b781f149.hlsl", false, 1, 1),
        "wpc_lit_sm_b0c0n0_19f3588f" => ("pimp_shader_lmap_4f8f060e.hlsl", true, 0, 1),
        "wpc_lit_sm_b0c0_6j30z67q" => ("pimp_shader_lmap_212e3fab.hlsl", false, 0, 1),
        "mc_lit_sm_b0c0n0_67fqj7jf" => ("pimp_shader_lprobe_60166054.hlsl", true, 0, 1),
        "wpc_lit_sm_r0c0s0_qfqz7850" => ("pimp_shader_lmap_5b9e3425.hlsl", false, 1, 0),
        "wpc_lit_sm_r0c0x0_5j63fj97" => ("pimp_shader_lmap_f4d09842.hlsl", false, 2, 0),
        "wpc_lit_sm_b0c0s0_3f3q946z" => ("pimp_shader_lmap_d2a8a4f1.hlsl", false, 1, 1),
        _ => return None,
    };
    let programs = match family {
        "mc_lit_sm_b0c0n0s0_21fz2e61" => Some(["pimp_shader_lprobe_347155f7.hlsl","pimp_shader_lprobe_22ee4c8.hlsl","pimp_shader_lprobe_3ea3f969.hlsl"]),
        "mc_lit_sm_r0c0s0_986ezzjq" => Some(["pimp_shader_lprobe_eb01efa5.hlsl","pimp_shader_lprobe_dd5e5e9a.hlsl","pimp_shader_lprobe_2e19f64c.hlsl"]),
        "wpc_lit_sm_r0c0n0_80fe30wz" => Some(["pimp_shader_lmap_d107eac0.hlsl","pimp_shader_lmap_5ab92038.hlsl","pimp_shader_lmap_b4969fec.hlsl"]),
        "mc_lit_sm_t0c0_90wz6fe2" => Some(["pimp_shader_lprobe_eb01efa5.hlsl","pimp_shader_lprobe_dd5e5e9a.hlsl","pimp_shader_lprobe_38330846.hlsl"]),
        "wpc_lit_sm_t0c0_2w3887e6" => Some(["pimp_shader_lmap_bfa6d365.hlsl","pimp_shader_lmap_3418199d.hlsl","pimp_shader_lmap_5f1166a0.hlsl"]),
        "wpc_lit_sm_t0c0n0_9ew0jz24" => Some(["pimp_shader_lmap_d107eac0.hlsl","pimp_shader_lmap_5ab92038.hlsl","pimp_shader_lmap_31b05f05.hlsl"]),
        "mc_lit_sm_t0c0n0_9qf6e4qj" => Some(["pimp_shader_lprobe_347155f7.hlsl","pimp_shader_lprobe_22ee4c8.hlsl","pimp_shader_lprobe_4063ff3.hlsl"]),
        "wpc_lit_sm_t0c0n0s0_weef63f3" => Some(["pimp_shader_lmap_b89a2252.hlsl","pimp_shader_lmap_3324e8aa.hlsl","pimp_shader_lmap_c236c45f.hlsl"]),
        "mc_lit_sm_b0c0_fw0jf955" => Some(["pimp_shader_lprobe_eb01efa5.hlsl","pimp_shader_lprobe_dd5e5e9a.hlsl","pimp_shader_lprobe_5c126d4.hlsl"]),
        "mc_lit_sm_b0c0s0_805w75j2" => Some(["pimp_shader_lprobe_eb01efa5.hlsl","pimp_shader_lprobe_dd5e5e9a.hlsl","pimp_shader_lprobe_ee63807c.hlsl"]),
        "wpc_lit_sm_b0c0n0_19f3588f" => Some(["pimp_shader_lmap_d107eac0.hlsl","pimp_shader_lmap_5ab92038.hlsl","pimp_shader_lmap_3da00f38.hlsl"]),
        "wpc_lit_sm_b0c0_6j30z67q" => Some(["pimp_shader_lmap_bfa6d365.hlsl","pimp_shader_lmap_3418199d.hlsl","pimp_shader_lmap_5301369d.hlsl"]),
        "mc_lit_sm_b0c0n0_67fqj7jf" => Some(["pimp_shader_lprobe_347155f7.hlsl","pimp_shader_lprobe_22ee4c8.hlsl","pimp_shader_lprobe_39f41161.hlsl"]),
        "wpc_lit_sm_r0c0s0_qfqz7850" => Some(["pimp_shader_lmap_d63b1bf7.hlsl","pimp_shader_lmap_5d85d10f.hlsl","pimp_shader_lmap_29b13d13.hlsl"]),
        "wpc_lit_sm_r0c0x0_5j63fj97" => Some(["pimp_shader_lmap_d63b1bf7.hlsl","pimp_shader_lmap_5d85d10f.hlsl","pimp_shader_lmap_86ff9174.hlsl"]),
        "wpc_lit_sm_b0c0s0_3f3q946z" => Some(["pimp_shader_lmap_d63b1bf7.hlsl","pimp_shader_lmap_5d85d10f.hlsl","pimp_shader_lmap_a087adc7.hlsl"]),
        _ => None,
    };
    Some(OrdinaryRecipe {expected,normal,specular,coverage,programs})
}

fn validate_ordinary(key: &str, material: &Value, recipe: &OrdinaryRecipe) -> Result<()> {
    let OrdinaryRecipe {expected,normal,specular,coverage,programs} = *recipe;
    let family = material["technique_set"].as_str().ok_or("Missing native material technique")?;
    let techniques = material["techniques"].as_array().ok_or("Missing native material techniques")?;
    let lit = techniques.iter().find(|technique| technique["index"] == 4).ok_or("Ordinary lit technique is missing")?;
    let passes = lit["passes"].as_array().ok_or("Native technique pass array is missing")?;
    if passes.len() != 1 { return Err(format!("Ordinary lit material {key} has unsupported pass count").into()); }
    let shader = passes[0]["pixel_shader"].as_str().ok_or("Native pixel shader name is missing")?;
    if shader != expected { return Err(format!("Native technique {family} has an unreviewed pixel shader: {shader}").into()); }
    if let Some([vertex,sun_vertex,sun_pixel]) = programs {
        let sun = techniques.iter().find(|technique| technique["index"] == 6)
            .ok_or_else(|| format!("Reviewed material {key} has no sun technique"))?;
        let sun_passes = sun["passes"].as_array().ok_or("Native sun pass array is missing")?;
        if passes[0]["vertex_shader"] != vertex || sun_passes.len() != 1
            || sun_passes[0]["vertex_shader"] != sun_vertex || sun_passes[0]["pixel_shader"] != sun_pixel {
            return Err(format!("Reviewed material {key} has unreviewed vertex or sun programs").into());
        }
        let lit_state = material["state_entries"][4].as_u64().ok_or("Reviewed lit state index is missing")? as usize;
        let sun_state = material["state_entries"][6].as_u64().ok_or("Reviewed sun state index is missing")? as usize;
        if material["state_bits"].get(lit_state).is_none() || material["state_bits"].get(sun_state).is_none()
            || material["state_bits"][lit_state] != material["state_bits"][sun_state] {
            return Err(format!("Reviewed material {key} has differing lit/sun render states").into());
        }
        let state = &material["state_bits"][lit_state];
        let rgb = state[0].as_u64().ok_or("Missing reviewed RGB state")?;
        let depth = state[1].as_u64().ok_or("Missing reviewed depth state")?;
        if (coverage == 1 && rgb & 0x7ff != 0x162) || (coverage != 1 && rgb >> 8 & 7 != 0)
            || (coverage == 1 && ![12,28].contains(&depth)) || (coverage != 1 && ![13,29].contains(&depth)) {
            return Err(format!("Reviewed material {key} has unreviewed coverage or depth state").into());
        }
        let mut routes = vec![json!({"source":0,"destination":0}),json!({"source":1,"destination":2}),
            json!({"source":2,"destination":5}),json!({"source":3,"destination":1})];
        if normal { routes.push(json!({"source":4,"destination":7})); }
        if family.starts_with("wpc_") { routes.push(json!({"source":5,"destination":6})); }
        let mut inputs = BTreeMap::from([(2695565377_u32,0_u64)]);
        if normal { inputs.insert(1507003663,257); }
        if specular == 1 { inputs.insert(887934131,if normal {514} else {257}); }
        for pass in [&passes[0],&sun_passes[0]] {
            if pass["vertex_routes"] != json!(routes) { return Err(format!("Reviewed material {key} has unreviewed vertex routes").into()); }
            let arguments = pass["arguments"].as_array().ok_or("Reviewed pass arguments are missing")?;
            let active: BTreeMap<_,_> = arguments.iter().filter(|argument| argument["type"] == 2)
                .map(|argument| Ok((argument["value"].as_u64().ok_or("Missing shader image hash")? as u32,
                    argument["location"].as_u64().ok_or("Missing shader image location")?)))
                .collect::<Result<_>>()?;
            if active != inputs { return Err(format!("Reviewed material {key} has unreviewed active texture bindings").into()); }
            for argument in arguments.iter().filter(|argument| matches!(argument["type"].as_u64(),Some(0 | 1 | 6 | 7 | 8))) {
                if argument["type"] != 6 || argument["value"] != 2418533825_u32 || argument["buffer"] != 1
                    || argument["location"] != 672 || argument["size"] != 12 {
                    return Err(format!("Reviewed material {key} has an unreviewed active material constant").into());
                }
                let constant = material["constants"].as_array().ok_or("Missing native constant table")?.iter()
                    .find(|constant| constant["hash"] == 2418533825_u32).ok_or("Missing bound occlusionAmount")?;
                if (0..3).any(|component| constant["literal"][component].as_f64().is_none_or(|value| !value.is_finite())) {
                    return Err(format!("Reviewed material {key} has invalid direct/probe/diffuse lighting multipliers").into());
                }
            }
        }
        for hash in inputs.keys() {
            let binding = material["textures"].as_array().ok_or("Missing native textures")?.iter()
                .find(|binding| binding["hash"] == *hash).ok_or_else(|| format!("Reviewed material {key} lacks active image {hash}"))?;
            if !matches!(binding["sampler"].as_u64(),Some(20 | 244)) { return Err(format!("Reviewed material {key} has an unreviewed active sampler for {hash}").into()); }
        }
    }
    Ok(())
}

fn unlit_recipe(family: &str) -> Option<(&'static str,&'static str)> {
    match family {
        "mc_unlit_replace_3j970129" | "wpc_unlit_replace_4688792e" => Some(("pimp_shader_unlit_49c3292c.hlsl","opaque")),
        "mc_unlit_add_ww82zw88" | "wpc_unlit_add_8ez3wzw3" => Some(("pimp_shader_unlit_4b1d2e0b.hlsl","add")),
        "mc_unlit_blend_588ewzff" => Some(("pimp_shader_unlit_4ea12045.hlsl","blend")),
        _ => None,
    }
}

fn validate_unlit(key: &str, material: &Value, pixel: &str, alpha: &str) -> Result<f32> {
    let techniques = material["techniques"].as_array().ok_or("Missing native unlit techniques")?;
    let technique = techniques.iter().find(|technique| technique["index"] == 2).ok_or("Missing native unlit technique")?;
    let passes = technique["passes"].as_array().ok_or("Missing native unlit passes")?;
    if passes.len() != 1 || passes[0]["pixel_shader"] != pixel || passes[0]["vertex_shader"] != "pimp_shader_unlit_4876d431.hlsl"
        || passes[0]["vertex_routes"] != json!([{"source":0,"destination":0},{"source":1,"destination":2},{"source":2,"destination":5}]) {
        return Err(format!("Unlit material {key} has unreviewed programs or vertex routes").into());
    }
    let arguments = passes[0]["arguments"].as_array().ok_or("Missing unlit arguments")?;
    let active: Vec<_> = arguments.iter().filter(|argument| matches!(argument["type"].as_u64(),Some(0 | 1 | 2 | 6 | 7 | 8))).collect();
    if active.len() != 2 || !active.iter().any(|argument| argument["type"] == 2 && argument["value"] == 2695565377_u32
        && argument["location"] == 0 && argument["size"] == 1 && argument["buffer"] == 0)
        || !active.iter().any(|argument| argument["type"] == 6 && argument["value"] == 3799286735_u32
            && argument["location"] == 16 && argument["size"] == 4 && argument["buffer"] == 2) {
        return Err(format!("Unlit material {key} has unreviewed active bindings").into());
    }
    let texture = material["textures"].as_array().ok_or("Missing unlit textures")?.iter()
        .find(|texture| texture["hash"] == 2695565377_u32).ok_or("Missing unlit color texture")?;
    if !matches!(texture["sampler"].as_u64(),Some(20 | 244)) { return Err(format!("Unlit material {key} has an unreviewed sampler").into()); }
    let constant = material["constants"].as_array().ok_or("Missing unlit constants")?.iter()
        .find(|constant| constant["hash"] == 3799286735_u32).ok_or("Missing bound scaleRGB")?;
    let scale = constant["literal"][0].as_f64().ok_or("Missing scaleRGB value")? as f32;
    if !scale.is_finite() || scale < 0. { return Err(format!("Unlit material {key} has invalid scaleRGB").into()); }
    let index = material["state_entries"][2].as_u64().ok_or("Missing native unlit state index")? as usize;
    let state = material["state_bits"][index][0].as_u64().ok_or("Missing native unlit state")?;
    let depth = material["state_bits"][index][1].as_u64().ok_or("Missing native unlit depth state")?;
    let (rgb,coverage) = match alpha { "opaque" => (0x12,0x12),"add" => (0x122,0x121),"blend" => (0x165,0x128),_ => unreachable!() };
    if state & 0x7ff != rgb || state >> 16 & 0x7ff != coverage || depth & !0x30 != if alpha == "opaque" {13} else {12} {
        return Err(format!("Unlit material {key} has unreviewed blend or depth state").into());
    }
    Ok(scale)
}

fn unlit_color(source: &texture::Image, scale: f32, alpha: &str) -> Result<texture::Image> {
    if source.depth != 1 || source.faces != 1 || source.width == 0 || source.height == 0 || source.levels.is_empty() {
        return Err("Unlit color input is not a valid 2D image".into());
    }
    let levels = (0..source.levels.len()).map(|level| -> Result<Vec<[f32;4]>> {
        texture::pixels(source,level)?.into_iter().map(|pixel| {
            let coverage = if alpha == "blend" {pixel[3]} else {1.};
            let value: [f32;4] = std::array::from_fn(|c| if c < 3 { pixel[c]*pixel[c]*scale*coverage } else if alpha == "add" {1.} else {pixel[3]});
            if value.iter().any(|value| !value.is_finite() || !(0. ..=65504.).contains(value)) || value[3] > 1. {
                return Err("Unlit converted color exceeds the finite half-float/coverage domain".into());
            }
            Ok(value)
        }).collect()
    }).collect::<Result<Vec<_>>>()?;
    let hdr = levels.iter().flatten().any(|pixel| pixel[..3].iter().any(|&value| value > 1.));
    let mut image = source.clone();
    image.format = if hdr {19} else {1};
    image.levels = levels.into_iter().map(|pixels| pixels.into_iter().flat_map(|pixel| {
        if hdr { pixel.into_iter().flat_map(|value| half::f16::from_f32(value).to_le_bytes()).collect::<Vec<_>>() }
        else { pixel.into_iter().enumerate().map(|(c,value)| {
            let value = if c == 3 {value} else if value <= 0.0031308 {value*12.92} else {1.055*value.powf(1./2.4)-0.055};
            (value*255.).round() as u8
        }).collect() }
    }).collect()).collect();
    Ok(image)
}

fn convert_unlit(key: &str, native: &Value, pixel: &str, alpha: &str, images: &BTreeMap<String,texture::Image>, shared: &mut BTreeMap<(String,String),String>, pack: &mut Pack) -> Result<Value> {
    let scale = validate_unlit(key,native,pixel,alpha)?;
    let binding = native["textures"].as_array().unwrap().iter().find(|binding| binding["hash"] == 2695565377_u32).unwrap();
    let name = binding["image"].as_str().ok_or("Missing unlit image name")?.to_ascii_lowercase();
    let source = images.get(&name).ok_or_else(|| format!("Missing unlit color image {name} for {key}"))?;
    let image = unlit_color(source,scale,alpha)?;
    let linear = image.format == 19;
    let channel_key = (name,format!("unlit-{}-{alpha}",scale.to_bits()));
    let uri = if let Some(uri) = shared.get(&channel_key) {uri.clone()} else {
        let uri = store_image(&image,format!("prepared/material-channels/{}/color.ktx2",shared.len()),!linear,"color",pack)?;
        shared.insert(channel_key,uri.clone()); uri
    };
    let index = native["state_entries"][2].as_u64().unwrap() as usize;
    let state = native["state_bits"][index][0].as_u64().unwrap();
    let cull = match state >> 14 & 3 {1 => "none",2 => "back",3 => "front",_ => return Err("Unknown unlit cull state".into())};
    Ok(json!({"name":key,"pbrMetallicRoughness":{"baseColorFactor":[1.,1.,1.,1.],"metallicFactor":0.},
        "extras":{"surface":{"version":3,"identity":key,"native":native,
            "channels":[{"uri":uri,"role":"color","layer":0,"color_space":if linear {"linear"} else {"srgb"},"sampler":binding["sampler"]}],
            "standard":{"unlit":true,"roughness":1.,"metallic":0.,"reflectance":0.,"base_color":[1.,1.,1.,1.]},
            "presentation":{"alpha":alpha,"cull":cull,"fog":true,"fog_mode":if alpha == "add" {"attenuation"} else {"color"},"visible":state & (1<<27) == 0},
            "source_vertex_color_transfer":if alpha == "blend" {"gamma2_alpha"} else {"gamma2"},
            "source_coverage":if alpha == "add" {"unlit_add"} else if alpha == "blend" {"unlit_blend"} else {"opaque"},
            "pending_runtime":if alpha == "add" {json!("Native ONE/ONE addition requires fog attenuation without adding fog color")} else {Value::Null},
            "approximations":["Vertex gamma2 transfer evaluated before interpolation", "Source gamma2 framebuffer composition represented in the shared linear HDR pipeline", "Source polygon-offset modes retained for runtime integration"]}}}))
}

pub fn inputs(native: &BTreeMap<String, Value>, catalog: &Catalog<'_>) -> Result<std::collections::BTreeSet<String>> {
    let mut required = std::collections::BTreeSet::new();
    for (key,material) in native {
        let family = material["technique_set"].as_str().ok_or("Missing native material technique")?;
        let index = if let Some((pixel,alpha)) = unlit_recipe(family) { validate_unlit(key,material,pixel,alpha)?; 2 } else {
            let recipe = ordinary_recipe(family).ok_or_else(|| format!("No reviewed recipe for {key}: {family}"))?;
            validate_ordinary(key,material,&recipe)?; 4
        };
        let lit = material["techniques"].as_array().unwrap().iter().find(|technique| technique["index"] == index).unwrap();
        for argument in lit["passes"][0]["arguments"].as_array().unwrap().iter().filter(|argument| argument["type"] == 2) {
            let binding = material["textures"].as_array().unwrap().iter().find(|binding| binding["hash"] == argument["value"])
                .ok_or_else(|| format!("Reviewed material {key} lacks active texture {}",argument["value"]))?;
            let name = binding["image"].as_str().ok_or("Missing active image name")?;
            let (zone,root) = catalog.resolve(8,name)?;
            let streams = &catalog.zones[zone].1.streams;
            if streams.u16(root.add(20)?)? == 0 || streams.u16(root.add(22)?)? == 0
                || streams.u16(root.add(24)?)? != 1 || streams.u8(root.add(4)?)? == 5 {
                return Err(format!("Reviewed material {key} active image {name} is not a valid 2D image").into());
            }
            required.insert(name.to_ascii_lowercase());
        }
    }
    Ok(required)
}

pub fn convert_known(native: &BTreeMap<String, Value>, images: &BTreeMap<String, texture::Image>, pack: &mut Pack) -> Result<BTreeMap<String, Value>> {
    let mut converted = BTreeMap::new();
    let mut missing = BTreeMap::<String, Vec<String>>::new();
    let mut shared = BTreeMap::<(String, String), String>::new();
    for (key, material) in native {
        let family = material["technique_set"].as_str().ok_or("Missing native material technique")?;
        if let Some((pixel,alpha)) = unlit_recipe(family) {
            converted.insert(key.clone(),convert_unlit(key,material,pixel,alpha,images,&mut shared,pack)?);
            continue;
        }
        let Some(recipe) = ordinary_recipe(family) else {
            missing.entry(family.into()).or_default().push(key.clone()); continue;
        };
        validate_ordinary(key,material,&recipe)?;
        let OrdinaryRecipe {normal,specular,coverage,..} = recipe;
        let lit = material["techniques"].as_array().unwrap().iter().find(|technique| technique["index"] == 4).unwrap();
        let passes = lit["passes"].as_array().unwrap();
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
        if family == "mc_lit_sm_b0c0n0s0_21fz2e61" {
            converted.get_mut(key).unwrap()["extras"]["surface"]["source_fragment_alpha"] = json!("sqrt_texture_vertex_coverage");
            converted.get_mut(key).unwrap()["extras"]["surface"]["pending_runtime"] = json!("Native glass fragment uses sqrt coverage and coverage-independent specular before gamma2 premultiplied composition; the shared linear PBR consumer does not yet reproduce that blend");
        }
        if converted.len() % 25 == 0 { println!("Converted {} reviewed materials with {} shared texture channels",converted.len(),shared.len()); }
    }
    println!("Converted {} source-reviewed materials; {} other technique families remain", converted.len(), missing.len());
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
    let mut missing = gaps(native,|name,_| converted.contains_key(name));
    missing.extend(native.keys().filter_map(|name| converted.get(name).and_then(|material| {
        material["extras"]["surface"]["pending_runtime"].as_str().map(|reason| format!("{name}: {reason}"))
    })));
    missing
}

pub fn unimplemented(native: &BTreeMap<String, Value>) -> Vec<String> {
    gaps(native,|_,material| {
        let family = material["technique_set"].as_str().unwrap();
        ordinary_recipe(family).is_some() || unlit_recipe(family).is_some() || crate::bake::supported(family)
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

#[derive(Clone,Debug,serde::Serialize)]
pub struct CamoPattern {
    pub images: [Option<String>;2],
    pub offset: [f32;2],
    pub scale: f32,
}

#[derive(Clone,Debug,serde::Serialize)]
pub struct CamoRecord {
    pub flags: u16,
    pub pairs: Vec<(Option<String>,Option<String>)>,
    pub constants: [f32;8],
}

#[derive(Clone,Debug,serde::Serialize)]
pub struct CamoDefinition {
    pub images: [Option<String>;2],
    pub patterns: Vec<CamoPattern>,
    pub material_sets: Vec<Vec<CamoRecord>>,
}

#[derive(Clone,Copy,Debug,serde::Serialize)]
pub struct CamoOption {
    pub selection: i32,
    pub material: bool,
}

fn camo_asset(catalog: &Catalog<'_>, zone: usize, field: Address, kind: u32) -> Result<Option<String>> {
    if catalog.zones[zone].1.streams.u32(field)? == 0 { return Ok(None); }
    let (_,owner,root) = catalog.pointer(zone,field,kind)?;
    let streams = &catalog.zones[owner].1.streams;
    let name = streams.text(Address::decode(streams.u32(root.add(if kind == 8 {72} else {0})?)?)?)?;
    Ok(Some(name.trim_start_matches(',').to_ascii_lowercase()))
}

pub fn read_camos(catalog: &Catalog<'_>) -> Result<BTreeMap<String,CamoDefinition>> {
    let mut result = BTreeMap::new();
    for (name,zone,root) in catalog.definitions(31) {
        let streams = &catalog.zones[zone].1.streams;
        let array = |field: Address, count: usize, stride: usize| -> Result<Option<Address>> {
            if count == 0 { return Ok(None); }
            let pointer = streams.u32(field)?;
            if pointer == 0 { return Err(format!("Camo {name} has a null positive-count array").into()); }
            let address = Address::decode(pointer)?;
            streams.read(address,count.checked_mul(stride).ok_or("Camo array size overflow")?)?;
            Ok(Some(address))
        };
        let scalar = |field: Address| -> Result<f32> {
            let value = f32::from_bits(streams.u32(field)?);
            if !value.is_finite() { return Err(format!("Camo {name} has a non-finite authored constant").into()); }
            Ok(value)
        };
        let images = [camo_asset(catalog,zone,root.add(4)?,8)?,camo_asset(catalog,zone,root.add(8)?,8)?];
        let count = streams.u32(root.add(16)?)? as usize;
        let mut patterns = Vec::new();
        if let Some(table) = array(root.add(12)?,count,20)? {
            for index in 0..count {
                let record = table.add(index*20)?;
                patterns.push(CamoPattern {images:[camo_asset(catalog,zone,record,8)?,camo_asset(catalog,zone,record.add(4)?,8)?],
                    offset:[scalar(record.add(8)?)?,scalar(record.add(12)?)?],scale:scalar(record.add(16)?)?});
            }
        }
        let count = streams.u32(root.add(24)?)? as usize;
        let mut material_sets = Vec::new();
        if let Some(table) = array(root.add(20)?,count,8)? {
            for index in 0..count {
                let set = table.add(index*8)?;
                let count = streams.u32(set)? as usize;
                let mut records = Vec::new();
                if let Some(table) = array(set.add(4)?,count,44)? {
                    for index in 0..count {
                        let record = table.add(index*44)?;
                        let count = streams.u16(record.add(2)?)? as usize;
                        let from = array(record.add(4)?,count,4)?;
                        let to = array(record.add(8)?,count,4)?;
                        let mut pairs = Vec::new();
                        for index in 0..count {
                            pairs.push((camo_asset(catalog,zone,from.ok_or("Missing source camo pair array")?.add(index*4)?,6)?,
                                camo_asset(catalog,zone,to.ok_or("Missing destination camo pair array")?.add(index*4)?,6)?));
                        }
                        let constants = (0..8).map(|index| scalar(record.add(12+index*4)?)).collect::<Result<Vec<_>>>()?
                            .try_into().map_err(|_| "Camo record does not contain eight constants")?;
                        records.push(CamoRecord {flags:streams.u16(record)?,pairs,constants});
                    }
                }
                material_sets.push(records);
            }
        }
        result.insert(name.to_ascii_lowercase(),CamoDefinition {images,patterns,material_sets});
    }
    if result.is_empty() { return Err("Required native camo definitions are missing".into()); }
    Ok(result)
}

pub fn read_camo_options(catalog: &Catalog<'_>) -> Result<Vec<Option<CamoOption>>> {
    let (zone,root) = catalog.resolve(42,"mp/weaponoptions_zm.csv")?;
    let streams = &catalog.zones[zone].1.streams;
    let columns = streams.u32(root.add(4)?)? as usize;
    let rows = streams.u32(root.add(8)?)? as usize;
    if columns < 5 { return Err("Native Zombies weapon-options table has fewer than five columns".into()); }
    let cells = Address::decode(streams.u32(root.add(12)?)?)?;
    let cell = |row: usize, column: usize| -> Result<String> {
        let pointer = streams.u32(cells.add((row*columns+column)*8)?)?;
        if pointer == 0 { Ok(String::new()) } else { Ok(streams.text(Address::decode(pointer)?)?) }
    };
    let mut options = vec![None;128];
    for row in 0..rows {
        if cell(row,1)? != "camo" { continue; }
        let id: usize = cell(row,0)?.parse()?;
        let selection = cell(row,4)?;
        if selection.is_empty() { continue; }
        let mode = cell(row,3)?;
        *options.get_mut(id).ok_or("Native camo option ID exceeds 127")? = Some(CamoOption {
            selection:selection.parse()?,material:!mode.is_empty() && mode.parse::<i32>()? != 0});
    }
    Ok(options)
}

pub fn camo_dependencies(camos: &BTreeMap<String,CamoDefinition>, options: &[Option<CamoOption>], packed_options: u32) -> std::collections::BTreeSet<String> {
    let mut names = std::collections::BTreeSet::new();
    let id = (packed_options&127) as usize;
    let Some(option) = options.get(id).copied().flatten().filter(|option| id != 0 && option.material) else { return names; };
    let Some(index) = option.selection.checked_sub(1).and_then(|value| usize::try_from(value).ok()) else { return names; };
    for camo in camos.values() {
        for record in camo.material_sets.get(index).into_iter().flatten() {
            for (from,to) in &record.pairs {
                if let (Some(from),Some(to)) = (from,to) && from != to { names.extend([from.clone(),to.clone()]); }
            }
        }
    }
    names
}

#[derive(Clone,Debug)]
pub enum CamoOverride {
    Image {mask:u32,scope:Option<String>,from:String,to:String},
    Material {mask:u32,from:String,to:String},
    Constant {mask:u32,scope:Option<String>,index:u32,value:[f32;4]},
}

fn queue_camo_image(queue: &mut Vec<CamoOverride>, mask: u32, scope: Option<&str>, from: Option<&str>, to: Option<&str>) -> bool {
    let (Some(from),Some(to)) = (from,to) else { return false; };
    if from == to || queue.len() >= 1024 { return false; }
    queue.push(CamoOverride::Image {mask,scope:scope.map(str::to_owned),from:from.into(),to:to.into()}); true
}

fn queue_camo_constant(queue: &mut Vec<CamoOverride>, mask: u32, scope: Option<&str>, index: u32, value: [f32;4]) {
    if queue.len() < 1024 { queue.push(CamoOverride::Constant {mask,scope:scope.map(str::to_owned),index,value}); }
}

pub fn queue_camo(camo: &CamoDefinition, options: &[Option<CamoOption>], packed_options: u32, mapped_distance: f32,
    mask: u32, materials: &BTreeMap<String,Value>, queue: &mut Vec<CamoOverride>) -> Result<()> {
    let id = (packed_options&127) as usize;
    let Some(option) = options.get(id).copied().flatten().filter(|_| id != 0 && mapped_distance < 1250.) else { return Ok(()); };
    let Some(index) = option.selection.checked_sub(1).and_then(|value| usize::try_from(value).ok()) else { return Ok(()); };
    if !option.material {
        if let Some(pattern) = camo.patterns.get(index)
            && queue_camo_image(queue,mask,None,camo.images[0].as_deref(),pattern.images[0].as_deref())
            && queue_camo_image(queue,mask,None,camo.images[1].as_deref(),pattern.images[1].as_deref()) {
            queue_camo_constant(queue,mask,None,181,[pattern.offset[0],pattern.offset[1],pattern.scale,1.]);
        }
        return Ok(());
    }
    for record in camo.material_sets.get(index).into_iter().flatten() {
        for (from,to) in &record.pairs {
            let (Some(from),Some(to)) = (from,to) else { continue; };
            if from == to || queue.len() >= 1024 { continue; }
            let source = materials.get(from).ok_or_else(|| format!("Camo source material {from} is missing"))?;
            let destination = materials.get(to).ok_or_else(|| format!("Camo destination material {to} is missing"))?;
            queue.push(CamoOverride::Material {mask,from:from.clone(),to:to.clone()});
            queue_camo_constant(queue,mask,Some(to),185,[record.constants[0],record.constants[1],record.constants[2],
                record.constants[3]+if record.flags&2 != 0 {10.} else {0.}]);
            for (flag,source_semantic,destination_semantic) in [(1,2,2),(2,5,13),(4,8,14)] {
                if record.flags&flag == 0 { continue; }
                for a in source["textures"].as_array().ok_or("Missing camo source texture table")?.iter().filter(|texture| texture["semantic"] == source_semantic) {
                    for b in destination["textures"].as_array().ok_or("Missing camo destination texture table")?.iter().filter(|texture| texture["semantic"] == destination_semantic) {
                        queue_camo_image(queue,mask,Some(to),b["image"].as_str(),a["image"].as_str());
                    }
                }
            }
        }
    }
    Ok(())
}

pub fn camo_material<'a>(queue: &'a [CamoOverride], original: &'a str, model_index: u32) -> &'a str {
    let bit = u32::from((1_u32<<(model_index&31)) as u16);
    queue.iter().rev().find_map(|command| if let CamoOverride::Material {mask,from,to} = command {
        (*mask&bit != 0 && from == original).then_some(to.as_str())
    } else {None}).unwrap_or(original)
}

pub fn camo_inputs(queue: &[CamoOverride], material: &Value, model_index: u32, initial: &BTreeMap<u32,[f32;4]>) -> Result<(Value,BTreeMap<u32,[f32;4]>)> {
    let mut result = material.clone();
    let name = material["name"].as_str().ok_or("Camo material identity is missing")?;
    let bit = u32::from((1_u32<<(model_index&31)) as u16);
    let mut constants = initial.clone();
    for texture in result["textures"].as_array_mut().ok_or("Missing selected camo texture table")? {
        texture["image"] = json!(texture["image"].as_str().ok_or("Missing selected camo image identity")?.to_ascii_lowercase());
    }
    for command in queue.iter().rev() {
        match command {
            CamoOverride::Image {mask,scope,from,to} if mask&bit != 0 && scope.as_deref().is_none_or(|scope| scope == name) => {
                for texture in result["textures"].as_array_mut().ok_or("Missing selected camo texture table")? {
                    if texture["image"] == *from { texture["image"] = json!(to); }
                }
            }
            CamoOverride::Constant {mask,scope,index,value} if mask&bit != 0 && scope.as_deref().is_none_or(|scope| scope == name) => {
                constants.insert(*index,*value);
            }
            _ => (),
        }
    }
    Ok((result,constants))
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
    fn camo_pattern_requires_both_image_swaps_and_keeps_partial_failure() {
        let mut options = vec![None;128];
        options[1] = Some(CamoOption {selection:1,material:false});
        let mut camo = CamoDefinition {images:[Some("solid".into()),Some("pattern".into())],
            patterns:vec![CamoPattern {images:[Some("red".into()),Some("stripes".into())],offset:[0.25,-0.5],scale:3.}],material_sets:Vec::new()};
        let mut queue = Vec::new();
        queue_camo(&camo,&options,129,0.,2,&BTreeMap::new(),&mut queue).unwrap();
        let material = json!({"name":"weapon","textures":[{"image":"solid"},{"image":"pattern"}]});
        let initial = BTreeMap::from([(181,[0.;4]),(185,[0.;4])]);
        let (bound,constants) = camo_inputs(&queue,&material,1,&initial).unwrap();
        assert_eq!(bound["textures"][0]["image"],"red");
        assert_eq!(bound["textures"][1]["image"],"stripes");
        assert_eq!(constants[&181],[0.25,-0.5,3.,1.]);
        assert_eq!(camo_inputs(&queue,&material,0,&initial).unwrap().1[&181],[0.;4]);
        camo.patterns[0].images[1]=Some("pattern".into());
        queue.clear();
        queue_camo(&camo,&options,1,0.,2,&BTreeMap::new(),&mut queue).unwrap();
        assert_eq!(queue.len(),1);
        let (bound,constants) = camo_inputs(&queue,&material,1,&initial).unwrap();
        assert_eq!(bound["textures"][0]["image"],"red");
        assert_eq!(bound["textures"][1]["image"],"pattern");
        assert_eq!(constants[&181],[0.;4]);
        camo.patterns[0].images[1]=Some("stripes".into());
        queue = vec![CamoOverride::Constant {mask:0,scope:None,index:181,value:[0.;4]};1023];
        queue_camo(&camo,&options,1,0.,2,&BTreeMap::new(),&mut queue).unwrap();
        assert_eq!(queue.len(),1024);
        assert_eq!(camo_inputs(&queue,&material,1,&initial).unwrap().1[&181],[0.;4]);
        for (id,distance) in [(0,0.),(1,1250.),(1,f32::NAN)] {
            queue.clear();
            queue_camo(&camo,&options,id,distance,2,&BTreeMap::new(),&mut queue).unwrap();
            assert!(queue.is_empty());
        }
    }

    #[test]
    fn camo_material_preserves_scope_semantics_and_native_consumer_order() {
        let mut options = vec![None;128];
        options[1] = Some(CamoOption {selection:1,material:true});
        let camo = CamoDefinition {images:[None,None],patterns:Vec::new(),material_sets:vec![vec![CamoRecord {
            flags:7,pairs:vec![(Some("a".into()),Some("b".into()))],constants:[6.,6.,0.55,0.,1.,2.,3.,4.]}]]};
        let materials = BTreeMap::from([
            ("a".into(),json!({"name":"a","textures":[{"semantic":2,"image":"basecolor"},{"semantic":5,"image":"basenormal"},{"semantic":8,"image":"basespec"}]})),
            ("b".into(),json!({"name":"b","textures":[{"semantic":2,"image":"camocolor"},{"semantic":13,"image":"camonormal"},{"semantic":14,"image":"camospec"}]})),
        ]);
        let mut queue = Vec::new();
        queue_camo(&camo,&options,1,0.,2,&materials,&mut queue).unwrap();
        assert_eq!(queue.len(),5);
        assert_eq!(camo_material(&queue,"a",1),"b");
        assert_eq!(camo_material(&queue,"a",0),"a");
        assert_eq!(camo_material(&queue,"a",17),"a");
        assert_eq!(camo_material(&queue,"a",33),"b");
        let initial = BTreeMap::from([(181,[0.;4]),(185,[0.;4])]);
        let (bound,constants) = camo_inputs(&queue,&materials["b"],1,&initial).unwrap();
        assert_eq!(constants[&185],[6.,6.,0.55,10.]);
        assert_eq!(constants[&181],[0.;4]);
        assert_eq!(bound["textures"].as_array().unwrap().iter().map(|texture| texture["image"].as_str().unwrap()).collect::<Vec<_>>(),["basecolor","basenormal","basespec"]);
        assert_eq!(camo_inputs(&queue,&materials["b"],17,&initial).unwrap().1[&185],[0.;4]);
        let other = json!({"name":"other","textures":[{"image":"camonormal"}]});
        assert_eq!(camo_inputs(&queue,&other,1,&initial).unwrap().0["textures"][0]["image"],"camonormal");
        queue.push(CamoOverride::Material {mask:2,from:"b".into(),to:"c".into()});
        assert_eq!(camo_material(&queue,"a",1),"b");
        queue.push(CamoOverride::Material {mask:2,from:"a".into(),to:"d".into()});
        assert_eq!(camo_material(&queue,"a",1),"d");
        queue.clear();
        queue_camo_constant(&mut queue,2,Some("b"),185,[1.;4]);
        queue_camo_constant(&mut queue,2,Some("b"),185,[2.;4]);
        queue_camo_image(&mut queue,2,Some("b"),Some("camocolor"),Some("red"));
        queue_camo_image(&mut queue,2,Some("b"),Some("red"),Some("blue"));
        let (bound,constants) = camo_inputs(&queue,&materials["b"],1,&initial).unwrap();
        assert_eq!(constants[&185],[1.;4]);
        assert_eq!(bound["textures"][0]["image"],"red");
        let empty = CamoDefinition {material_sets:vec![vec![CamoRecord {pairs:Vec::new(),flags:7,constants:[9.;8]}]],..camo};
        queue.clear();
        queue_camo(&empty,&options,1,0.,2,&materials,&mut queue).unwrap();
        assert!(queue.is_empty());
    }

    #[test]
    fn implemented_dispatch_does_not_satisfy_converted_material_closure() {
        let native = BTreeMap::from([
            ("ordinary".into(),json!({"technique_set":"mc_lit_sm_r0c0n0_2z223015"})),
            ("layered".into(),json!({"technique_set":"lit_sm_r0c0n0_b1c1n1v1"})),
            ("unknown".into(),json!({"technique_set":"unreviewed_native_family"})),
        ]);
        assert_eq!(unimplemented(&native),["unreviewed_native_family: 1 used materials, first unknown"]);
        assert_eq!(missing(&native,&BTreeMap::new()).len(),3);
        let pending = BTreeMap::from([("ordinary".into(),json!({"extras":{"surface":{"pending_runtime":"source fog operation"}}}))]);
        assert_eq!(missing(&native,&pending).len(),3);
        assert!(missing(&native,&pending).iter().any(|reason| reason == "ordinary: source fog operation"));
    }

    #[test]
    fn unlit_scale_and_coverage_preserve_hdr_and_source_alpha_roles() {
        let image = texture::Image {width:2,height:1,depth:1,faces:1,format:1,flags:0,
            levels:vec![vec![128,64,32,0,255,128,64,128],vec![128,128,128,255]]};
        let add = unlit_color(&image,50.,"add").unwrap();
        assert_eq!(add.format,19);
        assert_eq!(add.levels.len(),2);
        let pixels = texture::pixels(&add,0).unwrap();
        assert!((pixels[0][0]-12.59827).abs()<0.008);
        assert_eq!(pixels[0][3],1.);
        assert_eq!(pixels[1][0],50.);
        assert_eq!(pixels[1][3],1.);
        let blend = unlit_color(&image,50.,"blend").unwrap();
        let pixels = texture::pixels(&blend,0).unwrap();
        assert_eq!(pixels[0],[0.;4]);
        assert!((pixels[1][0]-25.09804).abs()<0.016);
        assert!((pixels[1][3]-128./255.).abs()<0.001);
        let replace = unlit_color(&image,50.,"opaque").unwrap();
        assert_eq!(texture::pixels(&replace,0).unwrap()[0][3],0.);
        let black = texture::Image {width:1,height:1,depth:1,faces:1,format:1,flags:0,levels:vec![vec![0,0,0,255]]};
        assert_eq!(unlit_color(&black,8.,"opaque").unwrap().format,1);
        assert!(unlit_color(&image,f32::NAN,"add").is_err());
    }
}

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

fn bake(source: &texture::Image, role: &str, detail: Option<(&texture::Image,u8,[f32;4])>) -> Result<texture::Image> {
    if source.depth != 1 || source.faces != 1 { return Err(format!("Ordinary lit {role} channel is not a 2D image").into()); }
    let detail_pixels = if let Some((image,_,parameters)) = detail {
        if role != "normal" || image.depth != 1 || image.faces != 1 || image.levels.is_empty()
            || !parameters.iter().all(|value| value.is_finite()) { return Err("Invalid source detail normal input".into()); }
        (0..image.levels.len()).map(|level| texture::pixels(image,level)).collect::<Result<Vec<_>>>()?
    } else { Vec::new() };
    let mut result = source.clone();
    result.format = 1;
    result.levels.clear();
    for level in 0..source.levels.len() {
        let mut bytes = Vec::new();
        for (index,pixel) in texture::pixels(source, level)?.into_iter().enumerate() {
            let mut value = pixel;
            match role {
                "color" => {
                    for channel in &mut value[..3] {
                        let linear = *channel * *channel;
                        *channel = if linear <= 0.0031308 { linear * 12.92 } else { 1.055 * linear.powf(1. / 2.4) - 0.055 };
                    }
                }
                "normal" => {
                    let mut slope = [pixel[0],pixel[1]].map(|value| value*f32::from_bits(0x40808102)+f32::from_bits(0xc0010204));
                    if let Some((image,sampler,parameters)) = detail {
                        let (width,height) = ((source.width>>level).max(1),(source.height>>level).max(1));
                        let uv = [(index%width) as f32+0.5,(index/width) as f32+0.5];
                        let uv = [uv[0]/width as f32*parameters[0],uv[1]/height as f32*parameters[1]];
                        let frequency = (image.width as f32*parameters[0].abs()).max(image.height as f32*parameters[1].abs()).max(1.);
                        let lod = (level as f32+frequency.log2()-(source.width.max(source.height) as f32).log2())
                            .floor().clamp(0.,(image.levels.len()-1) as f32) as usize;
                        let sample = texture::sample(&detail_pixels[lod],(image.width>>lod).max(1),(image.height>>lod).max(1),uv,sampler);
                        for axis in 0..2 { slope[axis]+=parameters[2]*(sample[axis]*f32::from_bits(0x40808102)+f32::from_bits(0xc0010204)); }
                    }
                    let [x,y] = slope;
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

fn camo_pixel(mut color: [f32;4], mut specular: [f32;4], detail: [f32;4], color_bias: f32, weight: f32) -> [[f32;4];2] {
    for component in 0..3 {
        let replacement_color = (detail[component]+color_bias-0.5).clamp(0.,1.);
        let replacement_specular = (specular[component]+detail[3]-0.5).clamp(0.,1.);
        let replacement_color = replacement_color+color[3]*(color[component]-replacement_color);
        let replacement_specular = replacement_specular+color[3]*(specular[component]-replacement_specular);
        color[component]+=weight*(replacement_color-color[component]);
        specular[component]+=weight*(replacement_specular-specular[component]);
    }
    [color,specular]
}

fn metallic_workflow(color: &texture::Image, specular: &texture::Image, samplers: [u8; 2], camo: Option<(&texture::Image,u8,[f32;4])>, occlusion: Option<(&texture::Image,u8)>) -> Result<(texture::Image,texture::Image)> {
    let mut sources = vec![color,specular];
    if let Some((image,_,parameters)) = camo {
        if !parameters.iter().all(|value| value.is_finite()) { return Err("Non-finite source camo parameters".into()); }
        sources.push(image);
    }
    if let Some((image,_)) = occlusion { sources.push(image); }
    if sources.iter().any(|image| image.faces != 1 || image.depth != 1 || image.levels.is_empty()) { return Err("Specular-workflow input is not a 2D image".into()); }
    let (width,height) = (sources.iter().map(|image| image.width).max().unwrap(),sources.iter().map(|image| image.height).max().unwrap());
    let mut base = texture::Image { width,height,depth:1,faces:1,format:1,flags:0,levels:Vec::new() };
    let mut response = base.clone();
    let decoded = sources.iter().map(|image| (0..image.levels.len()).map(|level| texture::pixels(image,level)).collect::<Result<Vec<_>>>()).collect::<Result<Vec<_>>>()?;
    for level in 0..sources.iter().map(|image| image.levels.len()).max().unwrap() {
        let (w,h) = ((width>>level).max(1),(height>>level).max(1));
        let mut base_bytes = Vec::with_capacity(w*h*4);
        let mut response_bytes = Vec::with_capacity(w*h*4);
        for y in 0..h { for x in 0..w {
            let uv = [(x as f32+0.5)/w as f32,(y as f32+0.5)/h as f32];
            let sample = |index: usize,uv,sampler| {
                let image = sources[index];
                let bias = width.max(height).ilog2()-image.width.max(image.height).ilog2();
                let source_level = level.saturating_sub(bias as usize).min(image.levels.len()-1);
                texture::sample(&decoded[index][source_level],(image.width>>source_level).max(1),(image.height>>source_level).max(1),uv,sampler)
            };
            let mut samples = [sample(0,uv,samplers[0]),sample(1,uv,samplers[1])];
            if let Some((_,sampler,parameters)) = camo {
                let detail_uv = std::array::from_fn(|axis| (uv[axis]-0.5)*parameters[2]+parameters[axis]+0.5);
                let detail = sample(2,detail_uv,sampler);
                let color_bias = if let Some((_,sampler)) = occlusion { sample(3,uv,sampler)[2] } else {1.};
                samples = camo_pixel(samples[0],samples[1],detail,color_bias,parameters[3]);
            }
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
    detail: bool,
    occlusion: u8,
    camo: bool,
    programs: Option<[&'static str;3]>,
}

fn ordinary_recipe(family: &str) -> Option<OrdinaryRecipe> {
    let (expected,normal,specular,coverage) = match family {
        "wpc_lit_sm_b0c0n0s0v0_efz5w56f" => ("pimp_shader_lmap_716f0714.hlsl", true, 1, 1),
        "mc_lit_sm_flag_t0c0d0n0s0_w75eqzzj" => ("pimp_shader_lprobe_f1c6978a.hlsl", true, 1, 2),
        "mc_lit_sm_flag_b0c0s0_6406ez95" => ("pimp_shader_lprobe_b781f149.hlsl", false, 1, 1),
        "mlv_lit_sm_r0c0d0n0_w65788j0" => ("pimp_shader_lprobe_bb12e125.hlsl", true, 0, 0),
        "mlv_lit_sm_r0c0n0x0_8f47740f" => ("pimp_shader_lprobe_181d1c0b.hlsl", true, 2, 0),
        "mlv_lit_sm_b0c0_50077370" => ("pimp_shader_lprobe_be61ff8.hlsl", false, 0, 1),
        "mlv_lit_sm_b0c0n0_322zqw73" => ("pimp_shader_lprobe_37d3284d.hlsl", true, 0, 1),
        "mlv_lit_sm_b0c0s0_41731e52" => ("pimp_shader_lprobe_e044b950.hlsl", false, 1, 1),
        "mlv_lit_sm_r0c0_475q06w3" => ("pimp_shader_lprobe_7d8789b8.hlsl", false, 0, 0),
        "mlv_lit_sm_r0c0n0x0_619829wj" => ("pimp_shader_lprobe_ae06fc7b.hlsl", true, 2, 0),
        "mlv_lit_sm_r0c0_j60qqj0e" => ("pimp_shader_lprobe_cb9c69c8.hlsl", false, 0, 0),
        "mlv_lit_sm_r0c0n0s0_wqqj839w" => ("pimp_shader_lprobe_1c0bf8d5.hlsl", true, 1, 0),
        "mlv_lit_sm_r0c0n0_3qf192ew" => ("pimp_shader_lprobe_f7a95e7d.hlsl", true, 0, 0),
        "mlv_lit_sm_t0c0n0s0_9jfj95q5" => ("pimp_shader_lprobe_e183a077.hlsl", true, 1, 2),
        "mc_lit_sm_t0c0n0s0_f933q84u" => ("pimp_shader_lprobe_c793a237.hlsl", true, 1, 2),
        "mc_lit_sm_b0c0n0s0_68f25z0u" => ("pimp_shader_lprobe_fa618ca5.hlsl", true, 1, 1),
        "mc_lit_sm_b0c0s0_q94f27qw" => ("pimp_shader_lprobe_c654bb10.hlsl", false, 1, 1),
        "mc_lit_sm_b0c0n0v0_3559e8jj" => ("pimp_shader_lprobe_66c64d74.hlsl", true, 0, 1),
        "wpc_lit_sm_b0c0n0v0_ew347fe3" => ("pimp_shader_lmap_82e99c4e.hlsl", true, 0, 1),
        "mc_lit_sm_r0c0d0n0_33ffej1u" => ("pimp_shader_lprobe_ecd7a93c.hlsl", true, 0, 0),
        "mc_lit_sm_r0c0d0n0s0_qwq4q449" => ("pimp_shader_lprobe_7750f94.hlsl", true, 1, 0),
        "mc_lit_sm_t0c0d0n0s0_68eww765" => ("pimp_shader_lprobe_fafd5736.hlsl", true, 1, 2),
        "mc_lit_sm_r0c0n0s0o0_3z86zq2z" => ("pimp_shader_lprobe_3c17e0d4.hlsl", true, 1, 0),
        "mc_lit_sm_r0c0n0s0o0_909j993q" => ("pimp_shader_lprobe_4dc2aa8d.hlsl", true, 1, 0),
        "mc_lit_sm_b0c0n0s0o0_jf00fe4q" => ("pimp_shader_lprobe_29115a54.hlsl", true, 1, 1),
        "mc_lit_sm_r0c0n0s0_ze50j7jz" => ("pimp_shader_lprobe_3a1bfa95.hlsl", true, 1, 0),
        "mc_lit_sm_r0c0n0s0_7j414q0z" => ("pimp_shader_lprobe_9eb27c7c.hlsl", true, 1, 0),
        "mc_lit_sm_r0c0n0s0_4w3wqfe5" => ("pimp_shader_lprobe_4bceb0cc.hlsl", true, 1, 0),
        "mc_lit_sm_r0c0n0x0_53144qq1" => ("pimp_shader_lprobe_4fd85412.hlsl", true, 2, 0),
        "mc_lit_sm_r0c0_5137qq99" => ("pimp_shader_lprobe_2a42c1a1.hlsl", false, 0, 0),
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
        "wpc_lit_sm_b0c0n0s0v0_efz5w56f" => Some(["pimp_shader_lmap_b89a2252.hlsl","pimp_shader_lmap_3324e8aa.hlsl","pimp_shader_lmap_3400e22.hlsl"]),
        "mc_lit_sm_flag_t0c0d0n0s0_w75eqzzj" => Some(["pimp_shader_lprobe_3f4a954b.hlsl","pimp_shader_lprobe_66a8e47e.hlsl","pimp_shader_lprobe_a824e6bf.hlsl"]),
        "mc_lit_sm_flag_b0c0s0_6406ez95" => Some(["pimp_shader_lprobe_37fa2fe.hlsl","pimp_shader_lprobe_5a9dd3cb.hlsl","pimp_shader_lprobe_e55840c0.hlsl"]),
        "mlv_lit_sm_r0c0d0n0_w65788j0" => Some(["pimp_shader_lprobe_63b41dee.hlsl","pimp_shader_lprobe_55ebacd1.hlsl","pimp_shader_lprobe_e2f09010.hlsl"]),
        "mlv_lit_sm_r0c0n0x0_8f47740f" => Some(["pimp_shader_lprobe_63b41dee.hlsl","pimp_shader_lprobe_55ebacd1.hlsl","pimp_shader_lprobe_41ff6d3e.hlsl"]),
        "mlv_lit_sm_b0c0_50077370" => Some(["pimp_shader_lprobe_bcc4a7bc.hlsl","pimp_shader_lprobe_8a9b1683.hlsl","pimp_shader_lprobe_52046ecd.hlsl"]),
        "mlv_lit_sm_b0c0n0_322zqw73" => Some(["pimp_shader_lprobe_63b41dee.hlsl","pimp_shader_lprobe_55ebacd1.hlsl","pimp_shader_lprobe_6e315978.hlsl"]),
        "mlv_lit_sm_b0c0s0_41731e52" => Some(["pimp_shader_lprobe_bcc4a7bc.hlsl","pimp_shader_lprobe_8a9b1683.hlsl","pimp_shader_lprobe_b9a6c865.hlsl"]),
        "mlv_lit_sm_r0c0_475q06w3" => Some(["pimp_shader_lprobe_bcc4a7bc.hlsl","pimp_shader_lprobe_8a9b1683.hlsl","pimp_shader_lprobe_2465f88d.hlsl"]),
        "mlv_lit_sm_r0c0n0x0_619829wj" => Some(["pimp_shader_lprobe_63b41dee.hlsl","pimp_shader_lprobe_55ebacd1.hlsl","pimp_shader_lprobe_f7e48d4e.hlsl"]),
        "mlv_lit_sm_r0c0_j60qqj0e" => Some(["pimp_shader_lprobe_bcc4a7bc.hlsl","pimp_shader_lprobe_8a9b1683.hlsl","pimp_shader_lprobe_927e18fd.hlsl"]),
        "mlv_lit_sm_r0c0n0s0_wqqj839w" => Some(["pimp_shader_lprobe_63b41dee.hlsl","pimp_shader_lprobe_55ebacd1.hlsl","pimp_shader_lprobe_45e989e0.hlsl"]),
        "mlv_lit_sm_r0c0n0_3qf192ew" => Some(["pimp_shader_lprobe_63b41dee.hlsl","pimp_shader_lprobe_55ebacd1.hlsl","pimp_shader_lprobe_ae4b2f48.hlsl"]),
        "mlv_lit_sm_t0c0n0s0_9jfj95q5" => Some(["pimp_shader_lprobe_63b41dee.hlsl","pimp_shader_lprobe_55ebacd1.hlsl","pimp_shader_lprobe_b861d142.hlsl"]),
        "mc_lit_sm_t0c0n0s0_f933q84u" => Some(["pimp_shader_lprobe_347155f7.hlsl","pimp_shader_lprobe_22ee4c8.hlsl","pimp_shader_lprobe_9e71d302.hlsl"]),
        "mc_lit_sm_b0c0n0s0_68f25z0u" => Some(["pimp_shader_lprobe_347155f7.hlsl","pimp_shader_lprobe_22ee4c8.hlsl","pimp_shader_lprobe_a383fd90.hlsl"]),
        "mc_lit_sm_b0c0s0_q94f27qw" => Some(["pimp_shader_lprobe_eb01efa5.hlsl","pimp_shader_lprobe_dd5e5e9a.hlsl","pimp_shader_lprobe_9fb6ca25.hlsl"]),
        "mc_lit_sm_b0c0n0v0_3559e8jj" => Some(["pimp_shader_lprobe_347155f7.hlsl","pimp_shader_lprobe_22ee4c8.hlsl","pimp_shader_lprobe_3f243c41.hlsl"]),
        "wpc_lit_sm_b0c0n0v0_ew347fe3" => Some(["pimp_shader_lmap_d107eac0.hlsl","pimp_shader_lmap_5ab92038.hlsl","pimp_shader_lmap_f0c69578.hlsl"]),
        "mc_lit_sm_r0c0d0n0_33ffej1u" => Some(["pimp_shader_lprobe_347155f7.hlsl","pimp_shader_lprobe_22ee4c8.hlsl","pimp_shader_lprobe_b535d809.hlsl"]),
        "mc_lit_sm_r0c0d0n0s0_qwq4q449" => Some(["pimp_shader_lprobe_347155f7.hlsl","pimp_shader_lprobe_22ee4c8.hlsl","pimp_shader_lprobe_5e977ea1.hlsl"]),
        "mc_lit_sm_t0c0d0n0s0_68eww765" => Some(["pimp_shader_lprobe_347155f7.hlsl","pimp_shader_lprobe_22ee4c8.hlsl","pimp_shader_lprobe_a31f2603.hlsl"]),
        "mc_lit_sm_r0c0n0s0o0_3z86zq2z" => Some(["pimp_shader_lprobe_347155f7.hlsl","pimp_shader_lprobe_22ee4c8.hlsl","pimp_shader_lprobe_65f591e1.hlsl"]),
        "mc_lit_sm_r0c0n0s0o0_909j993q" => Some(["pimp_shader_lprobe_347155f7.hlsl","pimp_shader_lprobe_22ee4c8.hlsl","pimp_shader_lprobe_1420dbb8.hlsl"]),
        "mc_lit_sm_b0c0n0s0o0_jf00fe4q" => Some(["pimp_shader_lprobe_347155f7.hlsl","pimp_shader_lprobe_22ee4c8.hlsl","pimp_shader_lprobe_70f32b61.hlsl"]),
        "mc_lit_sm_r0c0n0s0_ze50j7jz" => Some(["pimp_shader_lprobe_347155f7.hlsl","pimp_shader_lprobe_22ee4c8.hlsl","pimp_shader_lprobe_63f98ba0.hlsl"]),
        "mc_lit_sm_r0c0n0s0_7j414q0z" => Some(["pimp_shader_lprobe_347155f7.hlsl","pimp_shader_lprobe_22ee4c8.hlsl","pimp_shader_lprobe_c7500d49.hlsl"]),
        "mc_lit_sm_r0c0n0s0_4w3wqfe5" => Some(["pimp_shader_lprobe_7bb0619c.hlsl","pimp_shader_lprobe_4defd0a3.hlsl","pimp_shader_lprobe_122cc1f9.hlsl"]),
        "mc_lit_sm_r0c0n0x0_53144qq1" => Some(["pimp_shader_lprobe_347155f7.hlsl","pimp_shader_lprobe_22ee4c8.hlsl","pimp_shader_lprobe_163a2527.hlsl"]),
        "mc_lit_sm_r0c0_5137qq99" => Some(["pimp_shader_lprobe_eb01efa5.hlsl","pimp_shader_lprobe_dd5e5e9a.hlsl","pimp_shader_lprobe_73a0b094.hlsl"]),
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
    let detail = matches!(family,"mc_lit_sm_r0c0d0n0_33ffej1u" | "mc_lit_sm_r0c0d0n0s0_qwq4q449" | "mc_lit_sm_t0c0d0n0s0_68eww765" | "mc_lit_sm_flag_t0c0d0n0s0_w75eqzzj" | "mlv_lit_sm_r0c0d0n0_w65788j0");
    let occlusion = match family {
        "mc_lit_sm_r0c0n0s0o0_3z86zq2z" | "mc_lit_sm_r0c0n0s0o0_909j993q" => 1,
        "mc_lit_sm_b0c0n0s0o0_jf00fe4q" => 2,
        _ => 0,
    };
    let camo = occlusion == 1 || family == "mc_lit_sm_r0c0n0s0_7j414q0z";
    Some(OrdinaryRecipe {expected,normal,specular,coverage,detail,occlusion,camo,programs})
}

fn validate_ordinary(key: &str, material: &Value, recipe: &OrdinaryRecipe) -> Result<()> {
    let OrdinaryRecipe {expected,normal,specular,coverage,detail,occlusion,camo,programs} = *recipe;
    let family = material["technique_set"].as_str().ok_or("Missing native material technique")?;
    let reveal = matches!(family,"mc_lit_sm_b0c0n0v0_3559e8jj" | "wpc_lit_sm_b0c0n0v0_ew347fe3" | "wpc_lit_sm_b0c0n0s0v0_efz5w56f");
    let flag = matches!(family,"mc_lit_sm_flag_t0c0d0n0s0_w75eqzzj" | "mc_lit_sm_flag_b0c0s0_6406ez95");
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
        if normal || flag { routes.push(json!({"source":4,"destination":7})); }
        if family.starts_with("wpc_") || family.starts_with("mlv_") { routes.push(json!({"source":5,"destination":6})); }
        let mut inputs = BTreeMap::from([(2695565377_u32,0_u64)]);
        if normal { inputs.insert(1507003663,257); }
        if specular == 1 { inputs.insert(887934131,if detail {515} else if normal {514} else {257}); }
        if detail { inputs.insert(3948059469,if specular == 1 {770} else {514}); }
        if occlusion != 0 { inputs.insert(1610742065,if camo {1027} else {771}); }
        let scrolling = family == "mc_lit_sm_r0c0n0s0_4w3wqfe5";
        if camo { inputs.insert(3183121296,if occlusion != 0 {772} else {771}); }
        for (pass_index,pass) in [&passes[0],&sun_passes[0]].into_iter().enumerate() {
            if pass["vertex_routes"] != json!(routes) { return Err(format!("Reviewed material {key} has unreviewed vertex routes").into()); }
            let arguments = pass["arguments"].as_array().ok_or("Reviewed pass arguments are missing")?;
            let active: BTreeMap<_,_> = arguments.iter().filter(|argument| argument["type"] == 2)
                .map(|argument| Ok((argument["value"].as_u64().ok_or("Missing shader image hash")? as u32,
                    argument["location"].as_u64().ok_or("Missing shader image location")?)))
                .collect::<Result<_>>()?;
            let texture_arguments: Vec<_> = arguments.iter().filter(|argument| argument["type"] == 2).collect();
            if active != inputs || texture_arguments.len() != inputs.len()
                || texture_arguments.iter().any(|argument| argument["size"] != 1 || argument["buffer"] != 0) {
                return Err(format!("Reviewed material {key} has unreviewed active texture bindings").into());
            }
            if camo && arguments.iter().filter(|argument| argument["type"] == 5 && argument["value"] == 16777397_u32
                && argument["buffer"] == 1 && argument["location"] == 528 && argument["size"] == 16).count() != 1 {
                return Err(format!("Reviewed camo material {key} lacks its native weaponParam2 binding").into());
            }
            if (scrolling || flag) && arguments.iter().filter(|argument| argument["type"] == 3 && argument["value"] == 16777241_u32
                && argument["buffer"] == 2 && argument["location"] == 0 && argument["size"] == 16).count() != 1 {
                return Err(format!("Reviewed animated material {key} lacks its native gameTime binding").into());
            }
            for argument in arguments.iter().filter(|argument| matches!(argument["type"].as_u64(),Some(0 | 1 | 6 | 7 | 8))) {
                let hash = argument["value"].as_u64().ok_or("Missing material constant hash")?;
                let expected = match hash {
                    2418533825 => (672,12),
                    2932833280 if reveal => (928,16),
                    148072969 if detail => (848,16),
                    2651392336 if flag => (656,16),
                    411508820 if camo && occlusion != 0 && pass_index == 1 => (1008,16),
                    _ => return Err(format!("Reviewed material {key} has an unreviewed active material constant").into()),
                };
                let kind = if flag && hash == 2651392336 { 0 } else { 6 };
                if argument["type"] != kind || argument["buffer"] != 1
                    || argument["location"] != expected.0 || argument["size"] != expected.1 {
                    return Err(format!("Reviewed material {key} has an unreviewed active material constant").into());
                }
                let constant = material["constants"].as_array().ok_or("Missing native constant table")?.iter()
                    .find(|constant| constant["hash"] == hash).ok_or("Missing bound material constant")?;
                if (0..expected.1 as usize/4).any(|component| constant["literal"][component].as_f64().is_none_or(|value| !value.is_finite())) {
                    return Err(format!("Reviewed material {key} has invalid material constants").into());
                }
            }
            if detail && arguments.iter().filter(|argument| argument["type"] == 6 && argument["value"] == 148072969_u32).count() != 1 {
                return Err(format!("Reviewed material {key} lacks its bound detailScale").into());
            }
            if flag && arguments.iter().filter(|argument| argument["type"] == 0 && argument["value"] == 2651392336_u32).count() != 1 {
                return Err(format!("Reviewed flag material {key} lacks its bound flagParams").into());
            }
            if reveal {
                if arguments.iter().filter(|argument| argument["type"] == 6 && argument["value"] == 2932833280_u32).count() != 1 {
                    return Err(format!("Reviewed reveal material {key} lacks its bound alphaRevealParms").into());
                }
                let constant = material["constants"].as_array().unwrap().iter().find(|constant| constant["hash"] == 2932833280_u32).unwrap();
                let parameters: [f32;4] = serde_json::from_value(constant["literal"].clone())?;
                if parameters[0] <= 0. || parameters[1] < 0. {
                    return Err(format!("Reviewed reveal material {key} has invalid reveal parameters").into());
                }
            }
            if camo && occlusion != 0 && pass_index == 1
                && arguments.iter().filter(|argument| argument["type"] == 6 && argument["value"] == 411508820_u32).count() != 1 {
                return Err(format!("Reviewed material {key} lacks its sun specular scale binding").into());
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
        "mc_unlit_multiply_97923qz1" => Some(("pimp_shader_unlit_4f14dd58.hlsl","multiply")),
        "mc_unlitdecalblend_add_fzw1z66f" | "wpc_unlitdecalblend_add_j26wq580" => Some(("pimp_shader_unlitdecalblend_62b62074.hlsl","decal_add")),
        "mc_unlitdecalblend_multiply_34w7q3jw" | "wpc_unlitdecalblend_multiply_35079164" => Some(("pimp_shader_unlitdecalblend_5c3f6d9c.hlsl","decal_multiply")),
        _ => None,
    }
}

fn validate_unlit(key: &str, material: &Value, pixel: &str, alpha: &str) -> Result<f32> {
    let techniques = material["techniques"].as_array().ok_or("Missing native unlit techniques")?;
    let technique = techniques.iter().find(|technique| technique["index"] == 2).ok_or("Missing native unlit technique")?;
    let passes = technique["passes"].as_array().ok_or("Missing native unlit passes")?;
    let decal = matches!(alpha,"decal_add" | "decal_multiply");
    let scaled = !decal && alpha != "multiply";
    let vertex = if decal {pixel} else {"pimp_shader_unlit_4876d431.hlsl"};
    if passes.len() != 1 || passes[0]["pixel_shader"] != pixel || passes[0]["vertex_shader"] != vertex
        || passes[0]["vertex_routes"] != json!([{"source":0,"destination":0},{"source":1,"destination":2},{"source":2,"destination":5}]) {
        return Err(format!("Unlit material {key} has unreviewed programs or vertex routes").into());
    }
    let arguments = passes[0]["arguments"].as_array().ok_or("Missing unlit arguments")?;
    let active: Vec<_> = arguments.iter().filter(|argument| matches!(argument["type"].as_u64(),Some(0 | 1 | 2 | 6 | 7 | 8))).collect();
    if active.len() != (if scaled {2} else {1}) || !active.iter().any(|argument| argument["type"] == 2 && argument["value"] == 2695565377_u32
        && argument["location"] == 0 && argument["size"] == 1 && argument["buffer"] == 0)
        || scaled && !active.iter().any(|argument| argument["type"] == 6 && argument["value"] == 3799286735_u32
            && argument["location"] == 16 && argument["size"] == 4 && argument["buffer"] == 2) {
        return Err(format!("Unlit material {key} has unreviewed active bindings").into());
    }
    let texture = material["textures"].as_array().ok_or("Missing unlit textures")?.iter()
        .find(|texture| texture["hash"] == 2695565377_u32).ok_or("Missing unlit color texture")?;
    let sampler = texture["sampler"].as_u64();
    if !(if decal { matches!(sampler,Some(10 | 74)) } else { matches!(sampler,Some(20 | 244)) }) {
        return Err(format!("Unlit material {key} has an unreviewed sampler").into());
    }
    let scale = if scaled {
        let constant = material["constants"].as_array().ok_or("Missing unlit constants")?.iter()
            .find(|constant| constant["hash"] == 3799286735_u32).ok_or("Missing bound scaleRGB")?;
        constant["literal"][0].as_f64().ok_or("Missing scaleRGB value")? as f32
    } else {1.};
    if !scale.is_finite() || scale < 0. { return Err(format!("Unlit material {key} has invalid scaleRGB").into()); }
    let index = material["state_entries"][2].as_u64().ok_or("Missing native unlit state index")? as usize;
    let state = material["state_bits"][index][0].as_u64().ok_or("Missing native unlit state")?;
    let depth = material["state_bits"][index][1].as_u64().ok_or("Missing native unlit depth state")?;
    let (rgb,coverage) = match alpha { "opaque" => (0x12,0x12),"add" | "decal_add" => (0x122,0x121),
        "blend" => (0x165,0x128),"multiply" | "decal_multiply" => (0x131,0x128),_ => unreachable!() };
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
            let decal_weight = pixel[3]*(pixel[0]*0.25+pixel[1]*0.5+pixel[2]*0.25);
            let value: [f32;4] = std::array::from_fn(|c| if c == 3 {
                if matches!(alpha,"add" | "source_vm" | "source_vm_add") {1.}
                else if alpha == "source_vm_blend" {pixel[3].sqrt()}
                else if alpha == "decal_add" {decal_weight} else {pixel[3]}
            } else { match alpha {
                "decal_add" => (pixel[c]*decal_weight).powi(2),
                "decal_multiply" => (1.+pixel[3]*(pixel[c]-1.)).powi(2),
                "multiply" => pixel[c].powi(4),
                "source_layer" | "source_vm_add" => (pixel[c]*pixel[3]).powi(2)*scale,
                _ => pixel[c]*pixel[c]*scale*coverage,
            }});
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

fn authored_unlit_uv(source: &texture::Image, scale: [f32;2], offset: [f32;2], origin: [f32;2], rotation: f32, sampler: u8) -> Result<texture::Image> {
    if scale == [1.;2] && offset == [0.;2] && rotation == 0. {return Ok(source.clone());}
    if source.depth != 1 || source.faces != 1 || source.levels.is_empty()
        || !scale.into_iter().chain(offset).chain(origin).chain([rotation]).all(f32::is_finite) {
        return Err("Invalid authored single-layer UV input".into());
    }
    let mut image = source.clone();image.format = 19;image.levels.clear();
    let (sine,cosine) = (rotation*f32::from_bits(0x40c90fda)).sin_cos();
    for level in 0..source.levels.len() {
        let (width,height) = ((source.width>>level).max(1),(source.height>>level).max(1));
        let pixels = texture::pixels(source,level)?;
        let mut bytes = Vec::with_capacity(width*height*8);
        for y in 0..height {for x in 0..width {
            let uv = [(x as f32+0.5)/width as f32,(y as f32+0.5)/height as f32];
            let centered: [f32;2] = std::array::from_fn(|axis| (uv[axis]-origin[axis]+offset[axis])*scale[axis]);
            let uv = [cosine*centered[0]+sine*centered[1]+origin[0],-sine*centered[0]+cosine*centered[1]+origin[1]];
            let pixel = texture::sample(&pixels,width,height,uv,sampler);
            for value in pixel {bytes.extend(half::f16::from_f32(value).to_le_bytes());}
        }}
        image.levels.push(bytes);
    }
    Ok(image)
}

fn convert_single_unlit(key: &str, native: &Value, images: &BTreeMap<String,texture::Image>, shared: &mut BTreeMap<(String,String),String>, pack: &mut Pack) -> Result<Value> {
    let (index,hash,mode) = validate_single_unlit(key,native)?;
    let layered = index == 4;
    let binding = native["textures"].as_array().unwrap().iter().find(|binding| binding["hash"] == hash).unwrap();
    let name = binding["image"].as_str().ok_or("Missing single-layer unlit source image name")?.to_ascii_lowercase();
    let source = images.get(&name).ok_or_else(|| format!("Missing single-layer unlit image {name}"))?;
    let constants = native["constants"].as_array().unwrap();
    let constant = |hash: u32| -> Result<[f32;4]> {
        let row = constants.iter().find(|constant| constant["hash"] == hash).ok_or("Missing bound single-layer image parameter")?;
        Ok(serde_json::from_value(row["literal"].clone())?)
    };
    let eyes = mode == "eyes";
    let eye_state: Option<[f32;4]> = if eyes && !native["code_constants"]["171"].is_null() {
        let state: [f32;4] = serde_json::from_value(native["code_constants"]["171"].clone())?;
        if state.iter().any(|value| !value.is_finite()) {return Err("Non-finite native eye luminance input".into());}Some(state)
    } else {None};
    let amount = if eyes {eye_state.unwrap_or([0.;4])[1]} else {constant(if layered {14836402} else {2875883048})?[0]};
    let uv = if layered {Some([constant(3320325482)?,constant(3286424575)?,constant(3272022534)?,constant(1886964550)?])} else {None};
    let image = if let Some([scale,offset,origin,rotation]) = uv {
        authored_unlit_uv(source,[scale[0],scale[1]],[offset[0],offset[1]],[origin[0],origin[1]],rotation[0],20)?
    } else {source.clone()};
    let mut image = image;
    if eyes {image.levels.truncate(1);}
    let operation = if layered {"source_layer"} else {match mode {"add" => "source_vm_add","blend" => "source_vm_blend","alpha" => "source_vm_alpha",_ => "source_vm"}};
    let image = unlit_color(&image,amount*amount,operation)?;
    let linear = image.format == 19;
    let channel_key = (name.clone(),format!("single-unlit:{operation}:{}:{uv:?}",amount.to_bits()));
    let uri = if let Some(uri) = shared.get(&channel_key) {uri.clone()} else {
        let uri = store_image(&image,format!("prepared/material-channels/{}/color.ktx2",shared.len()),!linear,"color",pack)?;
        shared.insert(channel_key,uri.clone());uri
    };
    let source_key = (name,"source_color".into());
    let source_uri = if let Some(uri) = shared.get(&source_key) {uri.clone()} else {
        let uri = store_image(source,format!("prepared/material-channels/{}/source-color.ktx2",shared.len()),false,"source_color",pack)?;
        shared.insert(source_key,uri.clone());uri
    };
    let selected = native["state_entries"][index].as_u64().unwrap() as usize;
    let state = native["state_bits"][selected][0].as_u64().unwrap();
    let cull = match state>>14 & 3 {1 => "none",2 => "back",3 => "front",_ => return Err("Unknown single-layer unlit cull state".into())};
    let alpha = if layered || mode == "alpha" {"blend"} else if eyes {"opaque"} else {mode};
    let mut surface = json!({"version":3,"identity":key,"native":native,
        "channels":[{"uri":uri,"role":"color","layer":0,"color_space":if linear {"linear"} else {"srgb"},"sampler":20}],
        "source_input":{"uri":source_uri,"sampler":20,"color_space":"native_encoded"},
        "standard":{"unlit":true,"roughness":1.,"metallic":0.,"reflectance":0.,"base_color":[1.,1.,1.,1.]},
        "presentation":{"alpha":alpha,"cull":cull,"fog":mode != "layer_reflection","fog_mode":"color","visible":state & (1<<27) == 0},
        "source_vertex_color_transfer":"unused","source_coverage":if layered {"script_scaled_sqrt_coverage"} else {"source_viewmodel_emissive"},
        "source_emissive":{"amount":amount,"rgb":if layered || mode == "add" {"square(texture_rgb*texture_alpha*amount)"} else {"square(texture_rgb*amount)"},"texture_alpha_in_rgb":layered || mode == "add",
            "output_transfer":if layered {"sqrt_after_scene_hdr_scale"} else {"linear_without_gamma2_sqrt"}},
        "source_blend":{"rgb":[state&15,state>>4&15,state>>8&7],"alpha":[state>>16&15,state>>20&15,state>>24&7]},
        "approximations":["Static authored UV terms resampled at original mip resolution; dynamic parallax/time terms retained for runtime integration",
            "Native source color equations retained separately from unresolved framebuffer transfer and destination-alpha composition"]});
    if layered {
        surface["source_layer_uv"] = json!({"scale":constant(3320325482)?,"offset":constant(3286424575)?,
            "origin":constant(3272022534)?,"rotation":constant(1886964550)?,"scroll":constant(2196952607)?,
            "depth":constant(3342767871)?[0],"depth_epsilon":f32::from_bits(0x3727c5ac),"game_time_code":25,"game_time_component":3,
            "operation":"scale_centered_uv_plus_scaled_offset_and_positive_facing_view_parallax_then_rotate_then_add_scaled_time_scroll_and_origin",
            "rotation_operation":"rotation_x_times_one_plus_rotation_y_times_game_time_w","rotation_tau":f32::from_bits(0x40c90fda)});
        surface["source_script_coverage"] = json!({"code_constant":169,"component":0,"layer_alpha":constant(3337236870)?[0],
            "operation":"sqrt(texture_alpha*layer_alpha*(1-saturate(scriptVector0.x)))"});
        if mode == "layer_reflection" {
            surface["source_reflection"] = json!({"amount":constant(211931156)?[0],"gloss":constant(3432544439)?[0],
                "lod":"four_times_one_minus_gloss","weight":"saturate_three_quarters_plus_quarter_gloss_times_reflection_amount",
                "probe_decode":"rgb_divided_by_alpha_plus_epsilon","response":"separate_vertex_spherical_harmonics"});
            surface["pending_runtime"] = json!("Native no-fog rune layer requires view parallax including the depth epsilon, gameTime UV controls, live scriptVector0 alpha, separate view-dependent reflection/SH response and sqrt coverage in gamma2 composition; source consumers remain unconnected");
        } else {
            surface["pending_runtime"] = json!("Native wallbuy layer requires view parallax including the depth epsilon, gameTime UV controls and live scriptVector0 alpha, followed by source fog and sqrt coverage in gamma2 framebuffer composition; matching runtime consumers remain unconnected");
        }
    } else {
        surface["source_fragment_alpha"] = json!(if mode == "blend" {"sqrt_texture_coverage"} else if mode == "alpha" {"texture_coverage"} else {"one"});
        surface["pending_runtime"] = json!(match mode {
            "screen" => "Native tritium emission writes squared linear source RGB without the gamma2 sqrt, then blends with inverse destination color plus destination; source fog, screen blending and destination-alpha composition remain unconnected",
            "blend" => "Native translucent viewmodel emission writes squared source RGB without gamma2 sqrt and sqrt texture alpha with source-alpha blending; matching source fog and framebuffer composition remain unconnected",
            "alpha" => "Native viewmodel alpha emission writes squared source RGB and unchanged texture alpha with source-alpha blending; matching source fog and linear-output framebuffer composition remain unconnected",
            "eyes" => "Native cheap eyes use scriptVector2.y luminance with explicit LOD-zero sampling before source fog, scene HDR control0 and gamma2 output; mapped actor eye-color/luminance overrides and matching runtime composition remain unconnected",
            _ => "Native additive viewmodel emission squares texture-alpha-premultiplied source RGB, writes alpha one, and adds source fog color before ONE/ONE blending; matching source fog and framebuffer composition remain unconnected",
        });
        if eyes {
            surface["source_eye_luminance"] = json!({"code_constant":171,"component":1,"state":eye_state,"base_state":[0.,0.,0.,0.],
                "base_state_origin":"normal_view_zero_template_preserved_by_type2_constant_copy","lod":0.,"operation":"square_texture_rgb_times_luminance",
                "active_override_scope":"actor_scriptVector2_mapping_replaces_base_state"});
            surface["source_emissive"]["output_transfer"] = json!("sqrt_after_source_fog_and_scene_hdr_control0");
        }
    }
    Ok(json!({"name":key,"pbrMetallicRoughness":{"baseColorFactor":[1.,1.,1.,1.],"metallicFactor":0.},"extras":{"surface":surface}}))
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
    let composed = matches!(alpha,"multiply" | "decal_add" | "decal_multiply");
    let blend = match alpha {"decal_add" => "add","decal_multiply" => "multiply",_ => alpha};
    let mut converted = json!({"name":key,"pbrMetallicRoughness":{"baseColorFactor":[1.,1.,1.,1.],"metallicFactor":0.},
        "extras":{"surface":{"version":3,"identity":key,"native":native,
            "channels":[{"uri":uri,"role":"color","layer":0,"color_space":if linear {"linear"} else {"srgb"},"sampler":binding["sampler"]}],
            "standard":{"unlit":true,"roughness":1.,"metallic":0.,"reflectance":0.,"base_color":[1.,1.,1.,1.]},
            "presentation":{"alpha":blend,"cull":cull,"fog":alpha != "decal_add","fog_mode":if alpha == "add" || composed {"attenuation"} else {"color"},"visible":state & (1<<27) == 0},
            "source_vertex_color_transfer":if composed {"native_fragment_composition"} else if alpha == "blend" {"gamma2_alpha"} else {"gamma2"},
            "source_coverage":if composed {alpha} else if alpha == "add" {"unlit_add"} else if alpha == "blend" {"unlit_blend"} else {"opaque"},
            "pending_runtime":if alpha == "add" {json!("Native ONE/ONE addition requires fog attenuation without adding fog color")} else {Value::Null},
            "approximations":["Vertex gamma2 transfer evaluated before interpolation", "Source gamma2 framebuffer composition represented in the shared linear HDR pipeline", "Source polygon-offset modes retained for runtime integration"]}}});
    if composed {
        let source_key = (binding["image"].as_str().unwrap().to_ascii_lowercase(),"source_color".into());
        let source_uri = if let Some(uri) = shared.get(&source_key) {uri.clone()} else {
            let uri = store_image(source,format!("prepared/material-channels/{}/source-color.ktx2",shared.len()),false,"source_color",pack)?;
            shared.insert(source_key,uri.clone());uri
        };
        let surface = &mut converted["extras"]["surface"];
        surface["source_input"] = json!({"uri":source_uri,"sampler":binding["sampler"],"color_space":"native_encoded"});
        surface["source_composition"] = match alpha {
            "decal_add" => json!({"color":"texture_rgb_times_interpolated_vertex_rgb",
                "coverage":"texture_alpha_times_interpolated_vertex_alpha_times_color_luminance",
                "luminance_weights":[0.25,0.5,0.25],"rgb":"color_times_coverage","alpha":"coverage","fog":false}),
            "decal_multiply" => json!({"color":"texture_rgb_times_interpolated_vertex_rgb",
                "coverage":"texture_alpha_times_interpolated_vertex_alpha_times_fog_transmittance",
                "rgb":"one_plus_coverage_times_color_minus_one","alpha":"coverage","fog_neutral_rgb":[1.,1.,1.]}),
            _ => json!({"color":"square_texture_rgb_times_interpolated_vertex_rgb",
                "coverage":"texture_alpha_times_interpolated_vertex_alpha",
                "rgb":"one_plus_fog_transmittance_times_color_minus_one","alpha":"coverage","fog_neutral_rgb":[1.,1.,1.]}),
        };
        surface["approximations"] = json!(["Static texture terms evaluated for unit vertex tint and no fog; raw inputs retained for source fragment composition",
            "Native encoded framebuffer factors squared for shared linear HDR storage; exact destination-alpha and vertex/fog composition remain pending",
            "Source polygon-offset modes retained for runtime integration"]);
        surface["pending_runtime"] = json!(match alpha {
            "decal_add" => "Native additive decal computes coverage from interpolated vertex-tinted RGB luminance and texture/vertex alpha, then adds encoded color times that coverage without fog; source fragment tint, destination-alpha and gamma2 framebuffer composition remain unconnected",
            "decal_multiply" => "Native multiplicative decal interpolates its encoded texture/vertex color toward white using texture alpha times vertex alpha and fog transmittance, then multiplies the destination; source fragment tint, neutral-white fog and destination-alpha composition remain unconnected",
            _ => "Native unlit multiplication squares interpolated texture/vertex RGB before fog interpolates the encoded multiplier toward white; source fragment tint, neutral-white fog and destination-alpha composition remain unconnected",
        });
    }
    Ok(converted)
}

fn single_unlit_recipe(family: &str) -> Option<(usize,&'static str,&'static str,u32,&'static str)> {
    Some(match family {
        "wpc_sw4_3d_unlit_1layer_8691j77q" => (4,"pimp_shader_sw4_3d_unlit_1layer_20d9eb7d.hlsl","pimp_shader_sw4_3d_unlit_1layer_c5daea2d.hlsl",3714339118,"layer"),
        "mc_sw4_3d_unlit_1layer_nofog_91f00f80" => (4,"pimp_shader_sw4_3d_unlit_1layer_nofog_39bf2c98.hlsl","pimp_shader_sw4_3d_unlit_1layer_nofog_b5cdb07a.hlsl",3714339118,"layer_reflection"),
        "mc_sw4_3d_viewmodel_unlit_j299w86e" => (3,"pimp_shader_sw4_3d_viewmodel_unlit_89d1b67c.hlsl","pimp_shader_sw4_3d_viewmodel_unlit_89d1b67c.hlsl",3053961470,"screen"),
        "mc_sw4_3d_viewmodel_unlit_8216051u" => (3,"pimp_shader_sw4_3d_viewmodel_unlit_89d1b67c.hlsl","pimp_shader_sw4_3d_viewmodel_unlit_657ff8d2.hlsl",3053961470,"blend"),
        "mc_sw4_3d_viewmodel_unlit_60466j88" => (3,"pimp_shader_sw4_3d_viewmodel_unlit_89d1b67c.hlsl","pimp_shader_sw4_3d_viewmodel_unlit_ff86912b.hlsl",3053961470,"add"),
        "mc_sw4_3d_viewmodel_unlit_alpha_85z6j5w5" => (3,"pimp_shader_sw4_3d_viewmodel_unlit_89d1b67c.hlsl","pimp_shader_sw4_3d_viewmodel_unlit_alpha_161b096b.hlsl",3053961470,"alpha"),
        "mc_sw4_3d_model_unlit_cheap_zombie_eyes_jq3e7eqw" => (3,"pimp_shader_sw4_3d_model_unlit_cheap_zombie_eyes_6a8c6a7c.hlsl","pimp_shader_sw4_3d_model_unlit_cheap_zombie_eyes_ee6b6abd.hlsl",2695565377,"eyes"),
        _ => return None,
    })
}

fn falloff_recipe(family: &str) -> Option<(usize,&'static str,&'static str,&'static str)> {
    Some(match family {
        "mc_sw4_3d_unlit_3layer_nofog_falloff_w840w777" => (4,"pimp_shader_sw4_3d_unlit_3layer_nofog_falloff_197b665e.hlsl","pimp_shader_sw4_3d_unlit_3layer_nofog_falloff_997302f8.hlsl","add"),
        "mc_sw4_3d_unlit_3layer_nofog_falloff_5z943qe4" => (4,"pimp_shader_sw4_3d_unlit_3layer_nofog_falloff_197b665e.hlsl","pimp_shader_sw4_3d_unlit_3layer_nofog_falloff_597f540d.hlsl","opaque"),
        "mc_sw4_3d_unlit_3layer_nofog_falloff_5z909874" => (4,"pimp_shader_sw4_3d_unlit_3layer_nofog_falloff_197b665e.hlsl","pimp_shader_sw4_3d_unlit_3layer_nofog_falloff_c21aa953.hlsl","blend"),
        "mc_sw4_3d_unlit_3layer_nofog_falloff_zfeather_wq4jwzz6" => (3,"pimp_shader_sw4_3d_unlit_3layer_nofog_falloff_zfeather_a5140590.hlsl","pimp_shader_sw4_3d_unlit_3layer_nofog_falloff_zfeather_4b540a08.hlsl","zfeather"),
        "mlv_sw4_3d_unlit_3layer_nofog_falloff_fjz6ze0u" => (4,"pimp_shader_sw4_3d_unlit_3layer_nofog_falloff_5f626c64.hlsl","pimp_shader_sw4_3d_unlit_3layer_nofog_falloff_df6a08c2.hlsl","add"),
        "mc_sw4_3d_unlit_3layer_nofog_qw71w785" => (4,"pimp_shader_sw4_3d_unlit_3layer_nofog_falloff_197b665e.hlsl","pimp_shader_sw4_3d_unlit_3layer_nofog_61f120bd.hlsl","layer_blend"),
        "mc_sw4_3d_unlit_4layer_nofog_eee768jw" => (4,"pimp_shader_sw4_3d_unlit_4layer_nofog_cd11c014.hlsl","pimp_shader_sw4_3d_unlit_4layer_nofog_8b288e65.hlsl","four_add"),
        "mlv_sw4_3d_unlit_4layer_nofog_2j0ezz67" => (4,"pimp_shader_sw4_3d_unlit_4layer_nofog_3874c1d6.hlsl","pimp_shader_sw4_3d_unlit_4layer_nofog_7e4d8fa7.hlsl","four_add"),
        _ => return None,
    })
}

fn validate_falloff(key: &str, material: &Value) -> Result<(usize,&'static str)> {
    let family = material["technique_set"].as_str().ok_or("Missing three-layer falloff family")?;
    let (index,vertex,pixel,mode) = falloff_recipe(family).ok_or("Unreviewed three-layer falloff family")?;
    let feather = mode == "zfeather";
    let four = mode == "four_add";
    let mut routes = vec![json!({"source":0,"destination":0})];
    if feather {routes.push(json!({"source":1,"destination":2}));}
    routes.extend([json!({"source":2,"destination":5}),json!({"source":3,"destination":1}),json!({"source":4,"destination":7})]);
    if family.starts_with("mlv_") {routes.push(json!({"source":5,"destination":6}));}
    let mut constants = BTreeMap::from([(3320325482_u32,(80,8)),(3286424575,(88,8)),(2196952607,(96,8)),
        (3354717129,(104,8)),(3332229724,(112,8)),(128962428,(120,8)),(3398313000,(128,8)),
        (1565193213,(136,8)),(475721245,(144,8)),(3337236870,(152,4)),(3376302501,(156,4)),
        (1886964550,(164,8)),(1519946245,(176,8)),(1339484996,(184,8)),(1550647684,(192,8)),
        (114353733,(200,8)),(3272022534,(208,8)),(3342767871,(216,4)),(3379389788,(220,4)),(3416155453,(224,4))]);
    if mode != "opaque" {constants.insert(3410903492,(160,4));}
    if feather {constants.extend([(14836402,(228,4)),(3185247991,(232,4)),(3173127725,(236,4)),(3371644427,(240,4))]);}
    else if four {
        constants.insert(1886964550,(168,8));
        constants.extend([(3432544439,(228,4)),(14836402,(164,4)),(211931156,(232,4)),(3736587663,(240,8)),
            (3836749946,(248,8)),(3048924506,(256,8)),(3813076419,(264,8)),(3762764771,(272,4)),(3756660378,(276,4)),(2389732611,(280,8))]);
    } else {
        constants.extend([(3432544439,(228,4)),(14836402,(232,4)),(211931156,(236,4))]);
        if mode != "layer_blend" {constants.extend([(3185247991,(240,4)),(3173127725,(244,4))]);}
    }
    let mut arguments = vec![(3,67109077_u32,3,0,64),(3,67109093,0,576,64),
        (5,16777385,1,368,16),(if four {3} else {5},16777241,2,64,16)];
    if feather {arguments.extend([(4,18,0,259,1),(5,16777233,0,1520,16)]);}
    else {arguments.extend([(3,16777286,3,80,16),(3,16777287,3,96,16),(3,16777288,3,112,16)]);}
    let mut hashes = vec![3714339118_u32,3714374925,3714406764];
    if four {hashes.push(3714721355);}
    let locations = if four {vec![259,514,769,1024]} else if feather {vec![514,769,1024]} else {vec![258,513,768]};
    arguments.extend(hashes.iter().copied().zip(locations).map(|(hash,location)| (2,hash,0,location,1)));
    arguments.extend(constants.iter().map(|(&hash,&(location,size))| {
        let pixel = matches!(hash,14836402 | 211931156 | 3337236870 | 3376302501 | 3410903492 | 3432544439 | 3762764771);
        (if four && !pixel {0} else {6},hash,2,location,size)
    }));
    let mut state = match mode {"opaque" => [403867666_u32,13],"blend" | "layer_blend" => [422084965,12],"zfeather" => [421628194,12],_ => [422086946,12]};
    if family == "mc_sw4_3d_unlit_3layer_nofog_falloff_w840w777" && material["name"] == "mc/mtl_p6_zm_tm_pap_glow_effect" {state[0]=422070562;}
    let vortex = four && matches!(material["name"].as_str(),Some("mc/mtl_zm_tm_elm_portal_vortex" | "mlv/mtl_zm_tm_elm_portal_vortex"));
    if vortex {state=[422070562,28];}
    for selected in if feather {vec![3]} else {vec![4,6]} {
        let technique = material["techniques"].as_array().ok_or("Missing three-layer falloff techniques")?.iter()
            .find(|technique| technique["index"] == selected).ok_or("Missing selected three-layer falloff technique")?;
        let passes = technique["passes"].as_array().ok_or("Missing three-layer falloff passes")?;
        if passes.len() != 1 || passes[0]["vertex_shader"] != vertex || passes[0]["pixel_shader"] != pixel
            || passes[0]["vertex_routes"] != json!(routes) {
            return Err(format!("Three-layer falloff material {key} has unreviewed programs or vertex routes").into());
        }
        let state_index = material["state_entries"][selected].as_u64().ok_or("Missing three-layer falloff state index")? as usize;
        if material["state_bits"][state_index] != json!(state) {return Err(format!("Three-layer falloff material {key} has unreviewed render state").into());}
        let active = passes[0]["arguments"].as_array().ok_or("Missing three-layer falloff arguments")?;
        if active.len() != arguments.len() || arguments.iter().any(|&(kind,value,buffer,location,size)| active.iter()
            .filter(|argument| argument["type"] == kind && argument["value"] == value && argument["buffer"] == buffer
                && argument["location"] == location && argument["size"] == size).count() != 1) {
            return Err(format!("Three-layer falloff material {key} has unreviewed active arguments").into());
        }
    }
    for (&hash,&(_,size)) in &constants {
        let constant = material["constants"].as_array().ok_or("Missing three-layer falloff constants")?.iter()
            .find(|constant| constant["hash"] == hash).ok_or("Missing bound three-layer falloff constant")?;
        if (0..size as usize/4).any(|component| constant["literal"][component].as_f64().is_none_or(|value| !value.is_finite())) {
            return Err(format!("Three-layer falloff material {key} has non-finite constant {hash}").into());
        }
    }
    for &hash in &hashes {
        let binding = material["textures"].as_array().ok_or("Missing three-layer falloff images")?.iter()
            .find(|binding| binding["hash"] == hash).ok_or("Missing active three-layer falloff image")?;
        if if four {binding["sampler"] != if vortex {244} else {20}}
            else {!matches!(binding["sampler"].as_u64(),Some(20 | 84)) || (hash == hashes[0] && binding["sampler"] != 20)} {
            return Err(format!("Three-layer falloff material {key} has unreviewed sampler").into());
        }
    }
    Ok((index,mode))
}

#[derive(Clone,Copy,Debug)]
struct UnlitLayer {
    scale: [f32;2], offset: [f32;2], origin: [f32;2], rotation: [f32;2], scroll: [f32;2], depth: f32, alpha: Option<f32>,
}

fn falloff_pixel(samples: [[f32;4];3], alpha: [f32;2], last_alpha: Option<f32>, script: [f32;4], mode: &str) -> [f32;4] {
    let weights: [f32;2] = std::array::from_fn(|layer| samples[layer][3]*alpha[layer]*(1.-script[layer].clamp(0.,1.)));
    let coverage = if mode == "opaque" {1.} else {
        ((samples[2][3]*last_alpha.unwrap()*(1.-script[2].clamp(0.,1.))+weights[1]).clamp(0.,1.)+weights[0]).clamp(0.,1.)
    };
    let mut color: [f32;3] = std::array::from_fn(|component| samples[2][component]*samples[2][3]);
    for layer in [1,0] {for component in 0..3 {color[component]+=weights[layer]*(samples[layer][component]-color[component]);}}
    if matches!(mode,"add" | "zfeather") {for component in &mut color {*component*=coverage;}}
    [color[0],color[1],color[2],if mode == "blend" {coverage.sqrt()} else {1.}]
}

fn four_layer_pixel(samples: [[f32;4];4], alpha: [f32;4], script: [f32;4]) -> [f32;4] {
    let weights: [f32;4] = std::array::from_fn(|layer| samples[layer][3]*alpha[layer]*(1.-script[layer].clamp(0.,1.)));
    let mut color: [f32;3] = std::array::from_fn(|component| samples[3][component]*samples[3][3]);
    let mut coverage = weights[3];
    for layer in [2,1,0] {
        for component in 0..3 {color[component]+=weights[layer]*(samples[layer][component]-color[component]);}
        coverage = (coverage+weights[layer]).clamp(0.,1.);
    }
    [color[0]*coverage,color[1]*coverage,color[2]*coverage,1.]
}

fn falloff_image<const N: usize>(sources: [&texture::Image;N], samplers: [u8;N], layers: [UnlitLayer;N], script: [f32;4], amount: f32, mode: &str) -> Result<texture::Image> {
    if sources.iter().any(|source| source.depth != 1 || source.faces != 1 || source.width == 0 || source.height == 0 || source.levels.is_empty()) {
        return Err("Three-layer falloff input is not a valid 2D image".into());
    }
    let (width,height) = (sources.iter().map(|image| image.width).max().unwrap(),sources.iter().map(|image| image.height).max().unwrap());
    let mut output = texture::Image {width,height,depth:1,faces:1,format:19,flags:0,levels:Vec::new()};
    let decoded = sources.iter().map(|source| (0..source.levels.len()).map(|level| texture::pixels(source,level)).collect::<Result<Vec<_>>>()).collect::<Result<Vec<_>>>()?;
    let rotation = layers.map(|layer| (layer.rotation[0]*f32::from_bits(0x40c90fda)).sin_cos());
    for level in 0..sources.iter().map(|source| source.levels.len()).max().unwrap() {
        let (w,h) = ((width>>level).max(1),(height>>level).max(1));
        let mut bytes = Vec::with_capacity(w*h*8);
        for y in 0..h {for x in 0..w {
            let uv = [(x as f32+0.5)/w as f32,(y as f32+0.5)/h as f32];
            let samples: [[f32;4];N] = std::array::from_fn(|index| {
                let source = sources[index];let layer = layers[index];let (sine,cosine) = rotation[index];
                let centered: [f32;2] = std::array::from_fn(|axis| (uv[axis]-layer.origin[axis]+layer.offset[axis])*layer.scale[axis]);
                let uv = [cosine*centered[0]+sine*centered[1]+layer.origin[0],-sine*centered[0]+cosine*centered[1]+layer.origin[1]];
                let bias = width.max(height).ilog2()-source.width.max(source.height).ilog2();
                let lod = level.saturating_sub(bias as usize).min(source.levels.len()-1);
                texture::sample(&decoded[index][lod],(source.width>>lod).max(1),(source.height>>lod).max(1),uv,samplers[index])
            });
            let mut pixel = if mode == "four_add" {
                four_layer_pixel([samples[0],samples[1],samples[2],samples[3]],
                    [layers[0].alpha.unwrap(),layers[1].alpha.unwrap(),layers[2].alpha.unwrap(),layers[3].alpha.unwrap()],script)
            } else {
                falloff_pixel([samples[0],samples[1],samples[2]],[layers[0].alpha.unwrap(),layers[1].alpha.unwrap()],layers[2].alpha,script,
                    if mode == "layer_blend" {"blend"} else {mode})
            };
            for component in &mut pixel[..3] {*component=(*component*amount).powi(2);}
            if pixel.iter().any(|value| !value.is_finite() || !(0. ..=65504.).contains(value)) || pixel[3] > 1. {
                return Err("Three-layer falloff color exceeds the finite half-float/coverage domain".into());
            }
            for component in pixel {bytes.extend(half::f16::from_f32(component).to_le_bytes());}
        }}
        output.levels.push(bytes);
    }
    Ok(output)
}

fn convert_falloff(key: &str, native: &Value, images: &BTreeMap<String,texture::Image>, shared: &mut BTreeMap<(String,String),String>, pack: &mut Pack) -> Result<Value> {
    let (index,mode) = validate_falloff(key,native)?;
    let four = mode == "four_add";
    let plain = four || mode == "layer_blend";
    let constant = |hash: u32| -> Result<[f32;4]> {
        let row = native["constants"].as_array().unwrap().iter().find(|constant| constant["hash"] == hash).ok_or("Missing bound three-layer falloff parameter")?;
        Ok(serde_json::from_value(row["literal"].clone())?)
    };
    let vec2 = |hash| -> Result<[f32;2]> {let value = constant(hash)?;Ok([value[0],value[1]])};
    let mut parameter_hashes = vec![
        [3320325482,3286424575,3272022534,1886964550,2196952607,3342767871,3337236870],
        [3354717129,128962428,114353733,1519946245,3332229724,3379389788,3376302501],
        [3398313000,1565193213,1550647684,1339484996,475721245,3416155453,3410903492],
    ];
    if four {parameter_hashes.push([3736587663,3836749946,3813076419,2389732611,3048924506,3756660378,3762764771]);}
    let layers = parameter_hashes.into_iter().enumerate().map(|(index,hashes)| Ok(UnlitLayer {
        scale:vec2(hashes[0])?,offset:vec2(hashes[1])?,origin:vec2(hashes[2])?,rotation:vec2(hashes[3])?,scroll:vec2(hashes[4])?,depth:constant(hashes[5])?[0],
        alpha:if index == 2 && mode == "opaque" {None} else {Some(constant(hashes[6])?[0])},
    })).collect::<Result<Vec<_>>>()?;
    let mut sources = Vec::new();let mut samplers = Vec::new();let mut source_layers = Vec::new();let mut names = Vec::new();
    let mut hashes = vec![3714339118_u32,3714374925,3714406764];
    if four {hashes.push(3714721355);}
    for (index,hash) in hashes.into_iter().enumerate() {
        let binding = native["textures"].as_array().unwrap().iter().find(|binding| binding["hash"] == hash).unwrap();
        let name = binding["image"].as_str().ok_or("Missing three-layer falloff source image name")?.to_ascii_lowercase();
        let source = images.get(&name).ok_or_else(|| format!("Missing three-layer falloff image {name}"))?;
        let sampler = binding["sampler"].as_u64().unwrap() as u8;
        let source_key = (name.clone(),"source_color".into());
        let uri = if let Some(uri) = shared.get(&source_key) {uri.clone()} else {
            let uri = store_image(source,format!("prepared/material-channels/{}/source-color.ktx2",shared.len()),false,"source_color",pack)?;
            shared.insert(source_key,uri.clone());uri
        };
        let layer = layers[index];
        source_layers.push(json!({"uri":uri,"layer":index,"sampler":sampler,"color_space":"native_encoded",
            "scale":layer.scale,"offset":layer.offset,"origin":layer.origin,"rotation":layer.rotation,"scroll":layer.scroll,"depth":layer.depth,"alpha":layer.alpha}));
        sources.push(source);samplers.push(sampler);names.push(name);
    }
    let script = if native["code_constants"]["169"].is_null() {if plain {Some([0.;4])} else {None}} else {
        let state: [f32;4] = serde_json::from_value(native["code_constants"]["169"].clone())?;
        if state.iter().any(|value| !value.is_finite()) {return Err("Non-finite source scriptVector0 state".into());}Some(state)
    };
    let amount = constant(14836402)?[0];
    let mut channels = Vec::new();
    if let Some(script) = script {
        let color_key = (names.join("|"),format!("falloff:{mode}:{layers:?}:{script:?}:{}:{samplers:?}",amount.to_bits()));
        let uri = if let Some(uri) = shared.get(&color_key) {uri.clone()} else {
            let image = if four {
                falloff_image([sources[0],sources[1],sources[2],sources[3]],[samplers[0],samplers[1],samplers[2],samplers[3]],
                    [layers[0],layers[1],layers[2],layers[3]],script,amount,mode)?
            } else {
                falloff_image([sources[0],sources[1],sources[2]],[samplers[0],samplers[1],samplers[2]],
                    [layers[0],layers[1],layers[2]],script,amount,mode)?
            };
            let uri = store_image(&image,format!("prepared/material-channels/{}/color.ktx2",shared.len()),false,"color",pack)?;
            shared.insert(color_key,uri.clone());uri
        };
        channels.push(json!({"uri":uri,"role":"color","layer":0,"color_space":"linear","sampler":20}));
    }
    let selected = native["state_entries"][index].as_u64().unwrap() as usize;
    let state = native["state_bits"][selected][0].as_u64().unwrap();
    let cull = match state>>14 & 3 {1 => "none",2 => "back",3 => "front",_ => return Err("Unknown three-layer falloff cull state".into())};
    let mut surface = json!({"version":3,"identity":key,"native":native,"channels":channels,"source_layers":source_layers,
        "standard":{"unlit":true,"roughness":1.,"metallic":0.,"reflectance":0.,"base_color":[1.,1.,1.,1.]},
        "presentation":{"alpha":if mode == "zfeather" || four {"add"} else if mode == "layer_blend" {"blend"} else {mode},"cull":cull,"fog":false,"visible":state & (1<<27) == 0},
        "source_script_layers":{"code_constant":169,"state":script,"layer_order":if four {vec![3,2,1,0]} else {vec![2,1,0]},
            "weights":"texture_alpha_times_layer_alpha_times_one_minus_saturated_script_component",
            "rgb":if four {"layer4_rgb_times_layer4_texture_alpha_then_mix_layer3_then_layer2_then_layer1_without_saturating_mix_weights"} else {"layer3_rgb_times_layer3_texture_alpha_then_mix_layer2_then_layer1_without_saturating_mix_weights"},
            "coverage":if mode == "opaque" {"unused"} else {"add_each_preceding_layer_weight_and_saturate_after_each_step"},
            "layer3_alpha_affects_rgb":four,"layer3_alpha_bound":mode != "opaque","last_layer_alpha_affects_rgb":false,
            "rgb_times_coverage":matches!(mode,"add" | "zfeather" | "four_add"),
            "fragment_alpha":if matches!(mode,"blend" | "layer_blend") {"sqrt_coverage"} else {"one"}},
        "source_layer_uv":{"game_time_code":25,"game_time_component":3,"view_parallax":"negative_view_tangent_xy_divided_by_positive_view_normal_z_else_zero",
            "depth_epsilon":if four {f32::from_bits(0x3727c5ac)} else {0.},"evaluation":if four {"vertex_then_interpolate"} else {"fragment"},
            "operation":"scale_centered_uv_plus_scaled_offset_and_scaled_view_parallax_then_rotate_then_add_scaled_time_scroll_and_origin",
            "rotation_operation":"rotation_x_times_one_plus_rotation_y_times_game_time_w","rotation_tau":f32::from_bits(0x40c90fda)},
        "source_emissive":{"amount":amount,"rgb":if plain {"square_layer_color_times_hdr_amount"} else {"square_layer_color_times_view_falloff_times_hdr_amount"},"output_transfer":"sqrt_after_reflection_and_scene_hdr_control0"},
        "source_blend":{"rgb":[state&15,state>>4&15,state>>8&7],"alpha":[state>>16&15,state>>20&15,state>>24&7]},
        "approximations":["Explicit scriptVector0 states evaluate the three-layer color and coverage with static authored UV terms at source mip resolution",
            "Live view parallax, time, view falloff, scene lighting and framebuffer terms remain separate from the converted texture stage"]});
    if !plain {surface["source_falloff"] = json!({"power":constant(3185247991)?[0],"boost":constant(3173127725)?[0],
        "operation":"pow_saturate_dot_negative_normalized_view_position_and_normalized_normal_times_boost"});}
    if plain {
        surface["source_script_layers"]["base_state"] = json!([0.,0.,0.,0.]);
        surface["source_script_layers"]["base_state_origin"] = json!("normal_view_zero_template_preserved_by_type2_constant_copy");
        surface["source_script_layers"]["active_override_scope"] = json!("per_object_mapped_scriptVector0_replaces_base_state");
    }
    if mode == "zfeather" {
        surface["source_vertex_color_transfer"] = json!("linear_primary_vertex_rgb_and_squared_primary_vertex_alpha");
        surface["source_depth_fade"] = json!({"code_image":18,"near_plane_code":17,"fade_factor":constant(3371644427)?[0],
            "depth_epsilon":f32::from_bits(0x322bcc77),"fade_epsilon":f32::from_bits(0x38d1b717),
            "operation":"saturate_near_plane_divided_by_max_abs_scene_depth_minus_clip_w_divided_by_max_fade_factor",
            "rgb":"layer_color_times_coverage_times_vertex_rgb_times_vertex_alpha_squared_times_depth_fade"});
        surface["pending_runtime"] = json!("Native three-layer feather requires the live per-object scriptVector0 producer, native clock/offset UV controls, view parallax and falloff, primary vertex RGB/alpha, scene depth/near-plane/clip-w fade, global HDR control0 and native additive/destination-alpha gamma2 framebuffer composition");
    } else {
        surface["source_vertex_color_transfer"] = json!("unused_primary_color");
        surface["source_reflection"] = json!({"amount":constant(211931156)?[0],"gloss":constant(3432544439)?[0],"lod":"four_times_one_minus_gloss",
            "weight":"saturate_three_quarters_plus_quarter_gloss_times_reflection_amount","probe_decode":"rgb_divided_by_alpha_plus_epsilon",
            "decode_epsilon":f32::from_bits(0x358637bd),"response":"separate_grid_and_reflection_spherical_harmonics_ratio",
            "secondary_vertex_alpha_red_bias":if native["technique_set"].as_str().unwrap().starts_with("mlv_") {0.00001} else {0.}});
        surface["source_reflection"]["active"] = json!(constant(211931156)?[0] != 0.);
        surface["pending_runtime"] = json!(if four {
            "Native four-layer no-fog surface retains per-object scriptVector0 overrides, native clock UV rotation/scroll, vertex-evaluated positive-facing parallax including its depth epsilon, global HDR control0 and native additive/destination-alpha gamma2 composition; nonzero reflection requires its separate view-dependent SH consumer"
        } else if plain {
            "Native three-layer no-fog surface retains per-object scriptVector0 overrides, native clock UV rotation, nonzero view-dependent reflection/SH response, global HDR control0 and native source-alpha/destination-alpha gamma2 composition; authored zero layer depths leave parallax inactive"
        } else {"Native three-layer falloff requires the live per-object scriptVector0 producer, native clock/offset UV controls, view parallax and falloff, view-dependent reflection/SH response, global HDR control0 and native blend/destination-alpha gamma2 framebuffer composition"});
    }
    surface["source_script_layers"]["composed_color_available"] = json!(script.is_some());
    if four && native["state_bits"][selected][1] == 28 {
        let pending = surface["pending_runtime"].as_str().unwrap().to_owned();
        surface["pending_runtime"] = json!(format!("{pending}; native polygon-offset mode1 remains unconnected"));
    }
    Ok(json!({"name":key,"pbrMetallicRoughness":{"baseColorFactor":[1.,1.,1.,1.],"metallicFactor":0.},"extras":{"surface":surface}}))
}

pub fn selected_technique(family: &str) -> Option<usize> {
    if let Some((index,..)) = falloff_recipe(family) {Some(index)}
    else if let Some((index,..)) = single_unlit_recipe(family) {Some(index)}
    else if unlit_recipe(family).is_some() {Some(2)}
    else if ordinary_recipe(family).is_some() {Some(4)} else {None}
}

fn validate_single_unlit(key: &str, material: &Value) -> Result<(usize,u32,&'static str)> {
    let family = material["technique_set"].as_str().ok_or("Missing single-layer unlit family")?;
    let (index,vertex,pixel,image,mode) = single_unlit_recipe(family).ok_or("Unreviewed single-layer unlit family")?;
    let layered = index == 4;
    let eyes = mode == "eyes";
    let mut routes = vec![json!({"source":0,"destination":0}),json!({"source":2,"destination":5})];
    if layered {routes.extend([json!({"source":3,"destination":1}),json!({"source":4,"destination":7})]);}
    let mut constants = BTreeMap::new();
    if layered {
        constants.extend([(1886964550_u32,(0,112,8)),(2196952607,(0,96,8)),(3272022534,(0,120,8)),
            (3286424575,(0,88,8)),(3320325482,(0,80,8)),(3342767871,(0,128,4)),
            (14836402,(6,108,4)),(3337236870,(6,104,4))]);
        if mode == "layer_reflection" {constants.extend([(211931156,(6,136,4)),(3432544439,(6,132,4))]);}
    } else if !eyes {constants.insert(2875883048,(6,80,4));}
    let indices = if layered {vec![4,6]} else {vec![3]};
    let techniques = material["techniques"].as_array().ok_or("Missing single-layer unlit techniques")?;
    let state_index = material["state_entries"][index].as_u64().ok_or("Missing single-layer unlit state index")? as usize;
    let state = &material["state_bits"][state_index];
    let rgb = state[0].as_u64().ok_or("Missing single-layer unlit RGB state")?;
    let depth = state[1].as_u64().ok_or("Missing single-layer unlit depth state")?;
    let (blend,alpha) = match mode {"screen" => (0x12a,0x121),"add" => (0x122,0x121),"eyes" => (0x12,0x12),_ => (0x165,0x128)};
    if rgb & 0x7ff != blend || rgb >> 16 & 0x7ff != alpha || depth & !0x30 != if eyes {13} else {12} {
        return Err(format!("Single-layer unlit material {key} has unreviewed blend or depth state").into());
    }
    for index in indices {
        let technique = techniques.iter().find(|technique| technique["index"] == index).ok_or("Missing selected single-layer unlit technique")?;
        let passes = technique["passes"].as_array().ok_or("Missing single-layer unlit passes")?;
        if passes.len() != 1 || passes[0]["vertex_shader"] != vertex || passes[0]["pixel_shader"] != pixel
            || passes[0]["vertex_routes"] != json!(routes) {
            return Err(format!("Single-layer unlit material {key} has unreviewed programs or vertex routes").into());
        }
        let selected = material["state_entries"][index].as_u64().ok_or("Missing selected single-layer unlit state index")? as usize;
        if material["state_bits"][selected] != *state {return Err(format!("Single-layer unlit material {key} has differing selected render states").into());}
        let arguments = passes[0]["arguments"].as_array().ok_or("Missing selected single-layer unlit arguments")?;
        if matches!(mode,"eyes" | "alpha") {
            let expected_state = if eyes {json!([403867666_u32,13])} else {json!([422084965_u32,12])};
            if arguments.len() != 4 || !arguments.iter().any(|argument| argument["type"] == 3 && argument["value"] == 67109077_u32
                    && argument["buffer"] == 3 && argument["location"] == 0 && argument["size"] == 64)
                || !arguments.iter().any(|argument| argument["type"] == 3 && argument["value"] == 67109093_u32
                    && argument["buffer"] == 0 && argument["location"] == 576 && argument["size"] == 64)
                || *state != expected_state {
                return Err(format!("Selected eye/alpha material {key} has unreviewed matrices or render state").into());
            }
        }
        let images: Vec<_> = arguments.iter().filter(|argument| argument["type"] == 2).collect();
        if images.len() != 1 || images[0]["value"] != image || images[0]["buffer"] != 0
            || images[0]["location"] != 256 || images[0]["size"] != 1 {
            return Err(format!("Single-layer unlit material {key} has unreviewed image bindings").into());
        }
        let active: Vec<_> = arguments.iter().filter(|argument| matches!(argument["type"].as_u64(),Some(0 | 1 | 6 | 7 | 8))).collect();
        if active.len() != constants.len() {return Err(format!("Single-layer unlit material {key} has unreviewed material argument count").into());}
        for (&hash,&(kind,location,size)) in &constants {
            if active.iter().filter(|argument| argument["type"] == kind && argument["value"] == hash
                && argument["buffer"] == 2 && argument["location"] == location && argument["size"] == size).count() != 1 {
                return Err(format!("Single-layer unlit material {key} has unreviewed constant {hash}").into());
            }
            let constant = material["constants"].as_array().ok_or("Missing single-layer unlit constants")?.iter()
                .find(|constant| constant["hash"] == hash).ok_or("Missing bound single-layer unlit constant")?;
            if (0..size as usize/4).any(|component| constant["literal"][component].as_f64().is_none_or(|value| !value.is_finite())) {
                return Err(format!("Single-layer unlit material {key} has non-finite constant {hash}").into());
            }
        }
        if layered && (!arguments.iter().any(|argument| argument["type"] == 3 && argument["value"] == 16777241_u32
                && argument["buffer"] == 2 && argument["location"] == 64 && argument["size"] == 16)
            || !arguments.iter().any(|argument| argument["type"] == 5 && argument["value"] == 16777385_u32
                && argument["buffer"] == 1 && argument["location"] == 368 && argument["size"] == 16)) {
            return Err(format!("Single-layer unlit material {key} lacks reviewed gameTime or scriptVector0 binding").into());
        }
        if eyes && (arguments.len() != 4 || arguments.iter().filter(|argument| argument["type"] == 5
            && argument["value"] == 16777387_u32 && argument["buffer"] == 1 && argument["location"] == 400 && argument["size"] == 16).count() != 1) {
            return Err(format!("Cheap eye material {key} lacks its exact scriptVector2 luminance binding").into());
        }
    }
    let texture = material["textures"].as_array().ok_or("Missing single-layer unlit images")?.iter()
        .find(|binding| binding["hash"] == image).ok_or("Missing single-layer unlit source image")?;
    if texture["sampler"] != 20 {return Err(format!("Single-layer unlit material {key} has unreviewed sampler").into());}
    Ok((index,image,mode))
}

pub fn inputs(native: &BTreeMap<String, Value>, catalog: &Catalog<'_>) -> Result<std::collections::BTreeSet<String>> {
    let mut required = std::collections::BTreeSet::new();
    for (key,material) in native {
        let family = material["technique_set"].as_str().ok_or("Missing native material technique")?;
        let index = if crate::bake::supported(family) {crate::bake::validate_native(material)?;4}
        else if falloff_recipe(family).is_some() {validate_falloff(key,material)?.0}
        else if single_unlit_recipe(family).is_some() {validate_single_unlit(key,material)?.0}
        else if let Some((pixel,alpha)) = unlit_recipe(family) { validate_unlit(key,material,pixel,alpha)?; 2 } else {
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
        if falloff_recipe(family).is_some() {
            converted.insert(key.clone(),convert_falloff(key,material,images,&mut shared,pack)?);
            continue;
        }
        if single_unlit_recipe(family).is_some() {
            converted.insert(key.clone(),convert_single_unlit(key,material,images,&mut shared,pack)?);
            continue;
        }
        if let Some((pixel,alpha)) = unlit_recipe(family) {
            converted.insert(key.clone(),convert_unlit(key,material,pixel,alpha,images,&mut shared,pack)?);
            continue;
        }
        let Some(recipe) = ordinary_recipe(family) else {
            missing.entry(family.into()).or_default().push(key.clone()); continue;
        };
        validate_ordinary(key,material,&recipe)?;
        let OrdinaryRecipe {normal,specular,coverage,detail,occlusion,camo,..} = recipe;
        let reveal = matches!(family,"mc_lit_sm_b0c0n0v0_3559e8jj" | "wpc_lit_sm_b0c0n0v0_ew347fe3" | "wpc_lit_sm_b0c0n0s0v0_efz5w56f");
        let flag = matches!(family,"mc_lit_sm_flag_t0c0d0n0s0_w75eqzzj" | "mc_lit_sm_flag_b0c0s0_6406ez95");
        let lit = material["techniques"].as_array().unwrap().iter().find(|technique| technique["index"] == 4).unwrap();
        let passes = lit["passes"].as_array().unwrap();
        let mut channels = Vec::new();
        let camo_parameters = if camo && !material["code_constants"]["181"].is_null() {
            let value: [f32;4] = serde_json::from_value(material["code_constants"]["181"].clone())?;
            if !value.iter().all(|component| component.is_finite()) { return Err(format!("Invalid camo parameters for {key}").into()); }
            Some(value)
        } else { None };
        let mut source_camo = Value::Null;
        let mut source_inputs = BTreeMap::new();
        if camo || occlusion != 0 || detail || reveal {
            let mut inputs = Vec::new();
            if camo { inputs.extend([(2695565377_u32,"source_color"),(3183121296,"source_camo")]); }
            if reveal { inputs.push((2695565377,"source_color")); }
            if occlusion != 0 { inputs.push((1610742065,"source_occlusion")); }
            if detail { inputs.push((3948059469,"source_detail_normal")); }
            for (hash,role) in inputs {
                let binding = material["textures"].as_array().unwrap().iter().find(|binding| binding["hash"] == hash)
                    .ok_or_else(|| format!("Missing source camo input {hash} for {key}"))?;
                let name = binding["image"].as_str().ok_or("Missing source camo image")?.to_ascii_lowercase();
                let image = images.get(&name).ok_or_else(|| format!("Missing source camo image {name}"))?;
                let channel_key = (name,role.into());
                let uri = if let Some(uri) = shared.get(&channel_key) { uri.clone() } else {
                    let uri = store_image(image,format!("prepared/material-channels/{}/{role}.ktx2",shared.len()),false,role,pack)?;
                    shared.insert(channel_key,uri.clone()); uri
                };
                source_inputs.insert(role,json!({"uri":uri,"sampler":binding["sampler"],"transfer":if matches!(role,"source_color" | "source_camo") {"encoded_gamma2"} else {"linear"}}));
            }
            if camo {
                source_camo = json!({"inputs":{"source_color":source_inputs["source_color"],"source_camo":source_inputs["source_camo"]},
                    "code_constant":181,"parameters":camo_parameters,"color_bias":if occlusion == 1 {"occlusion_blue"} else {"one"},
                    "operation":"alpha_masked_gamma_color_and_specular","uv":"center_scale_offset"});
            }
        }
        let bindings = material["textures"].as_array().unwrap();
        let detail_input = if detail {
            let binding = bindings.iter().find(|value| value["hash"] == 3948059469_u32).ok_or("Missing bound detail normal")?;
            let constant = material["constants"].as_array().unwrap().iter().find(|value| value["hash"] == 148072969_u32)
                .ok_or("Missing bound detailScale")?;
            let parameters: [f32;4] = serde_json::from_value(constant["literal"].clone())?;
            let name = binding["image"].as_str().unwrap().to_ascii_lowercase();
            Some((images.get(&name).ok_or_else(|| format!("Missing detail normal image {name}"))?,binding["sampler"].as_u64().unwrap() as u8,parameters))
        } else { None };
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
                let mut identity = image_name.to_ascii_lowercase();
                if role == "normal" && let Some((_,sampler,parameters)) = detail_input {
                    let binding = material["textures"].as_array().unwrap().iter().find(|value| value["hash"] == 3948059469_u32).unwrap();
                    identity.push_str(&format!(":{}:{sampler}:{:?}",binding["image"].as_str().unwrap().to_ascii_lowercase(),parameters.map(f32::to_bits)));
                }
                let channel_key = (identity, role.to_owned());
                let srgb = role == "color";
                let uri = if let Some(uri) = shared.get(&channel_key) { uri.clone() } else {
                    let image = bake(source, role,if role == "normal" {detail_input} else {None})?;
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
            let mut pair = format!("{}:{}:{}:{}",names[0],names[1],samplers[0],samplers[1]);
            let occlusion_input = if camo && occlusion != 0 && camo_parameters.is_some_and(|parameters| parameters[3] != 0.) {
                let binding = bindings.iter().find(|value| value["hash"] == 1610742065_u32).ok_or("Missing bound camo occlusion")?;
                let name = binding["image"].as_str().ok_or("Missing bound camo occlusion image")?.to_ascii_lowercase();
                let sampler = binding["sampler"].as_u64().ok_or("Missing bound camo occlusion sampler")? as u8;
                pair.push_str(&format!(":occlusion:{name}:{sampler}"));
                Some((images.get(&name).ok_or_else(|| format!("Missing bound camo occlusion image {name}"))?,sampler))
            } else { None };
            let camo_input = if let Some(parameters) = camo_parameters.filter(|parameters| parameters[3] != 0.) {
                let binding = bindings.iter().find(|value| value["hash"] == 3183121296_u32).ok_or("Missing bound camo image")?;
                let name = binding["image"].as_str().ok_or("Missing bound camo image name")?.to_ascii_lowercase();
                let sampler = binding["sampler"].as_u64().ok_or("Missing bound camo sampler")? as u8;
                pair.push_str(&format!(":{name}:{sampler}:{:?}",parameters.map(f32::to_bits)));
                Some((images.get(&name).ok_or_else(|| format!("Missing bound camo image {name}"))?,sampler,parameters))
            } else { None };
            if !shared.contains_key(&(pair.clone(),"color".into())) {
                let (base,response) = metallic_workflow(&images[&names[0]],&images[&names[1]],samplers,camo_input,occlusion_input)?;
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
                "native_specular":native_specular,"source_camo":source_camo,
                "approximations":["Khronos specular-glossiness to metallic-roughness fit for shared upstream PBR and Solari inputs", "Native Blinn-Phong gloss fitted to GGX perceptual roughness; original specular and gloss retained", "Slope reconstruction baked before filtering", "Native lighting-term multipliers retained for runtime integration", "Vertex gamma2 transfer evaluated before interpolation"]}}}));
        if family == "mc_lit_sm_b0c0n0s0_21fz2e61" {
            converted.get_mut(key).unwrap()["extras"]["surface"]["source_fragment_alpha"] = json!("sqrt_texture_vertex_coverage");
            converted.get_mut(key).unwrap()["extras"]["surface"]["pending_runtime"] = json!("Native glass fragment uses sqrt coverage and coverage-independent specular before gamma2 premultiplied composition; the shared linear PBR consumer does not yet reproduce that blend");
        }
        let surface = &mut converted.get_mut(key).unwrap()["extras"]["surface"];
        if family.starts_with("mlv_") {
            surface["source_vertex_lighting"] = json!({"source":5,"destination":6,
                "encoded_rgb_scale":32.,"transfer":"gamma2_before_vertex_interpolation",
                "diffuse":{"operation":"divide_by_grid_spherical_harmonics","buffer":3,"rows":[5,6,7],"minimum":0.1},
                "specular":{"operation":"divide_by_reflection_spherical_harmonics","buffer":3,"rows":[8,9,10],"minimum":0.1},
                "alpha":"sun_shadow_fallback","stage":"separate_indirect_lighting_terms"});
            surface["pending_runtime"] = json!("Native vertex-lit material requires its secondary vertex RGB divided by separate grid/reflection spherical-harmonic responses before interpolation, separate indirect lighting multipliers and secondary-alpha sun-shadow fallback; geometry delivery and runtime lighting consumers remain unconnected");
            if specular == 2 {
                surface["source_ambient_specular"] = json!({"gloss_scale":0.0275,"bias":0.015,"fresnel_cap_scale":0.475});
            }
            if coverage == 2 { surface["source_framebuffer_alpha"] = json!("texture_vertex_coverage_after_cutoff"); }
            if coverage == 1 {
                surface["source_fragment_alpha"] = json!("sqrt_texture_vertex_coverage");
                surface["pending_runtime"] = json!("Native translucent vertex-lit material requires separate vertex grid/probe multipliers and secondary-alpha shadow fallback, plus sqrt coverage and coverage-scaled fog in gamma2 premultiplied composition; geometry and runtime consumers remain unconnected");
            }
            if family == "mlv_lit_sm_r0c0_475q06w3" {
                surface["source_vertex_alpha"] = json!("indirect_diffuse");
                surface["pending_runtime"] = json!("Native vertex-lit cheap sandbag additionally attenuates indirect diffuse with primary vertex alpha while retaining opaque coverage and unattenuated sun lighting; separate vertex grid/probe and shadow-fallback consumers remain unconnected");
            }
            if family == "mlv_lit_sm_r0c0n0x0_8f47740f" {
                surface["source_vertex_alpha"] = json!("indirect_diffuse_and_specular");
                surface["pending_runtime"] = json!("Native vertex-lit sandbag multiplies separate grid and reflection lighting by primary vertex alpha while retaining opaque coverage and unattenuated sun lighting; secondary vertex lighting, shadow fallback and gloss-dependent ambient specular consumers remain unconnected");
            }
        }
        if reveal {
            let parameters = &material["constants"].as_array().unwrap().iter().find(|value| value["hash"] == 2932833280_u32).unwrap()["literal"];
            surface["source_coverage"] = json!("gamma2_reveal_premultiplied");
            surface["source_reveal"] = json!({"input":source_inputs["source_color"],"parameters":parameters,
                "vertex_channel":"alpha","vertex_scale":f32::from_bits(0x3f7f7cee),"vertex_bias":f32::from_bits(0x3a83126f),
                "operation":"saturate_texture_alpha_between_vertex_power_thresholds"});
            surface["source_fragment_alpha"] = json!("sqrt_reveal_coverage");
            surface["pending_runtime"] = json!("Native alpha reveal uses interpolated vertex alpha and authored power thresholds before coverage, sqrt fragment alpha and gamma2 premultiplied fog composition; geometry-dependent reveal baking and matching runtime composition remain unconnected");
        }
        if let Some((_,_,parameters)) = detail_input {
            surface["source_detail_normal"] = json!({"input":source_inputs["source_detail_normal"],"parameters":parameters,
                "operation":"scaled_slope_add_before_normalization","uv":"multiply_xy"});
            surface["approximations"].as_array_mut().unwrap().push(json!("Detail normal slopes baked at base-normal resolution using a major-axis mip estimate before output filtering"));
        }
        if flag {
            let parameters = &material["constants"].as_array().unwrap().iter().find(|value| value["hash"] == 2651392336_u32).unwrap()["literal"];
            let phase_uv = [[0x4067e015_u32,0x3e81b6e4].map(f32::from_bits),
                [0x41040141_u32,0xbfba354d].map(f32::from_bits),[0x419c9f28_u32,0x40a7dddc].map(f32::from_bits)];
            let phase_time = [0x40b4f4ab_u32,0x4116cbe4,0x40f1463a].map(f32::from_bits);
            let normal_cosine_u = [0x41d96214_u32,0x41c601e2,0x420520e2].map(f32::from_bits);
            let normal_cosine_v = [0x3ff336eb_u32,0xc08ba7f9,0x410eafc8].map(f32::from_bits);
            surface["source_flag_motion"] = json!({"parameters":parameters,"parameter_hash":2651392336_u32,
                "time":{"code_constant":25,"component":3,"operation":"add_parameter_y_then_multiply_parameter_x"},
                "uv":"one_minus_uv0","vertex_alpha":"motion_amplitude_only","fragment_vertex_alpha":1.,
                "phase_uv":phase_uv,"phase_time":phase_time,
                "displacement_amplitude":[7.5,3.,f32::from_bits(0x3fd9999a)],"displacement_axis":"decoded_model_normal",
                "normal_cosine_u":normal_cosine_u,"normal_cosine_v":normal_cosine_v,
                "normal_derivative_scale":f32::from_bits(0x3ccccccd),
                "normal_operation":"add_vertex_alpha_scaled_cosine_derivatives_in_source_tangent_basis_then_normalize"});
            surface["source_coverage"] = json!(if coverage == 2 {"texture_mask_without_vertex_alpha"} else {"gamma2_texture_premultiplied_without_vertex_alpha"});
            if coverage == 2 {
                surface["source_framebuffer_alpha"] = json!("texture_coverage_after_cutoff");
                surface["pending_runtime"] = json!("Native masked flag material requires gameTime.w-driven three-wave displacement and derivative-adjusted normals using vertex alpha, with fragment vertex alpha forced to one and texture alpha retained after cutoff; live geometry and framebuffer-alpha consumers remain unconnected");
            } else {
                surface["source_fragment_alpha"] = json!("sqrt_texture_coverage_without_vertex_alpha");
                surface["pending_runtime"] = json!("Native translucent flag material requires gameTime.w-driven three-wave displacement and derivative-adjusted normals using vertex alpha, with fragment vertex alpha forced to one, separate grid/reflection responses and sqrt texture coverage in gamma2 premultiplied composition; live geometry and matching lighting/composition remain unconnected");
            }
        }
        if occlusion != 0 {
            surface["source_occlusion"] = json!({"input":source_inputs["source_occlusion"],
                "indirect_diffuse":if occlusion == 1 {"red"} else {"blue"},"indirect_specular":"green",
                "direct_diffuse":if occlusion == 1 {"none"} else {"green"},"direct_specular":"red"});
            if camo {
                surface["source_occlusion"]["sun_specular_scale"] = json!({"hash":411508820,"component":2});
                surface["pending_runtime"] = json!("Native weapon camo needs live scoped weaponParam2/image overrides and separate red indirect-diffuse/direct-specular, green reflection and blue camo-bias occlusion consumers; packed source inputs and explicit-state conversion do not establish those runtime bindings");
            } else {
                surface["source_fragment_alpha"] = json!("sqrt_texture_vertex_coverage");
                surface["pending_runtime"] = json!("Native catwalk gamma2 premultiplied lighting writes sqrt coverage and applies distinct source occlusion channels to lighting terms; the shared PBR consumer does not yet reproduce that composition");
            }
        }
        if matches!(family,"mc_lit_sm_r0c0n0s0_ze50j7jz" | "mc_lit_sm_r0c0n0s0o0_909j993q" | "mc_lit_sm_t0c0n0s0_f933q84u" | "mc_lit_sm_b0c0n0s0_68f25z0u" | "mc_lit_sm_b0c0s0_q94f27qw") {
            surface["source_color_matrix"] = json!({"buffer":0,"rows":[114,115,116],"stage":"linear_lighting_before_fog"});
        }
        match family {
            "mc_lit_sm_t0c0n0s0_f933q84u" => {
                surface["source_framebuffer_alpha"] = json!("texture_vertex_coverage_after_cutoff");
                surface["pending_runtime"] = json!("Native masked Ray Gun material applies the model color matrix to linear lighting before fog and retains texture times vertex framebuffer alpha after cutoff; both runtime consumers remain unconnected");
            }
            "mc_lit_sm_b0c0n0s0_68f25z0u" | "mc_lit_sm_b0c0s0_q94f27qw" => {
                surface["source_fragment_alpha"] = json!("sqrt_texture_vertex_coverage");
                surface["source_color_matrix"]["stage"] = json!("coverage_scaled_linear_lighting_before_fog");
                surface["pending_runtime"] = json!("Native translucent model material applies coverage to diffuse plus specular before its affine color matrix, then coverage-scaled fog and sqrt fragment alpha in gamma2 premultiplied composition; matching runtime composition remains unconnected");
            }
            "mc_lit_sm_r0c0n0s0_ze50j7jz" => {
                surface["pending_runtime"] = json!("Native model color matrix must be applied to linear diffuse plus specular before fog; its runtime parameter binding is not connected");
            }
            "mc_lit_sm_r0c0n0s0_7j414q0z" => {
                if camo_parameters.is_none() {
                    surface["pending_runtime"] = json!("Native attachment camo requires a bound per-object submission; unresolved world, dual-hand and alternate-weapon inputs prevent pack closure");
                } else {
                    surface["camo_binding"] = json!("prepared_native_input_state");
                }
            }
            "mc_lit_sm_r0c0n0s0_4w3wqfe5" => {
                surface["source_uv_animation"] = json!({"code_constant":25,"component":2,"translation":[0.,-1.]});
                surface["uv_animation"] = json!({"translation":[0.,-1.],"time_mode":"native_fractional_second"});
                surface["pending_runtime"] = json!("Native tank-tread UV binding subtracts gameTime.z from V on every channel, but the source render-game milliseconds and per-object shader-start inputs still require a gameplay producer; explicit setup inputs do not establish native runtime clock delivery");
            }
            "mc_lit_sm_r0c0n0x0_53144qq1" => {
                surface["source_vertex_alpha"] = json!("indirect_diffuse_and_specular");
                surface["source_ambient_specular"] = json!({"gloss_scale":0.0275,"bias":0.015,"fresnel_cap_scale":0.475});
                surface["pending_runtime"] = json!("Native sandbag vertex alpha attenuates indirect diffuse and specular without attenuating sun lighting; the source gloss-dependent ambient response also remains distinct from the shared direct-light PBR fit");
            }
            "mc_lit_sm_r0c0_5137qq99" => {
                surface["source_vertex_alpha"] = json!("indirect_diffuse");
                surface["pending_runtime"] = json!("Native cheap sandbag vertex alpha attenuates indirect diffuse without changing coverage or sun lighting; the runtime lighting consumer is not connected");
            }
            "mc_lit_sm_t0c0d0n0s0_68eww765" => {
                surface["source_framebuffer_alpha"] = json!("texture_vertex_coverage_after_cutoff");
                surface["pending_runtime"] = json!("Native masked detail material writes texture times vertex coverage after the 128/255 cutoff; the shared masked consumer forces framebuffer alpha to one");
            }
            _ => (),
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
        selected_technique(family).is_some() || crate::bake::supported(family)
    })
}

pub fn read(catalog: &Catalog<'_>, name: &str) -> Result<Value> {
    let (zone, root) = catalog.resolve(6, name)?;
    let streams = &catalog.zones[zone].1.streams;
    let (_, technique_zone, technique) = catalog.pointer(zone, root.add(92)?, 7)?;
    let technique_streams = &catalog.zones[technique_zone].1.streams;
    let family = technique_streams.text(Address::decode(technique_streams.u32(technique)?)?)?;
    let mut techniques = Vec::new();
    for index in [2, 3, 4, 6] {
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

use crate::camo::{CamoPattern, CamoRecord, CamoDefinition, CamoOption};

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

pub fn read_material_selection(catalog: &Catalog<'_>) -> Result<(BTreeMap<String,crate::camo::WeaponMaterials>, BTreeMap<String,crate::camo::AttachmentMaterials>)> {
    let weapon_name = |zone: usize,field: Address| -> Result<Option<String>> {
        let streams = &catalog.zones[zone].1.streams;
        let pointer = streams.u32(field)?;
        if pointer == 0 { return Ok(None); }
        let name = streams.text(Address::decode(pointer)?)?;
        let name = name.trim_start_matches(',');
        Ok((!name.is_empty()).then(|| format!("weapon.{}",name.to_ascii_lowercase())))
    };
    let mut attachments = BTreeMap::new();
    for (name,zone,root) in catalog.definitions(30) {
        let streams = &catalog.zones[zone].1.streams;
        attachments.insert(name.to_ascii_lowercase(),crate::camo::AttachmentMaterials {
            camo:camo_asset(catalog,zone,root.add(164)?,31)?, attachment_type:streams.u32(root.add(4)?)?,
            chain_next:streams.u32(root.add(8)?)?,chain_head:streams.u32(root.add(12)?)?, combination_mask:streams.u32(root.add(16)?)?,
            alternate_weapon_index:streams.u32(root.add(0x20)?)?,
            alternate_weapon_name:weapon_name(zone,root.add(0x1c)?)?,
            suppress168:streams.u8(root.add(168)?)?,suppress169:streams.u8(root.add(169)?)?,
            models:[camo_asset(catalog,zone,root.add(40)?,5)?,camo_asset(catalog,zone,root.add(44)?,5)?,camo_asset(catalog,zone,root.add(48)?,5)?],
        });
    }
    let mut weapons = BTreeMap::new();
    for (name,zone,root) in catalog.definitions(25) {
        let streams = &catalog.zones[zone].1.streams;
        let definition = Address::decode(streams.u32(root.add(8)?)?)?;
        let array = |field: Address,count: usize,kind: u32| -> Result<Vec<Option<String>>> {
            let pointer = streams.u32(field)?;
            if pointer == 0 { return Ok(vec![None;count]); }
            let table = Address::decode(pointer)?;
            (0..count).map(|index| camo_asset(catalog,zone,table.add(index*4)?,kind)).collect()
        };
        let base_attachment_flags = array(root.add(0x18)?,63,29)?.into_iter().map(|name| -> Result<Option<u8>> {
            let Some(name) = name else { return Ok(None); };
            let (owner,root) = catalog.resolve(29,&name)?;
            Ok(Some(catalog.zones[owner].1.streams.u8(root.add(0x31)?)?))
        }).collect::<Result<Vec<_>>>()?;
        weapons.insert(format!("weapon.{}",name.to_ascii_lowercase()),crate::camo::WeaponMaterials {
            camo:camo_asset(catalog,zone,definition.add(2420)?,31)?,
            main_models:array(definition.add(4)?,16,5)?,
            variant_models:array(root.add(40)?,8,5)?.try_into().map_err(|_| "Native viewmodel variant count is not eight")?,
            attachments:array(root.add(28)?,95,30)?,
            definition28:streams.u32(definition.add(28)?)?,definition44:streams.u32(definition.add(44)?)?,
            shared_ammo_related:streams.u8(definition.add(0x64f)?)?,alternate_weapon_index:streams.u32(definition.add(0x67c)?)?,
            alternate_weapon_name:weapon_name(zone,definition.add(0x678)?)?,base_attachment_flags,
        });
    }
    Ok((weapons,attachments))
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

pub fn camo_variants(library: &mut crate::camo::Library) -> Result<BTreeMap<String,Value>> {
    let mut inputs = BTreeMap::new();
    let crate::camo::Library {variants,materials,definitions,options,..} = library;
    let mut add = |bound: Value| {
        if variants.iter().any(|variant| variant.inputs == bound) { return; }
        let key = format!("camo.{}",variants.len());
        variants.push(crate::camo::PreparedVariant {inputs:bound.clone(),surface:None,
            pending:Some("Native camo state has not been converted".into())});
        inputs.insert(key,bound);
    };
    let initial = BTreeMap::from([(181,[0.;4])]);
    for material in materials.values() {
        let active = material["techniques"].as_array().ok_or("Missing camo material techniques")?.iter()
            .filter(|technique| technique["index"] == 4 || technique["index"] == 6)
            .flat_map(|technique| technique["passes"].as_array().into_iter().flatten())
            .flat_map(|pass| pass["arguments"].as_array().into_iter().flatten())
            .any(|argument| argument["type"] == 5 && matches!(argument["value"].as_u64(),Some(16777397 | 16777401)));
        if active { add(crate::camo::camo_inputs(&[],material,1,&initial)?.0); }
    }
    for camo in definitions.values() {
        let mut queue = Vec::new();
        crate::camo::queue_camo(camo,options,45,0.,2,materials,&mut queue)?;
        let originals: std::collections::BTreeSet<_> = queue.iter().filter_map(|command| {
            if let crate::camo::CamoOverride::Material {from,..} = command {Some(from.as_str())} else {None}
        }).collect();
        for original in originals {
            let selected = crate::camo::camo_material(&queue,original,1);
            let material = materials.get(selected).ok_or_else(|| format!("Missing selected camo material {selected}"))?;
            add(crate::camo::camo_inputs(&queue,material,1,&initial)?.0);
        }
    }
    Ok(inputs)
}

pub fn finish_camos(library: &mut crate::camo::Library, converted: &BTreeMap<String,Value>, pack: &mut Pack) -> Result<Vec<String>> {
    let mut pending = Vec::new();
    for (index,variant) in library.variants.iter_mut().enumerate() {
        let key = format!("camo.{index}");
        let surface = converted.get(&key).map(|material| &material["extras"]["surface"]);
        let reason = surface.and_then(|surface| surface["pending_runtime"].as_str()).map(str::to_owned)
            .or_else(|| surface.is_none().then(|| format!("Native camo material {} needs conversion for technique {} with its scoped source inputs",
                variant.inputs["name"],variant.inputs["technique_set"])));
        variant.pending = reason.clone();
        variant.surface = reason.is_none().then(|| surface.unwrap().clone());
        if let Some(reason) = reason { pending.push(format!("{key}: {reason}")); }
    }
    library.validate()?;
    if pack.entries.insert("prepared/camo.json".into(),Arc::new(serde_json::to_vec(library)?)).is_some() {
        return Err("Duplicate native camo library".into());
    }
    Ok(pending)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn attachment_camo_uses_base_alpha_as_mask_and_keeps_gloss_independent() {
        let color = [0.2,0.4,0.6,0.25];
        let specular = [0.1,0.3,0.8,0.7];
        let detail = [0.,0.25,1.,0.75];
        let result = camo_pixel(color,specular,detail,1.,0.5);
        for (actual,expected) in result.into_iter().flatten().zip([0.3125,0.53125,0.75,0.25,0.19375,0.39375,0.875,0.7]) {
            assert!((actual-expected).abs()<1e-6);
        }
        assert_eq!(camo_pixel(color,specular,detail,1.,0.),[color,specular]);
        let protected = [0.2,0.4,0.6,1.];
        for (actual,expected) in camo_pixel(protected,specular,detail,1.,1.).into_iter().flatten().zip([protected,specular].into_iter().flatten()) {
            assert!((actual-expected).abs()<1e-6);
        }
        let unmasked = [0.2,0.4,0.6,0.];
        for (actual,expected) in camo_pixel(unmasked,specular,detail,0.25,1.).into_iter().flatten().zip([0.,0.,0.75,0.,0.35,0.55,1.,0.7]) {
            assert!((actual-expected).abs()<1e-6);
        }
    }

    #[test]
    fn detail_normal_adds_weighted_slopes_before_normalization() {
        let base = texture::Image {width:1,height:1,depth:1,faces:1,format:1,flags:0,levels:vec![vec![255,128,0,255]]};
        let detail = texture::Image {levels:vec![vec![128,255,255,0]],..base.clone()};
        let result = bake(&base,"normal",Some((&detail,20,[8.,2.,0.5,0.]))).unwrap();
        assert_eq!(result.levels[0],[232,180,180,255]);
        assert_eq!(bake(&base,"normal",Some((&detail,20,[8.,2.,0.,0.]))).unwrap().levels,bake(&base,"normal",None).unwrap().levels);
        assert!(bake(&base,"normal",Some((&detail,20,[8.,f32::NAN,1.,0.]))).is_err());
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

    #[test]
    fn unlit_decal_multiply_keeps_transparent_neutral_and_opaque_black() {
        let image = texture::Image {width:2,height:1,depth:1,faces:1,format:1,flags:0,
            levels:vec![vec![32,64,128,0,0,0,0,255]]};
        let result = unlit_color(&image,1.,"decal_multiply").unwrap();
        let pixels = texture::pixels(&result,0).unwrap();
        assert_eq!(pixels[0],[1.,1.,1.,0.]);
        assert_eq!(pixels[1],[0.,0.,0.,1.]);
    }

    #[test]
    fn unlit_additive_decal_weights_green_twice_red_coverage() {
        let image = texture::Image {width:2,height:1,depth:1,faces:1,format:1,flags:0,
            levels:vec![vec![255,0,0,255,0,255,0,255]]};
        let result = unlit_color(&image,1.,"decal_add").unwrap();
        let pixels = texture::pixels(&result,0).unwrap();
        assert!(pixels[0][3] > 0.);
        assert!((pixels[1][3]-2.*pixels[0][3]).abs() <= 1./255.);
        assert_eq!(pixels[0][1],0.);
        assert_eq!(pixels[1][0],0.);
    }

    #[test]
    fn unlit_multiply_ignores_unbound_scale_and_retains_native_value() {
        let native = json!({"technique_set":"mc_unlit_multiply_97923qz1",
            "techniques":[{"index":2,"passes":[{"pixel_shader":"pimp_shader_unlit_4f14dd58.hlsl",
                "vertex_shader":"pimp_shader_unlit_4876d431.hlsl",
                "vertex_routes":[{"source":0,"destination":0},{"source":1,"destination":2},{"source":2,"destination":5}],
                "arguments":[{"type":2,"value":2695565377_u32,"location":0,"size":1,"buffer":0}]}]}],
            "textures":[{"hash":2695565377_u32,"image":"multiply","sampler":20}],
            "constants":[{"hash":3799286735_u32,"literal":[0.,0.,0.,1.]}],
            "state_entries":[255,255,0],"state_bits":[[422086961_u32,28]]});
        let image = texture::Image {width:1,height:1,depth:1,faces:1,format:1,flags:0,levels:vec![vec![128,128,128,255]]};
        assert_eq!(validate_unlit("multiply",&native,"pimp_shader_unlit_4f14dd58.hlsl","multiply").unwrap(),1.);
        let output = unlit_color(&image,0.,"multiply").unwrap();
        assert_eq!(output.levels,unlit_color(&image,50.,"multiply").unwrap().levels);
        assert!(texture::pixels(&output,0).unwrap()[0][0] > 0.);
        let mut pack = Pack::default();
        let converted = convert_unlit("multiply",&native,"pimp_shader_unlit_4f14dd58.hlsl","multiply",
            &BTreeMap::from([("multiply".into(),image)]),&mut BTreeMap::new(),&mut pack).unwrap();
        assert_eq!(converted["extras"]["surface"]["native"]["constants"],native["constants"]);
        assert!(converted["extras"]["surface"]["pending_runtime"].as_str().is_some());
    }

    #[test]
    fn falloff_last_layer_alpha_controls_coverage_without_changing_blended_rgb() {
        let samples = [[0.,0.,0.,0.],[0.,0.,0.,0.],[0.8,0.4,0.2,0.5]];
        let low = falloff_pixel(samples,[1.,1.],Some(0.25),[0.;4],"blend");
        let high = falloff_pixel(samples,[1.,1.],Some(0.75),[0.;4],"blend");
        assert_eq!(low[..3],high[..3]);
        assert!((high[3]*high[3]-3.*low[3]*low[3]).abs()<1e-6);
        let opaque = falloff_pixel(samples,[1.,1.],None,[0.,0.,1.,0.],"opaque");
        assert_eq!(opaque[..3],low[..3]);
        assert_eq!(opaque[3],1.);
        let hidden = falloff_pixel(samples,[1.,1.],Some(1.),[1.;4],"add");
        assert_eq!(hidden,[0.,0.,0.,1.]);
    }

    #[test]
    fn falloff_keeps_authored_mix_overshoot_and_saturates_only_script_inputs() {
        let samples = [[1.,0.,0.,1.],[0.,1.,0.,1.],[0.,0.,1.,1.]];
        let result = falloff_pixel(samples,[1.5,1.],Some(1.),[0.,1.,0.,0.],"add");
        assert!(result[0]>1.);
        assert!(result[2]<0.);
        assert_eq!(result[1],0.);
        assert_eq!(falloff_pixel(samples,[1.5,1.],Some(1.),[-4.,5.,0.5,0.],"add"),
            falloff_pixel(samples,[1.5,1.],Some(1.),[0.,1.,0.5,0.],"add"));
        let suppressed = falloff_pixel(samples,[1.5,1.],Some(1.),[1.,1.,0.,0.],"blend");
        assert_eq!(suppressed,[0.,0.,1.,1.]);
    }

    #[test]
    fn falloff_converted_texture_keeps_squared_hdr_overshoot_and_sqrt_alpha() {
        let source = |pixel| texture::Image {width:1,height:1,depth:1,faces:1,format:1,flags:0,levels:vec![pixel]};
        let red = source(vec![255,0,0,255]);let clear = source(vec![0,255,0,0]);let blue = source(vec![0,0,255,255]);
        let mut layers = [UnlitLayer {scale:[1.;2],offset:[0.;2],origin:[0.5;2],rotation:[0.;2],scroll:[0.;2],depth:0.,alpha:Some(1.)};3];
        layers[0].alpha = Some(1.5);
        let output = falloff_image([&red,&clear,&blue],[20;3],layers,[0.;4],2.,"add").unwrap();
        assert_eq!(texture::pixels(&output,0).unwrap()[0],[9.,0.,1.,1.]);
        layers[0].alpha = Some(0.);layers[2].alpha = Some(0.25);
        let output = falloff_image([&red,&clear,&blue],[20;3],layers,[0.;4],1.,"blend").unwrap();
        assert_eq!(texture::pixels(&output,0).unwrap()[0],[0.,0.,1.,0.5]);
    }

    #[test]
    fn four_layer_emission_keeps_last_alpha_in_coverage_and_unclamped_mix_weights() {
        let layers = [[1.,0.,0.,1.],[0.,1.,0.,1.],[0.,0.,1.,1.],[0.5,0.25,0.75,1.]];
        assert_eq!(four_layer_pixel(layers,[0.,0.,0.,1.],[0.;4]),[0.5,0.25,0.75,1.]);
        assert_eq!(four_layer_pixel(layers,[0.,0.,0.,1.],[0.,0.,0.,1.]),[0.,0.,0.,1.]);
        let overshoot = four_layer_pixel(layers,[2.,0.,0.,1.],[0.;4]);
        assert!(overshoot[0]>1.);
        assert!(overshoot[1]<0. && overshoot[2]<0.);
        assert_eq!(four_layer_pixel(layers,[-2.5,1.,3.,1.],[0.;4]),[0.,0.,0.,1.]);
        assert_eq!(four_layer_pixel(layers,[2.,0.,0.,1.],[-2.,3.,0.,0.]),
            four_layer_pixel(layers,[2.,0.,0.,1.],[0.,1.,0.,0.]));
    }

    #[test]
    fn viewmodel_alpha_keeps_raw_coverage_when_emission_is_zero() {
        let image = texture::Image {width:1,height:1,depth:1,faces:1,format:1,flags:0,levels:vec![vec![255,128,64,64]]};
        let alpha = unlit_color(&image,0.,"source_vm_alpha").unwrap();
        let pixel = texture::pixels(&alpha,0).unwrap()[0];
        assert_eq!(pixel[..3],[0.;3]);
        assert!((pixel[3]-64./255.).abs()<1e-6);
        let sqrt = unlit_color(&image,0.,"source_vm_blend").unwrap();
        assert!(texture::pixels(&sqrt,0).unwrap()[0][3]>pixel[3]);
    }
}

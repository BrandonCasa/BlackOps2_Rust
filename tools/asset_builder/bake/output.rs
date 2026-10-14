use super::*;

#[derive(Clone, Copy, Default)]
pub(super) struct Output {
    pub(super) base_alpha_response: bool,
    pub(super) base_alpha_reveal: bool,
    pub(super) layer_specular: bool,
    pub(super) layer_cutout: bool,
    pub(super) multiply_gloss: bool,
}

pub(super) fn mip_chain(
    width: usize,
    height: usize,
    pixels: Vec<[f32; 4]>,
    role: &str,
) -> Result<texture::Image> {
    if width == 0 || height == 0 || width.checked_mul(height) != Some(pixels.len()) {
        return Err("Baked material dimensions do not match pixel count".into());
    }
    if pixels
        .iter()
        .flatten()
        .any(|value| !value.is_finite() || role != "normal" && *value < 0.)
    {
        return Err("Baked material has a non-finite or negative channel".into());
    }
    let hdr = matches!(role, "color" | "native-specular")
        && pixels
            .iter()
            .any(|pixel| pixel.iter().any(|&value| value > 1.));
    let mut image = texture::Image {
        width,
        height,
        depth: 1,
        faces: 1,
        format: if hdr { 19 } else { 1 },
        flags: 0,
        levels: Vec::new(),
    };
    let (mut w, mut h, mut pixels) = (width, height, pixels);
    loop {
        let stride = if hdr { 8 } else { 4 };
        let mut bytes = vec![0; w * h * stride];
        parallel_chunks_mut(&mut bytes, 4096 * stride, |offset, bytes| {
            for (bytes, pixel) in bytes
                .chunks_exact_mut(stride)
                .zip(&pixels[offset / stride..])
            {
                let mut value = *pixel;
                if role == "normal" {
                    let length = value[..3].iter().map(|v| v * v).sum::<f32>().sqrt();
                    if length < 1e-8 {
                        value = [0., 0., 1., 1.];
                    } else {
                        for channel in &mut value[..3] {
                            *channel /= length;
                        }
                    }
                    for channel in &mut value[..3] {
                        *channel = *channel * 0.5 + 0.5;
                    }
                } else if role == "color" && !hdr {
                    for channel in &mut value[..3] {
                        *channel = if *channel <= 0.0031308 {
                            *channel * 12.92
                        } else {
                            1.055 * channel.powf(1. / 2.4) - 0.055
                        };
                    }
                }
                if hdr {
                    if value
                        .iter()
                        .any(|v| !v.is_finite() || !(0. ..=65504.).contains(v))
                        || role == "color" && value[3] > 1.
                    {
                        return Err("Baked material pixel exceeds half-float color range".into());
                    }
                    for (bytes, component) in bytes.chunks_exact_mut(2).zip(value) {
                        bytes.copy_from_slice(&half::f16::from_f32(component).to_le_bytes());
                    }
                } else {
                    if value
                        .iter()
                        .any(|v| !v.is_finite() || !(0. ..=1.).contains(v))
                    {
                        return Err("Baked material pixel exceeds UNORM range".into());
                    }
                    bytes.copy_from_slice(&value.map(|v| (v * 255.).round() as u8));
                }
            }
            Ok(())
        })?;
        image.levels.push(bytes);
        if w == 1 && h == 1 {
            break;
        }
        let (nw, nh) = ((w / 2).max(1), (h / 2).max(1));
        let mut next = vec![[0.; 4]; nw * nh];
        parallel_chunks_mut(&mut next, 4096, |offset, next| {
            for (index, pixel) in next.iter_mut().enumerate() {
                let (x, y) = ((offset + index) % nw, (offset + index) / nw);
                let values = [
                    pixels[(y * 2).min(h - 1) * w + (x * 2).min(w - 1)],
                    pixels[(y * 2).min(h - 1) * w + (x * 2 + 1).min(w - 1)],
                    pixels[(y * 2 + 1).min(h - 1) * w + (x * 2).min(w - 1)],
                    pixels[(y * 2 + 1).min(h - 1) * w + (x * 2 + 1).min(w - 1)],
                ];
                *pixel = std::array::from_fn(|c| values.iter().map(|v| v[c]).sum::<f32>() * 0.25);
            }
            Ok(())
        })?;
        (w, h, pixels) = (nw, nh, next);
    }
    Ok(image)
}

pub(super) fn store(
    native: &Value,
    uri: &str,
    width: usize,
    height: usize,
    outputs: [Vec<[f32; 4]>; 4],
    sampler: u8,
    charts: usize,
    pack: &mut Pack,
) -> Result<Value> {
    let family = native["technique_set"]
        .as_str()
        .and_then(family)
        .ok_or("Unreviewed baked material output")?;
    let Output {
        base_alpha_response,
        base_alpha_reveal,
        layer_specular,
        layer_cutout,
        multiply_gloss,
    } = family.output();
    let [albedo, normals, response, specular] = outputs;
    let mut channels = Vec::new();
    let mut outputs = vec![("color", albedo), ("normal", normals)];
    if layer_specular || base_alpha_response {
        outputs.push(("response", response));
    }
    if layer_specular {
        outputs.push(("native-specular", specular));
    }
    let mut native_specular = Value::Null;
    for (role, pixels) in outputs {
        let image = mip_chain(width, height, pixels, role)?;
        let path = format!("{uri}/{role}.ktx2");
        let srgb = role == "color" && image.format != 19;
        let bytes = texture::encode(
            &image,
            srgb,
            if role == "native-specular" {
                "reflectance"
            } else {
                role
            },
        )?;
        texture::validate(&bytes)?;
        pack.entries.insert(path.clone(), Arc::new(bytes));
        if role == "native-specular" {
            native_specular = json!({"uri":path,"specular_rgb":"linear","gloss_channel":"alpha"});
        } else {
            if role == "response" && base_alpha_response {
                native_specular = json!({"uri":path,"gloss_channel":"alpha","specular_rgb":"constant_0.04",
                    "source":"Direct-lobe Fresnel floor; native environment Fresnel fit differs"});
            }
            channels.push(json!({"uri":path,"role":role,"layer":0,"color_space":if srgb {"srgb"} else {"linear"},"sampler":sampler}));
        }
    }
    let mut material = json!({"name":native["name"],"pbrMetallicRoughness":{"baseColorFactor":[1.,1.,1.,1.],"metallicFactor":0.},
        "extras":{"surface":{"version":3,"identity":native["name"],"native":native,"channels":channels,
            "standard":{"roughness":1.,"metallic":if layer_specular {1.} else {0.},"reflectance":if layer_specular || base_alpha_response {0.5} else {0.},"base_color":[1.,1.,1.,1.]},
            "native_specular":native_specular,
            "presentation":{"alpha":"opaque","cull":"back","fog":true,"visible":true},
            "bake":{"width":width,"height":height,"charts":charts,"source_texel_density":true,"layout":if sampler == 20 {"periodic"} else {"atlas"}},
            "approximations":["Static material bake at source UV texel density; finite rasterization, BC compression and baked mip filtering", "Source normal slopes normalized before runtime texture filtering", "Source specular/gloss fitted to shared metallic/GGX PBR inputs where present; response mips average the fitted inputs"]}}});
    if layer_cutout {
        material["extras"]["surface"]["source_framebuffer_alpha"] =
            json!("selected_layer_alpha_or_one");
        material["extras"]["surface"]["pending_runtime"] = json!(
            "Native opaque layer selection writes selected layer alpha to the framebuffer; current StandardMaterial opaque output forces alpha to one"
        );
    }
    if sampler != 20 {
        material["extras"]["surface"]["approximations"]
            .as_array_mut()
            .unwrap()
            .push(json!(
                "Coarse atlas mip levels can mix unrelated charts beyond the eight-pixel gutter"
            ));
    }
    if base_alpha_response {
        material["extras"]["surface"]["approximations"].as_array_mut().unwrap().push(json!("Source effective gloss retained in response alpha and fitted from native exponent exp2(13*a) to GGX perceptual roughness (2/(exp2(13*a)+2))^(1/4). Dielectric F0=0.04 uses the native sun-lobe floor; this cannot reproduce the native environment Fresnel/mip fit, direct-lobe geometry/amplitude or inverse-SH compensation exactly. View-dependent lighting is not baked into albedo or emission"));
    }
    if multiply_gloss {
        material["extras"]["surface"]["approximations"].as_array_mut().unwrap().push(json!("Source effective gloss is base color alpha multiplied by the encoded multiply tint luminance dot(tint, [0.3,0.6,0.1]); this effective gloss is retained in response alpha before the shared GGX fit"));
    }
    if base_alpha_reveal {
        material["extras"]["surface"]["approximations"].as_array_mut().unwrap().push(json!("Source gloss mixes base color alpha and layer specular alpha with the authored reveal weight. Mixed linear RGB specular and encoded gloss retained in native-specular. The shared metallic/GGX fit cannot exactly reproduce the native probe Fresnel/mip response or direct-sun lobe geometry/amplitude; view-dependent lighting is not baked into albedo or emission"));
    }
    Ok(material)
}

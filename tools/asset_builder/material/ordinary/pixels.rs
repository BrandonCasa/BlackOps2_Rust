use super::super::*;

pub(in crate::material) fn bake(
    source: &texture::Image,
    role: &str,
    detail: Option<(&texture::Image, u8, [f32; 4])>,
) -> Result<texture::Image> {
    if source.depth != 1 || source.faces != 1 {
        return Err(format!("Ordinary lit {role} channel is not a 2D image").into());
    }
    let detail_pixels = if let Some((image, _, parameters)) = detail {
        if role != "normal"
            || image.depth != 1
            || image.faces != 1
            || image.levels.is_empty()
            || !parameters.iter().all(|value| value.is_finite())
        {
            return Err("Invalid source detail normal input".into());
        }
        (0..image.levels.len())
            .map(|level| texture::pixels(image, level))
            .collect::<Result<Vec<_>>>()?
    } else {
        Vec::new()
    };
    let mut result = source.clone();
    result.format = 1;
    result.levels.clear();
    for level in 0..source.levels.len() {
        let mut bytes = Vec::new();
        for (index, pixel) in texture::pixels(source, level)?.into_iter().enumerate() {
            let mut value = pixel;
            match role {
                "color" => {
                    for channel in &mut value[..3] {
                        let linear = *channel * *channel;
                        *channel = if linear <= 0.0031308 {
                            linear * 12.92
                        } else {
                            1.055 * linear.powf(1. / 2.4) - 0.055
                        };
                    }
                }
                "normal" => {
                    let mut slope = [pixel[0], pixel[1]].map(|value| {
                        value * f32::from_bits(0x40808102) + f32::from_bits(0xc0010204)
                    });
                    if let Some((image, sampler, parameters)) = detail {
                        let (width, height) = (
                            (source.width >> level).max(1),
                            (source.height >> level).max(1),
                        );
                        let uv = [(index % width) as f32 + 0.5, (index / width) as f32 + 0.5];
                        let uv = [
                            uv[0] / width as f32 * parameters[0],
                            uv[1] / height as f32 * parameters[1],
                        ];
                        let frequency = (image.width as f32 * parameters[0].abs())
                            .max(image.height as f32 * parameters[1].abs())
                            .max(1.);
                        let lod = (level as f32 + frequency.log2()
                            - (source.width.max(source.height) as f32).log2())
                        .floor()
                        .clamp(0., (image.levels.len() - 1) as f32)
                            as usize;
                        let sample = texture::sample(
                            &detail_pixels[lod],
                            (image.width >> lod).max(1),
                            (image.height >> lod).max(1),
                            uv,
                            sampler,
                        );
                        for axis in 0..2 {
                            slope[axis] += parameters[2]
                                * (sample[axis] * f32::from_bits(0x40808102)
                                    + f32::from_bits(0xc0010204));
                        }
                    }
                    let [x, y] = slope;
                    let length = (x * x + y * y + 1.).sqrt();
                    value = [
                        x / length * 0.5 + 0.5,
                        y / length * 0.5 + 0.5,
                        1. / length * 0.5 + 0.5,
                        1.,
                    ];
                }
                "response" => {
                    let roughness = (2. / (2_f32.powf(13. * pixel[3]) + 2.)).powf(0.25);
                    value = [1., roughness, 0., 1.];
                }
                _ => return Err(format!("Unknown material bake channel {role}").into()),
            }
            if !value.iter().all(|v| v.is_finite()) {
                return Err("Non-finite converted material texel".into());
            }
            if value.iter().any(|v| !(0. ..=1.).contains(v)) {
                return Err("Converted material channel exceeds UNORM range".into());
            }
            bytes.extend(value.map(|v| (v * 255.).round() as u8));
        }
        result.levels.push(bytes);
    }
    Ok(result)
}

pub fn pbr_pixel(diffuse: [f32; 3], specular: [f32; 3], gloss: f32) -> ([f32; 3], f32, f32) {
    let brightness = |rgb: [f32; 3]| {
        (rgb[0] * rgb[0] * 0.299 + rgb[1] * rgb[1] * 0.587 + rgb[2] * rgb[2] * 0.114).sqrt()
    };
    let energy = 1. - specular.into_iter().fold(0., f32::max);
    let specular_brightness = brightness(specular);
    let metallic = if specular_brightness < 0.04 {
        0.
    } else {
        let b = brightness(diffuse) * energy / 0.96 + specular_brightness - 0.08;
        ((-b + (b * b - 0.16 * (0.04 - specular_brightness)).max(0.).sqrt()) / 0.08).clamp(0., 1.)
    };
    let base = std::array::from_fn(|c| {
        let from_diffuse = diffuse[c] * energy / 0.96 / (1. - metallic).max(1e-6);
        let from_specular = (specular[c] - 0.04 * (1. - metallic)) / metallic.max(1e-6);
        (from_diffuse + (from_specular - from_diffuse) * metallic * metallic).clamp(0., 1.)
    });
    (
        base,
        metallic,
        (2. / (2_f32.powf(13. * gloss) + 2.)).powf(0.25),
    )
}

pub(in crate::material) fn camo_pixel(
    mut color: [f32; 4],
    mut specular: [f32; 4],
    detail: [f32; 4],
    color_bias: f32,
    weight: f32,
) -> [[f32; 4]; 2] {
    for component in 0..3 {
        let replacement_color = (detail[component] + color_bias - 0.5).clamp(0., 1.);
        let replacement_specular = (specular[component] + detail[3] - 0.5).clamp(0., 1.);
        let replacement_color =
            replacement_color + color[3] * (color[component] - replacement_color);
        let replacement_specular =
            replacement_specular + color[3] * (specular[component] - replacement_specular);
        color[component] += weight * (replacement_color - color[component]);
        specular[component] += weight * (replacement_specular - specular[component]);
    }
    [color, specular]
}

pub(in crate::material) fn metallic_workflow(
    color: &texture::Image,
    specular: &texture::Image,
    samplers: [u8; 2],
    camo: Option<(&texture::Image, u8, [f32; 4])>,
    occlusion: Option<(&texture::Image, u8)>,
) -> Result<(texture::Image, texture::Image)> {
    let mut sources = vec![color, specular];
    if let Some((image, _, parameters)) = camo {
        if !parameters.iter().all(|value| value.is_finite()) {
            return Err("Non-finite source camo parameters".into());
        }
        sources.push(image);
    }
    if let Some((image, _)) = occlusion {
        sources.push(image);
    }
    if sources
        .iter()
        .any(|image| image.faces != 1 || image.depth != 1 || image.levels.is_empty())
    {
        return Err("Specular-workflow input is not a 2D image".into());
    }
    let (width, height) = (
        sources.iter().map(|image| image.width).max().unwrap(),
        sources.iter().map(|image| image.height).max().unwrap(),
    );
    let mut base = texture::Image {
        width,
        height,
        depth: 1,
        faces: 1,
        format: 1,
        flags: 0,
        levels: Vec::new(),
    };
    let mut response = base.clone();
    let decoded = sources
        .iter()
        .map(|image| {
            (0..image.levels.len())
                .map(|level| texture::pixels(image, level))
                .collect::<Result<Vec<_>>>()
        })
        .collect::<Result<Vec<_>>>()?;
    for level in 0..sources
        .iter()
        .map(|image| image.levels.len())
        .max()
        .unwrap()
    {
        let (w, h) = ((width >> level).max(1), (height >> level).max(1));
        let mut base_bytes = Vec::with_capacity(w * h * 4);
        let mut response_bytes = Vec::with_capacity(w * h * 4);
        for y in 0..h {
            for x in 0..w {
                let uv = [(x as f32 + 0.5) / w as f32, (y as f32 + 0.5) / h as f32];
                let sample = |index: usize, uv, sampler| {
                    let image = sources[index];
                    let bias = width.max(height).ilog2() - image.width.max(image.height).ilog2();
                    let source_level = level
                        .saturating_sub(bias as usize)
                        .min(image.levels.len() - 1);
                    texture::sample(
                        &decoded[index][source_level],
                        (image.width >> source_level).max(1),
                        (image.height >> source_level).max(1),
                        uv,
                        sampler,
                    )
                };
                let mut samples = [sample(0, uv, samplers[0]), sample(1, uv, samplers[1])];
                if let Some((_, sampler, parameters)) = camo {
                    let detail_uv = std::array::from_fn(|axis| {
                        (uv[axis] - 0.5) * parameters[2] + parameters[axis] + 0.5
                    });
                    let detail = sample(2, detail_uv, sampler);
                    let color_bias = if let Some((_, sampler)) = occlusion {
                        sample(3, uv, sampler)[2]
                    } else {
                        1.
                    };
                    samples = camo_pixel(samples[0], samples[1], detail, color_bias, parameters[3]);
                }
                let (rgb, metallic, roughness) = pbr_pixel(
                    std::array::from_fn(|c| samples[0][c] * samples[0][c]),
                    std::array::from_fn(|c| samples[1][c] * samples[1][c]),
                    samples[1][3],
                );
                for value in rgb {
                    let srgb = if value <= 0.0031308 {
                        value * 12.92
                    } else {
                        1.055 * value.powf(1. / 2.4) - 0.055
                    };
                    base_bytes.push((srgb * 255.).round() as u8);
                }
                base_bytes.push((samples[0][3] * 255.).round() as u8);
                response_bytes.extend([
                    255,
                    (roughness * 255.).round() as u8,
                    (metallic * 255.).round() as u8,
                    255,
                ]);
            }
        }
        base.levels.push(base_bytes);
        response.levels.push(response_bytes);
    }
    Ok((base, response))
}

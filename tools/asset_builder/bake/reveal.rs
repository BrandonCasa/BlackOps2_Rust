use super::*;

pub(super) fn lookup(family: &str) -> Option<bool> {
    match family {
        "mc_lit_sm_b0c0n0v0_3559e8jj" | "wpc_lit_sm_b0c0n0v0_ew347fe3" => Some(false),
        "wpc_lit_sm_b0c0n0s0v0_efz5w56f" => Some(true),
        _ => None,
    }
}

pub(super) fn read(
    native: &Value,
    images: Option<&BTreeMap<String, texture::Image>>,
) -> Result<Recipe> {
    let specular = crate::material::validate_static_alpha_reveal(native)?;
    let parms = crate::material::contract::literal(native, 2932833280)?;
    let mut inputs = Vec::new();
    for hash in [2695565377, 1507003663]
        .into_iter()
        .chain(specular.then_some(887934131))
    {
        let binding = crate::material::contract::texture_binding(native, hash)?;
        let name = binding["image"]
            .as_str()
            .ok_or("Alpha reveal source image name is missing")?;
        let sampler = binding["sampler"]
            .as_u64()
            .and_then(|value| u8::try_from(value).ok())
            .ok_or("Alpha reveal native sampler is invalid")?;
        if let Some(images) = images {
            let image = images
                .get(&name.to_ascii_lowercase())
                .ok_or_else(|| format!("Alpha reveal source image is missing: {name}"))?;
            inputs.push(Input::read(image, sampler)?);
        }
    }
    Ok(Recipe {
        alpha_reveal: true,
        special: 0,
        special_parms: [[0.; 4]; 3],
        inputs,
        base_specular: specular,
        geometric_normal: false,
        second_additive: false,
        second_normal: 0,
        second_reveal: false,
        second_specular: false,
        second_parms: [0.; 4],
        third_multiply: false,
        layer_cutout: false,
        decal: false,
        multiply: false,
        additive: false,
        blend_color: false,
        reveal: false,
        layer_specular: false,
        base_alpha_response: false,
        parms,
    })
}

pub(super) fn periodic_key(surface: &Surface, first: usize) -> Option<Vec<u32>> {
    if !constant_frame(surface, first) {
        return None;
    }
    let alpha = constant_alpha(surface.colors.get(first..first + 3)?)?;
    Some(vec![
        alpha as u32,
        1_f32.to_bits(),
        0.5_f32.to_bits(),
        0.5_f32.to_bits(),
        1_f32.to_bits(),
        0_f32.to_bits(),
        0_f32.to_bits(),
    ])
}

fn constant_alpha(colors: &[[u8; 4]]) -> Option<u8> {
    (colors.len() == 3 && colors.iter().all(|color| color[3] == colors[0][3])).then(|| colors[0][3])
}

pub(super) fn finish(source: Option<&Value>, material: &mut Value) -> Result<()> {
    let source = source.ok_or("Alpha reveal original material conversion is missing")?;
    let reveal = &source["source_reveal"];
    if reveal["vertex_channel"] != "alpha"
        || reveal["input"]["uri"].as_str().is_none()
        || reveal["input"]["sampler"].as_u64().is_none()
        || reveal["parameters"]
            .as_array()
            .is_none_or(|values| values.len() != 4)
    {
        return Err("Alpha reveal original input binding or parameters are missing".into());
    }
    let surface = &mut material["extras"]["surface"];
    surface["source_reveal"] = reveal.clone();
    surface["source_reveal"]["baked"] = json!(true);
    surface["source_reveal"]["vertex_alpha_consumed"] = json!(true);
    surface["source_coverage"] = json!("baked_reveal_coverage");
    surface["source_fragment_alpha"] = source["source_fragment_alpha"].clone();
    surface["source_vertex_color_transfer"] = source["source_vertex_color_transfer"].clone();
    surface["presentation"] = source["presentation"].clone();
    surface["presentation"]["alpha"] = json!("blend");
    surface["pending_runtime"] = json!(
        "Source reveal coverage is baked from actual interpolated mesh alpha and native texture alpha; native sqrt fragment alpha and gamma2 premultiplied fog composition are approximated with straight linear PBR blending"
    );
    surface["approximations"].as_array_mut().ok_or("Alpha reveal approximation list is missing")?
        .push(json!("Authored reveal thresholds evaluated before coverage using native bilinear sampling and interpolated source vertex alpha; finite atlas rasterization and output mip filtering remain approximations. Vertex RGB retains the existing gamma2-before-interpolation PBR approximation"));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn periodic_reveal_requires_actual_constant_triangle_alpha() {
        assert_eq!(
            constant_alpha(&[[0, 255, 0, 128], [255, 0, 0, 128], [0, 0, 255, 128]]),
            Some(128)
        );
        assert_eq!(
            constant_alpha(&[[0, 255, 0, 128], [0, 255, 0, 127], [0, 255, 0, 128]]),
            None
        );
        assert_eq!(constant_alpha(&[]), None);
    }

    #[test]
    fn reveal_bake_keeps_raw_binding_and_only_declares_composition_approximation() {
        let source = json!({"source_reveal":{"input":{"uri":"prepared/source.ktx2","sampler":20,"transfer":"encoded_gamma2"},
            "parameters":[0.25,0.5,0.,0.],"vertex_channel":"alpha","vertex_scale":f32::from_bits(0x3f7f7cee),
            "vertex_bias":f32::from_bits(0x3a83126f)},"source_fragment_alpha":"sqrt_reveal_coverage",
            "source_vertex_color_transfer":"gamma2","presentation":{"alpha":"premultiplied","cull":"none","fog":true,"visible":true}});
        let mut material = json!({"extras":{"surface":{"approximations":[]}}});
        finish(Some(&source), &mut material).unwrap();
        let surface = &material["extras"]["surface"];
        assert_eq!(
            surface["source_reveal"]["input"],
            source["source_reveal"]["input"]
        );
        assert_eq!(
            surface["source_reveal"]["parameters"],
            source["source_reveal"]["parameters"]
        );
        assert_eq!(surface["source_reveal"]["baked"], true);
        assert_eq!(surface["source_reveal"]["vertex_alpha_consumed"], true);
        assert_eq!(surface["source_coverage"], "baked_reveal_coverage");
        assert_eq!(surface["presentation"]["alpha"], "blend");
        assert_eq!(surface["presentation"]["cull"], "none");
        assert_eq!(surface["source_vertex_color_transfer"], "gamma2");
        assert!(finish(None, &mut material).is_err());
    }
}

use super::*;

#[derive(Clone, Copy)]
pub(super) struct SmokeRecipe;

const TEXTURES: [(u32, u32, u8, &str, usize); 6] = [
    (170923560, 772, 2, "alpha_plus", 1),
    (1345510170, 1027, 66, "alpha", 0),
    (2515352718, 1282, 226, "alpha_mask", 3),
    (2564398276, 517, 2, "alpha_minus", 2),
    (2695565377, 1537, 2, "color_plus", 1),
    (2816785293, 1792, 2, "color_minus", 2),
];

const CONSTANTS: [(u32, u32, u32); 11] = [
    (14836402, 88, 4),
    (116600957, 128, 16),
    (504449656, 160, 8),
    (1426306028, 80, 8),
    (1426769011, 116, 8),
    (1621481545, 144, 8),
    (1638779318, 152, 8),
    (2010068229, 104, 8),
    (2010882330, 96, 8),
    (3371644427, 112, 4),
    (3599644333, 168, 8),
];

pub(super) fn lookup(family: &str) -> Option<SmokeRecipe> {
    (family == "mc_sw4_3d_zmb_smoke_pillar_cheap_w6w7e738").then_some(SmokeRecipe)
}

#[cfg(test)]
fn scroll_uv(uv: [f32; 2], scale: [f32; 2], speed: [f32; 2], time: f32) -> [f32; 2] {
    std::array::from_fn(|axis| {
        let phase = time * speed[axis] * scale[axis];
        uv[axis] * scale[axis] + phase - phase.floor()
    })
}

#[cfg(test)]
fn mask_uv(uv: [f32; 2], scale: [f32; 2], offset: [f32; 2]) -> [f32; 2] {
    std::array::from_fn(|axis| (uv[axis] - offset[axis]) * scale[axis] + offset[axis])
}

#[cfg(test)]
fn smoke_pixel(
    samples: [[f32; 4]; 6],
    color: [f32; 4],
    hdr: f32,
    vertex: [f32; 4],
    depth: f32,
    near: f32,
    clip_w: f32,
    fade: f32,
) -> [f32; 4] {
    let soft = ((near / depth.abs().max(0.) - clip_w) / fade.max(f32::from_bits(0x38d1b717)))
        .clamp(0., 1.);
    let density = samples[1][0] * samples[2][0] * (samples[0][0] + samples[3][0]);
    let weight = density * vertex[3] * vertex[3] * soft * color[3];
    let mut output = [0., 0., 0., 1.];
    for component in 0..3 {
        output[component] = ((samples[4][component] + samples[5][component])
            * vertex[component]
            * color[component]
            * weight
            * hdr)
            .powi(2);
    }
    output
}

impl SmokeRecipe {
    pub(super) fn validate(self, key: &str, native: &Value) -> Result<()> {
        if lookup(native["technique_set"].as_str().unwrap_or("")).is_none() {
            return Err(format!("Smoke pillar {key} has an unreviewed family").into());
        }
        let mut arguments = vec![
            (3, 67109077_u32, 3, 0, 64),
            (3, 67109093, 0, 576, 64),
            (4, 18, 0, 262, 1),
            (5, 16777233, 0, 1520, 16),
            (5, 16777241, 2, 64, 16),
        ];
        arguments.extend(
            TEXTURES
                .iter()
                .map(|&(hash, location, _, _, _)| (2, hash, 0, location, 1)),
        );
        arguments.extend(
            CONSTANTS
                .iter()
                .map(|&(hash, location, size)| (6, hash, 2, location, size)),
        );
        let selected = 3;
        let pass = contract::pass(native, selected)?;
        if pass["vertex_shader"] != "pimp_shader_sw4_3d_zmb_smoke_pillar_cheap_cd848ad4.hlsl"
            || pass["pixel_shader"] != "pimp_shader_sw4_3d_zmb_smoke_pillar_cheap_f53884f8.hlsl"
            || pass["vertex_routes"]
                != json!([
                    {"source":0,"destination":0},
                    {"source":1,"destination":2},
                    {"source":2,"destination":5}
                ])
        {
            return Err(
                format!("Smoke pillar {key} has unreviewed programs or vertex routes").into(),
            );
        }
        let state_index = native["state_entries"][selected]
            .as_u64()
            .ok_or("Missing smoke pillar state index")? as usize;
        if native["state_bits"][state_index] != json!([421628194_u32, 12]) {
            return Err(format!("Smoke pillar {key} has an unreviewed render state").into());
        }
        let active = pass["arguments"]
            .as_array()
            .ok_or("Missing smoke pillar arguments")?;
        if active.len() != arguments.len()
            || arguments
                .iter()
                .any(|&(kind, value, buffer, location, size)| {
                    active
                        .iter()
                        .filter(|argument| {
                            contract::argument(
                                argument,
                                [kind, value as u64, buffer, location as u64, size],
                            )
                        })
                        .count()
                        != 1
                })
        {
            return Err(format!("Smoke pillar {key} has unreviewed active arguments").into());
        }
        for &(hash, _, _) in &CONSTANTS {
            contract::literal(native, hash)?;
        }
        for &(hash, _, sampler, _, _) in &TEXTURES {
            if contract::texture_binding(native, hash)?["sampler"] != sampler {
                return Err(
                    format!("Smoke pillar {key} has an unreviewed sampler for {hash}").into(),
                );
            }
        }
        Ok(())
    }

    pub(super) fn convert(
        self,
        key: &str,
        native: &Value,
        images: &BTreeMap<String, texture::Image>,
        shared: &mut BTreeMap<(String, String), String>,
        pack: &mut Pack,
    ) -> Result<Value> {
        self.validate(key, native)?;
        let binding = contract::texture_binding(native, 2695565377)?;
        let name = binding["image"].as_str().ok_or("Missing smoke color image")?.to_ascii_lowercase();
        let source = images.get(&name).ok_or_else(|| format!("Missing smoke color image {name}"))?;
        let identity = (name, "smoke-standard".into());
        let uri = if let Some(uri) = shared.get(&identity) {
            uri.clone()
        } else {
            let image = ordinary::bake(source, "color", None)?;
            let uri = store_image(&image, format!("prepared/material-channels/{}/smoke.ktx2", shared.len()), true, "color", pack)?;
            shared.insert(identity, uri.clone());
            uri
        };
        let color = contract::literal(native, 116600957)?;
        let hdr = contract::literal(native, 14836402)?[0];
        let scale = contract::literal(native, 2010068229)?;
        let speed = contract::literal(native, 2010882330)?;
        let tint = [color[0], color[1], color[2]].map(|value| (value * color[3] * hdr).powi(2));
        Ok(json!({"name":key,"pbrMetallicRoughness":{"metallicFactor":0.},
            "extras":{"surface":{"version":3,"identity":key,"native":native,
                "channels":[{"uri":uri,"role":"color","layer":0,"color_space":"srgb","sampler":binding["sampler"]}],
                "standard":{"unlit":true,"roughness":1.,"metallic":0.,"reflectance":0.,"base_color":[tint[0],tint[1],tint[2],1.],"uv_transform":[scale[0],0.,0.,scale[1],0.,0.]},
                "presentation":{"alpha":"add","cull":"back","fog":false,"visible":true},
                "uv_animation":{"translation":[speed[0],speed[1]],"time_mode":"elapsed"},
                "source_smoke":{"consumer":"standard_material"},
                "approximations":["Source color-plus texture, tint, HDR scale and UV scroll use stock unlit additive material; independent six-layer density, squared vertex alpha and scene-depth softness are omitted"]}}}))
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn independent_scroll_domains_wrap_negative_native_phase() {
        assert_eq!(
            scroll_uv([0.25, 0.5], [2., 0.5], [-0.25, 1.], 3.),
            [1., 0.75]
        );
        assert_eq!(mask_uv([0.25, 0.75], [1., 1.25], [0., 0.5]), [0.25, 0.8125]);
        assert_ne!(
            scroll_uv([0.25; 2], [1.; 2], [0.5; 2], 1.),
            scroll_uv([0.25; 2], [2.; 2], [0.5; 2], 1.)
        );
    }

    #[test]
    fn smoke_adds_minus_maps_and_squares_vertex_alpha_before_emission() {
        let samples = [[0.5; 4], [0.5; 4], [0.25; 4], [0.25; 4], [1.; 4], [2.; 4]];
        let full = smoke_pixel(samples, [1., 1., 1., 0.25], 2., [1.; 4], 0.5, 1., 0., 1.);
        let value = 0.140625_f32.powi(2);
        assert_eq!(full, [value, value, value, 1.]);
    }

    #[test]
    fn smoke_depth_fade_uses_scene_distance_and_clip_w() {
        let samples = [[1.; 4]; 6];
        let full = smoke_pixel(samples, [1.; 4], 1., [1.; 4], 0.5, 1., 0., 1.);
        let half_depth = smoke_pixel(samples, [1.; 4], 1., [1.; 4], 0.5, 1., 1.5, 1.);
        let half_alpha = smoke_pixel(samples, [1.; 4], 1., [1., 1., 1., 0.5], 0.5, 1., 0., 1.);
        assert_eq!(full, [16., 16., 16., 1.]);
        assert_eq!(half_depth, [4., 4., 4., 1.]);
        assert_eq!(half_alpha, [1.; 4]);
        assert_eq!(
            smoke_pixel(samples, [1.; 4], 1., [1.; 4], 0., 1., 0., 1.),
            full
        );
        assert_eq!(
            smoke_pixel(samples, [1.; 4], 1., [1.; 4], 0.5, 1., 3., 1.),
            [0., 0., 0., 1.]
        );
    }
}

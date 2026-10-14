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
    pub(super) fn index(self) -> usize {
        3
    }

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
        let selected = self.index();
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
                            argument["type"] == kind
                                && argument["value"] == value
                                && argument["buffer"] == buffer
                                && argument["location"] == location
                                && argument["size"] == size
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
        let vec2 = |hash| -> Result<[f32; 2]> {
            let value = contract::literal(native, hash)?;
            Ok([value[0], value[1]])
        };
        let scales = [
            vec2(1426306028)?,
            vec2(2010068229)?,
            vec2(1621481545)?,
            vec2(504449656)?,
        ];
        let speeds = [vec2(1426769011)?, vec2(2010882330)?, vec2(1638779318)?];
        let mask_offset = vec2(3599644333)?;
        let color = contract::literal(native, 116600957)?;
        let hdr = contract::literal(native, 14836402)?[0];
        let mut layers = Vec::new();
        for &(hash, _, sampler, role, domain) in &TEXTURES {
            let binding = contract::texture_binding(native, hash)?;
            let name = binding["image"]
                .as_str()
                .ok_or("Missing smoke pillar image name")?
                .to_ascii_lowercase();
            let source = images
                .get(&name)
                .ok_or_else(|| format!("Missing smoke pillar image {name}"))?;
            let source_key = (name.clone(), "source_color".into());
            let uri = if let Some(uri) = shared.get(&source_key) {
                uri.clone()
            } else {
                let uri = store_image(
                    source,
                    format!(
                        "prepared/material-channels/{}/source-color.ktx2",
                        shared.len()
                    ),
                    false,
                    "source_color",
                    pack,
                )?;
                shared.insert(source_key, uri.clone());
                uri
            };
            layers.push(json!({"uri":uri,"hash":hash,"role":role,"sampler":sampler,"domain":domain,"color_space":"native_encoded"}));
        }
        let surface = json!({"version":3,"identity":key,"native":native,
            "channels":[],
            "source_layers":layers,
            "standard":{"unlit":true,"roughness":1.,"metallic":0.,"reflectance":0.,"base_color":[1.,1.,1.,1.]},
            "presentation":{"alpha":"add","cull":"back","fog":false,"visible":true},
            "source_smoke":{"consumer":"native_layers_vertex_time_depth_forward","color":color,"hdr_amount":hdr,"density":"alpha_red_times_mask_red_times_sum_alpha_plus_red_and_alpha_minus_red",
                "rgb":"square_sum_color_plus_rgb_and_color_minus_rgb_times_vertex_rgb_times_color_rgb_times_density_times_vertex_alpha_squared_times_depth_fade_times_color_alpha_times_hdr_amount",
                "fragment_alpha":1.,"evaluation":"live_fragment","model_fx_shader_start":0.},
            "source_uv_animation":{"game_time_code":25,"component":3,"evaluation":"fragment","scale":scales,"speed":speeds,"mask_offset":mask_offset,
                "scroll":"uv_times_scale_plus_fraction_game_time_w_times_speed_times_scale","mask":"uv_minus_offset_times_scale_plus_offset",
                "domains":["alpha","plus","minus","mask"],"independent_domains":true},
            "source_depth_fade":{"code_image":18,"render_target_size_code":17,"near_plane_scene_byte":928,
                "factor":contract::literal(native,3371644427)?[0],"depth_epsilon":0.,"fade_epsilon":f32::from_bits(0x38d1b717),
                "operation":"saturate_near_plane_divided_by_max_absolute_scene_depth_minus_clip_w_divided_by_max_fade_factor"},
            "source_vertex_color_transfer":"linear_primary_vertex_rgb_and_squared_primary_vertex_alpha",
            "source_emissive":{"output_transfer":"sqrt_after_scene_hdr_control0"},
            "source_blend":{"rgb":[2,2,1],"alpha":[1,2,1]},
            "approximations":["Native six-layer scrolling, interpolated primary RGBA and source-unit depth softness use a linear additive forward consumer; native scene HDR control and gamma2 framebuffer composition use the shared Bevy exposure and framebuffer"],
            "pending_runtime":"Native smoke scene HDR control0 and gamma2 framebuffer composition are approximated by the shared linear forward renderer"});
        Ok(
            json!({"name":key,"pbrMetallicRoughness":{"baseColorFactor":[1.,1.,1.,1.],"metallicFactor":0.},"extras":{"surface":surface}}),
        )
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

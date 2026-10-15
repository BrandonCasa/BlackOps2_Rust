use super::*;
use half::f16;

#[derive(Clone, Copy, PartialEq)]
pub(super) enum WorldRecipe {
    Water,
    Default,
    Shadow,
    Embers,
}

pub(super) fn lookup(family: &str) -> Option<WorldRecipe> {
    Some(match family {
        "wpc_cod7water_we876z2f" => WorldRecipe::Water,
        "wpc_default" => WorldRecipe::Default,
        "wpc_shadowcaster_wj6w5j60" => WorldRecipe::Shadow,
        "wpc_sw4_3d_burning_embers_nuketown_74j6971w" => WorldRecipe::Embers,
        _ => return None,
    })
}

type Argument = (u64, u64, u64, u64, u64);

fn pass_contract(
    key: &str,
    native: &Value,
    index: usize,
    programs: [&str; 2],
    routes: &[(u8, u8)],
    arguments: &[Argument],
    state: [u32; 2],
) -> Result<()> {
    let pass = contract::pass(native, index)?;
    let routes: Vec<_> = routes
        .iter()
        .map(|&(source, destination)| json!({"source":source,"destination":destination}))
        .collect();
    if pass["vertex_shader"] != programs[0]
        || pass["pixel_shader"] != programs[1]
        || pass["vertex_routes"] != json!(routes)
    {
        return Err(format!(
            "World material {key} has unreviewed programs or vertex routes in technique {index}"
        )
        .into());
    }
    let active = pass["arguments"]
        .as_array()
        .ok_or("Missing world material arguments")?;
    if active.len() != arguments.len()
        || arguments
            .iter()
            .any(|&(kind, value, buffer, location, size)| {
                active
                    .iter()
                    .filter(|argument| {
                        contract::argument(argument, [kind, value as u64, buffer, location, size])
                    })
                    .count()
                    != 1
            })
    {
        return Err(format!(
            "World material {key} has unreviewed active arguments in technique {index}"
        )
        .into());
    }
    let selected = native["state_entries"][index]
        .as_u64()
        .ok_or("Missing world material state index")? as usize;
    if native["state_bits"].get(selected) != Some(&json!(state)) {
        return Err(
            format!("World material {key} has unreviewed state in technique {index}").into(),
        );
    }
    for &(kind, hash, _, _, _) in arguments {
        if matches!(kind, 0 | 6) {
            let constants = native["constants"]
                .as_array()
                .ok_or("Missing world material constants")?;
            if constants
                .iter()
                .filter(|constant| constant["hash"] == hash)
                .count()
                != 1
            {
                return Err(format!(
                    "World material {key} has missing or duplicate bound constant {hash}"
                )
                .into());
            }
            contract::literal(native, hash as u32)?;
        }
    }
    Ok(())
}

impl WorldRecipe {
    pub(super) fn index(self) -> usize {
        match self {
            Self::Default | Self::Shadow => 2,
            Self::Water | Self::Embers => 4,
        }
    }

    pub(super) fn validate(self, key: &str, native: &Value) -> Result<()> {
        if native["technique_set"].as_str().and_then(lookup) != Some(self) {
            return Err(format!("World material {key} does not match its selected recipe").into());
        }
        let transforms = [(3, 67109077, 3, 0, 64), (3, 67109093, 0, 576, 64)];
        let bindings: &[(u32, u8, u8)] = match self {
            Self::Default => &[(2695565377, 1, 2)],
            Self::Shadow => &[(2695565377, 20, 2)],
            Self::Water => &[(449481358, 20, 5), (449481359, 20, 5)],
            Self::Embers => &[
                (2485960688, 20, 5),
                (2799170835, 20, 2),
                (3617419774, 20, 2),
                (4030327853, 20, 2),
            ],
        };
        match self {
            Self::Default => pass_contract(
                key,
                native,
                2,
                ["pimp_shader_texturedsimple_a25e2fda.hlsl"; 2],
                &[(0, 0), (2, 5)],
                &[(3, 67109097, 1, 64, 64), (2, 2695565377, 0, 0, 1)],
                [403851282, 13],
            )?,
            Self::Shadow => {
                let mut arguments = transforms.to_vec();
                arguments.push((2, 2695565377, 0, 0, 1));
                for index in [2, 3] {
                    pass_contract(
                        key,
                        native,
                        index,
                        ["pimp_shader_vertcolorsimple_b0625f15.hlsl"; 2],
                        &[(0, 0), (1, 2), (2, 5)],
                        &arguments,
                        [403867666, 13],
                    )?;
                }
                for (index, state) in [(0, [1214482, 1074004045]), (1, [1214482, 61])] {
                    let selected = native["state_entries"][index]
                        .as_u64()
                        .ok_or("Missing shadow-only depth state")?
                        as usize;
                    if native["state_bits"].get(selected) != Some(&json!(state)) {
                        return Err(format!(
                            "Shadow-only material {key} has unreviewed depth/shadow state"
                        )
                        .into());
                    }
                }
                if [4, 6].into_iter().any(|index| {
                    native["techniques"].as_array().is_some_and(|techniques| {
                        techniques
                            .iter()
                            .any(|technique| technique["index"] == index)
                    }) || native["state_entries"][index] != 255
                }) {
                    return Err(format!(
                        "Shadow-only material {key} unexpectedly has a lit presentation pass"
                    )
                    .into());
                }
            }
            Self::Water => {
                let mut arguments = transforms.to_vec();
                arguments.extend([
                    (0, 27397098, 2, 48, 16),
                    (0, 323802060, 2, 64, 16),
                    (0, 1855396200, 2, 32, 16),
                    (0, 3341333671, 2, 16, 16),
                    (2, 449481358, 0, 257, 1),
                    (2, 449481359, 0, 0, 1),
                    (3, 16777241, 2, 0, 16),
                    (4, 18, 0, 514, 1),
                    (4, 9, 0, 1283, 1),
                    (5, 16777233, 0, 1520, 16),
                    (6, 2251460126, 2, 160, 16),
                    (6, 2815841958, 2, 96, 16),
                    (6, 2815841959, 2, 80, 16),
                    (6, 3178532486, 2, 112, 16),
                    (6, 3178532507, 2, 128, 16),
                ]);
                for (index, pixel) in [
                    (4, "pimp_shader_cod7water_fcb1a754.hlsl"),
                    (6, "pimp_shader_cod7water_362781fd.hlsl"),
                ] {
                    pass_contract(
                        key,
                        native,
                        index,
                        ["pimp_shader_cod7water_fcb1a754.hlsl", pixel],
                        &[(0, 0), (1, 2), (5, 6)],
                        &arguments,
                        [135432210, 13],
                    )?;
                }
            }
            Self::Embers => {
                let mut arguments = transforms.to_vec();
                arguments.extend([
                    (0, 769609961, 2, 96, 4),
                    (0, 769610231, 2, 92, 4),
                    (0, 2629851747, 2, 100, 4),
                    (2, 2485960688, 0, 256, 1),
                    (2, 2799170835, 0, 513, 1),
                    (2, 3617419774, 0, 771, 1),
                    (2, 4030327853, 0, 1026, 1),
                    (3, 16777241, 2, 64, 16),
                    (5, 16777241, 2, 64, 16),
                    (6, 882815034, 2, 80, 4),
                    (6, 4131458919, 2, 84, 8),
                ]);
                let routes = &[(0, 0), (1, 2), (2, 5), (3, 1), (4, 7), (5, 6)];
                pass_contract(
                    key,
                    native,
                    4,
                    [
                        "pimp_shader_sw4_3d_burning_embers_nuketown_9e139b80.hlsl",
                        "pimp_shader_sw4_3d_burning_embers_nuketown_fbd23a5a.hlsl",
                    ],
                    routes,
                    &arguments,
                    [422084965, 28],
                )?;
                arguments.extend([
                    (3, 67109101, 0, 768, 64),
                    (4, 6, 0, 2313, 1),
                    (5, 16777253, 0, 1552, 16),
                    (5, 16777254, 0, 1584, 16),
                ]);
                pass_contract(
                    key,
                    native,
                    6,
                    [
                        "pimp_shader_sw4_3d_burning_embers_nuketown_2e02029b.hlsl",
                        "pimp_shader_sw4_3d_burning_embers_nuketown_6049e18a.hlsl",
                    ],
                    routes,
                    &arguments,
                    [422084965, 28],
                )?;
            }
        }
        for &(hash, sampler, semantic) in bindings {
            let textures = native["textures"]
                .as_array()
                .ok_or("Missing world material texture table")?;
            if textures
                .iter()
                .filter(|binding| binding["hash"] == hash)
                .count()
                != 1
            {
                return Err(format!(
                    "World material {key} has missing or duplicate active image {hash}"
                )
                .into());
            }
            let binding = contract::texture_binding(native, hash)?;
            if binding["sampler"] != sampler
                || binding["semantic"] != semantic
                || binding["image"].as_str().is_none_or(str::is_empty)
            {
                return Err(
                    format!("World material {key} has unreviewed image binding {hash}").into(),
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
        let mut surface = json!({"version":3,"identity":key,"native":native,"channels":[],
            "standard":{"unlit":self == Self::Default,"roughness":1.,"metallic":0.,"reflectance":0.,"base_color":[1.,1.,1.,1.]},
            "presentation":{"alpha":"opaque","cull":if self == Self::Default {"none"} else {"back"},"fog":false,"visible":self != Self::Shadow},
            "approximations":[]});
        match self {
            Self::Default => {
                let binding = contract::texture_binding(native, 2695565377)?;
                let source = image(native, images, 2695565377)?;
                let name = binding["image"].as_str().unwrap().to_ascii_lowercase();
                let channel_key = (name, "default-gamma2".into());
                let uri = if let Some(uri) = shared.get(&channel_key) {
                    uri.clone()
                } else {
                    let baked = ordinary::bake(source, "color", None)?;
                    let uri = store_image(
                        &baked,
                        format!("prepared/material-channels/{}/default.ktx2", shared.len()),
                        true,
                        "color",
                        pack,
                    )?;
                    shared.insert(channel_key, uri.clone());
                    uri
                };
                surface["channels"] =
                    json!([{"uri":uri,"role":"color","layer":0,"color_space":"srgb","sampler":1}]);
                surface["source_vertex_color_transfer"] = json!("unused");
                surface["source_coverage"] = json!("opaque");
                surface["approximations"] = json!([
                    "Native texturedsimple encoded RGB mapped through gamma2 to the shared linear unlit consumer; original nearest sampler and default artwork retained"
                ]);
            }
            Self::Shadow => {
                surface["source_shadow_only"] = json!({"visible_lit_pass":false,"cast_shadows":true,
                    "depth_prepass":{"state":native["state_bits"][native["state_entries"][0].as_u64().unwrap() as usize],"color_write":false},
                    "shadowmap":{"state":native["state_bits"][native["state_entries"][1].as_u64().unwrap() as usize],"color_write":false},
                    "tool_techniques":[2,3],"tool_color_excluded_from_presentation":true});
                surface["pending_runtime"] = json!(
                    "Native shadowcaster has no visible lit pass; its depth-prepass and shadowmap-only geometry consumer is not connected. Tool caulk/color is not a visible replacement"
                );
            }
            Self::Water => {
                let controls = WaterControls::read(native)?;
                let normal = ordinary::bake(image(native, images, 449481359)?, "normal", None)?;
                let uri = store_image(&normal, format!("prepared/material-channels/{}/water-normal.ktx2", shared.len()), false, "normal", pack)?;
                shared.insert((key.into(), "water-standard".into()), uri.clone());
                surface["channels"] = json!([{"uri":uri,"role":"normal","layer":0,"color_space":"linear","sampler":20}]);
                surface["standard"]["base_color"] = json!(controls.color_s);
                surface["standard"]["roughness"] = json!(0.);
                surface["standard"]["reflectance"] = json!((controls.control0[2].max(0.) / 0.16).sqrt());
                surface["presentation"]["alpha"] = json!("blend");
                surface["presentation"]["fog"] = json!(true);
                surface["uv_animation"] = json!({"translation":controls.velocity(0),"time_mode":"elapsed"});
                surface["source_water"] = json!({"consumer":"standard_material"});
                surface["approximations"] = json!(["Source water palette, first normal map and scroll use stock transparent PBR; four world-position slope domains, view palette, native depth/refraction and lighting response are omitted"]);
            }
            Self::Embers => {
                let controls = EmberControls::read(native)?;
                let color = bake_embers(native, images, controls, 0.)?;
                let normal = ordinary::bake(image(native, images, 2485960688)?, "normal", None)?;
                let mut channels = Vec::new();
                for (role, image) in [("color", color), ("normal", normal)] {
                    let uri = store_image(&image, format!("prepared/material-channels/{}/ember-{role}.ktx2", shared.len()), false, role, pack)?;
                    shared.insert((key.into(), format!("ember-standard-{role}")), uri.clone());
                    channels.push(json!({"uri":uri,"role":role,"layer":0,"color_space":"linear","sampler":20}));
                }
                surface["channels"] = json!(channels);
                surface["presentation"]["alpha"] = json!("blend");
                surface["presentation"]["fog"] = json!(true);
                surface["source_embers"] = json!({"consumer":"standard_material"});
                surface["approximations"] = json!(["Source ember composition at time zero and normal map use stock blended PBR; temporal flicker, independent scroll, native lightmap and reflection response are omitted"]);
            }
        }
        Ok(
            json!({"name":key,"pbrMetallicRoughness":{"baseColorFactor":[1.,1.,1.,1.],"metallicFactor":0.},"extras":{"surface":surface}}),
        )
    }
}

fn image<'a>(
    native: &Value,
    images: &'a BTreeMap<String, texture::Image>,
    hash: u32,
) -> Result<&'a texture::Image> {
    let binding = contract::texture_binding(native, hash)?;
    let name = binding["image"]
        .as_str()
        .ok_or("Missing world material image name")?
        .to_ascii_lowercase();
    let image = images
        .get(&name)
        .ok_or_else(|| format!("Missing active world material image {name}"))?;
    if image.width == 0
        || image.height == 0
        || image.depth != 1
        || image.faces != 1
        || image.levels.is_empty()
    {
        return Err(format!("Active world material image {name} is not a valid 2D image").into());
    }
    Ok(image)
}

struct WaterControls {
    _position: [f32; 4],
    speed: [f32; 4],
    lo: [f32; 4],
    hi: [f32; 4],
    _normal: [f32; 4],
    control0: [f32; 4],
    _control1: [f32; 4],
    _color_n: [f32; 4],
    color_s: [f32; 4],
}

impl WaterControls {
    fn read(native: &Value) -> Result<Self> {
        Ok(Self {
            _position: contract::literal(native, 3341333671)?,
            speed: contract::literal(native, 323802060)?,
            lo: contract::literal(native, 1855396200)?,
            hi: contract::literal(native, 27397098)?,
            _normal: contract::literal(native, 2251460126)?,
            control0: contract::literal(native, 2815841959)?,
            _control1: contract::literal(native, 2815841958)?,
            _color_n: contract::literal(native, 3178532486)?,
            color_s: contract::literal(native, 3178532507)?,
        })
    }

    fn velocity(&self, layer: usize) -> [f32; 2] {
        let directions = if layer < 2 { self.lo } else { self.hi };
        let offset = (layer % 2) * 2;
        [
            directions[offset] * self.speed[layer],
            directions[offset + 1] * self.speed[layer],
        ]
    }

    #[cfg(test)]
    fn uv(&self, layer: usize, source_xy: [f32; 2], time: f32) -> [f32; 2] {
        let velocity = self.velocity(layer);
        std::array::from_fn(|axis| source_xy[axis] * self._position[layer] + time * velocity[axis])
    }
}

#[cfg(test)]
fn water_normal(
    native: &Value,
    images: &BTreeMap<String, texture::Image>,
    source_xy: [f32; 2],
    time: f32,
    level: usize,
) -> Result<[f32; 3]> {
    let controls = WaterControls::read(native)?;
    let mut slope = [0., 0.];
    for layer in 0..4 {
        let hash = if layer < 2 { 449481359 } else { 449481358 };
        let source = image(native, images, hash)?;
        let level = level.min(source.levels.len() - 1);
        let uv = controls.uv(layer, source_xy, time);
        if uv.iter().any(|component| !component.is_finite()) {
            return Err("Non-finite source water coordinates".into());
        }
        let sample = texture::sample(
            &texture::pixels(source, level)?,
            (source.width >> level).max(1),
            (source.height >> level).max(1),
            uv,
            20,
        );
        let index = if layer < 2 { 0 } else { 2 };
        for axis in 0..2 {
            slope[axis] += sample[axis] * controls._normal[index] + controls._normal[index + 1];
        }
    }
    let length = (slope[0] * slope[0] + slope[1] * slope[1] + 1.).sqrt();
    let result = [slope[0] / length, slope[1] / length, 1. / length];
    if result.iter().any(|component| !component.is_finite()) {
        return Err("Non-finite source water normal".into());
    }
    Ok(result)
}

const FLICKER_WEIGHTS: [f32; 4] = [81.239487, 17.348024, 37.349838, 59.39484];

#[derive(Clone, Copy)]
struct EmberControls {
    scale: f32,
    direction: [f32; 2],
    minimum: f32,
    maximum: f32,
    seed: f32,
}

impl EmberControls {
    fn read(native: &Value) -> Result<Self> {
        let direction = contract::literal(native, 4131458919)?;
        Ok(Self {
            scale: contract::literal(native, 882815034)?[0],
            direction: [direction[0], direction[1]],
            minimum: contract::literal(native, 769609961)?[0],
            maximum: contract::literal(native, 769610231)?[0],
            seed: contract::literal(native, 2629851747)?[0],
        })
    }

    fn flicker(self, time: f32) -> f32 {
        let ten = time * 10.;
        let q = (ten - (ten - ten.floor())) * 0.1 + self.seed;
        let mut state = [q; 4];
        for axis in [0, 1, 2, 3, 0, 1, 2, 0] {
            let dot = state
                .into_iter()
                .zip(FLICKER_WEIGHTS)
                .map(|(a, b)| a * b)
                .sum::<f32>();
            state[axis] = dot - dot.floor();
        }
        self.minimum + (self.maximum - self.minimum) * state[0]
    }

    fn uv(self, uv: [f32; 2], time: f32) -> [f32; 2] {
        std::array::from_fn(|axis| {
            let scroll = time * self.direction[axis] * self.scale;
            let phase = scroll - scroll.floor();
            uv[axis] * self.scale + phase
        })
    }
}

fn ember_pixel(diffuse: [f32; 4], mask: [f32; 4], ember: [f32; 4], flicker: f32) -> [f32; 4] {
    std::array::from_fn(|axis| {
        let candidate = ember[axis] * flicker * mask[0] * if axis < 3 { 6. } else { 1. };
        let mixed = diffuse[axis] + mask[1] * (candidate - diffuse[axis]);
        if axis < 3 { mixed * mixed } else { mixed }
    })
}

fn bake_embers(
    native: &Value,
    images: &BTreeMap<String, texture::Image>,
    controls: EmberControls,
    time: f32,
) -> Result<texture::Image> {
    let sources = [
        image(native, images, 4030327853)?,
        image(native, images, 2799170835)?,
        image(native, images, 3617419774)?,
    ];
    let width = sources.iter().map(|image| image.width).max().unwrap();
    let height = sources.iter().map(|image| image.height).max().unwrap();
    let levels = sources
        .iter()
        .map(|image| image.levels.len())
        .max()
        .unwrap();
    let decoded = sources
        .iter()
        .map(|image| {
            (0..image.levels.len())
                .map(|level| texture::pixels(image, level))
                .collect::<Result<Vec<_>>>()
        })
        .collect::<Result<Vec<_>>>()?;
    let flicker = controls.flicker(time);
    if !flicker.is_finite() {
        return Err("Non-finite native ember flicker".into());
    }
    let mut result = texture::Image {
        width,
        height,
        depth: 1,
        faces: 1,
        format: 19,
        flags: 0,
        levels: Vec::new(),
    };
    for level in 0..levels {
        let w = (width >> level).max(1);
        let h = (height >> level).max(1);
        let mut bytes = Vec::with_capacity(w * h * 8);
        for y in 0..h {
            for x in 0..w {
                let uv = [(x as f32 + 0.5) / w as f32, (y as f32 + 0.5) / h as f32];
                let sample = |index: usize, uv| {
                    let source = sources[index];
                    let bias = width.max(height).ilog2() - source.width.max(source.height).ilog2();
                    let mip = level
                        .saturating_sub(bias as usize)
                        .min(source.levels.len() - 1);
                    texture::sample(
                        &decoded[index][mip],
                        (source.width >> mip).max(1),
                        (source.height >> mip).max(1),
                        uv,
                        20,
                    )
                };
                let value = ember_pixel(
                    sample(0, uv),
                    sample(1, uv),
                    sample(2, controls.uv(uv, time)),
                    flicker,
                );
                for component in value {
                    let half = f16::from_f32(component);
                    if !component.is_finite() || !half.is_finite() {
                        return Err("Non-finite converted burning ember channel".into());
                    }
                    bytes.extend(half.to_le_bytes());
                }
            }
        }
        result.levels.push(bytes);
    }
    Ok(result)
}
#[cfg(test)]
mod tests {
    use super::*;

    fn rgba_image(width: usize, height: usize, values: Vec<u8>) -> texture::Image {
        texture::Image {
            width,
            height,
            depth: 1,
            faces: 1,
            format: 1,
            flags: 0,
            levels: vec![values],
        }
    }

    #[test]
    fn ember_bake_preserves_hdr_texture_energy_and_raw_coverage() {
        let native = json!({"textures":[{"hash":4030327853_u32,"image":"diffuse"},
            {"hash":2799170835_u32,"image":"mask"},{"hash":3617419774_u32,"image":"ember"}]});
        let images = BTreeMap::from([
            ("diffuse".into(), rgba_image(1, 1, vec![0; 4])),
            ("mask".into(), rgba_image(1, 1, vec![255, 255, 0, 255])),
            ("ember".into(), rgba_image(1, 1, vec![255, 128, 0, 128])),
        ]);
        let controls = EmberControls {
            scale: 1.,
            direction: [0.; 2],
            minimum: 1.,
            maximum: 1.,
            seed: 2.,
        };
        let baked = bake_embers(&native, &images, controls, 0.).unwrap();
        assert_eq!(baked.format, 19);
        let pixel = texture::pixels(&baked, 0).unwrap()[0];
        assert_eq!(pixel[0], 36.);
        assert!((pixel[1] - 9.070727).abs() < 0.008);
        assert_eq!(pixel[2], 0.);
        assert!((pixel[3] - 128. / 255.).abs() < 0.0005);
    }

    #[test]
    fn water_normal_samples_independent_position_domains_before_world_slope_normalization() {
        let constants = [
            (3341333671_u32, [1.; 4]),
            (323802060, [0., 0.5, 0., 0.]),
            (1855396200, [0., 0., 0., 1.]),
            (27397098, [0.; 4]),
            (2251460126, [2., -1., 0., 0.]),
            (2815841959, [0.; 4]),
            (2815841958, [0.; 4]),
            (3178532486, [0.; 4]),
            (3178532507, [0.; 4]),
        ];
        let native = json!({"constants":constants.into_iter().map(|(hash,literal)|json!({"hash":hash,"literal":literal})).collect::<Vec<_>>(),
            "textures":[{"hash":449481359_u32,"image":"lo"},{"hash":449481358_u32,"image":"hi"}]});
        let images = BTreeMap::from([
            (
                "lo".into(),
                rgba_image(
                    2,
                    2,
                    vec![
                        0, 0, 0, 255, 255, 0, 0, 255, 0, 255, 0, 255, 255, 255, 0, 255,
                    ],
                ),
            ),
            ("hi".into(), rgba_image(1, 1, vec![128; 4])),
        ]);
        let zero = water_normal(&native, &images, [0.25, 0.25], 0., 0).unwrap();
        for (actual, expected) in zero.into_iter().zip([-2. / 3., -2. / 3., 1. / 3.]) {
            assert!((actual - expected).abs() < 1e-6);
        }
        let one = water_normal(&native, &images, [0.25, 0.25], 1., 0).unwrap();
        for (actual, expected) in one
            .into_iter()
            .zip([-2. / 5_f32.sqrt(), 0., 1. / 5_f32.sqrt()])
        {
            assert!((actual - expected).abs() < 1e-6);
        }
        let shifted = water_normal(&native, &images, [0.75, 0.25], 1., 0).unwrap();
        for (actual, expected) in
            shifted
                .into_iter()
                .zip([2. / 5_f32.sqrt(), 0., 1. / 5_f32.sqrt()])
        {
            assert!((actual - expected).abs() < 1e-6);
        }
    }

    #[test]
    fn ember_mask_channels_and_hdr_energy_remain_distinct() {
        assert_eq!(
            ember_pixel([0.5, 0.25, 0.75, 0.8], [1., 0., 1., 1.], [1.; 4], 1.),
            [0.25, 0.0625, 0.5625, 0.8]
        );
        assert_eq!(
            ember_pixel([0.; 4], [0.5, 1., 0., 1.], [1., 0.5, 0.25, 0.8], 1.),
            [9., 2.25, 0.5625, 0.4]
        );
        let a = ember_pixel([0.5; 4], [0.5, 0.5, 0., 1.], [0.25; 4], 0.9);
        let b = ember_pixel([0.5; 4], [0.5, 0.5, 1., 1.], [0.25; 4], 0.9);
        assert_eq!(a, b);
    }

    #[test]
    fn ember_flicker_is_seeded_quantized_and_has_no_short_loop() {
        let controls = EmberControls {
            scale: 1.,
            direction: [-0.01, -0.02],
            minimum: 0.89,
            maximum: 1.,
            seed: 2.,
        };
        assert_eq!(controls.flicker(0.001), controls.flicker(0.099));
        assert_ne!(controls.flicker(0.099), controls.flicker(0.101));
        assert_ne!(controls.flicker(0.001), controls.flicker(10.001));
        assert_ne!(
            controls.flicker(0.),
            EmberControls {
                seed: 3.,
                ..controls
            }
            .flicker(0.)
        );
        assert!((0..100).all(|tick| (0.89..=1.).contains(&controls.flicker(tick as f32 * 0.1))));
    }

    #[test]
    fn ember_negative_scroll_uses_native_fractional_phase() {
        let controls = EmberControls {
            scale: 2.,
            direction: [-0.25, 0.5],
            minimum: 0.,
            maximum: 1.,
            seed: 0.,
        };
        assert_eq!(controls.uv([0.25, 0.125], 0.5), [1.25, 0.75]);
        assert_eq!(controls.uv([0.25, 0.125], 100_000_000.), [0.5, 0.25]);
    }

    #[test]
    fn water_domains_use_source_positions_and_unwrapped_time() {
        let controls = WaterControls {
            position: [0.015625, 0.0078125, 0.020833334, 0.020833334],
            speed: [0.1, 0.075, 0.05, 0.05],
            lo: [1., 0., 0., 1.],
            hi: [1., 0., -0.86602545, -0.49999991],
            normal: [0.16, -0.08, 0.06, -0.03],
            control0: [0.; 4],
            control1: [0.; 4],
            color_n: [0.; 4],
            color_s: [0.; 4],
        };
        assert_eq!(controls.uv(0, [64., 32.], 10.), [2., 0.5]);
        assert_eq!(controls.uv(1, [64., 32.], 10.), [0.5, 1.]);
        assert!((controls.uv(3, [64., 32.], 10.)[0] - 0.9003206).abs() < 1e-6);
        assert_eq!(
            controls.uv(0, [64., 32.], 20.)[0] - controls.uv(0, [64., 32.], 10.)[0],
            1.
        );
    }
}

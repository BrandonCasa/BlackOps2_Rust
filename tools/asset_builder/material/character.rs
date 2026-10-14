use super::*;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum CharacterRecipe {
    Cloth,
    Skin,
    Cornea,
    Optic,
    Transparent,
    Canopy,
    CanopyShadow,
    CanopyBlend,
}

pub(super) fn lookup(family: &str) -> Option<CharacterRecipe> {
    Some(match family {
        "mc_sw4_3d_char_cloth_4z8fq5wu" => CharacterRecipe::Cloth,
        "mc_sw4_3d_char_skin_j92387z3" => CharacterRecipe::Skin,
        "mc_sw4_3d_char_eye_cornea_2eww29wu" => CharacterRecipe::Cornea,
        "mc_sw4_3d_optic_lens_4w47028j" => CharacterRecipe::Optic,
        "mc_sw4_3d_viewmodel_transparent_z9z0z75f" => CharacterRecipe::Transparent,
        "mc_treecanopy_3z378q8f" => CharacterRecipe::Canopy,
        "mc_treecanopy_sm_68zw237j" => CharacterRecipe::CanopyShadow,
        "mc_treecanopy_sm_j9ff996z" => CharacterRecipe::CanopyBlend,
        _ => return None,
    })
}

impl CharacterRecipe {
    pub(super) fn index(self) -> usize {
        4
    }

    fn canopy(self) -> bool {
        matches!(self, Self::Canopy | Self::CanopyShadow | Self::CanopyBlend)
    }

    fn transparent(self) -> bool {
        matches!(
            self,
            Self::Transparent | Self::Optic | Self::Cornea | Self::CanopyBlend
        )
    }

    fn programs(self, sun: bool) -> (&'static str, &'static str) {
        match (self, sun) {
            (Self::Cloth, false) => (
                "pimp_shader_sw4_3d_phong_emissive_alcatraz_b7d697d7.hlsl",
                "pimp_shader_sw4_3d_char_cloth_49ec0efa.hlsl",
            ),
            (Self::Cloth, true) => (
                "pimp_shader_sw4_3d_phong_emissive_alcatraz_7c70ecc.hlsl",
                "pimp_shader_sw4_3d_char_cloth_a995d189.hlsl",
            ),
            (Self::Skin, false) => (
                "pimp_shader_sw4_3d_phong_emissive_alcatraz_b7d697d7.hlsl",
                "pimp_shader_sw4_3d_char_skin_7f1d600.hlsl",
            ),
            (Self::Skin, true) => (
                "pimp_shader_sw4_3d_phong_emissive_alcatraz_7c70ecc.hlsl",
                "pimp_shader_sw4_3d_char_skin_abc7bb0a.hlsl",
            ),
            (Self::Cornea, false) => (
                "pimp_shader_sw4_3d_char_eye_cornea_bac46018.hlsl",
                "pimp_shader_sw4_3d_char_eye_cornea_566a2eb6.hlsl",
            ),
            (Self::Cornea, true) => (
                "pimp_shader_sw4_3d_char_eye_cornea_7b908494.hlsl",
                "pimp_shader_sw4_3d_char_eye_cornea_9371bd55.hlsl",
            ),
            (Self::Optic, false) => (
                "pimp_shader_sw4_3d_viewmodel_transparent_1a1ccc66.hlsl",
                "pimp_shader_sw4_3d_optic_lens_9511e3ca.hlsl",
            ),
            (Self::Optic, true) => (
                "pimp_shader_sw4_3d_viewmodel_transparent_7a381d58.hlsl",
                "pimp_shader_sw4_3d_optic_lens_75683cb9.hlsl",
            ),
            (Self::Transparent, false) => (
                "pimp_shader_sw4_3d_viewmodel_transparent_1a1ccc66.hlsl",
                "pimp_shader_sw4_3d_viewmodel_transparent_d3851d22.hlsl",
            ),
            (Self::Transparent, true) => (
                "pimp_shader_sw4_3d_viewmodel_transparent_7a381d58.hlsl",
                "pimp_shader_sw4_3d_viewmodel_transparent_c58fadfb.hlsl",
            ),
            (Self::Canopy, false) => (
                "pimp_shader_treecanopy_b6c201c.hlsl",
                "pimp_shader_treecanopy_ccc7fb2e.hlsl",
            ),
            (Self::Canopy, true) => (
                "pimp_shader_treecanopy_510833f2.hlsl",
                "pimp_shader_treecanopy_ccc7fb2e.hlsl",
            ),
            (Self::CanopyShadow, false) => (
                "pimp_shader_treecanopy_b6c201c.hlsl",
                "pimp_shader_treecanopy_ccc7fb2e.hlsl",
            ),
            (Self::CanopyShadow, true) => (
                "pimp_shader_treecanopy_f01e2cc8.hlsl",
                "pimp_shader_treecanopy_37b5f7fa.hlsl",
            ),
            (Self::CanopyBlend, false) => (
                "pimp_shader_treecanopy_b6c201c.hlsl",
                "pimp_shader_treecanopy_d0a663d0.hlsl",
            ),
            (Self::CanopyBlend, true) => (
                "pimp_shader_treecanopy_f01e2cc8.hlsl",
                "pimp_shader_treecanopy_2bd46f04.hlsl",
            ),
        }
    }

    fn textures(self) -> Vec<(u32, u64, &'static str)> {
        match self {
            Self::Cloth | Self::Skin => vec![
                (2351529600, 514, "source_specular_gloss"),
                (2485960688, 256, "source_normal"),
                (4030327853, 769, "source_color"),
            ],
            Self::Transparent => vec![
                (2351529600, 514, "source_specular_gloss"),
                (2485960688, 256, "source_normal"),
                (3053961470, 769, "source_color"),
            ],
            Self::Optic => vec![
                (2485960688, 256, "source_normal"),
                (2940931118, 515, "source_specular_color"),
                (3053961470, 1025, "source_color"),
                (3548237271, 770, "source_gloss"),
            ],
            Self::Cornea => vec![
                (4017940, 257, "source_highlight_mask"),
                (188291226, 512, "source_highlight_normal"),
            ],
            Self::Canopy | Self::CanopyShadow | Self::CanopyBlend => {
                vec![(2695565377, 0, "source_color")]
            }
        }
    }

    fn constants(self, sun: bool) -> Vec<(u32, u64, u64, u64, u64)> {
        match self {
            Self::Cloth => Vec::new(),
            Self::Skin => vec![
                (211931156, 6, 2, 80, 4),
                (1012612089, 6, 2, 88, 4),
                (1307013020, 6, 2, 84, 4),
            ],
            Self::Transparent => vec![(211931156, 6, 2, 84, 4), (1307013020, 6, 2, 80, 4)],
            Self::Optic => vec![
                (211931156, 6, 2, 84, 4),
                (994574754, 6, 2, 88, 4),
                (1307013020, 6, 2, 80, 4),
            ],
            Self::Cornea => vec![
                (1607250247, 0, 2, 112, 12),
                (2912016085, 0, 2, 80, 12),
                (287662256, 6, 2, 92, 4),
                (606829963, 6, 2, 132, 4),
                (2675429970, 6, 2, 96, 4),
                (3136287143, 6, 2, 124, 4),
                (3466487505, 6, 2, 128, 4),
                (3598703080, 6, 2, 100, 4),
                (3960860966, 6, 2, 136, 4),
            ],
            Self::Canopy | Self::CanopyShadow | Self::CanopyBlend => {
                let mut values = vec![(1041987663, 0, 2, 48, 16), (3054254906, 6, 1, 1024, 16)];
                if sun {
                    values.push((4125303575, 0, 2, 32, 16));
                }
                values
            }
        }
    }

    pub(super) fn validate(self, key: &str, native: &Value) -> Result<()> {
        if lookup(
            native["technique_set"]
                .as_str()
                .ok_or("Missing character/canopy family")?,
        ) != Some(self)
        {
            return Err(format!("Character/canopy recipe does not match {key}").into());
        }
        let mut routes = vec![json!({"source":0,"destination":0})];
        if self != Self::Cornea {
            routes.push(json!({"source":1,"destination":2}));
        }
        routes.extend([
            json!({"source":2,"destination":5}),
            json!({"source":3,"destination":1}),
        ]);
        if !self.canopy() {
            routes.push(json!({"source":4,"destination":7}));
        }
        let textures = self.textures();
        let mut selected_state = None;
        for index in [4, 6] {
            let pass = contract::pass(native, index)?;
            let (vertex, pixel) = self.programs(index == 6);
            if pass["vertex_shader"] != vertex
                || pass["pixel_shader"] != pixel
                || pass["vertex_routes"] != json!(routes)
            {
                return Err(format!(
                    "Character/canopy material {key} has unreviewed programs or vertex routes"
                )
                .into());
            }
            let arguments = pass["arguments"]
                .as_array()
                .ok_or("Missing character/canopy arguments")?;
            let active: Vec<_> = arguments
                .iter()
                .filter(|argument| argument["type"] == 2)
                .collect();
            if active.len() != textures.len()
                || textures.iter().any(|&(hash, location, _)| {
                    active
                        .iter()
                        .filter(|argument| {
                            argument["value"] == hash
                                && argument["location"] == location
                                && argument["buffer"] == 0
                                && argument["size"] == 1
                        })
                        .count()
                        != 1
                })
            {
                return Err(format!(
                    "Character/canopy material {key} has unreviewed active image bindings"
                )
                .into());
            }
            let constants = self.constants(index == 6);
            let active: Vec<_> = arguments
                .iter()
                .filter(|argument| matches!(argument["type"].as_u64(), Some(0 | 1 | 6 | 7 | 8)))
                .collect();
            if active.len() != constants.len() {
                return Err(format!(
                    "Character/canopy material {key} has unreviewed active constants"
                )
                .into());
            }
            for (hash, kind, buffer, location, size) in constants {
                if active
                    .iter()
                    .filter(|argument| {
                        argument["value"] == hash
                            && argument["type"] == kind
                            && argument["buffer"] == buffer
                            && argument["location"] == location
                            && argument["size"] == size
                    })
                    .count()
                    != 1
                {
                    return Err(format!("Character/canopy material {key} has an unreviewed binding for constant {hash}").into());
                }
                contract::literal(native, hash)?;
            }
            if matches!(self, Self::Cloth | Self::Skin)
                && arguments
                    .iter()
                    .filter(|argument| {
                        argument["type"] == 5
                            && argument["value"] == 16777385_u32
                            && argument["buffer"] == 1
                            && argument["location"] == 368
                            && argument["size"] == 16
                    })
                    .count()
                    != 1
            {
                return Err(format!(
                    "Character material {key} lacks the native charred object input"
                )
                .into());
            }
            let state_index = native["state_entries"][index]
                .as_u64()
                .ok_or("Missing character/canopy state index")?
                as usize;
            let state = native["state_bits"]
                .get(state_index)
                .ok_or("Character/canopy state index is invalid")?;
            if selected_state.is_some_and(|selected| selected != state) {
                return Err(format!(
                    "Character/canopy material {key} has differing lit/sun render states"
                )
                .into());
            }
            let rgb = state[0]
                .as_u64()
                .ok_or("Missing character/canopy RGB state")? as u32;
            let depth = state[1]
                .as_u64()
                .ok_or("Missing character/canopy depth state")?;
            let allowed = match self {
                Self::Cloth | Self::Skin => vec![(403867666, 13)],
                Self::Cornea => vec![(422084965, 12)],
                Self::Optic => vec![(422084965, 12), (422068581, 12)],
                Self::Transparent
                    if matches!(
                        native["name"].as_str(),
                        Some("mc/mtl_t6_attach_tritium_red" | "mc/mtl_t6_attach_tritium_wht")
                    ) =>
                {
                    vec![(153649509, 28)]
                }
                Self::Transparent => vec![(422084965, 12), (422068581, 12)],
                Self::Canopy | Self::CanopyShadow => vec![(403853330, 13)],
                Self::CanopyBlend => vec![(422068581, 12)],
            };
            if !allowed.contains(&(rgb, depth)) {
                return Err(format!(
                    "Character/canopy material {key} has an unreviewed blend, cull or depth state"
                )
                .into());
            }
            selected_state = Some(state);
        }
        for (hash, _, _) in textures {
            let binding = contract::texture_binding(native, hash)?;
            let expected = if self == Self::Cornea {
                vec![10]
            } else if self.canopy() {
                vec![20, 244]
            } else {
                vec![20]
            };
            if binding["sampler"]
                .as_u64()
                .is_none_or(|sampler| !expected.contains(&sampler))
                || binding["image"].as_str().is_none_or(str::is_empty)
            {
                return Err(format!(
                    "Character/canopy material {key} has an unreviewed image sampler for {hash}"
                )
                .into());
            }
        }
        if self == Self::Optic {
            let threshold = contract::literal(native, 994574754)?[0];
            if threshold <= 0. || !optic_decay(1. - threshold, threshold).is_finite() {
                return Err(format!(
                    "Optic material {key} has an undefined facing-decay threshold"
                )
                .into());
            }
        }
        if self == Self::Cornea {
            for (size, sharpness, brightness) in [
                (287662256, 2675429970, 3598703080),
                (3136287143, 3466487505, 606829963),
            ] {
                let controls = [
                    contract::literal(native, size)?[0],
                    contract::literal(native, sharpness)?[0],
                    contract::literal(native, brightness)?[0],
                ];
                if !cornea_highlight(1., 1., controls).is_finite() {
                    return Err(format!(
                        "Cornea material {key} has an undefined highlight response"
                    )
                    .into());
                }
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
        let mut source_inputs = BTreeMap::new();
        let mut inputs = BTreeMap::new();
        for (hash, _, role) in self.textures() {
            let binding = contract::texture_binding(native, hash)?;
            let name = binding["image"]
                .as_str()
                .ok_or("Missing character/canopy source image name")?
                .to_ascii_lowercase();
            let image = images
                .get(&name)
                .ok_or_else(|| format!("Missing character/canopy source image {name}"))?;
            if image.depth != 1 || image.faces != 1 || image.levels.is_empty() {
                return Err(format!("Character/canopy source image {name} is not 2D").into());
            }
            let uri = store_channel(&name, role, image, false, shared, pack)?;
            source_inputs.insert(role, json!({"uri":uri,"sampler":binding["sampler"],"transfer":if matches!(role,"source_color" | "source_specular_gloss" | "source_specular_color") {"encoded_gamma2"} else {"linear"}}));
            inputs.insert(
                role,
                (name, image, binding["sampler"].as_u64().unwrap() as u8),
            );
        }
        let mut channels = Vec::new();
        let mut native_specular = Value::Null;
        let mut color_space = "srgb";
        if self.canopy() {
            let (name, image, sampler) = &inputs["source_color"];
            let tint = contract::literal(native, 3054254906)?;
            let prepared = canopy_color(image, tint, self == Self::CanopyBlend)?;
            let identity = format!(
                "{name}:canopy:{:?}:{}",
                tint.map(f32::to_bits),
                self == Self::CanopyBlend
            );
            let uri = store_channel(&identity, "color", &prepared, false, shared, pack)?;
            channels.push(json!({"uri":uri,"role":"color","layer":0,"color_space":"linear","sampler":sampler}));
            color_space = "linear";
        } else if self == Self::Cornea {
            let (name, mask, sampler) = &inputs["source_highlight_mask"];
            let controls = [
                [
                    contract::literal(native, 287662256)?[0],
                    contract::literal(native, 2675429970)?[0],
                    contract::literal(native, 3598703080)?[0],
                ],
                [
                    contract::literal(native, 3136287143)?[0],
                    contract::literal(native, 3466487505)?[0],
                    contract::literal(native, 606829963)?[0],
                ],
            ];
            let prepared = cornea_envelope(mask, controls)?;
            let uri = store_channel(
                &format!(
                    "{name}:cornea-envelope:{:?}",
                    controls.map(|values| values.map(f32::to_bits))
                ),
                "color",
                &prepared,
                false,
                shared,
                pack,
            )?;
            channels.push(json!({"uri":uri,"role":"color","layer":0,"color_space":"linear","sampler":sampler}));
            color_space = "linear";
        } else {
            let (color_name, color, color_sampler) = &inputs["source_color"];
            let (normal_name, normal, normal_sampler) = &inputs["source_normal"];
            let normal = ordinary::bake(normal, "normal", None)?;
            let uri = store_channel(normal_name, "normal", &normal, false, shared, pack)?;
            channels.push(json!({"uri":uri,"role":"normal","layer":0,"color_space":"linear","sampler":normal_sampler}));
            let (specular, specular_identity, specular_sampler) = if self == Self::Optic {
                let (name, rgb, sampler) = &inputs["source_specular_color"];
                let (gloss_name, gloss, gloss_sampler) = &inputs["source_gloss"];
                (
                    combine_specular(rgb, gloss, [*sampler, *gloss_sampler])?,
                    format!("{name}:{sampler}:{gloss_name}:{gloss_sampler}:red-gloss"),
                    *sampler,
                )
            } else {
                let (name, image, sampler) = &inputs["source_specular_gloss"];
                ((*image).clone(), name.clone(), *sampler)
            };
            let specular_uri = store_channel(
                &specular_identity,
                "native-specular",
                &specular,
                false,
                shared,
                pack,
            )?;
            native_specular =
                json!({"uri":specular_uri,"gloss_channel":"alpha","specular_rgb":"gamma2"});
            let identity = format!(
                "{color_name}:{color_sampler}:{specular_identity}:{specular_sampler}:character"
            );
            if !shared.contains_key(&(identity.clone(), "color".into())) {
                let (base, response) = ordinary::metallic_workflow(
                    color,
                    &specular,
                    [*color_sampler, specular_sampler],
                    None,
                    None,
                )?;
                store_channel(&identity, "color", &base, true, shared, pack)?;
                store_channel(&identity, "response", &response, false, shared, pack)?;
            }
            for role in ["color", "response"] {
                channels.push(json!({"uri":shared[&(identity.clone(),role.into())],"role":role,"layer":0,"color_space":if role == "color" {"srgb"} else {"linear"},"sampler":color_sampler}));
            }
        }
        let state_index = native["state_entries"][4].as_u64().unwrap() as usize;
        let state = native["state_bits"][state_index][0].as_u64().unwrap() as u32;
        let cull = match state >> 14 & 3 {
            1 => "none",
            2 => "back",
            3 => "front",
            _ => unreachable!(),
        };
        let specular = !self.canopy() && self != Self::Cornea;
        let alpha = if self.transparent() {
            "blend"
        } else if self.canopy() {
            "mask"
        } else {
            "opaque"
        };
        let mut converted = json!({"name":key,"pbrMetallicRoughness":{"baseColorFactor":[1.,1.,1.,1.],"metallicFactor":if specular {1.} else {0.}},
            "extras":{"surface":{"version":3,"identity":key,"channels":channels,"native":native,"native_specular":native_specular,
            "standard":{"roughness":1.,"metallic":if specular {1.} else {0.},"reflectance":if specular {0.5} else {0.},"specular_tint":[1.,1.,1.],"base_color":[1.,1.,1.,1.]},
            "presentation":{"alpha":alpha,"cutoff":128./255.,"cull":cull,"fog":true,"visible":state & (1<<27) != 0},
            "source_inputs":source_inputs,"prepared_color_space":color_space,
            "approximations":["Native model/grid/sun lighting and gamma2 fog/framebuffer composition require matching runtime consumers"]}}});
        let surface = &mut converted["extras"]["surface"];
        if specular {
            surface["approximations"].as_array_mut().unwrap().extend([
                json!("Source slope normal reconstruction is baked before filtering"),
                json!("Native Blinn-Phong specular and gloss are fitted to upstream metallic-roughness PBR while retaining source channels"),
            ]);
        }
        match self {
            Self::Cloth | Self::Skin => {
                surface["source_vertex_color_transfer"] =
                    json!("square_before_vertex_interpolation_then_square_in_fragment");
                surface["source_character_lighting"] = json!({"diffuse":"square_diffuse_times_interpolated_vertex_squared","charred":{"code_constant":169,"component":0,"operation":"square_specular_rgb_after_multiplying_one_minus_saturated_charred","prepared_unmodulated":true},
                    "reflection_amount":if self == Self::Skin {contract::literal(native,211931156)?[0]} else {1.},"specular_amount":if self == Self::Skin {contract::literal(native,1307013020)?[0]} else {1.}});
                if self == Self::Skin {
                    surface["source_character_lighting"]["diffuse_normal_height"] =
                        json!(contract::literal(native, 1012612089)?[0]);
                    surface["source_character_lighting"]["normal_operation"] =
                        json!("scale_slope_for_diffuse_only_keep_full_specular_slope");
                }
                surface["pending_runtime"] = json!(if self == Self::Skin {
                    "Native skin requires squared vertex color before interpolation and another square in diffuse, independent diffuse/specular slope normals, dynamic object charred suppression of specular, separate reflection/direct-specular controls and native lighting/composition; those runtime consumers remain unconnected"
                } else {
                    "Native cloth requires squared vertex color before interpolation and another square in diffuse, plus dynamic object charred suppression of specular and native lighting/composition; those runtime consumers remain unconnected"
                });
            }
            Self::Transparent | Self::Optic => {
                surface["source_vertex_color_transfer"] = json!("gamma2");
                surface["source_fragment_alpha"] = json!("sqrt_texture_vertex_coverage");
                surface["source_transparent_specular"] = json!({"reflection_amount":contract::literal(native,211931156)?[0],"specular_amount":contract::literal(native,1307013020)?[0],"operation":"divide_combined_direct_and_reflected_specular_by_raw_alpha_before_final_sqrt","source_refraction":false});
                if self == Self::Optic {
                    surface["source_optic_decay"] = json!({"threshold":contract::literal(native,994574754)?[0],"normal":"geometric","operation":"smoothstep_zero_one_saturate_one_minus_dot_normal_to_view_over_threshold","targets":["direct_specular","reflection"],"prepared_unmodulated":true,"gloss_channel":"source_gloss.red"});
                }
                surface["pending_runtime"] = json!(if self == Self::Optic {
                    "Native optic lens requires geometric view-facing smoothstep attenuation of direct/reflected highlights, separate red-channel gloss, sqrt texture/vertex alpha, coverage-independent specular and native gamma2 fog/framebuffer composition; runtime consumers remain unconnected"
                } else {
                    "Native transparent viewmodel requires sqrt texture/vertex alpha, coverage-independent highlights, separate reflection/direct-specular controls and native gamma2 fog/framebuffer composition; runtime consumers remain unconnected"
                });
            }
            Self::Cornea => {
                let brightness = contract::literal(native, 3960860966)?[0].powi(2);
                surface["standard"]["base_color"] = json!([brightness, brightness, brightness, 1.]);
                surface["source_cornea"] = json!({"mask":source_inputs["source_highlight_mask"],"normal":source_inputs["source_highlight_normal"],"fill_directions":[contract::literal(native,2912016085)?,contract::literal(native,1607250247)?],
                    "highlight_sizes":[contract::literal(native,287662256)?[0],contract::literal(native,3136287143)?[0]],"highlight_sharpness":[contract::literal(native,2675429970)?[0],contract::literal(native,3466487505)?[0]],"highlight_brightness":[contract::literal(native,3598703080)?[0],contract::literal(native,606829963)?[0]],
                    "overall_brightness":contract::literal(native,3960860966)?[0],"rgb":"overall_brightness_squared_times_geometric_normal_lighting","coverage":"sqrt_saturate_sum_of_two_directional_highlights_minus_0.05_times_mask_red","prepared_authored_highlight_upper_envelope":true});
                surface["pending_runtime"] = json!(
                    "Native cornea requires two view/fill-direction highlights from the bumped normal only for alpha, with red-channel mask and square-root coverage; RGB uses constant overall brightness squared and unbumped geometric lighting. The packed coverage is the authored highlight upper envelope; live highlight and native lighting/composition consumers remain unconnected"
                );
            }
            Self::Canopy | Self::CanopyShadow | Self::CanopyBlend => {
                surface["source_canopy"] = json!({"lighting_parameters":contract::literal(native,1041987663)?,"scatter_color":contract::literal(native,4125303575)?,"color_tint":contract::literal(native,3054254906)?,
                    "normal_operation":"normalize_lerp_geometric_normal_radial_model_position_lighting_x","indirect_lighting":"max_grid_spherical_harmonics_0.1_times_vertex_alpha","vertex_rgb":"unused","coverage":if self == Self::CanopyBlend {"saturate_twice_texture_alpha"} else {"texture_alpha_cutoff_128_over_255"},"sun_scatter":"authored_scatter_rgb_and_alpha_with_lighting_parameters"});
                surface["pending_runtime"] = json!(
                    "Native canopy requires radial/geometric normal interpolation, vertex-alpha-scaled grid lighting, authored sun scatter and native fog/framebuffer composition; converted texture tint and coverage are available, but matching geometry/lighting consumers remain unconnected"
                );
            }
        }
        if self == Self::Transparent
            && native["state_bits"][state_index] == json!([153649509_u32, 28])
        {
            surface["source_render_state"] = json!({"polygon_offset":1,"color_write_rgb":true,"color_write_alpha":false,"depth_write":false,"depth_test":"less_equal"});
            let pending = surface["pending_runtime"].as_str().unwrap().to_owned();
            surface["pending_runtime"] = json!(format!(
                "{pending}; the native tritium variant additionally requires polygon offset1 and RGB-only framebuffer writes"
            ));
        }
        Ok(converted)
    }
}

fn store_channel(
    identity: &str,
    role: &str,
    image: &texture::Image,
    srgb: bool,
    shared: &mut BTreeMap<(String, String), String>,
    pack: &mut Pack,
) -> Result<String> {
    let key = (identity.to_owned(), role.to_owned());
    if let Some(uri) = shared.get(&key) {
        return Ok(uri.clone());
    }
    let uri = store_image(
        image,
        format!("prepared/material-channels/{}/{role}.ktx2", shared.len()),
        srgb,
        role,
        pack,
    )?;
    shared.insert(key, uri.clone());
    Ok(uri)
}

fn transform(
    source: &texture::Image,
    convert: impl Fn([f32; 4]) -> [f32; 4],
) -> Result<texture::Image> {
    if source.depth != 1 || source.faces != 1 || source.levels.is_empty() {
        return Err("Character/canopy conversion requires a 2D image".into());
    }
    let mut output = source.clone();
    output.format = 19;
    output.levels.clear();
    for level in 0..source.levels.len() {
        let mut bytes = Vec::new();
        for pixel in texture::pixels(source, level)? {
            let pixel = convert(pixel);
            if pixel
                .iter()
                .any(|value| !value.is_finite() || !(0. ..=65504.).contains(value))
                || pixel[3] > 1.
            {
                return Err(
                    "Character/canopy channel exceeds finite half-float or coverage range".into(),
                );
            }
            for value in pixel {
                bytes.extend(half::f16::from_f32(value).to_le_bytes());
            }
        }
        output.levels.push(bytes);
    }
    Ok(output)
}

fn canopy_color(source: &texture::Image, tint: [f32; 4], blend: bool) -> Result<texture::Image> {
    transform(source, |pixel| {
        [
            (pixel[0] * tint[0]).powi(2),
            (pixel[1] * tint[1]).powi(2),
            (pixel[2] * tint[2]).powi(2),
            if blend {
                (2. * pixel[3]).clamp(0., 1.)
            } else {
                pixel[3]
            },
        ]
    })
}

fn smooth(value: f32) -> f32 {
    let value = value.clamp(0., 1.);
    value * value * (3. - 2. * value)
}

fn cornea_highlight(half_dot: f32, fill_dot: f32, [size, sharpness, brightness]: [f32; 3]) -> f32 {
    let lower = 0.72 - 0.18 * (1. - sharpness);
    let upper = 0.72 + 0.18 * (1. - sharpness);
    let power = half_dot.clamp(0., 1.).powf(1. / size.max(0.00001));
    smooth((power - lower) / (upper - lower)) * smooth((fill_dot + 0.2) * 5.) * brightness
}

fn cornea_coverage(mask: f32, highlights: [f32; 2]) -> f32 {
    ((highlights[0] + highlights[1] - 0.05).clamp(0., 1.) * mask).sqrt()
}

fn cornea_envelope(source: &texture::Image, controls: [[f32; 3]; 2]) -> Result<texture::Image> {
    let highlights = controls
        .map(|control| cornea_highlight(0., 1., control).max(cornea_highlight(1., 1., control)));
    transform(source, |pixel| {
        [1., 1., 1., cornea_coverage(pixel[0], highlights)]
    })
}

fn optic_decay(geometric_normal_dot_view: f32, threshold: f32) -> f32 {
    smooth((1. - geometric_normal_dot_view) / threshold)
}

fn combine_specular(
    rgb: &texture::Image,
    gloss: &texture::Image,
    samplers: [u8; 2],
) -> Result<texture::Image> {
    if [rgb, gloss]
        .iter()
        .any(|image| image.faces != 1 || image.depth != 1 || image.levels.is_empty())
    {
        return Err("Optic specular inputs must be 2D images".into());
    }
    let width = rgb.width.max(gloss.width);
    let height = rgb.height.max(gloss.height);
    let mut output = texture::Image {
        width,
        height,
        depth: 1,
        faces: 1,
        format: 19,
        flags: 0,
        levels: Vec::new(),
    };
    for level in 0..rgb.levels.len().max(gloss.levels.len()) {
        let (w, h) = ((width >> level).max(1), (height >> level).max(1));
        let sources = [rgb, gloss];
        let mut decoded = Vec::new();
        for image in sources {
            let bias = width.max(height).ilog2() - image.width.max(image.height).ilog2();
            let source_level = level
                .saturating_sub(bias as usize)
                .min(image.levels.len() - 1);
            decoded.push((
                texture::pixels(image, source_level)?,
                (image.width >> source_level).max(1),
                (image.height >> source_level).max(1),
            ));
        }
        let mut bytes = Vec::with_capacity(w * h * 8);
        for y in 0..h {
            for x in 0..w {
                let uv = [(x as f32 + 0.5) / w as f32, (y as f32 + 0.5) / h as f32];
                let sample = |index: usize| {
                    texture::sample(
                        &decoded[index].0,
                        decoded[index].1,
                        decoded[index].2,
                        uv,
                        samplers[index],
                    )
                };
                let mut color = sample(0);
                color[3] = sample(1)[0];
                for value in color {
                    bytes.extend(half::f16::from_f32(value).to_le_bytes());
                }
            }
        }
        output.levels.push(bytes);
    }
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn image(rgba: [u8; 4]) -> texture::Image {
        texture::Image {
            width: 1,
            height: 1,
            depth: 1,
            faces: 1,
            format: 1,
            flags: 0,
            levels: vec![rgba.to_vec()],
        }
    }

    #[test]
    fn canopy_tint_preserves_mask_and_doubles_only_blended_coverage() {
        let source = image([128, 64, 32, 80]);
        let masked = texture::pixels(&canopy_color(&source, [0.5, 2., 1., 0.], false).unwrap(), 0)
            .unwrap()[0];
        let blended = texture::pixels(&canopy_color(&source, [0.5, 2., 1., 0.], true).unwrap(), 0)
            .unwrap()[0];
        for (actual, expected) in masked[..3].iter().zip([
            (128. / 255. * 0.5_f32).powi(2),
            (64. / 255. * 2_f32).powi(2),
            (32. / 255_f32).powi(2),
        ]) {
            assert!((actual - expected).abs() < 0.0003);
        }
        assert!((masked[3] - 80. / 255.).abs() < 0.0003);
        assert!((blended[3] - 160. / 255.).abs() < 0.0003);
    }

    #[test]
    fn cornea_mask_is_red_coverage_and_never_diffuse_color() {
        let prepared = cornea_envelope(
            &image([64, 200, 12, 255]),
            [[0.01, 0., 1.], [0.002, -2.88, 1.]],
        )
        .unwrap();
        let pixel = texture::pixels(&prepared, 0).unwrap()[0];
        assert_eq!(&pixel[..3], &[1., 1., 1.]);
        assert!((pixel[3] - (64. / 255_f32).sqrt()).abs() < 0.0003);
    }

    #[test]
    fn optic_gloss_comes_from_separate_red_channel() {
        let prepared = combine_specular(
            &image([30, 60, 120, 200]),
            &image([64, 240, 220, 190]),
            [20, 20],
        )
        .unwrap();
        let pixel = texture::pixels(&prepared, 0).unwrap()[0];
        for (actual, expected) in pixel.into_iter().zip([30., 60., 120., 64.]) {
            assert!((actual - expected / 255.).abs() < 0.0003);
        }
    }

    #[test]
    fn cornea_highlights_use_half_vector_power_fill_facing_and_combined_mask() {
        let controls = [1., 0., 0.8];
        assert!((cornea_highlight(0.72, 1., controls) - 0.4).abs() < 0.00001);
        assert!((cornea_highlight(0.72, -0.1, controls) - 0.2).abs() < 0.00001);
        assert_eq!(cornea_highlight(0.72, -0.2, controls), 0.);
        assert!((cornea_coverage(0.25, [0.4, 0.2]) - (0.55_f32 * 0.25).sqrt()).abs() < 0.00001);
        assert_eq!(cornea_coverage(1., [0.02, 0.02]), 0.);
    }

    #[test]
    fn optic_facing_decay_keeps_native_smoothstep_and_clamping() {
        assert_eq!(optic_decay(1., 0.2), 0.);
        assert!((optic_decay(0.9, 0.2) - 0.5).abs() < 0.00001);
        assert_eq!(optic_decay(0.8, 0.2), 1.);
        assert_eq!(optic_decay(-0.5, 0.2), 1.);
    }
}

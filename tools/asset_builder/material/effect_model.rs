use super::*;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum EffectModelRecipe {
    ProjectedBlood,
    ElectricDisc,
}

pub(super) fn lookup(family: &str) -> Option<EffectModelRecipe> {
    match family {
        "mc_projecteddecal_27zq5445" => Some(EffectModelRecipe::ProjectedBlood),
        "mc_sw4_3d_zm_elec_disc_w323fq8j" => Some(EffectModelRecipe::ElectricDisc),
        _ => None,
    }
}

impl EffectModelRecipe {
    pub(super) fn index(self) -> usize {
        if self == Self::ProjectedBlood { 4 } else { 3 }
    }

    fn textures(self) -> Vec<(u32, u64, u8)> {
        match self {
            Self::ProjectedBlood => vec![(2695565377, 0, 244)],
            Self::ElectricDisc => vec![
                (3897646725, 512, 20),
                (3054311507, 2049, 2),
                (2981422039, 1282, 2),
                (2695565377, 1795, 66),
                (170923560, 1540, 66),
                (1345510171, 1029, 226),
                (1345510170, 774, 2),
            ],
        }
    }

    fn constants(self) -> Vec<(u32, u64, u64)> {
        if self == Self::ProjectedBlood {
            return Vec::new();
        }
        vec![
            (14836402, 96, 4),
            (42483651, 100, 4),
            (68285721, 104, 4),
            (116600957, 176, 16),
            (1260895079, 152, 4),
            (1416012899, 232, 4),
            (1426306028, 80, 8),
            (1426769011, 88, 8),
            (1663396853, 208, 4),
            (2010068229, 120, 8),
            (2010882330, 112, 8),
            (2069895088, 160, 8),
            (2562820107, 236, 4),
            (3371644427, 128, 4),
            (3543472193, 200, 8),
            (3544024158, 192, 8),
            (3829158408, 212, 8),
            (3829629591, 224, 8),
            (3882028418, 248, 8),
            (4084488332, 132, 8),
            (4084951315, 144, 8),
            (4206754060, 240, 4),
            (4206754322, 244, 4),
        ]
    }

    pub(super) fn validate(self, key: &str, native: &Value) -> Result<()> {
        if lookup(
            native["technique_set"]
                .as_str()
                .ok_or("Missing effect-model family")?,
        ) != Some(self)
        {
            return Err(format!("Effect-model recipe does not match {key}").into());
        }
        let indices: &[usize] = if self == Self::ProjectedBlood {
            &[4, 6]
        } else {
            &[3]
        };
        for &index in indices {
            let pass = contract::pass(native, index)?;
            let (vertex, pixel) = match (self, index) {
                (Self::ProjectedBlood, 4) => (
                    "pimp_shader_projecteddecal_6532f59b.hlsl",
                    "pimp_shader_projecteddecal_6532f59b.hlsl",
                ),
                (Self::ProjectedBlood, 6) => (
                    "pimp_shader_projecteddecal_fe57c81c.hlsl",
                    "pimp_shader_projecteddecal_f3b76af3.hlsl",
                ),
                (Self::ElectricDisc, 3) => (
                    "pimp_shader_sw4_3d_zmb_sky_pillar_on_3ba124.hlsl",
                    "pimp_shader_sw4_3d_zm_elec_disc_985861f1.hlsl",
                ),
                _ => unreachable!(),
            };
            let routes = if self == Self::ProjectedBlood {
                json!([{"source":0,"destination":0}])
            } else {
                json!([{"source":0,"destination":0},{"source":1,"destination":2},{"source":2,"destination":5},{"source":3,"destination":1}])
            };
            if pass["vertex_shader"] != vertex
                || pass["pixel_shader"] != pixel
                || pass["vertex_routes"] != routes
            {
                return Err(format!(
                    "Effect-model material {key} has unreviewed programs or vertex routes"
                )
                .into());
            }
            let arguments = pass["arguments"]
                .as_array()
                .ok_or("Missing effect-model arguments")?;
            let textures = self.textures();
            if arguments.iter().filter(|a| a["type"] == 2).count() != textures.len() {
                return Err(
                    format!("Effect-model material {key} has unexpected active images").into(),
                );
            }
            for (hash, location, sampler) in textures {
                if arguments
                    .iter()
                    .filter(|a| contract::argument(a, [2, hash as u64, 0, location, 1]))
                    .count()
                    != 1
                {
                    return Err(
                        format!("Effect-model material {key} lacks active image {hash}").into(),
                    );
                }
                let binding = contract::texture_binding(native, hash)?;
                if binding["sampler"] != sampler
                    || binding["image"].as_str().is_none_or(str::is_empty)
                {
                    return Err(format!(
                        "Effect-model material {key} has an unreviewed sampler for {hash}"
                    )
                    .into());
                }
            }
            let constants = self.constants();
            if arguments
                .iter()
                .filter(|a| matches!(a["type"].as_u64(), Some(0 | 1 | 6 | 7 | 8)))
                .count()
                != constants.len()
            {
                return Err(
                    format!("Effect-model material {key} has unexpected active constants").into(),
                );
            }
            for (hash, location, size) in constants {
                if arguments
                    .iter()
                    .filter(|a| contract::argument(a, [6, hash as u64, 2, location, size]))
                    .count()
                    != 1
                {
                    return Err(format!(
                        "Effect-model material {key} lacks native constant {hash}"
                    )
                    .into());
                }
                contract::literal(native, hash)?;
            }
            if !arguments.iter().any(|a| a["type"] == 4 && a["value"] == 18) {
                return Err(
                    format!("Effect-model material {key} lacks its source depth input").into(),
                );
            }
            if self == Self::ElectricDisc
                && !arguments
                    .iter()
                    .any(|a| contract::argument(a, [5, 16777241, 2, 64, 16]))
            {
                return Err(format!("Electric-disc material {key} lacks native GameTime").into());
            }
            let state = native["state_entries"][index]
                .as_u64()
                .ok_or("Missing effect-model state index")? as usize;
            let expected = if self == Self::ProjectedBlood {
                json!([422068581_u32, 28])
            } else {
                json!([421611810_u32, 12])
            };
            if native["state_bits"][state] != expected {
                return Err(
                    format!("Effect-model material {key} has an unreviewed render state").into(),
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
        let mut inputs = Vec::new();
        for (hash, _, sampler) in self.textures().into_iter().filter(|(hash, _, _)| *hash == 2695565377) {
            let binding = contract::texture_binding(native, hash)?;
            let name = binding["image"].as_str().unwrap().to_ascii_lowercase();
            let image = images
                .get(&name)
                .ok_or_else(|| format!("Required effect-model image {name} is absent"))?;
            if image.depth != 1 || image.faces != 1 || image.levels.is_empty() {
                return Err(format!("Required effect-model image {name} is not 2D").into());
            }
            let identity = (name.clone(), "effect-model".to_owned());
            let uri = if let Some(uri) = shared.get(&identity) {
                uri.clone()
            } else {
                let color = ordinary::bake(image, "color", None)?;
                let uri = store_image(
                    &color,
                    format!(
                        "prepared/material-channels/{}/effect-model.ktx2",
                        shared.len()
                    ),
                    true,
                    "color",
                    pack,
                )?;
                shared.insert(identity, uri.clone());
                uri
            };
            inputs.push(json!({"uri":uri,"role":"color","layer":0,"color_space":"srgb","sampler":sampler}));
        }
        let projected = self == Self::ProjectedBlood;
        Ok(
            json!({"name":key,"pbrMetallicRoughness":{"baseColorFactor":[1.,1.,1.,1.],"metallicFactor":0.},
            "extras":{"surface":{"version":3,"identity":key,"native":native,"channels":[inputs[0]],"standard":{"roughness":1.,"metallic":0.,"reflectance":0.,"unlit":!projected},
            "presentation":{"alpha":if projected {"blend"} else {"add"},"cull":"none","fog":false,"visible":true},
            "source_effect_model":{"kind":if projected {"projected_blood"} else {"electric_disc"},"consumer":"standard_material"},
            "approximations":if projected {json!(["Source blood color uses upstream forward decal and PBR lighting; native projection edge and gamma2 framebuffer composition are approximated"])} else {json!(["Source electric-disc color uses stock additive material; native distortion, independent layered animation and depth softness are omitted"])}}}}),
        )
    }
}

use super::*;

pub(super) struct Input {
    pub(super) transform: [f32; 6],
    pub(super) width: usize,
    pub(super) height: usize,
    pub(super) sampler: u8,
    pub(super) pixels: Vec<[f32; 4]>,
}

impl Input {
    pub(super) fn read(image: &texture::Image, sampler: u8) -> Result<Self> {
        if image.faces != 1 || image.depth != 1 {
            return Err("Layered material input is not a 2D image".into());
        }
        Ok(Self {
            transform: [1., 0., 0., 1., 0., 0.],
            width: image.width,
            height: image.height,
            sampler,
            pixels: texture::pixels(image, 0)?,
        })
    }

    pub(super) fn sample(&self, uv: [f32; 2]) -> [f32; 4] {
        texture::sample(
            &self.pixels,
            self.width,
            self.height,
            self.uv(uv),
            self.sampler,
        )
    }

    pub(super) fn uv(&self, uv: [f32; 2]) -> [f32; 2] {
        let [xx, xy, yx, yy, x, y] = self.transform;
        [xx * uv[0] + xy * uv[1] + x, yx * uv[0] + yy * uv[1] + y]
    }
}

pub(super) struct Recipe {
    pub(super) alpha_reveal: bool,
    pub(super) special: u8,
    pub(super) special_parms: [[f32; 4]; 3],
    pub(super) inputs: Vec<Input>,
    pub(super) base_specular: bool,
    pub(super) geometric_normal: bool,
    pub(super) second_additive: bool,
    pub(super) second_normal: u8,
    pub(super) second_reveal: bool,
    pub(super) second_specular: bool,
    pub(super) second_parms: [f32; 4],
    pub(super) third_multiply: bool,
    pub(super) layer_cutout: bool,
    pub(super) decal: bool,
    pub(super) multiply: bool,
    pub(super) additive: bool,
    pub(super) blend_color: bool,
    pub(super) reveal: bool,
    pub(super) layer_specular: bool,
    pub(super) base_alpha_response: bool,
    pub(super) parms: [f32; 4],
}

impl Recipe {
    pub(super) fn gpu_data(&self) -> Result<(Vec<u8>, Vec<u8>)> {
        if self.inputs.is_empty() || self.inputs.len() > 9 {
            return Err("Layered GPU recipe must have one to nine source images".into());
        }
        let flags = self.decal as u32
            | (self.multiply as u32) << 1
            | (self.additive as u32) << 2
            | (self.blend_color as u32) << 3
            | (self.reveal as u32) << 4
            | (self.layer_specular as u32) << 5
            | (self.base_alpha_response as u32) << 6
            | (self.base_specular as u32) << 7
            | (self.geometric_normal as u32) << 8
            | (self.second_additive as u32) << 9
            | (u32::from(self.second_normal == 1)) << 10
            | (u32::from(self.second_normal == 2)) << 11
            | (self.second_reveal as u32) << 12
            | (self.second_specular as u32) << 13
            | (self.third_multiply as u32) << 14
            | (self.layer_cutout as u32) << 15
            | u32::from(self.special) << 16
            | (self.alpha_reveal as u32) << 23;
        let mut parameters = Vec::new();
        for value in [
            flags,
            0,
            self.inputs.len() as u32,
            self.inputs.len() as u32 - 1,
        ] {
            push32(&mut parameters, value);
        }
        for value in self
            .parms
            .into_iter()
            .chain(self.second_parms)
            .chain(self.special_parms.into_iter().flatten())
        {
            push32(&mut parameters, value.to_bits());
        }
        for index in [
            self.layer_specular_index(),
            self.second_specular_index(),
            self.second_normal_index(),
            self.second_color_index(),
        ] {
            push32(&mut parameters, index as u32);
        }
        let mut image_data = Vec::new();
        for index in 0..9 {
            let metadata = if let Some(input) = self.inputs.get(index) {
                let offset = u32::try_from(image_data.len() / 16)?;
                for pixel in &input.pixels {
                    for value in pixel {
                        push32(&mut image_data, value.to_bits());
                    }
                }
                [
                    input.width as u32,
                    input.height as u32,
                    offset,
                    input.sampler as u32,
                ]
            } else {
                [1, 1, 0, 20]
            };
            for value in metadata {
                push32(&mut parameters, value);
            }
        }
        for index in 0..9 {
            let transform = self
                .inputs
                .get(index)
                .map_or([1., 0., 0., 1., 0., 0.], |input| input.transform);
            for value in transform.into_iter().chain([0., 0.]) {
                push32(&mut parameters, value.to_bits());
            }
        }

        Ok((parameters, image_data))
    }

    pub(super) fn color_operations(&self) -> bool {
        self.multiply || self.additive || self.blend_color || self.geometric_normal
    }
    pub(super) fn color_only_layer(&self) -> bool {
        self.color_operations() && self.second_normal == 0
    }

    pub(super) fn auxiliary(
        &self,
        surface: &Surface,
        index: usize,
    ) -> Result<([f32; 2], [f32; 2], [f32; 4], [f32; 4], [f32; 2])> {
        if self.special == 127 {
            let uv = surface.uv[index];
            let color = surface.colors[index].map(|value| value as f32 / 255.);
            return Ok((uv, uv, color, color, uv));
        }
        if self.alpha_reveal {
            let uv = *surface
                .uv
                .get(index)
                .ok_or("Alpha reveal source UV is missing")?;
            return Ok((uv, uv, [1., 0.5, 0.5, 1.], [1., 0.5, 0.5, 1.], uv));
        }
        if self.third_multiply {
            let bytes = surface
                .auxiliary
                .get(index)
                .ok_or("Third layered world UV is missing")?;
            if bytes.len() != 16 {
                return Err("Three-color layered vertex requires sixteen auxiliary bytes".into());
            }
            let uv = |offset| {
                [
                    half::f16::from_bits(u16_at(bytes, offset)).to_f32(),
                    half::f16::from_bits(u16_at(bytes, offset + 2)).to_f32(),
                ]
            };
            let (uv1, uv2, uv3) = (uv(0), uv(4), uv(8));
            if uv1
                .into_iter()
                .chain(uv2)
                .chain(uv3)
                .any(|value| !value.is_finite())
            {
                return Err("Third layered world UV is not finite".into());
            }
            let frame = std::array::from_fn(|i| bytes[12 + i] as f32 / 255.);
            return Ok((uv1, uv2, frame, frame, uv3));
        }
        if self.second_normal != 2 {
            let (uv1, uv2, frame) = auxiliary(surface, index, self.decal, self.color_only_layer())?;
            return Ok((uv1, uv2, frame, frame, [0.; 2]));
        }
        let bytes = surface
            .auxiliary
            .get(index)
            .ok_or("Two-normal layered world vertex has no auxiliary data")?;
        if bytes.len() != 16 {
            return Err("Two-normal layered vertex requires sixteen auxiliary bytes".into());
        }
        let uv = |offset| {
            [
                half::f16::from_bits(u16_at(bytes, offset)).to_f32(),
                half::f16::from_bits(u16_at(bytes, offset + 2)).to_f32(),
            ]
        };
        let (uv1, uv2) = (uv(0), uv(4));
        if uv1.into_iter().chain(uv2).any(|value| !value.is_finite()) {
            return Err("Two-normal layered UV is not finite".into());
        }
        Ok((
            uv1,
            uv2,
            std::array::from_fn(|i| bytes[8 + i] as f32 / 255.),
            std::array::from_fn(|i| bytes[12 + i] as f32 / 255.),
            [0.; 2],
        ))
    }

    pub(super) fn second_color_index(&self) -> usize {
        if self.color_operations() { 3 } else { 4 }
    }
    pub(super) fn second_normal_index(&self) -> usize {
        self.second_color_index() + 1
    }
    pub(super) fn layer_specular_index(&self) -> usize {
        4 + usize::from(self.decal) + usize::from(self.second_normal != 0)
    }
    pub(super) fn second_specular_index(&self) -> usize {
        if self.color_operations() {
            5
        } else {
            self.layer_specular_index() + usize::from(self.layer_specular)
        }
    }
    pub(super) fn has_specular(&self) -> bool {
        self.base_specular || self.layer_specular || self.second_specular
    }

    pub(super) fn fritz_inputs(&self, uv: [f32; 2]) -> [[f32; 4]; 3] {
        let base = self.inputs[0].sample(uv);
        let mut color = base;
        let mut specular = self.inputs[2].sample(uv);
        let mut glow = self.inputs[3].sample(uv);
        let state = self.special_parms[1][0];
        if state > 0. {
            let warp = self.inputs[4].sample(uv.map(|value| value * self.second_parms[3]));
            let offset: [f32; 2] =
                std::array::from_fn(|axis| (warp[axis] * 2. - 1.) * self.second_parms[2]);
            let warped_uv = std::array::from_fn(|axis| uv[axis] + offset[axis]);
            let warped = self.inputs[0].sample(warped_uv);
            let luminance = warped[0] * 0.3 + warped[1] * 0.59 + warped[2] * 0.11;
            let luma = luminance * luminance * self.special_parms[0][0];
            color = std::array::from_fn(|axis| {
                if axis == 3 {
                    warped[3]
                } else {
                    base[axis] + state * (luma - base[axis])
                }
            });
            glow = self.inputs[3].sample(warped_uv);
            let sparkle_uv = std::array::from_fn(|axis| {
                uv[axis] * self.parms[3]
                    + (self.special_parms[2][3] * self.second_parms[axis] * self.parms[3])
                        .rem_euclid(1.)
                    + offset[axis]
            });
            let sparkle = self.inputs[5].sample(sparkle_uv);
            let vertical = uv[1] * uv[1] * uv[1] * sparkle[3];
            for axis in 0..3 {
                let target = luma + vertical * (sparkle[axis] - luma);
                glow[axis] = base[axis] + state * (target - base[axis]);
            }
            let warped = self.inputs[2].sample(warped_uv);
            let luminance = warped[0] * 0.3 + warped[1] * 0.59 + warped[2] * 0.11;
            for axis in 0..3 {
                specular[axis] += state * (luminance - specular[axis]);
            }
        }
        [color, specular, glow]
    }

    pub(super) fn emission_pixel(&self, uv: [f32; 2], weight: f32) -> [f32; 4] {
        if self.special == 2 {
            return std::array::from_fn(|channel| {
                if channel == 3 {
                    1.
                } else {
                    (self.second_parms[channel] * self.second_parms[3]).powi(2) * weight
                }
            });
        }
        let mut source = if self.special == 16 {
            self.fritz_inputs(uv)[2]
        } else {
            self.inputs[if self.special == 8 { 2 } else { 3 }].sample(uv)
        };
        if self.special == 64 {
            for channel in 0..4 {
                source[channel] *= self.special_parms[1][channel];
            }
        }
        std::array::from_fn(|channel| {
            if channel == 3 {
                1.
            } else {
                (source[channel] * source[3]).powi(2) * weight
            }
        })
    }

    #[cfg(test)]
    pub(super) fn sample_special(&self, uv: [f32; 2]) -> Result<[[f32; 4]; 4]> {
        let sample = |index: usize, uv| self.inputs[index].sample(uv);
        let decode = |value: f32| value * f32::from_bits(0x40808102) + f32::from_bits(0xc0010204);
        let normal = |slope: [f32; 2]| {
            let length = (slope[0] * slope[0] + slope[1] * slope[1] + 1.).sqrt();
            [slope[0] / length, slope[1] / length, 1. / length, 1.]
        };
        let fit = |color: [f32; 3], normal, rgb: [f32; 3], gloss, emission| {
            let (color, metallic, roughness) = crate::material::pbr_pixel(color, rgb, gloss);
            [
                [color[0], color[1], color[2], 1.],
                normal,
                [1., roughness, metallic, emission],
                [rgb[0], rgb[1], rgb[2], gloss],
            ]
        };
        if self.special == 16 {
            let [color, response, _] = self.fritz_inputs(uv);
            let n = sample(1, uv);
            let intensity = self.parms[2] * self.special_parms[1][1];
            let mut pixel = fit(
                std::array::from_fn(|axis| color[axis] * color[axis]),
                normal([decode(n[0]), decode(n[1])]),
                std::array::from_fn(|axis| response[axis] * response[axis]),
                response[3],
                intensity * intensity,
            );
            if self.special_parms[0][1] != 0. {
                pixel[0][3] = color[3];
            }
            return Ok(pixel);
        }
        if matches!(self.special, 4 | 8 | 32 | 64) {
            let base = sample(0, uv);
            let n = sample(1, uv);
            let color = std::array::from_fn(|axis| base[axis] * base[axis]);
            let normal = normal([decode(n[0]), decode(n[1])]);
            if self.special == 8 {
                return Ok(fit(
                    color,
                    normal,
                    [0.04; 3],
                    base[3],
                    self.parms[0] * self.parms[0],
                ));
            }
            if self.special == 32 {
                let response = sample(2, uv);
                let intensity = if self.special_parms[1][2] >= self.second_parms[2] {
                    self.parms[3]
                } else {
                    0.
                };
                let mut pixel = fit(
                    color,
                    normal,
                    std::array::from_fn(|axis| response[axis] * response[axis]),
                    response[3],
                    intensity * intensity,
                );
                if self.second_parms[3] != 0. {
                    pixel[0][3] = base[3];
                }
                return Ok(pixel);
            }
            let response = sample(2, uv);
            let rgb = std::array::from_fn(|axis| (response[axis] * self.parms[0]).powi(2));
            let emission = if self.special == 64 {
                self.parms[3]
            } else {
                self.parms[2] * self.parms[2] * self.parms[3]
            };
            let mut pixel = fit(
                color,
                normal,
                rgb,
                response[3] * self.parms[1],
                emission * emission,
            );
            if self.special == 4 && self.special_parms[0][0] != 0. {
                pixel[0][3] = base[3];
            }
            return Ok(pixel);
        }
        if self.special == 1 {
            let first_uv = std::array::from_fn(|axis| uv[axis] * self.parms[axis]);
            let second_uv = std::array::from_fn(|axis| uv[axis] * self.parms[axis + 2]);
            let mask = sample(0, uv);
            let weight = mask[1] * mask[1];
            let base = sample(1, uv);
            let first = sample(2, first_uv);
            let second = sample(3, second_uv);
            let slope = std::array::from_fn(|axis| {
                decode(base[axis]) * self.second_parms[1]
                    + decode(first[axis]) * ((1. - weight) * self.second_parms[0])
                    + decode(second[axis]) * (weight * self.second_parms[3])
            });
            let first = sample(4, first_uv);
            let second = sample(5, second_uv);
            let color = std::array::from_fn(|axis| {
                (first[axis] + weight * (second[axis] - first[axis])).powi(2)
            });
            let normal = normal(slope);
            if self.base_specular {
                let first = sample(6, first_uv);
                let second = sample(7, second_uv);
                let response: [f32; 4] =
                    std::array::from_fn(|axis| first[axis] + weight * (second[axis] - first[axis]));
                return Ok(fit(
                    color,
                    normal,
                    std::array::from_fn(|axis| response[axis + 1].powi(2)),
                    response[0],
                    1.,
                ));
            }
            return Ok([
                [color[0], color[1], color[2], 1.],
                normal,
                [1., 1., 0., 1.],
                [0.; 4],
            ]);
        }
        if uv.iter().any(|value| *value < 0.) {
            return Err("Water reveal source logarithm has a negative coordinate".into());
        }
        let time = self.special_parms[2][3];
        let motion = (time * f32::from_bits(0xbe116d3f)).rem_euclid(1.);
        let warp = sample(
            2,
            [
                uv[0] * f32::from_bits(0x4035c88e) * f32::from_bits(0x3f7f41f2),
                (uv[1] * f32::from_bits(0x4035c88e) + motion) * f32::from_bits(0xbdb851ec),
            ],
        );
        let power = warp[0] * 0.1 + 1.;
        let water_uv: [f32; 2] =
            std::array::from_fn(|axis| (uv[axis].powf(power) - 0.5) / self.parms[axis + 2] + 0.5);
        let scroll = time * self.parms[0];
        let first = sample(
            3,
            [
                water_uv[0] - scroll * f32::from_bits(0xbc11fcab),
                water_uv[1] - scroll * f32::from_bits(0x3f4cc98c),
            ],
        );
        let second = sample(
            3,
            [
                water_uv[0] - (scroll * f32::from_bits(0xbc6d3a96) + 0.5),
                water_uv[1] - (scroll * f32::from_bits(0x3fa663c2) + 0.5),
            ],
        );
        let sum: [f32; 2] = std::array::from_fn(|axis| decode(first[axis]) + decode(second[axis]));
        let length = (sum[0] * sum[0] + sum[1] * sum[1] + 4.).sqrt();
        let z = 2. / length;
        let water = [sum[0] / length / z, sum[1] / length / z, 1.];
        let positive = (water[0] + water[1]) * 0.5 > 0.;
        let response = sample(5, uv);
        let temporal = sample(0, uv)[0];
        let state = self.special_parms[1][2];
        let mut reveal = [
            ((temporal - (state + f32::from_bits(0x3d75c28f))) * f32::from_bits(0xc1055555))
                .clamp(0., 1.),
            ((temporal - (state + f32::from_bits(0xbcf5c28f))) * f32::from_bits(0x41855555))
                .clamp(0., 1.),
        ];
        reveal = reveal.map(|value| value * value * (3. - 2. * value));
        let bias = f32::from_bits(if positive { 0x3e5b6dca } else { 0x3e92491b });
        let rgb = std::array::from_fn(|axis| {
            let wet = response[axis] + bias;
            (wet + reveal[0] * (response[axis] - wet)).powi(2)
        });
        let wet_gloss = if positive { response[3] } else { 1. };
        let gloss = wet_gloss + reveal[0] * (response[3] - wet_gloss);
        let base = sample(1, uv);
        let slope = std::array::from_fn(|axis| {
            let base = decode(base[axis]);
            let wet = base * water[2] + water[axis] * self.parms[1];
            wet + reveal[0] * (base - wet)
        });
        let warped = sample(
            4,
            std::array::from_fn(|axis| (uv[axis] - water[axis] * water[2]).clamp(0., 1.)),
        );
        let base = sample(4, uv)[0];
        let tint = [0x3f3bc148, 0x3f49c91d, 0x3f649247].map(f32::from_bits);
        let color = std::array::from_fn(|axis| {
            let wet = warped[axis] * tint[axis];
            (wet + reveal[0] * (self.special_parms[0][axis] * base - wet)).powi(2)
        });
        Ok(fit(
            color,
            normal(slope),
            rgb,
            gloss,
            (reveal[0] * reveal[1]).powi(2),
        ))
    }

    pub(super) fn input_uv(&self, index: usize) -> usize {
        if self.alpha_reveal || self.special == 127 {
            0
        } else if self.third_multiply && index + 1 == self.inputs.len() {
            3
        } else if index == 0
            || !self.geometric_normal && index == 1
            || self.base_specular && index + 1 == self.inputs.len()
        {
            0
        } else if self.second_normal != 0 && index == self.second_normal_index()
            || self.second_specular && index == self.second_specular_index()
            || self.decal && index == self.second_color_index()
        {
            2
        } else {
            1
        }
    }

    #[cfg(test)]
    pub(super) fn reveal_weight(alpha: f32, vertex: f32, parms: [f32; 4]) -> Result<f32> {
        let a = vertex * f32::from_bits(0x3f7f7cee) + f32::from_bits(0x3a83126f);
        let q = 1. - a;
        let low = (q - parms[0] * (a.log2() * parms[1]).exp2()).clamp(0., 1.);
        let high = (q + parms[0] * (q.log2() * parms[1]).exp2()).clamp(0., 1.);
        if high <= low {
            return Err("Layered reveal thresholds have no interval".into());
        }
        Ok(((alpha - low) / (high - low)).clamp(0., 1.))
    }

    #[cfg(test)]
    pub(super) fn sample(
        &self,
        uv0: [f32; 2],
        uv1: [f32; 2],
        uv2: [f32; 2],
        green: f32,
        blue: f32,
        transform: [f32; 4],
    ) -> Result<[[f32; 4]; 4]> {
        self.sample_frames(uv0, uv1, uv2, green, blue, transform, transform, [0.; 4])
    }

    #[cfg(test)]
    pub(super) fn sample_frames(
        &self,
        uv0: [f32; 2],
        uv1: [f32; 2],
        uv2: [f32; 2],
        green: f32,
        blue: f32,
        transform: [f32; 4],
        second_transform: [f32; 4],
        third: [f32; 4],
    ) -> Result<[[f32; 4]; 4]> {
        if self.alpha_reveal {
            let base = self.inputs[0].sample(uv0);
            let sampled = self.inputs[1].sample(uv0);
            let slope = [sampled[0], sampled[1]]
                .map(|value| value * f32::from_bits(0x40808102) + f32::from_bits(0xc0010204));
            let length = (slope[0] * slope[0] + slope[1] * slope[1] + 1.).sqrt();
            let mut color = [
                base[0] * base[0],
                base[1] * base[1],
                base[2] * base[2],
                Self::reveal_weight(base[3], third[2], self.parms)?,
            ];
            let mut response = [1., 1., 0., 1.];
            let mut specular = [0.; 4];
            if self.base_specular {
                let source = self.inputs[2].sample(uv0);
                let rgb = [
                    source[0] * source[0],
                    source[1] * source[1],
                    source[2] * source[2],
                ];
                let (base, metallic, roughness) =
                    crate::material::pbr_pixel([color[0], color[1], color[2]], rgb, source[3]);
                color[..3].copy_from_slice(&base);
                response = [1., roughness, metallic, 1.];
                specular = [rgb[0], rgb[1], rgb[2], source[3]];
            }
            return Ok([
                color,
                [slope[0] / length, slope[1] / length, 1. / length, 1.],
                response,
                specular,
            ]);
        }
        if self.special != 0 {
            return self.sample_special(uv0);
        }
        let inputs = &self.inputs;
        if self.color_operations() {
            let base = inputs[0].sample(uv0);
            let layer = inputs[if self.geometric_normal { 1 } else { 2 }].sample(uv1);
            let second = if self.decal {
                inputs[self.second_color_index()].sample(uv2)
            } else {
                [1.; 4]
            };
            let second_weight = if self.second_reveal {
                Self::reveal_weight(second[3], blue, self.second_parms)?
            } else {
                blue * second[3]
            };
            let tint: [f32; 3] = std::array::from_fn(|c| 1. + green * (layer[c] - 1.));
            let mut color = std::array::from_fn(|c| {
                if c == 3 {
                    1.
                } else {
                    if self.blend_color {
                        let first = base[c] + green * layer[3] * (layer[c] - base[c]);
                        let weight = if self.decal { second_weight } else { 0. };
                        (first + weight * (second[c] - first)).powi(2)
                    } else if self.additive {
                        ((base[c] + layer[c] * green * layer[3]) * (1. + blue * (second[c] - 1.)))
                            .powi(2)
                    } else {
                        (base[c] * tint[c] * (1. + blue * (second[c] - 1.))).powi(2)
                    }
                }
            });
            let mut response = if self.base_alpha_response {
                let second_tint: [f32; 3] = std::array::from_fn(|c| 1. + blue * (second[c] - 1.));
                let gloss = if self.additive && self.decal {
                    base[3] * (second_tint[0] * 0.3 + second_tint[1] * 0.6 + second_tint[2] * 0.1)
                } else if self.additive || self.blend_color {
                    base[3]
                } else {
                    base[3] * (tint[0] * 0.3 + tint[1] * 0.6 + tint[2] * 0.1)
                };
                [
                    1.,
                    (2. / (2_f32.powf(13. * gloss) + 2.)).powf(0.25),
                    0.,
                    gloss,
                ]
            } else {
                [1., 1., 0., 1.]
            };
            let mut specular = [0.; 4];
            if self.has_specular() {
                let k = f32::from_bits(0x3e4ccccd);
                let mut source = if self.base_specular {
                    inputs.last().unwrap().sample(uv0)
                } else {
                    [k, k, k, base[3]]
                };
                if self.second_specular {
                    let second = inputs[self.second_specular_index()].sample(uv2);
                    source = std::array::from_fn(|c| {
                        source[c] + second_weight * (second[c] - source[c])
                    });
                }
                let rgb = std::array::from_fn(|c| source[c] * source[c]);
                let (base, metallic, roughness) =
                    crate::material::pbr_pixel([color[0], color[1], color[2]], rgb, source[3]);
                color = [base[0], base[1], base[2], 1.];
                response = [1., roughness, metallic, 1.];
                specular = [rgb[0], rgb[1], rgb[2], source[3]];
            }
            if self.geometric_normal {
                return Ok([color, [0., 0., 1., 1.], response, specular]);
            }
            let normal = inputs[1].sample(uv0);
            let decode =
                |value: f32| value * f32::from_bits(0x40808102) + f32::from_bits(0xc0010204);
            let mut slope = [normal[0], normal[1]].map(decode);
            if self.second_normal == 1 {
                let second_normal = inputs[self.second_normal_index()].sample(uv2);
                let s = [decode(second_normal[0]), decode(second_normal[1])];
                let rotated = [
                    s[0] * transform[0] + s[1] * transform[1],
                    s[0] * transform[2] + s[1] * transform[3],
                ];
                let weight = second_weight;
                slope = std::array::from_fn(|axis| {
                    slope[axis] + weight * (rotated[axis] - slope[axis])
                });
            }
            let length = (slope[0] * slope[0] + slope[1] * slope[1] + 1.).sqrt();
            return Ok([
                color,
                [slope[0] / length, slope[1] / length, 1. / length, 1.],
                response,
                specular,
            ]);
        }
        let (c0, c1, n0, n1) = (
            inputs[0].sample(uv0),
            inputs[2].sample(uv1),
            inputs[1].sample(uv0),
            inputs[3].sample(uv1),
        );
        let weight = if self.reveal {
            Self::reveal_weight(c1[3], green, self.parms)?
        } else if self.layer_cutout {
            if c1[3] * green >= 0.5 { 1. } else { 0. }
        } else {
            c1[3] * green
        };
        let decal = if self.decal {
            inputs[self.second_color_index()].sample(uv2)
        } else {
            [1.; 4]
        };
        let second_weight = if self.second_reveal {
            Self::reveal_weight(decal[3], blue, self.second_parms)?
        } else {
            blue * decal[3]
        };
        let third_color = if self.third_multiply {
            inputs.last().unwrap().sample([third[0], third[1]])
        } else {
            [1.; 4]
        };
        let third_tint: [f32; 3] = std::array::from_fn(|c| 1. + third[2] * (third_color[c] - 1.));
        let mut color = [1.; 4];
        if self.layer_cutout && weight == 1. {
            color[3] = c1[3];
        }
        for c in 0..3 {
            let first = c0[c] + weight * (c1[c] - c0[c]);
            let encoded = if self.second_additive {
                first + blue * decal[3] * decal[c]
            } else if self.second_normal != 0 {
                first + second_weight * (decal[c] - first)
            } else {
                first * (1. + blue * (decal[c] - 1.))
            };
            color[c] = (encoded * third_tint[c]).powi(2);
        }
        let decode = |value: f32| value * f32::from_bits(0x40808102) + f32::from_bits(0xc0010204);
        let s0 = [decode(n0[0]), decode(n0[1])];
        let s1 = [decode(n1[0]), decode(n1[1])];
        let rotated = [
            s1[0] * transform[0] + s1[1] * transform[1],
            s1[0] * transform[2] + s1[1] * transform[3],
        ];
        let mut slope = [
            s0[0] + weight * (rotated[0] - s0[0]),
            s0[1] + weight * (rotated[1] - s0[1]),
        ];
        if self.second_normal == 2 {
            let second = inputs[self.second_normal_index()].sample(uv2);
            let s = [decode(second[0]), decode(second[1])];
            let rotated = [
                s[0] * second_transform[0] + s[1] * second_transform[1],
                s[0] * second_transform[2] + s[1] * second_transform[3],
            ];
            let weight = second_weight;
            slope =
                std::array::from_fn(|axis| slope[axis] + weight * (rotated[axis] - slope[axis]));
        }
        let length = (slope[0] * slope[0] + slope[1] * slope[1] + 1.).sqrt();
        let normal = [slope[0] / length, slope[1] / length, 1. / length, 1.];
        let (mut response, mut specular) = ([1., 1., 0., 1.], [0.; 4]);
        if self.has_specular() {
            let k = f32::from_bits(0x3e4ccccd);
            let base_gloss = if self.base_alpha_response { c0[3] } else { 0. };
            let base = if self.base_specular {
                inputs.last().unwrap().sample(uv0)
            } else {
                [k, k, k, base_gloss]
            };
            let mut mixed = if self.layer_specular {
                let layer = inputs[self.layer_specular_index()].sample(uv1);
                std::array::from_fn(|c| base[c] + weight * (layer[c] - base[c]))
            } else {
                base
            };
            if self.second_specular {
                let second = inputs[self.second_specular_index()].sample(uv2);
                mixed = std::array::from_fn(|c| mixed[c] + second_weight * (second[c] - mixed[c]));
            }
            let rgb = std::array::from_fn(|c| mixed[c] * mixed[c]);
            let tint: [f32; 3] = std::array::from_fn(|c| 1. + blue * (decal[c] - 1.));
            let gloss = mixed[3]
                * if self.third_multiply {
                    third_tint[0] * 0.3 + third_tint[1] * 0.6 + third_tint[2] * 0.1
                } else if self.decal && !self.second_additive && self.second_normal == 0 {
                    tint[0] * 0.3 + tint[1] * 0.6 + tint[2] * 0.1
                } else {
                    1.
                };
            let (base, metallic, roughness) =
                crate::material::pbr_pixel([color[0], color[1], color[2]], rgb, gloss);
            color = [base[0], base[1], base[2], 1.];
            response = [1., roughness, metallic, 1.];
            specular = [rgb[0], rgb[1], rgb[2], gloss];
        } else if self.base_alpha_response {
            response = [
                1.,
                (2. / (2_f32.powf(13. * c0[3]) + 2.)).powf(0.25),
                0.,
                c0[3],
            ];
        }
        Ok([color, normal, response, specular])
    }
}

pub(super) fn auxiliary(
    surface: &Surface,
    index: usize,
    decal: bool,
    color_only_layer: bool,
) -> Result<([f32; 2], [f32; 2], [f32; 4])> {
    let bytes = surface
        .auxiliary
        .get(index)
        .ok_or("Layered world vertex has no auxiliary data")?;
    let size = if color_only_layer {
        if decal { 8 } else { 4 }
    } else if decal {
        12
    } else {
        8
    };
    if bytes.len() != size {
        return Err(format!(
            "Layered world vertex requires {size} auxiliary bytes, found {}",
            bytes.len()
        )
        .into());
    }
    let uv = [
        half::f16::from_bits(u16_at(bytes, 0)).to_f32(),
        half::f16::from_bits(u16_at(bytes, 2)).to_f32(),
    ];
    if !uv.iter().all(|v| v.is_finite()) {
        return Err("Layered world UV is not finite".into());
    }
    let uv2 = if decal {
        [
            half::f16::from_bits(u16_at(bytes, 4)).to_f32(),
            half::f16::from_bits(u16_at(bytes, 6)).to_f32(),
        ]
    } else {
        uv
    };
    if !uv2.iter().all(|v| v.is_finite()) {
        return Err("Layered decal UV is not finite".into());
    }
    Ok((
        uv,
        uv2,
        if color_only_layer {
            [1., 0., 0., 1.]
        } else {
            std::array::from_fn(|i| bytes[size - 4 + i] as f32 / 255.)
        },
    ))
}

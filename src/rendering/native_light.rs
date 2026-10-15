//! Source FX omni-light submission (00410c30), merge/selection (00430c10,
//! 00430a00), and view constants (00416df0). Units here are source inches.
use super::native::NativeOmniLights;
use bevy::prelude::*;

#[derive(Component, Clone, Copy, Default)]
pub(crate) struct FxOmniLight {
    pub radius: f32,
    pub rgb: Vec3,
    pub ignore_far_plane: bool,
}

impl FxOmniLight {
    pub fn evaluate(radius: f32, rgba: [f32; 4], ignore_far_plane: bool) -> Self {
        // 005b7e50 packs by clamping and truncating, before 005b7af0 expands
        // bytes to [0,1] and 00410c30 applies the sRGB transfer and alpha*32.
        let rgba = rgba.map(|v| v.clamp(0., 255.) as u8 as f32 / 255.);
        let linear = |c: f32| {
            if c <= 0.04045 {
                c * 0.07739938
            } else {
                ((c + 0.055) * 0.94786733).powf(2.4)
            }
        };
        Self {
            radius,
            rgb: Vec3::new(linear(rgba[0]), linear(rgba[1]), linear(rgba[2])) * (rgba[3] * 32.),
            ignore_far_plane,
        }
    }
}

#[derive(Clone, Copy, Default)]
struct Submitted {
    origin: Vec3,
    radius: f32,
    rgb: Vec3,
}

/// Stack storage matches the native 32-submission cap. Merge only after the
/// cap, so repeated nearby lights cannot admit submissions the source drops.
#[derive(Default)]
pub(crate) struct OmniSubmissions {
    lights: [Submitted; 32],
    count: usize,
}

impl OmniSubmissions {
    pub fn submit(&mut self, origin: Vec3, light: &FxOmniLight) {
        if light.radius > 0. && self.count < self.lights.len() {
            self.lights[self.count] = Submitted {
                origin,
                radius: light.radius.min(10000.),
                rgb: light.rgb,
            };
            self.count += 1;
        }
    }

    pub fn view(mut self, eye: Vec3) -> NativeOmniLights {
        let mut merged = 0;
        for i in 0..self.count {
            let next = self.lights[i];
            if let Some(prior) = self.lights[..merged].iter_mut().find(|prior| {
                (prior.radius - next.radius).abs() < 10.
                    && prior.origin.distance_squared(next.origin) < 100.
            }) {
                prior.origin = (prior.origin + next.origin) * 0.5;
                prior.radius = prior.radius.max(next.radius);
                prior.rgb += next.rgb;
            } else {
                self.lights[merged] = next;
                merged += 1;
            }
        }
        if merged > 4 {
            select(&mut self.lights[..merged], 4, eye);
        }
        let mut out = NativeOmniLights::default();
        for (i, light) in self.lights[..merged.min(4)].iter().enumerate() {
            let position = light.origin - eye;
            out.position_x[i] = position.x;
            out.position_y[i] = position.y;
            out.position_z[i] = position.z;
            out.falloffs[i] = -1. / light.radius;
            out.reds[i] = light.rgb.x;
            out.greens[i] = light.rgb.y;
            out.blues[i] = light.rgb.z;
        }
        out.enabled.x = if merged == 0 { 0. } else { 1. };
        out
    }
}

// Literal partition order from 00430a00 for the type-5-only list. Spot lights
// require its additional type-2 priority and remain rejected by FX spawning.
fn select(mut lights: &mut [Submitted], mut limit: usize, eye: Vec3) {
    let closer = |a: Submitted, b: Submitted| {
        a.origin.distance_squared(eye) * b.radius * b.radius
            <= b.origin.distance_squared(eye) * a.radius * a.radius
    };
    loop {
        let pivot = lights[0];
        let mut left = 0;
        let mut right = lights.len();
        loop {
            left += 1;
            while left < right && closer(lights[left], pivot) {
                left += 1;
            }
            if left <= right {
                loop {
                    right -= 1;
                    if right < left || !closer(pivot, lights[right]) {
                        break;
                    }
                }
            }
            if right < left {
                break;
            }
            lights.swap(left, right);
        }
        if left == lights.len() {
            lights.swap(0, right);
            left -= 1;
        }
        if left == limit {
            return;
        }
        if left < limit {
            limit -= left;
            lights = &mut lights[left..];
        } else {
            lights = &mut lights[..left];
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn light(radius: f32) -> FxOmniLight {
        FxOmniLight {
            radius,
            rgb: Vec3::ONE,
            ignore_far_plane: false,
        }
    }

    #[test]
    fn merges_with_strict_source_bounds_and_packs_eye_relative() {
        let mut batch = OmniSubmissions::default();
        batch.submit(Vec3::ZERO, &light(35.));
        batch.submit(Vec3::X * 8., &light(40.));
        batch.submit(Vec3::X * 14., &light(40.)); // exactly ten from merged center
        let view = batch.view(Vec3::X);
        assert_eq!(view.position_x, Vec4::new(3., 13., 0., 0.));
        assert_eq!(view.reds, Vec4::new(2., 1., 0., 0.));
        assert_eq!(view.falloffs.x, -1. / 40.);
    }

    #[test]
    fn submission_cap_precedes_merge() {
        let mut batch = OmniSubmissions::default();
        for _ in 0..40 {
            batch.submit(Vec3::ZERO, &light(35.));
        }
        assert_eq!(batch.view(Vec3::ZERO).reds, Vec4::new(32., 0., 0., 0.));
    }

    #[test]
    fn selection_uses_distance_over_radius_not_distance_alone() {
        let mut batch = OmniSubmissions::default();
        for x in [10., 20., 30., 40., 50.] {
            batch.submit(Vec3::X * x, &light(1.));
        }
        batch.submit(Vec3::X * 100., &light(100.));
        let view = batch.view(Vec3::ZERO);
        let mut positions = view.position_x.to_array();
        positions.sort_by(f32::total_cmp);
        assert_eq!(positions, [10., 20., 30., 100.]);
    }

    #[test]
    fn source_color_quantizes_before_linear_conversion() {
        let a = FxOmniLight::evaluate(20000., [255., 0., 0., 127.9], false);
        let mut batch = OmniSubmissions::default();
        batch.submit(Vec3::ZERO, &a);
        assert_eq!(batch.view(Vec3::ZERO).falloffs.x, -1. / 10000.);
        assert!((a.rgb.x - 32. * 127. / 255.).abs() < 0.00001);
        assert_eq!(a.rgb.y, 0.);
        assert_eq!(a.rgb.z, 0.);
    }
}

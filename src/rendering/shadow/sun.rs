//! Two sun projections: 0045be20, 0045c810, 0045bc40 and 0045c4a0.
//! Origins uses the native default shadow scale1 and zero center override.
use bevy::{prelude::*, render::extract_component::ExtractComponent};

#[derive(Component, Clone, Copy, Debug, PartialEq, ExtractComponent)]
#[extract_app(bevy::render::RenderApp)]
pub(crate) struct SunShadowViews {
    pub clip_from_world: [Mat4; 2],
    pub size: u32,
}

impl SunShadowViews {
    pub(super) fn new(
        eye: Vec3,
        sun_direction: Vec3,
        clip_from_eye: Mat4,
        world_bounds: [Vec3; 2],
    ) -> Self {
        // 00455720: 1024 << sm_sunQuality; 0041f2b0 defaults quality to0.
        // zm_tomb.csc overrides sm_sunSampleSizeNear to0.3, divided by
        // 1<<quality in0045d8f0. Scale1 yields a fourfold far sample spacing.
        let size = 1024;
        let near_sample = 0.3;
        let far_sample = near_sample * 4.0;
        let forward = -sun_direction;
        assert!(
            forward.is_finite() && (forward.length_squared() - 1.0).abs() < 0.001,
            "Original sun direction must be a unit vector"
        );
        let seed = if forward.x * forward.x + forward.y * forward.y >= 0.1 {
            Vec3::Z
        } else {
            Vec3::X
        };
        let left = seed.cross(forward).normalize();
        let up = forward.cross(left);
        let right = -left;
        let light_xy = |p: Vec3| Vec2::new(right.dot(p), up.dot(p));
        let light_eye = light_xy(eye);
        let inverse = clip_from_eye.inverse();
        // 00c5e630 contains four z=1 corners. 0074b550 uses reverse Z,
        // so these are the near plane, not far-frustum corners.
        let mut min = light_eye;
        let mut max = light_eye;
        for corner in [
            Vec3::new(-1., -1., 1.),
            Vec3::new(1., -1., 1.),
            Vec3::new(1., 1., 1.),
            Vec3::new(-1., 1., 1.),
        ] {
            let xy = light_xy(eye + inverse.project_point3(corner));
            min = min.min(xy);
            max = max.max(xy);
        }
        let span = (max - min).max_element();
        assert!(
            span.is_finite() && span > 0.0,
            "Original sun projection camera extent"
        );
        let pixel_scale = (size - 1) as f32 / span;
        let middle = (min + max) * 0.5;
        // The one-texel border and asymmetric ties are from0045c810.
        let anchor = Vec2::new(
            if light_eye.x < middle.x {
                1.0 + (light_eye.x - min.x) * pixel_scale
            } else {
                (size - 1) as f32 - (max.x - light_eye.x) * pixel_scale
            },
            if middle.y < light_eye.y {
                1.0 + (max.y - light_eye.y) * pixel_scale
            } else {
                (size - 1) as f32 - (light_eye.y - min.y) * pixel_scale
            },
        );
        let snapped = (light_eye / far_sample).floor() * far_sample;
        let view = Mat4::from_cols(
            right.extend(-snapped.x),
            up.extend(-snapped.y),
            forward.extend(0.0),
            Vec4::W,
        )
        .transpose();
        let radius = size as f32 * far_sample;
        let [near, far] = depth_bounds(eye, forward, world_bounds, radius);
        let clip_from_world = [near_sample, far_sample].map(|sample| {
            let delta = (snapped - light_eye) / sample;
            let pixels = (anchor + Vec2::new(delta.x, -delta.y)).floor();
            let offset = Vec2::new(
                2.0 * pixels.x / size as f32 - 1.0,
                1.0 - 2.0 * pixels.y / size as f32,
            );
            let scale = 2.0 / (size as f32 * sample);
            let depth_scale = 1.0 / (far - near);
            Mat4::from_cols(
                Vec4::new(scale, 0., 0., 0.),
                Vec4::new(0., scale, 0., 0.),
                Vec4::new(0., 0., -depth_scale, 0.),
                Vec4::new(offset.x, offset.y, far * depth_scale, 1.),
            ) * view
        });
        Self {
            clip_from_world,
            size,
        }
    }
}

fn depth_bounds(eye: Vec3, forward: Vec3, world: [Vec3; 2], radius: f32) -> [f32; 2] {
    // 0074bce0 unions the world with eye +/-2048. 0045b930 clips the
    // infinite sun line to that box, expands its hit bounds, then clamps.
    let min = world[0].min(eye - Vec3::splat(2048.0));
    let max = world[1].max(eye + Vec3::splat(2048.0));
    let mut hit_min = Vec3::splat(f32::MAX);
    let mut hit_max = Vec3::splat(-f32::MAX);
    let mut hits = 0;
    for edge in [min, max] {
        for axis in 0..3 {
            if forward[axis] == 0.0 {
                continue;
            }
            let t = (edge[axis] - eye[axis]) / forward[axis];
            let point = eye + forward * t;
            let a = !axis & 1;
            let b = !axis & 2;
            if point[a] >= min[a] && point[a] <= max[a] && point[b] >= min[b] && point[b] <= max[b]
            {
                hits += 1;
                hit_min = hit_min.min(point);
                hit_max = hit_max.max(point);
            }
        }
    }
    let (min, max) = if hits < 2 {
        (min, max)
    } else {
        (
            (hit_min - Vec3::splat(radius)).max(min),
            (hit_max + Vec3::splat(radius)).min(max),
        )
    };
    let low = Vec3::new(
        if forward.x >= 0. { min.x } else { max.x },
        if forward.y >= 0. { min.y } else { max.y },
        if forward.z >= 0. { min.z } else { max.z },
    );
    let high = Vec3::new(
        if forward.x >= 0. { max.x } else { min.x },
        if forward.y >= 0. { max.y } else { min.y },
        if forward.z >= 0. { max.z } else { min.z },
    );
    let near = forward.dot(low);
    let mut far = forward.dot(high);
    let limit = forward.dot(eye) + radius;
    if near < limit && limit < far {
        far = limit;
    }
    assert!(
        near.is_finite() && far.is_finite() && far > near,
        "Original sun shadow depth bounds"
    );
    [near, far]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn depth_bounds_keep_world_casters_and_limit_forward_reach() {
        let world = [Vec3::splat(-5000.), Vec3::splat(5000.)];
        assert_eq!(
            depth_bounds(Vec3::ZERO, Vec3::Z, world, 1228.8),
            [-5000., 1228.8]
        );
        assert_eq!(
            depth_bounds(Vec3::ZERO, -Vec3::Z, world, 1228.8),
            [-5000., 1228.8]
        );
    }

    #[test]
    fn both_projections_use_reverse_depth_and_fourfold_far_coverage() {
        let world = [Vec3::splat(-5000.), Vec3::splat(5000.)];
        let camera = bevy::math::proj::perspective_infinite_reverse(1.0, 16. / 9., 4.);
        // Oblique sun exercises the normal seed branch.
        let sun = Vec3::new(0.6, 0., 0.8);
        let views = SunShadowViews::new(Vec3::ZERO, sun, camera, world);
        let [near, far] = depth_bounds(Vec3::ZERO, -sun, world, 1228.8);
        for matrix in views.clip_from_world {
            assert!((matrix.project_point3(-sun * near).z - 1.).abs() < 1e-5);
            assert!(matrix.project_point3(-sun * far).z.abs() < 1e-5);
        }
        let near_step = views.clip_from_world[0].transform_vector3(Vec3::Y).length();
        let far_step = views.clip_from_world[1].transform_vector3(Vec3::Y).length();
        assert!((near_step / far_step - 4.).abs() < 1e-5);
        assert_eq!(views.size, 1024);
    }

    #[test]
    fn vertical_sun_keeps_finite_views_and_quantizes_camera_motion() {
        let world = [Vec3::splat(-5000.), Vec3::splat(5000.)];
        let camera = bevy::math::proj::perspective_infinite_reverse(1.0, 1., 4.);
        let eye = Vec3::new(0.31, 0.31, 0.31);
        let a = SunShadowViews::new(eye, Vec3::Z, camera, world);
        let b = SunShadowViews::new(eye + Vec3::X * 0.001, Vec3::Z, camera, world);
        for (a, b) in a.clip_from_world.into_iter().zip(b.clip_from_world) {
            assert!(a.is_finite() && b.is_finite());
            // The native midpoint branch can move one texel at an exactly
            // symmetric frustum. It must not introduce fractional-texel drift.
            let delta = (b.w_axis - a.w_axis) * 512.0;
            assert!((delta.x - delta.x.round()).abs() < 1e-5);
            assert!((delta.y - delta.y.round()).abs() < 1e-5);
            assert!(delta.x.abs() <= 1. && delta.y.abs() <= 1.);
            assert!(a.x_axis.abs_diff_eq(b.x_axis, 1e-6));
            assert!(a.y_axis.abs_diff_eq(b.y_axis, 1e-6));
            assert!(a.z_axis.abs_diff_eq(b.z_axis, 1e-6));
            assert_eq!(delta.z, 0.);
        }
    }

    #[test]
    fn symmetric_frustum_preserves_native_one_texel_border() {
        // Exactly representable 90-degree projection: near corners (+/-4,+/-4,-4).
        let camera = Mat4::from_cols(
            Vec4::X,
            Vec4::Y,
            Vec4::new(0., 0., 0., -1.),
            Vec4::new(0., 0., 4., 0.),
        );
        let views = SunShadowViews::new(
            Vec3::ZERO,
            Vec3::Z,
            camera,
            [Vec3::splat(-5000.), Vec3::splat(5000.)],
        );
        for matrix in views.clip_from_world {
            assert_eq!(matrix.w_axis.x, -1. / 512.);
            assert_eq!(matrix.w_axis.y, 1. / 512.);
        }
    }
}

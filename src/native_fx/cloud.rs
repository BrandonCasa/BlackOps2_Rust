//! Original shared cloud geometry (0043f4c0) and draw constants (00473600).
use crate::rendering::native_effect::{
    NativeCloudObject, NativeEffectGeometry, NativeEffectProgram, NativeEffectVertexLayout,
    NativeOutdoorMap,
};
use bevy::prelude::*;

#[derive(Resource)]
pub(super) struct Geometry(pub Handle<NativeEffectGeometry>);

pub(super) struct State {
    pub program: NativeEffectProgram,
    pub size: Vec2,
    pub velocity: Vec3,
    pub color: [f32; 4],
}

pub(super) fn geometry(assets: &mut Assets<NativeEffectGeometry>) -> Geometry {
    fn crt(state: &mut u32) -> f32 {
        *state = state.wrapping_mul(214013).wrapping_add(2531011);
        ((*state >> 16) & 32767) as f32
    }
    fn gaussian(state: &mut u32) -> (f32, f32) {
        loop {
            let x = crt(state) * 3.0517578e-5 * 2. - 1.;
            let y = crt(state) * 3.0517578e-5 * 2. - 1.;
            let square = x * x + y * y;
            if square <= 1. {
                let factor = (square.ln() * -2. / square).sqrt();
                return (x * factor, y * factor);
            }
        }
    }
    fn quantize(value: f32) -> i16 {
        (value.clamp(-1., 1.) * 32767.) as i16
    }

    // Native startup uses CRT's initial holdrand=1. Generate both original
    // distributions once; every cloud draw selects a contiguous index range.
    let mut random = 1;
    let mut permutation = 0x1ffu16;
    let mut point_index = 0usize;
    let mut uniform = [[0i16; 3]; 1024];
    let mut normal = [[0i16; 3]; 1024];
    let mut sequence = 0;
    for i in 0..8 {
        for j in 0..8 {
            for k in 0..16 {
                permutation = (0u16.wrapping_sub(permutation & 1) & 0x240) ^ (permutation >> 1);
                uniform[point_index] = [
                    quantize((crt(&mut random) * 3.051851e-5 + i as f32) * 0.25 - 1.),
                    quantize((crt(&mut random) * 3.051851e-5 + j as f32) * 0.25 - 1.),
                    quantize((crt(&mut random) * 3.051851e-5 + k as f32) * 0.125 - 1.),
                ];
                let (x, y) = gaussian(&mut random);
                let (z, _) = gaussian(&mut random);
                normal[sequence] = [quantize(x * 0.25), quantize(y * 0.25), quantize(z * 0.25)];
                point_index = permutation as usize;
                sequence += 1;
            }
        }
    }
    let mut vertices = Vec::with_capacity(0x10000);
    let mut indices = Vec::with_capacity(0x6000);
    for (point, position) in uniform.into_iter().chain(normal).enumerate() {
        for corner in 0i16..4 {
            for value in [position[0], position[1], position[2], corner << 13] {
                vertices.extend_from_slice(&value.to_le_bytes());
            }
        }
        for offset in [0u16, 1, 2, 2, 1, 3] {
            indices.extend_from_slice(&(point as u16 * 4 + offset).to_le_bytes());
        }
    }
    Geometry(assets.add(NativeEffectGeometry {
        layout: NativeEffectVertexLayout::ParticleCloud8,
        vertices,
        indices,
    }))
}

pub(super) fn constants(
    transform: &GlobalTransform,
    camera: &GlobalTransform,
    outdoor: Option<&NativeOutdoorMap>,
    state: &State,
) -> NativeCloudObject {
    let (_, camera_rotation, eye) = camera.to_scale_rotation_translation();
    let source_from_bevy = crate::rendering::world::source_frame().inverse();
    let world = source_from_bevy * Mat4::from_translation(-eye) * transform.to_matrix();
    let right = source_from_bevy
        .transform_vector3(camera_rotation * Vec3::X)
        .normalize();
    let up = source_from_bevy
        .transform_vector3(camera_rotation * Vec3::Y)
        .normalize();
    let velocity_world = if state.program == NativeEffectProgram::CloudRain {
        source_from_bevy
            .transform_vector3(state.velocity)
            .normalize_or_zero()
    } else {
        Vec3::ZERO
    };
    let cloud_matrix = match state.program {
        NativeEffectProgram::CloudRain | NativeEffectProgram::CloudVertical => {
            Vec4::new(state.size.x, state.size.y, 0., 0.)
        }
        NativeEffectProgram::CloudSnow
        | NativeEffectProgram::CloudSimple
        | NativeEffectProgram::CloudOutdoorAdditive
        | NativeEffectProgram::CloudAdditive => {
            // Generic branch of 00473600: preserve the signed comparison and
            // minimum width, shared by snow and both plain cloud programs.
            if state.size.x == state.size.y
                || state.velocity.length_squared() < 1.0000001e-6 * 0.0254f32.powi(2)
            {
                Vec4::new(state.size.x, 0., 0., state.size.y)
            } else {
                let velocity =
                    camera_rotation.inverse() * -state.velocity.normalize_or_zero() * state.size.y;
                let (mut x, mut y) = (velocity.x, velocity.y);
                if x >= 0.001 || y >= 0.001 {
                    let length = Vec2::new(x, y).length();
                    let width_scale = state.size.x / length;
                    let (width_x, width_y) = (y * width_scale, -x * width_scale);
                    if length < state.size.x {
                        x *= width_scale;
                        y *= width_scale;
                    }
                    Vec4::new(width_x, width_y, x, y)
                } else {
                    Vec4::new(state.size.x, 0., 0., state.size.y)
                }
            }
        }
        _ => unreachable!("cloud owns an original cloud vertex program"),
    };
    let outdoor_rows = if state.program.cloud_outdoor() {
        let outdoor = outdoor.expect("weather cloud requires the original outdoor map");
        // 0046b3b0: source native matrices use row vectors. Preserve the original
        // four rows as columns before composing our column-vector world frame.
        let lookup = Mat4::from_cols_array_2d(&outdoor.lookup_rows.map(|row| row.to_array()));
        // 0041f2b0 defaults: r_outdoorAwayBias 32 inches, r_outdoorDownBias 0.
        let bias = source_from_bevy.transform_vector3(camera_rotation * Vec3::Z * (32. * 0.0254));
        let absolute_world = source_from_bevy * transform.to_matrix();
        let outdoor_matrix = lookup * Mat4::from_translation(bias) * absolute_world;
        std::array::from_fn(|i| outdoor_matrix.row(i))
    } else {
        [Vec4::ZERO; 3]
    };
    let color = Vec4::from_array(
        state
            .color
            .map(|v| (v.clamp(0., 255.) as u8) as f32 * 0.003921569),
    );
    NativeCloudObject {
        world_rows: std::array::from_fn(|i| world.row(i)),
        outdoor_rows,
        view_right: right.extend(0.),
        view_up: up.extend(0.),
        cloud_matrix,
        velocity_world: velocity_world.extend(0.),
        color,
    }
}

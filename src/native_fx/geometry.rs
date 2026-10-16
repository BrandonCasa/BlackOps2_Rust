//! Original PACKED32 particle inputs; no Bevy material or mesh adapter.
use super::Trail;
use crate::rendering::native_effect::{NativeEffectGeometry, NativeEffectVertexLayout};
use bevy::{camera::primitives::Aabb, math::Affine2, prelude::*};

// 005b52a0 writes this binary16 packing, including its source truncation and
// saturation. Preserve it instead of changing UVs through an f32 mesh attribute.
fn packed_half(value: f32) -> u16 {
    let bits = value.to_bits();
    let narrowed = ((bits.wrapping_mul(2) ^ 0x8000_3fff) as i32) >> 14;
    let narrowed = if narrowed < 0x3fff {
        if narrowed < -0x3fff { 0 } else { narrowed }
    } else {
        0x3fff
    };
    (narrowed as u16 & 0x3fff) | ((bits >> 16) as u16 & 0xc000)
}

pub(super) fn vertex(bytes: &mut Vec<u8>, position: Vec3, color: [u8; 4], uv: Vec2, normal: u32) {
    for value in [position.x, position.y, position.z, -1.] {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    bytes.extend_from_slice(&color);
    bytes.extend_from_slice(&packed_half(uv.x).to_le_bytes());
    bytes.extend_from_slice(&packed_half(uv.y).to_le_bytes());
    bytes.extend_from_slice(&normal.to_le_bytes());
    bytes.extend_from_slice(&[0; 4]);
}

// 005b52a0 / 005b6000 / 005bf460 use CVTPS2DQ with nearest-even MXCSR.
// Round each full scaled product before masking, including the shifted Y/Z.
pub(super) fn pack_normal(normal: Vec3) -> u32 {
    let normal = normal.clamp(Vec3::NEG_ONE, Vec3::ONE);
    ((normal.x * 511.).round_ties_even() as i32 as u32 & 0x3ff)
        | ((normal.y * 523264.).round_ties_even() as i32 as u32 & 0xffc00)
        | ((normal.z * 535822336.).round_ties_even() as i32 as u32 & 0x3ff00000)
}

pub(super) fn quad(
    pose: Transform,
    normal: Vec3,
    color: [u8; 4],
    uv: Affine2,
    trim: [f32; 2],
) -> (NativeEffectGeometry, Aabb, u32) {
    // 005b52a0 packs the world-space normal as signed 10-bit components.
    // Convert the engine's axes back to the original Z-up frame first.
    let normal = Vec3::new(normal.x, -normal.z, normal.y).clamp(Vec3::NEG_ONE, Vec3::ONE);
    let packed_normal = pack_normal(normal);
    let tangent = pose.rotation * Vec3::X;
    let tangent = pack_normal(Vec3::new(tangent.x, -tangent.z, tangent.y));
    let mut vertices = Vec::with_capacity(128);
    let mut minimum = Vec3::splat(f32::INFINITY);
    let mut maximum = Vec3::splat(f32::NEG_INFINITY);
    // Source 005b52a0 A=right*cos+up*sin, B=right*sin-up*cos.
    // Transform rotates local X/Y normally, so its local Y represents -B.
    let minimum_u = (1. - trim[0]) * 0.5;
    let top_minimum_u = (1. - trim[1]) * 0.5;
    let maximum_u = (1. + trim[1]) * 0.5;
    for (position, texcoord) in [
        (
            Vec3::new(-trim[1] * 0.5, -0.5, 0.),
            Vec2::new(top_minimum_u, 1.),
        ),
        (Vec3::new(-trim[0] * 0.5, 0.5, 0.), Vec2::new(minimum_u, 0.)),
        (
            Vec3::new(trim[0] * 0.5, 0.5, 0.),
            Vec2::new((1. + trim[0]) * 0.5, 0.),
        ),
        (Vec3::new(trim[1] * 0.5, -0.5, 0.), Vec2::new(maximum_u, 1.)),
    ] {
        // Visibility bounds stay in entity-local coordinates even though the
        // original vertex stream bakes rotation and size into world offsets.
        minimum = minimum.min(position);
        maximum = maximum.max(position);
        let position = pose.rotation * (pose.scale * position);
        vertex(
            &mut vertices,
            Vec3::new(position.x, -position.z, position.y) / 0.0254,
            color,
            uv.transform_point2(texcoord),
            packed_normal,
        );
    }
    let indices = [0u16, 1, 2, 2, 3, 0]
        .into_iter()
        .flat_map(u16::to_le_bytes)
        .collect();
    for vertex in vertices.chunks_exact_mut(32) {
        vertex[28..32].copy_from_slice(&tangent.to_le_bytes());
    }
    (
        NativeEffectGeometry {
            layout: NativeEffectVertexLayout::Packed32,
            vertices,
            indices,
        },
        Aabb::from_min_max(minimum, maximum),
        6,
    )
}

/// Type 3 directional sprite (005b6bf0 / 005b6cc0 -> 005b6a80).
/// Origin, eye and direction use engine world axes/metres; size is the source
/// half-size in inches. Positions are source-inch offsets from origin, while
/// bounds use engine metres. The owner supplies translation only at draw time.
pub(super) fn directional(
    origin: Vec3,
    eye: Vec3,
    direction: Vec3,
    size: Vec2,
    rotation: f32,
    color: [u8; 4],
    uv: Affine2,
    trim: [f32; 2],
    pivot: Vec2,
) -> Result<(NativeEffectGeometry, Aabb, u32), String> {
    if !origin.is_finite()
        || !eye.is_finite()
        || !direction.is_finite()
        || !size.is_finite()
        || !rotation.is_finite()
        || !pivot.is_finite()
        || trim.iter().any(|width| !width.is_finite())
        || !uv.matrix2.is_finite()
        || !uv.translation.is_finite()
    {
        return Err("original directional sprite has non-finite inputs".into());
    }
    let source_frame = crate::rendering::world::source_frame();
    let source_from_bevy = source_frame.inverse();
    // Both source normalizations replace an exactly zero divisor with 1.
    // A zero direction or eye-parallel cross product stays zero, with no
    // camera-facing substitute and no rejection of the resulting geometry.
    let normalize = |value: Vec3| {
        let length = value.length();
        value * (1. / if length > 0. { length } else { 1. })
    };
    let up = normalize(source_from_bevy.transform_vector3(direction));
    let center = -up * size.y;
    let to_eye = source_from_bevy.transform_vector3(eye - origin) - center;
    let right = normalize(up.cross(to_eye));
    // 005b52a0 packs this world source normal before rotation/size/pivot.
    let normal = pack_normal(right.cross(up));
    let (sin, cos) = rotation.sin_cos();
    let a = (right * cos + up * sin) * size.x;
    let b = (right * sin - up * cos) * size.y;
    let center = center + a * pivot.x + b * pivot.y;
    let positions = [
        center - a * trim[1] + b,
        center - a * trim[0] - b,
        center + a * trim[0] - b,
        center + a * trim[1] + b,
    ];
    if positions.iter().any(|position| !position.is_finite()) {
        return Err("original directional sprite has non-finite positions".into());
    }
    let mut vertices = Vec::with_capacity(128);
    let mut minimum = Vec3::splat(f32::INFINITY);
    let mut maximum = Vec3::splat(f32::NEG_INFINITY);
    for (position, texcoord) in positions.into_iter().zip([
        Vec2::new((1. - trim[1]) * 0.5, 1.),
        Vec2::new((1. - trim[0]) * 0.5, 0.),
        Vec2::new((1. + trim[0]) * 0.5, 0.),
        Vec2::new((1. + trim[1]) * 0.5, 1.),
    ]) {
        let engine_position = source_frame.transform_vector3(position);
        minimum = minimum.min(engine_position);
        maximum = maximum.max(engine_position);
        vertex(
            &mut vertices,
            position,
            color,
            uv.transform_point2(texcoord),
            normal,
        );
    }
    Ok((
        NativeEffectGeometry {
            layout: NativeEffectVertexLayout::Packed32,
            vertices,
            indices: [0u16, 1, 2, 2, 3, 0]
                .into_iter()
                .flat_map(u16::to_le_bytes)
                .collect(),
        },
        Aabb::from_min_max(minimum, maximum),
        6,
    ))
}

pub(super) fn trail(
    source: &Trail,
    pose: Transform,
    start: Vec3,
    size: Vec2,
    rotation: f32,
    color: [u8; 4],
    uv: Affine2,
) -> Result<(NativeEffectGeometry, Aabb, u32), String> {
    let local_start = pose.compute_affine().inverse().transform_point3(start);
    let source_from_bevy = crate::rendering::world::source_frame().inverse();
    // 005b6000: A=right*cos+up*sin, B=right*sin-up*cos.
    let (sin, cos) = rotation.sin_cos();
    let a = Vec3::new(cos, sin, 0.);
    let b = Vec3::new(sin, -cos, 0.);
    let length = local_start.length();
    let sections = if source.split_distance > 0. {
        (length / (source.split_distance * 0.0254)).ceil().max(1.) as usize
    } else {
        1
    };
    let count = source.vertices.len();
    let vertex_count = (sections + 1)
        .checked_mul(count)
        .filter(|count| *count > 0 && *count <= u16::MAX as usize + 1)
        .ok_or("original trail exceeds one Uint16 draw or has no vertices")?;
    if !source.indices.len().is_multiple_of(2)
        || source.indices.iter().any(|v| *v as usize >= count)
    {
        return Err("original trail has an invalid edge index".into());
    }
    let mut vertices = Vec::with_capacity(vertex_count * 32);
    let mut indices = Vec::with_capacity(sections * source.indices.len() * 6);
    let mut minimum = Vec3::splat(f32::INFINITY);
    let mut maximum = Vec3::splat(f32::NEG_INFINITY);
    for ring in 0..=sections {
        let fraction = ring as f32 / sections as f32;
        let center = local_start * (1. - fraction);
        for source_vertex in &source.vertices {
            let position = center
                + a * (source_vertex.position[0] * size.x * 0.0254)
                + b * (source_vertex.position[1] * size.y * 0.0254);
            minimum = minimum.min(position);
            maximum = maximum.max(position);
            let texcoord = Vec2::new(
                if source.repeat_distance > 0. {
                    fraction * length / (source.repeat_distance * 0.0254)
                } else {
                    fraction
                },
                source_vertex.texcoord,
            );
            // Emit source-axis inches relative to the entity origin. The object
            // constant supplies translation only, so source normals retain unit
            // scale through VS618fd291's object 3x3.
            let source_position =
                source_from_bevy.transform_vector3(pose.rotation * (pose.scale * position));
            let normal =
                pose.rotation * (a * source_vertex.normal[0] + b * source_vertex.normal[1]);
            let normal = Vec3::new(normal.x, -normal.z, normal.y);
            vertex(
                &mut vertices,
                source_position,
                color,
                uv.transform_point2(texcoord),
                pack_normal(normal),
            );
        }
    }
    for ring in 0..sections {
        for edge in source.indices.chunks_exact(2) {
            let a = ring * count + edge[0] as usize;
            let b = ring * count + edge[1] as usize;
            let c = a + count;
            let d = b + count;
            for index in [a, b, c, b, d, c] {
                indices.extend_from_slice(&(index as u16).to_le_bytes());
            }
        }
    }
    let index_count = (indices.len() / 2) as u32;
    Ok((
        NativeEffectGeometry {
            layout: NativeEffectVertexLayout::Packed32,
            vertices,
            indices,
        },
        Aabb::from_min_max(minimum, maximum),
        index_count,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::native_fx::TrailVertex;

    fn position(geometry: &NativeEffectGeometry, index: usize) -> Vec3 {
        let vertex = &geometry.vertices[index * 32..];
        Vec3::from_array(std::array::from_fn(|axis| {
            f32::from_le_bytes(vertex[axis * 4..axis * 4 + 4].try_into().unwrap())
        }))
    }

    #[test]
    fn packed_normal_rounds_the_full_source_products_before_masking() {
        assert_eq!(
            pack_normal(Vec3::splat(0.5)),
            256 | (255 << 10) | (255 << 20)
        );
        assert_eq!(
            pack_normal(Vec3::splat(-0.5)),
            768 | (768 << 10) | (768 << 20)
        );
    }

    #[test]
    fn quad_bakes_source_offsets_and_packs_unit_normal_and_tangent() {
        let pose = Transform::from_xyz(10., 20., 30.)
            .with_rotation(Quat::from_rotation_y(std::f32::consts::FRAC_PI_2))
            .with_scale(Vec3::new(0.1016, 0.1524, 9.));
        let (geometry, bounds, count) = quad(
            pose,
            pose.rotation * Vec3::Z,
            [1, 2, 3, 4],
            Affine2::IDENTITY,
            [1., 1.],
        );
        assert_eq!(count, 6);
        assert!(position(&geometry, 0).abs_diff_eq(Vec3::new(0., -2., -3.), 1e-5));
        assert!(position(&geometry, 2).abs_diff_eq(Vec3::new(0., 2., 3.), 1e-5));
        assert!(Vec3::from(bounds.center).abs_diff_eq(Vec3::ZERO, 1e-6));
        assert!(Vec3::from(bounds.half_extents).abs_diff_eq(Vec3::new(0.5, 0.5, 0.), 1e-5));
        for vertex in geometry.vertices.chunks_exact(32) {
            assert_eq!(&vertex[16..20], &[1, 2, 3, 4]);
            assert_eq!(
                u32::from_le_bytes(vertex[24..28].try_into().unwrap()),
                pack_normal(Vec3::X)
            );
            assert_eq!(
                u32::from_le_bytes(vertex[28..32].try_into().unwrap()),
                pack_normal(Vec3::Y)
            );
        }
    }

    #[test]
    fn directional_starts_at_origin_and_extends_back_by_two_half_sizes() {
        let origin = Vec3::new(100., 200., 300.);
        let (geometry, bounds, count) = directional(
            origin,
            origin + Vec3::Z,
            Vec3::Y,
            Vec2::new(2., 3.),
            0.,
            [1, 2, 3, 4],
            Affine2::IDENTITY,
            [1., 1.],
            Vec2::ZERO,
        )
        .unwrap();
        assert_eq!(count, 6);
        assert!(position(&geometry, 0).abs_diff_eq(Vec3::new(-2., 0., -6.), 1e-5));
        assert!(position(&geometry, 2).abs_diff_eq(Vec3::new(2., 0., 0.), 1e-5));
        assert!(Vec3::from(bounds.center).abs_diff_eq(Vec3::new(0., -0.0762, 0.), 1e-5));
        assert!(Vec3::from(bounds.half_extents).abs_diff_eq(Vec3::new(0.0508, 0.0762, 0.), 1e-5));
    }

    #[test]
    fn directional_ribbon_pivot_and_pi_rotation_anchor_its_front_at_origin() {
        let (geometry, _, _) = directional(
            Vec3::ZERO,
            Vec3::Z,
            Vec3::Y,
            Vec2::new(2., 3.),
            std::f32::consts::PI,
            [255; 4],
            Affine2::IDENTITY,
            [1., 1.],
            Vec2::new(0., 2.),
        )
        .unwrap();
        assert!(position(&geometry, 0).abs_diff_eq(Vec3::new(2., 0., 6.), 1e-5));
        assert!(position(&geometry, 2).abs_diff_eq(Vec3::new(-2., 0., 0.), 1e-5));
        // Sprite rotation changes the geometry, while the source normal stays -Y.
        for vertex in geometry.vertices.chunks_exact(32) {
            assert_eq!(
                u32::from_le_bytes(vertex[24..28].try_into().unwrap()),
                0x80400
            );
        }
    }

    #[test]
    fn directional_preserves_trim_atlas_winding_color_and_unit_normal() {
        let (geometry, _, _) = directional(
            Vec3::ZERO,
            Vec3::Z,
            Vec3::Y * 17.,
            Vec2::new(20., 30.),
            0.,
            [1, 2, 3, 4],
            Affine2::from_scale_angle_translation(Vec2::new(0.5, 0.25), 0., Vec2::new(0.25, 0.5)),
            [0.5, 1.],
            Vec2::ZERO,
        )
        .unwrap();
        assert!(position(&geometry, 1).abs_diff_eq(Vec3::new(-10., 0., 0.), 1e-4));
        assert_eq!(geometry.indices, [0, 0, 1, 0, 2, 0, 2, 0, 3, 0, 0, 0]);
        let expected_uv = [
            [0x3400u16, 0x3a00],
            [0x3600, 0x3800],
            [0x3900, 0x3800],
            [0x3a00, 0x3a00],
        ];
        for (vertex, uv) in geometry.vertices.chunks_exact(32).zip(expected_uv) {
            assert_eq!(&vertex[16..20], &[1, 2, 3, 4]);
            assert_eq!(
                u16::from_le_bytes(vertex[20..22].try_into().unwrap()),
                uv[0]
            );
            assert_eq!(
                u16::from_le_bytes(vertex[22..24].try_into().unwrap()),
                uv[1]
            );
            assert_eq!(
                u32::from_le_bytes(vertex[24..28].try_into().unwrap()),
                0x80400
            );
        }
    }

    #[test]
    fn directional_zero_and_eye_parallel_directions_keep_source_degeneracy() {
        for (direction, expected_first) in
            [(Vec3::ZERO, Vec3::ZERO), (Vec3::Y, Vec3::new(0., 0., -6.))]
        {
            let (geometry, _, count) = directional(
                Vec3::ZERO,
                Vec3::Y,
                direction,
                Vec2::new(2., 3.),
                0.,
                [255; 4],
                Affine2::IDENTITY,
                [1., 1.],
                Vec2::ZERO,
            )
            .unwrap();
            assert_eq!(count, 6);
            assert!(position(&geometry, 0).abs_diff_eq(expected_first, 1e-5));
            assert!(position(&geometry, 0).abs_diff_eq(position(&geometry, 3), 1e-5));
            for vertex in geometry.vertices.chunks_exact(32) {
                assert_eq!(u32::from_le_bytes(vertex[24..28].try_into().unwrap()), 0);
            }
        }
    }

    #[test]
    fn trail_preserves_source_uv_axes_normal_and_position_units() {
        let source = Trail {
            vertices: vec![TrailVertex {
                position: [1., 2.],
                normal: [1., 0.],
                texcoord: 0.5,
            }],
            indices: vec![0, 0],
            split_distance: 0.,
            repeat_distance: 1.,
            scroll_ms: 0.,
        };
        let pose = Transform::from_xyz(100., 200., 300.);
        let (geometry, _, _) = trail(
            &source,
            pose,
            pose.translation + Vec3::Z * 0.0254,
            Vec2::ONE,
            0.,
            [127, 127, 127, 255],
            Affine2::IDENTITY,
        )
        .unwrap();
        let first = &geometry.vertices[..32];
        let read = |offset| f32::from_le_bytes(first[offset..offset + 4].try_into().unwrap());
        assert!((read(0) - 1.).abs() < 0.001);
        assert!((read(4) + 1.).abs() < 0.001);
        assert!((read(8) + 2.).abs() < 0.001);
        // U runs along the trail; V is the source cross-section coordinate.
        assert_eq!(&first[20..24], &[0, 0, 0, 0x38]);
        assert_eq!(u32::from_le_bytes(first[24..28].try_into().unwrap()), 511);
        let last = &geometry.vertices[32..64];
        assert_eq!(u16::from_le_bytes(last[22..24].try_into().unwrap()), 0x3800);
        assert_ne!(u16::from_le_bytes(last[20..22].try_into().unwrap()), 0);
    }

    #[test]
    fn trail_normal_ignores_position_scale_and_tracks_rotation() {
        let source = Trail {
            vertices: vec![TrailVertex {
                position: [0., 0.],
                normal: [1., 0.],
                texcoord: 0.,
            }],
            indices: vec![0, 0],
            split_distance: 0.,
            repeat_distance: 1.,
            scroll_ms: 0.,
        };
        let pose = Transform::from_scale(Vec3::splat(10.));
        let (geometry, _, _) = trail(
            &source,
            pose,
            Vec3::ZERO,
            Vec2::ONE,
            std::f32::consts::FRAC_PI_2,
            [127, 127, 127, 255],
            Affine2::IDENTITY,
        )
        .unwrap();
        let normal = u32::from_le_bytes(geometry.vertices[24..28].try_into().unwrap());
        assert_eq!(normal & 0x3ff, 0);
        assert_eq!((normal >> 20) & 0x3ff, 511);
    }
}

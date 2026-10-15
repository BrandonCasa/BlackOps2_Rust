//! Original PACKED32 particle inputs; no Bevy material or mesh adapter.
use super::Trail;
use crate::rendering::native_effect::NativeEffectGeometry;
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

fn vertex(bytes: &mut Vec<u8>, position: Vec3, color: [f32; 4], uv: Vec2) {
    for value in [position.x, position.y, position.z, -1.] {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    // 005b4da0 lightingFrac=0: source RGB bytes shift right one bit; alpha
    // retains its original byte. Nonzero source lighting is rejected by owner.
    let color = color.map(|value| value.clamp(0., 255.) as u8);
    bytes.extend_from_slice(&[color[0] >> 1, color[1] >> 1, color[2] >> 1, color[3]]);
    bytes.extend_from_slice(&packed_half(uv.x).to_le_bytes());
    bytes.extend_from_slice(&packed_half(uv.y).to_le_bytes());
    // The recovered zfeather program does not consume normal or tangent.
    bytes.extend_from_slice(&[0; 8]);
}

pub(super) fn quad(
    color: [f32; 4],
    uv: Affine2,
    trim: [f32; 2],
) -> (NativeEffectGeometry, Aabb, u32) {
    let mut vertices = Vec::with_capacity(128);
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
        vertex(
            &mut vertices,
            position,
            color,
            uv.transform_point2(texcoord),
        );
    }
    let indices = [0u16, 1, 2, 2, 3, 0]
        .into_iter()
        .flat_map(u16::to_le_bytes)
        .collect();
    (
        NativeEffectGeometry { vertices, indices },
        Aabb::from_min_max(
            Vec3::new(-trim[0].abs().max(trim[1].abs()) * 0.5, -0.5, 0.),
            Vec3::new(trim[0].abs().max(trim[1].abs()) * 0.5, 0.5, 0.),
        ),
        6,
    )
}

pub(super) fn trail(
    source: &Trail,
    pose: Transform,
    start: Vec3,
    size: Vec2,
    color: [f32; 4],
    uv: Affine2,
) -> Result<(NativeEffectGeometry, Aabb, u32), String> {
    let local_start = pose.rotation.inverse() * (start - pose.translation);
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
                + Vec3::new(
                    source_vertex.position[0] * size.x * 0.0254,
                    source_vertex.position[1] * size.y * 0.0254,
                    0.,
                );
            minimum = minimum.min(position);
            maximum = maximum.max(position);
            let texcoord = Vec2::new(
                source_vertex.texcoord,
                if source.repeat_distance > 0. {
                    fraction * length / (source.repeat_distance * 0.0254)
                } else {
                    fraction
                },
            );
            vertex(
                &mut vertices,
                position,
                color,
                uv.transform_point2(texcoord),
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
        NativeEffectGeometry { vertices, indices },
        Aabb::from_min_max(minimum, maximum),
        index_count,
    ))
}

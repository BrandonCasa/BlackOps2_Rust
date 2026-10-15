// PACKED32 deformation inputs from 00479a60 and 0047acf0/0047b150/
// 0047b6d0/0047bd80. Matrices are source-space DObjSkelMat columns.
// The caller dispatches one rigid range or one influence-count bucket at a time.
// This is the base deformation stage. Source tension correction (004798c0)
// is not required by the exported models and is rejected by the asset loader.
struct SkinRange {
    first_vertex: u32,
    vertex_count: u32,
    first_blend_u16: u32,
    influence_count: u32, // zero denotes a rigid list
    rigid_bone: u32,
    matrix_offset: u32,
    output_vertex: u32,
    padding: u32,
}

@group(0) @binding(0) var<storage, read> source_vertices: array<u32>;
@group(0) @binding(1) var<storage, read> source_blends: array<u32>;
@group(0) @binding(2) var<storage, read> bone_matrices: array<mat4x4<f32>>;
@group(0) @binding(3) var<storage, read_write> skinned_vertices: array<u32>;
@group(0) @binding(4) var<uniform> range: SkinRange;

fn blend_u16(index: u32) -> u32 {
    return (source_blends[index / 2u] >> ((index & 1u) * 16u)) & 65535u;
}

fn unpack_direction(packed: u32) -> vec3<f32> {
    // Native masks retain each lane's shift before float conversion.
    let lanes = (vec3<u32>(packed) & vec3<u32>(0x3ffu, 0xffc00u, 0x3ff00000u))
        ^ vec3<u32>(0x200u, 0x80000u, 0x20000000u);
    return (vec3<f32>(lanes) - vec3<f32>(512.0, 524288.0, 536870912.0))
        * bitcast<vec3<f32>>(vec3<u32>(0x3b004020u, 0x36004020u, 0x31004020u));
}

fn round_even(value: vec3<f32>) -> vec3<i32> {
    // CVTPS2DQ uses nearest-even in the original routine. State ties explicitly.
    let lower = floor(value);
    let fraction = value - lower;
    let increment = (fraction > vec3<f32>(0.5))
        | ((fraction == vec3<f32>(0.5)) & ((vec3<i32>(lower) & vec3<i32>(1)) != vec3<i32>(0)));
    return vec3<i32>(lower) + select(vec3<i32>(0), vec3<i32>(1), increment);
}

fn pack_direction(direction: vec3<f32>) -> u32 {
    // Do not normalize. Native conversion rounds shifted lanes, then masks;
    // rounding an unshifted value and shifting afterward changes Y/Z results.
    let scaled = clamp(direction, vec3<f32>(-1.0), vec3<f32>(1.0))
        * vec3<f32>(511.0, 523264.0, 535822336.0);
    let packed = bitcast<vec3<u32>>(round_even(scaled))
        & vec3<u32>(0x3ffu, 0xffc00u, 0x3ff00000u);
    return packed.x | packed.y | packed.z;
}

fn skin_matrix(vertex: u32) -> mat4x4<f32> {
    if range.influence_count == 0u {
        return bone_matrices[range.matrix_offset + range.rigid_bone];
    }
    let start = range.first_blend_u16 + vertex * (2u * range.influence_count - 1u);
    var weights = array<f32, 4>(1.0, 0.0, 0.0, 0.0);
    // 00c5def0 is exactly 2^-16, not UNORM16's 1/65535.
    for (var i = range.influence_count - 1u; i > 0u; i--) {
        weights[i] = f32(blend_u16(start + i * 2u)) * (1.0 / 65536.0);
        weights[0] = weights[0] - weights[i];
    }
    var matrix = bone_matrices[range.matrix_offset + blend_u16(start) / 64u] * weights[0];
    for (var i = 1u; i < range.influence_count; i++) {
        matrix += bone_matrices[range.matrix_offset + blend_u16(start + i * 2u - 1u) / 64u] * weights[i];
    }
    return matrix;
}

@compute @workgroup_size(64)
fn skin(@builtin(global_invocation_id) invocation: vec3<u32>) {
    let vertex = invocation.x;
    if vertex >= range.vertex_count { return; }
    let input = (range.first_vertex + vertex) * 8u;
    let output = (range.output_vertex + vertex) * 8u;
    let matrix = skin_matrix(vertex);
    let position = bitcast<vec3<f32>>(vec3<u32>(source_vertices[input],
        source_vertices[input + 1u], source_vertices[input + 2u]));
    let posed = matrix * vec4<f32>(position, 1.0);
    skinned_vertices[output] = bitcast<u32>(posed.x);
    skinned_vertices[output + 1u] = bitcast<u32>(posed.y);
    skinned_vertices[output + 2u] = bitcast<u32>(posed.z);
    // Original binormal sign, color, and packed half-float UV remain untouched.
    for (var i = 3u; i < 6u; i++) { skinned_vertices[output + i] = source_vertices[input + i]; }
    for (var i = 6u; i < 8u; i++) {
        let direction = matrix * vec4<f32>(unpack_direction(source_vertices[input + i]), 0.0);
        skinned_vertices[output + i] = pack_direction(direction.xyz);
    }
}

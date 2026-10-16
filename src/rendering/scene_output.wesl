// Full-resolution encoded source film input. Native accumulation is UNORM8;
// optional DLSS reconstructs E*E in HDR and must return to E before bloom.
@group(0) @binding(0) var source_scene: texture_2d<f32>;

struct FragmentInput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@fragment
fn fragment(input: FragmentInput) -> @location(0) vec4<f32> {
    return textureLoad(source_scene, vec2<i32>(input.position.xy), 0);
}

// Reencode each reconstructed texel before any encoded-domain filtering.
@fragment
fn fragment_reencode(input: FragmentInput) -> @location(0) vec4<f32> {
    let linear = textureLoad(source_scene, vec2<i32>(input.position.xy), 0);
    return vec4<f32>(sqrt(max(linear.rgb, vec3<f32>(0.0))), linear.a);
}

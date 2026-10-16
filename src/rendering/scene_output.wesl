// The native scene already contains encoded RGB. Preserve its pixels; only
// compensate for an actual sRGB output attachment's automatic write transfer.
// Original film/LUT/bloom belong before this final display adapter.
@group(0) @binding(0) var source_scene: texture_2d<f32>;

struct FragmentInput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@fragment
fn fragment(input: FragmentInput) -> @location(0) vec4<f32> {
    return textureLoad(source_scene, vec2<i32>(input.position.xy), 0);
}

// DLSS consumes linear color. Encode only when the output attachment does not.
@fragment
fn fragment_encode(input: FragmentInput) -> @location(0) vec4<f32> {
    let linear = textureLoad(source_scene, vec2<i32>(input.position.xy), 0);
    let encoded = select(linear.rgb * vec3<f32>(12.92),
        vec3<f32>(1.055) * pow(max(linear.rgb, vec3<f32>(0.0)), vec3<f32>(1.0 / 2.4)) - vec3<f32>(0.055),
        linear.rgb > vec3<f32>(0.0031308));
    return vec4<f32>(encoded, linear.a);
}

@fragment
fn fragment_srgb(input: FragmentInput) -> @location(0) vec4<f32> {
    let encoded = textureLoad(source_scene, vec2<i32>(input.position.xy), 0);
    let linear = select(encoded.rgb / vec3<f32>(12.92),
        pow((encoded.rgb + vec3<f32>(0.055)) / vec3<f32>(1.055), vec3<f32>(2.4)),
        encoded.rgb > vec3<f32>(0.04045));
    return vec4<f32>(linear, encoded.a);
}

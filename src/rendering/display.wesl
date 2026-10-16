// Intended native monitor ramp (00436330 / 0043c530), after film and all UI.
// This continuous shader emulation does not modify the operating-system ramp.
@group(0) @binding(0) var composed: texture_2d<f32>;
@group(0) @binding(1) var<uniform> brightness_offset: vec4<f32>;
struct Input { @builtin(position) position: vec4<f32>, @location(0) uv: vec2<f32> }
fn display_linear(input: Input) -> vec4<f32> {
    let color = textureLoad(composed, vec2<i32>(input.position.xy), 0);
    let u = clamp(color.rgb + vec3<f32>(brightness_offset.x), vec3<f32>(0.0), vec3<f32>(1.0));
    return vec4<f32>(u * u, color.a);
}
// The actual sRGB attachment applies the standard forward transfer once.
@fragment fn fragment_srgb(input: Input) -> @location(0) vec4<f32> {
    return display_linear(input);
}
// Ordinary T6's swapchain is UNORM. Apply that same forward transfer explicitly.
@fragment fn fragment_unorm(input: Input) -> @location(0) vec4<f32> {
    let color = display_linear(input);
    let encoded = select(color.rgb * vec3<f32>(12.92),
        vec3<f32>(1.055) * pow(color.rgb, vec3<f32>(1.0 / 2.4)) - vec3<f32>(0.055),
        color.rgb > vec3<f32>(0.0031308));
    return vec4<f32>(encoded, color.a);
}

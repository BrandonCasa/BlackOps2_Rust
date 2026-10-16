// Ordinary bloom_apply PS5b71154. Output remains encoded for source UI blends.
@group(0) @binding(0) var scene: texture_2d<f32>;
@group(0) @binding(1) var linear_clamp: sampler;
@group(0) @binding(2) var bloom: texture_2d<f32>;
@group(0) @binding(3) var generated_lut: texture_2d<f32>;
struct Input { @builtin(position) position: vec4<f32>, @location(0) uv: vec2<f32> }
@fragment fn apply(input: Input) -> @location(0) vec4<f32> {
    let encoded = textureSample(scene, linear_clamp, input.uv).rgb;
    let light = textureSample(bloom, linear_clamp, input.uv).rgb;
    let restored = sqrt(fma(encoded * encoded, vec3<f32>(4.0), light));
    let shoulder = fma(vec3<f32>(1.0) - exp2(fma(restored, vec3<f32>(-5.770780086517334), vec3<f32>(4.328084945678711))), vec3<f32>(0.25), vec3<f32>(0.75));
    let h = select(restored, shoulder, restored > vec3<f32>(0.75));
    let blue = h.b * 31.0;
    let uv = vec2<f32>((floor(blue) * 32.0 + h.r * 31.0 + 0.5) / 1024.0, (h.g * 31.0 + 0.5) / 32.0);
    let a = textureSample(generated_lut, linear_clamp, uv).rgb;
    let b = textureSample(generated_lut, linear_clamp, uv + vec2<f32>(0.03125, 0.0)).rgb;
    let result = fma(vec3<f32>(fract(blue)), b - a, a);
    return vec4<f32>(result, dot(result, vec3<f32>(0.299000, 0.587000, 0.114000)));
}

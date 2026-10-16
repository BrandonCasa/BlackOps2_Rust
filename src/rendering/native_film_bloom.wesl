// Ordinary 00460530 bloom: PS415b15ca/c252223e/7dedbf9c/b147bf02/2620090.
struct BloomConstants {
    thresholds: vec4<f32>, input_scale: vec4<f32>, input_bias: vec4<f32>,
    gamma: vec4<f32>, output_scale: vec4<f32>, output_bias: vec4<f32>,
    rgb_high: vec4<f32>, rgb_low: vec4<f32>, y_high: vec4<f32>, y_low: vec4<f32>,
    step: vec4<f32>,
}
@group(0) @binding(0) var source_image: texture_2d<f32>;
@group(0) @binding(1) var linear_clamp: sampler;
@group(0) @binding(2) var<uniform> controls: BloomConstants;
@group(0) @binding(3) var low_image: texture_2d<f32>;
struct Input { @builtin(position) position: vec4<f32>, @location(0) uv: vec2<f32> }

// D3D saturate maps NaN to zero, including the authored zero-width Y threshold.
fn source_saturate(value: vec4<f32>) -> vec4<f32> {
    return select(clamp(value, vec4<f32>(0.0), vec4<f32>(1.0)), vec4<f32>(0.0), value != value);
}
fn four_taps(uv: vec2<f32>, offset: vec2<f32>) -> vec4<f32> {
    let a = textureSample(source_image, linear_clamp, uv - offset);
    let b = textureSample(source_image, linear_clamp, uv + vec2<f32>(offset.x, -offset.y));
    let c = textureSample(source_image, linear_clamp, uv + vec2<f32>(-offset.x, offset.y));
    let d = textureSample(source_image, linear_clamp, uv + offset);
    return ((a + b) + c) + d;
}
@fragment fn downsample(input: Input) -> @location(0) vec4<f32> {
    // Filter in the original encoded domain before squaring.
    let encoded = four_taps(input.uv, controls.step.xy).rgb * 0.25;
    let linear = (encoded * encoded) * 4.0;
    let y = dot(linear, vec3<f32>(0.21258544921875, 0.7151947021484375, 0.0722198486328125));
    let t = controls.thresholds;
    let reciprocal = vec4<f32>(1.0) / (t.zzzw - t.xxxy);
    let ramp = source_saturate((vec4<f32>(y) - t.xxxy) * reciprocal);
    let eased = (ramp * ramp) * fma(ramp.zzzw, vec4<f32>(-2.0), vec4<f32>(3.0));
    let mapped = source_saturate(fma(vec4<f32>(linear, y) * eased, controls.input_scale, controls.input_bias));
    return fma(exp2(log2(mapped) * controls.gamma), controls.output_scale, controls.output_bias);
}
@fragment fn convolution(input: Input) -> @location(0) vec4<f32> {
    return four_taps(input.uv, controls.step.xy * 0.75) * 0.25;
}
fn blur(uv: vec2<f32>, axis: vec2<f32>, perpendicular: vec2<f32>) -> vec4<f32> {
    let center = uv + perpendicular * 0.5;
    // Preserve native accumulation order and all RGBA lanes, including luma.
    var result = textureSample(source_image, linear_clamp, center + axis * -2.299999952316284) * 0.234375;
    result = fma(textureSample(source_image, linear_clamp, center + axis * -4.099999904632568), vec4<f32>(0.01953125), result);
    result = fma(textureSample(source_image, linear_clamp, center + axis * -0.5), vec4<f32>(0.4921875), result);
    result = fma(textureSample(source_image, linear_clamp, center + axis * 1.2999999523162842), vec4<f32>(0.234375), result);
    return fma(textureSample(source_image, linear_clamp, center + axis * 3.0999999046325684), vec4<f32>(0.01953125), result);
}
@fragment fn blur_y(input: Input) -> @location(0) vec4<f32> {
    return blur(input.uv, vec2<f32>(0.0, controls.step.y), vec2<f32>(controls.step.x, 0.0));
}
@fragment fn blur_x(input: Input) -> @location(0) vec4<f32> {
    return blur(input.uv, vec2<f32>(controls.step.x, 0.0), vec2<f32>(0.0, controls.step.y));
}
@fragment fn combine(input: Input) -> @location(0) vec4<f32> {
    let low = textureSample(low_image, linear_clamp, input.uv);
    let high = textureSample(source_image, linear_clamp, input.uv);
    let rgb = fma(high.rgb, controls.rgb_high.rgb, low.rgb * controls.rgb_low.rgb);
    let y = fma(vec3<f32>(high.a), controls.y_high.rgb, low.a * controls.y_low.rgb);
    return vec4<f32>(rgb + y, 1.0);
}

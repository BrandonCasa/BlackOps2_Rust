import package::rendering::native_fx::NativeFxFrame;
import package::rendering::native_cloud::{NativeCloudObject, NativeCloudVaryings,
    native_cloud_rain_vertex, native_cloud_snow_vertex, native_cloud_fragment};

@group(0) @binding(0) var<uniform> frame: NativeFxFrame;
@group(0) @binding(1) var outdoor_map: texture_2d<f32>;
@group(0) @binding(2) var outdoor_sampler: sampler;
@group(1) @binding(0) var<uniform> object: NativeCloudObject;
@group(2) @binding(0) var color_map: texture_2d<f32>;
@group(2) @binding(1) var color_sampler: sampler;

struct Output {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) outdoor: vec3<f32>,
}

fn output(source: NativeCloudVaryings) -> Output {
    return Output(source.clip_position, source.uv, source.outdoor);
}

@vertex
fn vertex_rain(@location(0) packed: vec4<f32>) -> Output {
    return output(native_cloud_rain_vertex(packed, frame, object));
}

@vertex
fn vertex_snow(@location(0) packed: vec4<f32>) -> Output {
    return output(native_cloud_snow_vertex(packed, frame, object));
}

@fragment
fn fragment(input: Output) -> @location(0) vec4<f32> {
    let result = native_cloud_fragment(NativeCloudVaryings(input.position, input.uv, input.outdoor),
        object, color_map, color_sampler, outdoor_map, outdoor_sampler);
    // Both original material states use alphaTest=gt0.
    if result.w <= 0.0 { discard; }
    return result;
}

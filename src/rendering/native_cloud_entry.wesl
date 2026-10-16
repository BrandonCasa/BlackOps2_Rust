import package::rendering::native_fx::NativeFxFrame;
import package::rendering::native_cloud::{NativeCloudObject, NativeCloudVaryings,
    native_cloud_rain_vertex, native_cloud_snow_vertex, native_cloud_fragment,
    native_cloud_simple_vertex, native_cloud_simple_fragment, native_cloud_additive_fragment};

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

struct SimpleOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
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

@vertex
fn vertex_simple(@location(0) packed: vec4<f32>) -> SimpleOutput {
    let source = native_cloud_simple_vertex(packed, frame, object);
    return SimpleOutput(source.clip_position, source.uv);
}

// particlecloud_zwwzq468: VSbb0a1227, PS c6c75f18.
@vertex
fn vertex_vertical(@location(0) packed: vec4<f32>) -> SimpleOutput {
    let corners = array<vec2<f32>, 4>(vec2<f32>(0.0,0.0), vec2<f32>(0.0,1.0),
        vec2<f32>(1.0,0.0), vec2<f32>(1.0,1.0));
    let uv = corners[i32(packed.w * 4.0)];
    let corner = uv - vec2<f32>(0.5);
    let local = vec4<f32>(packed.xyz, 1.0);
    let world = vec4<f32>(dot(local, object.world_rows[0]), dot(local, object.world_rows[1]),
        dot(local, object.world_rows[2]), dot(local, object.world_rows[3]));
    let reciprocal_radius = inverseSqrt(dot(-world.xy, -world.xy));
    let side = vec3<f32>(-world.y * reciprocal_radius, 0.0, -world.x * reciprocal_radius);
    let up = object.view_up.xyz;
    let width = fma(up.yzx, side.yzx, -(side * up.zxy)) * (corner.x * object.cloud_matrix.x);
    let position = vec4<f32>(world.xyz + width + up * (corner.y * object.cloud_matrix.y), world.w);
    let rows = frame.scene.view_projection_rows;
    return SimpleOutput(vec4<f32>(dot(position, rows[0]), dot(position, rows[1]),
        dot(position, rows[2]), dot(position, rows[3])), uv);
}

@fragment
fn fragment_simple(input: SimpleOutput) -> @location(0) vec4<f32> {
    let result = native_cloud_simple_fragment(input.uv, object, color_map, color_sampler);
    // All particlecloud_9337w7f6 materials use alphaTest=gt0.
    if result.w <= 0.0 { discard; }
    return result;
}

@fragment
fn fragment_additive(input: SimpleOutput) -> @location(0) vec4<f32> {
    // All particlecloud_w7046j83 materials disable alpha testing.
    return native_cloud_additive_fragment(input.uv, object, color_map, color_sampler);
}

@fragment
fn fragment(input: Output) -> @location(0) vec4<f32> {
    let result = native_cloud_fragment(NativeCloudVaryings(input.position, input.uv, input.outdoor),
        object, color_map, color_sampler, outdoor_map, outdoor_sampler);
    // Both original material states use alphaTest=gt0.
    if result.w <= 0.0 { discard; }
    return result;
}

// particlecloud_zqe90390: VS102ffd67 / PScda0769c.
@fragment
fn fragment_outdoor_additive(input: Output) -> @location(0) vec4<f32> {
    let result = native_cloud_fragment(NativeCloudVaryings(input.position, input.uv, input.outdoor),
        object, color_map, color_sampler, outdoor_map, outdoor_sampler);
    return vec4<f32>(result.rgb * result.a, result.a);
}

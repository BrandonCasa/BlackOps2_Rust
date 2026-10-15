import package::rendering::native_model::{
    NativeModelScene, NativeModelObject, NativeModelVertexInput,
    NativeModelVaryings, native_model_vertex, native_model_fragment,
};

@group(0) @binding(0) var<uniform> scene: NativeModelScene;
@group(1) @binding(0) var<uniform> object: NativeModelObject;
@group(2) @binding(0) var color_map: texture_2d<f32>;
@group(2) @binding(1) var color_sampler: sampler;
@group(2) @binding(2) var normal_map: texture_2d<f32>;
@group(2) @binding(3) var normal_sampler: sampler;
@group(2) @binding(4) var reflection_probe: texture_cube<f32>;
@group(2) @binding(5) var reflection_sampler: sampler;
@group(3) @binding(0) var model_lighting: texture_3d<f32>;
@group(3) @binding(1) var model_lighting_sampler: sampler;

struct VertexInput {
    @location(0) position: vec4<f32>,
    @location(1) color: vec4<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) packed_normal: vec4<f32>,
    @location(4) packed_tangent: vec4<f32>,
}

struct Varyings {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) normal_fog: vec4<f32>,
    @location(3) fog_color: vec3<f32>,
    @location(4) bitangent: vec3<f32>,
    @location(5) tangent: vec3<f32>,
    @location(6) eye_to_vertex: vec4<f32>,
    @location(7) grid_coords_and_vis: vec4<f32>,
    @location(8) reflection_correction: vec3<f32>,
}

@vertex
fn vertex(input: VertexInput) -> Varyings {
    let source = NativeModelVertexInput(input.position, input.color, input.uv,
        input.packed_normal.xyz, input.packed_tangent.xyz);
    let result = native_model_vertex(source, scene, object);
    return Varyings(result.clip_position, result.color, result.uv,
        result.normal_fog, result.fog_color, result.bitangent, result.tangent,
        result.eye_to_vertex, result.grid_coords_and_vis, result.reflection_correction);
}

fn shade(input: Varyings) -> vec4<f32> {
    let source = NativeModelVaryings(input.clip_position, input.color, input.uv,
        input.normal_fog, input.fog_color, input.bitangent, input.tangent,
        input.eye_to_vertex, input.grid_coords_and_vis, input.reflection_correction);
    return native_model_fragment(source, scene, color_map, color_sampler,
        normal_map, normal_sampler, model_lighting, model_lighting_sampler,
        reflection_probe, reflection_sampler);
}

@fragment
fn fragment(input: Varyings) -> @location(0) vec4<f32> {
    return shade(input);
}

// The original shader already encodes its result. Undo the attachment transfer
// only when the actual color attachment will apply an sRGB encode itself.
@fragment
fn fragment_srgb(input: Varyings) -> @location(0) vec4<f32> {
    let encoded = shade(input);
    let linear = select(encoded.rgb / vec3<f32>(12.92),
        pow((encoded.rgb + vec3<f32>(0.055)) / vec3<f32>(1.055), vec3<f32>(2.4)),
        encoded.rgb > vec3<f32>(0.04045));
    return vec4<f32>(linear, encoded.a);
}

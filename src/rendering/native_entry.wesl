import package::rendering::native_world::{
    NativeWorldScene, NativeWorldObject, NativeWorldVertexInput,
    NativeWorldVaryings, native_world_vertex, native_world_fragment,
};

@group(0) @binding(0) var<uniform> scene: NativeWorldScene;
@group(1) @binding(0) var<uniform> object: NativeWorldObject;
@group(2) @binding(0) var color_map: texture_2d<f32>;
@group(2) @binding(1) var color_sampler: sampler;
@group(2) @binding(2) var normal_map: texture_2d<f32>;
@group(2) @binding(3) var normal_sampler: sampler;
@group(2) @binding(4) var lightmap_secondary: texture_2d<f32>;
@group(2) @binding(5) var lightmap_sampler: sampler;
@group(2) @binding(6) var reflection_probe: texture_cube<f32>;
@group(2) @binding(7) var reflection_sampler: sampler;

struct VertexInput {
    @location(0) position: vec4<f32>,
    @location(1) color: vec4<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) packed_normal: vec4<f32>,
    @location(4) packed_tangent: vec4<f32>,
    @location(5) lightmap_uv: vec2<f32>,
}

struct Varyings {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) uv_lightmap: vec4<f32>,
    @location(2) normal_fog: vec4<f32>,
    @location(3) fog_color: vec3<f32>,
    @location(4) bitangent: vec3<f32>,
    @location(5) tangent: vec3<f32>,
    @location(6) eye_to_vertex: vec3<f32>,
    @location(7) reflection_correction: vec3<f32>,
}

@vertex
fn vertex(input: VertexInput) -> Varyings {
    let source = NativeWorldVertexInput(input.position, input.color, input.uv,
        input.packed_normal.xyz, input.packed_tangent.xyz, input.lightmap_uv);
    let result = native_world_vertex(source, scene, object);
    return Varyings(result.clip_position, result.color, result.uv_lightmap,
        result.normal_fog, result.fog_color, result.bitangent, result.tangent,
        result.eye_to_vertex, result.reflection_correction);
}

fn shade(input: Varyings) -> vec4<f32> {
    let source = NativeWorldVaryings(input.clip_position, input.color,
        input.uv_lightmap, input.normal_fog, input.fog_color, input.bitangent,
        input.tangent, input.eye_to_vertex, input.reflection_correction);
    return native_world_fragment(source, scene, color_map, color_sampler,
        normal_map, normal_sampler, lightmap_secondary, lightmap_sampler,
        reflection_probe, reflection_sampler);
}

@fragment
fn fragment(input: Varyings) -> @location(0) vec4<f32> {
    return shade(input);
}

// The source result is already encoded. An sRGB attachment encodes writes, so
// undo that transfer here only for that target; preserve the source alpha.
@fragment
fn fragment_srgb(input: Varyings) -> @location(0) vec4<f32> {
    let encoded = shade(input);
    let linear = select(encoded.rgb / vec3<f32>(12.92),
        pow((encoded.rgb + vec3<f32>(0.055)) / vec3<f32>(1.055), vec3<f32>(2.4)),
        encoded.rgb > vec3<f32>(0.04045));
    return vec4<f32>(linear, encoded.a);
}

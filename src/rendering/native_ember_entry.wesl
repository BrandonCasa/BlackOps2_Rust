import package::rendering::native_world::{NativeWorldScene, NativeWorldObject, NativeWorldVertexInput};
import package::rendering::native_world_ember::{NativeEmberMaterial, NativeEmberLighting,
    NativeEmberVaryings, native_ember_vertex, native_ember_fragment};

@group(0) @binding(0) var<uniform> scene: NativeWorldScene;
@group(1) @binding(0) var<uniform> object: NativeWorldObject;
@group(2) @binding(0) var diffuse_map: texture_2d<f32>;
@group(2) @binding(1) var diffuse_sampler: sampler;
@group(2) @binding(2) var normal_map: texture_2d<f32>;
@group(2) @binding(3) var normal_sampler: sampler;
@group(2) @binding(4) var lightmap: texture_2d<f32>;
@group(2) @binding(5) var lightmap_sampler: sampler;
@group(2) @binding(6) var probe: texture_cube<f32>;
@group(2) @binding(7) var probe_sampler: sampler;
@group(2) @binding(8) var super_map: texture_2d<f32>;
@group(2) @binding(9) var super_sampler: sampler;
@group(2) @binding(10) var ember_map: texture_2d<f32>;
@group(2) @binding(11) var ember_sampler: sampler;
@group(2) @binding(12) var<uniform> material: NativeEmberMaterial;

struct Input {
    @location(0) position: vec4<f32>,
    @location(1) color: vec4<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) normal: vec4<f32>,
    @location(4) tangent: vec4<f32>,
    @location(5) lightmap_uv: vec2<f32>,
}
struct Varyings {
    @builtin(position) position: vec4<f32>,
    @location(0) fog_flicker: vec4<f32>,
    @location(1) eye_fog: vec4<f32>,
    @location(2) color: vec4<f32>,
    @location(3) normal: vec3<f32>,
    @location(4) tangent_sign: vec4<f32>,
    @location(5) uv_lightmap: vec4<f32>,
    @location(6) reflection_correction: vec3<f32>,
}
@vertex
fn vertex(input: Input) -> Varyings {
    let output = native_ember_vertex(NativeWorldVertexInput(input.position, input.color,
        input.uv, input.normal.xyz, input.tangent.xyz, input.lightmap_uv), scene, object, material);
    return Varyings(output.clip_position, output.fog_flicker, output.eye_fog, output.color,
        output.normal, output.tangent_sign, output.uv_lightmap, output.reflection_correction);
}
fn shade(input: Varyings) -> vec4<f32> {
    // 00416380 uses 6500K and r_heroLightScale=(1,1,1). The original matrix
    // product quantized by 0074d390 is identity (ember-hero-lighting.json).
    let lighting = NativeEmberLighting(scene.sun_position, scene.sun_diffuse,
        array<vec4<f32>, 3>(vec4<f32>(1.0,0.0,0.0,0.0),
            vec4<f32>(0.0,1.0,0.0,0.0), vec4<f32>(0.0,0.0,1.0,0.0)));
    return native_ember_fragment(NativeEmberVaryings(input.position, input.fog_flicker,
        input.eye_fog, input.color, input.normal, input.tangent_sign, input.uv_lightmap,
        input.reflection_correction), scene, lighting, material,
        diffuse_map, diffuse_sampler, normal_map, normal_sampler, lightmap, lightmap_sampler,
        probe, probe_sampler, super_map, super_sampler, ember_map, ember_sampler);
}
@fragment
fn fragment(input: Varyings) -> @location(0) vec4<f32> { return shade(input); }
@fragment
fn fragment_srgb(input: Varyings) -> @location(0) vec4<f32> {
    let encoded = shade(input);
    let linear = select(encoded.rgb / vec3<f32>(12.92),
        pow((encoded.rgb + vec3<f32>(0.055)) / vec3<f32>(1.055), vec3<f32>(2.4)),
        encoded.rgb > vec3<f32>(0.04045));
    return vec4<f32>(linear, encoded.a);
}

// Original plinth EMISSIVE: pixel parallax, vertex color, floatZ fade, no probe or fog.
import package::rendering::native_model::{NativeModelScene, NativeModelObject, NativeModelVertexInput};
import package::rendering::native_scripted_model::{
    NativeModelScriptedScene, NativeModelScriptInputs, NativeModelParallaxConstants,
    NativeScriptedVaryings, native_scripted_model_vertex,
    native_scripted_parallax_basis, native_scripted_three_layer_uvs,
};
import package::rendering::native_fx::NativeFxFrame;
@group(0) @binding(0) var<uniform> scene: NativeModelScene;
@group(0) @binding(1) var<uniform> view: NativeModelScriptedScene;
@group(1) @binding(0) var<uniform> object: NativeModelObject;
@group(1) @binding(1) var<uniform> scripts: NativeModelScriptInputs;
@group(2) @binding(0) var layer1_map: texture_2d<f32>;
@group(2) @binding(1) var layer1_sampler: sampler;
@group(2) @binding(2) var layer2_map: texture_2d<f32>;
@group(2) @binding(3) var layer2_sampler: sampler;
@group(2) @binding(4) var layer3_map: texture_2d<f32>;
@group(2) @binding(5) var layer3_sampler: sampler;
@group(2) @binding(12) var<uniform> material: NativeModelParallaxConstants;
@group(3) @binding(0) var<uniform> frame: NativeFxFrame;
@group(3) @binding(1) var float_z: texture_2d<f32>;
@group(3) @binding(2) var float_z_sampler: sampler;
struct VertexInput {
    @location(0) position: vec4<f32>,
    @location(1) color: vec4<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) normal: vec4<f32>,
    @location(4) tangent: vec4<f32>,
}
struct Varyings {
    @builtin(position) position: vec4<f32>,
    @location(0) eye: vec3<f32>,
    @location(1) color: vec4<f32>,
    @location(2) normal: vec3<f32>,
    @location(3) tangent_sign: vec4<f32>,
    @location(4) uv: vec2<f32>,
    @location(5) eye_distance: f32,
}
@vertex
fn vertex(input: VertexInput) -> Varyings {
    let result = native_scripted_model_vertex(NativeModelVertexInput(input.position,
        input.color, input.uv, input.normal.xyz, input.tangent.xyz), scene, object);
    return Varyings(result.clip_position, result.eye_fog.xyz, input.color,
        result.normal, result.tangent_sign, input.uv, result.clip_position.w);
}
fn shade(input: Varyings) -> vec4<f32> {
    let source = NativeScriptedVaryings(input.position, vec3<f32>(0.0), vec4<f32>(input.eye, 1.0),
        input.color, input.normal, input.tangent_sign, input.uv, vec4<f32>(0.0), vec3<f32>(0.0), vec3<f32>(0.0));
    let basis = native_scripted_parallax_basis(source);
    let c = material.source;
    let uv = native_scripted_three_layer_uvs(input.uv, basis.ray, material, view.game_time.w);
    let layer3 = textureSample(layer3_map, layer3_sampler, uv.layer3);
    let layer2 = textureSample(layer2_map, layer2_sampler, uv.layer2);
    let layer1 = textureSample(layer1_map, layer1_sampler, uv.layer1);
    let gates = vec3<f32>(1.0) - clamp(scripts.vectors[0].xyz, vec3<f32>(0.0), vec3<f32>(1.0));
    let alpha2 = gates.y * c[4].w * layer2.a;
    let alpha1 = gates.x * c[4].z * layer1.a;
    let combined2 = fma(vec3<f32>(alpha2), fma(-layer3.rgb, vec3<f32>(layer3.a), layer2.rgb), layer3.a * layer3.rgb);
    let composed = fma(vec3<f32>(alpha1), layer1.rgb - combined2, combined2);
    let coverage2 = clamp(fma(layer3.a, gates.z * c[5].x, alpha2), 0.0, 1.0);
    let coverage1 = clamp(fma(layer1.a, gates.x * c[4].z, coverage2), 0.0, 1.0);
    let colored = vec4<f32>(composed, coverage1) * input.color;
    let depth = textureSample(float_z, float_z_sampler, input.position.xy / frame.render_target_size.xy).r;
    let scene_distance = frame.z_near.x / max(abs(depth), 0.0);
    let fade = clamp((scene_distance - input.eye_distance) * (1.0 / max(c[10].x, 0.00009999999747378752)), 0.0, 1.0);
    let faded = ((fade * input.color.a) * colored.a) * colored.rgb;
    let eye = input.eye * inverseSqrt(dot(input.eye, input.eye));
    let facing = clamp(dot(-eye, basis.normal), 0.0, 1.0);
    let falloff = exp2(log2(facing) * c[9].z) * c[9].w;
    let gamma = (falloff * c[9].y) * faded;
    return vec4<f32>(sqrt((gamma * gamma) * scene.world.hdr_control0.x), 1.0);
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

// Smoke pillar VS cd848ad4 / PS f53884f8. Shares the existing floatZ view.
struct NativeModelSmokeConstants { source: array<vec4<f32>, 6> }
@group(2) @binding(6) var smoke_alpha_minus: texture_2d<f32>;
@group(2) @binding(7) var smoke_alpha_minus_sampler: sampler;
@group(2) @binding(8) var smoke_color: texture_2d<f32>;
@group(2) @binding(9) var smoke_color_sampler: sampler;
@group(2) @binding(10) var smoke_color_minus: texture_2d<f32>;
@group(2) @binding(11) var smoke_color_minus_sampler: sampler;
@group(2) @binding(13) var<uniform> smoke: NativeModelSmokeConstants;
struct SmokeInput {
    @location(0) position: vec3<f32>,
    @location(1) color: vec4<f32>,
    @location(2) uv: vec2<f32>,
}
struct SmokeVaryings {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) eye_distance: f32,
}
@vertex
fn vertex_smoke(input: SmokeInput) -> SmokeVaryings {
    let position = vec4<f32>(input.position, 1.0);
    let world = vec4<f32>(dot(position, object.world_rows[0]), dot(position, object.world_rows[1]),
        dot(position, object.world_rows[2]), dot(position, object.world_rows[3]));
    let clip = vec4<f32>(dot(world, scene.world.view_projection_rows[0]), dot(world, scene.world.view_projection_rows[1]),
        dot(world, scene.world.view_projection_rows[2]), dot(world, scene.world.view_projection_rows[3]));
    return SmokeVaryings(clip, input.color, input.uv, clip.w);
}
fn shade_smoke(input: SmokeVaryings) -> vec4<f32> {
    let c = smoke.source;
    let seconds = scene.world.game_time.w;
    let alpha_uv = fma(input.uv, c[0].xy, fract((vec2<f32>(seconds) * c[2].yz) * c[0].xy));
    let alpha2 = textureSample(layer2_map, layer2_sampler, alpha_uv).r;
    let mask_uv = fma(input.uv - c[5].zw, c[5].xy, c[5].zw);
    let mask = textureSample(layer3_map, layer3_sampler, mask_uv).r;
    let color_uv = fma(input.uv, c[1].zw, fract((vec2<f32>(seconds) * c[1].xy) * c[1].zw));
    let alpha = textureSample(layer1_map, layer1_sampler, color_uv).r;
    let color = textureSample(smoke_color, smoke_color_sampler, color_uv).rgb;
    let minus_uv = fma(input.uv, c[4].xy, fract((vec2<f32>(seconds) * c[4].zw) * c[4].xy));
    let alpha_minus = textureSample(smoke_alpha_minus, smoke_alpha_minus_sampler, minus_uv).r;
    let color_minus = textureSample(smoke_color_minus, smoke_color_minus_sampler, minus_uv).rgb;
    // Despite their names, the original program adds both "Minus" samples.
    let combined = vec4<f32>(color + color_minus, (alpha + alpha_minus) * (alpha2 * mask)) * input.color;
    let depth = textureSample(float_z, float_z_sampler, input.position.xy / frame.render_target_size.xy).r;
    let distance = frame.z_near.x / max(abs(depth), 0.0);
    let fade = clamp((1.0 / max(c[2].x, 0.00009999999747378752)) * (distance - input.eye_distance), 0.0, 1.0);
    let faded = vec4<f32>(combined.rgb, combined.a * (fade * input.color.a)) * c[3];
    let gamma = (faded.a * faded.rgb) * c[0].z;
    return vec4<f32>(sqrt((gamma * gamma) * scene.world.hdr_control0.x), 1.0);
}
@fragment
fn fragment_smoke(input: SmokeVaryings) -> @location(0) vec4<f32> { return shade_smoke(input); }
@fragment
fn fragment_smoke_srgb(input: SmokeVaryings) -> @location(0) vec4<f32> {
    let encoded = shade_smoke(input);
    let linear = select(encoded.rgb / vec3<f32>(12.92),
        pow((encoded.rgb + vec3<f32>(0.055)) / vec3<f32>(1.055), vec3<f32>(2.4)),
        encoded.rgb > vec3<f32>(0.04045));
    return vec4<f32>(linear, encoded.a);
}

import package::rendering::native_model_draw::NativeModelDrawMatrices;
import package::rendering::native_weapon_model_code::{NativeUi3dReticleConstants, native_ui3d_reticle};
// Original reticle VS9b0c84e4/dcb3bcbc/b51cfb3a and PS1f5ed1d/8b92cd8b/7069b90a.
import package::rendering::native_model::{NativeModelScene, NativeModelObject};
import package::rendering::native_world::native_world_unpack_direction;
@group(0) @binding(0) var<uniform> scene: NativeModelScene;
@group(1) @binding(0) var<uniform> object: NativeModelObject;

@group(1) @binding(2) var<uniform> weapon: NativeModelDrawMatrices;
struct NativeReticleConstants { source: array<vec4<f32>, 5> }
@group(2) @binding(26) var<uniform> material: NativeReticleConstants;
@group(2) @binding(0) var detail_map: texture_2d<f32>;
@group(2) @binding(1) var detail_sampler: sampler;
@group(2) @binding(2) var color_map: texture_2d<f32>;
@group(2) @binding(3) var color_sampler: sampler;
@group(3) @binding(0) var ui3d: texture_2d<f32>;
@group(3) @binding(1) var ui3d_sampler: sampler;
struct Input {
    @location(0) position: vec4<f32>, @location(2) uv: vec2<f32>,
    @location(3) packed_normal: vec4<f32>, @location(4) packed_tangent: vec4<f32>,
}
struct Varyings { @builtin(position) position: vec4<f32>, @location(0) uv_mask: vec4<f32> }
fn clip_position(position: vec3<f32>) -> vec4<f32> {
    let p = vec4<f32>(position, 1.0);
    return vec4<f32>(dot(p, weapon.clip[0]), dot(p, weapon.clip[1]), dot(p, weapon.clip[2]), dot(p, weapon.clip[3]));
}
@vertex fn vertex_reticle_dynamic(input: Input) -> Varyings {
    let p = vec4<f32>(input.position.xyz, 1.0);
    let view_position = vec3<f32>(dot(p, weapon.world_view[0]), dot(p, weapon.world_view[1]), dot(p, weapon.world_view[2]));
    let eye = view_position * inverseSqrt(dot(view_position, view_position));
    let normal_input = native_world_unpack_direction(input.packed_normal.xyz);
    let tangent_input = native_world_unpack_direction(input.packed_tangent.xyz);
    // Preserve the source normal's ZXY row order and its pre-rotation cross.
    let raw_normal = vec3<f32>(dot(normal_input, weapon.inverse_transpose[2].xyz),
        dot(normal_input, weapon.inverse_transpose[0].xyz), dot(normal_input, weapon.inverse_transpose[1].xyz));
    let normal = raw_normal * inverseSqrt(dot(raw_normal, raw_normal));
    let raw_tangent = vec3<f32>(dot(tangent_input, weapon.inverse_transpose[0].xyz),
        dot(tangent_input, weapon.inverse_transpose[1].xyz), dot(tangent_input, weapon.inverse_transpose[2].xyz));
    let tangent = raw_tangent * inverseSqrt(dot(raw_tangent, raw_tangent));
    let crossed = fma(normal.zxy, tangent.zxy, -(normal * tangent.yzx)) * -sign(input.position.w);
    let scale = material.source[1].y;
    let mask = vec2<f32>(fma(dot(tangent, eye), scale, 0.5), fma(-dot(crossed, eye), scale, 0.5));
    return Varyings(clip_position(input.position.xyz), vec4<f32>(input.uv, mask));
}
@vertex fn vertex_reticle_static(input: Input) -> Varyings {
    return Varyings(clip_position(input.position.xyz), vec4<f32>(input.uv, 0.0, 0.0));
}
fn shade_reticle(input: Varyings, two_color: bool, screen_noise: bool) -> vec4<f32> {
    let c = material.source;
    let mask_uv = select(input.uv_mask.zw, input.uv_mask.xy, screen_noise);
    let mask = textureSample(color_map, color_sampler, mask_uv);
    var glow_color = c[0].rgb;
    var reticle_color = c[2].yzw;
    if two_color {
        glow_color = fma(vec3<f32>(weapon.parameter0.z), c[4].rgb - glow_color, glow_color);
        reticle_color = fma(vec3<f32>(weapon.parameter0.z), c[3].rgb - reticle_color, reticle_color);
    }
    let glow = exp2(log2(mask.g) * c[1].z) * glow_color;
    let noise_uv = select(input.uv_mask.xy, input.position.xy / weapon.target_size.xy, screen_noise) * c[1].x;
    let noise = textureSample(detail_map, detail_sampler, noise_uv).r;
    let noisy = glow * noise;
    let transmission = fma(-c[1].w, noise, 1.0);
    let reticle = mask.r * reticle_color;
    let gamma = fma(reticle, vec3<f32>(transmission), noisy) * c[0].a;
    var result = vec4<f32>(gamma * gamma, mask.b * c[2].x) * (1.0 - weapon.parameter0.y);
    if !two_color && !screen_noise { result *= clamp(weapon.parameter0.w * 2.0, 0.0, 1.0); }
    return vec4<f32>(sqrt(result.rgb * scene.world.hdr_control0.x), result.a);
}
fn attachment_srgb(encoded: vec4<f32>) -> vec4<f32> {
    let linear = select(encoded.rgb / vec3<f32>(12.92), pow((encoded.rgb + vec3<f32>(0.055)) / vec3<f32>(1.055), vec3<f32>(2.4)), encoded.rgb > vec3<f32>(0.04045));
    return vec4<f32>(linear, encoded.a);
}
@fragment fn fragment_reticle_dynamic(input: Varyings) -> @location(0) vec4<f32> { return shade_reticle(input, false, false); }
@fragment fn fragment_reticle_dynamic_srgb(input: Varyings) -> @location(0) vec4<f32> { return attachment_srgb(shade_reticle(input, false, false)); }
@fragment fn fragment_reticle_two_color(input: Varyings) -> @location(0) vec4<f32> { return shade_reticle(input, true, false); }
@fragment fn fragment_reticle_two_color_srgb(input: Varyings) -> @location(0) vec4<f32> { return attachment_srgb(shade_reticle(input, true, false)); }
@fragment fn fragment_reticle_static(input: Varyings) -> @location(0) vec4<f32> { return shade_reticle(input, false, true); }
@fragment fn fragment_reticle_static_srgb(input: Varyings) -> @location(0) vec4<f32> { return attachment_srgb(shade_reticle(input, false, true)); }
fn shade_ui3d(input: Varyings) -> vec4<f32> {
    return native_ui3d_reticle(input.uv_mask.xy, input.position.xy, weapon.target_size.xy,
        weapon.ui3d_setup5, scene, NativeUi3dReticleConstants(material.source),
        ui3d, ui3d_sampler, detail_map, detail_sampler);
}
@fragment fn fragment_reticle_ui3d(input: Varyings) -> @location(0) vec4<f32> { return shade_ui3d(input); }
@fragment fn fragment_reticle_ui3d_srgb(input: Varyings) -> @location(0) vec4<f32> { return attachment_srgb(shade_ui3d(input)); }

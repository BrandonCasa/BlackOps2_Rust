// Original objective UNLIT: VS/PS640b2e1. Source shield tint pulses use
// fractional gameTime.Z and inverse-transpose world/view normals.
import package::rendering::native_model::{NativeModelScene, NativeModelObject};
import package::rendering::native_world::native_world_unpack_direction;
import package::rendering::native_model_draw::NativeModelDrawMatrices;
import package::rendering::native_character_model::NativeSourceCharacterConstants;
@group(0) @binding(0) var<uniform> scene: NativeModelScene;
@group(1) @binding(0) var<uniform> object: NativeModelObject;
@group(1) @binding(2) var<uniform> matrices: NativeModelDrawMatrices;
@group(2) @binding(0) var color_map: texture_2d<f32>;
@group(2) @binding(1) var color_sampler: sampler;
@group(2) @binding(26) var<uniform> material: NativeSourceCharacterConstants;
struct Input { @location(0) position: vec3<f32>, @location(2) uv: vec2<f32>, @location(3) normal: vec4<f32> }
struct Varyings { @builtin(position) position: vec4<f32>, @location(0) uv_time: vec3<f32>, @location(1) normal: vec3<f32> }
@vertex fn vertex_objective(input: Input) -> Varyings {
    let p = vec4<f32>(input.position, 1.0);
    let world = vec4<f32>(dot(p, object.world_rows[0]), dot(p, object.world_rows[1]), dot(p, object.world_rows[2]), dot(p, object.world_rows[3]));
    let clip = vec4<f32>(dot(world, scene.world.view_projection_rows[0]), dot(world, scene.world.view_projection_rows[1]), dot(world, scene.world.view_projection_rows[2]), dot(world, scene.world.view_projection_rows[3]));
    let normal = native_world_unpack_direction(input.normal.xyz);
    let view_normal = vec3<f32>(dot(normal, matrices.inverse_transpose[0].xyz), dot(normal, matrices.inverse_transpose[1].xyz), dot(normal, matrices.inverse_transpose[2].xyz));
    return Varyings(clip, vec3<f32>(input.uv, scene.world.game_time.z), view_normal);
}
fn shade_objective(input: Varyings) -> vec4<f32> {
    let facing = inverseSqrt(dot(input.normal, input.normal)) * input.normal.z;
    let phase = fma(facing, -0.5, input.uv_time.z) * 6.283180236816406;
    let weight = fma(-sin(phase), 0.5, 0.5);
    let tint = fma(vec3<f32>(weight), material.source[1].rgb - material.source[0].rgb, material.source[0].rgb);
    let color = textureSample(color_map, color_sampler, input.uv_time.xy);
    let luma = dot(vec3<f32>(0.3330000042915344), color.rgb) + 0.5;
    let tinted = tint * luma;
    let blend = fma(weight, material.source[2].y - material.source[2].x, material.source[2].x);
    return vec4<f32>(fma(tinted, vec3<f32>(color.a), color.rgb * blend), color.a);
}
fn srgb_attachment(color: vec4<f32>) -> vec4<f32> {
    let linear = select(color.rgb / vec3<f32>(12.92), pow((color.rgb + vec3<f32>(0.055)) / vec3<f32>(1.055), vec3<f32>(2.4)), color.rgb > vec3<f32>(0.04045));
    return vec4<f32>(linear, color.a);
}
@fragment fn fragment_objective(input: Varyings) -> @location(0) vec4<f32> { return shade_objective(input); }
@fragment fn fragment_objective_srgb(input: Varyings) -> @location(0) vec4<f32> { return srgb_attachment(shade_objective(input)); }

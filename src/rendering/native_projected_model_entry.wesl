// Original projected decal VS/PS6532f59b: a depth-projected volume,
// local Y/Z texture projection, local X edge alpha and source grid atlas light.
import package::rendering::native_fx::NativeFxFrame;
import package::rendering::native_model::{NativeModelObject, NativeModelScene};
import package::rendering::native_model_draw::NativeModelDrawMatrices;
import package::rendering::native_model_variants::native_model_atlas_irradiance;
@group(0) @binding(0) var<uniform> frame: NativeFxFrame;
@group(0) @binding(1) var float_z: texture_2d<f32>;
@group(0) @binding(2) var float_z_sampler: sampler;
@group(1) @binding(0) var<uniform> object: NativeModelObject;
@group(1) @binding(2) var<uniform> matrices: NativeModelDrawMatrices;
@group(2) @binding(0) var color_map: texture_2d<f32>;
@group(2) @binding(1) var color_sampler: sampler;
@group(3) @binding(0) var atlas: texture_3d<f32>;
@group(3) @binding(1) var atlas_sampler: sampler;
struct Input { @location(0) position: vec3<f32> }
struct Varyings { @builtin(position) position: vec4<f32>, @location(0) view_position: vec3<f32> }
@vertex fn vertex_projected(input: Input) -> Varyings {
    let p = vec4<f32>(input.position, 1.0);
    let world = vec4<f32>(dot(p, object.world_rows[0]), dot(p, object.world_rows[1]), dot(p, object.world_rows[2]), dot(p, object.world_rows[3]));
    let clip = vec4<f32>(dot(world, frame.scene.view_projection_rows[0]), dot(world, frame.scene.view_projection_rows[1]), dot(world, frame.scene.view_projection_rows[2]), dot(world, frame.scene.view_projection_rows[3]));
    let view_position = vec3<f32>(dot(p, matrices.world_view[0]), dot(p, matrices.world_view[1]), dot(p, matrices.world_view[2]));
    return Varyings(clip, view_position);
}
fn shade_projected(input: Varyings) -> vec4<f32> {
    let depth_uv = input.position.xy * frame.render_target_size.zw;
    let stored_z = textureSampleLevel(float_z, float_z_sampler, depth_uv, 0.0).r;
    let depth = frame.z_near.x / max(abs(stored_z), 0.0);
    let ray = input.view_position.xy / input.view_position.z;
    let view_position = vec4<f32>(depth * ray, depth, 1.0);
    // VS6532f59b composes inverseWorldMatrix with inverseViewMatrix and emits
    // three uniform plane rows. The shared draw owner supplies that same inverse
    // world/view transform directly; constant plane interpolation is unnecessary.
    let local = vec3<f32>(dot(view_position, matrices.inverse_world_view[0]), dot(view_position, matrices.inverse_world_view[1]), dot(view_position, matrices.inverse_world_view[2]));
    let uv = fma(local, vec3<f32>(0.5), vec3<f32>(0.5));
    let color = textureSample(color_map, color_sampler, uv.yz);
    let raw_normal = vec3<f32>(object.world_rows[0].x, object.world_rows[1].x, object.world_rows[2].x);
    let normal = raw_normal * inverseSqrt(dot(raw_normal, raw_normal));
    // 004189a0 computes1.5/atlas extent; derive it from the one owning texture.
    let lookup_scale = vec3<f32>(1.5) / vec3<f32>(textureDimensions(atlas));
    let scene = NativeModelScene(frame.scene, vec4<f32>(lookup_scale, 0.0));
    let irradiance = native_model_atlas_irradiance(normal, object.grid_lighting_coords_and_vis.xyz, scene, atlas, atlas_sampler);
    let linear = ((color.rgb * color.rgb) * irradiance) * frame.scene.hdr_control0.x;
    let alpha = (1.0 - abs(local.x)) * color.a;
    if any(uv < vec3<f32>(0.0)) || any((vec3<f32>(1.0) - uv) < vec3<f32>(0.0)) || !(alpha > 0.0) { discard; }
    return vec4<f32>(sqrt(linear * 32.0), alpha);
}
fn srgb_attachment(color: vec4<f32>) -> vec4<f32> {
    let linear = select(color.rgb / vec3<f32>(12.92), pow((color.rgb + vec3<f32>(0.055)) / vec3<f32>(1.055), vec3<f32>(2.4)), color.rgb > vec3<f32>(0.04045));
    return vec4<f32>(linear, color.a);
}
@fragment fn fragment_projected(input: Varyings) -> @location(0) vec4<f32> { return shade_projected(input); }
@fragment fn fragment_projected_srgb(input: Varyings) -> @location(0) vec4<f32> { return srgb_attachment(shade_projected(input)); }

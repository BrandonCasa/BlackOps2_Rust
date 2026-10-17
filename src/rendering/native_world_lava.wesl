import package::rendering::native_world::sample_material;
// Original wpc_sw4_3d_unlit_3layer_normal_script_q1z1f6w1:
// VS88827d5f / PSc8864f0c. This world PS reads only Layer1/2/3Map.
import package::rendering::native_world::{NativeWorldScene};
import package::rendering::native_world_wallbuy::{NativeWorldParallaxLayer, native_world_parallax_uv};

struct NativeLavaConstants { layers: array<NativeWorldParallaxLayer, 3> }
struct NativeLavaVaryings {
    clip_position: vec4<f32>,
    fog_color: vec3<f32>,
    eye_fog: vec4<f32>,
    color: vec4<f32>,
    normal: vec3<f32>,
    tangent_sign: vec4<f32>,
    uv: vec2<f32>,
}

fn native_lava_fragment(input: NativeLavaVaryings, scene: NativeWorldScene,
    material: NativeLavaConstants, script_vector0: vec4<f32>,
    layer1_map: texture_2d<f32>, layer1_sampler: sampler,
    layer2_map: texture_2d<f32>, layer2_sampler: sampler,
    layer3_map: texture_2d<f32>, layer3_sampler: sampler) -> vec4<f32> {
    let normal = inverseSqrt(dot(input.normal, input.normal)) * input.normal;
    let bitangent = fma(normal.yzx, input.tangent_sign.zxy,
        -(normal.zxy * input.tangent_sign.yzx)) * input.tangent_sign.w;
    let eye = inverseSqrt(dot(-input.eye_fog.xyz, -input.eye_fog.xyz)) * -input.eye_fog.xyz;
    let facing = dot(eye, normal);
    let divided = -vec2<f32>(dot(eye, input.tangent_sign.xyz), dot(eye, bitangent)) / facing;
    let mask = select(0u, 0xffffffffu, facing > 0.0);
    let ray = bitcast<vec2<f32>>(bitcast<vec2<u32>>(divided) & vec2<u32>(mask));
    let c = material.layers;
    let uv3 = native_world_parallax_uv(input.uv, ray, c[2].depth.x, c[2], scene.game_time.w);
    let uv2 = native_world_parallax_uv(input.uv, ray, c[1].depth.x, c[1], scene.game_time.w);
    let uv1 = native_world_parallax_uv(input.uv, ray, c[0].depth.x, c[0], scene.game_time.w);
    let layer3 = sample_material(scene.texture_mip_scale, layer3_map, layer3_sampler, uv3);
    let layer2 = sample_material(scene.texture_mip_scale, layer2_map, layer2_sampler, uv2);
    let delta2 = fma(-layer3.rgb, vec3<f32>(layer3.a), layer2.rgb);
    let color3 = layer3.a * layer3.rgb;
    let alpha = (vec2<f32>(1.0) - clamp(script_vector0.yx, vec2<f32>(0.0), vec2<f32>(1.0)))
        * vec2<f32>(c[1].scroll_alpha_hdr.z, c[0].scroll_alpha_hdr.z);
    let color2 = fma(vec3<f32>(alpha.x * layer2.a), delta2, color3);
    let layer1 = sample_material(scene.texture_mip_scale, layer1_map, layer1_sampler, uv1);
    let color1 = fma(vec3<f32>(alpha.y * layer1.a), layer1.rgb - color2, color2);
    let colored = color1 * input.color.rgb;
    let amount = fma(script_vector0.x, -c[0].scroll_alpha_hdr.w, c[0].scroll_alpha_hdr.w);
    let emission = amount * colored;
    let delta = fma(emission, emission, -input.fog_color);
    let fogged = fma(vec3<f32>(input.eye_fog.w), delta, input.fog_color);
    return vec4<f32>(sqrt(fogged * scene.hdr_control0.x), 1.0);
}

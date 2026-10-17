import package::rendering::native_world::sample_material;
// wpc_sw4_3d_burning_embers_nuketown_74j6971w, LIT:
// VS9e139b80 / PSfbd23a5a. Original disassembly is in ember-{vs,ps}.asm.
import package::rendering::native_world::{NativeWorldScene, NativeWorldObject,
    NativeWorldVertexInput, native_world_vertex};

struct NativeEmberMaterial {
    scale_direction_max: vec4<f32>, // Ember_Scale, Ember_Direction.xy, Flicker_Max
    min_seed: vec4<f32>,            // Flicker_Min, Seed_Value
}
struct NativeEmberLighting {
    sun_position: vec4<f32>,
    sun_diffuse: vec4<f32>,
    hero_rows: array<vec4<f32>, 3>,
}
struct NativeEmberVaryings {
    clip_position: vec4<f32>,
    fog_flicker: vec4<f32>,
    eye_fog: vec4<f32>,
    color: vec4<f32>,
    normal: vec3<f32>,
    tangent_sign: vec4<f32>,
    uv_lightmap: vec4<f32>,
    reflection_correction: vec3<f32>,
}

fn native_ember_vertex(input: NativeWorldVertexInput, scene: NativeWorldScene,
    object: NativeWorldObject, material: NativeEmberMaterial) -> NativeEmberVaryings {
    let time10 = scene.game_time.w * 10.0;
    let quantized = fma(scene.game_time.w, 10.0, -fract(time10));
    let seed = fma(quantized, 0.10000000149011612, material.min_seed.y);
    let coefficients = vec4<f32>(81.239487, 17.348024, 37.349838, 59.394840);
    var noise = vec4<f32>(seed);
    noise.x = fract(dot(noise, coefficients));
    noise.y = fract(dot(noise, coefficients));
    noise.z = fract(dot(noise, coefficients));
    noise.w = fract(dot(noise, coefficients));
    noise.x = fract(dot(noise, coefficients));
    noise.y = fract(dot(noise, coefficients));
    noise.z = fract(dot(noise, coefficients));
    let random = fract(dot(noise, coefficients));
    let flicker = fma(random, material.scale_direction_max.w - material.min_seed.x, material.min_seed.x);
    let output = native_world_vertex(input, scene, object);
    return NativeEmberVaryings(output.clip_position, vec4<f32>(output.fog_color, flicker),
        vec4<f32>(output.eye_to_vertex, output.normal_fog.w), input.color,
        output.normal_fog.xyz, vec4<f32>(output.tangent, sign(input.position.w)),
        output.uv_lightmap, output.reflection_correction);
}

fn native_ember_fragment(input: NativeEmberVaryings, scene: NativeWorldScene,
    light: NativeEmberLighting, material: NativeEmberMaterial,
    diffuse_map: texture_2d<f32>, diffuse_sampler: sampler,
    normal_map: texture_2d<f32>, normal_sampler: sampler,
    lightmap: texture_2d<f32>, lightmap_sampler: sampler,
    probe: texture_cube<f32>, probe_sampler: sampler,
    super_map: texture_2d<f32>, super_sampler: sampler,
    ember_map: texture_2d<f32>, ember_sampler: sampler) -> vec4<f32> {
    let geometric_normal = inverseSqrt(dot(input.normal, input.normal)) * input.normal;
    let bitangent = fma(geometric_normal.yzx, input.tangent_sign.zxy,
        -(geometric_normal.zxy * input.tangent_sign.yzx)) * input.tangent_sign.w;
    let normal_sample = sample_material(scene.texture_mip_scale, normal_map, normal_sampler, input.uv_lightmap.xy);
    let slopes = fma(normal_sample.rg, vec2<f32>(4.015748023986816), vec2<f32>(-2.0157480239868164));
    let mapped = fma(vec3<f32>(slopes.y), bitangent,
        fma(vec3<f32>(slopes.x), input.tangent_sign.xyz, geometric_normal));
    let normal = inverseSqrt(dot(mapped, mapped)) * mapped;
    let inverse_distance = inverseSqrt(dot(input.eye_fog.xyz, input.eye_fog.xyz));
    let incident = inverse_distance * input.eye_fog.xyz;
    let half_vector = fma(-input.eye_fog.xyz, vec3<f32>(inverse_distance), light.sun_position.xyz);
    let reflection_direction = fma(normal, vec3<f32>(-2.0 * dot(incident, normal)), incident);
    let reflection_sample = textureSampleLevel(probe, probe_sampler, reflection_direction, 4.0);
    let reflected = (1.0 / (reflection_sample.a + 9.999999974752427e-07)) * reflection_sample.rgb;
    let control = sample_material(scene.texture_mip_scale, super_map, super_sampler, input.uv_lightmap.xy);
    let gloss_squared = control.b * control.b;
    let reflection_strength = clamp(fma(gloss_squared, 0.765625, -0.015625), 0.0, 1.0);
    let ambient_uv = input.uv_lightmap.zw * vec2<f32>(1.0, 0.3333333432674408);
    let ambient_sample = textureSample(lightmap, lightmap_sampler, ambient_uv);
    let ambient = (1.0 / (ambient_sample.a + 9.999999974752427e-07)) * ambient_sample.rgb;
    let other_uv = fma(input.uv_lightmap.zwzw,
        vec4<f32>(1.0, 0.3333333432674408, 1.0, 0.3333333432674408),
        vec4<f32>(0.0, 0.3333333432674408, 0.0, 0.6666666865348816));
    let direction_sample = textureSample(lightmap, lightmap_sampler, other_uv.zw);
    let directional_sample = textureSample(lightmap, lightmap_sampler, other_uv.xy);
    let direction = fma(direction_sample.rgb, vec3<f32>(2.0), vec3<f32>(-1.0));
    let sunlight = direction_sample.a * light.sun_diffuse.xyz;
    let directional = (1.0 / (directional_sample.a + 9.999999974752427e-07)) * directional_sample.rgb;
    let reflection_light = fma(directional, vec3<f32>(clamp(dot(direction, geometric_normal), 0.0, 1.0)), ambient);
    var diffuse_light = fma(directional, vec3<f32>(clamp(dot(direction, normal), 0.0, 1.0)), ambient);
    let reflection = (reflection_strength * reflected) * (reflection_light * input.reflection_correction);
    let half_direction = inverseSqrt(dot(half_vector, half_vector)) * half_vector;
    let nh = clamp(dot(normal, half_direction), 0.0, 1.0);
    let lh = clamp(dot(half_direction, light.sun_position.xyz), 0.0, 1.0);
    let nl = clamp(dot(light.sun_position.xyz, normal), 0.0, 1.0);
    let specular_geometry = (nl / fma(lh * lh, 0.5450000166893005, 0.4650000035762787)) * (nh * 0.375);
    let fresnel = exp2(lh * -10.0);
    diffuse_light = fma(vec3<f32>(nl), sunlight, diffuse_light);
    let specular_strength = (fma(fma(-control.b, control.b, 1.0), fresnel, gloss_squared) * specular_geometry) * 0.25;
    let specular = fma(reflection, vec3<f32>(0.20000000298023224), sunlight * specular_strength);
    let offset = fract((scene.game_time.w * material.scale_direction_max.yz) * material.scale_direction_max.x);
    let ember_uv = fma(input.uv_lightmap.xy, vec2<f32>(material.scale_direction_max.x), offset);
    var ember = sample_material(scene.texture_mip_scale, ember_map, ember_sampler, ember_uv);
    ember *= vec4<f32>(vec3<f32>(input.fog_flicker.w), 1.0);
    ember *= vec4<f32>(vec3<f32>(control.r), input.fog_flicker.w);
    let diffuse = sample_material(scene.texture_mip_scale, diffuse_map, diffuse_sampler, input.uv_lightmap.xy);
    let delta = fma(ember, vec4<f32>(6.0, 6.0, 6.0, control.r), -diffuse);
    let composed = fma(vec4<f32>(control.g), delta, diffuse) * input.color;
    let lit = composed.a * fma(composed.rgb * composed.rgb, diffuse_light, specular);
    let hero_input = vec4<f32>(lit, 1.0);
    let hero = vec3<f32>(dot(hero_input, light.hero_rows[0]),
        dot(hero_input, light.hero_rows[1]), dot(hero_input, light.hero_rows[2]));
    let fogged = fma(vec3<f32>(input.eye_fog.w), hero - input.fog_flicker.xyz, input.fog_flicker.xyz);
    return vec4<f32>(sqrt(fogged * scene.hdr_control0.x), sqrt(clamp(composed.a, 0.0, 1.0)));
}

import package::rendering::native_world::{
    NativeWorldScene, NativeWorldObject, NativeWorldVertexInput, NativeWorldVaryings,
    native_world_unpack_direction, native_omni_diffuse,
};

// Original T6 world LIT programs, from rendering-20261012/{source,shaders}:
// wpc_lit_sm_r0c0n0_80fe30wz: pimp_technique_lmap_7f91dc6a.
// VS d107eac0 SHA256 a2d0f3e46a6a37bbdba445b91c0134202db711fac4fb43c7b68195e9ecac292d.
// PS c6b996da SHA256 10194a72769e92faae977557accd83691df2f283b6cfbbfb3c9f7f8538ce3028.
// wpc_lit_sm_r0c0n0s0_1zzj1138: pimp_technique_lmap_eeb8e9d6.
// VS b89a2252 SHA256 141c0bd5e5d3d456fcea5c3a862f7182189495ef4552bdd6a9b6212d02a6e74b.
// PS 353f0d80 SHA256 842475d764e0e1333b7cb234a2a678e581bd874bccaaeea20a129d6b007bbf22.
// wpc_lit_sm_r0c0s0_qfqz7850: pimp_technique_lmap_aa205905.
// VS d63b1bf7 SHA256 931af2e1c0491dc6c61165dd8545a3d2fb8108361629a33707f53f28edfbf72d.
// PS 5b9e3425 SHA256 43b944634592348c729e3cb38d829369be4483d52639e392574e643724e0f6c4.

struct NativeWorldGeometricVertexInput {
    position: vec3<f32>,
    color: vec4<f32>,
    uv: vec2<f32>,
    packed_normal: vec3<f32>,
    lightmap_uv: vec2<f32>,
}

// These source VS programs share the position/fog/normal instruction sequence.
// The returned struct also has slots used by the other world programs. Only
// the dedicated source wrappers below produce each program's required slots.
fn native_world_position_fog(
    input: NativeWorldGeometricVertexInput,
    scene: NativeWorldScene,
    object: NativeWorldObject,
) -> NativeWorldVaryings {
    let position = vec4<f32>(input.position, 1.0);
    let world_position = vec4<f32>(
        dot(position, object.world_rows[0]),
        dot(position, object.world_rows[1]),
        dot(position, object.world_rows[2]),
        dot(position, object.world_rows[3]),
    );
    var output: NativeWorldVaryings;
    output.clip_position = vec4<f32>(
        dot(world_position, scene.view_projection_rows[0]),
        dot(world_position, scene.view_projection_rows[1]),
        dot(world_position, scene.view_projection_rows[2]),
        dot(world_position, scene.view_projection_rows[3]),
    );
    output.color = input.color * object.color;
    output.uv_lightmap = vec4<f32>(input.uv, input.lightmap_uv);

    let height_term = fma(scene.fog_consts.w, world_position.z, scene.fog_consts.x);
    let height_exp = exp2(min(height_term, 64.0) * 1.4426950216293335);
    let height_integral = select(height_term + 1.0, height_exp, height_term < 0.0);
    let height_delta = world_position.z * scene.fog_consts.w;
    let near_horizontal = abs(height_delta) < 9.999999747378752e-05;
    let denominator = select(height_delta, 1.0, near_horizontal);
    let fog_integral = select(
        (height_integral - scene.fog_consts2.x) / denominator,
        clamp(scene.fog_consts2.x, 0.0, 1.0),
        near_horizontal,
    ) * scene.fog_consts.y;
    let distance_squared = dot(world_position.xyz, world_position.xyz);
    let direction = inverseSqrt(distance_squared) * world_position.xyz;
    let sun_amount = clamp(fma(scene.sun_fog.y, dot(scene.sun_fog_dir.xyz, direction), scene.sun_fog.x), 0.0, 1.0);
    let fog_amount = 1.0 - min(exp2(fma(fog_integral, sqrt(distance_squared), scene.fog_consts.z)), 1.0);
    let fog = fma(vec4<f32>(sun_amount), scene.sun_fog_color - scene.fog_color, scene.fog_color);
    output.fog_color = fog.xyz;
    output.eye_to_vertex = world_position.xyz;

    output.normal_fog = vec4<f32>(vec3<f32>(0.0), fma(-fog_amount, fog.w, 1.0));
    return output;
}

fn native_world_plain_base_vertex(
    input: NativeWorldGeometricVertexInput,
    scene: NativeWorldScene,
    object: NativeWorldObject,
) -> NativeWorldVaryings {
    var output = native_world_position_fog(input, scene, object);
    let decoded_normal = native_world_unpack_direction(input.packed_normal);
    let transformed_normal = vec3<f32>(
        dot(decoded_normal, object.world_rows[0].xyz),
        dot(decoded_normal, object.world_rows[1].xyz),
        dot(decoded_normal, object.world_rows[2].xyz),
    );
    let normal = inverseSqrt(dot(transformed_normal, transformed_normal)) * transformed_normal;
    output.normal_fog = vec4<f32>(normal, output.normal_fog.w);
    return output;
}

// 80fe30wz VS: packed world stride 36, no auxiliary stream. It transforms N/T
// independently and builds handed bitangent. Reflection SH is never read.
fn native_world_diffuse_vertex(
    input: NativeWorldVertexInput,
    scene: NativeWorldScene,
    object: NativeWorldObject,
) -> NativeWorldVaryings {
    let geometric = NativeWorldGeometricVertexInput(input.position.xyz, input.color,
        input.uv, input.packed_normal, input.lightmap_uv);
    var output = native_world_plain_base_vertex(geometric, scene, object);
    let decoded_tangent = native_world_unpack_direction(input.packed_tangent);
    let transformed_tangent = vec3<f32>(
        dot(decoded_tangent, object.world_rows[0].xyz),
        dot(decoded_tangent, object.world_rows[1].xyz),
        dot(decoded_tangent, object.world_rows[2].xyz),
    );
    let tangent = inverseSqrt(dot(transformed_tangent, transformed_tangent)) * transformed_tangent;
    output.tangent = tangent;
    output.bitangent = sign(input.position.w) * cross(output.normal_fog.xyz, tangent);
    return output;
}

// qfqz7850 VS: same packed world stride, but neither position.w nor tangent
// is consumed. SH correction uses the transformed geometric normal in VS;
// interpolate that correction, instead of recomputing it from the PS normal.
fn native_world_geometric_specular_vertex(
    input: NativeWorldGeometricVertexInput,
    scene: NativeWorldScene,
    object: NativeWorldObject,
) -> NativeWorldVaryings {
    var output = native_world_plain_base_vertex(input, scene, object);
    let normal = output.normal_fog.xyz;
    let quadratic = vec4<f32>(normal.z * normal.x, normal.y * normal.z, normal.x * normal.y,
        fma(normal.x, normal.x, -normal.y * normal.y));
    let reflection_linear = dot(object.reflection_lighting_sh1, vec4<f32>(normal, 1.0));
    let reflection_quadratic = dot(object.reflection_lighting_sh2, quadratic);
    let reflection_scale = fma(object.reflection_lighting_sh0.w, normal.z * normal.z,
        reflection_quadratic + reflection_linear);
    output.reflection_correction = vec3<f32>(1.0) / max(
        reflection_scale * object.reflection_lighting_sh0.xyz,
        vec3<f32>(0.10000000149011612),
    );
    return output;
}

fn native_world_plain_mapped_normal(
    input: NativeWorldVaryings,
    normal_sample: vec2<f32>,
) -> vec3<f32> {
    let slopes = fma(normal_sample, vec2<f32>(4.015748023986816), vec2<f32>(-2.0157480239868164));
    let mapped_normal = fma(vec3<f32>(slopes.y), input.bitangent,
        fma(vec3<f32>(slopes.x), input.tangent, input.normal_fog.xyz));
    return inverseSqrt(dot(mapped_normal, mapped_normal)) * mapped_normal;
}

// 80fe30wz PS: original t0/s0 C, t1/s1 N, t13/s13 secondary lightmap.
// No reflection probe, specular input, material constant or alpha test.
fn native_world_diffuse_fragment(
    input: NativeWorldVaryings,
    scene: NativeWorldScene,
    color_sample: vec4<f32>,
    normal_map: texture_2d<f32>, normal_sampler: sampler,
    lightmap_secondary: texture_2d<f32>, lightmap_sampler: sampler,
    blended: bool,
    opacity: f32,
) -> vec4<f32> {
    let normal_sample = textureSample(normal_map, normal_sampler, input.uv_lightmap.xy);
    let normal = native_world_plain_mapped_normal(input, normal_sample.rg);
    let lm_uv = input.uv_lightmap.zw;
    let other_uv = fma(lm_uv.xyxy, vec4<f32>(1.0, 0.3333333432674408, 1.0, 0.3333333432674408),
        vec4<f32>(0.0, 0.3333333432674408, 0.0, 0.6666666865348816));
    let direction_sample = textureSample(lightmap_secondary, lightmap_sampler, other_uv.zw);
    let directional_sample = textureSample(lightmap_secondary, lightmap_sampler, other_uv.xy);
    let light_direction = fma(direction_sample.rgb, vec3<f32>(2.0), vec3<f32>(-1.0));
    let cosine = clamp(dot(light_direction, normal), 0.0, 1.0);
    let directional = (1.0 / (directional_sample.a + 9.999999974752427e-07)) * directional_sample.rgb;
    let directed_light = cosine * directional;
    let ambient_uv = lm_uv * vec2<f32>(1.0, 0.3333333432674408);
    let ambient_sample = textureSample(lightmap_secondary, lightmap_sampler, ambient_uv);
    let ambient_reciprocal_alpha = 1.0 / (ambient_sample.a + 9.999999974752427e-07);
    let light = fma(ambient_sample.rgb, vec3<f32>(ambient_reciprocal_alpha), directed_light);
    let colored = color_sample.rgb * input.color.rgb;
    let diffuse = colored * colored;
    if blended {
        let fog_color = opacity * input.fog_color;
        let fog_delta = fma(light * diffuse, vec3<f32>(opacity), -fog_color);
        let fogged = fma(vec3<f32>(input.normal_fog.w), fog_delta, fog_color);
        return vec4<f32>(sqrt(fogged * scene.hdr_control0.x), sqrt(opacity));
    }
    // PS37a44012 adds the combined-glight diffuse before fog/exposure. Keep
    // PSc6b996da's original fused subtract when that variant is inactive.
    var fog_delta = fma(diffuse, light, -input.fog_color);
    if scene.omni_lights.enabled.x != 0.0 {
        let omni = native_omni_diffuse(scene.omni_lights, input.eye_to_vertex, normal);
        let lit = fma(diffuse, omni, diffuse * light);
        fog_delta = lit - input.fog_color;
    }
    let fogged = fma(vec3<f32>(input.normal_fog.w), fog_delta, input.fog_color);
    return vec4<f32>(sqrt(fogged * scene.hdr_control0.x), 1.0);
}

// Identical reflection instruction sequence in PS353f0d80 and PS5b9e3425.
// S.rgb is the authored gamma-space specular color; S.a is gloss.
fn native_world_plain_specular_reflection(
    eye_to_vertex: vec3<f32>,
    normal: vec3<f32>,
    specular_sample: vec4<f32>,
    reflection_probe: texture_cube<f32>, reflection_sampler: sampler,
) -> vec3<f32> {
    let incident = inverseSqrt(dot(eye_to_vertex, eye_to_vertex)) * eye_to_vertex;
    let reflection_direction = fma(normal, vec3<f32>(-2.0 * dot(incident, normal)), incident);
    let fresnel = exp2(clamp(dot(normal, -incident), 0.0, 1.0) * -9.279999732971191);
    let gloss = specular_sample.a;
    let sample = textureSampleLevel(reflection_probe, reflection_sampler, reflection_direction, fma(-4.0, gloss, 4.0));
    let probe = (1.0 / (sample.a + 9.999999974752427e-07)) * sample.rgb;
    let coefficients = fma(vec4<f32>(gloss),
        vec4<f32>(1.0416667461395264, 0.4749999940395355, 0.018229199573397636, 0.25),
        vec4<f32>(0.0, 0.0, -0.015625, 0.75));
    let specular = specular_sample.rgb * specular_sample.rgb;
    let response_a = fma(coefficients.x, min(fresnel, coefficients.y), coefficients.z);
    let factor = clamp(fma(specular, vec3<f32>(coefficients.w - response_a), vec3<f32>(response_a)),
        vec3<f32>(0.0), vec3<f32>(1.0));
    return probe * factor;
}

// Identical lightmap, geometric-normal reflection correction, vertex tint,
// fog and target encoding instructions in the two source specular programs.
fn native_world_plain_specular_lighting(
    input: NativeWorldVaryings,
    scene: NativeWorldScene,
    normal: vec3<f32>,
    reflected: vec3<f32>,
    color_sample: vec4<f32>,
    lightmap_secondary: texture_2d<f32>, lightmap_sampler: sampler,
    blended: bool,
    opacity: f32,
) -> vec4<f32> {
    let lm_uv = input.uv_lightmap.zw;
    let ambient_uv = lm_uv * vec2<f32>(1.0, 0.3333333432674408);
    let ambient_sample = textureSample(lightmap_secondary, lightmap_sampler, ambient_uv);
    let ambient = (1.0 / (ambient_sample.a + 9.999999974752427e-07)) * ambient_sample.rgb;
    let other_uv = fma(lm_uv.xyxy, vec4<f32>(1.0, 0.3333333432674408, 1.0, 0.3333333432674408),
        vec4<f32>(0.0, 0.3333333432674408, 0.0, 0.6666666865348816));
    let directional_sample = textureSample(lightmap_secondary, lightmap_sampler, other_uv.xy);
    let direction_sample = textureSample(lightmap_secondary, lightmap_sampler, other_uv.zw);
    let light_direction = fma(direction_sample.rgb, vec3<f32>(2.0), vec3<f32>(-1.0));
    let directional = (1.0 / (directional_sample.a + 9.999999974752427e-07)) * directional_sample.rgb;
    let diffuse_light = fma(directional, vec3<f32>(clamp(dot(light_direction, normal), 0.0, 1.0)), ambient);
    let reflection_light = fma(directional, vec3<f32>(clamp(dot(light_direction, input.normal_fog.xyz), 0.0, 1.0)), ambient)
        * input.reflection_correction;
    let reflection = reflected * reflection_light;
    let colored = color_sample.rgb * input.color.rgb;
    let diffuse = colored * colored;
    let lit = fma(diffuse, diffuse_light, reflection);
    if blended {
        let fog_color = opacity * input.fog_color;
        let fog_delta = fma(lit, vec3<f32>(opacity), -fog_color);
        let fogged = fma(vec3<f32>(input.normal_fog.w), fog_delta, fog_color);
        return vec4<f32>(sqrt(fogged * scene.hdr_control0.x), sqrt(opacity));
    }
    let fogged = fma(vec3<f32>(input.normal_fog.w), lit - input.fog_color, input.fog_color);
    return vec4<f32>(sqrt(fogged * scene.hdr_control0.x), 1.0);
}

// 1zzj1138 uses the existing native_world_vertex: its VS b89a2252 is exactly
// the already-ported binary. PS native slots: C0/s0, N1/s1, S2/s2, LM13/s13,
// cube15/s15; only cb0[20].x is read by PS. No material constants.
fn native_world_normal_specular_fragment(
    input: NativeWorldVaryings,
    scene: NativeWorldScene,
    color_sample: vec4<f32>,
    normal_map: texture_2d<f32>, normal_sampler: sampler,
    specular_map: texture_2d<f32>, specular_sampler: sampler,
    lightmap_secondary: texture_2d<f32>, lightmap_sampler: sampler,
    reflection_probe: texture_cube<f32>, reflection_sampler: sampler,
    blended: bool,
    opacity: f32,
) -> vec4<f32> {
    let normal_sample = textureSample(normal_map, normal_sampler, input.uv_lightmap.xy);
    let normal = native_world_plain_mapped_normal(input, normal_sample.rg);
    let specular = textureSample(specular_map, specular_sampler, input.uv_lightmap.xy);
    let reflected = native_world_plain_specular_reflection(input.eye_to_vertex, normal,
        specular, reflection_probe, reflection_sampler);
    return native_world_plain_specular_lighting(input, scene, normal, reflected,
        color_sample, lightmap_secondary, lightmap_sampler, blended, opacity);
}

// qfqz7850 PS native slots: C0/s0, S1/s1, LM13/s13, cube15/s15.
// This program normalizes the interpolated geometric normal in PS. It uses
// that normal for diffuse/probe direction, and the unnormalized interpolant
// for reflection-light correction, matching its two separate source dp3s.
fn native_world_geometric_specular_fragment(
    input: NativeWorldVaryings,
    scene: NativeWorldScene,
    color_map: texture_2d<f32>, color_sampler: sampler,
    specular_map: texture_2d<f32>, specular_sampler: sampler,
    lightmap_secondary: texture_2d<f32>, lightmap_sampler: sampler,
    reflection_probe: texture_cube<f32>, reflection_sampler: sampler,
    blended: bool,
) -> vec4<f32> {
    let normal = inverseSqrt(dot(input.normal_fog.xyz, input.normal_fog.xyz)) * input.normal_fog.xyz;
    let specular = textureSample(specular_map, specular_sampler, input.uv_lightmap.xy);
    let reflected = native_world_plain_specular_reflection(input.eye_to_vertex, normal,
        specular, reflection_probe, reflection_sampler);
    let color = textureSample(color_map, color_sampler, input.uv_lightmap.xy);
    return native_world_plain_specular_lighting(input, scene, normal, reflected,
        color, lightmap_secondary, lightmap_sampler, blended, color.a * input.color.a);
}

// wpc_lit_sm_r0c0x0_5j63fj97: PSf4d09842, VSd63b1bf7.
// Color alpha controls gloss directly, unlike the separate specular-map family.
fn native_world_geometric_gloss_fragment(
    input: NativeWorldVaryings, scene: NativeWorldScene,
    color_map: texture_2d<f32>, color_sampler: sampler,
    lightmap_secondary: texture_2d<f32>, lightmap_sampler: sampler,
    reflection_probe: texture_cube<f32>, reflection_sampler: sampler,
) -> vec4<f32> {
    let incident = inverseSqrt(dot(input.eye_to_vertex, input.eye_to_vertex)) * input.eye_to_vertex;
    let normal = inverseSqrt(dot(input.normal_fog.xyz, input.normal_fog.xyz)) * input.normal_fog.xyz;
    let direction = fma(normal, vec3<f32>(-2.0 * dot(incident, normal)), incident);
    let fresnel = exp2(clamp(dot(normal, -incident), 0.0, 1.0) * -9.279999732971191);
    let color = textureSample(color_map, color_sampler, input.uv_lightmap.xy);
    let gloss = color.a;
    let probe_sample = textureSampleLevel(reflection_probe, reflection_sampler, direction, fma(-4.0, gloss, 4.0));
    let probe = (1.0 / (probe_sample.a + 9.999999974752427e-07)) * probe_sample.rgb;
    let strength = fma(gloss, min(fresnel, gloss * 0.4749999940395355),
        fma(gloss, 0.027499999850988388, 0.014999999664723873));
    return native_world_plain_specular_lighting(input, scene, normal, strength * probe,
        color, lightmap_secondary, lightmap_sampler, false, 1.0);
}

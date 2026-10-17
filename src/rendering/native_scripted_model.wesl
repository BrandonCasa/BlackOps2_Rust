import package::rendering::native_world::sample_material;
import package::rendering::native_model::{
    NativeModelScene, NativeModelObject, NativeModelVertexInput, native_model_vertex_lighting,
};
import package::rendering::native_model_variants::{native_model_atlas_irradiance};

// Original per-view values consumed by these programs, without light-unit or
// coordinate conversions: cb0[18], cb0[19], and cb2[4].w respectively.
struct NativeModelScriptedScene {
    sun_position: vec4<f32>,
    sun_diffuse: vec4<f32>,
    game_time: vec4<f32>,
    hero_lighting: array<vec4<f32>, 3>,
}

// Native scriptVector0..3 at original cb1[23..26], not inferred material names.
struct NativeModelScriptInputs {
    vectors: array<vec4<f32>, 4>,
}

// Original cb2[5..15]. The loader fills the bound components from their exact
// material hashes. Unused padding/components are never shader input controls.
struct NativeModelParallaxConstants {
    source: array<vec4<f32>, 11>,
}

struct NativeModelEmissiveConstants {
    controls: vec4<f32>, // cb2[5]: ReflectionAmount, SpecularAmount, hdrAmount, Specular_Push (Threshold for script-multi).
    gloss_push: vec4<f32>, // X: Gloss_Push; Y: Emissive_Push (Alcatraz cb2[6].x).
}

struct NativeFlickerConstants {
    controls: vec4<f32>, // hdrAmount, FlickerSpeed, FlickerMin, FlickerMax
    map_column: vec4<f32>, // FlickerMapCol in X
}

// LIT37631e0c / PSbb5863e8, with shared VSb7d697d7. Fixed 0.04 dielectric
// specular, diffuse-alpha gloss, uncorrected atlas and scriptVector1.x suppression.
// PS SHA256 a7a2654f184ba9b5f18661cf34fdb36726abd40426037a9a5d30480068f596c8.
fn native_model_flicker_fragment(
    input: NativeScriptedVaryings, scene: NativeModelScene,
    view: NativeModelScriptedScene, scripts: NativeModelScriptInputs,
    material: NativeFlickerConstants,
    normal_map: texture_2d<f32>, normal_sampler: sampler,
    diffuse_map: texture_2d<f32>, diffuse_sampler: sampler,
    flicker_map: texture_2d<f32>, flicker_sampler: sampler,
    emissive_map: texture_2d<f32>, emissive_sampler: sampler,
    model_lighting: texture_3d<f32>, model_lighting_sampler: sampler,
    reflection_probe: texture_cube<f32>, reflection_sampler: sampler,
) -> vec4<f32> {
    let geometric = input.normal * inverseSqrt(dot(input.normal, input.normal));
    let bitangent = native_scripted_bitangent(geometric, input.tangent_sign);
    let normal_sample = sample_material(scene.world.texture_mip_scale, normal_map, normal_sampler, input.uv);
    let slopes = fma(normal_sample.rg, vec2<f32>(4.015748023986816), vec2<f32>(-2.0157480239868164));
    let mapped = fma(vec3<f32>(slopes.y), bitangent,
        fma(vec3<f32>(slopes.x), input.tangent_sign.xyz, geometric));
    let normal = mapped * inverseSqrt(dot(mapped, mapped));
    let eye_length = inverseSqrt(dot(input.eye_fog.xyz, input.eye_fog.xyz));
    let incident = eye_length * input.eye_fog.xyz;
    let half_raw = fma(-input.eye_fog.xyz, vec3<f32>(eye_length), view.sun_position.xyz);
    let reflection_direction = fma(normal, vec3<f32>(-2.0 * dot(incident, normal)), incident);
    let fresnel = exp2(clamp(dot(normal, -incident), 0.0, 1.0) * -9.279999732971191);
    let diffuse_sample = sample_material(scene.world.texture_mip_scale, diffuse_map, diffuse_sampler, input.uv);
    let gloss = diffuse_sample.a;
    let probe_sample = textureSampleLevel(reflection_probe, reflection_sampler,
        reflection_direction, fma(gloss, -4.0, 4.0));
    let probe = (1.0 / (probe_sample.a + 9.999999974752427e-07)) * probe_sample.rgb;
    let coefficients = fma(vec4<f32>(gloss),
        vec4<f32>(1.0416667461395264, 0.4749999940395355, 0.018229199573397636, 0.25),
        vec4<f32>(0.0, 0.0, -0.015625, 0.75));
    let response_a = fma(coefficients.x, min(fresnel, coefficients.y), coefficients.z);
    let reflected = clamp(fma(coefficients.w - response_a, 0.03999999910593033, response_a), 0.0, 1.0) * probe;
    let half_vector = half_raw * inverseSqrt(dot(half_raw, half_raw));
    let ndoth = clamp(dot(normal, half_vector), 0.0, 1.0);
    let hdotsun = clamp(dot(half_vector, view.sun_position.xyz), 0.0, 1.0);
    let exponent = exp2(gloss * 13.0);
    let lobe = exp2(log2(ndoth) * exponent) * fma(exponent, 0.125, 0.25);
    let fresnel_direct = fma(exp2(hdotsun * -10.0), 0.9599999785423279, 0.03999999910593033);
    let visibility_gloss = clamp(gloss + 0.5450000166893005, 0.0, 1.0);
    let denominator = fma(hdotsun * hdotsun, visibility_gloss, -visibility_gloss) + 1.0099999904632568;
    let ndotsun = clamp(dot(view.sun_position.xyz, normal), 0.0, 1.0);
    let sun = input.grid_coords_and_vis.w * view.sun_diffuse.xyz;
    let direct_specular = (fresnel_direct * (lobe * (ndotsun / denominator))) * sun;
    var lit = fma(reflected, input.reflection_correction, direct_specular);
    let irradiance = native_model_atlas_irradiance(normal, input.grid_coords_and_vis.xyz,
        scene, model_lighting, model_lighting_sampler) * 32.0;
    let colored = diffuse_sample.rgb * input.color.rgb;
    lit = fma(colored * colored, fma(vec3<f32>(ndotsun), sun, irradiance), lit);
    let flicker_uv = vec2<f32>(material.map_column.x, view.game_time.w * material.controls.y);
    let flicker_sample = sample_material(scene.world.texture_mip_scale, flicker_map, flicker_sampler, flicker_uv).r;
    let flicker = fma(flicker_sample, material.controls.w - material.controls.z, material.controls.z);
    let emission_scale = (flicker * clamp(1.0 - scripts.vectors[1].x, 0.0, 1.0)) * material.controls.x;
    let emission = sample_material(scene.world.texture_mip_scale, emissive_map, emissive_sampler, input.uv);
    let gamma_emission = emission_scale * (emission.a * emission.rgb);
    lit = fma(gamma_emission, gamma_emission, lit);
    let fogged = fma(vec3<f32>(input.eye_fog.w), lit - input.fog_color, input.fog_color);
    return vec4<f32>(sqrt(fogged * scene.world.hdr_control0.x), 1.0);
}

struct NativeScriptedVaryings {
    clip_position: vec4<f32>,
    fog_color: vec3<f32>,
    eye_fog: vec4<f32>,
    color: vec4<f32>,
    normal: vec3<f32>,
    tangent_sign: vec4<f32>,
    uv: vec2<f32>,
    grid_coords_and_vis: vec4<f32>,
    reflection_correction: vec3<f32>,
    grid_correction: vec3<f32>,
}

// VS254a3594 SHA82ce1da9277c0d26d87bc9b0164cb3f364ccd3911753225ca614ffc76ff4071e
// and VSb7d697d7 SHAb1bacc64d083fb8b2a60ab28e346e14bb6f0dd9cbf21930480f7dc647f42dabe.
// Their position/fog/N/T/SH instruction sequences match the existing model
// implementation. These VS outputs retain T/sign: their PS reconstructs B
// from the normalized interpolated N, rather than interpolating a VS-built B.
fn native_scripted_model_vertex(
    input: NativeModelVertexInput,
    scene: NativeModelScene,
    object: NativeModelObject,
) -> NativeScriptedVaryings {
    return native_scripted_model_vertex_lighting(input, scene, object, vec4<f32>(0.0), false);
}

// VSbb68573a uses the same basis/fog with authored per-placement vertex light.
fn native_scripted_model_vertex_lighting(
    input: NativeModelVertexInput, scene: NativeModelScene,
    object: NativeModelObject, vertex_light: vec4<f32>, baked: bool,
) -> NativeScriptedVaryings {
    let model = native_model_vertex_lighting(input, scene, object, vertex_light, baked);
    return NativeScriptedVaryings(model.clip_position, model.fog_color,
        vec4<f32>(model.eye_to_vertex.xyz, model.normal_fog.w), model.color,
        model.normal_fog.xyz, vec4<f32>(model.tangent, sign(input.position.w)),
        model.uv, model.grid_coords_and_vis, model.reflection_correction, model.grid_correction);
}

struct NativeParallaxBasis {
    normal: vec3<f32>,
    bitangent: vec3<f32>,
    ray: vec2<f32>,
}

fn native_scripted_bitangent(normal: vec3<f32>, tangent_sign: vec4<f32>) -> vec3<f32> {
    return fma(normal.yzx, tangent_sign.zxy, -(normal.zxy * tangent_sign.yzx)) * tangent_sign.w;
}

fn native_scripted_parallax_basis(input: NativeScriptedVaryings) -> NativeParallaxBasis {
    let normal = inverseSqrt(dot(input.normal, input.normal)) * input.normal;
    let bitangent = native_scripted_bitangent(normal, input.tangent_sign);
    let eye = inverseSqrt(dot(-input.eye_fog.xyz, -input.eye_fog.xyz)) * -input.eye_fog.xyz;
    let facing = dot(eye, normal);
    let divided = -vec2<f32>(dot(eye, input.tangent_sign.xyz), dot(eye, bitangent)) / facing;
    // Original division precedes its bit-mask. Keep zero/back-facing behavior
    // exactly, including the mask of any division-produced nonfinite bits.
    let mask = select(0u, 0xffffffffu, facing > 0.0);
    let ray = bitcast<vec2<f32>>(bitcast<vec2<u32>>(divided) & vec2<u32>(mask));
    return NativeParallaxBasis(normal, bitangent, ray);
}

fn native_scripted_layer_uv(
    uv: vec2<f32>, ray: vec2<f32>,
    scale: vec2<f32>, offset: vec2<f32>, scroll: vec2<f32>,
    origin: vec2<f32>, rotation: vec2<f32>, depth: f32, seconds: f32,
) -> vec2<f32> {
    let displacement = ray * depth;
    let shifted = fma(offset, scale, fma(uv - origin, scale, displacement * scale));
    let angle = (fma(rotation.y, seconds, 1.0) * rotation.x) * 6.2831854820251465;
    let sine = sin(angle);
    let cosine = cos(angle);
    let rotated = vec2<f32>(dot(vec2<f32>(cosine, sine), shifted),
        dot(vec2<f32>(-sine, cosine), shifted));
    return fma(vec2<f32>(seconds) * scroll, scale, rotated) + origin;
}

struct NativeThreeLayerUvs {
    layer1: vec2<f32>,
    layer2: vec2<f32>,
    layer3: vec2<f32>,
}

fn native_scripted_three_layer_uvs(
    uv: vec2<f32>, ray: vec2<f32>,
    material: NativeModelParallaxConstants, seconds: f32,
) -> NativeThreeLayerUvs {
    let c = material.source;
    return NativeThreeLayerUvs(
        native_scripted_layer_uv(uv, ray, c[0].xy, c[0].zw, c[1].xy,
            c[8].xy, c[5].yz, c[8].z, seconds),
        native_scripted_layer_uv(uv, ray, c[1].zw, c[2].zw, c[2].xy,
            c[7].zw, c[6].xy, c[8].w, seconds),
        native_scripted_layer_uv(uv, ray, c[3].xy, c[3].zw, c[4].xy,
            c[7].xy, c[6].zw, c[9].x, seconds),
    );
}

fn native_scripted_reflection_finish(
    input: NativeScriptedVaryings,
    scene: NativeModelScene,
    normal: vec3<f32>, emission: vec3<f32>, gloss: f32, reflection_amount: f32,
    reflection_probe: texture_cube<f32>, reflection_sampler: sampler,
) -> vec4<f32> {
    let incident = inverseSqrt(dot(input.eye_fog.xyz, input.eye_fog.xyz)) * input.eye_fog.xyz;
    let direction = fma(normal, vec3<f32>(-2.0 * dot(incident, normal)), incident);
    let sample = textureSampleLevel(reflection_probe, reflection_sampler,
        direction, fma(-4.0, gloss, 4.0));
    let probe = (1.0 / (sample.a + 9.999999974752427e-07)) * sample.rgb;
    let reflected = (probe * clamp(fma(0.25, gloss, 0.75), 0.0, 1.0)) * reflection_amount;
    let lit = fma(reflected, input.reflection_correction, emission);
    let fogged = fma(vec3<f32>(input.eye_fog.w), lit - input.fog_color, input.fog_color);
    return vec4<f32>(sqrt(fogged * scene.world.hdr_control0.x), 1.0);
}

// mc_sw4_3d_unlit_3layer_normal_script_q0zez5ju, LITb01f538a, PS654e07c7:
// 1d573639d10331f2045b67ec586b1558ef052b8b739829db9af31e02e821050d.
// scriptVector0.xy gates separate layers; raw X also gates emission before its
// square. It is neither a generic brightness multiplier nor an emissive clone.
fn native_model_crystal_fragment(
    input: NativeScriptedVaryings,
    scene: NativeModelScene, view: NativeModelScriptedScene,
    scripts: NativeModelScriptInputs, material: NativeModelParallaxConstants,
    normal_map: texture_2d<f32>, normal_sampler: sampler,
    layer1_map: texture_2d<f32>, layer1_sampler: sampler,
    layer2_map: texture_2d<f32>, layer2_sampler: sampler,
    layer3_map: texture_2d<f32>, layer3_sampler: sampler,
    reflection_probe: texture_cube<f32>, reflection_sampler: sampler,
) -> vec4<f32> {
    let basis = native_scripted_parallax_basis(input);
    let c = material.source;
    let uv = native_scripted_three_layer_uvs(input.uv, basis.ray, material, view.game_time.w);
    let layer3 = sample_material(scene.world.texture_mip_scale, layer3_map, layer3_sampler, uv.layer3);
    let layer2 = sample_material(scene.world.texture_mip_scale, layer2_map, layer2_sampler, uv.layer2);
    let layer2_delta = fma(-layer3.rgb, vec3<f32>(layer3.a), layer2.rgb);
    let layer3_color = layer3.a * layer3.rgb;
    let alpha = (vec2<f32>(1.0) - clamp(scripts.vectors[0].yx, vec2<f32>(0.0), vec2<f32>(1.0)))
        * c[4].wz;
    let combined2 = fma(vec3<f32>(alpha.x * layer2.a), layer2_delta, layer3_color);
    let layer1 = sample_material(scene.world.texture_mip_scale, layer1_map, layer1_sampler, uv.layer1);
    let composed = fma(vec3<f32>(alpha.y * layer1.a), layer1.rgb - combined2, combined2);
    let colored = composed * input.color.rgb;
    let amount = fma(scripts.vectors[0].x, -c[9].z, c[9].z);
    let gamma_emission = amount * colored;
    let emission = gamma_emission * gamma_emission;
    let sample = sample_material(scene.world.texture_mip_scale, normal_map, normal_sampler, input.uv * c[10].xy);
    let slope = fma(sample.rg, vec2<f32>(4.015748023986816),
        vec2<f32>(-2.0157480239868164)) * c[10].z;
    let mapped = fma(vec3<f32>(slope.y), basis.bitangent,
        fma(vec3<f32>(slope.x), input.tangent_sign.xyz, basis.normal));
    let normal = inverseSqrt(dot(mapped, mapped)) * mapped;
    return native_scripted_reflection_finish(input, scene, normal, emission, c[9].y, c[9].w,
        reflection_probe, reflection_sampler);
}

// mc_sw4_3d_emissive_tomb_qww536q2, LIT883af603, PS88e8a7be:
// 09aae4cb2bde110854a104fba87436bc1055f3f313b9b3ffa2bba3c9bce103b0.
// Required original sunPosition/sunDiffuse, atlas, probe and scriptVector3.
fn native_model_phong_emissive_fragment(
    input: NativeScriptedVaryings,
    scene: NativeModelScene, view: NativeModelScriptedScene,
    glow_multiplier: vec4<f32>, material: NativeModelEmissiveConstants, transparent: bool,
    normal_map: texture_2d<f32>, normal_sampler: sampler,
    color_map: texture_2d<f32>, color_sampler: sampler,
    specular_map: texture_2d<f32>, specular_sampler: sampler,
    glow_map: texture_2d<f32>, glow_sampler: sampler,
    model_lighting: texture_3d<f32>, model_lighting_sampler: sampler,
    reflection_probe: texture_cube<f32>, reflection_sampler: sampler,
) -> vec4<f32> {
    let incident = inverseSqrt(dot(input.eye_fog.xyz, input.eye_fog.xyz)) * input.eye_fog.xyz;
    let half_raw = fma(-input.eye_fog.xyz,
        vec3<f32>(inverseSqrt(dot(input.eye_fog.xyz, input.eye_fog.xyz))), view.sun_position.xyz);
    let half_vector = inverseSqrt(dot(half_raw, half_raw)) * half_raw;
    let geometric = inverseSqrt(dot(input.normal, input.normal)) * input.normal;
    let bitangent = native_scripted_bitangent(geometric, input.tangent_sign);
    let normal_sample = sample_material(scene.world.texture_mip_scale, normal_map, normal_sampler, input.uv);
    let slopes = fma(normal_sample.rg, vec2<f32>(4.015748023986816), vec2<f32>(-2.0157480239868164));
    let mapped = fma(vec3<f32>(slopes.y), bitangent,
        fma(vec3<f32>(slopes.x), input.tangent_sign.xyz, geometric));
    let normal = inverseSqrt(dot(mapped, mapped)) * mapped;
    let ndoth = clamp(dot(normal, half_vector), 0.0, 1.0);
    let hdotsun = clamp(dot(half_vector, view.sun_position.xyz), 0.0, 1.0);
    let specular_sample = sample_material(scene.world.texture_mip_scale, specular_map, specular_sampler, input.uv);
    let gloss = specular_sample.a * material.gloss_push.x;
    let exponent = exp2(gloss * 13.0);
    let lobe = exp2(log2(ndoth) * exponent) * fma(exponent, 0.125, 0.25);
    let fresnel_direct = exp2(hdotsun * -10.0);
    let visibility_gloss = clamp(fma(specular_sample.a, material.gloss_push.x,
        0.5450000166893005), 0.0, 1.0);
    let specular_gamma = specular_sample.rgb * material.controls.w;
    let specular = specular_gamma * specular_gamma;
    let denominator = fma(hdotsun * hdotsun, visibility_gloss, -visibility_gloss) + 1.0099999904632568;
    let attenuation = clamp(dot(view.sun_position.xyz, normal), 0.0, 1.0) / denominator;
    let fresnel_color = fma(fma(-specular_gamma, specular_gamma, vec3<f32>(1.0)),
        vec3<f32>(fresnel_direct), specular);
    let sun = input.grid_coords_and_vis.w * view.sun_diffuse.xyz;
    let direct_specular = ((lobe * attenuation) * fresnel_color) * material.controls.y * sun;
    let reflection_direction = fma(normal, vec3<f32>(-2.0 * dot(incident, normal)), incident);
    let fresnel = exp2(clamp(dot(normal, -incident), 0.0, 1.0) * -9.279999732971191);
    let coefficients = fma(vec4<f32>(gloss),
        vec4<f32>(1.0416667461395264, 0.4749999940395355, 0.018229199573397636, 0.25),
        vec4<f32>(0.0, 0.0, -0.015625, 0.75));
    let probe_sample = textureSampleLevel(reflection_probe, reflection_sampler,
        reflection_direction, fma(-4.0, gloss, 4.0));
    let probe = (1.0 / (probe_sample.a + 9.999999974752427e-07)) * probe_sample.rgb;
    let response_a = fma(coefficients.x, min(fresnel, coefficients.y), coefficients.z);
    let reflected = clamp(fma(specular, vec3<f32>(coefficients.w - response_a), vec3<f32>(response_a)),
        vec3<f32>(0.0), vec3<f32>(1.0)) * probe * material.controls.x;
    var lit = fma(reflected, input.reflection_correction, direct_specular);
    let color_sample = sample_material(scene.world.texture_mip_scale, color_map, color_sampler, input.uv);
    let colored = color_sample.rgb * input.color.rgb;
    let diffuse = colored * colored;
    let irradiance = native_model_atlas_irradiance(normal, input.grid_coords_and_vis.xyz,
        scene, model_lighting, model_lighting_sampler) * input.grid_correction * 32.0;
    let diffuse_light = fma(vec3<f32>(clamp(dot(view.sun_position.xyz, normal), 0.0, 1.0)), sun, irradiance);
    lit = fma(diffuse, diffuse_light, lit);
    let glow = sample_material(scene.world.texture_mip_scale, glow_map, glow_sampler, input.uv) * glow_multiplier;
    let gamma_emission = (glow.a * glow.rgb) * material.controls.z;
    lit = fma(gamma_emission, gamma_emission, lit);
    let fogged = fma(vec3<f32>(input.eye_fog.w), lit - input.fog_color, input.fog_color);
    // PSd217360d outputs square-root alpha; original material state uses gt0.
    let alpha = select(1.0, sqrt(color_sample.a * input.color.a), transparent);
    if transparent && !(alpha > 0.0) { discard; }
    return vec4<f32>(sqrt(fogged * scene.world.hdr_control0.x), alpha);
}

struct NativeEyeVaryings {
    clip_position: vec4<f32>,
    fog_color: vec3<f32>,
    eye_fog: vec4<f32>,
    uv: vec2<f32>,
}

// mc_sw4_3d_model_unlit_cheap_zombie_eyes_jq3e7eqw, LIT673b5a2b.
// VS6a8c6a7c SHA21d1d5b4d04d2a48579b4fb1ca3e956e3da7d18768fbd9a9abb593b028075c77.
// Position and one UV are the complete source input contract: no vertex color,
// normal, tangent, probe or atlas is sampled by this eye program.
fn native_model_eye_vertex(
    position_xyz: vec3<f32>, uv: vec2<f32>, scene: NativeModelScene, object: NativeModelObject,
) -> NativeEyeVaryings {
    let position = vec4<f32>(position_xyz, 1.0);
    let world = vec4<f32>(dot(position, object.world_rows[0]),
        dot(position, object.world_rows[1]), dot(position, object.world_rows[2]),
        dot(position, object.world_rows[3]));
    let distance_squared = dot(world.xyz, world.xyz);
    let distance = sqrt(distance_squared);
    let direction = inverseSqrt(distance_squared) * world.xyz;
    let sun_amount = clamp(fma(scene.world.sun_fog.y,
        dot(scene.world.sun_fog_dir.xyz, direction), scene.world.sun_fog.x), 0.0, 1.0);
    let fog = fma(vec4<f32>(sun_amount), scene.world.sun_fog_color - scene.world.fog_color,
        scene.world.fog_color);
    let height_term = fma(scene.world.fog_consts.w, world.z, scene.world.fog_consts.x);
    let height_exp = exp2(min(height_term, 64.0) * 1.4426950216293335);
    let height_integral = select(height_term + 1.0, height_exp, height_term < 0.0);
    let height_delta = world.z * scene.world.fog_consts.w;
    let horizontal = abs(height_delta) < 9.999999747378752e-05;
    let denominator = select(height_delta, 1.0, horizontal);
    let integral = select((height_integral - scene.world.fog_consts2.x) / denominator,
        clamp(scene.world.fog_consts2.x, 0.0, 1.0), horizontal) * scene.world.fog_consts.y;
    let amount = 1.0 - min(exp2(fma(integral, distance, scene.world.fog_consts.z)), 1.0);
    let clip = vec4<f32>(dot(world, scene.world.view_projection_rows[0]),
        dot(world, scene.world.view_projection_rows[1]), dot(world, scene.world.view_projection_rows[2]),
        dot(world, scene.world.view_projection_rows[3]));
    return NativeEyeVaryings(clip, fog.xyz, vec4<f32>(world.xyz, fma(-amount, fog.w, 1.0)), uv);
}

// PSee6b6abd SHAb675cb3b38c47ca83937763fe741d2926e161b098bd29253a32ecab5cac4c3d2.
// The source explicitly uses mip0; scriptVector2.y scales gamma RGB before its
// square, and alpha is opaque. Eye-name heuristics and emissive edits are absent.
fn native_model_eye_fragment(
    input: NativeEyeVaryings, scene: NativeModelScene, scripts: NativeModelScriptInputs,
    color_map: texture_2d<f32>, color_sampler: sampler,
) -> vec4<f32> {
    let sample = textureSampleLevel(color_map, color_sampler, input.uv, 0.0);
    let colored = sample.rgb * scripts.vectors[2].y;
    let fog_delta = fma(colored, colored, -input.fog_color);
    let fogged = fma(vec3<f32>(input.eye_fog.w), fog_delta, input.fog_color);
    return vec4<f32>(sqrt(fogged * scene.world.hdr_control0.x), 1.0);
}

// mc_sw4_3d_char_cloth_4z8fq5wu, LITb0d91c7, PS49ec0efa.
// SHA256 14053d7a2689973c49430b8cbe7a04e4b8c6d352cda47fd64cb600fab4cf7f19.
// Cloth squares vertex RGB before the diffuse square; vertex alpha scales both
// probe and atlas, scriptVector0.x suppresses specular, then cb0[114..116]
// transforms the combined lighting before fog and HDR encoding.
// Skin PS07f1d600 SHA256 785e8ef0f0936016ce9dd18f67cdcad1c15bdbed58c174e93049df3f46b79b98
// adds reflection/specular amounts and a separate diffuse normal slope scale.
// Cloth PS49ec0efa uses the same calculation with all three controls equal to 1.
// Transparent viewmodel PSd3851d22 divides specular lighting by raw alpha,
// uses vertex RGB once, omits hero correction, and outputs square-root alpha.
// PS SHA256 c6feed7dc04625723b5d8fdf4bba166468c9c40ec41d0c5efb44afa9470d9800.
// Its VS1a1ccc66 adds only clip depth to an interpolant unused by this PS.
fn native_model_phong_fragment(
    input: NativeScriptedVaryings,
    scene: NativeModelScene, view: NativeModelScriptedScene,
    specular_suppression: f32, controls: vec3<f32>, transparent: bool,
    normal_map: texture_2d<f32>, normal_sampler: sampler,
    color_map: texture_2d<f32>, color_sampler: sampler,
    specular_map: texture_2d<f32>, specular_sampler: sampler,
    model_lighting: texture_3d<f32>, model_lighting_sampler: sampler,
    reflection_probe: texture_cube<f32>, reflection_sampler: sampler,
) -> vec4<f32> {
    let color_sample = sample_material(scene.world.texture_mip_scale, color_map, color_sampler, input.uv);
    let vertex_color = select(input.color.rgb * input.color.rgb, input.color.rgb, transparent);
    let colored = color_sample.rgb * vertex_color;
    return native_model_phong_shade(input, scene, view, specular_suppression, controls,
        transparent, true, colored * colored, color_sample.a * input.color.a, vec3<f32>(0.0),
        normal_map, normal_sampler, specular_map, specular_sampler,
        model_lighting, model_lighting_sampler, reflection_probe, reflection_sampler);
}

fn native_model_phong_shade(
    input: NativeScriptedVaryings,
    scene: NativeModelScene, view: NativeModelScriptedScene,
    specular_suppression: f32, controls: vec3<f32>, transparent: bool, hero: bool,
    diffuse: vec3<f32>, raw_alpha: f32, emission: vec3<f32>,
    normal_map: texture_2d<f32>, normal_sampler: sampler,
    specular_map: texture_2d<f32>, specular_sampler: sampler,
    model_lighting: texture_3d<f32>, model_lighting_sampler: sampler,
    reflection_probe: texture_cube<f32>, reflection_sampler: sampler,
) -> vec4<f32> {
    return native_model_phong_shade_samples(input, scene, view, specular_suppression,
        controls, transparent, hero, diffuse, raw_alpha, emission,
        sample_material(scene.world.texture_mip_scale, normal_map, normal_sampler, input.uv).rg,
        sample_material(scene.world.texture_mip_scale, specular_map, specular_sampler, input.uv),
        model_lighting, model_lighting_sampler, reflection_probe, reflection_sampler);
}

fn native_model_phong_shade_samples(
    input: NativeScriptedVaryings,
    scene: NativeModelScene, view: NativeModelScriptedScene,
    specular_suppression: f32, controls: vec3<f32>, transparent: bool, hero: bool,
    diffuse: vec3<f32>, raw_alpha: f32, emission: vec3<f32>,
    normal_sample: vec2<f32>, specular_sample: vec4<f32>,
    model_lighting: texture_3d<f32>, model_lighting_sampler: sampler,
    reflection_probe: texture_cube<f32>, reflection_sampler: sampler,
) -> vec4<f32> {
    let incident = inverseSqrt(dot(input.eye_fog.xyz, input.eye_fog.xyz)) * input.eye_fog.xyz;
    let half_raw = fma(-input.eye_fog.xyz,
        vec3<f32>(inverseSqrt(dot(input.eye_fog.xyz, input.eye_fog.xyz))), view.sun_position.xyz);
    let half_vector = inverseSqrt(dot(half_raw, half_raw)) * half_raw;
    let geometric = inverseSqrt(dot(input.normal, input.normal)) * input.normal;
    let bitangent = native_scripted_bitangent(geometric, input.tangent_sign);
    let slopes = fma(normal_sample, vec2<f32>(4.015748023986816), vec2<f32>(-2.0157480239868164));
    let mapped = fma(vec3<f32>(slopes.y), bitangent,
        fma(vec3<f32>(slopes.x), input.tangent_sign.xyz, geometric));
    let normal = inverseSqrt(dot(mapped, mapped)) * mapped;
    let ndoth = clamp(dot(normal, half_vector), 0.0, 1.0);
    let hdotsun = clamp(dot(half_vector, view.sun_position.xyz), 0.0, 1.0);
    let gloss = specular_sample.a;
    let exponent = exp2(gloss * 13.0);
    let lobe = exp2(log2(ndoth) * exponent) * fma(exponent, 0.125, 0.25);
    let fresnel_direct = exp2(hdotsun * -10.0);
    let visibility_gloss = clamp(specular_sample.a + 0.5450000166893005, 0.0, 1.0);
    let specular_gamma = specular_sample.rgb * (1.0 - clamp(specular_suppression, 0.0, 1.0));
    let specular = specular_gamma * specular_gamma;
    let denominator = fma(hdotsun * hdotsun, visibility_gloss, -visibility_gloss) + 1.0099999904632568;
    let attenuation = clamp(dot(view.sun_position.xyz, normal), 0.0, 1.0) / denominator;
    let fresnel_color = fma(fma(-specular_gamma, specular_gamma, vec3<f32>(1.0)),
        vec3<f32>(fresnel_direct), specular);
    let sun = input.grid_coords_and_vis.w * view.sun_diffuse.xyz;
    let direct_specular = (((lobe * attenuation) * fresnel_color) * controls.y) * sun;
    let reflection_direction = fma(normal, vec3<f32>(-2.0 * dot(incident, normal)), incident);
    let fresnel = exp2(clamp(dot(normal, -incident), 0.0, 1.0) * -9.279999732971191);
    let coefficients = fma(vec4<f32>(gloss),
        vec4<f32>(1.0416667461395264, 0.4749999940395355, 0.018229199573397636, 0.25),
        vec4<f32>(0.0, 0.0, -0.015625, 0.75));
    let probe_sample = textureSampleLevel(reflection_probe, reflection_sampler,
        reflection_direction, fma(-4.0, gloss, 4.0));
    let probe = (1.0 / (probe_sample.a + 9.999999974752427e-07)) * probe_sample.rgb;
    let response_a = fma(coefficients.x, min(fresnel, coefficients.y), coefficients.z);
    let reflected = clamp(fma(specular, vec3<f32>(coefficients.w - response_a), vec3<f32>(response_a)),
        vec3<f32>(0.0), vec3<f32>(1.0)) * probe * (select(input.color.a, 1.0, transparent) * controls.x);
    var lit = fma(reflected, input.reflection_correction, direct_specular);
    if transparent { lit /= raw_alpha; }
    let diffuse_slopes = slopes * controls.z;
    let diffuse_mapped = fma(vec3<f32>(diffuse_slopes.y), bitangent,
        fma(vec3<f32>(diffuse_slopes.x), input.tangent_sign.xyz, geometric));
    let diffuse_normal = diffuse_mapped * inverseSqrt(dot(diffuse_mapped, diffuse_mapped));
    let irradiance = native_model_atlas_irradiance(diffuse_normal, input.grid_coords_and_vis.xyz,
        scene, model_lighting, model_lighting_sampler) * select(input.color.a, 1.0, transparent) * 32.0;
    let diffuse_light = fma(vec3<f32>(clamp(dot(view.sun_position.xyz, diffuse_normal), 0.0, 1.0)), sun, irradiance);
    lit = fma(diffuse, diffuse_light, lit) + emission;
    if hero && !transparent {
        let hero_input = vec4<f32>(lit, 1.0);
        lit = vec3<f32>(dot(hero_input, view.hero_lighting[0]),
            dot(hero_input, view.hero_lighting[1]), dot(hero_input, view.hero_lighting[2]));
    }
    let fogged = fma(vec3<f32>(input.eye_fog.w), lit - input.fog_color, input.fog_color);
    let alpha = select(1.0, sqrt(raw_alpha), transparent);
    if transparent && !(alpha > 0.0) { discard; }
    return vec4<f32>(sqrt(fogged * scene.world.hdr_control0.x), alpha);
}

struct NativeClothRevealConstants { source: array<vec4<f32>, 4> }

// LIT35345000/e0e8d5d3: Temporal_Map.r and scriptVector3.xy control
// the two material states. The cutout variant applies vertex alpha twice.
fn native_model_cloth_reveal_fragment(
    input: NativeScriptedVaryings, scene: NativeModelScene, view: NativeModelScriptedScene,
    scripts: NativeModelScriptInputs, material: NativeClothRevealConstants, alpha_test: bool,
    normal_map: texture_2d<f32>, normal_sampler: sampler,
    color_map: texture_2d<f32>, color_sampler: sampler,
    specular_map: texture_2d<f32>, specular_sampler: sampler,
    temporal_map: texture_2d<f32>, temporal_sampler: sampler,
    model_lighting: texture_3d<f32>, model_lighting_sampler: sampler,
    reflection_probe: texture_cube<f32>, reflection_sampler: sampler,
) -> vec4<f32> {
    let temporal = sample_material(scene.world.texture_mip_scale, temporal_map, temporal_sampler, input.uv).r;
    let thresholds = vec2<f32>(scripts.vectors[3].x) + vec2<f32>(0.05999999865889549, -0.029999999329447746);
    let ramps = clamp((vec2<f32>(temporal) - thresholds)
        * vec2<f32>(-8.333333015441895, 16.66666603088379), vec2<f32>(0.0), vec2<f32>(1.0));
    let curves = (ramps * ramps) * fma(ramps, vec2<f32>(-2.0), vec2<f32>(3.0));
    let sample = sample_material(scene.world.texture_mip_scale, color_map, color_sampler, input.uv);
    let base = sample * input.color;
    let state1 = base.r * material.source[2].rgb;
    let altered = fma(vec3<f32>(scripts.vectors[3].y),
        fma(vec3<f32>(base.r), material.source[3].rgb, -state1), state1);
    let blended = fma(vec4<f32>(curves.x), fma(-sample, input.color, vec4<f32>(altered, base.a)), base);
    let colored = blended.rgb * input.color.rgb;
    let alpha = blended.a * input.color.a;
    if alpha_test && fma(blended.a, input.color.a, -0.501960813999176) < 0.0 { discard; }
    let edge_color = fma(vec3<f32>(scripts.vectors[3].y),
        material.source[1].rgb - material.source[0].yzw, material.source[0].yzw);
    let edge = (edge_color * (curves.y * curves.x)) * material.source[0].x;
    let result = native_model_phong_shade(input, scene, view, scripts.vectors[0].x,
        vec3<f32>(1.0, material.source[3].w, 1.0), false, true, colored * colored, 1.0,
        (edge * edge) * input.color.a, normal_map, normal_sampler,
        specular_map, specular_sampler, model_lighting, model_lighting_sampler,
        reflection_probe, reflection_sampler);
    return vec4<f32>(result.rgb, select(1.0, alpha, alpha_test));
}

// Script-multi LIT9e60f6fe / PSedc347f5, SHA256 e5b60c20b36e0daea90154c5270d5dd5520579ddf25984ef9712fcea33746121.
// Red-channel gloss, raw RGB specular, uncorrected atlas, and scriptVector0.z
// threshold emission are distinct from the Alcatraz/tomb emissive programs.
// Opaque PSf502551a uses the same RGB evaluation, no discard, and constant alpha 1.
fn native_model_emissive_multi_fragment(
    input: NativeScriptedVaryings,
    scene: NativeModelScene, view: NativeModelScriptedScene,
    scripts: NativeModelScriptInputs, material: NativeModelEmissiveConstants, alpha_test: bool,
    normal_map: texture_2d<f32>, normal_sampler: sampler,
    color_map: texture_2d<f32>, color_sampler: sampler,
    specular_map: texture_2d<f32>, specular_sampler: sampler,
    glow_map: texture_2d<f32>, glow_sampler: sampler,
    model_lighting: texture_3d<f32>, model_lighting_sampler: sampler,
    reflection_probe: texture_cube<f32>, reflection_sampler: sampler,
) -> vec4<f32> {
    let color_sample = sample_material(scene.world.texture_mip_scale, color_map, color_sampler, input.uv);
    // Original fused ge128 comparison; output alpha remains the raw product.
    if alpha_test && fma(color_sample.a, input.color.a, -0.501960813999176) < 0.0 { discard; }
    let incident = inverseSqrt(dot(input.eye_fog.xyz, input.eye_fog.xyz)) * input.eye_fog.xyz;
    let half_raw = fma(-input.eye_fog.xyz,
        vec3<f32>(inverseSqrt(dot(input.eye_fog.xyz, input.eye_fog.xyz))), view.sun_position.xyz);
    let half_vector = inverseSqrt(dot(half_raw, half_raw)) * half_raw;
    let geometric = inverseSqrt(dot(input.normal, input.normal)) * input.normal;
    let bitangent = native_scripted_bitangent(geometric, input.tangent_sign);
    let normal_sample = sample_material(scene.world.texture_mip_scale, normal_map, normal_sampler, input.uv);
    let slopes = fma(normal_sample.rg, vec2<f32>(4.015748023986816), vec2<f32>(-2.0157480239868164));
    let mapped = fma(vec3<f32>(slopes.y), bitangent,
        fma(vec3<f32>(slopes.x), input.tangent_sign.xyz, geometric));
    let normal = inverseSqrt(dot(mapped, mapped)) * mapped;
    let ndoth = clamp(dot(normal, half_vector), 0.0, 1.0);
    let hdotsun = clamp(dot(half_vector, view.sun_position.xyz), 0.0, 1.0);
    let specular_sample = sample_material(scene.world.texture_mip_scale, specular_map, specular_sampler, input.uv);
    let gloss = specular_sample.r;
    let exponent = exp2(gloss * 13.0);
    let lobe = exp2(log2(ndoth) * exponent) * fma(exponent, 0.125, 0.25);
    let fresnel_direct = exp2(hdotsun * -10.0);
    let visibility_gloss = clamp(specular_sample.r + 0.5450000166893005, 0.0, 1.0);
    let specular_gamma = specular_sample.rgb;
    let specular = specular_gamma * specular_gamma;
    let denominator = fma(hdotsun * hdotsun, visibility_gloss, -visibility_gloss) + 1.0099999904632568;
    let attenuation = clamp(dot(view.sun_position.xyz, normal), 0.0, 1.0) / denominator;
    let fresnel_color = fma(fma(-specular_gamma, specular_gamma, vec3<f32>(1.0)),
        vec3<f32>(fresnel_direct), specular);
    let sun = input.grid_coords_and_vis.w * view.sun_diffuse.xyz;
    let direct_specular = ((lobe * attenuation) * fresnel_color) * material.controls.y * sun;
    let reflection_direction = fma(normal, vec3<f32>(-2.0 * dot(incident, normal)), incident);
    let fresnel = exp2(clamp(dot(normal, -incident), 0.0, 1.0) * -9.279999732971191);
    let coefficients = fma(vec4<f32>(gloss),
        vec4<f32>(1.0416667461395264, 0.4749999940395355, 0.018229199573397636, 0.25),
        vec4<f32>(0.0, 0.0, -0.015625, 0.75));
    let probe_sample = textureSampleLevel(reflection_probe, reflection_sampler,
        reflection_direction, fma(-4.0, gloss, 4.0));
    let probe = (1.0 / (probe_sample.a + 9.999999974752427e-07)) * probe_sample.rgb;
    let response_a = fma(coefficients.x, min(fresnel, coefficients.y), coefficients.z);
    let reflected = clamp(fma(specular, vec3<f32>(coefficients.w - response_a), vec3<f32>(response_a)),
        vec3<f32>(0.0), vec3<f32>(1.0)) * probe * material.controls.x;
    var lit = fma(reflected, input.reflection_correction, direct_specular);
    let colored = color_sample.rgb * input.color.rgb;
    let diffuse = colored * colored;
    let irradiance = native_model_atlas_irradiance(normal, input.grid_coords_and_vis.xyz,
        scene, model_lighting, model_lighting_sampler) * 32.0;
    let diffuse_light = fma(vec3<f32>(clamp(dot(view.sun_position.xyz, normal), 0.0, 1.0)), sun, irradiance);
    lit = fma(diffuse, diffuse_light, lit);
    let glow = sample_material(scene.world.texture_mip_scale, glow_map, glow_sampler, input.uv);
    let emission_scale = select(0.0, 1.0, scripts.vectors[0].z >= material.controls.w) * material.controls.z;
    let gamma_emission = emission_scale * (glow.a * glow.rgb);
    lit = fma(gamma_emission, gamma_emission, lit);
    let fogged = fma(vec3<f32>(input.eye_fog.w), lit - input.fog_color, input.fog_color);
    return vec4<f32>(sqrt(fogged * scene.world.hdr_control0.x), select(1.0, color_sample.a * input.color.a, alpha_test));
}

struct NativeClanTagConstants {
    controls: vec4<f32>, // reflection, detail tiling, specular, row count
    atlas: vec4<f32>, // character dimensions, derivative scales
    characters: vec4<f32>, // original weaponParam1
}

// Player icon PS616ad5fa SHA256 2036e1b8060a5f5710b1fb043308ae31fe76f5ecdc8847bfb377cc458a7ab2c5.
// Controls are reflection amount, specular amount and detail normal tiling.
// VS1a1ccc66 shares every consumed interpolant with the scripted model vertex.
fn native_model_player_icon_fragment(
    input: NativeScriptedVaryings, scene: NativeModelScene, view: NativeModelScriptedScene,
    controls: vec3<f32>, normal_map: texture_2d<f32>, normal_sampler: sampler,
    color_map: texture_2d<f32>, color_sampler: sampler,
    specular_map: texture_2d<f32>, specular_sampler: sampler,
    model_lighting: texture_3d<f32>, model_lighting_sampler: sampler,
    reflection_probe: texture_cube<f32>, reflection_sampler: sampler,
) -> vec4<f32> {
    let geometric = input.normal * inverseSqrt(dot(input.normal, input.normal));
    let bitangent = native_scripted_bitangent(geometric, input.tangent_sign);
    let normal_sample = sample_material(scene.world.texture_mip_scale, normal_map, normal_sampler, input.uv * controls.z);
    let slopes = fma(normal_sample.rg, vec2<f32>(4.015748023986816), vec2<f32>(-2.0157480239868164));
    let mapped = fma(vec3<f32>(slopes.y), bitangent,
        fma(vec3<f32>(slopes.x), input.tangent_sign.xyz, geometric));
    let normal = mapped * inverseSqrt(dot(mapped, mapped));
    let eye_length = inverseSqrt(dot(input.eye_fog.xyz, input.eye_fog.xyz));
    let incident = eye_length * input.eye_fog.xyz;
    let half_raw = fma(-input.eye_fog.xyz, vec3<f32>(eye_length), view.sun_position.xyz);
    let reflection_direction = fma(normal, vec3<f32>(-2.0 * dot(incident, normal)), incident);
    let fresnel = exp2(clamp(dot(normal, -incident), 0.0, 1.0) * -9.279999732971191);
    let response = fma(min(fresnel, 0.4749999940395355), 1.0416667461395264, 0.0026042000390589237);
    let probe_sample = textureSampleLevel(reflection_probe, reflection_sampler, reflection_direction, 0.0);
    let probe = (1.0 / (probe_sample.a + 9.999999974752427e-07)) * probe_sample.rgb;
    let specular_sample = sample_material(scene.world.texture_mip_scale, specular_map, specular_sampler, input.uv).rgb;
    let specular = specular_sample * specular_sample;
    let fresnel_color = fma(-specular_sample, specular_sample, vec3<f32>(1.0));
    let reflected = (probe * min(fma(specular, vec3<f32>(1.0 - response), vec3<f32>(response)),
        vec3<f32>(1.0))) * controls.x;
    let half_vector = half_raw * inverseSqrt(dot(half_raw, half_raw));
    let hdotsun = clamp(dot(half_vector, view.sun_position.xyz), 0.0, 1.0);
    let ndoth = clamp(dot(normal, half_vector), 0.0, 1.0);
    let lobe = exp2(log2(ndoth) * 8192.0) * 1024.25;
    let denominator = fma(hdotsun, hdotsun, 0.009999999776482582);
    let direct_fresnel = fma(fresnel_color, vec3<f32>(exp2(hdotsun * -10.0)), specular);
    let ndotsun = clamp(dot(view.sun_position.xyz, normal), 0.0, 1.0);
    let attenuation = (ndotsun / denominator) * lobe;
    let sun = input.grid_coords_and_vis.w * view.sun_diffuse.xyz;
    let direct_specular = ((direct_fresnel * attenuation) * controls.y) * sun;
    let lit_specular = fma(reflected, input.reflection_correction, direct_specular);
    let irradiance = native_model_atlas_irradiance(normal, input.grid_coords_and_vis.xyz,
        scene, model_lighting, model_lighting_sampler) * 32.0;
    let diffuse_light = fma(vec3<f32>(ndotsun), sun, irradiance);
    let color = sample_material(scene.world.texture_mip_scale, color_map, color_sampler, input.uv) * input.color;
    let lit = fma(color.rgb * color.rgb, diffuse_light, lit_specular);
    let hero_input = vec4<f32>(lit, 1.0);
    let hero = vec3<f32>(dot(hero_input, view.hero_lighting[0]),
        dot(hero_input, view.hero_lighting[1]), dot(hero_input, view.hero_lighting[2]));
    let fogged = fma(vec3<f32>(input.eye_fog.w), hero - input.fog_color, input.fog_color);
    let alpha = sqrt(color.a);
    if !(alpha > 0.0) { discard; }
    return vec4<f32>(sqrt(fogged * scene.world.hdr_control0.x), alpha);
}

// PS8e093a29 SHA256 add033436e508c8e918f72bb9628c73cbbfd15eccfd7e03f39069ae0d226819a. VS1a1ccc66 adds only an unused
// clip-depth interpolant to VSb7d697d7; all consumed inputs are identical.
fn native_model_clan_tag_fragment(
    input: NativeScriptedVaryings, scene: NativeModelScene,
    view: NativeModelScriptedScene, material: NativeClanTagConstants,
    normal_map: texture_2d<f32>, normal_sampler: sampler,
    detail_map: texture_2d<f32>, detail_sampler: sampler,
    color_map: texture_2d<f32>, color_sampler: sampler,
    model_lighting: texture_3d<f32>, model_lighting_sampler: sampler,
    reflection_probe: texture_cube<f32>, reflection_sampler: sampler,
) -> vec4<f32> {
    let c = material.controls;
    let a = material.atlas;
    let x = input.uv.x;
    let local_x = select(input.uv.x, select(x - 0.25, select(x - 0.5, x - 0.75, x >= 0.75), x >= 0.5), x >= 0.25);
    let glyph = select(material.characters.x, select(material.characters.y,
        select(material.characters.z, material.characters.w, x >= 0.75), x >= 0.5), x >= 0.25);
    let row = floor(glyph / c.w + 0.0010000000474974513);
    let column = fma(-row, c.w, glyph);
    let uv = vec2<f32>(fma(column, a.x, (local_x * a.x) * 4.0), fma(row, a.y, input.uv.y * a.y));
    let detail_sample = sample_material(scene.world.texture_mip_scale, detail_map, detail_sampler, uv * c.y);
    let dx = dpdx(input.uv) * ((a.x * a.z) * scene.world.texture_mip_scale);
    let dy = ((dpdy(input.uv) * a.w) * a.y) * scene.world.texture_mip_scale;
    let normal_sample = textureSampleGrad(normal_map, normal_sampler, uv, dx, dy);
    let color = textureSampleGrad(color_map, color_sampler, uv, dx, dy) * input.color;
    let base_slope = fma(normal_sample.rg, vec2<f32>(4.015748023986816), vec2<f32>(-2.0157480239868164));
    let slope = fma(detail_sample.rg, vec2<f32>(4.015748023986816), base_slope) + vec2<f32>(-2.0157480239868164);
    let geometric = input.normal * inverseSqrt(dot(input.normal, input.normal));
    let bitangent = native_scripted_bitangent(geometric, input.tangent_sign);
    let mapped = fma(vec3<f32>(slope.y), bitangent, fma(vec3<f32>(slope.x), input.tangent_sign.xyz, geometric));
    let normal = mapped * inverseSqrt(dot(mapped, mapped));
    let inverse_eye = inverseSqrt(dot(input.eye_fog.xyz, input.eye_fog.xyz));
    let incident = input.eye_fog.xyz * inverse_eye;
    let half_raw = fma(-input.eye_fog.xyz, vec3<f32>(inverse_eye), view.sun_position.xyz);
    let direction = fma(normal, vec3<f32>(-2.0 * dot(incident, normal)), incident);
    let probe = textureSampleLevel(reflection_probe, reflection_sampler, direction, 4.0);
    let reflected = ((1.0 / (probe.a + 9.999999974752427e-07)) * probe.rgb * c.x) * input.reflection_correction;
    let half_vector = half_raw * inverseSqrt(dot(half_raw, half_raw));
    let ndoth = clamp(dot(normal, half_vector), 0.0, 1.0);
    let hdotsun = clamp(dot(half_vector, view.sun_position.xyz), 0.0, 1.0);
    let ndotsun = clamp(dot(view.sun_position.xyz, normal), 0.0, 1.0);
    let fresnel = fma(exp2(hdotsun * -10.0), 0.75, 0.25);
    let attenuation = ndotsun / fma(hdotsun * hdotsun, 0.5450000166893005, 0.4650000035762787);
    let specular = ((ndoth * 0.375) * attenuation) * fresnel * c.z;
    let sun = input.grid_coords_and_vis.w * view.sun_diffuse.xyz;
    let reflection_and_sun = fma(reflected, vec3<f32>(0.17578125), specular * sun);
    let irradiance = native_model_atlas_irradiance(normal, input.grid_coords_and_vis.xyz,
        scene, model_lighting, model_lighting_sampler) * 32.0;
    let diffuse_light = fma(vec3<f32>(ndotsun), sun, irradiance);
    let lit = vec4<f32>(fma(color.rgb * color.rgb, diffuse_light, reflection_and_sun), 1.0);
    let hero = vec3<f32>(dot(lit, view.hero_lighting[0]), dot(lit, view.hero_lighting[1]), dot(lit, view.hero_lighting[2]));
    let fogged = fma(vec3<f32>(input.eye_fog.w), hero - input.fog_color, input.fog_color);
    let alpha = sqrt(color.a);
    if !(alpha > 0.0) { discard; }
    return vec4<f32>(sqrt(fogged * scene.world.hdr_control0.x), alpha);
}

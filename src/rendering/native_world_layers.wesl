import package::rendering::native_world::{
    NativeWorldScene, NativeWorldObject, NativeWorldVertexInput,
    native_world_vertex,
};

// T6 LIT only: lit_sm_r0c0n0x0_b1c1n1, b1c1n1v1 and b1c1n1s1v1.
// Shared VS layer_lmap_c9b52d6f, SHA256:
// 426a3d8625a1ef1a53ea0ae7bcf2e20e25ab872d32c7d559ab13b51480343a6f.
// Plain PS layer_lmap_cd368409, SHA256:
// ecd2546d039fee1fd49b7e5d41a8d5c3f5bf9306a63d89a0355bdc1d2799669c.
// Reveal PS layer_lmap_18793676, LIT technique layer_lmap_278e410d, SHA256:
// f6a7735540658474070253af697b4b27dc439725059d754839ec25ac5a1775ae.
// Reveal/specular PS layer_lmap_7f753776, SHA256:
// e8d9c313f784b9c395136138c3d5a043269c535d3c24552b7fbddbbff7bb171c.
// Original DXBC/disassembly: evidence/rendering-20261012/{source,shaders}.

struct NativeWorldLayerVaryings {
    clip_position: vec4<f32>,
    uv_lightmap: vec4<f32>,
    normal_fog: vec4<f32>,
    fog_color: vec3<f32>,
    bitangent: vec3<f32>,
    tangent: vec3<f32>,
    eye_to_vertex: vec3<f32>,
    blend_uv: vec4<f32>,          // Native TEXCOORD6: primary RG, auxiliary UV.
    normal_transform: vec4<f32>,  // Native COLOR1: interpolate UNORM Q unchanged.
    reflection_correction: vec3<f32>,
}

struct NativeWorldRevealMaterial {
    // Native PS cb1[59], PerObjectConsts byte 944, alphaRevealParms1.
    // Only XY are read. The selected material supplies the actual authored value.
    alpha_reveal_parms1: vec4<f32>,
}

// PACKED_WORLD_T1N1: original primary stride 36, auxiliary stride 8.
// Auxiliary +0 = Float16x2 UV, +4 = Unorm8x4 Q. All outputs interpolate
// perspective-correctly. Primary RG are blend inputs, not an albedo tint.
fn native_world_layer_vertex(
    primary: NativeWorldVertexInput,
    layer_uv: vec2<f32>,
    normal_transform: vec4<f32>,
    scene: NativeWorldScene,
    object: NativeWorldObject,
) -> NativeWorldLayerVaryings {
    // Position, fog, decoded basis and inverse reflection SH instructions match
    // the ordinary world VS. Only the source color/auxiliary outputs differ.
    let world = native_world_vertex(primary, scene, object);
    return NativeWorldLayerVaryings(
        world.clip_position, world.uv_lightmap, world.normal_fog,
        world.fog_color, world.bitangent, world.tangent, world.eye_to_vertex,
        vec4<f32>(primary.color.rg, layer_uv), normal_transform,
        world.reflection_correction,
    );
}

fn native_world_layer_normal(
    input: NativeWorldLayerVaryings,
    base_normal: vec2<f32>,
    layer_normal: vec2<f32>,
    weight: f32,
) -> vec3<f32> {
    let layer_slope = fma(layer_normal, vec2<f32>(4.015748023986816), vec2<f32>(-2.0157480239868164));
    // Q is a slope transform, not a packed tangent or an orthonormal basis.
    // Decode after interpolation, exactly as the original PS does.
    let q = fma(input.normal_transform, vec4<f32>(2.0), vec4<f32>(-1.0));
    let transformed_slope = vec2<f32>(dot(layer_slope, q.xy), dot(layer_slope, q.zw));
    let base_slope = fma(base_normal, vec2<f32>(4.015748023986816), vec2<f32>(-2.0157480239868164));
    let slope = fma(vec2<f32>(weight), transformed_slope - base_slope, base_slope);
    let mapped_normal = fma(vec3<f32>(slope.y), input.bitangent,
        fma(vec3<f32>(slope.x), input.tangent, input.normal_fog.xyz));
    return inverseSqrt(dot(mapped_normal, mapped_normal)) * mapped_normal;
}

struct NativeWorldLayerProbe {
    linear_color: vec3<f32>,
    fresnel: f32,
}

fn native_world_layer_probe(
    input: NativeWorldLayerVaryings,
    normal: vec3<f32>,
    gloss: f32,
    reflection_probe: texture_cube<f32>,
    reflection_sampler: sampler,
) -> NativeWorldLayerProbe {
    let incident = inverseSqrt(dot(input.eye_to_vertex, input.eye_to_vertex)) * input.eye_to_vertex;
    let direction = fma(normal, vec3<f32>(-2.0 * dot(incident, normal)), incident);
    let fresnel = exp2(clamp(dot(normal, -incident), 0.0, 1.0) * -9.279999732971191);
    let sample = textureSampleLevel(reflection_probe, reflection_sampler, direction, fma(-4.0, gloss, 4.0));
    let reciprocal_alpha = 1.0 / (sample.a + 9.999999974752427e-07);
    return NativeWorldLayerProbe(reciprocal_alpha * sample.rgb, fresnel);
}

// Identical LIT lightmap/fog/HDR tail in both selected layer pixel programs.
fn native_world_layer_lighting(
    input: NativeWorldLayerVaryings,
    scene: NativeWorldScene,
    normal: vec3<f32>,
    color_gamma: vec3<f32>,
    reflected: vec3<f32>,
    lightmap_secondary: texture_2d<f32>,
    lightmap_sampler: sampler,
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
    // Blend authored colors before their original square transfer. There is no
    // primary vertex RGB tint and no transparency in either of these programs.
    let diffuse = color_gamma * color_gamma;
    let lit = fma(diffuse, diffuse_light, reflected * reflection_light);
    let fogged = fma(vec3<f32>(input.normal_fog.w), lit - input.fog_color, input.fog_color);
    // Same original encoded-target contract as native_world_fragment.
    return vec4<f32>(sqrt(fogged * scene.hdr_control0.x), 1.0);
}

// lit_sm_r0c0n0x0_b1c1n1: native t0/s0 base color, t1/s1 base normal,
// t2/s2 layer color, t3/s3 layer normal, t13/s13 lightmap, t15/s15 probe.
// No material constants are bound. Keep source UNORM views and sampler states.
fn native_world_layer_fragment(
    input: NativeWorldLayerVaryings,
    scene: NativeWorldScene,
    hard_select: bool,
    color_map: texture_2d<f32>, color_sampler: sampler,
    normal_map: texture_2d<f32>, normal_sampler: sampler,
    layer_color_map: texture_2d<f32>, layer_color_sampler: sampler,
    layer_normal_map: texture_2d<f32>, layer_normal_sampler: sampler,
    lightmap_secondary: texture_2d<f32>, lightmap_sampler: sampler,
    reflection_probe: texture_cube<f32>, reflection_sampler: sampler,
) -> vec4<f32> {
    let layer_normal = textureSample(layer_normal_map, layer_normal_sampler, input.blend_uv.zw);
    let base_normal = textureSample(normal_map, normal_sampler, input.uv_lightmap.xy);
    let layer_color = textureSample(layer_color_map, layer_color_sampler, input.blend_uv.zw);
    let weight = layer_color.a * input.blend_uv.y;
    let selected = weight >= 0.5;
    var normal: vec3<f32>;
    if hard_select {
        // PS5d72fcf0 selects entire slopes at exactly0.5; it never discards.
        let base_slope = fma(base_normal.rg, vec2<f32>(4.015748023986816), vec2<f32>(-2.0157480239868164));
        let layer_slope = fma(layer_normal.rg, vec2<f32>(4.015748023986816), vec2<f32>(-2.0157480239868164));
        let q = fma(input.normal_transform, vec4<f32>(2.0), vec4<f32>(-1.0));
        let transformed = vec2<f32>(dot(layer_slope, q.xy), dot(layer_slope, q.zw));
        let slope = select(base_slope, transformed, selected);
        let mapped = fma(vec3<f32>(slope.y), input.bitangent,
            fma(vec3<f32>(slope.x), input.tangent, input.normal_fog.xyz));
        normal = inverseSqrt(dot(mapped, mapped)) * mapped;
    } else {
        normal = native_world_layer_normal(input, base_normal.rg, layer_normal.rg, weight);
    }
    let base_color = textureSample(color_map, color_sampler, input.uv_lightmap.xy);
    // Base alpha stays the gloss even where the layer fully replaces the color.
    let gloss = base_color.a;
    let probe = native_world_layer_probe(input, normal, gloss, reflection_probe, reflection_sampler);
    let strength = fma(gloss, min(probe.fresnel, gloss * 0.4749999940395355),
        fma(gloss, 0.027499999850988388, 0.014999999664723873));
    var color_gamma: vec3<f32>;
    if hard_select { color_gamma = select(base_color.rgb, layer_color.rgb, selected); }
    else { color_gamma = fma(vec3<f32>(weight), layer_color.rgb - base_color.rgb, base_color.rgb); }
    var result = native_world_layer_lighting(input, scene, normal, color_gamma,
        strength * probe.linear_color, lightmap_secondary, lightmap_sampler);
    if hard_select { result.a = select(1.0, weight, selected); }
    return result;
}

// The reveal threshold sequence is identical in PS18793676 and PS7f753776.
// Keep native div_sat behavior without an epsilon or endpoint override.
fn native_world_layer_reveal_weight(
    blend: f32,
    layer_alpha: f32,
    material: NativeWorldRevealMaterial,
) -> f32 {
    let inset = fma(blend, 0.9980000257492065, 0.0010000000474974513);
    let complement = 1.0 - inset;
    let powers = vec2<f32>(-exp2(log2(inset) * material.alpha_reveal_parms1.y),
        exp2(log2(complement) * material.alpha_reveal_parms1.y));
    let thresholds = clamp(fma(vec2<f32>(material.alpha_reveal_parms1.x), powers,
        vec2<f32>(complement)), vec2<f32>(0.0), vec2<f32>(1.0));
    return clamp((layer_alpha - thresholds.x) / (thresholds.y - thresholds.x), 0.0, 1.0);
}

// lit_sm_r0c0n0x0_b1c1n1v1 binds native t0..3, t13, t15 and cb1[59].xy.
// Its gloss remains C0.a and its reflection response matches the plain layer
// family. This program never reads a layer specular texture.
fn native_world_reveal_layer_fragment(
    input: NativeWorldLayerVaryings,
    scene: NativeWorldScene,
    material: NativeWorldRevealMaterial,
    color_map: texture_2d<f32>, color_sampler: sampler,
    normal_map: texture_2d<f32>, normal_sampler: sampler,
    layer_color_map: texture_2d<f32>, layer_color_sampler: sampler,
    layer_normal_map: texture_2d<f32>, layer_normal_sampler: sampler,
    lightmap_secondary: texture_2d<f32>, lightmap_sampler: sampler,
    reflection_probe: texture_cube<f32>, reflection_sampler: sampler,
) -> vec4<f32> {
    let layer_color = textureSample(layer_color_map, layer_color_sampler, input.blend_uv.zw);
    let weight = native_world_layer_reveal_weight(input.blend_uv.y, layer_color.a, material);
    let layer_normal = textureSample(layer_normal_map, layer_normal_sampler, input.blend_uv.zw);
    let base_normal = textureSample(normal_map, normal_sampler, input.uv_lightmap.xy);
    let normal = native_world_layer_normal(input, base_normal.rg, layer_normal.rg, weight);
    let base_color = textureSample(color_map, color_sampler, input.uv_lightmap.xy);
    let gloss = base_color.a;
    let probe = native_world_layer_probe(input, normal, gloss, reflection_probe, reflection_sampler);
    let strength = fma(gloss, min(probe.fresnel, gloss * 0.4749999940395355),
        fma(gloss, 0.027499999850988388, 0.014999999664723873));
    let color_gamma = fma(vec3<f32>(weight), layer_color.rgb - base_color.rgb, base_color.rgb);
    return native_world_layer_lighting(input, scene, normal, color_gamma,
        strength * probe.linear_color, lightmap_secondary, lightmap_sampler);
}

// Shared specular/color/normal lighting tail of lit_sm_r0c0n0x0_b1c1n1s1
// (PS aa3a8509, SHA256 304a7e21cae7f4c99a1cb5f50cac7bcb25ce71e991acb05536736dc2ab2e38c0)
// and its s1v1 reveal variant. Both bind native t4/s4 specular/gloss.
// The entry points supply their original blend weight and sampled C1 color;
// only the reveal variant reads alphaRevealParms1. Neither reads colorTint.
fn native_world_specular_layer_fragment(
    input: NativeWorldLayerVaryings,
    scene: NativeWorldScene,
    layer_color: vec4<f32>,
    weight: f32,
    color_map: texture_2d<f32>, color_sampler: sampler,
    normal_map: texture_2d<f32>, normal_sampler: sampler,
    layer_normal_map: texture_2d<f32>, layer_normal_sampler: sampler,
    layer_specular_map: texture_2d<f32>, layer_specular_sampler: sampler,
    lightmap_secondary: texture_2d<f32>, lightmap_sampler: sampler,
    reflection_probe: texture_cube<f32>, reflection_sampler: sampler,
) -> vec4<f32> {
    let layer_normal = textureSample(layer_normal_map, layer_normal_sampler, input.blend_uv.zw);
    let base_normal = textureSample(normal_map, normal_sampler, input.uv_lightmap.xy);
    let normal = native_world_layer_normal(input, base_normal.rg, layer_normal.rg, weight);
    let layer_specular = textureSample(layer_specular_map, layer_specular_sampler, input.blend_uv.zw);
    let base_color = textureSample(color_map, color_sampler, input.uv_lightmap.xy);
    // Literal base specular is 0.2; base gloss is the actual C0 alpha.
    let base_specular = vec4<f32>(vec3<f32>(0.20000000298023224), base_color.a);
    let specular_gloss = fma(vec4<f32>(weight), layer_specular - base_specular, base_specular);
    let gloss = specular_gloss.a;
    let probe = native_world_layer_probe(input, normal, gloss, reflection_probe, reflection_sampler);
    let coefficients = fma(vec4<f32>(gloss),
        vec4<f32>(1.0416667461395264, 0.4749999940395355, 0.018229199573397636, 0.25),
        vec4<f32>(0.0, 0.0, -0.015625, 0.75));
    let specular = specular_gloss.rgb * specular_gloss.rgb;
    let response_a = fma(coefficients.x, min(probe.fresnel, coefficients.y), coefficients.z);
    let factor = clamp(fma(specular, vec3<f32>(coefficients.w - response_a), vec3<f32>(response_a)),
        vec3<f32>(0.0), vec3<f32>(1.0));
    let color_gamma = fma(vec3<f32>(weight), layer_color.rgb - base_color.rgb, base_color.rgb);
    return native_world_layer_lighting(input, scene, normal, color_gamma,
        probe.linear_color * factor, lightmap_secondary, lightmap_sampler);
}

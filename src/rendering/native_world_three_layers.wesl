import package::rendering::native_world::{
    NativeWorldScene, NativeWorldObject, NativeWorldVertexInput, native_world_vertex,
};
import package::rendering::native_world_layers::{
    NativeWorldLayerVaryings, NativeWorldRevealMaterial,
    native_world_layer_reveal_weight, native_world_layer_lighting, native_world_layer_probe, native_world_layer_normal,
};
import package::rendering::native_world_plain::{
    native_world_plain_specular_reflection,
};

// T6 world LIT three-layer programs, rendering-20261012/{source,shaders}.
// Shared VS layer_lmap_94510929 SHA256:
// f63f59e2d5174509caf55262e50fa98e3e227aa3cf07843d4d167a9e49a99a7b.
// lit_sm_r0c0n0x0_b1c1n1_b2c2n2s2v2: LIT layer_lmap_ff3d175,
// PS60dd249f SHA256:
// 3d2f9cb0df16c04610fad51a6428a8cb70d88668c2c05434462c561fc09d8ea0.
// lit_sm_r0c0n0x0_b1c1n1s1v1_b2c2n2s2v2: LIT layer_lmap_ecc55da2,
// PSd29e97e0 SHA256:
// e0d3428dbbe5ee0d56eefb791f7dc973387721654db4050303108758d1ef422f.

struct NativeWorldThreeLayerVaryings {
    clip_position: vec4<f32>,
    blend_color: vec3<f32>, // Original primary COLOR0; PS reads G/B as weights.
    uv_lightmap: vec4<f32>,
    normal_fog: vec4<f32>,
    fog_color: vec3<f32>,
    bitangent: vec3<f32>,
    tangent: vec3<f32>,
    eye_to_vertex: vec3<f32>,
    layer_uvs: vec4<f32>, // Original TEXCOORD6: UV1.xy, UV2.zw.
    normal_transform1: vec4<f32>, // Original COLOR1: interpolate UNORM Q1.
    normal_transform2: vec4<f32>, // Original TEXCOORD7: interpolate UNORM Q2.
    reflection_correction: vec3<f32>,
}

struct NativeWorldThreeLayerRevealMaterial {
    // PS60dd249f reads original cb1[60].xy, PerObjectConsts byte960.
    alpha_reveal_parms2: vec4<f32>,
}

struct NativeWorldThreeLayerDoubleRevealMaterial {
    // PSd29e97e0 reads original cb1[59].xy and cb1[60].xy, bytes944/960.
    alpha_reveal_parms1: vec4<f32>,
    alpha_reveal_parms2: vec4<f32>,
}

// PACKED_WORLD_T2N2: primary stride36; auxiliary stride16 with Float16x4
// UV1/UV2 at0, Unorm8x4 Q1 at8, Unorm8x4 Q2 at12. Q is slope-transform data,
// decoded in PS after perspective interpolation, independently for each layer.
// Source94510929 shares position/fog/basis/SH instructions with the ordinary
// world VS; this wrapper preserves its different color and auxiliary outputs.
fn native_world_three_layer_vertex(
    primary: NativeWorldVertexInput,
    layer_uvs: vec4<f32>,
    normal_transform1: vec4<f32>,
    normal_transform2: vec4<f32>,
    scene: NativeWorldScene,
    object: NativeWorldObject,
) -> NativeWorldThreeLayerVaryings {
    let world = native_world_vertex(primary, scene, object);
    return NativeWorldThreeLayerVaryings(world.clip_position, primary.color.rgb,
        world.uv_lightmap, world.normal_fog, world.fog_color, world.bitangent,
        world.tangent, world.eye_to_vertex, layer_uvs, normal_transform1,
        normal_transform2, world.reflection_correction);
}

fn native_world_three_layer_transformed_slope(
    normal_sample: vec2<f32>,
    encoded_transform: vec4<f32>,
) -> vec2<f32> {
    let slope = fma(normal_sample, vec2<f32>(4.015748023986816), vec2<f32>(-2.0157480239868164));
    let q = fma(encoded_transform, vec4<f32>(2.0), vec4<f32>(-1.0));
    return vec2<f32>(dot(slope, q.xy), dot(slope, q.zw));
}

// Both source programs first blend slopes N0 -> transformed N1 -> transformed
// N2. They map the final slope through the basis and normalize exactly once.
fn native_world_three_layer_normal(
    input: NativeWorldThreeLayerVaryings,
    base_normal: vec2<f32>,
    layer_normal1: vec2<f32>,
    layer_normal2: vec2<f32>,
    weights: vec2<f32>,
) -> vec3<f32> {
    let base_slope = fma(base_normal, vec2<f32>(4.015748023986816), vec2<f32>(-2.0157480239868164));
    let slope1 = native_world_three_layer_transformed_slope(layer_normal1, input.normal_transform1);
    let blended1 = fma(vec2<f32>(weights.x), slope1 - base_slope, base_slope);
    let slope2 = native_world_three_layer_transformed_slope(layer_normal2, input.normal_transform2);
    let slope = fma(vec2<f32>(weights.y), slope2 - blended1, blended1);
    let mapped_normal = fma(vec3<f32>(slope.y), input.bitangent,
        fma(vec3<f32>(slope.x), input.tangent, input.normal_fog.xyz));
    return inverseSqrt(dot(mapped_normal, mapped_normal)) * mapped_normal;
}

// Lightmap, geometric-normal reflection correction, fog and target encoding
// instructions match native_world_layer_lighting. Populate its common fields
// with the actual first-layer data; that helper never consumes the layer fields.
fn native_world_three_layer_lighting_input(
    input: NativeWorldThreeLayerVaryings,
) -> NativeWorldLayerVaryings {
    return NativeWorldLayerVaryings(input.clip_position, input.uv_lightmap,
        input.normal_fog, input.fog_color, input.bitangent, input.tangent,
        input.eye_to_vertex, vec4<f32>(input.blend_color.rg, input.layer_uvs.xy),
        input.normal_transform1, input.reflection_correction);
}

// PS60dd249f required texture/sampler pairs (registers are not all equal):
// C0 t0/s0, N0 t1/s1, C1 t2/s2, N1 t3/s4, C2 t4/s3, N2 t5/s5,
// S2 t6/s6, secondary lightmap t13/s13, reflection cube t15/s15.
// Preserve each named material sampler and original UNORM samples/mip chains.
// Only alphaRevealParms2 is required; stored tints/occlusion values are unbound.
fn native_world_three_layer_reveal_fragment(
    input: NativeWorldThreeLayerVaryings,
    scene: NativeWorldScene,
    material: NativeWorldThreeLayerRevealMaterial,
    color_map: texture_2d<f32>, color_sampler: sampler,
    normal_map: texture_2d<f32>, normal_sampler: sampler,
    layer_color_map1: texture_2d<f32>, layer_color_sampler1: sampler,
    layer_normal_map1: texture_2d<f32>, layer_normal_sampler1: sampler,
    layer_color_map2: texture_2d<f32>, layer_color_sampler2: sampler,
    layer_normal_map2: texture_2d<f32>, layer_normal_sampler2: sampler,
    layer_specular_map2: texture_2d<f32>, layer_specular_sampler2: sampler,
    lightmap_secondary: texture_2d<f32>, lightmap_sampler: sampler,
    reflection_probe: texture_cube<f32>, reflection_sampler: sampler,
) -> vec4<f32> {
    let layer_normal1 = textureSample(layer_normal_map1, layer_normal_sampler1, input.layer_uvs.xy);
    let base_normal = textureSample(normal_map, normal_sampler, input.uv_lightmap.xy);
    let layer_color1 = textureSample(layer_color_map1, layer_color_sampler1, input.layer_uvs.xy);
    let weight1 = layer_color1.a * input.blend_color.g;
    let layer_normal2 = textureSample(layer_normal_map2, layer_normal_sampler2, input.layer_uvs.zw);
    let layer_color2 = textureSample(layer_color_map2, layer_color_sampler2, input.layer_uvs.zw);
    // The reveal helper's equation is identical for cb1[59] and cb1[60].
    // Its compact argument carries this program's actual alphaRevealParms2.
    let weight2 = native_world_layer_reveal_weight(input.blend_color.b, layer_color2.a,
        NativeWorldRevealMaterial(material.alpha_reveal_parms2));
    let weights = vec2<f32>(weight1, weight2);
    let normal = native_world_three_layer_normal(input, base_normal.rg,
        layer_normal1.rg, layer_normal2.rg, weights);
    return native_world_three_layer_reveal_lighting(input, scene, normal, weight1, weight2,
        layer_color1, layer_color2, color_map, color_sampler,
        layer_specular_map2, layer_specular_sampler2, lightmap_secondary, lightmap_sampler,
        reflection_probe, reflection_sampler);
}

// PSd29e97e0 required pairs: C0 t0/s0, N0 t1/s1, C1 t2/s2, N1 t3/s4,
// S1 t4/s6, C2 t5/s3, N2 t6/s5, S2 t7/s7, LM t13/s13, cube t15/s15.
// Requires alphaRevealParms1 and alphaRevealParms2; all other stored material
// constants remain unbound by this source pass. Both layers reveal separately.
fn native_world_three_layer_double_reveal_fragment(
    input: NativeWorldThreeLayerVaryings,
    scene: NativeWorldScene,
    material: NativeWorldThreeLayerDoubleRevealMaterial,
    color_map: texture_2d<f32>, color_sampler: sampler,
    normal_map: texture_2d<f32>, normal_sampler: sampler,
    layer_color_map1: texture_2d<f32>, layer_color_sampler1: sampler,
    layer_normal_map1: texture_2d<f32>, layer_normal_sampler1: sampler,
    layer_specular_map1: texture_2d<f32>, layer_specular_sampler1: sampler,
    layer_color_map2: texture_2d<f32>, layer_color_sampler2: sampler,
    layer_normal_map2: texture_2d<f32>, layer_normal_sampler2: sampler,
    layer_specular_map2: texture_2d<f32>, layer_specular_sampler2: sampler,
    lightmap_secondary: texture_2d<f32>, lightmap_sampler: sampler,
    reflection_probe: texture_cube<f32>, reflection_sampler: sampler,
) -> vec4<f32> {
    let layer_normal1 = textureSample(layer_normal_map1, layer_normal_sampler1, input.layer_uvs.xy);
    let base_normal = textureSample(normal_map, normal_sampler, input.uv_lightmap.xy);
    let layer_color1 = textureSample(layer_color_map1, layer_color_sampler1, input.layer_uvs.xy);
    let weight1 = native_world_layer_reveal_weight(input.blend_color.g, layer_color1.a,
        NativeWorldRevealMaterial(material.alpha_reveal_parms1));
    let layer_normal2 = textureSample(layer_normal_map2, layer_normal_sampler2, input.layer_uvs.zw);
    let layer_color2 = textureSample(layer_color_map2, layer_color_sampler2, input.layer_uvs.zw);
    let weight2 = native_world_layer_reveal_weight(input.blend_color.b, layer_color2.a,
        NativeWorldRevealMaterial(material.alpha_reveal_parms2));
    let weights = vec2<f32>(weight1, weight2);
    let normal = native_world_three_layer_normal(input, base_normal.rg,
        layer_normal1.rg, layer_normal2.rg, weights);
    let layer_specular1 = textureSample(layer_specular_map1, layer_specular_sampler1, input.layer_uvs.xy);
    let base_color = textureSample(color_map, color_sampler, input.uv_lightmap.xy);
    let base_specular = vec4<f32>(vec3<f32>(0.20000000298023224), base_color.a);
    let specular1 = fma(vec4<f32>(weight1), layer_specular1 - base_specular, base_specular);
    let layer_specular2 = textureSample(layer_specular_map2, layer_specular_sampler2, input.layer_uvs.zw);
    let specular_gloss = fma(vec4<f32>(weight2), layer_specular2 - specular1, specular1);
    let reflected = native_world_plain_specular_reflection(input.eye_to_vertex,
        normal, specular_gloss, reflection_probe, reflection_sampler);
    let color1 = fma(vec3<f32>(weight1), layer_color1.rgb - base_color.rgb, base_color.rgb);
    let color_gamma = fma(vec3<f32>(weight2), layer_color2.rgb - color1, color1);
    return native_world_layer_lighting(native_world_three_layer_lighting_input(input),
        scene, normal, color_gamma, reflected, lightmap_secondary, lightmap_sampler);
}

// PSc6dc192c: first overlay changes color only; only layer2 transforms normals.
fn native_world_color_three_layer_specular_fragment(
    input: NativeWorldThreeLayerVaryings,
    scene: NativeWorldScene,
    layer_color2: vec4<f32>, weight2: f32,
    color_map: texture_2d<f32>, color_sampler: sampler,
    normal_map: texture_2d<f32>, normal_sampler: sampler,
    layer_color_map1: texture_2d<f32>, layer_color_sampler1: sampler,
    layer_normal_map2: texture_2d<f32>, layer_normal_sampler2: sampler,
    layer_specular_map2: texture_2d<f32>, layer_specular_sampler2: sampler,
    lightmap_secondary: texture_2d<f32>, lightmap_sampler: sampler,
    reflection_probe: texture_cube<f32>, reflection_sampler: sampler,
) -> vec4<f32> {
    let base_normal = textureSample(normal_map, normal_sampler, input.uv_lightmap.xy);
    let layer_color1 = textureSample(layer_color_map1, layer_color_sampler1, input.layer_uvs.xy);
    let weight1 = layer_color1.a * input.blend_color.g;
    let layer_normal2 = textureSample(layer_normal_map2, layer_normal_sampler2, input.layer_uvs.zw);
    let normal = native_world_color_three_normal(input, base_normal.rg, layer_normal2.rg, weight2);
    return native_world_three_layer_reveal_lighting(input, scene, normal, weight1, weight2,
        layer_color1, layer_color2, color_map, color_sampler,
        layer_specular_map2, layer_specular_sampler2, lightmap_secondary, lightmap_sampler,
        reflection_probe, reflection_sampler);
}


// Shared source tail after each program has computed its actual normal and weights.
fn native_world_three_layer_reveal_lighting(
    input: NativeWorldThreeLayerVaryings, scene: NativeWorldScene,
    normal: vec3<f32>, weight1: f32, weight2: f32,
    layer_color1: vec4<f32>, layer_color2: vec4<f32>,
    color_map: texture_2d<f32>, color_sampler: sampler,
    layer_specular_map2: texture_2d<f32>, layer_specular_sampler2: sampler,
    lightmap_secondary: texture_2d<f32>, lightmap_sampler: sampler,
    reflection_probe: texture_cube<f32>, reflection_sampler: sampler,
) -> vec4<f32> {
    let layer_specular2 = textureSample(layer_specular_map2, layer_specular_sampler2, input.layer_uvs.zw);
    let base_color = textureSample(color_map, color_sampler, input.uv_lightmap.xy);
    // Layer1 has no specular input. Original literal RGB is0.2 and gloss is
    // C0.a; neither is blended by weight1 before the reveal into S2.
    let base_specular = vec4<f32>(vec3<f32>(0.20000000298023224), base_color.a);
    let specular_gloss = fma(vec4<f32>(weight2), layer_specular2 - base_specular, base_specular);
    // The source reflection response matches the already-ported specular PS:
    // source gloss mip, squared specular RGB, Fresnel coefficients, saturation.
    let reflected = native_world_plain_specular_reflection(input.eye_to_vertex,
        normal, specular_gloss, reflection_probe, reflection_sampler);
    let color1 = fma(vec3<f32>(weight1), layer_color1.rgb - base_color.rgb, base_color.rgb);
    let color_gamma = fma(vec3<f32>(weight2), layer_color2.rgb - color1, color1);
    return native_world_layer_lighting(native_world_three_layer_lighting_input(input),
        scene, normal, color_gamma, reflected, lightmap_secondary, lightmap_sampler);
}

fn native_world_color_three_normal(input: NativeWorldThreeLayerVaryings,
    base_normal: vec2<f32>, layer_normal2: vec2<f32>, weight2: f32) -> vec3<f32> {
    let base_slope = fma(base_normal, vec2<f32>(4.015748023986816), vec2<f32>(-2.0157480239868164));
    let slope2 = native_world_three_layer_transformed_slope(layer_normal2, input.normal_transform2);
    let slope = fma(vec2<f32>(weight2), slope2 - base_slope, base_slope);
    let mapped = fma(vec3<f32>(slope.y), input.bitangent,
        fma(vec3<f32>(slope.x), input.tangent, input.normal_fog.xyz));
    return inverseSqrt(dot(mapped, mapped)) * mapped;
}
// PS76c68e0c: overlays do not alter base gloss, and bind no specular map.
fn native_world_color_three_layer_normal_fragment(
    input: NativeWorldThreeLayerVaryings, scene: NativeWorldScene,
    color_map: texture_2d<f32>, color_sampler: sampler,
    normal_map: texture_2d<f32>, normal_sampler: sampler,
    color_map1: texture_2d<f32>, color_sampler1: sampler,
    color_map2: texture_2d<f32>, color_sampler2: sampler,
    normal_map2: texture_2d<f32>, normal_sampler2: sampler,
    lightmap_secondary: texture_2d<f32>, lightmap_sampler: sampler,
    reflection_probe: texture_cube<f32>, reflection_sampler: sampler,
) -> vec4<f32> {
    let top = textureSample(color_map2, color_sampler2, input.layer_uvs.zw);
    let weight2 = top.a * input.blend_color.b;
    let normal2 = textureSample(normal_map2, normal_sampler2, input.layer_uvs.zw);
    let normal0 = textureSample(normal_map, normal_sampler, input.uv_lightmap.xy);
    let normal = native_world_color_three_normal(input, normal0.rg, normal2.rg, weight2);
    let base = textureSample(color_map, color_sampler, input.uv_lightmap.xy);
    let lighting_input = native_world_three_layer_lighting_input(input);
    let probe = native_world_layer_probe(lighting_input, normal, base.a, reflection_probe, reflection_sampler);
    let strength = fma(base.a, min(probe.fresnel, base.a * 0.4749999940395355),
        fma(base.a, 0.027499999850988388, 0.014999999664723873));
    let reflected = strength * probe.linear_color;
    let layer = textureSample(color_map1, color_sampler1, input.layer_uvs.xy);
    let weight1 = layer.a * input.blend_color.g;
    let color1 = fma(vec3<f32>(weight1), layer.rgb - base.rgb, base.rgb);
    let color = fma(vec3<f32>(weight2), top.rgb - color1, color1);
    return native_world_layer_lighting(lighting_input, scene, normal, color, reflected, lightmap_secondary, lightmap_sampler);
}

// PS675e1822: reveal N1/S1, then multiply RGB and scale gloss by overlay luminance.
// Shared VS3250349a has one packed Q at auxiliary byte8. In this program it
// transforms N1 (the other users of this vertex format transform N2).
fn native_world_reveal_overlay_fragment(
    input: NativeWorldThreeLayerVaryings, scene: NativeWorldScene, material: NativeWorldRevealMaterial, additive: bool, post_multiplier: vec4<f32>,
    color_map: texture_2d<f32>, color_sampler: sampler,
    normal_map: texture_2d<f32>, normal_sampler: sampler,
    color_map1: texture_2d<f32>, color_sampler1: sampler,
    normal_map1: texture_2d<f32>, normal_sampler1: sampler,
    specular_map1: texture_2d<f32>, specular_sampler1: sampler,
    color_map2: texture_2d<f32>, color_sampler2: sampler,
    lightmap_secondary: texture_2d<f32>, lightmap_sampler: sampler,
    reflection_probe: texture_cube<f32>, reflection_sampler: sampler,
) -> vec4<f32> {
    let layer_color = textureSample(color_map1, color_sampler1, input.layer_uvs.xy);
    let weight = native_world_layer_reveal_weight(input.blend_color.g, layer_color.a, material);
    var lighting_input = native_world_three_layer_lighting_input(input);
    lighting_input.normal_transform = input.normal_transform2;
    let normal1 = textureSample(normal_map1, normal_sampler1, input.layer_uvs.xy);
    let normal0 = textureSample(normal_map, normal_sampler, input.uv_lightmap.xy);
    let normal = native_world_layer_normal(lighting_input, normal0.rg, normal1.rg, weight);
    let specular1 = textureSample(specular_map1, specular_sampler1, input.layer_uvs.xy);
    let base = textureSample(color_map, color_sampler, input.uv_lightmap.xy);
    let specular0 = vec4<f32>(vec3<f32>(0.20000000298023224), base.a);
    let specular = fma(vec4<f32>(weight), specular1 - specular0, specular0);
    let overlay = textureSample(color_map2, color_sampler2, input.layer_uvs.zw);
    let revealed_color = fma(vec3<f32>(weight), layer_color.rgb - base.rgb, base.rgb);
    var gloss = specular.a;
    var color: vec3<f32>;
    var alpha = 1.0;
    if additive {
        // PS4751c8d3 preserves gloss and adds vertex.b * C2.rgb * C2.a.
        let added = overlay.rgb * input.blend_color.b;
        color = fma(added, vec3<f32>(overlay.a), revealed_color);
        alpha = clamp(fma(overlay.a, input.blend_color.b, 1.0), 0.0, 1.0);
    } else {
        let modulation = fma(vec3<f32>(input.blend_color.b), overlay.rgb - vec3<f32>(1.0), vec3<f32>(1.0));
        let gloss_scale = dot(modulation, vec3<f32>(0.30000001192092896, 0.6000000238418579, 0.10000000149011612));
        gloss = gloss_scale * gloss;
        color = modulation * revealed_color;
    }
    gloss = post_multiplier.w * gloss;
    color = post_multiplier.xyz * color;
    let reflected = native_world_plain_specular_reflection(input.eye_to_vertex, normal,
        vec4<f32>(specular.rgb, gloss), reflection_probe, reflection_sampler);
    var result = native_world_layer_lighting(lighting_input, scene, normal, color,
        reflected, lightmap_secondary, lightmap_sampler);
    result.a = alpha;
    return result;
}

// PSf5127bac: ordinary layer1 normal/color blend, base gloss, additive layer2.
fn native_world_layer_add_fragment(
    input: NativeWorldThreeLayerVaryings, scene: NativeWorldScene,
    color_map: texture_2d<f32>, color_sampler: sampler,
    normal_map: texture_2d<f32>, normal_sampler: sampler,
    color_map1: texture_2d<f32>, color_sampler1: sampler,
    normal_map1: texture_2d<f32>, normal_sampler1: sampler,
    color_map2: texture_2d<f32>, color_sampler2: sampler,
    lightmap_secondary: texture_2d<f32>, lightmap_sampler: sampler,
    reflection_probe: texture_cube<f32>, reflection_sampler: sampler,
) -> vec4<f32> {
    let layer = textureSample(color_map1, color_sampler1, input.layer_uvs.xy);
    let weight = layer.a * input.blend_color.g;
    var source = native_world_three_layer_lighting_input(input);
    source.normal_transform = input.normal_transform2;
    let normal1 = textureSample(normal_map1, normal_sampler1, input.layer_uvs.xy);
    let normal0 = textureSample(normal_map, normal_sampler, input.uv_lightmap.xy);
    let normal = native_world_layer_normal(source, normal0.rg, normal1.rg, weight);
    let base = textureSample(color_map, color_sampler, input.uv_lightmap.xy);
    let probe = native_world_layer_probe(source, normal, base.a, reflection_probe, reflection_sampler);
    let strength = fma(base.a, min(probe.fresnel, base.a * 0.4749999940395355),
        fma(base.a, 0.027499999850988388, 0.014999999664723873));
    let blended = fma(vec3<f32>(weight), layer.rgb - base.rgb, base.rgb);
    let overlay = textureSample(color_map2, color_sampler2, input.layer_uvs.zw);
    let added = overlay.rgb * input.blend_color.b;
    let color = fma(added, vec3<f32>(overlay.a), blended);
    var result = native_world_layer_lighting(source, scene, normal, color,
        strength * probe.linear_color, lightmap_secondary, lightmap_sampler);
    result.a = clamp(fma(overlay.a, input.blend_color.b, 1.0), 0.0, 1.0);
    return result;
}

// PSd0c7b3bf: two ordinary alpha blends; original base gloss is unchanged.
fn native_world_three_normals_fragment(
    input: NativeWorldThreeLayerVaryings, scene: NativeWorldScene,
    color_map: texture_2d<f32>, color_sampler: sampler,
    normal_map: texture_2d<f32>, normal_sampler: sampler,
    color_map1: texture_2d<f32>, color_sampler1: sampler,
    normal_map1: texture_2d<f32>, normal_sampler1: sampler,
    color_map2: texture_2d<f32>, color_sampler2: sampler,
    normal_map2: texture_2d<f32>, normal_sampler2: sampler,
    lightmap_secondary: texture_2d<f32>, lightmap_sampler: sampler,
    reflection_probe: texture_cube<f32>, reflection_sampler: sampler,
) -> vec4<f32> {
    let layer1 = textureSample(color_map1, color_sampler1, input.layer_uvs.xy);
    let layer2 = textureSample(color_map2, color_sampler2, input.layer_uvs.zw);
    let weights = vec2<f32>(layer1.a * input.blend_color.g, layer2.a * input.blend_color.b);
    let normal0 = textureSample(normal_map, normal_sampler, input.uv_lightmap.xy);
    let normal1 = textureSample(normal_map1, normal_sampler1, input.layer_uvs.xy);
    let normal2 = textureSample(normal_map2, normal_sampler2, input.layer_uvs.zw);
    let normal = native_world_three_layer_normal(input, normal0.rg, normal1.rg, normal2.rg, weights);
    let base = textureSample(color_map, color_sampler, input.uv_lightmap.xy);
    let source = native_world_three_layer_lighting_input(input);
    let probe = native_world_layer_probe(source, normal, base.a, reflection_probe, reflection_sampler);
    let strength = fma(base.a, min(probe.fresnel, base.a * 0.4749999940395355),
        fma(base.a, 0.027499999850988388, 0.014999999664723873));
    let color1 = fma(vec3<f32>(weights.x), layer1.rgb - base.rgb, base.rgb);
    let color = fma(vec3<f32>(weights.y), layer2.rgb - color1, color1);
    return native_world_layer_lighting(source, scene, normal, color,
        strength * probe.linear_color, lightmap_secondary, lightmap_sampler);
}

// PS076b69a5: second reveal affects normals/color only; retain first-layer specular.
fn native_world_three_layer_retained_specular_fragment(
    input: NativeWorldThreeLayerVaryings,
    scene: NativeWorldScene,
    material: NativeWorldThreeLayerDoubleRevealMaterial,
    color_map: texture_2d<f32>, color_sampler: sampler,
    normal_map: texture_2d<f32>, normal_sampler: sampler,
    layer_color_map1: texture_2d<f32>, layer_color_sampler1: sampler,
    layer_normal_map1: texture_2d<f32>, layer_normal_sampler1: sampler,
    layer_specular_map1: texture_2d<f32>, layer_specular_sampler1: sampler,
    layer_color_map2: texture_2d<f32>, layer_color_sampler2: sampler,
    layer_normal_map2: texture_2d<f32>, layer_normal_sampler2: sampler,
    lightmap_secondary: texture_2d<f32>, lightmap_sampler: sampler,
    reflection_probe: texture_cube<f32>, reflection_sampler: sampler,
) -> vec4<f32> {
    let layer_normal1 = textureSample(layer_normal_map1, layer_normal_sampler1, input.layer_uvs.xy);
    let base_normal = textureSample(normal_map, normal_sampler, input.uv_lightmap.xy);
    let layer_color1 = textureSample(layer_color_map1, layer_color_sampler1, input.layer_uvs.xy);
    let weight1 = native_world_layer_reveal_weight(input.blend_color.g, layer_color1.a,
        NativeWorldRevealMaterial(material.alpha_reveal_parms1));
    let layer_normal2 = textureSample(layer_normal_map2, layer_normal_sampler2, input.layer_uvs.zw);
    let layer_color2 = textureSample(layer_color_map2, layer_color_sampler2, input.layer_uvs.zw);
    let weight2 = native_world_layer_reveal_weight(input.blend_color.b, layer_color2.a,
        NativeWorldRevealMaterial(material.alpha_reveal_parms2));
    let weights = vec2<f32>(weight1, weight2);
    let normal = native_world_three_layer_normal(input, base_normal.rg,
        layer_normal1.rg, layer_normal2.rg, weights);
    let layer_specular1 = textureSample(layer_specular_map1, layer_specular_sampler1, input.layer_uvs.xy);
    let base_color = textureSample(color_map, color_sampler, input.uv_lightmap.xy);
    let base_specular = vec4<f32>(vec3<f32>(0.20000000298023224), base_color.a);
    let specular1 = fma(vec4<f32>(weight1), layer_specular1 - base_specular, base_specular);
    let reflected = native_world_plain_specular_reflection(input.eye_to_vertex,
        normal, specular1, reflection_probe, reflection_sampler);
    let color1 = fma(vec3<f32>(weight1), layer_color1.rgb - base_color.rgb, base_color.rgb);
    let color_gamma = fma(vec3<f32>(weight2), layer_color2.rgb - color1, color1);
    return native_world_layer_lighting(native_world_three_layer_lighting_input(input),
        scene, normal, color_gamma, reflected, lightmap_secondary, lightmap_sampler);
}

// PScfff3f36 / PSad25a73a: overlays leave the base normal map unchanged.
fn native_world_color_overlays_fragment(
    input: NativeWorldThreeLayerVaryings, scene: NativeWorldScene, add_multiply: bool,
    color_map: texture_2d<f32>, color_sampler: sampler,
    normal_map: texture_2d<f32>, normal_sampler: sampler,
    color_map1: texture_2d<f32>, color_sampler1: sampler,
    color_map2: texture_2d<f32>, color_sampler2: sampler,
    lightmap_secondary: texture_2d<f32>, lightmap_sampler: sampler,
    reflection_probe: texture_cube<f32>, reflection_sampler: sampler,
) -> vec4<f32> {
    let normal_sample = textureSample(normal_map, normal_sampler, input.uv_lightmap.xy);
    let slope = fma(normal_sample.rg, vec2<f32>(4.015748023986816), vec2<f32>(-2.0157480239868164));
    let mapped = fma(vec3<f32>(slope.y), input.bitangent,
        fma(vec3<f32>(slope.x), input.tangent, input.normal_fog.xyz));
    let normal = inverseSqrt(dot(mapped, mapped)) * mapped;
    let base = textureSample(color_map, color_sampler, input.uv_lightmap.xy);
    let layer1 = textureSample(color_map1, color_sampler1, input.layer_uvs.xy);
    let layer2 = textureSample(color_map2, color_sampler2, input.layer_uvs.zw);
    var gloss = base.a;
    var color: vec3<f32>;
    var alpha = 1.0;
    if add_multiply {
        let multiplier = fma(vec3<f32>(input.blend_color.b), layer2.rgb - vec3<f32>(1.0), vec3<f32>(1.0));
        gloss = dot(multiplier, vec3<f32>(0.30000001192092896, 0.6000000238418579, 0.10000000149011612)) * gloss;
        let added = layer1.rgb * input.blend_color.g;
        color = multiplier * fma(added, vec3<f32>(layer1.a), base.rgb);
        alpha = clamp(fma(layer1.a, input.blend_color.g, 1.0), 0.0, 1.0);
    } else {
        let weight1 = layer1.a * input.blend_color.g;
        let weight2 = layer2.a * input.blend_color.b;
        let color1 = fma(vec3<f32>(weight1), layer1.rgb - base.rgb, base.rgb);
        color = fma(vec3<f32>(weight2), layer2.rgb - color1, color1);
    }
    let source = native_world_three_layer_lighting_input(input);
    let probe = native_world_layer_probe(source, normal, gloss, reflection_probe, reflection_sampler);
    let strength = fma(gloss, min(probe.fresnel, gloss * 0.4749999940395355),
        fma(gloss, 0.027499999850988388, 0.014999999664723873));
    var result = native_world_layer_lighting(source, scene, normal, color,
        strength * probe.linear_color, lightmap_secondary, lightmap_sampler);
    result.a = alpha;
    return result;
}

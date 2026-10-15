import package::rendering::native_world::{
    NativeWorldScene, NativeWorldObject, NativeWorldVertexInput, native_world_vertex,
};
import package::rendering::native_world_layers::{
    NativeWorldLayerVaryings, NativeWorldRevealMaterial,
    native_world_layer_reveal_weight, native_world_layer_lighting,
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

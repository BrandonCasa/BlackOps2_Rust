import package::rendering::native_world::{NativeWorldScene, NativeWorldObject, NativeWorldVertexInput};
import package::rendering::native_world_layers::{
    NativeWorldLayerVaryings, NativeWorldRevealMaterial, native_world_layer_normal,
    native_world_layer_reveal_weight, native_world_layer_lighting,
};
import package::rendering::native_world_plain::{
    NativeWorldGeometricVertexInput, native_world_diffuse_vertex,
    native_world_geometric_specular_vertex, native_world_plain_specular_reflection,
};
import package::rendering::native_world_three_layers::{
    NativeWorldThreeLayerVaryings, NativeWorldThreeLayerRevealMaterial,
    NativeWorldThreeLayerDoubleRevealMaterial, native_world_three_layer_normal,
    native_world_three_layer_lighting_input,
};

// Original world LIT programs; rendering-20261012/{source,shaders}.
// The four S0 normal-layer families cover229draws/8materials in Origins:
// lit_sm_r0c0n0s0_b1c1n1, PS6278286e SHA256
// 5c505dfe1bc6585f8513745ef9af92dac737a1f3d4d2e37ed931285e574cc263;
// lit_sm_r0c0n0s0_b1c1n1s1v1, PSd03b9b11 SHA256
// 6a98ee910fdc348989086966da55dc13d56794344838816f24de9d50bdd0f1d8;
// lit_sm_r0c0n0s0_b1c1n1_b2c2n2s2v2, PScf9388f8 SHA256
// 08e8256fdee46d4276947a442d51424284907165e6fb2d36e5617d88c4606384;
// lit_sm_r0c0n0s0_b1c1n1s1v1_b2c2n2s2v2, PS7dd03b87 SHA256
// 5969c4df49d834492d4da79f7ab84481eb13a74aeec58b5015167941296c339f.
// Their VS binaries are exactly the existing layer_c9b52d6f and94510929:
// 426a3d8625a1ef1a53ea0ae7bcf2e20e25ab872d32c7d559ab13b51480343a6f,
// f63f59e2d5174509caf55262e50fa98e3e227aa3cf07843d4d167a9e49a99a7b.
// Reuse native_world_layer_vertex/T1N1aux8 and
// native_world_three_layer_vertex/T2N2aux16 respectively, with no VS alias.
// These PS read actual S0.rgba; C0.a is unused. The normal blend, decoded
// lightmap/probe, squared specular response, SH correction, fog and HDR tails
// match the imported helpers instruction for instruction. No occlusionAmount,
// colorTint, detailScale or other stored material constants are shader inputs.

// PS6278286e: C0t0/s0,N0t1/s1,S0t2/s4,C1t3/s2,N1t4/s3,
// lightmap13/s13,cube15/s15. No material constants. S0 remains unchanged across
// the first normal/color layer: C1.a*primaryG blends only normals and color.
fn native_world_base_specular_layer_fragment(
    input: NativeWorldLayerVaryings, scene: NativeWorldScene,
    color_map: texture_2d<f32>, color_sampler: sampler,
    normal_map: texture_2d<f32>, normal_sampler: sampler,
    specular_map: texture_2d<f32>, specular_sampler: sampler,
    layer_color_map: texture_2d<f32>, layer_color_sampler: sampler,
    layer_normal_map: texture_2d<f32>, layer_normal_sampler: sampler,
    lightmap_secondary: texture_2d<f32>, lightmap_sampler: sampler,
    reflection_probe: texture_cube<f32>, reflection_sampler: sampler,
) -> vec4<f32> {
    let layer_normal = textureSample(layer_normal_map, layer_normal_sampler, input.blend_uv.zw);
    let base_normal = textureSample(normal_map, normal_sampler, input.uv_lightmap.xy);
    let layer_color = textureSample(layer_color_map, layer_color_sampler, input.blend_uv.zw);
    let weight = layer_color.a * input.blend_uv.y;
    let normal = native_world_layer_normal(input, base_normal.rg, layer_normal.rg, weight);
    let specular = textureSample(specular_map, specular_sampler, input.uv_lightmap.xy);
    let reflected = native_world_plain_specular_reflection(input.eye_to_vertex, normal,
        specular, reflection_probe, reflection_sampler);
    let base_color = textureSample(color_map, color_sampler, input.uv_lightmap.xy);
    let color_gamma = fma(vec3<f32>(weight), layer_color.rgb - base_color.rgb, base_color.rgb);
    return native_world_layer_lighting(input, scene, normal, color_gamma, reflected,
        lightmap_secondary, lightmap_sampler);
}

// PSd03b9b11: same original slots plus S1t5/s5 and cb1[59]alphaRevealParms1.
// Reveal weight blends all four S0/S1 channels before squaring specular RGB.
fn native_world_base_specular_reveal_layer_fragment(
    input: NativeWorldLayerVaryings, scene: NativeWorldScene, material: NativeWorldRevealMaterial,
    color_map: texture_2d<f32>, color_sampler: sampler,
    normal_map: texture_2d<f32>, normal_sampler: sampler,
    specular_map: texture_2d<f32>, specular_sampler: sampler,
    layer_color_map: texture_2d<f32>, layer_color_sampler: sampler,
    layer_normal_map: texture_2d<f32>, layer_normal_sampler: sampler,
    layer_specular_map: texture_2d<f32>, layer_specular_sampler: sampler,
    lightmap_secondary: texture_2d<f32>, lightmap_sampler: sampler,
    reflection_probe: texture_cube<f32>, reflection_sampler: sampler,
) -> vec4<f32> {
    let layer_normal = textureSample(layer_normal_map, layer_normal_sampler, input.blend_uv.zw);
    let base_normal = textureSample(normal_map, normal_sampler, input.uv_lightmap.xy);
    let layer_color = textureSample(layer_color_map, layer_color_sampler, input.blend_uv.zw);
    let weight = native_world_layer_reveal_weight(input.blend_uv.y, layer_color.a, material);
    let normal = native_world_layer_normal(input, base_normal.rg, layer_normal.rg, weight);
    let layer_specular = textureSample(layer_specular_map, layer_specular_sampler, input.blend_uv.zw);
    let base_specular = textureSample(specular_map, specular_sampler, input.uv_lightmap.xy);
    let specular = fma(vec4<f32>(weight), layer_specular - base_specular, base_specular);
    let reflected = native_world_plain_specular_reflection(input.eye_to_vertex, normal,
        specular, reflection_probe, reflection_sampler);
    let base_color = textureSample(color_map, color_sampler, input.uv_lightmap.xy);
    let color_gamma = fma(vec3<f32>(weight), layer_color.rgb - base_color.rgb, base_color.rgb);
    return native_world_layer_lighting(input, scene, normal, color_gamma, reflected,
        lightmap_secondary, lightmap_sampler);
}

// PScf9388f8: C0t0/s0,N0t1/s1,S0t2/s6,C1t3/s2,N1t4/s4,
// C2t5/s3,N2t6/s5,S2t7/s7,LM13/s13,cube15/s15; cb1[60]alphaRevealParms2.
// First layer keeps S0 untouched; the second reveal blends actual S0 into S2.
fn native_world_base_specular_three_layer_reveal_fragment(
    input: NativeWorldThreeLayerVaryings, scene: NativeWorldScene,
    material: NativeWorldThreeLayerRevealMaterial,
    color_map: texture_2d<f32>, color_sampler: sampler,
    normal_map: texture_2d<f32>, normal_sampler: sampler,
    specular_map: texture_2d<f32>, specular_sampler: sampler,
    layer_color_map1: texture_2d<f32>, layer_color_sampler1: sampler,
    layer_normal_map1: texture_2d<f32>, layer_normal_sampler1: sampler,
    layer_color_map2: texture_2d<f32>, layer_color_sampler2: sampler,
    layer_normal_map2: texture_2d<f32>, layer_normal_sampler2: sampler,
    layer_specular_map2: texture_2d<f32>, layer_specular_sampler2: sampler,
    lightmap_secondary: texture_2d<f32>, lightmap_sampler: sampler,
    reflection_probe: texture_cube<f32>, reflection_sampler: sampler,
) -> vec4<f32> {
    let normal1 = textureSample(layer_normal_map1, layer_normal_sampler1, input.layer_uvs.xy);
    let normal0 = textureSample(normal_map, normal_sampler, input.uv_lightmap.xy);
    let color1 = textureSample(layer_color_map1, layer_color_sampler1, input.layer_uvs.xy);
    let weight1 = color1.a * input.blend_color.g;
    let normal2 = textureSample(layer_normal_map2, layer_normal_sampler2, input.layer_uvs.zw);
    let color2 = textureSample(layer_color_map2, layer_color_sampler2, input.layer_uvs.zw);
    let weight2 = native_world_layer_reveal_weight(input.blend_color.b, color2.a,
        NativeWorldRevealMaterial(material.alpha_reveal_parms2));
    let normal = native_world_three_layer_normal(input, normal0.rg, normal1.rg,
        normal2.rg, vec2<f32>(weight1, weight2));
    let specular2 = textureSample(layer_specular_map2, layer_specular_sampler2, input.layer_uvs.zw);
    let specular0 = textureSample(specular_map, specular_sampler, input.uv_lightmap.xy);
    let specular = fma(vec4<f32>(weight2), specular2 - specular0, specular0);
    let reflected = native_world_plain_specular_reflection(input.eye_to_vertex, normal,
        specular, reflection_probe, reflection_sampler);
    let color0 = textureSample(color_map, color_sampler, input.uv_lightmap.xy);
    let blended1 = fma(vec3<f32>(weight1), color1.rgb - color0.rgb, color0.rgb);
    let color_gamma = fma(vec3<f32>(weight2), color2.rgb - blended1, blended1);
    return native_world_layer_lighting(native_world_three_layer_lighting_input(input),
        scene, normal, color_gamma, reflected, lightmap_secondary, lightmap_sampler);
}

// PS7dd03b87: C0t0/s0,N0t1/s1,S0t2/s6,C1t3/s2,N1t4/s4,S1t5/s7,
// C2t6/s3,N2t7/s5,S2t8/s8,LM13/s13,cube15/s15; cb1[59/60]reveal1/2.
// Normal slopes, gamma-space colors and S.rgba blend sequentially0->1->2.
fn native_world_base_specular_three_layer_double_reveal_fragment(
    input: NativeWorldThreeLayerVaryings, scene: NativeWorldScene,
    material: NativeWorldThreeLayerDoubleRevealMaterial,
    color_map: texture_2d<f32>, color_sampler: sampler,
    normal_map: texture_2d<f32>, normal_sampler: sampler,
    specular_map: texture_2d<f32>, specular_sampler: sampler,
    layer_color_map1: texture_2d<f32>, layer_color_sampler1: sampler,
    layer_normal_map1: texture_2d<f32>, layer_normal_sampler1: sampler,
    layer_specular_map1: texture_2d<f32>, layer_specular_sampler1: sampler,
    layer_color_map2: texture_2d<f32>, layer_color_sampler2: sampler,
    layer_normal_map2: texture_2d<f32>, layer_normal_sampler2: sampler,
    layer_specular_map2: texture_2d<f32>, layer_specular_sampler2: sampler,
    lightmap_secondary: texture_2d<f32>, lightmap_sampler: sampler,
    reflection_probe: texture_cube<f32>, reflection_sampler: sampler,
) -> vec4<f32> {
    let normal1 = textureSample(layer_normal_map1, layer_normal_sampler1, input.layer_uvs.xy);
    let normal0 = textureSample(normal_map, normal_sampler, input.uv_lightmap.xy);
    let color1 = textureSample(layer_color_map1, layer_color_sampler1, input.layer_uvs.xy);
    let weight1 = native_world_layer_reveal_weight(input.blend_color.g, color1.a,
        NativeWorldRevealMaterial(material.alpha_reveal_parms1));
    let normal2 = textureSample(layer_normal_map2, layer_normal_sampler2, input.layer_uvs.zw);
    let color2 = textureSample(layer_color_map2, layer_color_sampler2, input.layer_uvs.zw);
    let weight2 = native_world_layer_reveal_weight(input.blend_color.b, color2.a,
        NativeWorldRevealMaterial(material.alpha_reveal_parms2));
    let normal = native_world_three_layer_normal(input, normal0.rg, normal1.rg,
        normal2.rg, vec2<f32>(weight1, weight2));
    let specular1 = textureSample(layer_specular_map1, layer_specular_sampler1, input.layer_uvs.xy);
    let specular0 = textureSample(specular_map, specular_sampler, input.uv_lightmap.xy);
    let blended_specular1 = fma(vec4<f32>(weight1), specular1 - specular0, specular0);
    let specular2 = textureSample(layer_specular_map2, layer_specular_sampler2, input.layer_uvs.zw);
    let specular = fma(vec4<f32>(weight2), specular2 - blended_specular1, blended_specular1);
    let reflected = native_world_plain_specular_reflection(input.eye_to_vertex, normal,
        specular, reflection_probe, reflection_sampler);
    let color0 = textureSample(color_map, color_sampler, input.uv_lightmap.xy);
    let blended1 = fma(vec3<f32>(weight1), color1.rgb - color0.rgb, color0.rgb);
    let color_gamma = fma(vec3<f32>(weight2), color2.rgb - blended1, blended1);
    return native_world_layer_lighting(native_world_three_layer_lighting_input(input),
        scene, normal, color_gamma, reflected, lightmap_secondary, lightmap_sampler);
}

// lit_sm_r0c0n0_b1c1n1s1v1: PS4a20c8d9 SHA256
// 78b3c80a0162932e601ae30612193194d5c3ad3084e831794dc1e9bf2eae23b8.
// Uses existing c9b52d6f VS/T1N1aux8. C0t0/s0,N0t1/s1,C1t2/s2,N1t3/s3,
// S1t4/s4,LM13/s13,cube15/s15; requires cb1[59]alphaRevealParms1 only.
// This source's base specular literal is(.2,.2,.2,0): C0.a is never read.
// This differs from the x0 family, whose base gloss comes from C0.a.
fn native_world_no_base_gloss_reveal_layer_fragment(
    input: NativeWorldLayerVaryings, scene: NativeWorldScene, material: NativeWorldRevealMaterial,
    color_map: texture_2d<f32>, color_sampler: sampler,
    normal_map: texture_2d<f32>, normal_sampler: sampler,
    layer_color_map: texture_2d<f32>, layer_color_sampler: sampler,
    layer_normal_map: texture_2d<f32>, layer_normal_sampler: sampler,
    layer_specular_map: texture_2d<f32>, layer_specular_sampler: sampler,
    lightmap_secondary: texture_2d<f32>, lightmap_sampler: sampler,
    reflection_probe: texture_cube<f32>, reflection_sampler: sampler,
) -> vec4<f32> {
    let layer_normal = textureSample(layer_normal_map, layer_normal_sampler, input.blend_uv.zw);
    let base_normal = textureSample(normal_map, normal_sampler, input.uv_lightmap.xy);
    let layer_color = textureSample(layer_color_map, layer_color_sampler, input.blend_uv.zw);
    let weight = native_world_layer_reveal_weight(input.blend_uv.y, layer_color.a, material);
    let normal = native_world_layer_normal(input, base_normal.rg, layer_normal.rg, weight);
    let layer_specular = textureSample(layer_specular_map, layer_specular_sampler, input.blend_uv.zw);
    let base_specular = vec4<f32>(vec3<f32>(0.20000000298023224), 0.0);
    let specular = fma(vec4<f32>(weight), layer_specular - base_specular, base_specular);
    let reflected = native_world_plain_specular_reflection(input.eye_to_vertex, normal,
        specular, reflection_probe, reflection_sampler);
    let base_color = textureSample(color_map, color_sampler, input.uv_lightmap.xy);
    let color_gamma = fma(vec3<f32>(weight), layer_color.rgb - base_color.rgb, base_color.rgb);
    return native_world_layer_lighting(input, scene, normal, color_gamma, reflected,
        lightmap_secondary, lightmap_sampler);
}

// lit_sm_r0c0n0_b1c1n1 and_b1c1n1v1 bind no probe/specular images.
// Their VS a028e5fd SHA256
// f7cfc4c689148d988dc9e2f376e380aba90d261fd642dc88d4af065b735079a1
// shares exactly position/fog/packed-normal/tangent/handedness instructions
// with native_world_diffuse_vertex, then emits primaryRG,UV1,Q unchanged.
// Keep that shader's smaller required interface, without SH/reflection inputs.
struct NativeWorldDiffuseLayerVaryings {
    clip_position: vec4<f32>,
    uv_lightmap: vec4<f32>,
    normal_fog: vec4<f32>,
    fog_color: vec3<f32>,
    bitangent: vec3<f32>,
    tangent: vec3<f32>,
    blend_uv: vec4<f32>,
    normal_transform: vec4<f32>,
}

fn native_world_diffuse_layer_vertex(
    primary: NativeWorldVertexInput, layer_uv: vec2<f32>, normal_transform: vec4<f32>,
    scene: NativeWorldScene, object: NativeWorldObject,
) -> NativeWorldDiffuseLayerVaryings {
    let world = native_world_diffuse_vertex(primary, scene, object);
    return NativeWorldDiffuseLayerVaryings(world.clip_position, world.uv_lightmap,
        world.normal_fog, world.fog_color, world.bitangent, world.tangent,
        vec4<f32>(primary.color.rg, layer_uv), normal_transform);
}

fn native_world_diffuse_layer_normal(
    input: NativeWorldDiffuseLayerVaryings, base_normal: vec2<f32>,
    layer_normal: vec2<f32>, weight: f32,
) -> vec3<f32> {
    let layer_slope = fma(layer_normal, vec2<f32>(4.015748023986816), vec2<f32>(-2.0157480239868164));
    let q = fma(input.normal_transform, vec4<f32>(2.0), vec4<f32>(-1.0));
    let transformed_slope = vec2<f32>(dot(layer_slope, q.xy), dot(layer_slope, q.zw));
    let base_slope = fma(base_normal, vec2<f32>(4.015748023986816), vec2<f32>(-2.0157480239868164));
    let slope = fma(vec2<f32>(weight), transformed_slope - base_slope, base_slope);
    let mapped = fma(vec3<f32>(slope.y), input.bitangent,
        fma(vec3<f32>(slope.x), input.tangent, input.normal_fog.xyz));
    return inverseSqrt(dot(mapped, mapped)) * mapped;
}

// Both no-probe PS have this identical tail. It evaluates no Fresnel, cube,
// geometric-normal reflection light or SH. Preserve the first fused fog subtract.
fn native_world_diffuse_layer_lighting(
    input: NativeWorldDiffuseLayerVaryings, scene: NativeWorldScene,
    normal: vec3<f32>, color_gamma: vec3<f32>,
    lightmap_secondary: texture_2d<f32>, lightmap_sampler: sampler,
) -> vec4<f32> {
    let uv = input.uv_lightmap.zw;
    let other_uv = fma(uv.xyxy, vec4<f32>(1.0, 0.3333333432674408, 1.0, 0.3333333432674408),
        vec4<f32>(0.0, 0.3333333432674408, 0.0, 0.6666666865348816));
    let direction_sample = textureSample(lightmap_secondary, lightmap_sampler, other_uv.zw);
    let directional_sample = textureSample(lightmap_secondary, lightmap_sampler, other_uv.xy);
    let direction = fma(direction_sample.rgb, vec3<f32>(2.0), vec3<f32>(-1.0));
    let cosine = clamp(dot(direction, normal), 0.0, 1.0);
    let directional = (1.0 / (directional_sample.a + 9.999999974752427e-07)) * directional_sample.rgb;
    let directed_light = cosine * directional;
    let ambient_sample = textureSample(lightmap_secondary, lightmap_sampler, uv * vec2<f32>(1.0, 0.3333333432674408));
    let light = fma(ambient_sample.rgb, vec3<f32>(1.0 / (ambient_sample.a + 9.999999974752427e-07)), directed_light);
    let diffuse = color_gamma * color_gamma;
    let fog_delta = fma(diffuse, light, -input.fog_color);
    let fogged = fma(vec3<f32>(input.normal_fog.w), fog_delta, input.fog_color);
    return vec4<f32>(sqrt(fogged * scene.hdr_control0.x), 1.0);
}

// PS91feb334 SHA256
// 9dda59d8c31c755a62a9dc8e050830f4a2c7a345cb3ec071e769154c6f52989b.
// C0t0/s0,N0t1/s1,C1t2/s2,N1t3/s3,LM13/s13. No material constants.
// PACKED_WORLD_T1N1 auxiliary UV1 Float16x2@0,Q1 Unorm8x4@4,stride8.
fn native_world_diffuse_layer_fragment(
    input: NativeWorldDiffuseLayerVaryings, scene: NativeWorldScene,
    color_map: texture_2d<f32>, color_sampler: sampler,
    normal_map: texture_2d<f32>, normal_sampler: sampler,
    layer_color_map: texture_2d<f32>, layer_color_sampler: sampler,
    layer_normal_map: texture_2d<f32>, layer_normal_sampler: sampler,
    lightmap_secondary: texture_2d<f32>, lightmap_sampler: sampler,
) -> vec4<f32> {
    let layer_normal = textureSample(layer_normal_map, layer_normal_sampler, input.blend_uv.zw);
    let base_normal = textureSample(normal_map, normal_sampler, input.uv_lightmap.xy);
    let layer_color = textureSample(layer_color_map, layer_color_sampler, input.blend_uv.zw);
    let weight = layer_color.a * input.blend_uv.y;
    let normal = native_world_diffuse_layer_normal(input, base_normal.rg, layer_normal.rg, weight);
    let base_color = textureSample(color_map, color_sampler, input.uv_lightmap.xy);
    let color_gamma = fma(vec3<f32>(weight), layer_color.rgb - base_color.rgb, base_color.rgb);
    return native_world_diffuse_layer_lighting(input, scene, normal, color_gamma,
        lightmap_secondary, lightmap_sampler);
}

// PS44b1014b SHA256
// 32d53496690d2bc27602c1a842459349ed5d0f9dcb8213c2f252ff8415a6274f.
// Same original pairs/layout, requires cb1[59]alphaRevealParms1. No S/probe.
fn native_world_diffuse_reveal_layer_fragment(
    input: NativeWorldDiffuseLayerVaryings, scene: NativeWorldScene, material: NativeWorldRevealMaterial,
    color_map: texture_2d<f32>, color_sampler: sampler,
    normal_map: texture_2d<f32>, normal_sampler: sampler,
    layer_color_map: texture_2d<f32>, layer_color_sampler: sampler,
    layer_normal_map: texture_2d<f32>, layer_normal_sampler: sampler,
    lightmap_secondary: texture_2d<f32>, lightmap_sampler: sampler,
) -> vec4<f32> {
    let layer_color = textureSample(layer_color_map, layer_color_sampler, input.blend_uv.zw);
    let weight = native_world_layer_reveal_weight(input.blend_uv.y, layer_color.a, material);
    let layer_normal = textureSample(layer_normal_map, layer_normal_sampler, input.blend_uv.zw);
    let base_normal = textureSample(normal_map, normal_sampler, input.uv_lightmap.xy);
    let normal = native_world_diffuse_layer_normal(input, base_normal.rg, layer_normal.rg, weight);
    let base_color = textureSample(color_map, color_sampler, input.uv_lightmap.xy);
    let color_gamma = fma(vec3<f32>(weight), layer_color.rgb - base_color.rgb, base_color.rgb);
    return native_world_diffuse_layer_lighting(input, scene, normal, color_gamma,
        lightmap_secondary, lightmap_sampler);
}

// lit_sm_r0c0s0_b1c1: VSf55271f1 SHA256
// 1b0299b39908c8a0ca78e812a5365fce71e2edb3676a8181c726ce9f36e28b83.
// Position/fog/packed geometric normal/SH exactly match the imported geometric
// VS instructions; only primaryRG and auxiliaryUV1 outputs are different.
// PACKED_WORLD_T1N0 auxiliary Float16x2@0,stride4. No tangent or Q input.
struct NativeWorldGeometricLayerVaryings {
    clip_position: vec4<f32>,
    uv_lightmap: vec4<f32>,
    normal_fog: vec4<f32>,
    fog_color: vec3<f32>,
    eye_to_vertex: vec3<f32>,
    blend_uv: vec4<f32>,
    reflection_correction: vec3<f32>,
}

fn native_world_geometric_layer_vertex(
    primary: NativeWorldGeometricVertexInput, layer_uv: vec2<f32>,
    scene: NativeWorldScene, object: NativeWorldObject,
) -> NativeWorldGeometricLayerVaryings {
    let world = native_world_geometric_specular_vertex(primary, scene, object);
    return NativeWorldGeometricLayerVaryings(world.clip_position, world.uv_lightmap,
        world.normal_fog, world.fog_color, world.eye_to_vertex,
        vec4<f32>(primary.color.rg, layer_uv), world.reflection_correction);
}

// PS5e9f74f0 SHA256
// a8da238c4b88203b338c25e07fab3bef653831461616e9de7cf1602ed0a4f732.
// C0t0/s0,S0t1/s2,C1t2/s1,LM13/s13,cube15/s15. No material constants.
// Normalize interpolated N for shading; reflection-light dot uses the original
// unnormalized geometric interpolant. S0 stays unchanged; C0.a is unused.
fn native_world_geometric_layer_fragment(
    input: NativeWorldGeometricLayerVaryings, scene: NativeWorldScene,
    color_map: texture_2d<f32>, color_sampler: sampler,
    specular_map: texture_2d<f32>, specular_sampler: sampler,
    layer_color_map: texture_2d<f32>, layer_color_sampler: sampler,
    lightmap_secondary: texture_2d<f32>, lightmap_sampler: sampler,
    reflection_probe: texture_cube<f32>, reflection_sampler: sampler,
) -> vec4<f32> {
    let normal = inverseSqrt(dot(input.normal_fog.xyz, input.normal_fog.xyz)) * input.normal_fog.xyz;
    let specular = textureSample(specular_map, specular_sampler, input.uv_lightmap.xy);
    let reflected = native_world_plain_specular_reflection(input.eye_to_vertex, normal,
        specular, reflection_probe, reflection_sampler);
    let layer_color = textureSample(layer_color_map, layer_color_sampler, input.blend_uv.zw);
    let weight = layer_color.a * input.blend_uv.y;
    let base_color = textureSample(color_map, color_sampler, input.uv_lightmap.xy);
    let color_gamma = fma(vec3<f32>(weight), layer_color.rgb - base_color.rgb, base_color.rgb);
    // The shared lighting tail reads none of the tangent/Q fields. Zero words
    // below are unused struct padding, and introduce no vertex/texture inputs.
    let lighting_input = NativeWorldLayerVaryings(input.clip_position, input.uv_lightmap,
        input.normal_fog, input.fog_color, vec3<f32>(0.0), vec3<f32>(0.0),
        input.eye_to_vertex, input.blend_uv, vec4<f32>(0.0), input.reflection_correction);
    return native_world_layer_lighting(lighting_input, scene, normal, color_gamma,
        reflected, lightmap_secondary, lightmap_sampler);
}

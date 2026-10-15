import package::rendering::native_model::{
    NativeModelScene, NativeModelObject, NativeModelVaryings,
};
import package::rendering::native_world::{NativeWorldObject};
import package::rendering::native_world_plain::{
    NativeWorldGeometricVertexInput, native_world_plain_base_vertex,
    native_world_plain_specular_reflection,
};

// Original model LIT programs: rendering-20261012/{source,shaders}.
// Five families use native_model_vertex's exact VS347155f7 binary:
// c435f796400eaef72e6849437620e16bc09187e242d6b58fcd9482f3eea53319.
// mc_lit_sm_r0c0n0_2z223015: technique3cd72a65, PSa06c1664,
// 123ab1ba022475d418aaaae277ab16ea640241469d93330b5858dd3b271f05c4.
// mc_lit_sm_r0c0n0s0_zqq1fze7: technique651eb933, PS4bceb0cc,
// 5c88d806b6235333a637f54e03dca4fa4bec3a6ef27eada702a272dac50cef0a.
// mc_lit_sm_t0c0n0s0_47jq982j: techniquebace39da, PSb646e86e,
// 023f8b381a0517b70e25520ce3089394d627de6b9d41fea3e423d5fba5e6e26a.
// mc_lit_sm_t0c0n0_9qf6e4qj: techniquecf99b4f9, PS5de44ec6,
// a9f80bb99f8872b6d836bc24ad7043ffbd608bc782f9230617ff36eab8e3c743.
// mc_lit_sm_r0c0n0x0_53144qq1: technique1099c721, PS4fd85412,
// 31a3fa53d35bb200780d03d73edf5e5d33c4b565b5feccccce33795d43ddecab.
// Each family keeps its original material inputs and lighting algebra.

// Original PS cb1[42], byte 672. Preserve the authored float4; only Y and Z
// are read by the two explicit-specular programs below.
struct NativeModelOcclusion {
    occlusion_amount: vec4<f32>,
}

// mc_lit_sm_r0c0_77e21qq8: technique72800b63, VSeb01efa5,
// 0966aa5a569b0ba7056f89553b8673490f178f42270f33879036424af8f152f5;
// PS9c5921d1,
// ee7817c5135caf7b1980b32b64590392a039d283eee39afafcecea3c43cb906b.
// PACKED32 still owns this stream, but this VS consumes neither tangent nor
// position.w. Its actual outputs omit tangent and bitangent completely.
struct NativeModelGeometricVertexInput {
    position: vec3<f32>,
    color: vec4<f32>,
    uv: vec2<f32>,
    packed_normal: vec3<f32>,
}

struct NativeModelGeometricVaryings {
    clip_position: vec4<f32>,
    color: vec4<f32>,
    uv: vec2<f32>,
    normal_fog: vec4<f32>,
    fog_color: vec3<f32>,
    eye_to_vertex: vec4<f32>,
    grid_coords_and_vis: vec4<f32>,
    reflection_correction: vec3<f32>,
    grid_correction: vec3<f32>,
}

fn native_model_geometric_vertex(
    input: NativeModelGeometricVertexInput,
    scene: NativeModelScene,
    object: NativeModelObject,
) -> NativeModelGeometricVaryings {
    return native_model_geometric_vertex_lighting(input, scene, object, vec4<f32>(0.0), false);
}

// Original MLV VSbcc4a7bc retains the same geometric transform and fog.
fn native_model_geometric_vertex_lighting(
    input: NativeModelGeometricVertexInput, scene: NativeModelScene,
    object: NativeModelObject, vertex_light: vec4<f32>, baked: bool,
) -> NativeModelGeometricVaryings {
    // Exact shared position/fog/geometric-normal instruction sequence. This
    // model program has no lightmap UV input; the helper's UV.zw is unused.
    let world = native_world_plain_base_vertex(
        NativeWorldGeometricVertexInput(input.position, input.color, input.uv,
            input.packed_normal, vec2<f32>(0.0)),
        scene.world,
        NativeWorldObject(object.world_rows, object.reflection_lighting_sh0,
            object.reflection_lighting_sh1, object.reflection_lighting_sh2),
    );
    let normal = world.normal_fog.xyz;
    let squared_yz = normal.yz * normal.yz;
    let quadratic = vec4<f32>(normal.z * normal.x, normal.y * normal.z,
        normal.x * normal.y, fma(normal.x, normal.x, -squared_yz.x));
    let normal_one = vec4<f32>(normal, 1.0);
    let grid_quadratic = dot(object.grid_lighting_sh2, quadratic);
    let reflection_quadratic = dot(object.reflection_lighting_sh2, quadratic);
    let grid_linear = dot(object.grid_lighting_sh1, normal_one);
    let reflection_linear = dot(object.reflection_lighting_sh1, normal_one);
    let reflection_scale = fma(object.reflection_lighting_sh0.w, squared_yz.y,
        reflection_quadratic + reflection_linear);
    let reflection_irradiance = max(reflection_scale * object.reflection_lighting_sh0.xyz,
        vec3<f32>(0.10000000149011612));
    let grid_scale = fma(object.grid_lighting_sh0.w, squared_yz.y,
        grid_quadratic + grid_linear);
    let grid_irradiance = max(grid_scale * object.grid_lighting_sh0.xyz,
        vec3<f32>(0.10000000149011612));
    let irradiance = select(grid_irradiance, (vertex_light.rgb * vertex_light.rgb) * 32.0, baked);
    var grid_coords = object.grid_lighting_coords_and_vis;
    grid_coords.w = select(grid_coords.w, vertex_light.a, baked);
    return NativeModelGeometricVaryings(world.clip_position, world.color, input.uv,
        world.normal_fog, world.fog_color, vec4<f32>(world.eye_to_vertex, 0.0),
        grid_coords, irradiance / reflection_irradiance,
        select(vec3<f32>(1.0), irradiance / grid_irradiance, baked));
}

fn native_model_mapped_normal(
    input: NativeModelVaryings,
    normal_sample: vec2<f32>,
) -> vec3<f32> {
    let slopes = fma(normal_sample, vec2<f32>(4.015748023986816),
        vec2<f32>(-2.0157480239868164));
    let mapped = fma(vec3<f32>(slopes.y), input.bitangent,
        fma(vec3<f32>(slopes.x), input.tangent, input.normal_fog.xyz));
    return inverseSqrt(dot(mapped, mapped)) * mapped;
}

// Native modelLighting t13/s13 is the actual original 3D atlas. The source
// lookup normal lies on the 4x4x4 slot's boundary, using the max-absolute axis.
fn native_model_atlas_irradiance(
    normal: vec3<f32>,
    grid_coords: vec3<f32>,
    scene: NativeModelScene,
    model_lighting: texture_3d<f32>, model_lighting_sampler: sampler,
) -> vec3<f32> {
    let major_axis = max(max(abs(normal.z), abs(normal.y)), abs(normal.x));
    let lighting_direction = (normal * scene.lighting_lookup_scale.xyz) / major_axis;
    let sample = textureSample(model_lighting, model_lighting_sampler,
        lighting_direction + grid_coords);
    return sample.rgb * sample.rgb;
}

// Identical final diffuse instructions in PSa06c1664, PS9c5921d1 and PS5de44ec6.
// Their first fused subtract differs from the specular programs' add-then-fog.
fn native_model_diffuse_lighting(
    normal: vec3<f32>,
    grid_coords: vec3<f32>,
    normal_fog: vec4<f32>,
    fog_color: vec3<f32>,
    colored: vec3<f32>,
    irradiance_scale: vec3<f32>,
    scene: NativeModelScene,
    model_lighting: texture_3d<f32>, model_lighting_sampler: sampler,
) -> vec3<f32> {
    let irradiance = native_model_atlas_irradiance(normal, grid_coords, scene,
        model_lighting, model_lighting_sampler) * irradiance_scale;
    let diffuse = colored * colored;
    let fog_delta = fma(irradiance * diffuse, vec3<f32>(32.0), -fog_color);
    let fogged = fma(vec3<f32>(normal_fog.w), fog_delta, fog_color);
    return sqrt(fogged * scene.world.hdr_control0.x);
}

// 2z223015: C0/s0, N1/s1, atlas13/s13. No probe or material constant input.
fn native_model_diffuse_fragment(
    input: NativeModelVaryings,
    scene: NativeModelScene,
    color_map: texture_2d<f32>, color_sampler: sampler,
    normal_map: texture_2d<f32>, normal_sampler: sampler,
    model_lighting: texture_3d<f32>, model_lighting_sampler: sampler,
) -> vec4<f32> {
    let normal_sample = textureSample(normal_map, normal_sampler, input.uv);
    let normal = native_model_mapped_normal(input, normal_sample.rg);
    let color_sample = textureSample(color_map, color_sampler, input.uv);
    let result = native_model_diffuse_lighting(normal, input.grid_coords_and_vis.xyz,
        input.normal_fog, input.fog_color, color_sample.rgb * input.color.rgb,
        input.grid_correction, scene, model_lighting, model_lighting_sampler);
    return vec4<f32>(result, 1.0);
}

// 77e21qq8: C0/s0, atlas13/s13. Normalize the geometric interpolant in PS.
fn native_model_geometric_diffuse_fragment(
    input: NativeModelGeometricVaryings,
    scene: NativeModelScene,
    alpha_test: bool, vertex_occlusion: bool,
    color_map: texture_2d<f32>, color_sampler: sampler,
    model_lighting: texture_3d<f32>, model_lighting_sampler: sampler,
) -> vec4<f32> {
    let normal = inverseSqrt(dot(input.normal_fog.xyz, input.normal_fog.xyz))
        * input.normal_fog.xyz;
    let color_sample = textureSample(color_map, color_sampler, input.uv);
    let result = native_model_diffuse_lighting(normal, input.grid_coords_and_vis.xyz,
        input.normal_fog, input.fog_color, color_sample.rgb * input.color.rgb,
        input.grid_correction * select(1.0, input.color.a, vertex_occlusion), scene, model_lighting, model_lighting_sampler);
    // PS61d17973 uses the same fused ge128 comparison as the normal-mapped
    // cutout. PS2a42c1a1 instead scales squared atlas lighting by vertex alpha.
    if alpha_test && fma(color_sample.a, input.color.a, -0.501960813999176) < 0.0 {
        discard;
    }
    return vec4<f32>(result, select(1.0, color_sample.a * input.color.a, alpha_test));
}

// 9qf6e4qj: C0/s0, N1/s1, atlas13/s13. The fixed threshold is literal
// 0x3f008081 in original DXBC. Keep the fused alpha comparison and output alpha.
fn native_model_alpha_diffuse_fragment(
    input: NativeModelVaryings,
    scene: NativeModelScene,
    color_map: texture_2d<f32>, color_sampler: sampler,
    normal_map: texture_2d<f32>, normal_sampler: sampler,
    model_lighting: texture_3d<f32>, model_lighting_sampler: sampler,
) -> vec4<f32> {
    let color_sample = textureSample(color_map, color_sampler, input.uv);
    let colored = color_sample * input.color;
    let normal_sample = textureSample(normal_map, normal_sampler, input.uv);
    let normal = native_model_mapped_normal(input, normal_sample.rg);
    let result = native_model_diffuse_lighting(normal, input.grid_coords_and_vis.xyz,
        input.normal_fog, input.fog_color, colored.rgb, vec3<f32>(1.0), scene,
        model_lighting, model_lighting_sampler);
    // Finish implicit-derivative sampling before terminating a quad lane.
    // This retains the original surviving samples and fused alpha decision.
    if fma(color_sample.a, input.color.a, -0.501960813999176) < 0.0 {
        discard;
    }
    return vec4<f32>(result, colored.a);
}

// Identical lighting instructions in PS4bceb0cc and PSb646e86e. The reflected
// response is the exact sequence also used by the explicit-specular world PS;
// only that response helper is shared, never world lightmap evaluation.
fn native_model_specular_lighting(
    input: NativeModelVaryings,
    scene: NativeModelScene,
    occlusion: NativeModelOcclusion,
    normal: vec3<f32>,
    reflected: vec3<f32>,
    colored: vec3<f32>,
    model_lighting: texture_3d<f32>, model_lighting_sampler: sampler,
) -> vec3<f32> {
    let reflection = (reflected * occlusion.occlusion_amount.y)
        * input.reflection_correction;
    let irradiance = native_model_atlas_irradiance(normal,
        input.grid_coords_and_vis.xyz, scene, model_lighting, model_lighting_sampler)
        * occlusion.occlusion_amount.z * input.grid_correction;
    let diffuse = colored * colored;
    let lit = fma(irradiance * diffuse, vec3<f32>(32.0), reflection);
    let fogged = fma(vec3<f32>(input.normal_fog.w), lit - input.fog_color, input.fog_color);
    return sqrt(fogged * scene.world.hdr_control0.x);
}

// zqq1fze7: C0/s0, N1/s1, S2/s2, atlas13/s13, cube15/s15, cb1[42].
fn native_model_normal_specular_fragment(
    input: NativeModelVaryings,
    scene: NativeModelScene,
    occlusion: NativeModelOcclusion,
    color_map: texture_2d<f32>, color_sampler: sampler,
    normal_map: texture_2d<f32>, normal_sampler: sampler,
    specular_map: texture_2d<f32>, specular_sampler: sampler,
    model_lighting: texture_3d<f32>, model_lighting_sampler: sampler,
    reflection_probe: texture_cube<f32>, reflection_sampler: sampler,
) -> vec4<f32> {
    let normal_sample = textureSample(normal_map, normal_sampler, input.uv);
    let normal = native_model_mapped_normal(input, normal_sample.rg);
    let specular = textureSample(specular_map, specular_sampler, input.uv);
    let reflected = native_world_plain_specular_reflection(input.eye_to_vertex.xyz,
        normal, specular, reflection_probe, reflection_sampler);
    let color_sample = textureSample(color_map, color_sampler, input.uv);
    let result = native_model_specular_lighting(input, scene, occlusion, normal,
        reflected, color_sample.rgb * input.color.rgb, model_lighting, model_lighting_sampler);
    return vec4<f32>(result, 1.0);
}

// 47jq982j: the same real inputs and response as zqq1fze7, with the original
// fixed alpha decision and surviving authored alpha. Defer lane termination
// until all implicit derivatives have been evaluated for the complete quad.
fn native_model_alpha_specular_fragment(
    input: NativeModelVaryings,
    scene: NativeModelScene,
    occlusion: NativeModelOcclusion,
    color_map: texture_2d<f32>, color_sampler: sampler,
    normal_map: texture_2d<f32>, normal_sampler: sampler,
    specular_map: texture_2d<f32>, specular_sampler: sampler,
    model_lighting: texture_3d<f32>, model_lighting_sampler: sampler,
    reflection_probe: texture_cube<f32>, reflection_sampler: sampler,
) -> vec4<f32> {
    let color_sample = textureSample(color_map, color_sampler, input.uv);
    let colored = color_sample * input.color;
    let normal_sample = textureSample(normal_map, normal_sampler, input.uv);
    let normal = native_model_mapped_normal(input, normal_sample.rg);
    let specular = textureSample(specular_map, specular_sampler, input.uv);
    let reflected = native_world_plain_specular_reflection(input.eye_to_vertex.xyz,
        normal, specular, reflection_probe, reflection_sampler);
    let result = native_model_specular_lighting(input, scene, occlusion, normal,
        reflected, colored.rgb, model_lighting, model_lighting_sampler);
    if fma(color_sample.a, input.color.a, -0.501960813999176) < 0.0 {
        discard;
    }
    return vec4<f32>(result, colored.a);
}

// 53144qq1: source sandbags. C.a owns implicit gloss, while vertex alpha
// independently scales both reflection and atlas lighting. Neither is opacity.
fn native_model_vertex_occlusion_fragment(
    input: NativeModelVaryings,
    scene: NativeModelScene,
    color_map: texture_2d<f32>, color_sampler: sampler,
    normal_map: texture_2d<f32>, normal_sampler: sampler,
    model_lighting: texture_3d<f32>, model_lighting_sampler: sampler,
    reflection_probe: texture_cube<f32>, reflection_sampler: sampler,
) -> vec4<f32> {
    let normal_sample = textureSample(normal_map, normal_sampler, input.uv);
    let normal = native_model_mapped_normal(input, normal_sample.rg);
    let incident = inverseSqrt(dot(input.eye_to_vertex.xyz, input.eye_to_vertex.xyz))
        * input.eye_to_vertex.xyz;
    let direction = fma(normal, vec3<f32>(-2.0 * dot(incident, normal)), incident);
    let fresnel = exp2(clamp(dot(normal, -incident), 0.0, 1.0) * -9.279999732971191);
    let color_sample = textureSample(color_map, color_sampler, input.uv);
    let gloss = color_sample.a;
    let probe_sample = textureSampleLevel(reflection_probe, reflection_sampler,
        direction, fma(-4.0, gloss, 4.0));
    let probe = (1.0 / (probe_sample.a + 9.999999974752427e-07)) * probe_sample.rgb;
    let strength = fma(gloss, min(fresnel, gloss * 0.4749999940395355),
        fma(gloss, 0.027499999850988388, 0.014999999664723873));
    let reflected = ((strength * probe) * input.color.a) * input.reflection_correction;
    let irradiance = native_model_atlas_irradiance(normal,
        input.grid_coords_and_vis.xyz, scene, model_lighting, model_lighting_sampler)
        * input.color.a * input.grid_correction;
    let colored = color_sample.rgb * input.color.rgb;
    let diffuse = colored * colored;
    let lit = fma(diffuse * irradiance, vec3<f32>(32.0), reflected);
    let fogged = fma(vec3<f32>(input.normal_fog.w), lit - input.fog_color, input.fog_color);
    return vec4<f32>(sqrt(fogged * scene.world.hdr_control0.x), 1.0);
}

// PS5c2357e1 / PS0be61ff8 / PS60166054 / PS37d3284d: diffuse and fog
// are premultiplied by source alpha before the square-root target encoding.
fn native_model_blended_diffuse(
    normal: vec3<f32>, grid_coords: vec3<f32>, correction: vec3<f32>,
    normal_fog: vec4<f32>, fog_color: vec3<f32>, colored: vec4<f32>,
    scene: NativeModelScene, atlas: texture_3d<f32>, atlas_sampler: sampler,
) -> vec4<f32> {
    let irradiance = native_model_atlas_irradiance(normal, grid_coords, scene, atlas, atlas_sampler) * correction;
    let diffuse = colored.rgb * colored.rgb;
    let lit = (irradiance * diffuse) * colored.a;
    let fog = colored.a * fog_color;
    let delta = fma(lit, vec3<f32>(32.0), -fog);
    let fogged = fma(vec3<f32>(normal_fog.w), delta, fog);
    let alpha = sqrt(colored.a);
    if !(alpha > 0.0) { discard; }
    return vec4<f32>(sqrt(fogged * scene.world.hdr_control0.x), alpha);
}

// Geometric specular PS77fb8779, premultiplied PSb781f149 and MLVe044b950.
fn native_model_geometric_specular_fragment(
    input: NativeModelGeometricVaryings, scene: NativeModelScene,
    occlusion: NativeModelOcclusion, transparent: bool,
    color_map: texture_2d<f32>, color_sampler: sampler,
    specular_map: texture_2d<f32>, specular_sampler: sampler,
    atlas: texture_3d<f32>, atlas_sampler: sampler,
    probe: texture_cube<f32>, probe_sampler: sampler,
) -> vec4<f32> {
    let normal = inverseSqrt(dot(input.normal_fog.xyz, input.normal_fog.xyz)) * input.normal_fog.xyz;
    let specular = textureSample(specular_map, specular_sampler, input.uv);
    let reflected = native_world_plain_specular_reflection(input.eye_to_vertex.xyz,
        normal, specular, probe, probe_sampler);
    let reflection = (reflected * occlusion.occlusion_amount.y) * input.reflection_correction;
    let irradiance = (native_model_atlas_irradiance(normal, input.grid_coords_and_vis.xyz,
        scene, atlas, atlas_sampler) * occlusion.occlusion_amount.z) * input.grid_correction;
    let colored = textureSample(color_map, color_sampler, input.uv) * input.color;
    let diffuse = colored.rgb * colored.rgb;
    let lit = fma(irradiance * diffuse, vec3<f32>(32.0), reflection);
    var delta = lit - input.fog_color;
    var fog = input.fog_color;
    var alpha = 1.0;
    if transparent {
        fog = colored.a * fog;
        delta = fma(lit, vec3<f32>(colored.a), -fog);
        alpha = sqrt(colored.a);
    }
    let fogged = fma(vec3<f32>(input.normal_fog.w), delta, fog);
    if transparent && !(alpha > 0.0) { discard; }
    return vec4<f32>(sqrt(fogged * scene.world.hdr_control0.x), alpha);
}

// Original PSecd7a93c / PSbb12e125: detail XY tiles UV; Z weights its normal
// slope before adding to the base slope. W is unused, not a color multiplier.
fn native_model_detail_diffuse_fragment(
    input: NativeModelVaryings, scene: NativeModelScene, detail_scale: vec4<f32>,
    color_map: texture_2d<f32>, color_sampler: sampler,
    normal_map: texture_2d<f32>, normal_sampler: sampler,
    detail_map: texture_2d<f32>, detail_sampler: sampler,
    model_lighting: texture_3d<f32>, model_lighting_sampler: sampler,
) -> vec4<f32> {
    let detail = textureSample(detail_map, detail_sampler, input.uv * detail_scale.xy).rg;
    let base = textureSample(normal_map, normal_sampler, input.uv).rg;
    let scale = vec2<f32>(4.015748023986816);
    let bias = vec2<f32>(-2.0157480239868164);
    let slopes = fma(vec2<f32>(detail_scale.z), fma(detail, scale, bias), fma(base, scale, bias));
    let mapped = fma(vec3<f32>(slopes.y), input.bitangent,
        fma(vec3<f32>(slopes.x), input.tangent, input.normal_fog.xyz));
    let normal = mapped * inverseSqrt(dot(mapped, mapped));
    let color = textureSample(color_map, color_sampler, input.uv).rgb * input.color.rgb;
    return vec4<f32>(native_model_diffuse_lighting(normal, input.grid_coords_and_vis.xyz,
        input.normal_fog, input.fog_color, color, input.grid_correction, scene,
        model_lighting, model_lighting_sampler), 1.0);
}

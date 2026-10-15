import package::rendering::native_world::{
    NativeWorldScene, NativeWorldObject, NativeWorldVertexInput,
    native_world_vertex,
};

// T6 mc_lit_sm_r0c0n0x0_q361191u, LIT pimp_technique_lprobe_235c93bf.
// VS lprobe_347155f7: SHA256
// c435f796400eaef72e6849437620e16bc09187e242d6b58fcd9482f3eea53319.
// PS lprobe_f9c3b462: SHA256
// f52471b21243d08ca325262d100111d91ad6c9f25fb00d144296f3493ed7298d.
// Source: rendering-20261012/shaders/<sha256>.asm and original DXBC literals.

struct NativeModelScene {
    world: NativeWorldScene,
    lighting_lookup_scale: vec4<f32>, // Original PS cb0[56], byte 896; XYZ used.
}

// Original cb3[0..11], 176 bytes. Grid lighting and probe lighting are distinct
// native inputs. The host must supply the selected model's actual coefficients.
struct NativeModelObject {
    world_rows: array<vec4<f32>, 4>,
    grid_lighting_coords_and_vis: vec4<f32>, // cb3[4]
    grid_lighting_sh0: vec4<f32>,           // cb3[5]
    grid_lighting_sh1: vec4<f32>,           // cb3[6]
    grid_lighting_sh2: vec4<f32>,           // cb3[7]
    reflection_lighting_sh0: vec4<f32>,     // cb3[8]
    reflection_lighting_sh1: vec4<f32>,     // cb3[9]
    reflection_lighting_sh2: vec4<f32>,     // cb3[10]
}

// Original PACKED model stream, stride 32: Float32x4 at 0 (native inches and
// binormal sign), Unorm8x4 color at 16, Float16x2 UV at 20, packed UNORM10
// normal/tangent at 24/28. Pass XYZ of each Unorm10_10_10_2 input unchanged.
struct NativeModelVertexInput {
    position: vec4<f32>,
    color: vec4<f32>,
    uv: vec2<f32>,
    packed_normal: vec3<f32>,
    packed_tangent: vec3<f32>,
}

struct NativeModelVaryings {
    clip_position: vec4<f32>,
    color: vec4<f32>,
    uv: vec2<f32>,
    normal_fog: vec4<f32>,
    fog_color: vec3<f32>,
    bitangent: vec3<f32>,
    tangent: vec3<f32>,
    eye_to_vertex: vec4<f32>,
    grid_coords_and_vis: vec4<f32>,
    reflection_correction: vec3<f32>,
}

fn native_model_vertex(
    input: NativeModelVertexInput,
    scene: NativeModelScene,
    object: NativeModelObject,
) -> NativeModelVaryings {
    // The position/fog/basis instructions match the ordinary world program.
    // Its lightmap UV output is unused here; models have no lightmap UV input.
    // Its inverse reflection SH output is also unused: this VS divides the
    // grid irradiance by reflection irradiance directly, before interpolation.
    let world = native_world_vertex(
        NativeWorldVertexInput(input.position, input.color, input.uv,
            input.packed_normal, input.packed_tangent, vec2<f32>(0.0)),
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
    let grid_scale = fma(object.grid_lighting_sh0.w, squared_yz.y, grid_quadratic + grid_linear);
    let grid_irradiance = max(grid_scale * object.grid_lighting_sh0.xyz,
        vec3<f32>(0.10000000149011612));
    return NativeModelVaryings(world.clip_position, world.color, input.uv,
        world.normal_fog, world.fog_color, world.bitangent, world.tangent,
        vec4<f32>(world.eye_to_vertex, 0.0), object.grid_lighting_coords_and_vis,
        grid_irradiance / reflection_irradiance);
}

// Native t0/s0 color, t1/s1 normal, t13/s13 MODEL LIGHTING 3D texture,
// t15/s15 reflection cube. The 3D texture is the native lighting lookup, not
// the world's directional lightmap or a replacement ambient-light constant.
// Preserve native texture encoding and sampler states; no material CB is used.
fn native_model_fragment(
    input: NativeModelVaryings,
    scene: NativeModelScene,
    color_map: texture_2d<f32>, color_sampler: sampler,
    normal_map: texture_2d<f32>, normal_sampler: sampler,
    model_lighting: texture_3d<f32>, model_lighting_sampler: sampler,
    reflection_probe: texture_cube<f32>, reflection_sampler: sampler,
) -> vec4<f32> {
    let normal_sample = textureSample(normal_map, normal_sampler, input.uv);
    let slopes = fma(normal_sample.rg, vec2<f32>(4.015748023986816), vec2<f32>(-2.0157480239868164));
    let mapped_normal = fma(vec3<f32>(slopes.y), input.bitangent,
        fma(vec3<f32>(slopes.x), input.tangent, input.normal_fog.xyz));
    let normal = inverseSqrt(dot(mapped_normal, mapped_normal)) * mapped_normal;
    let incident = inverseSqrt(dot(input.eye_to_vertex.xyz, input.eye_to_vertex.xyz)) * input.eye_to_vertex.xyz;
    let reflection_direction = fma(normal, vec3<f32>(-2.0 * dot(incident, normal)), incident);
    let fresnel = exp2(clamp(dot(normal, -incident), 0.0, 1.0) * -9.279999732971191);
    let color_sample = textureSample(color_map, color_sampler, input.uv);
    let gloss = color_sample.a;
    let probe_sample = textureSampleLevel(reflection_probe, reflection_sampler,
        reflection_direction, fma(-4.0, gloss, 4.0));
    let probe = (1.0 / (probe_sample.a + 9.999999974752427e-07)) * probe_sample.rgb;
    let reflection_strength = fma(gloss, min(fresnel, gloss * 0.4749999940395355),
        fma(gloss, 0.027499999850988388, 0.014999999664723873));
    // Color samples and vertex RGB multiply before the source square transfer.
    let colored = color_sample.rgb * input.color.rgb;
    let diffuse = colored * colored;
    let reflected = (reflection_strength * probe) * input.reflection_correction;

    let major_axis = max(max(abs(normal.z), abs(normal.y)), abs(normal.x));
    let lighting_direction = (normal * scene.lighting_lookup_scale.xyz) / major_axis;
    let lighting_coords = lighting_direction + input.grid_coords_and_vis.xyz;
    let lighting_sample = textureSample(model_lighting, model_lighting_sampler, lighting_coords);
    let irradiance = lighting_sample.rgb * lighting_sample.rgb;
    let diffuse_lighting = diffuse * irradiance;
    let lit = fma(diffuse_lighting, vec3<f32>(32.0), reflected);
    let fogged = fma(vec3<f32>(input.normal_fog.w), lit - input.fog_color, input.fog_color);
    // Original encoded render-target output, matching the world-port contract.
    return vec4<f32>(sqrt(fogged * scene.world.hdr_control0.x), 1.0);
}

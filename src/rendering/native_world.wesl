// T6 wpc_lit_sm_r0c0n0x0_f488034e, LIT pimp_technique_lmap_46f86ab7.
// VS b89a2252: DXBC SHA256 141c0bd5e5d3d456fcea5c3a862f7182189495ef4552bdd6a9b6212d02a6e74b.
// PS 9a71a1e7: DXBC SHA256 87d754bc643d43073c60f47dd53e3a7ca1f5cc75d8e7a458204004096299ca24.
// Ported from rendering-20261012/shaders/<sha256>.asm and original DXBC literals.
// Host entrypoints own bindings and interpolation. These are compact input
// structs, not byte-compatible copies of the original constant buffers.

// 00416df0 / PS cb0[60..66]. Four globally selected lights, in source units.
struct NativeOmniLights {
    position_x: vec4<f32>,
    position_y: vec4<f32>,
    position_z: vec4<f32>,
    falloffs: vec4<f32>,
    reds: vec4<f32>,
    greens: vec4<f32>,
    blues: vec4<f32>,
    enabled: vec4<f32>,
}

struct NativeWorldScene {
    // Original cb0[36..40]: dot products with these ROWS produce clip position.
    view_projection_rows: array<vec4<f32>, 4>,
    inverse_view_rows: array<vec4<f32>, 3>, // Original cb0[40..42], right/up/forward.

    fog_color: vec4<f32>,       // cb0[26]
    fog_consts: vec4<f32>,      // cb0[27]
    fog_consts2: vec4<f32>,     // cb0[28]
    sun_fog_dir: vec4<f32>,     // cb0[29].xyz
    sun_fog_color: vec4<f32>,   // cb0[30]
    sun_fog: vec4<f32>,         // cb0[31].xy
    hdr_control0: vec4<f32>,    // PS cb0[20].x
    game_time: vec4<f32>,      // original gameTime.w in seconds
    sun_position: vec4<f32>,   // original cb0[18]
    sun_diffuse: vec4<f32>,    // original cb0[19]
    omni_lights: NativeOmniLights,
}

// pimp_shader_lmap_37a44012: combined_glight branch. Preserve the native
// reciprocal-distance and squared linear falloff, rather than inverse square.
fn native_omni_diffuse(
    lights: NativeOmniLights, position: vec3<f32>, normal: vec3<f32>,
) -> vec3<f32> {
    let x = lights.position_x - vec4<f32>(position.x);
    let y = lights.position_y - vec4<f32>(position.y);
    let z = lights.position_z - vec4<f32>(position.z);
    let distance = sqrt(fma(z, z, fma(x, x, y * y)));
    let reciprocal_distance = vec4<f32>(1.0) / distance;
    let dx = x * reciprocal_distance;
    let dy = y * reciprocal_distance;
    let dz = z * reciprocal_distance;
    let falloff = clamp(fma(distance, lights.falloffs, vec4<f32>(1.0)), vec4<f32>(0.0), vec4<f32>(1.0));
    let cosine = clamp(fma(dz, vec4<f32>(normal.z),
        fma(dx, vec4<f32>(normal.x), dy * normal.y)), vec4<f32>(0.0), vec4<f32>(1.0));
    let weights = cosine * (falloff * falloff);
    return vec3<f32>(dot(lights.reds, weights), dot(lights.greens, weights), dot(lights.blues, weights));
}

struct NativeWorldObject {
    // Original cb3[0..4]. Native axes/length units must also match fog and probe
    // data. The resulting xyz is the eye-to-vertex vector used by the shader;
    // compose the current eye translation here, not a frozen camera position.
    world_rows: array<vec4<f32>, 4>,
    reflection_lighting_sh0: vec4<f32>, // cb3[8]
    reflection_lighting_sh1: vec4<f32>, // cb3[9]
    reflection_lighting_sh2: vec4<f32>, // cb3[10]
    color: vec4<f32>, // per-mark packed color; ordinary world surfaces use one
}

struct NativeWorldVertexInput {
    // Source POSITION.xyz is in inches; w carries tangent handedness, not a
    // homogeneous coordinate. The source VS substitutes 1 for transformation.
    position: vec4<f32>,
    color: vec4<f32>,
    uv: vec2<f32>,
    packed_normal: vec3<f32>,   // Original NORMAL unsigned packed components.
    packed_tangent: vec3<f32>,  // Original TEXCOORD2 unsigned packed components.
    lightmap_uv: vec2<f32>,
}

struct NativeWorldVaryings {
    clip_position: vec4<f32>,
    color: vec4<f32>,
    uv_lightmap: vec4<f32>,
    normal_fog: vec4<f32>,
    fog_color: vec3<f32>,
    bitangent: vec3<f32>,
    tangent: vec3<f32>,
    eye_to_vertex: vec3<f32>,
    reflection_correction: vec3<f32>,
}

fn native_world_unpack_direction(packed: vec3<f32>) -> vec3<f32> {
    return select(2.0 * packed, 2.0 * packed - vec3<f32>(2.0), packed >= vec3<f32>(0.5));
}

// This entry also accepts an already decoded source-space basis. A glTF basis
// must first be converted back to the same source frame as position/world_rows.
fn native_world_vertex_with_basis(
    input: NativeWorldVertexInput,
    decoded_normal: vec3<f32>,
    decoded_tangent: vec3<f32>,
    scene: NativeWorldScene,
    object: NativeWorldObject,
) -> NativeWorldVaryings {
    let position = vec4<f32>(input.position.xyz, 1.0);
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

    let transformed_normal = vec3<f32>(
        dot(decoded_normal, object.world_rows[0].xyz),
        dot(decoded_normal, object.world_rows[1].xyz),
        dot(decoded_normal, object.world_rows[2].xyz),
    );
    let normal = inverseSqrt(dot(transformed_normal, transformed_normal)) * transformed_normal;
    output.normal_fog = vec4<f32>(normal, fma(-fog_amount, fog.w, 1.0));
    let transformed_tangent = vec3<f32>(
        dot(decoded_tangent, object.world_rows[0].xyz),
        dot(decoded_tangent, object.world_rows[1].xyz),
        dot(decoded_tangent, object.world_rows[2].xyz),
    );
    let tangent = inverseSqrt(dot(transformed_tangent, transformed_tangent)) * transformed_tangent;
    output.tangent = tangent;
    output.bitangent = sign(input.position.w) * cross(normal, tangent);
    output.eye_to_vertex = world_position.xyz;

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

fn native_world_vertex(
    input: NativeWorldVertexInput,
    scene: NativeWorldScene,
    object: NativeWorldObject,
) -> NativeWorldVaryings {
    return native_world_vertex_with_basis(input,
        native_world_unpack_direction(input.packed_normal),
        native_world_unpack_direction(input.packed_tangent), scene, object);
}

// Native t0/s0 color, t1/s1 normal, t13/s13 secondary lightmap, t15/s15 probe.
// Preserve UNORM samples and source samplers. Color alpha is gloss; vertex
// color multiplies the color sample BEFORE squaring. Normals are RG slopes,
// not tangent-space XYZ or the usual reconstructed-positive-Z BC5 normals.
// Lightmap has three vertical bands: ambient, directional color, direction.
// Probe mip chain and RGB/(alpha+epsilon) encoding are original input data.
fn native_world_fragment(
    input: NativeWorldVaryings,
    scene: NativeWorldScene,
    color_map: texture_2d<f32>,
    color_sampler: sampler,
    normal_map: texture_2d<f32>,
    normal_sampler: sampler,
    lightmap_secondary: texture_2d<f32>,
    lightmap_sampler: sampler,
    reflection_probe: texture_cube<f32>,
    reflection_sampler: sampler,
) -> vec4<f32> {
    let normal_sample = textureSample(normal_map, normal_sampler, input.uv_lightmap.xy);
    let slopes = fma(normal_sample.xy, vec2<f32>(4.015748023986816), vec2<f32>(-2.0157480239868164));
    let mapped_normal = fma(vec3<f32>(slopes.y), input.bitangent,
        fma(vec3<f32>(slopes.x), input.tangent, input.normal_fog.xyz));
    let normal = inverseSqrt(dot(mapped_normal, mapped_normal)) * mapped_normal;
    let incident = inverseSqrt(dot(input.eye_to_vertex, input.eye_to_vertex)) * input.eye_to_vertex;
    let reflection_direction = fma(normal, vec3<f32>(-2.0 * dot(incident, normal)), incident);
    let fresnel = exp2(clamp(dot(normal, -incident), 0.0, 1.0) * -9.279999732971191);
    let color_sample = textureSample(color_map, color_sampler, input.uv_lightmap.xy);
    let gloss = color_sample.a;
    let probe_sample = textureSampleLevel(reflection_probe, reflection_sampler, reflection_direction, fma(-4.0, gloss, 4.0));
    let probe = probe_sample.rgb / (probe_sample.a + 9.999999974752427e-07);
    let reflection_strength = fma(gloss, min(fresnel, gloss * 0.4749999940395355),
        fma(gloss, 0.027499999850988388, 0.014999999664723873));
    let colored = color_sample.rgb * input.color.rgb;
    let diffuse = colored * colored;
    let reflected = reflection_strength * probe;

    let lm_uv = input.uv_lightmap.zw;
    let ambient_uv = lm_uv * vec2<f32>(1.0, 0.3333333432674408);
    let ambient_sample = textureSample(lightmap_secondary, lightmap_sampler, ambient_uv);
    let ambient = ambient_sample.rgb / (ambient_sample.a + 9.999999974752427e-07);
    let other_uv = fma(lm_uv.xyxy, vec4<f32>(1.0, 0.3333333432674408, 1.0, 0.3333333432674408),
        vec4<f32>(0.0, 0.3333333432674408, 0.0, 0.6666666865348816));
    let directional_sample = textureSample(lightmap_secondary, lightmap_sampler, other_uv.xy);
    let direction_sample = textureSample(lightmap_secondary, lightmap_sampler, other_uv.zw);
    let light_direction = fma(direction_sample.rgb, vec3<f32>(2.0), vec3<f32>(-1.0));
    let directional = directional_sample.rgb / (directional_sample.a + 9.999999974752427e-07);
    let diffuse_light = fma(directional, vec3<f32>(clamp(dot(light_direction, normal), 0.0, 1.0)), ambient);
    let reflection_light = fma(directional, vec3<f32>(clamp(dot(light_direction, input.normal_fog.xyz), 0.0, 1.0)), ambient)
        * input.reflection_correction;
    let lit = fma(diffuse, diffuse_light, reflected * reflection_light);
    let fogged = fma(vec3<f32>(input.normal_fog.w), lit - input.fog_color, input.fog_color);
    // Original render-target encoding. This is NOT linear Bevy HDR output and
    // must not pass through an additional sRGB encode or a PBR tonemapper.
    return vec4<f32>(sqrt(fogged * scene.hdr_control0.x), 1.0);
}

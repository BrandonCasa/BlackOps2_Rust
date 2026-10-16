import package::rendering::native_world::NativeWorldScene;

// Original mc_skycubemaphdr_1f117w7u, UNLIT/EMISSIVE slot2/3.
// VS2306aa25 SHA256 c487cb809bc9966defa62bd849050c03c69e0ed4450b909d7a8361e7e00b3232.
// PS2306aa25 SHA256 20df10d8a579a1f007c665fbd2016eebe4194c0ea851f24a930d9c110f16448d.
// No sunlight input: weather does not multiply the source cube radiance.
struct NativeSkyUniform {
    scene: NativeWorldScene,
    world_rows: array<vec4<f32>, 4>,
    rotation_size: vec4<f32>,
    color_parms: vec4<f32>,
}

struct NativeSkyVaryings {
    clip_position: vec4<f32>,
    cube_directions: vec4<f32>,
    far_position_transition: vec4<f32>,
    fog_color_amount: vec4<f32>,
}

fn native_sky_fog_amount(position: vec3<f32>, scene: NativeWorldScene) -> f32 {
    let height_term = fma(scene.fog_consts.w, position.z, scene.fog_consts.x);
    let height_exp = exp2(min(height_term, 64.0) * 1.4426950216293335);
    let height_integral = select(height_term + 1.0, height_exp, height_term < 0.0);
    let delta = position.z * scene.fog_consts.w;
    let horizontal = abs(delta) < 9.999999747378752e-05;
    let denominator = select(delta, 1.0, horizontal);
    let integral = select((height_integral - scene.fog_consts2.x) / denominator,
        clamp(scene.fog_consts2.x, 0.0, 1.0), horizontal) * scene.fog_consts.y;
    return 1.0 - min(exp2(fma(integral, sqrt(dot(position, position)), scene.fog_consts.z)), 1.0);
}

fn native_sky_vertex(position: vec3<f32>, sky: NativeSkyUniform) -> NativeSkyVaryings {
    // The source VS uses dp3 for all four world rows. Omitting translation
    // makes this camera-centered and places depth at the projection far plane.
    let world = vec4<f32>(dot(position, sky.world_rows[0].xyz),
        dot(position, sky.world_rows[1].xyz), dot(position, sky.world_rows[2].xyz),
        dot(position, sky.world_rows[3].xyz));
    var output: NativeSkyVaryings;
    output.clip_position = vec4<f32>(dot(world, sky.scene.view_projection_rows[0]),
        dot(world, sky.scene.view_projection_rows[1]), dot(world, sky.scene.view_projection_rows[2]),
        dot(world, sky.scene.view_projection_rows[3]));
    let z = select(position.z, -position.z, sky.rotation_size.w < 0.0);
    output.cube_directions = vec4<f32>(dot(position.xy, sky.rotation_size.yz),
        dot(position.xy, sky.rotation_size.xy), z, -z);
    let far_position = inverseSqrt(dot(position, position)) * position * 20000000.0;
    // Original r_skyTransition defaults to zero; no Origins weather changes it.
    output.far_position_transition = vec4<f32>(far_position, 0.0);
    let direction = inverseSqrt(dot(far_position, far_position)) * far_position;
    let sun_amount = clamp(fma(sky.scene.sun_fog.y,
        dot(sky.scene.sun_fog_dir.xyz, direction), sky.scene.sun_fog.x), 0.0, 1.0);
    let fog = fma(vec4<f32>(sun_amount), sky.scene.sun_fog_color - sky.scene.fog_color, sky.scene.fog_color);
    let amount = native_sky_fog_amount(far_position, sky.scene);
    // Preserve source mad(-amount, opacity, 1), then 1-result rounding.
    output.fog_color_amount = vec4<f32>(fog.xyz, 1.0 - fma(-amount, fog.w, 1.0));
    return output;
}

fn native_sky_fragment(input: NativeSkyVaryings, sky: NativeSkyUniform,
    cube: texture_cube<f32>, cube_sampler: sampler) -> vec4<f32> {
    let first = textureSample(cube, cube_sampler, input.cube_directions.xyz);
    let second = textureSample(cube, cube_sampler, input.cube_directions.xyw);
    var a: vec3<f32>;
    var b: vec3<f32>;
    if sky.color_parms.x > 0.0 {
        a = first.rgb / (first.a + 9.999999974752427e-07);
        b = second.rgb / (second.a + 9.999999974752427e-07);
    } else {
        a = first.rgb * first.rgb;
        b = second.rgb * second.rgb;
    }
    let blended = fma(vec3<f32>(input.far_position_transition.w), b - a, a);
    // r_skyColorTemp6500 produces identity, r_skyColorMultiplier defaults1.
    let color = blended * sky.color_parms.y;
    let pixel_fog = native_sky_fog_amount(input.far_position_transition.xyz, sky.scene);
    let amount = pixel_fog * input.fog_color_amount.w;
    let fog_delta = fma(-color, vec3<f32>(1.0), input.fog_color_amount.rgb) * amount;
    let radiance = fma(vec3<f32>(sky.color_parms.w), fog_delta, color);
    return vec4<f32>(sqrt(radiance * sky.scene.hdr_control0.x), 1.0);
}

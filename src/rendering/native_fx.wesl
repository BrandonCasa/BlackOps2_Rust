// T6 effect_w77q49e8, UNLIT/EMISSIVE pimp_technique_zfeather_5d3514a4.
// VS 409a0e7d: SHA256 39764eb10a784b5128edbfac298c2ee711bc37cfe1c15c58a88562cc86d96160.
// PS 409a0e7d: SHA256 4b65550e08213987dedbf4d4c96414de2f970244e8124c251734637cf87b7997.
// Equations and literals come from rendering-20261012/shaders/<sha256>.asm.
// The host owns source material state, bindings and perspective interpolation.
import package::rendering::native_world::NativeWorldScene;

struct NativeFxFrame {
    scene: NativeWorldScene,
    z_near: vec4<f32>,             // PS cb0[58].x, original floatZ encoding.
    render_target_size: vec4<f32>, // PS cb0[95].zw: reciprocal target size.
}

struct NativeFxObject {
    // VS cb3[0..4]. Source axes and inches, with the current eye translation.
    world_rows: array<vec4<f32>, 4>,
    feather_parms: vec4<f32>, // VS/PS cb1[46]; preserve the material literal.
}

struct NativeFxVertexInput {
    // Source code.position.xyz; its fourth component is ignored by this VS.
    // GENERIC declaration: position Float32x4 at 0, color Unorm8x4 at 16,
    // texcoord Float32x2 at 20, stride 32. No normal is consumed by this pass.
    position: vec3<f32>,
    color: vec4<f32>,
    uv: vec2<f32>,
}

struct NativeFxVaryings {
    clip_position: vec4<f32>,
    color: vec4<f32>,
    fog_color: vec3<f32>,
    uv_depth_fog: vec4<f32>,
}

fn native_fx_vertex(
    input: NativeFxVertexInput,
    frame: NativeFxFrame,
    object: NativeFxObject,
) -> NativeFxVaryings {
    let scene = frame.scene;
    let position = vec4<f32>(input.position, 1.0);
    let world_position = vec4<f32>(
        dot(position, object.world_rows[0]),
        dot(position, object.world_rows[1]),
        dot(position, object.world_rows[2]),
        dot(position, object.world_rows[3]),
    );
    var output: NativeFxVaryings;
    output.clip_position = vec4<f32>(
        dot(world_position, scene.view_projection_rows[0]),
        dot(world_position, scene.view_projection_rows[1]),
        dot(world_position, scene.view_projection_rows[2]),
        dot(world_position, scene.view_projection_rows[3]),
    );
    let vertex_depth = output.clip_position.w;
    output.color = vec4<f32>(input.color.xyz,
        clamp(vertex_depth * object.feather_parms.x, 0.0, 1.0) * input.color.w);

    let distance_squared = dot(world_position.xyz, world_position.xyz);
    let direction = inverseSqrt(distance_squared) * world_position.xyz;
    let sun_amount = clamp(fma(scene.sun_fog.y,
        dot(scene.sun_fog_dir.xyz, direction), scene.sun_fog.x), 0.0, 1.0);
    let fog = fma(vec4<f32>(sun_amount), scene.sun_fog_color - scene.fog_color,
        scene.fog_color);
    output.fog_color = sqrt(clamp(fog.xyz * scene.hdr_control0.x,
        vec3<f32>(0.0), vec3<f32>(1.0)));

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
    let fog_amount = 1.0 - min(exp2(fma(fog_integral, sqrt(distance_squared),
        scene.fog_consts.z)), 1.0);
    let fog_transmission = fma(-fog_amount, fog.w, 1.0);
    // The source squares transmission before perspective interpolation.
    output.uv_depth_fog = vec4<f32>(input.uv, vertex_depth,
        fog_transmission * fog_transmission);
    return output;
}

fn native_fx_fragment(
    input: NativeFxVaryings,
    fragment_position: vec4<f32>,
    frame: NativeFxFrame,
    object: NativeFxObject,
    color_map: texture_2d<f32>,
    color_sampler: sampler,
    float_z: texture_2d<f32>,
    float_z_sampler: sampler,
) -> vec4<f32> {
    let screen_uv = fragment_position.xy * frame.render_target_size.zw;
    let sampled_depth = textureSample(float_z, float_z_sampler, screen_uv).x;
    // floatZ stores zNear/viewDepth. A stock forward-Z depth attachment must
    // first be resolved to this source encoding, or the intersection fades fail.
    let scene_depth = frame.z_near.x / max(abs(sampled_depth), 0.0);
    let depth_feather = clamp((scene_depth - input.uv_depth_fog.z)
        * object.feather_parms.x, 0.0, 1.0);
    let sample = textureSample(color_map, color_sampler, input.uv_depth_fog.xy);
    let color = fma(vec3<f32>(input.uv_depth_fog.w),
        fma(input.color.xyz, sample.xyz, -input.fog_color), input.fog_color);
    return vec4<f32>(color, depth_feather * (sample.w * input.color.w));
}

// wpc_sw4_3d_unlit_1layer_8691j77q, LIT VS20d9eb7d / PSc5daea2d.
// Original DXBC disassembly: target/reports/wallbuy-{vs,ps}.asm.
import package::rendering::native_world::{NativeWorldScene, NativeWorldObject,
    native_world_unpack_direction};
import package::rendering::native_world_plain::{NativeWorldGeometricVertexInput,
    native_world_position_fog};

struct NativeWorldParallaxLayer {
    scale_offset: vec4<f32>,       // cb2[5]: Layer1Scale.xy, Layer1Offset.xy
    scroll_alpha_hdr: vec4<f32>,   // cb2[6]: Layer1Scroll.xy, Layer1Alpha, hdrAmount
    rotation_origin: vec4<f32>,    // cb2[7]: Layer1Rotation.xy, Layer1Origin.xy
    depth: vec4<f32>,              // cb2[8].x: Layer1Depth
}

struct NativeWallbuyInput {
    position: vec4<f32>,
    uv: vec2<f32>,
    packed_normal: vec3<f32>,
    packed_tangent: vec3<f32>,
}

struct NativeWallbuyVaryings {
    clip_position: vec4<f32>,
    uv: vec2<f32>,
    fog_transmittance: f32,
    fog_color: vec3<f32>,
}

fn native_wallbuy_vertex(
    input: NativeWallbuyInput,
    scene: NativeWorldScene,
    object: NativeWorldObject,
    material: NativeWorldParallaxLayer,
    game_time_seconds: f32,
) -> NativeWallbuyVaryings {
    let position = native_world_position_fog(NativeWorldGeometricVertexInput(
        input.position.xyz, vec4<f32>(0.0), input.uv, vec3<f32>(0.0), vec2<f32>(0.0)),
        scene, object);
    let decoded_normal = native_world_unpack_direction(input.packed_normal);
    let decoded_tangent = native_world_unpack_direction(input.packed_tangent);
    let transformed_normal = vec3<f32>(dot(decoded_normal, object.world_rows[0].xyz),
        dot(decoded_normal, object.world_rows[1].xyz), dot(decoded_normal, object.world_rows[2].xyz));
    let transformed_tangent = vec3<f32>(dot(decoded_tangent, object.world_rows[0].xyz),
        dot(decoded_tangent, object.world_rows[1].xyz), dot(decoded_tangent, object.world_rows[2].xyz));
    let normal = inverseSqrt(dot(transformed_normal, transformed_normal)) * transformed_normal;
    let tangent = inverseSqrt(dot(transformed_tangent, transformed_tangent)) * transformed_tangent;
    // This VS uses NEGATIVE position.w sign, unlike the ordinary LIT basis.
    let bitangent = -sign(input.position.w) * cross(normal, tangent);
    let eye = -position.eye_to_vertex;
    let direction = inverseSqrt(dot(eye, eye)) * eye;
    let facing = dot(direction, normal);
    var parallax = vec2<f32>(0.0);
    if facing > 0.0 {
        parallax = -vec2<f32>(dot(direction, tangent), dot(direction, bitangent)) / facing;
    }
    // Source adds this offset even when the authored depth is exactly zero.
    let uv = native_world_parallax_uv(input.uv, parallax,
        material.depth.x + 9.999999747378752e-06, material, game_time_seconds);
    return NativeWallbuyVaryings(position.clip_position, uv,
        position.normal_fog.w, position.fog_color);
}

// Shared by the source wallbuy VS and lava PS. Lava uses its exact authored
// depth; only wallbuy adds the source epsilon before calling this sequence.
fn native_world_parallax_uv(source_uv: vec2<f32>, ray: vec2<f32>, depth: f32,
    material: NativeWorldParallaxLayer, game_time_seconds: f32) -> vec2<f32> {
    var uv = (ray * depth) * material.scale_offset.xy;
    uv = fma(source_uv - material.rotation_origin.zw, material.scale_offset.xy, uv);
    uv = fma(material.scale_offset.zw, material.scale_offset.xy, uv);
    let angle = (fma(material.rotation_origin.y, game_time_seconds, 1.0)
        * material.rotation_origin.x) * 6.2831854820251465;
    let sine = sin(angle);
    let cosine = cos(angle);
    let rotated = vec2<f32>(dot(vec2<f32>(cosine, sine), uv),
        dot(vec2<f32>(-sine, cosine), uv));
    uv = fma(game_time_seconds * material.scroll_alpha_hdr.xy, material.scale_offset.xy, rotated);
    return uv + material.rotation_origin.zw;
}

fn native_wallbuy_fragment(
    input: NativeWallbuyVaryings,
    scene: NativeWorldScene,
    material: NativeWorldParallaxLayer,
    script_vector0: vec4<f32>,
    layer_map: texture_2d<f32>, layer_sampler: sampler,
) -> vec4<f32> {
    let sample = textureSample(layer_map, layer_sampler, input.uv);
    let color = (sample.a * sample.rgb) * material.scroll_alpha_hdr.w;
    let delta = fma(color, color, -input.fog_color);
    let fogged = fma(vec3<f32>(input.fog_transmittance), delta, input.fog_color);
    let alpha = ((1.0 - clamp(script_vector0.x, 0.0, 1.0)) * material.scroll_alpha_hdr.z) * sample.a;
    return vec4<f32>(sqrt(fogged * scene.hdr_control0.x), sqrt(alpha));
}

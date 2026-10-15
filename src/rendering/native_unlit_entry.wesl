import package::rendering::native_world::{NativeWorldScene, NativeWorldObject};
import package::rendering::native_world::{NativeWorldVertexInput, native_world_vertex};
import package::rendering::native_world_lava::{NativeLavaConstants, NativeLavaVaryings, native_lava_fragment};
import package::rendering::native_world_plain::{NativeWorldGeometricVertexInput, native_world_position_fog};
import package::rendering::native_world_wallbuy::{NativeWorldParallaxLayer, NativeWallbuyInput,
    NativeWallbuyVaryings, native_wallbuy_vertex, native_wallbuy_fragment};

@group(0) @binding(0) var<uniform> scene: NativeWorldScene;
@group(1) @binding(0) var<uniform> object: NativeWorldObject;
@group(2) @binding(0) var color_map: texture_2d<f32>;
@group(2) @binding(1) var color_sampler: sampler;
struct UnlitConstants { scale_rgb: vec4<f32> }
@group(2) @binding(2) var<uniform> material: UnlitConstants;
@group(2) @binding(3) var<uniform> wallbuy: NativeWorldParallaxLayer;
@group(2) @binding(8) var lava_layer2: texture_2d<f32>;
@group(2) @binding(9) var lava_sampler2: sampler;
@group(2) @binding(10) var lava_layer3: texture_2d<f32>;
@group(2) @binding(11) var lava_sampler3: sampler;
@group(2) @binding(12) var<uniform> lava: NativeLavaConstants;

struct LavaInput {
    @location(0) position: vec4<f32>,
    @location(1) color: vec4<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) normal: vec4<f32>,
    @location(4) tangent: vec4<f32>,
}
struct LavaVaryings {
    @builtin(position) position: vec4<f32>,
    @location(0) fog_color: vec3<f32>,
    @location(1) eye_fog: vec4<f32>,
    @location(2) color: vec4<f32>,
    @location(3) normal: vec3<f32>,
    @location(4) tangent_sign: vec4<f32>,
    @location(5) uv: vec2<f32>,
}
@vertex
fn vertex_lava(input: LavaInput) -> LavaVaryings {
    let output = native_world_vertex(NativeWorldVertexInput(input.position, input.color, input.uv,
        input.normal.xyz, input.tangent.xyz, vec2<f32>(0.0)), scene, object);
    return LavaVaryings(output.clip_position, output.fog_color,
        vec4<f32>(output.eye_to_vertex, output.normal_fog.w), input.color,
        output.normal_fog.xyz, vec4<f32>(output.tangent, sign(input.position.w)), input.uv);
}
fn shade_lava(input: LavaVaryings) -> vec4<f32> {
    // Like the wallbuy decals, this map's lava surface belongs to static BSP.
    return native_lava_fragment(NativeLavaVaryings(input.position, input.fog_color, input.eye_fog,
        input.color, input.normal, input.tangent_sign, input.uv), scene, lava, vec4<f32>(0.0),
        color_map, color_sampler, lava_layer2, lava_sampler2, lava_layer3, lava_sampler3);
}
@fragment
fn fragment_lava(input: LavaVaryings) -> @location(0) vec4<f32> { return shade_lava(input); }
@fragment
fn fragment_lava_srgb(input: LavaVaryings) -> @location(0) vec4<f32> { return attachment_srgb(shade_lava(input)); }

struct WallbuyInput {
    @location(0) position: vec4<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) normal: vec4<f32>,
    @location(4) tangent: vec4<f32>,
}
struct WallbuyVaryings {
    @builtin(position) position: vec4<f32>,
    @location(0) uv_fog: vec3<f32>,
    @location(1) fog_color: vec3<f32>,
}
@vertex
fn vertex_wallbuy(input: WallbuyInput) -> WallbuyVaryings {
    let output = native_wallbuy_vertex(NativeWallbuyInput(input.position, input.uv,
        input.normal.xyz, input.tangent.xyz), scene, object, wallbuy, scene.game_time.w);
    return WallbuyVaryings(output.clip_position, vec3<f32>(output.uv, output.fog_transmittance), output.fog_color);
}
fn shade_wallbuy(input: WallbuyVaryings) -> vec4<f32> {
    // Static BSP uses the original zero-initialized scriptVector0 (code169,
    // descriptor class2, sourceState+0x1290). Entity overrides are separate.
    return native_wallbuy_fragment(NativeWallbuyVaryings(input.position, input.uv_fog.xy,
        input.uv_fog.z, input.fog_color), scene, wallbuy, vec4<f32>(0.0), color_map, color_sampler);
}
@fragment
fn fragment_wallbuy(input: WallbuyVaryings) -> @location(0) vec4<f32> { return shade_wallbuy(input); }
@fragment
fn fragment_wallbuy_srgb(input: WallbuyVaryings) -> @location(0) vec4<f32> { return attachment_srgb(shade_wallbuy(input)); }

struct Input {
    @location(0) position: vec3<f32>,
    @location(1) color: vec4<f32>,
    @location(2) uv: vec2<f32>,
}
struct Varyings {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) uv_fog: vec3<f32>,
    @location(2) fog_color: vec3<f32>,
}

// VS4876d431 and VS5c3f6d9c share the existing native position/fog sequence.
// Only position and fog transmittance are consumed; no normal or lightmap is read.
@vertex
fn vertex_fog(input: Input) -> Varyings {
    let source = NativeWorldGeometricVertexInput(input.position, input.color,
        input.uv, vec3<f32>(0.0), vec2<f32>(0.0));
    let output = native_world_position_fog(source, scene, object);
    return Varyings(output.clip_position, input.color, vec3<f32>(input.uv, output.normal_fog.w), output.fog_color);
}

// VS62b62074 explicitly supplies fog=1. Preserve its no-fog behavior.
@vertex
fn vertex_no_fog(input: Input) -> Varyings {
    let position = vec4<f32>(input.position, 1.0);
    let world = vec4<f32>(dot(position, object.world_rows[0]),
        dot(position, object.world_rows[1]), dot(position, object.world_rows[2]),
        dot(position, object.world_rows[3]));
    let clip = vec4<f32>(dot(world, scene.view_projection_rows[0]),
        dot(world, scene.view_projection_rows[1]), dot(world, scene.view_projection_rows[2]),
        dot(world, scene.view_projection_rows[3]));
    return Varyings(clip, input.color, vec3<f32>(input.uv, 1.0), vec3<f32>(0.0));
}

// PS4b1d2e0b: source gamma color squared, scaleRGB.x, fog, HDR, sqrt.
fn shade_add(input: Varyings) -> vec4<f32> {
    let color = textureSample(color_map, color_sampler, input.uv_fog.xy) * input.color;
    let scaled = (color.rgb * color.rgb) * material.scale_rgb.x;
    return vec4<f32>(sqrt((scaled * input.uv_fog.z) * scene.hdr_control0.x), color.a);
}

// PS62b62074 weights green twice as much as red/blue, without HDR transfer.
fn shade_decal_add(input: Varyings) -> vec4<f32> {
    let color = textureSample(color_map, color_sampler, input.uv_fog.xy) * input.color;
    let opacity = (color.a * dot(color.rbg, vec3<f32>(0.25, 0.25, 0.5))) * input.uv_fog.z;
    return vec4<f32>(opacity * color.rgb, opacity);
}

// PS5c3f6d9c fades toward the multiplicative identity in fog.
fn shade_decal_multiply(input: Varyings) -> vec4<f32> {
    let sample = textureSample(color_map, color_sampler, input.uv_fog.xy);
    let delta = fma(sample.rgb, input.color.rgb, vec3<f32>(-1.0));
    let opacity = (sample.a * input.color.a) * input.uv_fog.z;
    return vec4<f32>(fma(vec3<f32>(opacity), delta, vec3<f32>(1.0)), opacity);
}

fn attachment_srgb(encoded: vec4<f32>) -> vec4<f32> {
    let linear = select(encoded.rgb / vec3<f32>(12.92),
        pow((encoded.rgb + vec3<f32>(0.055)) / vec3<f32>(1.055), vec3<f32>(2.4)),
        encoded.rgb > vec3<f32>(0.04045));
    return vec4<f32>(linear, encoded.a);
}
@fragment
fn fragment_add(input: Varyings) -> @location(0) vec4<f32> { return shade_add(input); }
@fragment
fn fragment_add_srgb(input: Varyings) -> @location(0) vec4<f32> { return attachment_srgb(shade_add(input)); }
@fragment
fn fragment_decal_add(input: Varyings) -> @location(0) vec4<f32> { return shade_decal_add(input); }
@fragment
fn fragment_decal_add_srgb(input: Varyings) -> @location(0) vec4<f32> { return attachment_srgb(shade_decal_add(input)); }
@fragment
fn fragment_decal_multiply(input: Varyings) -> @location(0) vec4<f32> { return shade_decal_multiply(input); }
@fragment
fn fragment_decal_multiply_srgb(input: Varyings) -> @location(0) vec4<f32> { return attachment_srgb(shade_decal_multiply(input)); }

// PS49c3292c: opaque unlit includes the source sun-dependent fog color.
fn shade_replace(input: Varyings) -> vec4<f32> {
    let color = textureSample(color_map, color_sampler, input.uv_fog.xy) * input.color;
    let delta = fma(color.rgb * color.rgb, vec3<f32>(material.scale_rgb.x), -input.fog_color);
    let fogged = fma(vec3<f32>(input.uv_fog.z), delta, input.fog_color);
    return vec4<f32>(sqrt(fogged * scene.hdr_control0.x), color.a);
}
@fragment
fn fragment_replace(input: Varyings) -> @location(0) vec4<f32> { return shade_replace(input); }
@fragment
fn fragment_replace_srgb(input: Varyings) -> @location(0) vec4<f32> { return attachment_srgb(shade_replace(input)); }

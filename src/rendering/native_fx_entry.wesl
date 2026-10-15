import package::rendering::native_fx::{
    NativeFxFrame, NativeFxObject, NativeFxVertexInput, NativeFxVaryings,
    native_fx_vertex, native_fx_fragment,
};

struct NativeEffectTransform {
    world_rows: array<vec4<f32>, 4>,
    outdoor_rows: array<vec4<f32>, 3>,
}
struct NativeEffectConstants { feather_parms: vec4<f32>, additive_parms: array<vec4<f32>, 4>, }

@group(0) @binding(0) var<uniform> frame: NativeFxFrame;
@group(0) @binding(1) var float_z: texture_2d<f32>;
@group(0) @binding(2) var float_z_sampler: sampler;
@group(1) @binding(0) var<uniform> transform: NativeEffectTransform;
@group(2) @binding(0) var color_map: texture_2d<f32>;
@group(2) @binding(1) var color_sampler: sampler;
@group(2) @binding(2) var<uniform> material: NativeEffectConstants;
@group(3) @binding(1) var outdoor_map: texture_2d<f32>;
@group(3) @binding(2) var outdoor_sampler: sampler;

struct VertexInput {
    @location(0) position: vec4<f32>,
    @location(1) color: vec4<f32>,
    @location(2) uv: vec2<f32>,
}

struct Varyings {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) fog_color: vec3<f32>,
    @location(2) uv_depth_fog: vec4<f32>,
}

fn source_object() -> NativeFxObject {
    return NativeFxObject(transform.world_rows, material.feather_parms);
}

@vertex
fn vertex(input: VertexInput) -> Varyings {
    let source = NativeFxVertexInput(input.position.xyz, input.color, input.uv);
    let result = native_fx_vertex(source, frame, source_object());
    return Varyings(result.clip_position, result.color, result.fog_color,
        result.uv_depth_fog);
}

@fragment
fn fragment(input: Varyings) -> @location(0) vec4<f32> {
    let source = NativeFxVaryings(input.position, input.color, input.fog_color,
        input.uv_depth_fog);
    let result = native_fx_fragment(source, input.position, frame, source_object(),
        color_map, color_sampler, float_z, float_z_sampler);
    // All recovered effect_w77q49e8 materials specify alphaTest gt0.
    if !(result.a > 0.0) { discard; }
    // Source RGB is encoded before blending; the target must preserve that
    // arithmetic. An sRGB attachment would change the blend itself.
    return result;
}

// effect_jqj65q53: VS1136047 / PS8788956c, original additive eye-offset pass.
struct AdditiveInput {
    @location(0) position: vec4<f32>,
    @location(1) color: vec4<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) normal: u32,
}
struct AdditiveVaryings {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) uv_depth: vec3<f32>,
}
fn additive_vertex(input: AdditiveInput, feather: bool, local_normal: bool) -> AdditiveVaryings {
    let local = vec4<f32>(input.position.xyz, 1.0);
    let world = vec4<f32>(dot(local, transform.world_rows[0]),
        dot(local, transform.world_rows[1]), dot(local, transform.world_rows[2]),
        dot(local, transform.world_rows[3]));
    let rows = frame.scene.view_projection_rows;
    var clip = vec4<f32>(dot(world, rows[0]), dot(world, rows[1]),
        dot(world, rows[2]), dot(world, rows[3]));
    let original_depth = clip.w;
    let offset_w = max(clip.w - material.additive_parms[3].x, clip.z);
    let xy_scale = max(offset_w / clip.w, 0.5);
    if !feather && clip.z < clip.w {
        clip = vec4<f32>(clip.xy * xy_scale, clip.z, offset_w);
    }
    let packed = vec3<u32>(input.normal, input.normal >> 10u, input.normal >> 20u) & vec3<u32>(1023u);
    let normal_unorm = vec3<f32>(packed) / 1023.0;
    // The source shader decodes UNORM10 as signed, without renormalization.
    var normal = select(normal_unorm * 2.0, normal_unorm * 2.0 - 2.0,
        normal_unorm >= vec3<f32>(0.5));
    // VS618fd291 applies the source object 3x3 after signed normal decoding.
    // It intentionally does not normalize the transformed normal.
    if local_normal {
        normal = vec3<f32>(dot(normal, transform.world_rows[0].xyz),
            dot(normal, transform.world_rows[1].xyz),
            dot(normal, transform.world_rows[2].xyz));
    }
    let facing = dot(normalize(world.xyz), normal);
    let falloff = clamp(fma(facing * facing, material.additive_parms[0].z,
        material.additive_parms[0].w), 0.0, 1.0);
    let tint = fma(vec3<f32>(falloff),
        material.additive_parms[1].xyz - material.additive_parms[2].xyz,
        material.additive_parms[2].xyz);
    let alpha = select(input.color.a, clamp(original_depth * material.feather_parms.x, 0.0, 1.0) * input.color.a, feather);
    return AdditiveVaryings(clip, vec4<f32>((falloff * input.color.xyz) * tint, alpha), vec3<f32>(input.uv, original_depth));
}
@fragment
fn fragment_additive(input: AdditiveVaryings) -> @location(0) vec4<f32> {
    let color = textureSample(color_map, color_sampler, input.uv_depth.xy) * input.color;
    return vec4<f32>(color.rgb * color.a, color.a);
}

@vertex
fn vertex_additive(input: AdditiveInput) -> AdditiveVaryings {
    return additive_vertex(input, false, false);
}
// effect_5e098749: VS6e83538d / PSa0389359. No fog or eye offset.
@vertex
fn vertex_additive_feather(input: AdditiveInput) -> AdditiveVaryings {
    return additive_vertex(input, true, false);
}
@fragment
fn fragment_additive_feather(input: AdditiveVaryings) -> @location(0) vec4<f32> {
    let uv = input.position.xy * frame.render_target_size.zw;
    let sampled_depth = textureSample(float_z, float_z_sampler, uv).x;
    let depth = frame.z_near.x / max(abs(sampled_depth), 0.0);
    let fade = clamp((depth - input.uv_depth.z) * material.feather_parms.x, 0.0, 1.0);
    let color = textureSample(color_map, color_sampler, input.uv_depth.xy) * input.color;
    let alpha = fade * color.a;
    return vec4<f32>(alpha * color.rgb, alpha);
}

// effect_z096f2e2: VS/PS b9c91c1c. No normal falloff, fog, or eye offset.
struct OutdoorVaryings {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) uv_depth: vec3<f32>,
    @location(2) outdoor: vec3<f32>,
}
// VS a0389359 shares this position/color/depth sequence with VS b9c91c1c.
fn simple_vertex(input: VertexInput, eye_offset: bool, feather: bool) -> AdditiveVaryings {
    let local = vec4<f32>(input.position.xyz, 1.0);
    let world = vec4<f32>(dot(local, transform.world_rows[0]),
        dot(local, transform.world_rows[1]), dot(local, transform.world_rows[2]),
        dot(local, transform.world_rows[3]));
    let rows = frame.scene.view_projection_rows;
    var clip = vec4<f32>(dot(world, rows[0]), dot(world, rows[1]),
        dot(world, rows[2]), dot(world, rows[3]));
    // VS8b0718c3 applies the offset before both alpha and depth interpolation.
    if eye_offset && clip.z < clip.w {
        let offset_w = max(clip.w - material.additive_parms[3].x, clip.z);
        let xy_scale = max(offset_w / clip.w, 0.5);
        clip = vec4<f32>(clip.xy * xy_scale, clip.z, offset_w);
    }
    let alpha = select(input.color.a, clamp(clip.w * material.feather_parms.x, 0.0, 1.0) * input.color.a, feather);
    return AdditiveVaryings(clip, vec4<f32>(input.color.rgb, alpha), vec3<f32>(input.uv, clip.w));
}
@vertex
fn vertex_simple_feather(input: VertexInput) -> AdditiveVaryings {
    return simple_vertex(input, false, true);
}
@vertex
fn vertex_eye_feather(input: VertexInput) -> AdditiveVaryings {
    return simple_vertex(input, true, true);
}
// VS18ca01f6 / VS255edc40 preserve vertex alpha; their PS is the same
// texture*vertex premultiplied additive operation as PS8788956c.
@vertex
fn vertex_simple_additive(input: VertexInput) -> AdditiveVaryings {
    return simple_vertex(input, false, false);
}
@vertex
fn vertex_eye_additive(input: VertexInput) -> AdditiveVaryings {
    return simple_vertex(input, true, false);
}
@vertex
fn vertex_outdoor_feather(input: VertexInput) -> OutdoorVaryings {
    let source = simple_vertex(input, false, true);
    let local = vec4<f32>(input.position.xyz, 1.0);
    let outdoor = vec3<f32>(dot(local, transform.outdoor_rows[0]),
        dot(local, transform.outdoor_rows[1]), dot(local, transform.outdoor_rows[2]));
    return OutdoorVaryings(source.position, source.color, source.uv_depth, outdoor);
}
@fragment
fn fragment_outdoor_feather(input: OutdoorVaryings) -> @location(0) vec4<f32> {
    let uv = input.position.xy * frame.render_target_size.zw;
    let sampled_depth = textureSample(float_z, float_z_sampler, uv).x;
    let depth = frame.z_near.x / max(abs(sampled_depth), 0.0);
    let fade = clamp((depth - input.uv_depth.z) * material.feather_parms.x, 0.0, 1.0);
    let color = textureSample(color_map, color_sampler, input.uv_depth.xy) * input.color;
    let height = textureSample(outdoor_map, outdoor_sampler, input.outdoor.xy).a;
    // 0041f2b0 / 0041e8c0: outdoorFeatherParms.x = r_outdoorFeather = 8.
    let outdoor_fade = clamp((input.outdoor.z - height) * 8.0, 0.0, 1.0);
    let alpha = outdoor_fade * (fade * color.a);
    return vec4<f32>(alpha * color.rgb, alpha);
}

// effect_737e1je4: VS618fd291 / PSa0389359, object-local trail normals.
@vertex
fn vertex_local_additive_feather(input: AdditiveInput) -> AdditiveVaryings {
    return additive_vertex(input, true, true);
}

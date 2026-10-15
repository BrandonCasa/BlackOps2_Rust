import package::rendering::native_fx::{
    NativeFxFrame, NativeFxObject, NativeFxVertexInput, NativeFxVaryings,
    native_fx_vertex, native_fx_fragment,
};

struct NativeEffectTransform { world_rows: array<vec4<f32>, 4>, }
struct NativeEffectConstants { feather_parms: vec4<f32>, }

@group(0) @binding(0) var<uniform> frame: NativeFxFrame;
@group(0) @binding(1) var float_z: texture_2d<f32>;
@group(0) @binding(2) var float_z_sampler: sampler;
@group(1) @binding(0) var<uniform> transform: NativeEffectTransform;
@group(2) @binding(0) var color_map: texture_2d<f32>;
@group(2) @binding(1) var color_sampler: sampler;
@group(2) @binding(2) var<uniform> material: NativeEffectConstants;

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

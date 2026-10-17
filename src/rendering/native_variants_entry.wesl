import package::rendering::native_world::sample_material;
import package::rendering::native_world::{
    NativeWorldScene, NativeWorldObject, NativeWorldVertexInput,
};
import package::rendering::native_world_layers::{
    NativeWorldLayerVaryings, NativeWorldRevealMaterial, native_world_layer_vertex, native_world_layer_reveal_weight,
};
import package::rendering::native_world_plain::NativeWorldGeometricVertexInput;
import package::rendering::native_world_three_layers::{
    NativeWorldThreeLayerVaryings, NativeWorldThreeLayerRevealMaterial,
    NativeWorldThreeLayerDoubleRevealMaterial, native_world_three_layer_vertex,
    native_world_color_overlays_fragment, native_world_three_layer_retained_specular_fragment, native_world_three_normals_fragment, native_world_layer_add_fragment, native_world_reveal_overlay_fragment, native_world_color_three_layer_specular_fragment, native_world_color_three_layer_normal_fragment,
};
import package::rendering::native_world_variants::{
    NativeWorldDiffuseLayerVaryings, NativeWorldGeometricLayerVaryings,
    native_world_diffuse_layer_vertex, native_world_geometric_layer_vertex,
    native_world_base_specular_layer_fragment,
    native_world_base_specular_reveal_layer_fragment,
    native_world_base_specular_three_layer_reveal_fragment,
    native_world_base_specular_three_layer_double_reveal_fragment,
    native_world_no_base_gloss_reveal_layer_fragment,
    native_world_diffuse_layer_fragment, native_world_diffuse_reveal_layer_fragment,
    native_world_geometric_layer_fragment, native_world_diffuse_layer_lighting, native_world_diffuse_layer_normal,
};

@group(0) @binding(0) var<uniform> scene: NativeWorldScene;
@group(1) @binding(0) var<uniform> object: NativeWorldObject;
@group(2) @binding(0) var color_map: texture_2d<f32>;
@group(2) @binding(1) var color_sampler: sampler;
@group(2) @binding(2) var normal_map: texture_2d<f32>;
@group(2) @binding(3) var normal_sampler: sampler;
@group(2) @binding(4) var lightmap_secondary: texture_2d<f32>;
@group(2) @binding(5) var lightmap_sampler: sampler;
@group(2) @binding(6) var reflection_probe: texture_cube<f32>;
@group(2) @binding(7) var reflection_sampler: sampler;
@group(2) @binding(8) var color_map1: texture_2d<f32>;
@group(2) @binding(9) var color_sampler1: sampler;
@group(2) @binding(10) var normal_map1: texture_2d<f32>;
@group(2) @binding(11) var normal_sampler1: sampler;
@group(2) @binding(12) var specular_map1: texture_2d<f32>;
@group(2) @binding(13) var specular_sampler1: sampler;
@group(2) @binding(14) var color_map2: texture_2d<f32>;
@group(2) @binding(15) var color_sampler2: sampler;
@group(2) @binding(16) var normal_map2: texture_2d<f32>;
@group(2) @binding(17) var normal_sampler2: sampler;
@group(2) @binding(18) var specular_map2: texture_2d<f32>;
@group(2) @binding(19) var specular_sampler2: sampler;
@group(2) @binding(20) var<uniform> reveal2: NativeWorldThreeLayerRevealMaterial;
@group(2) @binding(21) var<uniform> double_reveal: NativeWorldThreeLayerDoubleRevealMaterial;
@group(2) @binding(22) var specular_map: texture_2d<f32>;
@group(2) @binding(23) var specular_sampler: sampler;
@group(2) @binding(24) var<uniform> reveal: NativeWorldRevealMaterial;

struct LayerVertexInput {
    @location(0) position: vec4<f32>,
    @location(1) color: vec4<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) packed_normal: vec4<f32>,
    @location(4) packed_tangent: vec4<f32>,
    @location(5) lightmap_uv: vec2<f32>,
    @location(6) layer_uv: vec2<f32>,
    @location(7) normal_transform: vec4<f32>,
}

struct ThreeLayerVertexInput {
    @location(0) position: vec4<f32>,
    @location(1) color: vec4<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) packed_normal: vec4<f32>,
    @location(4) packed_tangent: vec4<f32>,
    @location(5) lightmap_uv: vec2<f32>,
    @location(6) layer_uvs: vec4<f32>,
    @location(7) normal_transform1: vec4<f32>,
    @location(8) normal_transform2: vec4<f32>,
}

struct GeometricLayerVertexInput {
    @location(0) position: vec3<f32>,
    @location(1) color: vec4<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) packed_normal: vec4<f32>,
    @location(5) lightmap_uv: vec2<f32>,
    @location(6) layer_uv: vec2<f32>,
}

struct LayerVaryings {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv_lightmap: vec4<f32>,
    @location(1) normal_fog: vec4<f32>,
    @location(2) fog_color: vec3<f32>,
    @location(3) bitangent: vec3<f32>,
    @location(4) tangent: vec3<f32>,
    @location(5) eye_to_vertex: vec3<f32>,
    @location(6) blend_uv: vec4<f32>,
    @location(7) normal_transform: vec4<f32>,
    @location(8) reflection_correction: vec3<f32>,
}

struct ThreeLayerVaryings {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) blend_color: vec3<f32>,
    @location(1) uv_lightmap: vec4<f32>,
    @location(2) normal_fog: vec4<f32>,
    @location(3) fog_color: vec3<f32>,
    @location(4) bitangent: vec3<f32>,
    @location(5) tangent: vec3<f32>,
    @location(6) eye_to_vertex: vec3<f32>,
    @location(7) layer_uvs: vec4<f32>,
    @location(8) normal_transform1: vec4<f32>,
    @location(9) normal_transform2: vec4<f32>,
    @location(10) reflection_correction: vec3<f32>,
}

struct DiffuseLayerVaryings {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv_lightmap: vec4<f32>,
    @location(1) normal_fog: vec4<f32>,
    @location(2) fog_color: vec3<f32>,
    @location(3) bitangent: vec3<f32>,
    @location(4) tangent: vec3<f32>,
    @location(5) blend_uv: vec4<f32>,
    @location(6) normal_transform: vec4<f32>,
}

struct GeometricLayerVaryings {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv_lightmap: vec4<f32>,
    @location(1) normal_fog: vec4<f32>,
    @location(2) fog_color: vec3<f32>,
    @location(3) eye_to_vertex: vec3<f32>,
    @location(4) blend_uv: vec4<f32>,
    @location(5) reflection_correction: vec3<f32>,
}

fn primary(input: LayerVertexInput) -> NativeWorldVertexInput {
    return NativeWorldVertexInput(input.position, input.color, input.uv,
        input.packed_normal.xyz, input.packed_tangent.xyz, input.lightmap_uv);
}

@vertex
fn vertex_layer(input: LayerVertexInput) -> LayerVaryings {
    let result = native_world_layer_vertex(primary(input), input.layer_uv,
        input.normal_transform, scene, object);
    return LayerVaryings(result.clip_position, result.uv_lightmap, result.normal_fog,
        result.fog_color, result.bitangent, result.tangent, result.eye_to_vertex,
        result.blend_uv, result.normal_transform, result.reflection_correction);
}

@vertex
fn vertex_three(input: ThreeLayerVertexInput) -> ThreeLayerVaryings {
    let world_input = NativeWorldVertexInput(input.position, input.color, input.uv,
        input.packed_normal.xyz, input.packed_tangent.xyz, input.lightmap_uv);
    let result = native_world_three_layer_vertex(world_input, input.layer_uvs,
        input.normal_transform1, input.normal_transform2, scene, object);
    return ThreeLayerVaryings(result.clip_position, result.blend_color, result.uv_lightmap,
        result.normal_fog, result.fog_color, result.bitangent, result.tangent,
        result.eye_to_vertex, result.layer_uvs, result.normal_transform1,
        result.normal_transform2, result.reflection_correction);
}

@vertex
fn vertex_diffuse_layer(input: LayerVertexInput) -> DiffuseLayerVaryings {
    let result = native_world_diffuse_layer_vertex(primary(input), input.layer_uv,
        input.normal_transform, scene, object);
    return DiffuseLayerVaryings(result.clip_position, result.uv_lightmap, result.normal_fog,
        result.fog_color, result.bitangent, result.tangent, result.blend_uv,
        result.normal_transform);
}

@vertex
fn vertex_geometric_layer(input: GeometricLayerVertexInput) -> GeometricLayerVaryings {
    let world_input = NativeWorldGeometricVertexInput(input.position, input.color,
        input.uv, input.packed_normal.xyz, input.lightmap_uv);
    let result = native_world_geometric_layer_vertex(world_input, input.layer_uv, scene, object);
    return GeometricLayerVaryings(result.clip_position, result.uv_lightmap, result.normal_fog,
        result.fog_color, result.eye_to_vertex, result.blend_uv, result.reflection_correction);
}

fn source_layer(input: LayerVaryings) -> NativeWorldLayerVaryings {
    return NativeWorldLayerVaryings(input.clip_position, input.uv_lightmap,
        input.normal_fog, input.fog_color, input.bitangent, input.tangent,
        input.eye_to_vertex, input.blend_uv, input.normal_transform, input.reflection_correction);
}

fn source_three(input: ThreeLayerVaryings) -> NativeWorldThreeLayerVaryings {
    return NativeWorldThreeLayerVaryings(input.clip_position, input.blend_color,
        input.uv_lightmap, input.normal_fog, input.fog_color, input.bitangent,
        input.tangent, input.eye_to_vertex, input.layer_uvs, input.normal_transform1,
        input.normal_transform2, input.reflection_correction);
}

fn source_diffuse(input: DiffuseLayerVaryings) -> NativeWorldDiffuseLayerVaryings {
    return NativeWorldDiffuseLayerVaryings(input.clip_position, input.uv_lightmap,
        input.normal_fog, input.fog_color, input.bitangent, input.tangent,
        input.blend_uv, input.normal_transform);
}

fn source_geometric(input: GeometricLayerVaryings) -> NativeWorldGeometricLayerVaryings {
    return NativeWorldGeometricLayerVaryings(input.clip_position, input.uv_lightmap,
        input.normal_fog, input.fog_color, input.eye_to_vertex, input.blend_uv,
        input.reflection_correction);
}

fn shade_base_specular_layer(input: LayerVaryings) -> vec4<f32> {
    return native_world_base_specular_layer_fragment(source_layer(input), scene,
        color_map, color_sampler, normal_map, normal_sampler, specular_map, specular_sampler,
        color_map1, color_sampler1, normal_map1, normal_sampler1,
        lightmap_secondary, lightmap_sampler, reflection_probe, reflection_sampler);
}

fn shade_base_specular_reveal_layer(input: LayerVaryings) -> vec4<f32> {
    return native_world_base_specular_reveal_layer_fragment(source_layer(input), scene, reveal,
        color_map, color_sampler, normal_map, normal_sampler, specular_map, specular_sampler,
        color_map1, color_sampler1, normal_map1, normal_sampler1, specular_map1, specular_sampler1,
        lightmap_secondary, lightmap_sampler, reflection_probe, reflection_sampler);
}

fn shade_base_specular_three_layer_reveal(input: ThreeLayerVaryings) -> vec4<f32> {
    return native_world_base_specular_three_layer_reveal_fragment(source_three(input), scene, reveal2,
        color_map, color_sampler, normal_map, normal_sampler, specular_map, specular_sampler,
        color_map1, color_sampler1, normal_map1, normal_sampler1,
        color_map2, color_sampler2, normal_map2, normal_sampler2, specular_map2, specular_sampler2,
        lightmap_secondary, lightmap_sampler, reflection_probe, reflection_sampler);
}

fn shade_base_specular_three_layer_double_reveal(input: ThreeLayerVaryings) -> vec4<f32> {
    return native_world_base_specular_three_layer_double_reveal_fragment(source_three(input), scene, double_reveal,
        color_map, color_sampler, normal_map, normal_sampler, specular_map, specular_sampler,
        color_map1, color_sampler1, normal_map1, normal_sampler1, specular_map1, specular_sampler1,
        color_map2, color_sampler2, normal_map2, normal_sampler2, specular_map2, specular_sampler2,
        lightmap_secondary, lightmap_sampler, reflection_probe, reflection_sampler);
}

fn shade_no_base_gloss_reveal_layer(input: LayerVaryings) -> vec4<f32> {
    return native_world_no_base_gloss_reveal_layer_fragment(source_layer(input), scene, reveal,
        color_map, color_sampler, normal_map, normal_sampler,
        color_map1, color_sampler1, normal_map1, normal_sampler1, specular_map1, specular_sampler1,
        lightmap_secondary, lightmap_sampler, reflection_probe, reflection_sampler);
}

fn shade_diffuse_layer(input: DiffuseLayerVaryings) -> vec4<f32> {
    return native_world_diffuse_layer_fragment(source_diffuse(input), scene,
        color_map, color_sampler, normal_map, normal_sampler,
        color_map1, color_sampler1, normal_map1, normal_sampler1,
        lightmap_secondary, lightmap_sampler);
}

fn shade_diffuse_reveal_layer(input: DiffuseLayerVaryings) -> vec4<f32> {
    return native_world_diffuse_reveal_layer_fragment(source_diffuse(input), scene, reveal,
        color_map, color_sampler, normal_map, normal_sampler,
        color_map1, color_sampler1, normal_map1, normal_sampler1,
        lightmap_secondary, lightmap_sampler);
}

fn shade_geometric_layer(input: GeometricLayerVaryings) -> vec4<f32> {
    return native_world_geometric_layer_fragment(source_geometric(input), scene,
        color_map, color_sampler, specular_map, specular_sampler, color_map1, color_sampler1,
        lightmap_secondary, lightmap_sampler, reflection_probe, reflection_sampler);
}

// Source RGB is already encoded. Cancel only an sRGB attachment's transfer.
fn attachment_srgb(encoded: vec4<f32>) -> vec4<f32> {
    let linear = select(encoded.rgb / vec3<f32>(12.92),
        pow((encoded.rgb + vec3<f32>(0.055)) / vec3<f32>(1.055), vec3<f32>(2.4)),
        encoded.rgb > vec3<f32>(0.04045));
    return vec4<f32>(linear, encoded.a);
}

@fragment
fn fragment_base_specular_layer(input: LayerVaryings) -> @location(0) vec4<f32> {
    return shade_base_specular_layer(input);
}
@fragment
fn fragment_base_specular_layer_srgb(input: LayerVaryings) -> @location(0) vec4<f32> {
    return attachment_srgb(shade_base_specular_layer(input));
}
@fragment
fn fragment_base_specular_reveal_layer(input: LayerVaryings) -> @location(0) vec4<f32> {
    return shade_base_specular_reveal_layer(input);
}
@fragment
fn fragment_base_specular_reveal_layer_srgb(input: LayerVaryings) -> @location(0) vec4<f32> {
    return attachment_srgb(shade_base_specular_reveal_layer(input));
}
@fragment
fn fragment_base_specular_three_layer_reveal(input: ThreeLayerVaryings) -> @location(0) vec4<f32> {
    return shade_base_specular_three_layer_reveal(input);
}
@fragment
fn fragment_base_specular_three_layer_reveal_srgb(input: ThreeLayerVaryings) -> @location(0) vec4<f32> {
    return attachment_srgb(shade_base_specular_three_layer_reveal(input));
}
@fragment
fn fragment_base_specular_three_layer_double_reveal(input: ThreeLayerVaryings) -> @location(0) vec4<f32> {
    return shade_base_specular_three_layer_double_reveal(input);
}
@fragment
fn fragment_base_specular_three_layer_double_reveal_srgb(input: ThreeLayerVaryings) -> @location(0) vec4<f32> {
    return attachment_srgb(shade_base_specular_three_layer_double_reveal(input));
}
@fragment
fn fragment_no_base_gloss_reveal_layer(input: LayerVaryings) -> @location(0) vec4<f32> {
    return shade_no_base_gloss_reveal_layer(input);
}
@fragment
fn fragment_no_base_gloss_reveal_layer_srgb(input: LayerVaryings) -> @location(0) vec4<f32> {
    return attachment_srgb(shade_no_base_gloss_reveal_layer(input));
}
@fragment
fn fragment_diffuse_layer(input: DiffuseLayerVaryings) -> @location(0) vec4<f32> {
    return shade_diffuse_layer(input);
}
@fragment
fn fragment_diffuse_layer_srgb(input: DiffuseLayerVaryings) -> @location(0) vec4<f32> {
    return attachment_srgb(shade_diffuse_layer(input));
}
@fragment
fn fragment_diffuse_reveal_layer(input: DiffuseLayerVaryings) -> @location(0) vec4<f32> {
    return shade_diffuse_reveal_layer(input);
}
@fragment
fn fragment_diffuse_reveal_layer_srgb(input: DiffuseLayerVaryings) -> @location(0) vec4<f32> {
    return attachment_srgb(shade_diffuse_reveal_layer(input));
}
@fragment
fn fragment_geometric_layer(input: GeometricLayerVaryings) -> @location(0) vec4<f32> {
    return shade_geometric_layer(input);
}
@fragment
fn fragment_geometric_layer_srgb(input: GeometricLayerVaryings) -> @location(0) vec4<f32> {
    return attachment_srgb(shade_geometric_layer(input));
}


// VSf26e80c6 / VSe2f44d32: one or two auxiliary half-float UV pairs.
struct DiffuseColorInput {
    @location(0) position: vec4<f32>,
    @location(1) color: vec4<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) packed_normal: vec4<f32>,
    @location(4) packed_tangent: vec4<f32>,
    @location(5) lightmap_uv: vec2<f32>,
    @location(6) layer_uv: vec2<f32>,
}
struct DiffuseDoubleColorInput {
    @location(0) position: vec4<f32>,
    @location(1) color: vec4<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) packed_normal: vec4<f32>,
    @location(4) packed_tangent: vec4<f32>,
    @location(5) lightmap_uv: vec2<f32>,
    @location(6) layer_uvs: vec4<f32>,
}
struct DiffuseColorVaryings {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv_lightmap: vec4<f32>,
    @location(1) normal_fog: vec4<f32>,
    @location(2) fog_color: vec3<f32>,
    @location(3) bitangent: vec3<f32>,
    @location(4) tangent: vec3<f32>,
    @location(5) weights: vec2<f32>,
    @location(6) layer_uvs: vec4<f32>,
}
fn diffuse_color_vertex(source: NativeWorldVertexInput, uvs: vec4<f32>) -> DiffuseColorVaryings {
    let result = native_world_diffuse_layer_vertex(source, uvs.xy, vec4<f32>(0.0), scene, object);
    return DiffuseColorVaryings(result.clip_position, result.uv_lightmap, result.normal_fog,
        result.fog_color, result.bitangent, result.tangent, source.color.gb, uvs);
}
@vertex
fn vertex_diffuse_color(input: DiffuseColorInput) -> DiffuseColorVaryings {
    let source = NativeWorldVertexInput(input.position, input.color, input.uv,
        input.packed_normal.xyz, input.packed_tangent.xyz, input.lightmap_uv);
    return diffuse_color_vertex(source, vec4<f32>(input.layer_uv, vec2<f32>(0.0)));
}
@vertex
fn vertex_diffuse_double_color(input: DiffuseDoubleColorInput) -> DiffuseColorVaryings {
    let source = NativeWorldVertexInput(input.position, input.color, input.uv,
        input.packed_normal.xyz, input.packed_tangent.xyz, input.lightmap_uv);
    return diffuse_color_vertex(source, input.layer_uvs);
}
fn diffuse_color_lighting(input: DiffuseColorVaryings, color: vec3<f32>) -> vec4<f32> {
    let sampled_normal = sample_material(scene.texture_mip_scale, normal_map, normal_sampler, input.uv_lightmap.xy);
    let slope = fma(sampled_normal.rg, vec2<f32>(4.015748023986816), vec2<f32>(-2.0157480239868164));
    let mapped = fma(vec3<f32>(slope.y), input.bitangent,
        fma(vec3<f32>(slope.x), input.tangent, input.normal_fog.xyz));
    let normal = inverseSqrt(dot(mapped, mapped)) * mapped;
    let source = NativeWorldDiffuseLayerVaryings(input.clip_position, input.uv_lightmap,
        input.normal_fog, input.fog_color, input.bitangent, input.tangent, vec4<f32>(0.0), vec4<f32>(0.0));
    return native_world_diffuse_layer_lighting(source, scene, normal,
        color, lightmap_secondary, lightmap_sampler);
}
// PS7982c7aa: C0.rgb * lerp(1, C1.rgb, vertex.g), with diffuse light only.
// Neither color alpha participates; no probe, normalMap1, or reveal constants.
fn diffuse_multiply_color(input: DiffuseColorVaryings) -> vec3<f32> {
    let layer = sample_material(scene.texture_mip_scale, color_map1, color_sampler1, input.layer_uvs.xy);
    let multiplier = fma(vec3<f32>(input.weights.x), layer.rgb - vec3<f32>(1.0), vec3<f32>(1.0));
    let base = sample_material(scene.texture_mip_scale, color_map, color_sampler, input.uv_lightmap.xy);
    return multiplier * base.rgb;
}
fn shade_diffuse_multiply(input: DiffuseColorVaryings) -> vec4<f32> {
    return diffuse_color_lighting(input, diffuse_multiply_color(input));
}
// PS61a9e8fe applies the second multiplier using vertex.b before squaring RGB.
fn shade_diffuse_double_multiply(input: DiffuseColorVaryings) -> vec4<f32> {
    let color = diffuse_multiply_color(input);
    let layer = sample_material(scene.texture_mip_scale, color_map2, color_sampler2, input.layer_uvs.zw);
    let multiplier = fma(vec3<f32>(input.weights.y), layer.rgb - vec3<f32>(1.0), vec3<f32>(1.0));
    return diffuse_color_lighting(input, color * multiplier);
}
@fragment
fn fragment_diffuse_multiply(input: DiffuseColorVaryings) -> @location(0) vec4<f32> { return shade_diffuse_multiply(input); }
@fragment
fn fragment_diffuse_multiply_srgb(input: DiffuseColorVaryings) -> @location(0) vec4<f32> { return attachment_srgb(shade_diffuse_multiply(input)); }
@fragment
fn fragment_diffuse_double_multiply(input: DiffuseColorVaryings) -> @location(0) vec4<f32> { return shade_diffuse_double_multiply(input); }
@fragment
fn fragment_diffuse_double_multiply_srgb(input: DiffuseColorVaryings) -> @location(0) vec4<f32> { return attachment_srgb(shade_diffuse_double_multiply(input)); }

// VS3250349a PACKED_WORLD_T2N1: UV1/UV2 at0, only Q2 at8, stride12.
struct ColorThreeInput {
    @location(0) position: vec4<f32>,
    @location(1) color: vec4<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) packed_normal: vec4<f32>,
    @location(4) packed_tangent: vec4<f32>,
    @location(5) lightmap_uv: vec2<f32>,
    @location(6) layer_uvs: vec4<f32>,
    @location(7) normal_transform2: vec4<f32>,
}
@vertex
fn vertex_color_three(input: ColorThreeInput) -> ThreeLayerVaryings {
    let source = NativeWorldVertexInput(input.position, input.color, input.uv,
        input.packed_normal.xyz, input.packed_tangent.xyz, input.lightmap_uv);
    let result = native_world_three_layer_vertex(source, input.layer_uvs,
        vec4<f32>(0.0), input.normal_transform2, scene, object);
    return ThreeLayerVaryings(result.clip_position, result.blend_color, result.uv_lightmap,
        result.normal_fog, result.fog_color, result.bitangent, result.tangent,
        result.eye_to_vertex, result.layer_uvs, result.normal_transform1,
        result.normal_transform2, result.reflection_correction);
}
fn shade_color_three(input: ThreeLayerVaryings, layer_color2: vec4<f32>, weight2: f32) -> vec4<f32> {
    return native_world_color_three_layer_specular_fragment(source_three(input), scene, layer_color2, weight2,
        color_map, color_sampler, normal_map, normal_sampler, color_map1, color_sampler1,
        normal_map2, normal_sampler2, specular_map2, specular_sampler2,
        lightmap_secondary, lightmap_sampler, reflection_probe, reflection_sampler);
}
fn shade_color_three_reveal(input: ThreeLayerVaryings) -> vec4<f32> {
    let layer = sample_material(scene.texture_mip_scale, color_map2, color_sampler2, input.layer_uvs.zw);
    let weight = native_world_layer_reveal_weight(input.blend_color.b, layer.a,
        NativeWorldRevealMaterial(reveal2.alpha_reveal_parms2));
    return shade_color_three(input, layer, weight);
}
// PSa3337049 uses ordinary C2.a * vertex.b and binds no reveal uniform.
fn shade_color_three_specular(input: ThreeLayerVaryings) -> vec4<f32> {
    let layer = sample_material(scene.texture_mip_scale, color_map2, color_sampler2, input.layer_uvs.zw);
    return shade_color_three(input, layer, layer.a * input.blend_color.b);
}
@fragment
fn fragment_color_three_reveal(input: ThreeLayerVaryings) -> @location(0) vec4<f32> { return shade_color_three_reveal(input); }
@fragment
fn fragment_color_three_reveal_srgb(input: ThreeLayerVaryings) -> @location(0) vec4<f32> { return attachment_srgb(shade_color_three_reveal(input)); }

@fragment
fn fragment_color_three_specular(input: ThreeLayerVaryings) -> @location(0) vec4<f32> { return shade_color_three_specular(input); }
@fragment
fn fragment_color_three_specular_srgb(input: ThreeLayerVaryings) -> @location(0) vec4<f32> { return attachment_srgb(shade_color_three_specular(input)); }

fn shade_color_three_normal(input: ThreeLayerVaryings) -> vec4<f32> {
    return native_world_color_three_layer_normal_fragment(source_three(input), scene,
        color_map, color_sampler, normal_map, normal_sampler, color_map1, color_sampler1,
        color_map2, color_sampler2, normal_map2, normal_sampler2,
        lightmap_secondary, lightmap_sampler, reflection_probe, reflection_sampler);
}
@fragment
fn fragment_color_three_normal(input: ThreeLayerVaryings) -> @location(0) vec4<f32> { return shade_color_three_normal(input); }
@fragment
fn fragment_color_three_normal_srgb(input: ThreeLayerVaryings) -> @location(0) vec4<f32> { return attachment_srgb(shade_color_three_normal(input)); }

fn shade_reveal_overlay(input: ThreeLayerVaryings, additive: bool) -> vec4<f32> {
    return native_world_reveal_overlay_fragment(source_three(input), scene, reveal, additive, vec4<f32>(1.0),
        color_map, color_sampler, normal_map, normal_sampler, color_map1, color_sampler1,
        normal_map1, normal_sampler1, specular_map1, specular_sampler1, color_map2, color_sampler2,
        lightmap_secondary, lightmap_sampler, reflection_probe, reflection_sampler);
}
@fragment
fn fragment_reveal_multiply(input: ThreeLayerVaryings) -> @location(0) vec4<f32> { return shade_reveal_overlay(input, false); }
@fragment
fn fragment_reveal_multiply_srgb(input: ThreeLayerVaryings) -> @location(0) vec4<f32> { return attachment_srgb(shade_reveal_overlay(input, false)); }

@fragment
fn fragment_reveal_add(input: ThreeLayerVaryings) -> @location(0) vec4<f32> { return shade_reveal_overlay(input, true); }
@fragment
fn fragment_reveal_add_srgb(input: ThreeLayerVaryings) -> @location(0) vec4<f32> { return attachment_srgb(shade_reveal_overlay(input, true)); }

// VSb0b22809: T2N1 without SH/probe work. Q at byte8 transforms layer1.
@vertex
fn vertex_diffuse_three(input: ColorThreeInput) -> ThreeLayerVaryings {
    let primary = NativeWorldVertexInput(input.position, input.color, input.uv,
        input.packed_normal.xyz, input.packed_tangent.xyz, input.lightmap_uv);
    let result = native_world_diffuse_layer_vertex(primary, input.layer_uvs.xy,
        input.normal_transform2, scene, object);
    return ThreeLayerVaryings(result.clip_position, input.color.rgb, result.uv_lightmap,
        result.normal_fog, result.fog_color, result.bitangent, result.tangent,
        vec3<f32>(0.0), input.layer_uvs, vec4<f32>(0.0), input.normal_transform2, vec3<f32>(0.0));
}
// PS89d59c60/5c9a2e1f share normal blending and the complete diffuse multiply tail.
fn shade_diffuse_layer_multiply(input: ThreeLayerVaryings, layer: vec4<f32>, weight: f32) -> vec4<f32> {
    let source = NativeWorldDiffuseLayerVaryings(input.clip_position, input.uv_lightmap,
        input.normal_fog, input.fog_color, input.bitangent, input.tangent,
        vec4<f32>(input.blend_color.rg, input.layer_uvs.xy), input.normal_transform2);
    let base_normal = sample_material(scene.texture_mip_scale, normal_map, normal_sampler, input.uv_lightmap.xy);
    let layer_normal = sample_material(scene.texture_mip_scale, normal_map1, normal_sampler1, input.layer_uvs.xy);
    let normal = native_world_diffuse_layer_normal(source, base_normal.rg, layer_normal.rg, weight);
    let base = sample_material(scene.texture_mip_scale, color_map, color_sampler, input.uv_lightmap.xy);
    let color = fma(vec3<f32>(weight), layer.rgb - base.rgb, base.rgb);
    let overlay = sample_material(scene.texture_mip_scale, color_map2, color_sampler2, input.layer_uvs.zw);
    let multiplier = fma(vec3<f32>(input.blend_color.b), overlay.rgb - vec3<f32>(1.0), vec3<f32>(1.0));
    return native_world_diffuse_layer_lighting(source, scene, normal, color * multiplier, lightmap_secondary, lightmap_sampler);
}
fn diffuse_layer_multiply(input: ThreeLayerVaryings) -> vec4<f32> {
    let layer = sample_material(scene.texture_mip_scale, color_map1, color_sampler1, input.layer_uvs.xy);
    return shade_diffuse_layer_multiply(input, layer, layer.a * input.blend_color.g);
}
fn diffuse_reveal_multiply(input: ThreeLayerVaryings) -> vec4<f32> {
    let layer = sample_material(scene.texture_mip_scale, color_map1, color_sampler1, input.layer_uvs.xy);
    let weight = native_world_layer_reveal_weight(input.blend_color.g, layer.a, reveal);
    return shade_diffuse_layer_multiply(input, layer, weight);
}
@fragment
fn fragment_diffuse_layer_multiply(input: ThreeLayerVaryings) -> @location(0) vec4<f32> { return diffuse_layer_multiply(input); }
@fragment
fn fragment_diffuse_layer_multiply_srgb(input: ThreeLayerVaryings) -> @location(0) vec4<f32> { return attachment_srgb(diffuse_layer_multiply(input)); }
@fragment
fn fragment_diffuse_reveal_multiply(input: ThreeLayerVaryings) -> @location(0) vec4<f32> { return diffuse_reveal_multiply(input); }
@fragment
fn fragment_diffuse_reveal_multiply_srgb(input: ThreeLayerVaryings) -> @location(0) vec4<f32> { return attachment_srgb(diffuse_reveal_multiply(input)); }

fn shade_layer_add(input: ThreeLayerVaryings) -> vec4<f32> {
    return native_world_layer_add_fragment(source_three(input), scene,
        color_map, color_sampler, normal_map, normal_sampler, color_map1, color_sampler1,
        normal_map1, normal_sampler1, color_map2, color_sampler2,
        lightmap_secondary, lightmap_sampler, reflection_probe, reflection_sampler);
}
@fragment
fn fragment_layer_add(input: ThreeLayerVaryings) -> @location(0) vec4<f32> { return shade_layer_add(input); }
@fragment
fn fragment_layer_add_srgb(input: ThreeLayerVaryings) -> @location(0) vec4<f32> { return attachment_srgb(shade_layer_add(input)); }

fn shade_three_normals(input: ThreeLayerVaryings) -> vec4<f32> {
    return native_world_three_normals_fragment(source_three(input), scene,
        color_map, color_sampler, normal_map, normal_sampler, color_map1, color_sampler1,
        normal_map1, normal_sampler1, color_map2, color_sampler2, normal_map2, normal_sampler2,
        lightmap_secondary, lightmap_sampler, reflection_probe, reflection_sampler);
}
@fragment
fn fragment_three_normals(input: ThreeLayerVaryings) -> @location(0) vec4<f32> { return shade_three_normals(input); }
@fragment
fn fragment_three_normals_srgb(input: ThreeLayerVaryings) -> @location(0) vec4<f32> { return attachment_srgb(shade_three_normals(input)); }

fn shade_retained_specular(input: ThreeLayerVaryings) -> vec4<f32> {
    return native_world_three_layer_retained_specular_fragment(source_three(input), scene, double_reveal,
        color_map, color_sampler, normal_map, normal_sampler, color_map1, color_sampler1,
        normal_map1, normal_sampler1, specular_map1, specular_sampler1,
        color_map2, color_sampler2, normal_map2, normal_sampler2,
        lightmap_secondary, lightmap_sampler, reflection_probe, reflection_sampler);
}
@fragment
fn fragment_retained_specular(input: ThreeLayerVaryings) -> @location(0) vec4<f32> { return shade_retained_specular(input); }
@fragment
fn fragment_retained_specular_srgb(input: ThreeLayerVaryings) -> @location(0) vec4<f32> { return attachment_srgb(shade_retained_specular(input)); }

// VS8b6985a0: T2 auxiliary stream is two UV pairs, without slope transforms.
@vertex
fn vertex_color_overlays(input: DiffuseDoubleColorInput) -> ThreeLayerVaryings {
    let primary = NativeWorldVertexInput(input.position, input.color, input.uv,
        input.packed_normal.xyz, input.packed_tangent.xyz, input.lightmap_uv);
    let result = native_world_three_layer_vertex(primary, input.layer_uvs,
        vec4<f32>(0.0), vec4<f32>(0.0), scene, object);
    return ThreeLayerVaryings(result.clip_position, result.blend_color, result.uv_lightmap,
        result.normal_fog, result.fog_color, result.bitangent, result.tangent,
        result.eye_to_vertex, result.layer_uvs, result.normal_transform1,
        result.normal_transform2, result.reflection_correction);
}
fn shade_color_overlays(input: ThreeLayerVaryings, add_multiply: bool) -> vec4<f32> {
    return native_world_color_overlays_fragment(source_three(input), scene, add_multiply,
        color_map, color_sampler, normal_map, normal_sampler, color_map1, color_sampler1,
        color_map2, color_sampler2, lightmap_secondary, lightmap_sampler, reflection_probe, reflection_sampler);
}
@fragment
fn fragment_color_double_blend(input: ThreeLayerVaryings) -> @location(0) vec4<f32> { return shade_color_overlays(input, false); }
@fragment
fn fragment_color_double_blend_srgb(input: ThreeLayerVaryings) -> @location(0) vec4<f32> { return attachment_srgb(shade_color_overlays(input, false)); }
@fragment
fn fragment_color_add_multiply(input: ThreeLayerVaryings) -> @location(0) vec4<f32> { return shade_color_overlays(input, true); }
@fragment
fn fragment_color_add_multiply_srgb(input: ThreeLayerVaryings) -> @location(0) vec4<f32> { return attachment_srgb(shade_color_overlays(input, true)); }

@group(2) @binding(26) var color_map3: texture_2d<f32>;
@group(2) @binding(27) var color_sampler3: sampler;
// VSaaeb8157: T3N1 auxiliary UV1/2 at0, UV3 at8, Q1 at12, stride16.
struct FourLayerInput {
    @location(0) position: vec4<f32>,
    @location(1) color: vec4<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) packed_normal: vec4<f32>,
    @location(4) packed_tangent: vec4<f32>,
    @location(5) lightmap_uv: vec2<f32>,
    @location(6) layer_uvs: vec4<f32>,
    @location(7) uv3: vec2<f32>,
    @location(8) normal_transform: vec4<f32>,
}
struct FourLayerVaryings {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) blend_color: vec3<f32>,
    @location(1) uv_lightmap: vec4<f32>,
    @location(2) normal_fog: vec4<f32>,
    @location(3) fog_color: vec3<f32>,
    @location(4) bitangent: vec3<f32>,
    @location(5) tangent: vec3<f32>,
    @location(6) eye_to_vertex: vec3<f32>,
    @location(7) layer_uvs: vec4<f32>,
    @location(8) normal_transform1: vec4<f32>,
    @location(9) normal_transform2: vec4<f32>,
    @location(10) reflection_correction: vec3<f32>,
    @location(11) uv3_weight: vec3<f32>,
}
@vertex
fn vertex_four_layer(input: FourLayerInput) -> FourLayerVaryings {
    let primary = NativeWorldVertexInput(input.position, input.color, input.uv,
        input.packed_normal.xyz, input.packed_tangent.xyz, input.lightmap_uv);
    let result = native_world_three_layer_vertex(primary, input.layer_uvs,
        vec4<f32>(0.0), input.normal_transform, scene, object);
    return FourLayerVaryings(result.clip_position, result.blend_color, result.uv_lightmap,
        result.normal_fog, result.fog_color, result.bitangent, result.tangent,
        result.eye_to_vertex, result.layer_uvs, result.normal_transform1,
        result.normal_transform2, result.reflection_correction, vec3<f32>(input.uv3, input.color.a));
}
// PSb81c762c: vertex alpha controls final multiply, after the additive overlay.
fn shade_four_layer(input: FourLayerVaryings) -> vec4<f32> {
    let source = NativeWorldThreeLayerVaryings(input.clip_position, input.blend_color,
        input.uv_lightmap, input.normal_fog, input.fog_color, input.bitangent,
        input.tangent, input.eye_to_vertex, input.layer_uvs, input.normal_transform1,
        input.normal_transform2, input.reflection_correction);
    let layer3 = sample_material(scene.texture_mip_scale, color_map3, color_sampler3, input.uv3_weight.xy);
    let multiplier = fma(vec3<f32>(input.uv3_weight.z), layer3.rgb - vec3<f32>(1.0), vec3<f32>(1.0));
    let gloss_scale = dot(multiplier, vec3<f32>(0.30000001192092896, 0.6000000238418579, 0.10000000149011612));
    return native_world_reveal_overlay_fragment(source, scene, reveal, true, vec4<f32>(multiplier, gloss_scale),
        color_map, color_sampler, normal_map, normal_sampler, color_map1, color_sampler1,
        normal_map1, normal_sampler1, specular_map1, specular_sampler1, color_map2, color_sampler2,
        lightmap_secondary, lightmap_sampler, reflection_probe, reflection_sampler);
}
@fragment
fn fragment_four_layer(input: FourLayerVaryings) -> @location(0) vec4<f32> { return shade_four_layer(input); }
@fragment
fn fragment_four_layer_srgb(input: FourLayerVaryings) -> @location(0) vec4<f32> { return attachment_srgb(shade_four_layer(input)); }

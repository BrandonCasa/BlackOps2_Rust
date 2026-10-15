import package::rendering::native_world_layers::{NativeWorldRevealMaterial, native_world_layer_reveal_weight};
import package::rendering::native_world::{
    NativeWorldScene, NativeWorldObject, NativeWorldVertexInput,
    NativeWorldVaryings, native_world_vertex,
};
import package::rendering::native_world_plain::{
    NativeWorldGeometricVertexInput, native_world_plain_base_vertex,
    native_world_diffuse_vertex, native_world_diffuse_fragment,
    native_world_normal_specular_fragment, native_world_geometric_gloss_fragment,
    native_world_geometric_specular_vertex, native_world_geometric_specular_fragment,
};

@group(2) @binding(24) var<uniform> reveal_material: NativeWorldRevealMaterial;

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
@group(2) @binding(8) var specular_map: texture_2d<f32>;
@group(2) @binding(9) var specular_sampler: sampler;

struct VertexInput {
    @location(0) position: vec4<f32>,
    @location(1) color: vec4<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) packed_normal: vec4<f32>,
    @location(4) packed_tangent: vec4<f32>,
    @location(5) lightmap_uv: vec2<f32>,
}

struct GeometricVertexInput {
    @location(0) position: vec3<f32>,
    @location(1) color: vec4<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) packed_normal: vec4<f32>,
    @location(5) lightmap_uv: vec2<f32>,
}

struct DiffuseVaryings {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) uv_lightmap: vec4<f32>,
    @location(2) normal_fog: vec4<f32>,
    @location(3) fog_color: vec3<f32>,
    @location(4) bitangent: vec3<f32>,
    @location(5) tangent: vec3<f32>,
    @location(6) eye_to_vertex: vec3<f32>,
}

struct SpecularVaryings {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) uv_lightmap: vec4<f32>,
    @location(2) normal_fog: vec4<f32>,
    @location(3) fog_color: vec3<f32>,
    @location(4) bitangent: vec3<f32>,
    @location(5) tangent: vec3<f32>,
    @location(6) eye_to_vertex: vec3<f32>,
    @location(7) reflection_correction: vec3<f32>,
}

struct GeometricSpecularVaryings {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) uv_lightmap: vec4<f32>,
    @location(2) normal_fog: vec4<f32>,
    @location(3) fog_color: vec3<f32>,
    @location(6) eye_to_vertex: vec3<f32>,
    @location(7) reflection_correction: vec3<f32>,
}

fn primary(input: VertexInput) -> NativeWorldVertexInput {
    return NativeWorldVertexInput(input.position, input.color, input.uv,
        input.packed_normal.xyz, input.packed_tangent.xyz, input.lightmap_uv);
}

@vertex
fn vertex_diffuse(input: VertexInput) -> DiffuseVaryings {
    let result = native_world_diffuse_vertex(primary(input), scene, object);
    return DiffuseVaryings(result.clip_position, result.color, result.uv_lightmap,
        result.normal_fog, result.fog_color, result.bitangent, result.tangent,
        result.eye_to_vertex);
}

@vertex
fn vertex_specular(input: VertexInput) -> SpecularVaryings {
    // The selected source VS is the same b89a2252 binary as the original port.
    let result = native_world_vertex(primary(input), scene, object);
    return SpecularVaryings(result.clip_position, result.color, result.uv_lightmap,
        result.normal_fog, result.fog_color, result.bitangent, result.tangent,
        result.eye_to_vertex, result.reflection_correction);
}

@vertex
fn vertex_geometric_specular(input: GeometricVertexInput) -> GeometricSpecularVaryings {
    let source = NativeWorldGeometricVertexInput(input.position.xyz, input.color,
        input.uv, input.packed_normal.xyz, input.lightmap_uv);
    let result = native_world_geometric_specular_vertex(source, scene, object);
    return GeometricSpecularVaryings(result.clip_position, result.color,
        result.uv_lightmap, result.normal_fog, result.fog_color,
        result.eye_to_vertex, result.reflection_correction);
}

fn shade_diffuse(input: DiffuseVaryings, cutout: bool, blended: bool) -> vec4<f32> {
    let color = textureSample(color_map, color_sampler, input.uv_lightmap.xy);
    if cutout && fma(color.a, input.color.a, -0.501960813999176) < 0.0 { discard; }
    var source: NativeWorldVaryings;
    source.clip_position = input.clip_position;
    source.color = input.color;
    source.uv_lightmap = input.uv_lightmap;
    source.normal_fog = input.normal_fog;
    source.fog_color = input.fog_color;
    source.bitangent = input.bitangent;
    source.tangent = input.tangent;
    source.eye_to_vertex = input.eye_to_vertex;
    let lit = native_world_diffuse_fragment(source, scene, color,
        normal_map, normal_sampler, lightmap_secondary, lightmap_sampler, blended, color.a * input.color.a);
    if blended { return lit; }
    return vec4<f32>(lit.rgb, select(1.0, color.a * input.color.a, cutout));
}

fn shade_diffuse_reveal(input: DiffuseVaryings) -> vec4<f32> {
    let color = textureSample(color_map, color_sampler, input.uv_lightmap.xy);
    var source: NativeWorldVaryings;
    source.clip_position = input.clip_position;
    source.color = input.color;
    source.uv_lightmap = input.uv_lightmap;
    source.normal_fog = input.normal_fog;
    source.fog_color = input.fog_color;
    source.bitangent = input.bitangent;
    source.tangent = input.tangent;
    source.eye_to_vertex = input.eye_to_vertex;
    let lit = native_world_diffuse_fragment(source, scene, color,
        normal_map, normal_sampler, lightmap_secondary, lightmap_sampler, true,
        native_world_layer_reveal_weight(input.color.a, color.a, reveal_material));
    return lit;
}

fn shade_specular(input: SpecularVaryings, cutout: bool) -> vec4<f32> {
    let color = textureSample(color_map, color_sampler, input.uv_lightmap.xy);
    if cutout && fma(color.a, input.color.a, -0.501960813999176) < 0.0 { discard; }
    let source = NativeWorldVaryings(input.clip_position, input.color,
        input.uv_lightmap, input.normal_fog, input.fog_color, input.bitangent,
        input.tangent, input.eye_to_vertex, input.reflection_correction);
    let lit = native_world_normal_specular_fragment(source, scene,
        color, normal_map, normal_sampler, specular_map,
        specular_sampler, lightmap_secondary, lightmap_sampler,
        reflection_probe, reflection_sampler, false, 1.0);
    return vec4<f32>(lit.rgb, select(1.0, color.a * input.color.a, cutout));
}

fn shade_specular_reveal(input: SpecularVaryings) -> vec4<f32> {
    let color = textureSample(color_map, color_sampler, input.uv_lightmap.xy);
    let source = NativeWorldVaryings(input.clip_position, input.color,
        input.uv_lightmap, input.normal_fog, input.fog_color, input.bitangent,
        input.tangent, input.eye_to_vertex, input.reflection_correction);
    let opacity = native_world_layer_reveal_weight(input.color.a, color.a, reveal_material);
    let lit = native_world_normal_specular_fragment(source, scene,
        color, normal_map, normal_sampler, specular_map,
        specular_sampler, lightmap_secondary, lightmap_sampler,
        reflection_probe, reflection_sampler, true, opacity);
    return lit;
}

fn geometric_source(input: GeometricSpecularVaryings) -> NativeWorldVaryings {
    var source: NativeWorldVaryings;
    source.clip_position = input.clip_position;
    source.color = input.color;
    source.uv_lightmap = input.uv_lightmap;
    source.normal_fog = input.normal_fog;
    source.fog_color = input.fog_color;
    source.eye_to_vertex = input.eye_to_vertex;
    source.reflection_correction = input.reflection_correction;
    return source;
}
fn shade_geometric_specular(input: GeometricSpecularVaryings, blended: bool) -> vec4<f32> {
    return native_world_geometric_specular_fragment(geometric_source(input), scene,
        color_map, color_sampler, specular_map, specular_sampler,
        lightmap_secondary, lightmap_sampler, reflection_probe, reflection_sampler, blended);
}

// Source programs return encoded RGB. Cancel an sRGB attachment's transfer
// only when that view is selected; preserve the source alpha.
fn attachment_srgb(encoded: vec4<f32>) -> vec4<f32> {
    let linear = select(encoded.rgb / vec3<f32>(12.92),
        pow((encoded.rgb + vec3<f32>(0.055)) / vec3<f32>(1.055), vec3<f32>(2.4)),
        encoded.rgb > vec3<f32>(0.04045));
    return vec4<f32>(linear, encoded.a);
}

@fragment
fn fragment_diffuse(input: DiffuseVaryings) -> @location(0) vec4<f32> {
    return shade_diffuse(input, false, false);
}

@fragment
fn fragment_diffuse_srgb(input: DiffuseVaryings) -> @location(0) vec4<f32> {
    return attachment_srgb(shade_diffuse(input, false, false));
}

@fragment
fn fragment_specular(input: SpecularVaryings) -> @location(0) vec4<f32> {
    return shade_specular(input, false);
}

@fragment
fn fragment_specular_srgb(input: SpecularVaryings) -> @location(0) vec4<f32> {
    return attachment_srgb(shade_specular(input, false));
}

@fragment
fn fragment_geometric_specular(input: GeometricSpecularVaryings) -> @location(0) vec4<f32> {
    return shade_geometric_specular(input, false);
}

@fragment
fn fragment_geometric_specular_srgb(input: GeometricSpecularVaryings) -> @location(0) vec4<f32> {
    return attachment_srgb(shade_geometric_specular(input, false));
}

// VS bfa6d365 uses exactly the existing position/fog/geometric-normal sequence.
// SHA256 b6fb653aee80afd8b1b6e83a193da50bcda61e536089f741710b90aa2df6804f.
@vertex
fn vertex_geometric_diffuse(input: GeometricVertexInput) -> GeometricSpecularVaryings {
    let source = NativeWorldGeometricVertexInput(input.position.xyz, input.color,
        input.uv, input.packed_normal.xyz, input.lightmap_uv);
    let result = native_world_plain_base_vertex(source, scene, object);
    return GeometricSpecularVaryings(result.clip_position, result.color,
        result.uv_lightmap, result.normal_fog, result.fog_color,
        result.eye_to_vertex, vec3<f32>(0.0));
}

// PS 2d3e6f96: C0/s0, lightmap13/s13, cb0[20].x only.
// SHA256 14d6e09b190d41a4696a698c8a16b74b22dc995f63d4dd8b7aca76f9b8f37596.
fn shade_geometric_diffuse(input: GeometricSpecularVaryings, blended: bool) -> vec4<f32> {
    let color = textureSample(color_map, color_sampler, input.uv_lightmap.xy);
    if !blended && fma(color.a, input.color.a, -0.501960813999176) < 0.0 { discard; }
    let normal = inverseSqrt(dot(input.normal_fog.xyz, input.normal_fog.xyz)) * input.normal_fog.xyz;
    let lm_uv = input.uv_lightmap.zw;
    let other_uv = fma(lm_uv.xyxy,
        vec4<f32>(1.0, 0.3333333432674408, 1.0, 0.3333333432674408),
        vec4<f32>(0.0, 0.3333333432674408, 0.0, 0.6666666865348816));
    let direction_sample = textureSample(lightmap_secondary, lightmap_sampler, other_uv.zw);
    let directional_sample = textureSample(lightmap_secondary, lightmap_sampler, other_uv.xy);
    let direction = fma(direction_sample.rgb, vec3<f32>(2.0), vec3<f32>(-1.0));
    let cosine = clamp(dot(direction, normal), 0.0, 1.0);
    let directional = (1.0 / (directional_sample.a + 9.999999974752427e-07)) * directional_sample.rgb;
    let ambient = textureSample(lightmap_secondary, lightmap_sampler,
        lm_uv * vec2<f32>(1.0, 0.3333333432674408));
    let light = fma(ambient.rgb, vec3<f32>(1.0 / (ambient.a + 9.999999974752427e-07)), cosine * directional);
    let colored = color.rgb * input.color.rgb;
    let fog_delta = fma(colored * colored, light, -input.fog_color);
    if blended {
        let opacity = color.a * input.color.a;
        let fog_color = opacity * input.fog_color;
        let lit = (colored * colored) * light;
        let fog_delta = fma(lit, vec3<f32>(opacity), -fog_color);
        let fogged = fma(vec3<f32>(input.normal_fog.w), fog_delta, fog_color);
        return vec4<f32>(sqrt(fogged * scene.hdr_control0.x), sqrt(opacity));
    }
    let fogged = fma(vec3<f32>(input.normal_fog.w), fog_delta, input.fog_color);
    return vec4<f32>(sqrt(fogged * scene.hdr_control0.x), color.a * input.color.a);
}

@fragment
fn fragment_geometric_diffuse(input: GeometricSpecularVaryings) -> @location(0) vec4<f32> {
    return shade_geometric_diffuse(input, false);
}

@fragment
fn fragment_geometric_diffuse_srgb(input: GeometricSpecularVaryings) -> @location(0) vec4<f32> {
    return attachment_srgb(shade_geometric_diffuse(input, false));
}

// Cutout PS439f5633 (diffuse) and PSb019cd69 (specular) share the opaque
// lighting math and VSd107eac0/VSb89a2252. They discard below128/255 and
// output sampled alpha times vertex alpha. SHA256 respectively:
// 33277cb5e0e528e4e1514aff9749696bf9fd961ee5c46a304a5255fe387c8089
// b8c7064d55cff4b612e7351e65b64f025fe8b6f1985577f05c188d5dafef82de
@fragment
fn fragment_diffuse_cutout(input: DiffuseVaryings) -> @location(0) vec4<f32> {
    return shade_diffuse(input, true, false);
}
@fragment
fn fragment_diffuse_cutout_srgb(input: DiffuseVaryings) -> @location(0) vec4<f32> {
    return attachment_srgb(shade_diffuse(input, true, false));
}
@fragment
fn fragment_specular_cutout(input: SpecularVaryings) -> @location(0) vec4<f32> {
    return shade_specular(input, true);
}
@fragment
fn fragment_specular_cutout_srgb(input: SpecularVaryings) -> @location(0) vec4<f32> {
    return attachment_srgb(shade_specular(input, true));
}

// Original blended PS4f8f060e, PS212e3fab and PSd2a8a4f1 multiply
// lit color AND fog by opacity before HDR square-root encoding. Output alpha
// is sqrt(opacity); material RGB uses One/OneMinusSrcAlpha, alpha uses
// OneMinusDstAlpha/One. The source gt0 state does not add a PS discard.
@fragment
fn fragment_diffuse_blend(input: DiffuseVaryings) -> @location(0) vec4<f32> {
    return shade_diffuse(input, false, true);
}
@fragment
fn fragment_diffuse_blend_srgb(input: DiffuseVaryings) -> @location(0) vec4<f32> {
    return attachment_srgb(shade_diffuse(input, false, true));
}
@fragment
fn fragment_geometric_diffuse_blend(input: GeometricSpecularVaryings) -> @location(0) vec4<f32> {
    return shade_geometric_diffuse(input, true);
}
@fragment
fn fragment_geometric_diffuse_blend_srgb(input: GeometricSpecularVaryings) -> @location(0) vec4<f32> {
    return attachment_srgb(shade_geometric_diffuse(input, true));
}
@fragment
fn fragment_geometric_specular_blend(input: GeometricSpecularVaryings) -> @location(0) vec4<f32> {
    return shade_geometric_specular(input, true);
}
@fragment
fn fragment_geometric_specular_blend_srgb(input: GeometricSpecularVaryings) -> @location(0) vec4<f32> {
    return attachment_srgb(shade_geometric_specular(input, true));
}

// PS716f0714 uses cb1[58].xy alphaRevealParms and the same normal/specular
// lighting as PS353f0d80. Its reveal curve is exactly the layer reveal helper.
@fragment
fn fragment_specular_reveal(input: SpecularVaryings) -> @location(0) vec4<f32> {
    return shade_specular_reveal(input);
}
@fragment
fn fragment_specular_reveal_srgb(input: SpecularVaryings) -> @location(0) vec4<f32> {
    return attachment_srgb(shade_specular_reveal(input));
}

// PS82e99c4e: diffuse-only equivalent of PS716f0714, including cb1[58].xy.
@fragment
fn fragment_diffuse_reveal(input: DiffuseVaryings) -> @location(0) vec4<f32> {
    return shade_diffuse_reveal(input);
}
@fragment
fn fragment_diffuse_reveal_srgb(input: DiffuseVaryings) -> @location(0) vec4<f32> {
    return attachment_srgb(shade_diffuse_reveal(input));
}

fn shade_geometric_gloss(input: GeometricSpecularVaryings) -> vec4<f32> {
    return native_world_geometric_gloss_fragment(geometric_source(input), scene,
        color_map, color_sampler, lightmap_secondary, lightmap_sampler, reflection_probe, reflection_sampler);
}
@fragment
fn fragment_geometric_gloss(input: GeometricSpecularVaryings) -> @location(0) vec4<f32> { return shade_geometric_gloss(input); }
@fragment
fn fragment_geometric_gloss_srgb(input: GeometricSpecularVaryings) -> @location(0) vec4<f32> { return attachment_srgb(shade_geometric_gloss(input)); }

import package::rendering::native_world::{NativeWorldObject, native_world_unpack_direction};
import package::rendering::native_scripted_model::NativeModelScriptedScene;
import package::rendering::native_world_plain::{NativeWorldGeometricVertexInput, native_world_position_fog};
import package::rendering::native_world_plain::native_world_plain_specular_reflection;
import package::rendering::native_model::{
    NativeModelScene, NativeModelObject, NativeModelVertexInput,
    NativeModelVaryings, native_model_vertex_lighting_with_basis, native_model_vertex, native_model_vertex_lighting, native_model_fragment,
};
import package::rendering::native_model_variants::{
    native_model_specular_lighting, native_model_atlas_irradiance, native_model_detail_diffuse_fragment, NativeModelOcclusion, NativeModelGeometricVertexInput, NativeModelGeometricVaryings,
    native_model_geometric_vertex, native_model_geometric_vertex_lighting, native_model_diffuse_fragment,
    native_model_geometric_diffuse_fragment, native_model_alpha_diffuse_fragment,
    native_model_normal_specular_fragment, native_model_alpha_specular_fragment,
    native_model_blended_specular_fragment,
    native_model_vertex_occlusion_fragment, native_model_geometric_specular_fragment, native_model_blended_diffuse, native_model_mapped_normal,
};

@group(0) @binding(0) var<uniform> scene: NativeModelScene;
@group(0) @binding(1) var<uniform> view: NativeModelScriptedScene;
@group(1) @binding(0) var<uniform> object: NativeModelObject;
@group(2) @binding(0) var color_map: texture_2d<f32>;
@group(2) @binding(1) var color_sampler: sampler;
@group(2) @binding(2) var normal_map: texture_2d<f32>;
@group(2) @binding(3) var normal_sampler: sampler;
@group(2) @binding(4) var reflection_probe: texture_cube<f32>;
@group(2) @binding(5) var reflection_sampler: sampler;
@group(2) @binding(6) var specular_map: texture_2d<f32>;
@group(2) @binding(7) var specular_sampler: sampler;
@group(2) @binding(8) var<uniform> occlusion: NativeModelOcclusion;
@group(3) @binding(0) var model_lighting: texture_3d<f32>;
@group(3) @binding(1) var model_lighting_sampler: sampler;

struct VertexInput {
    @location(0) position: vec4<f32>,
    @location(1) color: vec4<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) packed_normal: vec4<f32>,
    @location(4) packed_tangent: vec4<f32>,
}

struct Varyings {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) normal_fog: vec4<f32>,
    @location(3) fog_color: vec3<f32>,
    @location(4) bitangent: vec3<f32>,
    @location(5) tangent: vec3<f32>,
    @location(6) eye_to_vertex: vec4<f32>,
    @location(7) grid_coords_and_vis: vec4<f32>,
    @location(8) reflection_correction: vec3<f32>,
    @location(9) grid_correction: vec3<f32>,
}

struct GeometricVertexInput {
    @location(0) position: vec3<f32>,
    @location(1) color: vec4<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) packed_normal: vec4<f32>,
}

struct GeometricVaryings {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) normal_fog: vec4<f32>,
    @location(3) fog_color: vec3<f32>,
    @location(6) eye_to_vertex: vec4<f32>,
    @location(7) grid_coords_and_vis: vec4<f32>,
    @location(8) reflection_correction: vec3<f32>,
    @location(9) grid_correction: vec3<f32>,
}

@vertex
fn vertex(input: VertexInput) -> Varyings {
    let source = NativeModelVertexInput(input.position, input.color, input.uv,
        input.packed_normal.xyz, input.packed_tangent.xyz);
    let result = native_model_vertex(source, scene, object);
    return Varyings(result.clip_position, result.color, result.uv,
        result.normal_fog, result.fog_color, result.bitangent, result.tangent,
        result.eye_to_vertex, result.grid_coords_and_vis, result.reflection_correction, result.grid_correction);
}

@vertex
fn vertex_geometric(input: GeometricVertexInput) -> GeometricVaryings {
    let result = native_model_geometric_vertex(
        NativeModelGeometricVertexInput(input.position, input.color, input.uv,
            input.packed_normal.xyz), scene, object);
    return GeometricVaryings(result.clip_position, result.color, result.uv,
        result.normal_fog, result.fog_color, result.eye_to_vertex,
        result.grid_coords_and_vis, result.reflection_correction, result.grid_correction);
}

fn source(input: Varyings) -> NativeModelVaryings {
    return NativeModelVaryings(input.clip_position, input.color, input.uv,
        input.normal_fog, input.fog_color, input.bitangent, input.tangent,
        input.eye_to_vertex, input.grid_coords_and_vis, input.reflection_correction, input.grid_correction);
}

fn shade(input: Varyings) -> vec4<f32> {
    return native_model_fragment(source(input), scene, color_map, color_sampler,
        normal_map, normal_sampler, model_lighting, model_lighting_sampler,
        reflection_probe, reflection_sampler);
}

@fragment
fn fragment(input: Varyings) -> @location(0) vec4<f32> {
    return shade(input);
}

// The original shader already encodes its result. Undo the attachment transfer
// only when the actual color attachment will apply an sRGB encode itself.
@fragment
fn fragment_srgb(input: Varyings) -> @location(0) vec4<f32> {
    return attachment_srgb(shade(input));
}

fn attachment_srgb(encoded: vec4<f32>) -> vec4<f32> {
    let linear = select(encoded.rgb / vec3<f32>(12.92),
        pow((encoded.rgb + vec3<f32>(0.055)) / vec3<f32>(1.055), vec3<f32>(2.4)),
        encoded.rgb > vec3<f32>(0.04045));
    return vec4<f32>(linear, encoded.a);
}

fn shade_diffuse(input: Varyings) -> vec4<f32> {
    return native_model_diffuse_fragment(source(input), scene,
        color_map, color_sampler, normal_map, normal_sampler,
        model_lighting, model_lighting_sampler);
}

fn shade_alpha_diffuse(input: Varyings) -> vec4<f32> {
    return native_model_alpha_diffuse_fragment(source(input), scene,
        color_map, color_sampler, normal_map, normal_sampler,
        model_lighting, model_lighting_sampler);
}

fn shade_geometric_diffuse(input: GeometricVaryings, alpha_test: bool, vertex_occlusion: bool) -> vec4<f32> {
    let geometric = NativeModelGeometricVaryings(input.clip_position, input.color,
        input.uv, input.normal_fog, input.fog_color, input.eye_to_vertex,
        input.grid_coords_and_vis, input.reflection_correction, input.grid_correction);
    return native_model_geometric_diffuse_fragment(geometric, scene, alpha_test, vertex_occlusion,
        color_map, color_sampler, model_lighting, model_lighting_sampler);
}

fn shade_specular(input: Varyings) -> vec4<f32> {
    return native_model_normal_specular_fragment(source(input), scene, occlusion,
        color_map, color_sampler, normal_map, normal_sampler,
        specular_map, specular_sampler, model_lighting, model_lighting_sampler,
        reflection_probe, reflection_sampler);
}

fn shade_alpha_specular(input: Varyings) -> vec4<f32> {
    return native_model_alpha_specular_fragment(source(input), scene, occlusion,
        color_map, color_sampler, normal_map, normal_sampler,
        specular_map, specular_sampler, model_lighting, model_lighting_sampler,
        reflection_probe, reflection_sampler);
}

fn shade_blended_specular(input: Varyings, unattenuated_reflection: bool) -> vec4<f32> {
    return native_model_blended_specular_fragment(source(input), scene, occlusion,
        unattenuated_reflection, color_map, color_sampler, normal_map, normal_sampler,
        specular_map, specular_sampler, model_lighting, model_lighting_sampler,
        reflection_probe, reflection_sampler);
}

@fragment
fn fragment_specular_blended(input: Varyings) -> @location(0) vec4<f32> {
    return shade_blended_specular(input, false);
}
@fragment
fn fragment_specular_blended_srgb(input: Varyings) -> @location(0) vec4<f32> {
    return attachment_srgb(shade_blended_specular(input, false));
}
@fragment
fn fragment_specular_blended_reflection(input: Varyings) -> @location(0) vec4<f32> {
    return shade_blended_specular(input, true);
}
@fragment
fn fragment_specular_blended_reflection_srgb(input: Varyings) -> @location(0) vec4<f32> {
    return attachment_srgb(shade_blended_specular(input, true));
}

fn shade_vertex_occlusion(input: Varyings) -> vec4<f32> {
    return native_model_vertex_occlusion_fragment(source(input), scene,
        color_map, color_sampler, normal_map, normal_sampler,
        model_lighting, model_lighting_sampler, reflection_probe, reflection_sampler);
}

@fragment
fn fragment_diffuse(input: Varyings) -> @location(0) vec4<f32> {
    return shade_diffuse(input);
}

@fragment
fn fragment_diffuse_srgb(input: Varyings) -> @location(0) vec4<f32> {
    return attachment_srgb(shade_diffuse(input));
}

@fragment
fn fragment_alpha_diffuse(input: Varyings) -> @location(0) vec4<f32> {
    return shade_alpha_diffuse(input);
}

@fragment
fn fragment_alpha_diffuse_srgb(input: Varyings) -> @location(0) vec4<f32> {
    return attachment_srgb(shade_alpha_diffuse(input));
}

@fragment
fn fragment_geometric_diffuse(input: GeometricVaryings) -> @location(0) vec4<f32> {
    return shade_geometric_diffuse(input, false, false);
}

@fragment
fn fragment_geometric_diffuse_srgb(input: GeometricVaryings) -> @location(0) vec4<f32> {
    return attachment_srgb(shade_geometric_diffuse(input, false, false));
}

@fragment
fn fragment_geometric_alpha(input: GeometricVaryings) -> @location(0) vec4<f32> {
    return shade_geometric_diffuse(input, true, false);
}
@fragment
fn fragment_geometric_alpha_srgb(input: GeometricVaryings) -> @location(0) vec4<f32> {
    return attachment_srgb(shade_geometric_diffuse(input, true, false));
}
@fragment
fn fragment_geometric_occlusion(input: GeometricVaryings) -> @location(0) vec4<f32> {
    return shade_geometric_diffuse(input, false, true);
}
@fragment
fn fragment_geometric_occlusion_srgb(input: GeometricVaryings) -> @location(0) vec4<f32> {
    return attachment_srgb(shade_geometric_diffuse(input, false, true));
}

@fragment
fn fragment_specular(input: Varyings) -> @location(0) vec4<f32> {
    return shade_specular(input);
}

@fragment
fn fragment_specular_srgb(input: Varyings) -> @location(0) vec4<f32> {
    return attachment_srgb(shade_specular(input));
}

@fragment
fn fragment_alpha_specular(input: Varyings) -> @location(0) vec4<f32> {
    return shade_alpha_specular(input);
}

@fragment
fn fragment_alpha_specular_srgb(input: Varyings) -> @location(0) vec4<f32> {
    return attachment_srgb(shade_alpha_specular(input));
}

@fragment
fn fragment_vertex_occlusion(input: Varyings) -> @location(0) vec4<f32> {
    return shade_vertex_occlusion(input);
}

@fragment
fn fragment_vertex_occlusion_srgb(input: Varyings) -> @location(0) vec4<f32> {
    return attachment_srgb(shade_vertex_occlusion(input));
}

// Per-placement LOD stream supplied separately from shared model geometry.
@vertex
fn vertex_mlv(input: VertexInput, @location(5) vertex_light: vec4<f32>) -> Varyings {
    let result = native_model_vertex_lighting(
        NativeModelVertexInput(input.position, input.color, input.uv,
            input.packed_normal.xyz, input.packed_tangent.xyz),
        scene, object, vertex_light, true);
    return Varyings(result.clip_position, result.color, result.uv,
        result.normal_fog, result.fog_color, result.bitangent, result.tangent,
        result.eye_to_vertex, result.grid_coords_and_vis, result.reflection_correction,
        result.grid_correction);
}

@vertex
fn vertex_geometric_mlv(input: GeometricVertexInput, @location(5) vertex_light: vec4<f32>) -> GeometricVaryings {
    let result = native_model_geometric_vertex_lighting(
        NativeModelGeometricVertexInput(input.position, input.color, input.uv,
            input.packed_normal.xyz), scene, object, vertex_light, true);
    return GeometricVaryings(result.clip_position, result.color, result.uv,
        result.normal_fog, result.fog_color, result.eye_to_vertex,
        result.grid_coords_and_vis, result.reflection_correction, result.grid_correction);
}

fn shade_blended_diffuse(input: Varyings) -> vec4<f32> {
    let normal_sample = textureSample(normal_map, normal_sampler, input.uv);
    let normal = native_model_mapped_normal(source(input), normal_sample.rg);
    let colored = textureSample(color_map, color_sampler, input.uv) * input.color;
    return native_model_blended_diffuse(normal, input.grid_coords_and_vis.xyz,
        input.grid_correction, input.normal_fog, input.fog_color, colored,
        scene, model_lighting, model_lighting_sampler);
}
fn shade_blended_geometric(input: GeometricVaryings) -> vec4<f32> {
    let normal = inverseSqrt(dot(input.normal_fog.xyz, input.normal_fog.xyz)) * input.normal_fog.xyz;
    let colored = textureSample(color_map, color_sampler, input.uv) * input.color;
    return native_model_blended_diffuse(normal, input.grid_coords_and_vis.xyz,
        input.grid_correction, input.normal_fog, input.fog_color, colored,
        scene, model_lighting, model_lighting_sampler);
}
@fragment
fn fragment_blended_diffuse(input: Varyings) -> @location(0) vec4<f32> { return shade_blended_diffuse(input); }
@fragment
fn fragment_blended_diffuse_srgb(input: Varyings) -> @location(0) vec4<f32> { return attachment_srgb(shade_blended_diffuse(input)); }
@fragment
fn fragment_blended_geometric(input: GeometricVaryings) -> @location(0) vec4<f32> { return shade_blended_geometric(input); }
@fragment
fn fragment_blended_geometric_srgb(input: GeometricVaryings) -> @location(0) vec4<f32> { return attachment_srgb(shade_blended_geometric(input)); }

fn shade_geometric_specular(input: GeometricVaryings, transparent: bool) -> vec4<f32> {
    let geometric = NativeModelGeometricVaryings(input.clip_position, input.color,
        input.uv, input.normal_fog, input.fog_color, input.eye_to_vertex,
        input.grid_coords_and_vis, input.reflection_correction, input.grid_correction);
    return native_model_geometric_specular_fragment(geometric, scene, occlusion, transparent,
        color_map, color_sampler, specular_map, specular_sampler, model_lighting,
        model_lighting_sampler, reflection_probe, reflection_sampler);
}
@fragment
fn fragment_geometric_specular(input: GeometricVaryings) -> @location(0) vec4<f32> { return shade_geometric_specular(input, false); }
@fragment
fn fragment_geometric_specular_srgb(input: GeometricVaryings) -> @location(0) vec4<f32> { return attachment_srgb(shade_geometric_specular(input, false)); }
@fragment
fn fragment_geometric_specular_blended(input: GeometricVaryings) -> @location(0) vec4<f32> { return shade_geometric_specular(input, true); }
@fragment
fn fragment_geometric_specular_blended_srgb(input: GeometricVaryings) -> @location(0) vec4<f32> { return attachment_srgb(shade_geometric_specular(input, true)); }

@group(2) @binding(10) var detail_map: texture_2d<f32>;
@group(2) @binding(11) var detail_sampler: sampler;
struct NativeModelDetailScale { detail_scale: vec4<f32> }
@group(2) @binding(12) var<uniform> detail: NativeModelDetailScale;
fn shade_detail_diffuse(input: Varyings) -> vec4<f32> {
    return native_model_detail_diffuse_fragment(source(input), scene, detail.detail_scale,
        color_map, color_sampler, normal_map, normal_sampler, detail_map, detail_sampler,
        model_lighting, model_lighting_sampler);
}
@fragment
fn fragment_detail_diffuse(input: Varyings) -> @location(0) vec4<f32> { return shade_detail_diffuse(input); }
@fragment
fn fragment_detail_diffuse_srgb(input: Varyings) -> @location(0) vec4<f32> { return attachment_srgb(shade_detail_diffuse(input)); }

struct NativeModelReveal { reveal: vec4<f32> }
@group(2) @binding(13) var<uniform> reveal: NativeModelReveal;
@group(2) @binding(14) var occlusion_map: texture_2d<f32>;
@group(2) @binding(15) var occlusion_sampler: sampler;
// Source PS66c64d74: vertex alpha selects a nonlinear reveal interval.
fn shade_reveal(input: Varyings) -> vec4<f32> {
    let normal = native_model_mapped_normal(source(input), textureSample(normal_map, normal_sampler, input.uv).rg);
    let irradiance = native_model_atlas_irradiance(normal, input.grid_coords_and_vis.xyz, scene, model_lighting, model_lighting_sampler);
    let sample = textureSample(color_map, color_sampler, input.uv);
    let colored = sample.rgb * input.color.rgb;
    let diffuse = colored * colored;
    let adjusted = fma(input.color.a, 0.9980000257492065, 0.0010000000474974513);
    let complement = 1.0 - adjusted;
    let powers = vec2<f32>(-exp2(log2(adjusted) * reveal.reveal.y), exp2(log2(complement) * reveal.reveal.y));
    let bounds = clamp(fma(vec2<f32>(reveal.reveal.x), powers, vec2<f32>(complement)), vec2<f32>(0.0), vec2<f32>(1.0));
    let alpha = clamp((sample.a - bounds.x) / (bounds.y - bounds.x), 0.0, 1.0);
    let lit = (irradiance * diffuse) * alpha;
    let fog = alpha * input.fog_color;
    let delta = fma(lit, vec3<f32>(32.0), -fog);
    let fogged = fma(vec3<f32>(input.normal_fog.w), delta, fog);
    let encoded_alpha = sqrt(alpha);
    if !(encoded_alpha > 0.0) { discard; }
    return vec4<f32>(sqrt(fogged * scene.world.hdr_control0.x), encoded_alpha);
}
// Source PS29115a54: occlusion texture G weights reflection, B weights atlas.
fn shade_mapped_occlusion(input: Varyings) -> vec4<f32> {
    let normal = native_model_mapped_normal(source(input), textureSample(normal_map, normal_sampler, input.uv).rg);
    let specular = textureSample(specular_map, specular_sampler, input.uv);
    let reflected = native_world_plain_specular_reflection(input.eye_to_vertex.xyz, normal, specular, reflection_probe, reflection_sampler);
    let occ = textureSample(occlusion_map, occlusion_sampler, input.uv);
    let reflection = (reflected * occ.g) * input.reflection_correction;
    let irradiance = native_model_atlas_irradiance(normal, input.grid_coords_and_vis.xyz, scene, model_lighting, model_lighting_sampler) * occ.b;
    let colored = textureSample(color_map, color_sampler, input.uv) * input.color;
    let diffuse = colored.rgb * colored.rgb;
    let lit = fma(irradiance * diffuse, vec3<f32>(32.0), reflection);
    let fog = colored.a * input.fog_color;
    let delta = fma(lit, vec3<f32>(colored.a), -fog);
    let fogged = fma(vec3<f32>(input.normal_fog.w), delta, fog);
    let alpha = sqrt(colored.a);
    if !(alpha > 0.0) { discard; }
    return vec4<f32>(sqrt(fogged * scene.world.hdr_control0.x), alpha);
}
@fragment
fn fragment_reveal(input: Varyings) -> @location(0) vec4<f32> { return shade_reveal(input); }
@fragment
fn fragment_reveal_srgb(input: Varyings) -> @location(0) vec4<f32> { return attachment_srgb(shade_reveal(input)); }
@fragment
fn fragment_mapped_occlusion(input: Varyings) -> @location(0) vec4<f32> { return shade_mapped_occlusion(input); }
@fragment
fn fragment_mapped_occlusion_srgb(input: Varyings) -> @location(0) vec4<f32> { return attachment_srgb(shade_mapped_occlusion(input)); }

// VSb6c201c: spherical-normal blend and vertex SH. Sway/scatter globals are unused.
struct NativeModelCanopyConstants { lighting: vec4<f32>, tint: vec4<f32> }
@group(2) @binding(16) var<uniform> canopy: NativeModelCanopyConstants;
struct CanopyVaryings {
    @builtin(position) position: vec4<f32>,
    @location(0) uv_fog: vec3<f32>,
    @location(1) lighting: vec3<f32>,
    @location(2) fog_color: vec3<f32>,
}
@vertex
fn vertex_canopy(input: GeometricVertexInput) -> CanopyVaryings {
    let transformed = native_world_position_fog(
        NativeWorldGeometricVertexInput(input.position, input.color, input.uv, input.packed_normal.xyz, vec2<f32>(0.0)),
        scene.world, NativeWorldObject(object.world_rows, vec4<f32>(0.0), vec4<f32>(0.0), vec4<f32>(0.0), vec4<f32>(1.0)));
    let radial = vec3<f32>(dot(input.position, object.world_rows[0].xyz),
        dot(input.position, object.world_rows[1].xyz), dot(input.position, object.world_rows[2].xyz));
    let unpacked = native_world_unpack_direction(input.packed_normal.xyz);
    let original = vec3<f32>(dot(unpacked, object.world_rows[0].xyz),
        dot(unpacked, object.world_rows[1].xyz), dot(unpacked, object.world_rows[2].xyz));
    let delta = fma(radial, vec3<f32>(inverseSqrt(dot(radial, radial))), -original);
    let blended = fma(vec3<f32>(canopy.lighting.x), delta, original);
    let normal = blended * inverseSqrt(dot(blended, blended));
    let squared_yz = normal.yz * normal.yz;
    let quadratic = vec4<f32>(normal.z * normal.x, normal.y * normal.z,
        normal.x * normal.y, fma(normal.x, normal.x, -squared_yz.x));
    let scale = fma(object.grid_lighting_sh0.w, squared_yz.y,
        dot(object.grid_lighting_sh2, quadratic) + dot(object.grid_lighting_sh1, vec4<f32>(normal, 1.0)));
    let irradiance = max(scale * object.grid_lighting_sh0.xyz, vec3<f32>(0.10000000149011612));
    return CanopyVaryings(transformed.clip_position, vec3<f32>(input.uv, transformed.normal_fog.w),
        irradiance * input.color.a, transformed.fog_color);
}
// PSccc7fb2e uses colorTint, not vertex RGB, and no atlas texture or reflection.
fn shade_canopy(input: CanopyVaryings, quasi: bool) -> vec4<f32> {
    let sample = textureSample(color_map, color_sampler, input.uv_fog.xy);
    let alpha = select(sample.a, clamp(sample.a + sample.a, 0.0, 1.0), quasi);
    if (!quasi && sample.a - 0.501960813999176 < 0.0) || (quasi && !(alpha > 0.0)) { discard; }
    let color = sample.rgb * canopy.tint.rgb;
    let delta = fma(color * color, input.lighting, -input.fog_color);
    let fogged = fma(vec3<f32>(input.uv_fog.z), delta, input.fog_color);
    return vec4<f32>(sqrt(fogged * scene.world.hdr_control0.x), alpha);
}
@fragment
fn fragment_canopy(input: CanopyVaryings) -> @location(0) vec4<f32> { return shade_canopy(input, false); }
@fragment
fn fragment_canopy_srgb(input: CanopyVaryings) -> @location(0) vec4<f32> { return attachment_srgb(shade_canopy(input, false)); }

struct NativeModelFlagParams { params: vec4<f32> }
@group(2) @binding(17) var<uniform> flag: NativeModelFlagParams;
// VS3f4a954b / VS037fa2fe: original displacement and derivative normal.
@vertex
fn vertex_flag(input: VertexInput) -> Varyings {
    let uv = vec2<f32>(1.0) - input.uv;
    let products = uv.xyxy * vec4<f32>(3.623051881790161, 0.2533484697341919, 8.250306129455566, -1.4547516107559204);
    let third = uv * vec2<f32>(19.577713012695312, 5.245832443237305);
    let time = (scene.world.game_time.w + flag.params.y) * flag.params.x;
    let phases = fma(vec2<f32>(time), vec2<f32>(5.654866695404053, 9.42477798461914), products.yw + products.xz);
    let phase3 = fma(time, 7.539822578430176, third.y + third.x);
    let sine = sin(phases);
    let cosine = cos(phases);
    let slopes = cosine.xxyy * vec4<f32>(27.172889709472656, 1.9001134634017944, 24.750919342041016, -4.364254474639893);
    let gradient = (fma(vec2<f32>(cos(phase3)), vec2<f32>(33.28211212158203, 8.917915344238281), slopes.zw + slopes.xy) * input.color.a) * 0.02500000037252903;
    let displacement = fma(sin(phase3), 1.7000000476837158, sine.y * 3.0 + sine.x * 7.5) * input.color.a;
    let normal = native_world_unpack_direction(input.packed_normal.xyz);
    let tangent = native_world_unpack_direction(input.packed_tangent.xyz);
    let bitangent = fma(normal.yzx, tangent.zxy, -(normal.zxy * tangent.yzx)) * sign(input.position.w);
    let deformed_normal = normal + fma(vec3<f32>(gradient.x), tangent, gradient.y * bitangent);
    let deformed = NativeModelVertexInput(vec4<f32>(fma(vec3<f32>(displacement), normal, input.position.xyz), input.position.w),
        vec4<f32>(input.color.rgb, 1.0), input.uv, input.packed_normal.xyz, input.packed_tangent.xyz);
    let result = native_model_vertex_lighting_with_basis(deformed, scene, object, vec4<f32>(0.0), false, deformed_normal, tangent);
    return Varyings(result.clip_position, result.color, result.uv, result.normal_fog, result.fog_color,
        result.bitangent, result.tangent, vec4<f32>(result.eye_to_vertex.xyz, 0.5), result.grid_coords_and_vis,
        result.reflection_correction, result.grid_correction);
}
fn shade_flag_detail(input: Varyings) -> vec4<f32> {
    let colored = textureSample(color_map, color_sampler, input.uv) * input.color;
    let detail_sample = textureSample(detail_map, detail_sampler, input.uv * detail.detail_scale.xy).rg;
    let base_sample = textureSample(normal_map, normal_sampler, input.uv).rg;
    let detail_slopes = fma(detail_sample, vec2<f32>(4.015748023986816), vec2<f32>(-2.0157480239868164));
    let base_slopes = fma(base_sample, vec2<f32>(4.015748023986816), vec2<f32>(-2.0157480239868164));
    let slopes = fma(vec2<f32>(detail.detail_scale.z), detail_slopes, base_slopes);
    let mapped = fma(vec3<f32>(slopes.y), input.bitangent, fma(vec3<f32>(slopes.x), input.tangent, input.normal_fog.xyz));
    let normal = mapped * inverseSqrt(dot(mapped, mapped));
    let specular = textureSample(specular_map, specular_sampler, input.uv);
    let reflected = native_world_plain_specular_reflection(input.eye_to_vertex.xyz, normal, specular, reflection_probe, reflection_sampler);
    let result = native_model_specular_lighting(source(input), scene, occlusion, normal,
        reflected, colored.rgb, model_lighting, model_lighting_sampler);
    if colored.a - 0.501960813999176 < 0.0 { discard; }
    return vec4<f32>(result, colored.a);
}
fn shade_flag_web(input: Varyings) -> vec4<f32> {
    let geometric = GeometricVaryings(input.clip_position, input.color, input.uv, input.normal_fog,
        input.fog_color, input.eye_to_vertex, input.grid_coords_and_vis, input.reflection_correction, input.grid_correction);
    return shade_geometric_specular(geometric, true);
}
@fragment
fn fragment_flag_detail(input: Varyings) -> @location(0) vec4<f32> { return shade_flag_detail(input); }
@fragment
fn fragment_flag_detail_srgb(input: Varyings) -> @location(0) vec4<f32> { return attachment_srgb(shade_flag_detail(input)); }
@fragment
fn fragment_flag_web(input: Varyings) -> @location(0) vec4<f32> { return shade_flag_web(input); }
@fragment
fn fragment_flag_web_srgb(input: Varyings) -> @location(0) vec4<f32> { return attachment_srgb(shade_flag_web(input)); }

// PSd0a663d0: source straight-alpha quasi-opaque canopy, alpha=saturate(2*A).
@fragment
fn fragment_canopy_quasi(input: CanopyVaryings) -> @location(0) vec4<f32> { return shade_canopy(input, true); }
@fragment
fn fragment_canopy_quasi_srgb(input: CanopyVaryings) -> @location(0) vec4<f32> { return attachment_srgb(shade_canopy(input, true)); }

// VS7bb0619c differs from VS347155f7 by exactly one operation:
// output UV.y = input UV.y - gameTime.z (native fractional game seconds).
// The PS is the identical original PS4bceb0cc normal-specular program.
@vertex
fn vertex_scrolling(input: VertexInput) -> Varyings {
    let source = NativeModelVertexInput(input.position, input.color, input.uv,
        input.packed_normal.xyz, input.packed_tangent.xyz);
    var result = native_model_vertex(source, scene, object);
    result.uv.y = result.uv.y - scene.world.game_time.z;
    return Varyings(result.clip_position, result.color, result.uv,
        result.normal_fog, result.fog_color, result.bitangent, result.tangent,
        result.eye_to_vertex, result.grid_coords_and_vis, result.reflection_correction, result.grid_correction);
}

// Original PS3c17e0d4, SHA256 7ab5003d6ee9aad875cc883e0b3fcf350f269ba1d44bf5ad3374fc35327c5611.
// weaponParam2 comes from code (0052c1d0/0052b7c0), not colorDetailScale.
@group(2) @binding(18) var<uniform> weapon_detail: vec4<f32>;
// PS4dc2aa8d adds only the hero-lighting transform to PS3c17e0d4.
// SHA256 848b089a23c14d6690923ac2fbd4f15eadbe27facf6531f4ff1a72bb956b6fa4.
fn weapon_detail_lighting(input: Varyings) -> vec3<f32> {
    let detail_uv = fma(input.uv - vec2<f32>(0.5), vec2<f32>(weapon_detail.z), weapon_detail.xy) + vec2<f32>(0.5);
    let detail_sample = textureSample(detail_map, detail_sampler, detail_uv);
    let occ = textureSample(occlusion_map, occlusion_sampler, input.uv);
    let detail_color = clamp(detail_sample.rgb + vec3<f32>(occ.b) - vec3<f32>(0.5), vec3<f32>(0.0), vec3<f32>(1.0));
    let color = textureSample(color_map, color_sampler, input.uv);
    let masked_color = fma(vec3<f32>(color.a), color.rgb - detail_color, detail_color);
    let colored = fma(vec3<f32>(weapon_detail.w), masked_color - color.rgb, color.rgb) * input.color.rgb;
    let diffuse = colored * colored;
    let normal = native_model_mapped_normal(source(input), textureSample(normal_map, normal_sampler, input.uv).rg);
    let irradiance = native_model_atlas_irradiance(normal, input.grid_coords_and_vis.xyz, scene, model_lighting, model_lighting_sampler);
    let diffuse_lit = diffuse * (occ.r * irradiance);
    let specular = textureSample(specular_map, specular_sampler, input.uv);
    let detail_specular = clamp(vec3<f32>(detail_sample.a - 0.5) + specular.rgb, vec3<f32>(0.0), vec3<f32>(1.0));
    let masked_specular = fma(vec3<f32>(color.a), specular.rgb - detail_specular, detail_specular);
    let specular_gamma = fma(vec3<f32>(weapon_detail.w), masked_specular - specular.rgb, specular.rgb);
    let reflected = native_world_plain_specular_reflection(input.eye_to_vertex.xyz, normal,
        vec4<f32>(specular_gamma, specular.a), reflection_probe, reflection_sampler);
    let reflection = (occ.g * reflected) * input.reflection_correction;
    return fma(diffuse_lit, vec3<f32>(32.0), reflection);
}
fn finish_weapon_detail(input: Varyings, lit: vec3<f32>) -> vec4<f32> {
    let fogged = fma(vec3<f32>(input.normal_fog.w), lit - input.fog_color, input.fog_color);
    return vec4<f32>(sqrt(fogged * scene.world.hdr_control0.x), 1.0);
}
fn shade_weapon_detail_hero(input: Varyings) -> vec4<f32> {
    let hero_input = vec4<f32>(weapon_detail_lighting(input), 1.0);
    let lit = vec3<f32>(dot(hero_input, view.hero_lighting[0]),
        dot(hero_input, view.hero_lighting[1]), dot(hero_input, view.hero_lighting[2]));
    return finish_weapon_detail(input, lit);
}
@fragment
fn fragment_weapon_detail(input: Varyings) -> @location(0) vec4<f32> { return finish_weapon_detail(input, weapon_detail_lighting(input)); }
@fragment
fn fragment_weapon_detail_srgb(input: Varyings) -> @location(0) vec4<f32> { return attachment_srgb(finish_weapon_detail(input, weapon_detail_lighting(input))); }
@fragment
fn fragment_weapon_detail_hero(input: Varyings) -> @location(0) vec4<f32> { return shade_weapon_detail_hero(input); }
@fragment
fn fragment_weapon_detail_hero_srgb(input: Varyings) -> @location(0) vec4<f32> { return attachment_srgb(shade_weapon_detail_hero(input)); }

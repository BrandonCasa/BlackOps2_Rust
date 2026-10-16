import package::rendering::native_weapon_model_optic::native_optic_fragment;
import package::rendering::native_weapon_model_staff::native_staff_fragment;
import package::rendering::native_character_model::{NativeSourceCharacterConstants,
    native_character_hair, native_character_skin_cutout, native_character_emission, native_character_reveal_emission,
    native_character_ember_flicker, native_character_ember, native_character_glass, native_character_fritz, native_character_fill, native_character_cornea};
import package::rendering::native_model::{NativeModelScene, NativeModelObject, NativeModelVertexInput};
import package::rendering::native_scripted_model::{
    NativeModelScriptedScene, NativeModelScriptInputs, NativeModelParallaxConstants,
    NativeFlickerConstants, native_model_flicker_fragment,
    NativeClothRevealConstants, native_model_cloth_reveal_fragment,
    native_model_player_icon_fragment,
    NativeClanTagConstants, native_model_clan_tag_fragment, NativeModelEmissiveConstants, NativeScriptedVaryings, NativeEyeVaryings,
    native_scripted_model_vertex, native_scripted_model_vertex_lighting, native_model_crystal_fragment, native_model_phong_emissive_fragment,
    native_model_emissive_multi_fragment, native_model_phong_fragment, native_model_eye_vertex, native_model_eye_fragment, native_scripted_layer_uv, native_scripted_bitangent,
    native_scripted_parallax_basis, native_scripted_three_layer_uvs,
};
import package::rendering::native_camo::{NativeModelCamoParameters, native_model_camo_fragment};

@group(0) @binding(0) var<uniform> scene: NativeModelScene;
@group(0) @binding(1) var<uniform> view: NativeModelScriptedScene;
@group(1) @binding(0) var<uniform> object: NativeModelObject;
@group(1) @binding(1) var<uniform> scripts: NativeModelScriptInputs;
@group(2) @binding(0) var input0: texture_2d<f32>;
@group(2) @binding(1) var sampler0: sampler;
@group(2) @binding(2) var input1: texture_2d<f32>;
@group(2) @binding(3) var sampler1: sampler;
@group(2) @binding(4) var input2: texture_2d<f32>;
@group(2) @binding(5) var sampler2: sampler;
@group(2) @binding(6) var input3: texture_2d<f32>;
@group(2) @binding(7) var sampler3: sampler;
@group(2) @binding(8) var probe: texture_cube<f32>;
@group(2) @binding(9) var probe_sampler: sampler;
@group(2) @binding(10) var weapon_normal: texture_2d<f32>;
@group(2) @binding(11) var weapon_normal_sampler: sampler;
@group(2) @binding(12) var<uniform> parallax: NativeModelParallaxConstants;
@group(2) @binding(13) var<uniform> disc: NativeModelEmissiveConstants;
@group(2) @binding(14) var<uniform> camo: NativeModelCamoParameters;
@group(2) @binding(20) var<uniform> flicker: NativeFlickerConstants;
@group(2) @binding(21) var<uniform> phong_controls: vec4<f32>;
@group(2) @binding(23) var<uniform> staff_emission_on: vec4<f32>;
@group(2) @binding(22) var<uniform> cloth_reveal: NativeClothRevealConstants;
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
    @location(0) fog_color: vec3<f32>,
    @location(1) eye_fog: vec4<f32>,
    @location(2) color: vec4<f32>,
    @location(3) normal: vec3<f32>,
    @location(4) tangent_sign: vec4<f32>,
    @location(5) uv: vec2<f32>,
    @location(6) grid_coords_and_vis: vec4<f32>,
    @location(7) reflection_correction: vec3<f32>,
    @location(8) grid_correction: vec3<f32>,
}

struct EyeVertexInput {
    @location(0) position: vec3<f32>,
    @location(2) uv: vec2<f32>,
}

// Original VS254a3594 omits the grid-coordinate interpolant. It supplies SH
// correction directly; these two pixel programs never sample the atlas.
struct ParallaxVaryings {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) fog_color: vec3<f32>,
    @location(1) eye_fog: vec4<f32>,
    @location(2) color: vec4<f32>,
    @location(3) normal: vec3<f32>,
    @location(4) tangent_sign: vec4<f32>,
    @location(5) uv: vec2<f32>,
    @location(7) reflection_correction: vec3<f32>,
}

struct EyeVaryings {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) fog_color: vec3<f32>,
    @location(1) eye_fog: vec4<f32>,
    @location(2) uv: vec2<f32>,
}

@vertex
fn vertex_scripted(input: VertexInput) -> Varyings {
    let result = native_scripted_model_vertex(NativeModelVertexInput(input.position,
        input.color, input.uv, input.packed_normal.xyz, input.packed_tangent.xyz), scene, object);
    return Varyings(result.clip_position, result.fog_color, result.eye_fog, result.color,
        result.normal, result.tangent_sign, result.uv, result.grid_coords_and_vis,
        result.reflection_correction, result.grid_correction);
}

fn parallax_vertex(input: VertexInput) -> ParallaxVaryings {
    let result = native_scripted_model_vertex(NativeModelVertexInput(input.position,
        input.color, input.uv, input.packed_normal.xyz, input.packed_tangent.xyz), scene, object);
    return ParallaxVaryings(result.clip_position, result.fog_color, result.eye_fog, result.color,
        result.normal, result.tangent_sign, result.uv, result.reflection_correction);
}

@vertex
fn vertex_eyes(input: EyeVertexInput) -> EyeVaryings {
    let result = native_model_eye_vertex(input.position, input.uv, scene, object);
    return EyeVaryings(result.clip_position, result.fog_color, result.eye_fog, result.uv);
}

fn source(input: Varyings) -> NativeScriptedVaryings {
    return NativeScriptedVaryings(input.clip_position, input.fog_color, input.eye_fog,
        input.color, input.normal, input.tangent_sign, input.uv, input.grid_coords_and_vis,
        input.reflection_correction, input.grid_correction);
}

fn source_parallax(input: ParallaxVaryings) -> NativeScriptedVaryings {
    // The shared library's grid-coordinate field is unused by both parallax
    // programs; it is absent from their actual vertex/fragment interface.
    return NativeScriptedVaryings(input.clip_position, input.fog_color, input.eye_fog,
        input.color, input.normal, input.tangent_sign, input.uv, vec4<f32>(0.0),
        input.reflection_correction, vec3<f32>(1.0));
}

fn shade_crystal(input: ParallaxVaryings) -> vec4<f32> {
    return native_model_crystal_fragment(source_parallax(input), scene, view, scripts, parallax,
        input0, sampler0, input1, sampler1, input2, sampler2, input3, sampler3,
        probe, probe_sampler);
}

fn shade_camo(input: ParallaxVaryings) -> vec4<f32> {
    return native_model_camo_fragment(source_parallax(input), scene, view, parallax, camo,
        input0, sampler0, input1, sampler1, input2, sampler2, input3, sampler3,
        weapon_normal, weapon_normal_sampler, probe, probe_sampler);
}

fn shade_disc(input: Varyings) -> vec4<f32> {
    return native_model_phong_emissive_fragment(source(input), scene, view, scripts.vectors[3], disc, false,
        input0, sampler0, input1, sampler1, input2, sampler2, input3, sampler3,
        model_lighting, model_lighting_sampler, probe, probe_sampler);
}

fn shade_eyes(input: EyeVaryings) -> vec4<f32> {
    return native_model_eye_fragment(NativeEyeVaryings(input.clip_position, input.fog_color,
        input.eye_fog, input.uv), scene, scripts, input0, sampler0);
}

fn attachment_srgb(encoded: vec4<f32>) -> vec4<f32> {
    let linear = select(encoded.rgb / vec3<f32>(12.92),
        pow((encoded.rgb + vec3<f32>(0.055)) / vec3<f32>(1.055), vec3<f32>(2.4)),
        encoded.rgb > vec3<f32>(0.04045));
    return vec4<f32>(linear, encoded.a);
}

@fragment
fn fragment_crystal(input: ParallaxVaryings) -> @location(0) vec4<f32> { return shade_crystal(input); }
@fragment
fn fragment_crystal_srgb(input: ParallaxVaryings) -> @location(0) vec4<f32> { return attachment_srgb(shade_crystal(input)); }
@fragment
fn fragment_camo(input: ParallaxVaryings) -> @location(0) vec4<f32> { return shade_camo(input); }
@fragment
fn fragment_camo_srgb(input: ParallaxVaryings) -> @location(0) vec4<f32> { return attachment_srgb(shade_camo(input)); }
@fragment
fn fragment_disc(input: Varyings) -> @location(0) vec4<f32> { return shade_disc(input); }
@fragment
fn fragment_disc_srgb(input: Varyings) -> @location(0) vec4<f32> { return attachment_srgb(shade_disc(input)); }
@fragment
fn fragment_eyes(input: EyeVaryings) -> @location(0) vec4<f32> { return shade_eyes(input); }
@fragment
fn fragment_eyes_srgb(input: EyeVaryings) -> @location(0) vec4<f32> { return attachment_srgb(shade_eyes(input)); }

// Original Alcatraz PS431e7c81 uses Emissive_Push, not scriptVector3.
fn shade_emissive(input: Varyings, transparent: bool) -> vec4<f32> {
    return native_model_phong_emissive_fragment(source(input), scene, view,
        vec4<f32>(disc.gloss_push.y), disc, transparent,
        input0, sampler0, input1, sampler1, input2, sampler2, input3, sampler3,
        model_lighting, model_lighting_sampler, probe, probe_sampler);
}
@fragment
fn fragment_emissive(input: Varyings) -> @location(0) vec4<f32> { return shade_emissive(input, false); }
@fragment
fn fragment_emissive_srgb(input: Varyings) -> @location(0) vec4<f32> { return attachment_srgb(shade_emissive(input, false)); }

@vertex
fn vertex_scripted_mlv(input: VertexInput, @location(5) vertex_light: vec4<f32>) -> Varyings {
    let result = native_scripted_model_vertex_lighting(NativeModelVertexInput(input.position,
        input.color, input.uv, input.packed_normal.xyz, input.packed_tangent.xyz),
        scene, object, vertex_light, true);
    return Varyings(result.clip_position, result.fog_color, result.eye_fog, result.color,
        result.normal, result.tangent_sign, result.uv, result.grid_coords_and_vis,
        result.reflection_correction, result.grid_correction);
}

@fragment
fn fragment_emissive_transparent(input: Varyings) -> @location(0) vec4<f32> { return shade_emissive(input, true); }
@fragment
fn fragment_emissive_transparent_srgb(input: Varyings) -> @location(0) vec4<f32> { return attachment_srgb(shade_emissive(input, true)); }

// Original VS63533fae uses SH correction, not the ordinary MLV baked RGB path.
// Its only vertex-light contribution is alpha * 0.00001 added to red correction.
@vertex
fn vertex_crystal_mlv(input: VertexInput, @location(5) vertex_light: vec4<f32>) -> ParallaxVaryings {
    var result = parallax_vertex(input);
    result.reflection_correction.x = fma(vertex_light.a, 0.000009999999747378752,
        result.reflection_correction.x);
    return result;
}

@vertex
fn vertex_parallax(input: VertexInput) -> ParallaxVaryings { return parallax_vertex(input); }

// Original four-layer globals cb2[5..17]. Unlike the crystal program, UV
// parallax and rotation happen in the vertex stage, with depth + 0.00001.
struct NativeModelFourLayerConstants { source: array<vec4<f32>, 13> }
@group(2) @binding(15) var<uniform> four: NativeModelFourLayerConstants;
struct FourVaryings {
    @builtin(position) position: vec4<f32>,
    @location(0) eye_fog: vec4<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) fog_color: vec3<f32>,
    @location(3) reflection_correction: vec3<f32>,
    @location(4) uv12: vec4<f32>,
    @location(5) uv34: vec4<f32>,
}
struct FourInput {
    @location(0) position: vec4<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) packed_normal: vec4<f32>,
    @location(4) packed_tangent: vec4<f32>,
}
fn four_vertex(input: FourInput, vertex_alpha: f32, fog: bool) -> FourVaryings {
    let result = native_scripted_model_vertex(NativeModelVertexInput(input.position,
        vec4<f32>(1.0), input.uv, input.packed_normal.xyz, input.packed_tangent.xyz), scene, object);
    // Source VS uses negative position.w handedness for this parallax basis.
    let bitangent = -native_scripted_bitangent(result.normal, result.tangent_sign);
    let eye = -result.eye_fog.xyz * inverseSqrt(dot(result.eye_fog.xyz, result.eye_fog.xyz));
    let facing = dot(eye, result.normal);
    let divided = -vec2<f32>(dot(eye, result.tangent_sign.xyz), dot(eye, bitangent)) / facing;
    let ray = bitcast<vec2<f32>>(bitcast<vec2<u32>>(divided) & vec2<u32>(select(0u, 0xffffffffu, facing > 0.0)));
    let c = four.source;
    let epsilon = 0.000009999999747378752;
    let uv1 = native_scripted_layer_uv(input.uv, ray, c[0].xy, c[0].zw, c[1].xy, c[8].xy, c[5].zw, c[8].z + epsilon, view.game_time.w);
    let uv2 = native_scripted_layer_uv(input.uv, ray, c[1].zw, c[2].zw, c[2].xy, c[7].zw, c[6].xy, c[8].w + epsilon, view.game_time.w);
    let uv3 = native_scripted_layer_uv(input.uv, ray, c[3].xy, c[3].zw, c[4].xy, c[7].xy, c[6].zw, c[9].x + epsilon, view.game_time.w);
    let uv4 = native_scripted_layer_uv(input.uv, ray, c[10].xy, c[10].zw, c[11].xy, c[11].zw, c[12].zw, c[12].y + epsilon, view.game_time.w);
    let correction = vec3<f32>(fma(vertex_alpha, epsilon, result.reflection_correction.x), result.reflection_correction.yz);
    return FourVaryings(result.clip_position, vec4<f32>(result.eye_fog.xyz, select(1.0, result.eye_fog.w, fog)),
        result.normal, select(vec3<f32>(0.0), result.fog_color, fog), correction, vec4<f32>(uv1, uv2), vec4<f32>(uv3, uv4));
}
@vertex
fn vertex_four_fog(input: FourInput) -> FourVaryings { return four_vertex(input, 0.0, true); }
@vertex
fn vertex_four_nofog(input: FourInput) -> FourVaryings { return four_vertex(input, 0.0, false); }
@vertex
fn vertex_four_mlv(input: FourInput, @location(5) lighting: vec4<f32>) -> FourVaryings { return four_vertex(input, lighting.a, false); }
fn shade_four(input: FourVaryings, fog: bool) -> vec4<f32> {
    let c = four.source;
    let layer1 = textureSample(input0, sampler0, input.uv12.xy);
    let layer2 = textureSample(input1, sampler1, input.uv12.zw);
    let layer3 = textureSample(input2, sampler2, input.uv34.xy);
    let layer4 = textureSample(input3, sampler3, input.uv34.zw);
    let gates = vec4<f32>(1.0) - clamp(scripts.vectors[0], vec4<f32>(0.0), vec4<f32>(1.0));
    let alpha3 = gates.z * c[5].x * layer3.a;
    let alpha2 = gates.y * c[4].w * layer2.a;
    let alpha1 = gates.x * c[4].z * layer1.a;
    let combined3 = fma(vec3<f32>(alpha3), fma(-layer4.rgb, vec3<f32>(layer4.a), layer3.rgb), layer4.a * layer4.rgb);
    let combined2 = fma(vec3<f32>(alpha2), layer2.rgb - combined3, combined3);
    let composed = fma(vec3<f32>(alpha1), layer1.rgb - combined2, combined2);
    var gamma = composed;
    if !fog {
        let alpha4 = gates.w * c[12].x;
        let coverage3 = clamp(fma(layer4.a, alpha4, alpha3), 0.0, 1.0);
        let coverage2 = clamp(fma(layer2.a, gates.y * c[4].w, coverage3), 0.0, 1.0);
        let coverage1 = clamp(fma(layer1.a, gates.x * c[4].z, coverage2), 0.0, 1.0);
        gamma = coverage1 * composed;
    }
    gamma *= c[5].y;
    let emission = gamma * gamma;
    let normal = input.normal * inverseSqrt(dot(input.normal, input.normal));
    let eye = input.eye_fog.xyz * inverseSqrt(dot(input.eye_fog.xyz, input.eye_fog.xyz));
    let direction = fma(normal, vec3<f32>(-2.0 * dot(eye, normal)), eye);
    let reflection = textureSampleLevel(probe, probe_sampler, direction, fma(-4.0, c[9].y, 4.0));
    let decoded = reflection.rgb * (1.0 / (reflection.a + 9.999999974752427e-07));
    let reflected = (decoded * clamp(fma(0.25, c[9].y, 0.75), 0.0, 1.0)) * c[9].z;
    var lit = fma(reflected, input.reflection_correction, emission);
    if fog { lit = fma(vec3<f32>(input.eye_fog.w), lit - input.fog_color, input.fog_color); }
    return vec4<f32>(sqrt(lit * scene.world.hdr_control0.x), 1.0);
}
@fragment
fn fragment_four_fog(input: FourVaryings) -> @location(0) vec4<f32> { return shade_four(input, true); }
@fragment
fn fragment_four_fog_srgb(input: FourVaryings) -> @location(0) vec4<f32> { return attachment_srgb(shade_four(input, true)); }
@fragment
fn fragment_four_nofog(input: FourVaryings) -> @location(0) vec4<f32> { return shade_four(input, false); }
@fragment
fn fragment_four_nofog_srgb(input: FourVaryings) -> @location(0) vec4<f32> { return attachment_srgb(shade_four(input, false)); }

// VS197b665e / VS5f626c64: no fog or vertex color; SH reflection correction.
struct FalloffVaryings {
    @builtin(position) position: vec4<f32>,
    @location(0) eye: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) tangent_sign: vec4<f32>,
    @location(3) uv: vec2<f32>,
    @location(4) reflection_correction: vec3<f32>,
}
fn falloff_vertex(input: FourInput, vertex_alpha: f32) -> FalloffVaryings {
    let result = native_scripted_model_vertex(NativeModelVertexInput(input.position,
        vec4<f32>(1.0), input.uv, input.packed_normal.xyz, input.packed_tangent.xyz), scene, object);
    let correction = vec3<f32>(fma(vertex_alpha, 0.000009999999747378752,
        result.reflection_correction.x), result.reflection_correction.yz);
    return FalloffVaryings(result.clip_position, result.eye_fog.xyz, result.normal,
        result.tangent_sign, result.uv, correction);
}
@vertex
fn vertex_falloff(input: FourInput) -> FalloffVaryings { return falloff_vertex(input, 0.0); }
@vertex
fn vertex_falloff_mlv(input: FourInput, @location(5) lighting: vec4<f32>) -> FalloffVaryings {
    return falloff_vertex(input, lighting.a);
}
// PS597f540d omits coverage. PS997302f8 and PSdf6a08c2 have identical
// instructions and accumulate coverage from all three script-controlled layers.
fn shade_falloff(input: FalloffVaryings, mode: u32) -> vec4<f32> {
    let source = NativeScriptedVaryings(input.position, vec3<f32>(0.0),
        vec4<f32>(input.eye, 1.0), vec4<f32>(1.0), input.normal, input.tangent_sign,
        input.uv, vec4<f32>(0.0), input.reflection_correction, vec3<f32>(1.0));
    let basis = native_scripted_parallax_basis(source);
    let c = parallax.source;
    let uv = native_scripted_three_layer_uvs(input.uv, basis.ray, parallax, view.game_time.w);
    let layer3 = textureSample(input2, sampler2, uv.layer3);
    let layer2 = textureSample(input1, sampler1, uv.layer2);
    let layer1 = textureSample(input0, sampler0, uv.layer1);
    let gates = vec3<f32>(1.0) - clamp(scripts.vectors[0].xyz, vec3<f32>(0.0), vec3<f32>(1.0));
    let alpha2 = gates.y * c[4].w * layer2.a;
    let alpha1 = gates.x * c[4].z * layer1.a;
    let combined2 = fma(vec3<f32>(alpha2), fma(-layer3.rgb, vec3<f32>(layer3.a), layer2.rgb), layer3.a * layer3.rgb);
    let composed = fma(vec3<f32>(alpha1), layer1.rgb - combined2, combined2);
    var color = composed;
    var alpha = 1.0;
    if mode != 0u {
        let coverage2 = clamp(fma(layer3.a, gates.z * c[5].x, alpha2), 0.0, 1.0);
        let coverage1 = clamp(fma(layer1.a, gates.x * c[4].z, coverage2), 0.0, 1.0);
        if mode == 1u {
            color *= coverage1;
        } else {
            // PSc21aa953 outputs sqrt(coverage), leaving RGB unmultiplied.
            alpha = sqrt(coverage1);
            if !(alpha > 0.0) { discard; }
        }
    }
    let eye = input.eye * inverseSqrt(dot(input.eye, input.eye));
    let facing = clamp(dot(-eye, basis.normal), 0.0, 1.0);
    var emission_scale = c[9].z;
    if mode != 3u {
        let falloff = exp2(log2(facing) * c[10].x) * c[10].y;
        emission_scale = falloff * emission_scale;
    }
    let gamma = emission_scale * color;
    let direction = fma(basis.normal, vec3<f32>(-2.0 * dot(eye, basis.normal)), eye);
    let reflection = textureSampleLevel(probe, probe_sampler, direction, fma(-4.0, c[9].y, 4.0));
    let decoded = reflection.rgb * (1.0 / (reflection.a + 9.999999974752427e-07));
    let reflected = (decoded * clamp(fma(0.25, c[9].y, 0.75), 0.0, 1.0)) * c[9].w;
    let lit = fma(reflected, input.reflection_correction, gamma * gamma);
    return vec4<f32>(sqrt(lit * scene.world.hdr_control0.x), alpha);
}
@fragment
fn fragment_falloff(input: FalloffVaryings) -> @location(0) vec4<f32> { return shade_falloff(input, 0u); }
@fragment
fn fragment_falloff_srgb(input: FalloffVaryings) -> @location(0) vec4<f32> { return attachment_srgb(shade_falloff(input, 0u)); }
@fragment
fn fragment_falloff_coverage(input: FalloffVaryings) -> @location(0) vec4<f32> { return shade_falloff(input, 1u); }
@fragment
fn fragment_falloff_coverage_srgb(input: FalloffVaryings) -> @location(0) vec4<f32> { return attachment_srgb(shade_falloff(input, 1u)); }

fn shade_character(input: Varyings, controls: vec3<f32>) -> vec4<f32> {
    return native_model_phong_fragment(source(input), scene, view, scripts.vectors[0].x, controls, false,
        input0, sampler0, input1, sampler1, input2, sampler2,
        model_lighting, model_lighting_sampler, probe, probe_sampler);
}
@fragment
fn fragment_cloth(input: Varyings) -> @location(0) vec4<f32> { return shade_character(input, vec3<f32>(1.0)); }
@fragment
fn fragment_cloth_srgb(input: Varyings) -> @location(0) vec4<f32> { return attachment_srgb(shade_character(input, vec3<f32>(1.0))); }
@fragment
fn fragment_skin(input: Varyings) -> @location(0) vec4<f32> { return shade_character(input, phong_controls.xyz); }
@fragment
fn fragment_skin_srgb(input: Varyings) -> @location(0) vec4<f32> { return attachment_srgb(shade_character(input, phong_controls.xyz)); }

fn shade_player_icon(input: Varyings) -> vec4<f32> {
    return native_model_player_icon_fragment(source(input), scene, view, phong_controls.xyz,
        input0, sampler0, input1, sampler1, input2, sampler2,
        model_lighting, model_lighting_sampler, probe, probe_sampler);
}
@fragment
fn fragment_player_icon(input: Varyings) -> @location(0) vec4<f32> { return shade_player_icon(input); }
@fragment
fn fragment_player_icon_srgb(input: Varyings) -> @location(0) vec4<f32> { return attachment_srgb(shade_player_icon(input)); }

fn shade_viewmodel_transparent(input: Varyings) -> vec4<f32> {
    return native_model_phong_fragment(source(input), scene, view, 0.0, phong_controls.xyz, true,
        input0, sampler0, input1, sampler1, input2, sampler2,
        model_lighting, model_lighting_sampler, probe, probe_sampler);
}
@fragment
fn fragment_viewmodel_transparent(input: Varyings) -> @location(0) vec4<f32> { return shade_viewmodel_transparent(input); }
@fragment
fn fragment_viewmodel_transparent_srgb(input: Varyings) -> @location(0) vec4<f32> { return attachment_srgb(shade_viewmodel_transparent(input)); }

fn shade_flicker(input: Varyings) -> vec4<f32> {
    return native_model_flicker_fragment(source(input), scene, view, scripts, flicker,
        input0, sampler0, input1, sampler1, input2, sampler2, input3, sampler3,
        model_lighting, model_lighting_sampler, probe, probe_sampler);
}
@fragment
fn fragment_flicker(input: Varyings) -> @location(0) vec4<f32> { return shade_flicker(input); }
@fragment
fn fragment_flicker_srgb(input: Varyings) -> @location(0) vec4<f32> { return attachment_srgb(shade_flicker(input)); }

fn shade_emissive_multi(input: Varyings, alpha_test: bool) -> vec4<f32> {
    return native_model_emissive_multi_fragment(source(input), scene, view, scripts, disc, alpha_test,
        input0, sampler0, input1, sampler1, input2, sampler2, input3, sampler3,
        model_lighting, model_lighting_sampler, probe, probe_sampler);
}
@fragment
fn fragment_emissive_multi(input: Varyings) -> @location(0) vec4<f32> { return shade_emissive_multi(input, true); }
@fragment
fn fragment_emissive_multi_srgb(input: Varyings) -> @location(0) vec4<f32> { return attachment_srgb(shade_emissive_multi(input, true)); }
@fragment
fn fragment_emissive_multi_opaque(input: Varyings) -> @location(0) vec4<f32> { return shade_emissive_multi(input, false); }
@fragment
fn fragment_emissive_multi_opaque_srgb(input: Varyings) -> @location(0) vec4<f32> { return attachment_srgb(shade_emissive_multi(input, false)); }

@fragment
fn fragment_falloff_alpha(input: FalloffVaryings) -> @location(0) vec4<f32> { return shade_falloff(input, 2u); }
@fragment
fn fragment_falloff_alpha_srgb(input: FalloffVaryings) -> @location(0) vec4<f32> { return attachment_srgb(shade_falloff(input, 2u)); }

// PS61f120bd shares the three-layer blend and sqrt coverage with PSc21aa953,
// but applies hdrAmount directly, without the angular falloff calculation.
@fragment
fn fragment_three_alpha(input: FalloffVaryings) -> @location(0) vec4<f32> { return shade_falloff(input, 3u); }
@fragment
fn fragment_three_alpha_srgb(input: FalloffVaryings) -> @location(0) vec4<f32> { return attachment_srgb(shade_falloff(input, 3u)); }

@group(2) @binding(19) var<uniform> clan_tag: NativeClanTagConstants;
fn shade_clan_tag(input: Varyings) -> vec4<f32> {
    return native_model_clan_tag_fragment(source(input), scene, view, clan_tag,
        input0, sampler0, input1, sampler1, input2, sampler2,
        model_lighting, model_lighting_sampler, probe, probe_sampler);
}
@fragment
fn fragment_clan_tag(input: Varyings) -> @location(0) vec4<f32> { return shade_clan_tag(input); }
@fragment
fn fragment_clan_tag_srgb(input: Varyings) -> @location(0) vec4<f32> { return attachment_srgb(shade_clan_tag(input)); }

fn shade_cloth_reveal(input: Varyings, alpha_test: bool) -> vec4<f32> {
    return native_model_cloth_reveal_fragment(source(input), scene, view, scripts, cloth_reveal, alpha_test,
        input0, sampler0, input1, sampler1, input2, sampler2, input3, sampler3,
        model_lighting, model_lighting_sampler, probe, probe_sampler);
}
@fragment
fn fragment_cloth_reveal(input: Varyings) -> @location(0) vec4<f32> { return shade_cloth_reveal(input, false); }
@fragment
fn fragment_cloth_reveal_srgb(input: Varyings) -> @location(0) vec4<f32> { return attachment_srgb(shade_cloth_reveal(input, false)); }
@fragment
fn fragment_cloth_reveal_cutout(input: Varyings) -> @location(0) vec4<f32> { return shade_cloth_reveal(input, true); }
@fragment
fn fragment_cloth_reveal_cutout_srgb(input: Varyings) -> @location(0) vec4<f32> { return attachment_srgb(shade_cloth_reveal(input, true)); }

@group(2) @binding(26) var<uniform> source_character: NativeSourceCharacterConstants;
fn shade_source_hair(input: Varyings) -> vec4<f32> {
    return native_character_hair(source(input), scene, view, source_character,
        input0, sampler0, input1, sampler1, input2, sampler2,
        model_lighting, model_lighting_sampler, probe, probe_sampler);
}
fn shade_source_skin_cutout(input: Varyings) -> vec4<f32> {
    return native_character_skin_cutout(source(input), scene, view, scripts, source_character,
        input0, sampler0, input1, sampler1, input2, sampler2,
        model_lighting, model_lighting_sampler, probe, probe_sampler);
}
fn shade_source_character_emission(input: Varyings) -> vec4<f32> {
    return native_character_emission(source(input), scene, view, scripts, source_character,
        input0, sampler0, input1, sampler1, input2, sampler2, input3, sampler3,
        model_lighting, model_lighting_sampler, probe, probe_sampler);
}
fn shade_source_reveal_emission(input: Varyings, cutout: bool) -> vec4<f32> {
    return native_character_reveal_emission(source(input), scene, view, scripts, source_character, cutout,
        input0, sampler0, input1, sampler1, input2, sampler2, input3, sampler3, weapon_normal, weapon_normal_sampler,
        model_lighting, model_lighting_sampler, probe, probe_sampler);
}
@fragment
fn fragment_source_hair(input: Varyings) -> @location(0) vec4<f32> { return shade_source_hair(input); }
@fragment
fn fragment_source_hair_srgb(input: Varyings) -> @location(0) vec4<f32> { return attachment_srgb(shade_source_hair(input)); }
@fragment
fn fragment_source_skin_cutout(input: Varyings) -> @location(0) vec4<f32> { return shade_source_skin_cutout(input); }
@fragment
fn fragment_source_skin_cutout_srgb(input: Varyings) -> @location(0) vec4<f32> { return attachment_srgb(shade_source_skin_cutout(input)); }
@fragment
fn fragment_source_cloth_emission(input: Varyings) -> @location(0) vec4<f32> { return shade_source_character_emission(input); }
@fragment
fn fragment_source_cloth_emission_srgb(input: Varyings) -> @location(0) vec4<f32> { return attachment_srgb(shade_source_character_emission(input)); }
@fragment
fn fragment_source_skin_emission(input: Varyings) -> @location(0) vec4<f32> { return shade_source_character_emission(input); }
@fragment
fn fragment_source_skin_emission_srgb(input: Varyings) -> @location(0) vec4<f32> { return attachment_srgb(shade_source_character_emission(input)); }
@fragment
fn fragment_source_reveal_emission(input: Varyings) -> @location(0) vec4<f32> { return shade_source_reveal_emission(input, false); }
@fragment
fn fragment_source_reveal_emission_srgb(input: Varyings) -> @location(0) vec4<f32> { return attachment_srgb(shade_source_reveal_emission(input, false)); }
@fragment
fn fragment_source_reveal_emission_cutout(input: Varyings) -> @location(0) vec4<f32> { return shade_source_reveal_emission(input, true); }
@fragment
fn fragment_source_reveal_emission_cutout_srgb(input: Varyings) -> @location(0) vec4<f32> { return attachment_srgb(shade_source_reveal_emission(input, true)); }

@group(2) @binding(27) var character_reveal: texture_2d<f32>;
@group(2) @binding(28) var character_reveal_sampler: sampler;

struct CharacterExtendedVaryings {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) fog_color: vec3<f32>,
    @location(1) eye_fog: vec4<f32>,
    @location(2) color: vec4<f32>,
    @location(3) normal: vec3<f32>,
    @location(4) tangent_sign: vec4<f32>,
    @location(5) uv: vec2<f32>,
    @location(6) grid_coords_and_vis: vec4<f32>,
    @location(7) reflection_correction: vec3<f32>,
    @location(8) grid_correction: vec3<f32>,
    @location(9) extra: vec4<f32>,
}
fn character_extended(data: NativeScriptedVaryings, extra: vec4<f32>) -> CharacterExtendedVaryings {
    return CharacterExtendedVaryings(data.clip_position, data.fog_color, data.eye_fog, data.color,
        data.normal, data.tangent_sign, data.uv, data.grid_coords_and_vis, data.reflection_correction, data.grid_correction, extra);
}
fn character_source(input: CharacterExtendedVaryings) -> NativeScriptedVaryings {
    return NativeScriptedVaryings(input.clip_position, input.fog_color, input.eye_fog, input.color,
        input.normal, input.tangent_sign, input.uv, input.grid_coords_and_vis, input.reflection_correction, input.grid_correction);
}
@vertex
fn vertex_source_ember(input: VertexInput) -> CharacterExtendedVaryings {
    let data = native_scripted_model_vertex(NativeModelVertexInput(input.position, input.color,
        input.uv, input.packed_normal.xyz, input.packed_tangent.xyz), scene, object);
    return character_extended(data, vec4<f32>(native_character_ember_flicker(view.game_time.w, source_character), 0.0, 0.0, 0.0));
}
@vertex
fn vertex_source_glass(input: VertexInput) -> CharacterExtendedVaryings {
    let data = native_scripted_model_vertex(NativeModelVertexInput(input.position, input.color,
        input.uv, input.packed_normal.xyz, input.packed_tangent.xyz), scene, object);
    let bitangent = native_scripted_bitangent(data.normal, data.tangent_sign);
    let eye = inverseSqrt(dot(-data.eye_fog.xyz, -data.eye_fog.xyz)) * -data.eye_fog.xyz;
    let facing = dot(eye, data.normal);
    let divided = -vec2<f32>(dot(eye, data.tangent_sign.xyz), dot(eye, bitangent)) / facing;
    let mask = select(0u, 0xffffffffu, facing > 0.0);
    let ray = bitcast<vec2<f32>>(bitcast<vec2<u32>>(divided) & vec2<u32>(mask));
    let emission_uv = fma(ray, vec2<f32>(source_character.source[0].z + 9.999999747378752e-06), data.uv);
    return character_extended(data, vec4<f32>(emission_uv, 0.0, 0.0));
}
fn shade_source_ember(input: CharacterExtendedVaryings) -> vec4<f32> {
    return native_character_ember(character_source(input), input.extra.x, scene, view, scripts, source_character,
        input0, sampler0, input1, sampler1, input2, sampler2, input3, sampler3,
        weapon_normal, weapon_normal_sampler, character_reveal, character_reveal_sampler,
        model_lighting, model_lighting_sampler, probe, probe_sampler);
}
fn shade_source_glass(input: CharacterExtendedVaryings) -> vec4<f32> {
    return native_character_glass(character_source(input), input.extra.xy, scene, view, source_character,
        input0, sampler0, input1, sampler1, input2, sampler2,
        model_lighting, model_lighting_sampler, probe, probe_sampler);
}
@fragment
fn fragment_source_ember(input: CharacterExtendedVaryings) -> @location(0) vec4<f32> { return shade_source_ember(input); }
@fragment
fn fragment_source_ember_srgb(input: CharacterExtendedVaryings) -> @location(0) vec4<f32> { return attachment_srgb(shade_source_ember(input)); }
@fragment
fn fragment_source_glass(input: CharacterExtendedVaryings) -> @location(0) vec4<f32> { return shade_source_glass(input); }
@fragment
fn fragment_source_glass_srgb(input: CharacterExtendedVaryings) -> @location(0) vec4<f32> { return attachment_srgb(shade_source_glass(input)); }

fn shade_source_fritz(input: Varyings, cutout: bool) -> vec4<f32> {
    return native_character_fritz(source(input), scene, view, scripts, source_character, cutout,
        input0, sampler0, input1, sampler1, input2, sampler2, input3, sampler3,
        weapon_normal, weapon_normal_sampler, character_reveal, character_reveal_sampler,
        model_lighting, model_lighting_sampler, probe, probe_sampler);
}
@fragment
fn fragment_source_fritz(input: Varyings) -> @location(0) vec4<f32> { return shade_source_fritz(input, false); }
@fragment
fn fragment_source_fritz_srgb(input: Varyings) -> @location(0) vec4<f32> { return attachment_srgb(shade_source_fritz(input, false)); }
@fragment
fn fragment_source_fritz_cutout(input: Varyings) -> @location(0) vec4<f32> { return shade_source_fritz(input, true); }
@fragment
fn fragment_source_fritz_cutout_srgb(input: Varyings) -> @location(0) vec4<f32> { return attachment_srgb(shade_source_fritz(input, true)); }

struct CorneaVertexInput {
    @location(0) position: vec4<f32>, @location(2) uv: vec2<f32>,
    @location(3) packed_normal: vec4<f32>, @location(4) packed_tangent: vec4<f32>,
}
struct CorneaVaryings {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) fog_color: vec3<f32>, @location(1) eye_fog: vec4<f32>,
    @location(3) normal: vec3<f32>, @location(4) tangent_sign: vec4<f32>,
    @location(5) uv: vec2<f32>, @location(6) grid_coords_and_vis: vec4<f32>,
    @location(9) fill1: vec3<f32>, @location(10) fill2: vec3<f32>,
}
@vertex
fn vertex_source_cornea(input: CorneaVertexInput) -> CorneaVaryings {
    let data = native_scripted_model_vertex(NativeModelVertexInput(input.position, vec4<f32>(0.0), input.uv,
        input.packed_normal.xyz, input.packed_tangent.xyz), scene, object);
    return CorneaVaryings(data.clip_position, data.fog_color, data.eye_fog, data.normal, data.tangent_sign, data.uv,
        data.grid_coords_and_vis, native_character_fill(source_character.source[0].xyz, scene), native_character_fill(source_character.source[2].xyz, scene));
}
fn shade_source_cornea(input: CorneaVaryings) -> vec4<f32> {
    let data = NativeScriptedVaryings(input.clip_position, input.fog_color, input.eye_fog, vec4<f32>(0.0), input.normal,
        input.tangent_sign, input.uv, input.grid_coords_and_vis, vec3<f32>(0.0), vec3<f32>(0.0));
    return native_character_cornea(data, input.fill1, input.fill2, scene, source_character,
        input0, sampler0, input1, sampler1, model_lighting, model_lighting_sampler);
}
@fragment
fn fragment_source_cornea(input: CorneaVaryings) -> @location(0) vec4<f32> { return shade_source_cornea(input); }
@fragment
fn fragment_source_cornea_srgb(input: CorneaVaryings) -> @location(0) vec4<f32> { return attachment_srgb(shade_source_cornea(input)); }

fn shade_staff(input: ParallaxVaryings) -> vec4<f32> {
    return native_staff_fragment(source_parallax(input), scene, view, scripts, parallax, staff_emission_on.x,
        input0, sampler0, input1, sampler1, input2, sampler2, input3, sampler3,
        weapon_normal, weapon_normal_sampler, probe, probe_sampler);
}
@fragment fn fragment_staff(input: ParallaxVaryings) -> @location(0) vec4<f32> { return shade_staff(input); }
@fragment fn fragment_staff_srgb(input: ParallaxVaryings) -> @location(0) vec4<f32> { return attachment_srgb(shade_staff(input)); }

@group(2) @binding(24) var<uniform> optic_controls: vec4<f32>;
struct NativeModelSingleLayerConstants { source: array<vec4<f32>, 4> }
@group(2) @binding(25) var<uniform> single: NativeModelSingleLayerConstants;
fn shade_optic(input: Varyings) -> vec4<f32> {
    return native_optic_fragment(source(input), scene, view, optic_controls, input0, sampler0,
        input1, sampler1, input2, sampler2, input3, sampler3, model_lighting,
        model_lighting_sampler, probe, probe_sampler);
}
@fragment fn fragment_optic(input: Varyings) -> @location(0) vec4<f32> { return shade_optic(input); }
@fragment fn fragment_optic_srgb(input: Varyings) -> @location(0) vec4<f32> { return attachment_srgb(shade_optic(input)); }
@vertex fn vertex_single(input: FourInput) -> FourVaryings {
    let result = native_scripted_model_vertex(NativeModelVertexInput(input.position,
        vec4<f32>(1.0), input.uv, input.packed_normal.xyz, input.packed_tangent.xyz), scene, object);
    let bitangent = -native_scripted_bitangent(result.normal, result.tangent_sign);
    let eye = -result.eye_fog.xyz * inverseSqrt(dot(result.eye_fog.xyz, result.eye_fog.xyz));
    let facing = dot(eye, result.normal);
    let divided = -vec2<f32>(dot(eye, result.tangent_sign.xyz), dot(eye, bitangent)) / facing;
    let ray = bitcast<vec2<f32>>(bitcast<vec2<u32>>(divided) & vec2<u32>(select(0u, 0xffffffffu, facing > 0.0)));
    let c = single.source;
    let uv = native_scripted_layer_uv(input.uv, ray, c[0].xy, c[0].zw, c[1].xy,
        c[2].zw, c[2].xy, c[3].x + 0.000009999999747378752, view.game_time.w);
    return FourVaryings(result.clip_position, vec4<f32>(result.eye_fog.xyz, 0.0), result.normal,
        vec3<f32>(0.0), result.reflection_correction, vec4<f32>(uv, 0.0, 0.0), vec4<f32>(0.0));
}
fn shade_single(input: FourVaryings) -> vec4<f32> {
    let c = single.source;
    let normal = input.normal * inverseSqrt(dot(input.normal, input.normal));
    let eye = input.eye_fog.xyz * inverseSqrt(dot(input.eye_fog.xyz, input.eye_fog.xyz));
    let direction = fma(normal, vec3<f32>(-2.0 * dot(eye, normal)), eye);
    let reflection = textureSampleLevel(probe, probe_sampler, direction, fma(-4.0, c[3].y, 4.0));
    let decoded = reflection.rgb * (1.0 / (reflection.a + 9.999999974752427e-07));
    let reflected = (decoded * clamp(fma(0.25, c[3].y, 0.75), 0.0, 1.0)) * c[3].z;
    let layer = textureSample(input0, sampler0, input.uv12.xy);
    let emission = (layer.a * layer.rgb) * c[1].w;
    let lit = fma(reflected, input.reflection_correction, emission * emission);
    let alpha = sqrt(((1.0 - clamp(scripts.vectors[0].x, 0.0, 1.0)) * c[1].z) * layer.a);
    if !(alpha > 0.0) { discard; }
    return vec4<f32>(sqrt(lit * scene.world.hdr_control0.x), alpha);
}
@fragment fn fragment_single(input: FourVaryings) -> @location(0) vec4<f32> { return shade_single(input); }
@fragment fn fragment_single_srgb(input: FourVaryings) -> @location(0) vec4<f32> { return attachment_srgb(shade_single(input)); }

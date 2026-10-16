import package::rendering::native_model_variants::native_model_atlas_irradiance;
import package::rendering::native_model::NativeModelScene;
import package::rendering::native_scripted_model::{NativeModelScriptedScene,
    NativeModelScriptInputs, NativeScriptedVaryings, native_model_phong_shade, native_model_phong_shade_samples, native_scripted_bitangent};

struct NativeSourceCharacterConstants { source: array<vec4<f32>, 5> }

// PS910a9f03: no script specular suppression, vertex alpha attenuation or
// cb0[114..116]. Alpha is twice the fourth root of diffuse/vertex alpha.
fn native_character_hair(input: NativeScriptedVaryings, scene: NativeModelScene,
    view: NativeModelScriptedScene, material: NativeSourceCharacterConstants,
    normal_map: texture_2d<f32>, normal_sampler: sampler,
    color_map: texture_2d<f32>, color_sampler: sampler,
    specular_map: texture_2d<f32>, specular_sampler: sampler,
    atlas: texture_3d<f32>, atlas_sampler: sampler, probe: texture_cube<f32>, probe_sampler: sampler) -> vec4<f32> {
    let color = textureSample(color_map, color_sampler, input.uv) * input.color;
    var lighting_input = input;
    lighting_input.color.a = 1.0;
    let result = native_model_phong_shade(lighting_input, scene, view, 0.0,
        vec3<f32>(material.source[0].x, 1.0, 1.0), false, false, color.rgb * color.rgb,
        1.0, vec3<f32>(0.0), normal_map, normal_sampler, specular_map, specular_sampler,
        atlas, atlas_sampler, probe, probe_sampler);
    let root = sqrt(sqrt(color.a));
    let alpha = root + root;
    if !(alpha > 0.0) { discard; }
    return vec4<f32>(result.rgb, alpha);
}

// PSf19fb54c / PSc65881e3: scriptVector0.x switches to source warp,
// squared diffuse luma and moving sparkles; .y independently scales emission.
fn native_character_fritz(input: NativeScriptedVaryings, scene: NativeModelScene,
    view: NativeModelScriptedScene, scripts: NativeModelScriptInputs, material: NativeSourceCharacterConstants, cutout: bool,
    normal_map: texture_2d<f32>, normal_sampler: sampler,
    color_map: texture_2d<f32>, color_sampler: sampler,
    specular_map: texture_2d<f32>, specular_sampler: sampler,
    glow_map: texture_2d<f32>, glow_sampler: sampler,
    warp_map: texture_2d<f32>, warp_sampler: sampler,
    sparkle_map: texture_2d<f32>, sparkle_sampler: sampler,
    atlas: texture_3d<f32>, atlas_sampler: sampler, probe: texture_cube<f32>, probe_sampler: sampler) -> vec4<f32> {
    let normal_sample = textureSample(normal_map, normal_sampler, input.uv).rg;
    let original = textureSample(color_map, color_sampler, input.uv);
    var diffuse = original;
    var specular = textureSample(specular_map, specular_sampler, input.uv);
    var glow: vec4<f32>;
    if scripts.vectors[0].x > 0.0 {
        let warp_sample = textureSample(warp_map, warp_sampler, input.uv * material.source[2].x).rg;
        let warp = fma(warp_sample, vec2<f32>(2.0), vec2<f32>(-1.0));
        let warped_uv = fma(vec2<f32>(material.source[1].w), warp, input.uv);
        let warped_color = textureSample(color_map, color_sampler, warped_uv);
        let luma = dot(warped_color.rgb, vec3<f32>(0.30000001192092896, 0.5899999737739563, 0.10999999940395355));
        let luma_boost = (luma * luma) * material.source[2].y;
        diffuse = vec4<f32>(fma(vec3<f32>(scripts.vectors[0].x), vec3<f32>(luma_boost) - original.rgb, original.rgb), warped_color.a);
        let glow_sample = textureSample(glow_map, glow_sampler, warped_uv);
        let offsets = fract((view.game_time.w * material.source[1].xy) * material.source[1].z);
        let sparkle_uv = fma(vec2<f32>(material.source[1].w), warp,
            fma(input.uv, vec2<f32>(material.source[1].z), offsets));
        let sparkle = textureSample(sparkle_map, sparkle_sampler, sparkle_uv);
        let sparkle_amount = ((input.uv.y * input.uv.y) * input.uv.y) * sparkle.a;
        let sparkled = fma(vec3<f32>(sparkle_amount), sparkle.rgb - vec3<f32>(luma_boost), vec3<f32>(luma_boost));
        glow = vec4<f32>(fma(vec3<f32>(scripts.vectors[0].x), sparkled - original.rgb, original.rgb), glow_sample.a);
        let warped_specular = textureSample(specular_map, specular_sampler, warped_uv);
        let specular_luma = dot(warped_specular.rgb, vec3<f32>(0.30000001192092896, 0.5899999737739563, 0.10999999940395355));
        specular = vec4<f32>(fma(vec3<f32>(scripts.vectors[0].x), vec3<f32>(specular_luma) - specular.rgb, specular.rgb), specular.a);
    } else {
        glow = textureSample(glow_map, glow_sampler, input.uv);
    }
    let colored = diffuse * input.color;
    let emission = ((glow.a * glow.rgb) * (material.source[0].z * scripts.vectors[0].y));
    var lighting_input = input;
    lighting_input.color.a = 1.0;
    let result = native_model_phong_shade_samples(lighting_input, scene, view, 0.0,
        vec3<f32>(material.source[0].xy, 1.0), false, false,
        colored.rgb * colored.rgb, 1.0, emission * emission,
        normal_sample, specular, atlas, atlas_sampler, probe, probe_sampler);
    if cutout && fma(diffuse.a, input.color.a, -0.501960813999176) < 0.0 { discard; }
    return vec4<f32>(result.rgb, select(1.0, colored.a, cutout));
}

// VSbac46018: both authored fill directions pass through current native
// inverseViewMatrix rows, normalize and negate before interpolation.
fn native_character_fill(direction: vec3<f32>, scene: NativeModelScene) -> vec3<f32> {
    let fill = vec3<f32>(dot(direction, scene.world.inverse_view_rows[0].xyz),
        dot(direction, scene.world.inverse_view_rows[1].xyz), dot(direction, scene.world.inverse_view_rows[2].xyz));
    return -(fill * inverseSqrt(dot(fill, fill)));
}
fn native_character_highlight(normal: vec3<f32>, eye_vector: vec3<f32>, fill_vector: vec3<f32>, size: f32, sharpness: f32) -> f32 {
    let one_minus = 1.0 - sharpness;
    let upper = fma(one_minus, 0.18000000715255737, 0.7200000286102295);
    let lower = fma(-one_minus, 0.18000000715255737, 0.7200000286102295);
    let reciprocal_range = 1.0 / (upper - lower);
    let reciprocal_size = 1.0 / max(size, 9.999999747378752e-06);
    let fill = fill_vector * inverseSqrt(dot(fill_vector, fill_vector));
    let half_raw = fma(-eye_vector, vec3<f32>(inverseSqrt(dot(-eye_vector, -eye_vector))), -fill);
    let half_vector = half_raw * inverseSqrt(dot(half_raw, half_raw));
    let power = exp2(log2(clamp(dot(normal, half_vector), 0.0, 1.0)) * reciprocal_size);
    let weight = clamp(reciprocal_range * (power - lower), 0.0, 1.0);
    let curve = (weight * weight) * fma(weight, -2.0, 3.0);
    let facing = clamp((dot(normal, -fill) + 0.20000000298023224) * 5.0, 0.0, 1.0);
    let facing_curve = (facing * facing) * fma(facing, -2.0, 3.0);
    return facing_curve * curve;
}

// PS566a2eb6: highlight alpha uses mapped normal while lit RGB deliberately
// uses the geometric normal; the texture is an alpha mask, never diffuse color.
fn native_character_cornea(input: NativeScriptedVaryings, fill1: vec3<f32>, fill2: vec3<f32>,
    scene: NativeModelScene, material: NativeSourceCharacterConstants,
    normal_map: texture_2d<f32>, normal_sampler: sampler, mask_map: texture_2d<f32>, mask_sampler: sampler,
    atlas: texture_3d<f32>, atlas_sampler: sampler) -> vec4<f32> {
    let geometric = input.normal * inverseSqrt(dot(input.normal, input.normal));
    let bitangent = native_scripted_bitangent(geometric, input.tangent_sign);
    let slopes = fma(textureSample(normal_map, normal_sampler, input.uv).rg,
        vec2<f32>(4.015748023986816), vec2<f32>(-2.0157480239868164));
    let mapped = fma(vec3<f32>(slopes.x), input.tangent_sign.xyz, slopes.y * bitangent);
    let normal_raw = fma(input.normal, vec3<f32>(inverseSqrt(dot(input.normal, input.normal))), mapped);
    let normal = normal_raw * inverseSqrt(dot(normal_raw, normal_raw));
    let h1 = native_character_highlight(normal, input.eye_fog.xyz, fill1, material.source[0].w, material.source[1].x) * material.source[1].y;
    let h2 = native_character_highlight(normal, input.eye_fog.xyz, fill2, material.source[2].w, material.source[3].x);
    let highlight = clamp(fma(h2, material.source[3].y, h1) - 0.05000000074505806, 0.0, 1.0);
    let alpha = sqrt(highlight * textureSample(mask_map, mask_sampler, input.uv).r);
    let irradiance = native_model_atlas_irradiance(geometric, input.grid_coords_and_vis.xyz, scene, atlas, atlas_sampler) * 32.0;
    let sun = input.grid_coords_and_vis.w * scene.world.sun_diffuse.xyz;
    let lit = fma(vec3<f32>(clamp(dot(scene.world.sun_position.xyz, geometric), 0.0, 1.0)), sun, irradiance);
    let intensity = material.source[3].z * material.source[3].z;
    let delta = fma(vec3<f32>(intensity), lit, -input.fog_color);
    let fogged = fma(vec3<f32>(input.eye_fog.w), delta, input.fog_color);
    if !(alpha > 0.0) { discard; }
    return vec4<f32>(sqrt(fogged * scene.world.hdr_control0.x), alpha);
}

// PSbfafae7f: source cutout uses diffuse alpha times vertex-alpha squared.
fn native_character_skin_cutout(input: NativeScriptedVaryings, scene: NativeModelScene,
    view: NativeModelScriptedScene, scripts: NativeModelScriptInputs, material: NativeSourceCharacterConstants,
    normal_map: texture_2d<f32>, normal_sampler: sampler,
    color_map: texture_2d<f32>, color_sampler: sampler,
    specular_map: texture_2d<f32>, specular_sampler: sampler,
    atlas: texture_3d<f32>, atlas_sampler: sampler, probe: texture_cube<f32>, probe_sampler: sampler) -> vec4<f32> {
    let sample = textureSample(color_map, color_sampler, input.uv);
    let vertex = input.color * input.color;
    let color = sample * vertex;
    let result = native_model_phong_shade(input, scene, view, scripts.vectors[0].x,
        material.source[0].xyz, false, true, color.rgb * color.rgb, 1.0,
        vec3<f32>(0.0), normal_map, normal_sampler, specular_map, specular_sampler,
        atlas, atlas_sampler, probe, probe_sampler);
    if fma(vertex.a, sample.a, -0.501960813999176) < 0.0 { discard; }
    return vec4<f32>(result.rgb, color.a);
}

// PS111adca3 / PSf4616e1c: source emission is vertex-colored and alpha
// multiplied before the square, then vertex alpha multiplies the result again.
fn native_character_emission(input: NativeScriptedVaryings, scene: NativeModelScene,
    view: NativeModelScriptedScene, scripts: NativeModelScriptInputs, material: NativeSourceCharacterConstants,
    normal_map: texture_2d<f32>, normal_sampler: sampler,
    color_map: texture_2d<f32>, color_sampler: sampler,
    specular_map: texture_2d<f32>, specular_sampler: sampler,
    emission_map: texture_2d<f32>, emission_sampler: sampler,
    atlas: texture_3d<f32>, atlas_sampler: sampler, probe: texture_cube<f32>, probe_sampler: sampler) -> vec4<f32> {
    let diffuse = textureSample(color_map, color_sampler, input.uv).rgb * (input.color.rgb * input.color.rgb);
    let sample = textureSample(emission_map, emission_sampler, input.uv) * input.color;
    let emission = (sample.a * sample.rgb) * material.source[0].w;
    return native_model_phong_shade(input, scene, view, scripts.vectors[0].x,
        material.source[0].xyz, false, true, diffuse * diffuse, 1.0,
        (emission * emission) * input.color.a, normal_map, normal_sampler,
        specular_map, specular_sampler, atlas, atlas_sampler, probe, probe_sampler);
}

// PSb6a2f4cb / PSae63e624: reveal controls are interpolated before shading;
// the temporal edge and pulsing diffuse emission combine before their square.
fn native_character_reveal_emission(input: NativeScriptedVaryings, scene: NativeModelScene,
    view: NativeModelScriptedScene, scripts: NativeModelScriptInputs, material: NativeSourceCharacterConstants, cutout: bool,
    normal_map: texture_2d<f32>, normal_sampler: sampler,
    color_map: texture_2d<f32>, color_sampler: sampler,
    specular_map: texture_2d<f32>, specular_sampler: sampler,
    emission_map: texture_2d<f32>, emission_sampler: sampler,
    temporal_map: texture_2d<f32>, temporal_sampler: sampler,
    atlas: texture_3d<f32>, atlas_sampler: sampler, probe: texture_cube<f32>, probe_sampler: sampler) -> vec4<f32> {
    let temporal = textureSample(temporal_map, temporal_sampler, input.uv).r;
    let thresholds = vec2<f32>(scripts.vectors[3].x) + vec2<f32>(0.05999999865889549, -0.029999999329447746);
    let ramps = clamp((vec2<f32>(temporal) - thresholds) * vec2<f32>(-8.333333015441895, 16.66666603088379), vec2<f32>(0.0), vec2<f32>(1.0));
    let curves = (ramps * ramps) * fma(ramps, vec2<f32>(-2.0), vec2<f32>(3.0));
    let sample = textureSample(color_map, color_sampler, input.uv);
    let base = sample * input.color;
    let state1 = base.r * material.source[2].rgb;
    let altered = fma(vec3<f32>(scripts.vectors[3].y), fma(vec3<f32>(base.r), material.source[3].rgb, -state1), state1);
    let blended = fma(vec4<f32>(curves.x), fma(-sample, input.color, vec4<f32>(altered, base.a)), base);
    let colored = blended.rgb * input.color.rgb;
    let pulse = fma(sin(view.game_time.w * material.source[4].x), 0.5, 0.5);
    let pulse_scale = fma(pulse, material.source[3].w - 1.0, 1.0);
    let emissive = textureSample(emission_map, emission_sampler, input.uv).rgb;
    let diffuse_emission = (pulse_scale * base.rgb) * emissive * curves.y;
    let edge_color = fma(vec3<f32>(scripts.vectors[3].y), material.source[1].rgb - material.source[0].yzw, material.source[0].yzw);
    let edge = fma(vec3<f32>(curves.y * curves.x), edge_color, diffuse_emission) * material.source[0].x;
    let result = native_model_phong_shade(input, scene, view, scripts.vectors[0].x,
        vec3<f32>(1.0, material.source[4].y, 1.0), false, true, colored * colored, 1.0,
        (edge * edge) * input.color.a, normal_map, normal_sampler, specular_map, specular_sampler,
        atlas, atlas_sampler, probe, probe_sampler);
    let alpha = blended.a * input.color.a;
    if cutout && fma(blended.a, input.color.a, -0.501960813999176) < 0.0 { discard; }
    return vec4<f32>(result.rgb, select(1.0, alpha, cutout));
}

// VSa3e80090: material/time seeded source sequence, quantized at ten Hz.
fn native_character_ember_flicker(seconds: f32, material: NativeSourceCharacterConstants) -> f32 {
    let ticks = fma(seconds, 10.0, -fract(seconds * 10.0));
    let seed = fma(ticks, 0.10000000149011612, material.source[2].w);
    let weights = vec4<f32>(81.23948669433594, 17.348024368286133, 37.34983825683594, 59.394840240478516);
    var state = vec4<f32>(seed);
    state.x = fract(dot(state, weights));
    state.y = fract(dot(state.xwww, weights));
    state.z = fract(dot(state.xyww, weights));
    state.w = fract(dot(state, weights));
    state.x = fract(dot(state, weights));
    state.y = fract(dot(state, weights));
    state.z = fract(dot(state, weights));
    let random = fract(dot(state, weights));
    return fma(random, material.source[2].y - material.source[2].z, material.source[2].z);
}

// PSb04a3ce2. Heat/ember UV scrolling, reveal mask and two vertex-color
// multiplications stay in the same domains as the original source program.
fn native_character_ember(input: NativeScriptedVaryings, flicker: f32,
    scene: NativeModelScene, view: NativeModelScriptedScene, scripts: NativeModelScriptInputs,
    material: NativeSourceCharacterConstants,
    normal_map: texture_2d<f32>, normal_sampler: sampler,
    color_map: texture_2d<f32>, color_sampler: sampler,
    specular_map: texture_2d<f32>, specular_sampler: sampler,
    heat_map: texture_2d<f32>, heat_sampler: sampler,
    ember_map: texture_2d<f32>, ember_sampler: sampler,
    reveal_map: texture_2d<f32>, reveal_sampler: sampler,
    atlas: texture_3d<f32>, atlas_sampler: sampler, probe: texture_cube<f32>, probe_sampler: sampler) -> vec4<f32> {
    var offsets = view.game_time.w * material.source[1];
    offsets = vec4<f32>(offsets.xy * material.source[0].w, offsets.zw * material.source[2].x);
    offsets = fract(offsets);
    let ember_uv = fma(input.uv, vec2<f32>(material.source[2].x), offsets.zw);
    let ember_sample = textureSample(ember_map, ember_sampler, ember_uv);
    let ember = ember_sample.rgb;
    let heat_uv = fma(input.uv, vec2<f32>(material.source[0].w), offsets.xy);
    let heat = textureSample(heat_map, heat_sampler, heat_uv);
    let heated = (heat.rgb * flicker) * 6.0;
    let base = textureSample(color_map, color_sampler, input.uv).rgb * input.color.rgb;
    let reveal_sample = textureSample(reveal_map, reveal_sampler, input.uv);
    let amount = clamp(reveal_sample.r * input.color.r, 0.0, 1.0);
    let reveal = reveal_sample * input.color;
    let diffuse = fma(vec3<f32>(amount), fma(heated, ember, -base), base) * input.color.rgb;
    let heat_emission = (heat * vec4<f32>(flicker, flicker, flicker, 1.0))
        * vec4<f32>(ember, flicker) * vec4<f32>(6.0, 6.0, 6.0, ember_sample.a) * reveal;
    let emission = (heat_emission.a * heat_emission.rgb) * material.source[3].x;
    return native_model_phong_shade(input, scene, view, scripts.vectors[0].x,
        material.source[0].xyz, false, true, diffuse * diffuse, 1.0,
        (emission * emission) * input.color.a, normal_map, normal_sampler,
        specular_map, specular_sampler, atlas, atlas_sampler, probe, probe_sampler);
}

// PS492a869b, VS84960e5a: emission samples the same authored ColorMap
// at a vertex-calculated depth-displaced UV. Its lighting omits the matrix.
fn native_character_glass(input: NativeScriptedVaryings, emission_uv: vec2<f32>,
    scene: NativeModelScene, view: NativeModelScriptedScene, material: NativeSourceCharacterConstants,
    normal_map: texture_2d<f32>, normal_sampler: sampler,
    color_map: texture_2d<f32>, color_sampler: sampler,
    specular_map: texture_2d<f32>, specular_sampler: sampler,
    atlas: texture_3d<f32>, atlas_sampler: sampler, probe: texture_cube<f32>, probe_sampler: sampler) -> vec4<f32> {
    let color = textureSample(color_map, color_sampler, input.uv) * input.color;
    let sample = textureSample(color_map, color_sampler, emission_uv);
    let emission = (sample.a * sample.rgb) * material.source[0].w;
    var lighting_input = input;
    lighting_input.color.a = 1.0;
    let result = native_model_phong_shade(lighting_input, scene, view, 0.0,
        vec3<f32>(material.source[0].xy, 1.0), false, false, color.rgb * color.rgb, 1.0,
        emission * emission, normal_map, normal_sampler, specular_map, specular_sampler,
        atlas, atlas_sampler, probe, probe_sampler);
    let alpha = sqrt(color.a);
    if !(alpha > 0.0) { discard; }
    return vec4<f32>(result.rgb, alpha);
}

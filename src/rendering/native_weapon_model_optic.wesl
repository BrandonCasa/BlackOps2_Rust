// mc_sw4_3d_optic_lens_4w47028j LIT PS9511e3ca; VS1a1ccc66.
import package::rendering::native_model::NativeModelScene;
import package::rendering::native_scripted_model::{NativeScriptedVaryings, NativeModelScriptedScene, native_scripted_bitangent};
import package::rendering::native_model_variants::native_model_atlas_irradiance;
fn native_optic_fragment(input: NativeScriptedVaryings, scene: NativeModelScene,
    view: NativeModelScriptedScene, controls: vec4<f32>,
    normal_map: texture_2d<f32>, normal_sampler: sampler,
    color_map: texture_2d<f32>, color_sampler: sampler,
    specular_map: texture_2d<f32>, specular_sampler: sampler,
    gloss_map: texture_2d<f32>, gloss_sampler: sampler,
    model_lighting: texture_3d<f32>, model_lighting_sampler: sampler,
    probe: texture_cube<f32>, probe_sampler: sampler) -> vec4<f32> {
    let eye_length = inverseSqrt(dot(input.eye_fog.xyz, input.eye_fog.xyz));
    let incident = input.eye_fog.xyz * eye_length;
    let half_raw = fma(-input.eye_fog.xyz, vec3<f32>(eye_length), view.sun_position.xyz);
    let half_vector = half_raw * inverseSqrt(dot(half_raw, half_raw));
    let geometric = input.normal * inverseSqrt(dot(input.normal, input.normal));
    let bitangent = native_scripted_bitangent(geometric, input.tangent_sign);
    let normal_sample = textureSample(normal_map, normal_sampler, input.uv);
    let slopes = fma(normal_sample.rg, vec2<f32>(4.015748023986816), vec2<f32>(-2.0157480239868164));
    let mapped = fma(vec3<f32>(slopes.y), bitangent, fma(vec3<f32>(slopes.x), input.tangent_sign.xyz, geometric));
    let normal = mapped * inverseSqrt(dot(mapped, mapped));
    let ndoth = clamp(dot(normal, half_vector), 0.0, 1.0);
    let hdotsun = clamp(dot(half_vector, view.sun_position.xyz), 0.0, 1.0);
    let gloss = textureSample(gloss_map, gloss_sampler, input.uv).r;
    let exponent = exp2(gloss * 13.0);
    let lobe = exp2(log2(ndoth) * exponent) * fma(exponent, 0.125, 0.25);
    let visibility = clamp(gloss + 0.5450000166893005, 0.0, 1.0);
    let denominator = fma(hdotsun * hdotsun, visibility, -visibility) + 1.0099999904632568;
    let attenuation = clamp(dot(view.sun_position.xyz, normal), 0.0, 1.0) / denominator;
    let specular_gamma = textureSample(specular_map, specular_sampler, input.uv).rgb;
    let specular = specular_gamma * specular_gamma;
    let fresnel = fma(fma(-specular_gamma, specular_gamma, vec3<f32>(1.0)), vec3<f32>(exp2(hdotsun * -10.0)), specular);
    let eye_facing = dot(geometric, -incident);
    let t = clamp((1.0 - eye_facing) * (1.0 / controls.z), 0.0, 1.0);
    let decay = (t * t) * fma(t, -2.0, 3.0);
    let amount = decay * controls.xy;
    let sun = input.grid_coords_and_vis.w * view.sun_diffuse.xyz;
    let direct = (((lobe * attenuation) * fresnel) * amount.x) * sun;
    let direction = fma(normal, vec3<f32>(-2.0 * dot(incident, normal)), incident);
    let coeff = fma(vec4<f32>(gloss), vec4<f32>(1.0416667461395264, 0.4749999940395355, 0.018229199573397636, 0.25), vec4<f32>(0.0, 0.0, -0.015625, 0.75));
    let sample = textureSampleLevel(probe, probe_sampler, direction, fma(-4.0, gloss, 4.0));
    let decoded = sample.rgb * (1.0 / (sample.a + 9.999999974752427e-07));
    let response = fma(coeff.x, min(exp2(clamp(dot(normal, -incident), 0.0, 1.0) * -9.279999732971191), coeff.y), coeff.z);
    let reflected = (decoded * clamp(fma(specular, vec3<f32>(coeff.w - response), vec3<f32>(response)), vec3<f32>(0.0), vec3<f32>(1.0))) * amount.y;
    let color = textureSample(color_map, color_sampler, input.uv) * input.color;
    let specular_lit = fma(reflected, input.reflection_correction, direct) / color.a;
    let irradiance = native_model_atlas_irradiance(normal, input.grid_coords_and_vis.xyz, scene, model_lighting, model_lighting_sampler) * 32.0;
    let light = fma(vec3<f32>(clamp(dot(view.sun_position.xyz, normal), 0.0, 1.0)), sun, irradiance);
    let lit = fma(color.rgb * color.rgb, light, specular_lit);
    let fogged = fma(vec3<f32>(input.eye_fog.w), lit - input.fog_color, input.fog_color);
    let alpha = sqrt(color.a);
    if !(alpha > 0.0) { discard; }
    return vec4<f32>(sqrt(fogged * scene.world.hdr_control0.x), alpha);
}

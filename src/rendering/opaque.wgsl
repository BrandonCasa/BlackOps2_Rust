// Project-owned opaque response. Bevy retains PBR lights, shadows and post processing.
#import bevy_pbr::{
    forward_io::{VertexOutput, FragmentOutput},
    pbr_fragment::pbr_input_from_standard_material,
    pbr_functions::{apply_pbr_lighting, main_pass_post_lighting_processing, calculate_tbn_mikktspace},
    lighting::F_AB,
    mesh_view_bindings::view,
}

struct SurfaceParameters {
    response: vec4<f32>,
    radiance: vec4<f32>,
    modes: vec4<u32>,
    lighting: vec4<f32>,
    sh0: vec4<f32>,
    sh1: vec4<f32>,
    sh2: vec4<f32>,
}
@group(#{MATERIAL_BIND_GROUP}) @binding(100) var<uniform> surface: SurfaceParameters;
@group(#{MATERIAL_BIND_GROUP}) @binding(101) var normal_map: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(102) var normal_sampler: sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(103) var second_normal: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(104) var second_normal_sampler: sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(105) var specular_map: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(106) var specular_sampler: sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(107) var second_specular: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(108) var second_specular_sampler: sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(109) var control_map: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(110) var control_sampler: sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(111) var packed_lightmap: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(112) var lightmap_sampler: sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(113) var reflection_map: texture_cube<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(114) var reflection_sampler: sampler;

fn normal_slope(sampled: vec2<f32>) -> vec2<f32> {
    // Raw BC5 source slope convention, with DirectX Y reversed for glTF tangents.
    return (sampled * 4.015748 - 2.015748) * vec2(1.0, -1.0) * surface.response.x;
}

fn source_basis(direction: vec3<f32>) -> vec3<f32> {
    // Preparation uses (x,z,-y); SH and cube faces retain source Z-up.
    return vec3(direction.x, -direction.z, direction.y);
}

fn probe_diffuse(normal: vec3<f32>) -> vec3<f32> {
    let n = source_basis(normal);
    let k = dot(surface.sh2, vec4(n.z*n.x, n.y*n.z, n.x*n.y, n.x*n.x-n.y*n.y))
        + dot(surface.sh1, vec4(n, 1.0)) + surface.sh0.w*n.z*n.z;
    return max(surface.sh0.rgb * k, vec3(0.1));
}

fn atlas_sample(uv: vec2<f32>, band: f32) -> vec4<f32> {
    // Clamp within each band to avoid bilinear bleeding into its neighbour.
    let dimensions = vec2<f32>(textureDimensions(packed_lightmap));
    let inset = vec2(0.5) / vec2(dimensions.x, dimensions.y / 3.0);
    let w = clamp(uv, inset, vec2(1.0) - inset);
    return textureSample(packed_lightmap, lightmap_sampler, vec2(w.x, (w.y+band)/3.0));
}

@fragment
fn fragment(in: VertexOutput, @builtin(front_facing) is_front: bool) -> FragmentOutput {
    var pbr = pbr_input_from_standard_material(in, is_front);
    var weights = vec2(1.0, 0.0);
    var detail_uv = in.uv;
    if (surface.modes.x & 1u) != 0u {
        let mask = textureSample(control_map, control_sampler, in.uv).rg;
        // Match diffuse bake: red=surface layer 1, green=core layer 0.
        weights = vec2(mask.g, mask.r) / max(mask.r+mask.g, 0.0001);
        detail_uv *= surface.response.w;
    }
    let slope0 = select(vec2(0.0), normal_slope(textureSample(normal_map, normal_sampler, detail_uv).rg), (surface.modes.x & 2u) != 0u);
    let slope1 = select(vec2(0.0), normal_slope(textureSample(second_normal, second_normal_sampler, detail_uv).rg), (surface.modes.x & 4u) != 0u);
#ifdef VERTEX_TANGENTS
    let tbn = calculate_tbn_mikktspace(pbr.world_normal, in.world_tangent);
    let slope = slope0*weights.x + slope1*weights.y;
    pbr.N = normalize(tbn * vec3(slope, 1.0));
#endif
    let sampled0 = textureSample(specular_map, specular_sampler, detail_uv);
    let sampled1 = textureSample(second_specular, second_specular_sampler, detail_uv);
    let s0 = select(vec4(0.5,0.5,0.5,0.36), sampled0, surface.modes.y != 0u);
    let s1 = select(s0, sampled1, surface.modes.z != 0u);
    let sg = s0*weights.x + s1*weights.y;
    let f0 = clamp(sg.rgb*sg.rgb*surface.response.z, vec3(0.0), vec3(0.16));
    pbr.material.reflectance = sqrt(f0/0.16);
    pbr.material.perceptual_roughness = pow(1.0-sg.a, surface.response.y);
    // No diffuse reblend: Bevy has already sampled the derived sRGB albedo and
    // multiplied the retained vertex color. Data samplers above are all linear.
    if surface.modes.w == 1u {
#ifdef VERTEX_UVS_B
        let ambient = atlas_sample(in.uv_b, 0.0);
        let directional = atlas_sample(in.uv_b, 1.0);
        let direction = atlas_sample(in.uv_b, 2.0).rgb*2.0-1.0;
        let world_direction = vec3(direction.x, direction.z, -direction.y);
        pbr.lightmap_light = (ambient.rgb/max(ambient.a, 1.0/255.0)
            + directional.rgb/max(directional.a, 1.0/255.0)*clamp(dot(world_direction,pbr.N),0.0,1.0))*surface.radiance.x;
#endif
    } else if surface.modes.w == 2u {
        pbr.lightmap_light = probe_diffuse(pbr.N)*surface.radiance.x;
    } else if surface.modes.w == 3u {
        // Retained instance color mean: linear irradiance, alpha is not opacity.
        pbr.lightmap_light = surface.lighting.rgb*surface.radiance.x;
    }
    var out: FragmentOutput;
    out.color = apply_pbr_lighting(pbr);
    if surface.lighting.w > 0.0 {
        let roughness = pbr.material.perceptual_roughness;
        let r = reflect(-pbr.V,pbr.N);
        let source = source_basis(r);
        // Retained DDS faces already use source sampling axes. Unlike a cube
        // authored for Bevy's world basis, this requires no additional Z flip.
        let radiance = textureSampleLevel(reflection_map, reflection_sampler,
            source, roughness*f32(textureNumLevels(reflection_map)-1u)).rgb;
        let fab = F_AB(roughness, max(dot(pbr.N,pbr.V),0.0001));
        let reflection_weight = (f0*fab.x + fab.y)*pbr.specular_occlusion;
        out.color = vec4(out.color.rgb + radiance*reflection_weight*surface.radiance.y*view.exposure, 1.0);
    }
    out.color = main_pass_post_lighting_processing(pbr, out.color);
    return out;
}

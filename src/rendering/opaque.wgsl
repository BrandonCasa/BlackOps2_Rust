// Project-owned opaque response. Bevy retains PBR lights, shadows and post processing.
#import bevy_pbr::{
    forward_io::{VertexOutput, FragmentOutput},
    pbr_fragment::pbr_input_from_standard_material,
    pbr_functions::{apply_pbr_lighting, main_pass_post_lighting_processing, calculate_tbn_mikktspace, alpha_discard},
    lighting::F_AB,
    mesh_view_bindings::view,
    pbr_types::{STANDARD_MATERIAL_FLAGS_ALPHA_MODE_PREMULTIPLIED, STANDARD_MATERIAL_FLAGS_ALPHA_MODE_RESERVED_BITS},
}

struct SurfaceParameters {
    response: vec4<f32>,
    radiance: vec4<f32>,
    modes: vec4<u32>,
    lighting: vec4<f32>,
    sh0: vec4<f32>,
    sh1: vec4<f32>,
    sh2: vec4<f32>,
    layers: vec4<u32>,
    reveal: vec4<f32>,
    wetness: vec4<f32>,
    water: vec4<f32>,
    velocity: vec4<f32>,
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
    return max(surface.sh0.rgb * k, vec3(0.0));
}

fn atlas_sample(uv: vec2<f32>, band: f32) -> vec4<f32> {
    // Clamp within each band to avoid bilinear bleeding into its neighbour.
    let dimensions = vec2<f32>(textureDimensions(packed_lightmap));
    let inset = vec2(0.5) / vec2(dimensions.x, dimensions.y / 3.0);
    let w = clamp(uv, inset, vec2(1.0) - inset);
    return textureSample(packed_lightmap, lightmap_sampler, vec2(w.x, (w.y+band)/3.0));
}

fn reveal(control: f32, alpha: f32, parameters: vec2<f32>) -> f32 {
    let c = clamp(control * 0.998 + 0.001, 0.001, 0.999);
    let low = clamp(1.0-c-parameters.x*pow(c,parameters.y),0.0,1.0);
    let high = clamp(1.0-c+parameters.x*pow(1.0-c,parameters.y),0.0,1.0);
    return clamp((alpha-low)/max(high-low,0.001),0.0,1.0);
}
fn orient(slope: vec2<f32>, rotation: vec4<f32>) -> vec2<f32> {
    let original = slope * vec2(1.0,-1.0);
    return vec2(dot(original,rotation.xy),dot(original,rotation.zw))*vec2(1.0,-1.0);
}
fn layer_color(base: vec3<f32>, overlay: vec4<f32>, weight: f32, operation: u32) -> vec3<f32> {
    if operation == 2u { return base*mix(vec3(1.0),overlay.rgb,weight); }
    if operation == 3u { return base+overlay.rgb*weight; }
    return mix(base,overlay.rgb,weight);
}
@fragment
fn fragment(in: VertexOutput, @builtin(front_facing) is_front: bool,
    @location(8) layer_uv: vec4<f32>, @location(9) layer3_uv: vec2<f32>,
    @location(10) rotation1: vec4<f32>, @location(11) rotation2: vec4<f32>,
    @location(12) baked_color: vec3<f32>) -> FragmentOutput {
    var standard = in;
    let layered = surface.layers.x > 1u;
    if layered { standard.color = vec4(1.0); }
    var pbr = pbr_input_from_standard_material(standard, is_front);
    if (pbr.material.flags & STANDARD_MATERIAL_FLAGS_ALPHA_MODE_RESERVED_BITS) == STANDARD_MATERIAL_FLAGS_ALPHA_MODE_PREMULTIPLIED {
        pbr.material.base_color.a *= surface.wetness.w;
    }
    var weights = vec3(1.0,0.0,0.0);
    var uv0 = in.uv;
    var uv1 = in.uv;
    var uv2 = in.uv;
    let water = (surface.modes.x & 16u) != 0u;
    let tile_macro = (surface.modes.x & 1u) != 0u;
    if tile_macro {
        let mask = textureSample(control_map, control_sampler, in.uv).rg;
        weights = vec3(mask.g,mask.r,0.0)/max(mask.r+mask.g,0.0001);
        // Normal 0 describes the whole object's shape at authored UVs.
        // Normals 1/2 accompany coating/core (color 1/0), respectively.
        uv1 = in.uv * surface.response.w;
    }
    var slope0 = select(vec2(0.0), normal_slope(textureSample(normal_map, normal_sampler, uv0).rg), (surface.modes.x & 2u) != 0u);
    var slope1 = select(vec2(0.0), normal_slope(textureSample(second_normal, second_normal_sampler, uv1).rg), (surface.modes.x & 4u) != 0u);
    var slope2 = vec2(0.0);
    var s0 = select(vec4(0.5,0.5,0.5,0.36), textureSample(specular_map,specular_sampler,uv0), surface.modes.y != 0u);
    var s1 = select(s0, textureSample(second_specular,second_specular_sampler,uv1), (surface.modes.z & 1u) != 0u);
    var s2 = s0;
    if layered {
        uv1 = layer_uv.xy;
        uv2 = layer_uv.zw;
        let c0 = textureSample(color0,color0_sampler,uv0);
        let c1 = textureSample(color1,color1_sampler,uv1);
        let c2 = textureSample(color2,color2_sampler,uv2);
        let c3 = textureSample(color3,color3_sampler,layer3_uv);
        // Authored vertex G/B control consecutive overlays. Alpha is the retained
        // texture reveal/macro mask, sampled as data by the sRGB texture's alpha.
        let w1 = reveal(in.color.g,c1.a,surface.reveal.xy);
        let w2 = select(0.0,reveal(in.color.b,c2.a,surface.reveal.zw),surface.layers.x > 2u);
        var albedo = layer_color(c0.rgb,c1,w1,surface.layers.y);
        if surface.layers.x > 2u { albedo = layer_color(albedo,c2,w2,surface.layers.z); }
        if surface.layers.x > 3u { albedo = layer_color(albedo,c3,c3.a,surface.layers.w); }
        pbr.material.base_color = vec4(albedo,1.0);
        let response1 = select(0.0,w1,surface.layers.y == 1u);
        let response2 = select(0.0,w2,surface.layers.z == 1u);
        weights = vec3((1.0-response1)*(1.0-response2),response1*(1.0-response2),response2);
        slope1 = select(slope0,orient(normal_slope(textureSample(second_normal,second_normal_sampler,uv1).rg),rotation1),(surface.modes.x & 4u) != 0u);
        slope2 = select(slope0,orient(normal_slope(textureSample(third_normal,third_normal_sampler,uv2).rg),rotation2),(surface.modes.x & 8u) != 0u);
        s1 = select(s0,textureSample(second_specular,second_specular_sampler,uv1),(surface.modes.z & 1u) != 0u);
        s2 = select(s0,textureSample(third_specular,third_specular_sampler,uv2),(surface.modes.z & 2u) != 0u);
    }
    var slope = slope0*weights.x+slope1*weights.y+slope2*weights.z;
    var sg = s0*weights.x+s1*weights.y+s2*weights.z;
    if tile_macro {
        let coating = select(vec2(0.0),normal_slope(textureSample(second_normal,second_normal_sampler,uv1).rg),(surface.modes.x & 4u) != 0u);
        let core = select(vec2(0.0),normal_slope(textureSample(third_normal,third_normal_sampler,uv1).rg),(surface.modes.x & 8u) != 0u);
        // Blend detail slopes onto the unscaled macro normal. The shared diffuse
        // bake already includes this mask's occlusion, so do not multiply it again.
        slope = slope0 + core*weights.x + coating*weights.y;
        s0 = select(vec4(0.5,0.5,0.5,0.36),textureSample(specular_map,specular_sampler,uv1),surface.modes.y != 0u);
        s1 = select(s0,textureSample(second_specular,second_specular_sampler,uv1),(surface.modes.z & 1u) != 0u);
        // Specular bindings follow coating/core order, like detail normals.
        sg = s0*weights.y+s1*weights.x;
    }
    if water {
        let t = surface.radiance.z;
        let wuv = in.uv*surface.water.z;
        let a = normal_slope(textureSample(normal_map,normal_sampler,wuv+t*surface.velocity.xy).rg);
        let b = normal_slope(textureSample(second_normal,second_normal_sampler,wuv*1.37+t*surface.velocity.zw).rg);
        slope = (a+b)*surface.water.x;
        sg = vec4(vec3(sqrt(0.02/surface.response.z)),1.0-surface.water.y*surface.water.y);
        // Transparent tint approximates shallow refraction; no opaque scene copy.
        pbr.material.base_color.a = surface.water.w;
    }
#ifdef VERTEX_TANGENTS
    let tbn = calculate_tbn_mikktspace(pbr.world_normal, in.world_tangent);
    pbr.N = normalize(tbn * vec3(slope, 1.0));
#endif
    var f0 = clamp(sg.rgb*sg.rgb*surface.response.z,vec3(0.0),vec3(0.16));
    let patches = 0.65+0.35*sin(in.world_position.x*0.71+sin(in.world_position.z*0.39));
    let wet = surface.wetness.x * smoothstep(0.35,0.85,pbr.world_normal.y) * patches;
    pbr.material.base_color = vec4(pbr.material.base_color.rgb*(1.0-wet*surface.wetness.y),pbr.material.base_color.a);
    f0 = mix(f0,vec3(0.04),wet);
    pbr.material.reflectance = sqrt(f0/0.16);
    pbr.material.perceptual_roughness = mix(pow(clamp(1.0-sg.a,0.0,1.0),surface.response.y),surface.wetness.z,wet);
    pbr.material.base_color = alpha_discard(pbr.material,pbr.material.base_color);
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
        // Interpolated retained instance irradiance; alpha is not surface opacity.
        pbr.lightmap_light = (baked_color + probe_diffuse(pbr.N)*surface.radiance.w)*surface.radiance.x;
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
        out.color = vec4(out.color.rgb + radiance*reflection_weight*surface.radiance.y*view.exposure, out.color.a);
    }
    out.color = main_pass_post_lighting_processing(pbr, out.color);
    // Source ONE/INV_SRC_ALPHA describes the blend state, not premultiplied DDS
    // albedo. Our freshly evaluated lighting must supply premultiplied radiance.
    if (pbr.material.flags & STANDARD_MATERIAL_FLAGS_ALPHA_MODE_RESERVED_BITS) == STANDARD_MATERIAL_FLAGS_ALPHA_MODE_PREMULTIPLIED {
        out.color = vec4(out.color.rgb*out.color.a, out.color.a);
    }
    return out;
}

@group(#{MATERIAL_BIND_GROUP}) @binding(115) var color0: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(116) var color0_sampler: sampler;

@group(#{MATERIAL_BIND_GROUP}) @binding(117) var color1: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(118) var color1_sampler: sampler;

@group(#{MATERIAL_BIND_GROUP}) @binding(119) var color2: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(120) var color2_sampler: sampler;

@group(#{MATERIAL_BIND_GROUP}) @binding(121) var color3: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(122) var color3_sampler: sampler;

@group(#{MATERIAL_BIND_GROUP}) @binding(123) var third_normal: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(124) var third_normal_sampler: sampler;

@group(#{MATERIAL_BIND_GROUP}) @binding(125) var third_specular: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(126) var third_specular_sampler: sampler;

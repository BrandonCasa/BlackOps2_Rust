// Project-owned opaque response. Bevy retains PBR lights, shadows and post processing.
#import bevy_pbr::{
    forward_io::{VertexOutput, FragmentOutput},
    pbr_fragment::pbr_input_from_standard_material,
    pbr_functions::{main_pass_post_lighting_processing, calculate_tbn_mikktspace, alpha_discard},
    mesh_view_bindings::view,
    pbr_types::{STANDARD_MATERIAL_FLAGS_ALPHA_MODE_PREMULTIPLIED, STANDARD_MATERIAL_FLAGS_ALPHA_MODE_RESERVED_BITS, STANDARD_MATERIAL_FLAGS_UNLIT_BIT, STANDARD_MATERIAL_FLAGS_DOUBLE_SIDED_BIT},
}
#import origins::surface_lighting::apply_pbr_lighting

#import origins::surface_bindings::{OriginsSurfaceData, animation_time, diagnostic_mode, set_surface_slot, parameters_for, sample_normal_map, sample_second_normal, sample_specular_map, sample_second_specular, sample_control_map, sample_color_base, sample_color_first, sample_color_second, sample_color_third, sample_third_normal, sample_third_specular}
#import bevy_pbr::mesh_bindings::mesh
#import origins::surface_effect::{effect_uv, effect_color}

fn effect_sample(slot: u32, layer: u32, uv: vec2<f32>) -> vec4<f32> {
    if layer == 0u { return sample_color_base(slot,uv); }
    if layer == 1u { return sample_color_first(slot,uv); }
    return sample_color_second(slot,uv);
}

fn normal_slope(sampled: vec4<f32>, strength: f32, two_component: bool) -> vec2<f32> {
    // Offline preparation has already decoded native slopes and changed Y.
    let xy = sampled.rg * 2.0 - 1.0;
    let z = select(sqrt(max(1.0-dot(xy,xy),0.0001)), sampled.b*2.0-1.0, !two_component);
    return xy/max(z,0.01)*strength;
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
    @location(10) rotation_first: vec4<f32>, @location(11) rotation_second: vec4<f32>) -> FragmentOutput {
    #ifdef BINDLESS
    let slot = mesh[in.instance_index].material_and_lightmap_bind_group_slot & 0xffffu;
#else
    let slot = 0u;
#endif
    let surface = parameters_for(slot);
    set_surface_slot(slot);
    var standard = in;
    let layered = surface.layers.x > 1u;
    standard.color = vec4(1.0); // Authored vertex G/B remain layer controls only.
    var pbr = pbr_input_from_standard_material(standard, is_front);
    let face_sign = select(1.0, -1.0, !is_front && (pbr.material.flags & STANDARD_MATERIAL_FLAGS_DOUBLE_SIDED_BIT) != 0u);
    // Bevy conditionally flips its prepared normal when no base normal map is
    // bound. Our layered map uses the original interpolated frame, then orients
    // the entire resolved normal once, independently of that base shader flag.
    pbr.world_normal = in.world_normal * face_sign;
    pbr.N = normalize(pbr.world_normal);
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
        let mask = sample_control_map(slot, in.uv).rg;
        weights = select(vec3(1.0,0.0,0.0),vec3(mask.g,mask.r,0.0)/max(mask.r+mask.g,0.0001),mask.r+mask.g > 0.0001);
        // Normal 0 describes the whole object's shape at authored UVs.
        // Normals 1/2 accompany coating/core (color 1/0), respectively.
        uv1 = in.uv * surface.response.w;
        // Control magnitudes used to include macro darkness. Only normalized
        // material coverage survives; scene illumination is evaluated now.
        pbr.material.base_color = vec4(sample_color_base(slot, uv1).rgb*weights.x + sample_color_first(slot, uv1).rgb*weights.y, 1.0);
    }
    var slope0 = select(vec2(0.0), normal_slope(sample_normal_map(slot, uv0), surface.response.x, (surface.modes.w & 1u) != 0u), (surface.modes.x & 2u) != 0u);
    var slope1 = select(vec2(0.0), normal_slope(sample_second_normal(slot, uv1), surface.response.x, (surface.modes.w & 2u) != 0u), (surface.modes.x & 4u) != 0u);
    var slope2 = vec2(0.0);
    var s0 = select(vec4(1.0,0.8,0.0,0.0), sample_specular_map(slot, uv0), surface.modes.y != 0u);
    var s1 = select(s0, sample_second_specular(slot, uv1), (surface.modes.z & 1u) != 0u);
    var s2 = s0;
    let effect = (surface.modes.x & 32u) != 0u;
    if layered && !effect {
        uv1 = layer_uv.xy;
        uv2 = layer_uv.zw;
        let c0 = sample_color_base(slot, uv0);
        let c1 = sample_color_first(slot, uv1);
        let c2 = sample_color_second(slot, uv2);
        let c3 = sample_color_third(slot, layer3_uv);
        // Authored vertex G/B control consecutive overlays. Alpha is the retained
        // texture reveal/macro mask, sampled as data by the sRGB texture's alpha.
        // Reveal controls coverage, but cannot replace texture opacity. Blood,
        // grime and other overlays retain their soft alpha even at full paint.
        // The same effective coverage blends their normal/specular response.
        let w1 = reveal(in.color.g,c1.a,surface.reveal.xy)*clamp(c1.a,0.0,1.0);
        let w2 = select(0.0,reveal(in.color.b,c2.a,surface.reveal.zw)*clamp(c2.a,0.0,1.0),surface.layers.x > 2u);
        var albedo = layer_color(c0.rgb,c1,w1,surface.layers.y);
        if surface.layers.x > 2u { albedo = layer_color(albedo,c2,w2,surface.layers.z); }
        if surface.layers.x > 3u { albedo = layer_color(albedo,c3,c3.a,surface.layers.w); }
        pbr.material.base_color = vec4(albedo,1.0);
        let response1 = select(0.0,w1,surface.layers.y == 1u);
        let response2 = select(0.0,w2,surface.layers.z == 1u);
        weights = vec3((1.0-response1)*(1.0-response2),response1*(1.0-response2),response2);
        slope1 = select(slope0,orient(normal_slope(sample_second_normal(slot, uv1), surface.response.x, (surface.modes.w & 2u) != 0u),rotation_first),(surface.modes.x & 4u) != 0u);
        slope2 = select(slope0,orient(normal_slope(sample_third_normal(slot, uv2), surface.response.x, (surface.modes.w & 4u) != 0u),rotation_second),(surface.modes.x & 8u) != 0u);
        s1 = select(s0,sample_second_specular(slot, uv1),(surface.modes.z & 1u) != 0u);
        s2 = select(s0,sample_third_specular(slot, uv2),(surface.modes.z & 2u) != 0u);
    }
    var slope = slope0*weights.x+slope1*weights.y+slope2*weights.z;
    var sg = s0*weights.x+s1*weights.y+s2*weights.z;
    if tile_macro {
        let coating = select(vec2(0.0),normal_slope(sample_second_normal(slot, uv1), surface.response.x, (surface.modes.w & 2u) != 0u),(surface.modes.x & 4u) != 0u);
        let core = select(vec2(0.0),normal_slope(sample_third_normal(slot, uv1), surface.response.x, (surface.modes.w & 4u) != 0u),(surface.modes.x & 8u) != 0u);
        // Blend detail slopes onto the unscaled macro normal.
        slope = slope0 + core*weights.x + coating*weights.y;
        s0 = select(vec4(1.0,0.8,0.0,0.0),sample_specular_map(slot, uv1),surface.modes.y != 0u);
        s1 = select(s0,sample_second_specular(slot, uv1),(surface.modes.z & 1u) != 0u);
        // Prepared response layers follow color layers (core/coating); their
        // normals retain the separate macro/coating/core native binding order.
        sg = s0*weights.x+s1*weights.y;
    }
    if water {
        let t = animation_time(slot);
        let wuv = in.uv*surface.water.z;
        let a = normal_slope(sample_normal_map(slot, wuv+t*surface.velocity.xy), surface.response.x, (surface.modes.w & 1u) != 0u);
        let b = normal_slope(sample_second_normal(slot, wuv*1.37+t*surface.velocity.zw), surface.response.x, (surface.modes.w & 2u) != 0u);
        slope = (a+b)*surface.water.x;
        sg = vec4(1.0,surface.water.y,0.0,0.0);
        // Transparent tint approximates shallow refraction; no opaque scene copy.
        pbr.material.base_color.a = surface.water.w;
    }
    if effect {
        let normal = normalize(pbr.world_normal);
        let tangent = normalize(in.world_tangent.xyz);
        let bitangent = cross(normal,tangent)*in.world_tangent.w;
        let facing = max(dot(pbr.V,normal),0.0);
        let parallax = -vec2(dot(pbr.V,tangent),dot(pbr.V,bitangent))/max(facing,0.001);
        var samples: array<vec4<f32>,3>;
        var coverage = vec3(0.0);
        for (var layer = 0u; layer < 3u; layer++) {
            let uv = effect_uv(in.uv,layer,surface.effect,animation_time(slot),parallax);
            samples[layer] = effect_sample(slot,layer,uv);
            coverage[layer] = effect_sample(slot,surface.effect.coverage_layers[layer],uv).a;
        }
        let resolved = effect_color(samples,coverage,surface.effect,facing);
        pbr.material.base_color = vec4(resolved.rgb*in.color.rgb,resolved.a*in.color.a);
        // Opaque source effects write full coverage; blended variants retain
        // their native accumulated alpha and source blend/cull disposition.
        if (pbr.material.flags & STANDARD_MATERIAL_FLAGS_ALPHA_MODE_RESERVED_BITS) == 0u {
            pbr.material.base_color.a = 1.0;
        }
    }
#ifdef VERTEX_TANGENTS
    let tbn = calculate_tbn_mikktspace(in.world_normal, in.world_tangent);
    pbr.N = normalize(tbn * vec3(slope, 1.0)) * face_sign;
#endif
    pbr.material.reflectance = select(vec3(0.5),vec3(sqrt(0.02/0.16)),water);
    pbr.material.perceptual_roughness = clamp(sg.g,0.089,1.0);
    pbr.material.metallic = clamp(sg.b,0.0,1.0);
    pbr.material.clearcoat = clamp(sg.a,0.0,1.0);
    if (surface.modes.x & 128u) != 0u {
        // Actor conversion removes the StandardMaterial response binding;
        // preserve authored cavity occlusion from the already sampled map.
        let cavity = clamp(sg.r,0.0,1.0);
        pbr.diffuse_occlusion *= vec3(cavity);
        pbr.specular_occlusion *= cavity;
    }
    pbr.material.clearcoat_perceptual_roughness = surface.response.y;
    pbr.clearcoat_N = normalize(pbr.world_normal);
    pbr.material.base_color = alpha_discard(pbr.material,pbr.material.base_color);
    var out: FragmentOutput;
    if (pbr.material.flags & STANDARD_MATERIAL_FLAGS_UNLIT_BIT) != 0u {
        out.color = pbr.material.base_color;
    } else {
        out.color = apply_pbr_lighting(pbr);
    }
    // The diagnostic adapter isolates the same realtime IBL used by Bevy.
    let view_mode = diagnostic_mode(slot);
    if view_mode == 6u || view_mode >= 8u {
        if (pbr.material.flags & STANDARD_MATERIAL_FLAGS_UNLIT_BIT) != 0u {
            out.color = vec4(0.0,0.0,0.0,1.0);
        }
        return out;
    }
    if view_mode != 0u {
        var debug = vec3(pbr.material.perceptual_roughness);
        if view_mode == 1u { debug = fract(vec3(surface.response.z)*vec3(0.618034,0.414214,0.732051)); }
        if view_mode == 3u { debug = vec3(pbr.material.metallic); }
        if view_mode == 4u { debug = pbr.N*0.5+0.5; }
        if view_mode == 7u { debug = normalize(pbr.world_normal)*0.5+0.5; }
        if view_mode == 5u { debug = vec3(pbr.material.base_color.a); }
        out.color = vec4(debug,1.0);
        return out;
    }
    out.color = main_pass_post_lighting_processing(pbr, out.color);
    // Source ONE/INV_SRC_ALPHA describes the blend state, not premultiplied DDS
    // albedo. Our freshly evaluated lighting must supply premultiplied radiance.
    if (pbr.material.flags & STANDARD_MATERIAL_FLAGS_ALPHA_MODE_RESERVED_BITS) == STANDARD_MATERIAL_FLAGS_ALPHA_MODE_PREMULTIPLIED {
        out.color = vec4(out.color.rgb*out.color.a, out.color.a);
    }
    return out;
}

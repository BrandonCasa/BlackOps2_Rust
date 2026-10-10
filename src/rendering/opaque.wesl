@if(VISIBILITY_RANGE_DITHER)
import bevy_pbr::render::pbr_functions::visibility_range_dither;

import bevy_pbr::render::{
forward_io::{VertexOutput, FragmentOutput},
pbr_fragment::pbr_input_from_standard_material,
pbr_functions::{main_pass_post_lighting_processing, calculate_tbn_mikktspace, alpha_discard},
mesh_view_bindings::view,
pbr_types::{STANDARD_MATERIAL_FLAGS_ALPHA_MODE_PREMULTIPLIED, STANDARD_MATERIAL_FLAGS_ALPHA_MODE_RESERVED_BITS, STANDARD_MATERIAL_FLAGS_UNLIT_BIT, STANDARD_MATERIAL_FLAGS_DOUBLE_SIDED_BIT},
};
@if(ORIGINS_DIAGNOSTIC) import origins::surface_lighting::apply_pbr_lighting;
@if(!ORIGINS_DIAGNOSTIC) import bevy_pbr::render::pbr_functions::apply_pbr_lighting;
import origins::surface_bindings::{OriginsSurfaceData, animation_time, diagnostic_mode, set_surface_slot, parameters_for, sample_normal_map, sample_second_normal, sample_specular_map, sample_second_specular, sample_control_map, sample_third_normal, sample_third_specular, sample_color_base_grad, sample_color_first_grad, sample_color_second_grad, sample_color_third_grad};
import bevy_pbr::render::mesh_bindings::mesh;
import origins::surface_effect::{effect_uv, effect_color};
import origins::native_weapon::native_fragment;



fn effect_sample(slot: u32, layer: u32, uv: vec2<f32>, gradient: mat2x2<f32>) -> vec4<f32> {
    if layer == 0u { return sample_color_base_grad(slot,uv,gradient); }
    if layer == 1u { return sample_color_first_grad(slot,uv,gradient); }
    return sample_color_second_grad(slot,uv,gradient);
}

fn normal_slope(sampled: vec4<f32>, strength: f32, two_component: bool) -> vec2<f32> {
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
    let gradient0 = mat2x2(dpdx(in.uv), dpdy(in.uv));
    let layer_gradient1 = mat2x2(dpdx(layer_uv.xy), dpdy(layer_uv.xy));
    let layer_gradient2 = mat2x2(dpdx(layer_uv.zw), dpdy(layer_uv.zw));
    let layer_gradient3 = mat2x2(dpdx(layer3_uv), dpdy(layer3_uv));
    @if(VISIBILITY_RANGE_DITHER) visibility_range_dither(in.position, in.visibility_range_dither);
    @if(BINDLESS) let slot = mesh[in.instance_index].material_and_lightmap_bind_group_slot & 0xffffu;
    @if(!BINDLESS) let slot = 0u;
    let surface = parameters_for(slot);
    @if(ORIGINS_DIAGNOSTIC) set_surface_slot(slot);
    var standard = in;
    let layered = surface.layers.x > 1u;
    standard.color = vec4(1.0);
    var pbr = pbr_input_from_standard_material(standard, is_front);
    if surface.native.kind.x != 0u {
        set_surface_slot(slot);
        return native_fragment(in, is_front, slot, surface, pbr);
    }
    let face_sign = select(1.0, -1.0, !is_front && (pbr.material.flags & STANDARD_MATERIAL_FLAGS_DOUBLE_SIDED_BIT) != 0u);
    pbr.world_normal = in.world_normal * face_sign;
    pbr.N = normalize(pbr.world_normal);
    if (pbr.material.flags & STANDARD_MATERIAL_FLAGS_ALPHA_MODE_RESERVED_BITS) == STANDARD_MATERIAL_FLAGS_ALPHA_MODE_PREMULTIPLIED {
        pbr.material.base_color.a *= surface.wetness.w;
    }
    let unlit = (pbr.material.flags & STANDARD_MATERIAL_FLAGS_UNLIT_BIT) != 0u;
    @if(ORIGINS_DIAGNOSTIC) let needs_surface = true;
    @if(!ORIGINS_DIAGNOSTIC) let needs_surface = !unlit;
    var weights = vec3(1.0,0.0,0.0);
    let uv0 = in.uv;
    var uv1 = in.uv;
    var gradient1 = gradient0;
    let water = (surface.modes.x & 16u) != 0u;
    let tile_macro = (surface.modes.x & 1u) != 0u;
    let effect = (surface.modes.x & 32u) != 0u;
    if tile_macro {
        let mask = sample_control_map(slot, in.uv, gradient0).rg;
        weights = select(vec3(1.0,0.0,0.0),vec3(mask.g,mask.r,0.0)/max(mask.r+mask.g,0.0001),mask.r+mask.g > 0.0001);
        uv1 = in.uv * surface.response.w;
        gradient1 = gradient0 * surface.response.w;
        if !layered && !effect {
            pbr.material.base_color = vec4(sample_color_base_grad(slot, uv1, gradient1).rgb*weights.x + sample_color_first_grad(slot, uv1, gradient1).rgb*weights.y, 1.0);
        }
    }
    var slope0 = vec2(0.0);
    var slope1 = vec2(0.0);
    var slope2 = vec2(0.0);
    var s0 = vec4(1.0,0.8,0.0,0.0);
    var s1 = s0;
    var s2 = s0;
    if needs_surface && !water {
        if (surface.modes.x & 2u) != 0u {
            slope0 = normal_slope(sample_normal_map(slot, uv0, gradient0), surface.response.x, (surface.modes.w & 1u) != 0u);
        }
        if !tile_macro && surface.modes.y != 0u {
            s0 = sample_specular_map(slot, uv0, gradient0);
        }
        s1 = s0;
        s2 = s0;
    }
    if layered && !effect {
        uv1 = layer_uv.xy;
        gradient1 = layer_gradient1;
        let c0 = sample_color_base_grad(slot, uv0, gradient0);
        let c1 = sample_color_first_grad(slot, uv1, gradient1);
        let w1 = reveal(in.color.g,c1.a,surface.reveal.xy)*clamp(c1.a,0.0,1.0);
        var w2 = 0.0;
        var albedo = layer_color(c0.rgb,c1,w1,surface.layers.y);
        if surface.layers.x > 2u {
            let c2 = sample_color_second_grad(slot, layer_uv.zw, layer_gradient2);
            w2 = reveal(in.color.b,c2.a,surface.reveal.zw)*clamp(c2.a,0.0,1.0);
            albedo = layer_color(albedo,c2,w2,surface.layers.z);
        }
        if surface.layers.x > 3u {
            let c3 = sample_color_third_grad(slot, layer3_uv, layer_gradient3);
            albedo = layer_color(albedo,c3,c3.a,surface.layers.w);
        }
        pbr.material.base_color = vec4(albedo,1.0);
        let response1 = select(0.0,w1,surface.layers.y == 1u);
        let response2 = select(0.0,w2,surface.layers.z == 1u);
        weights = vec3((1.0-response1)*(1.0-response2),response1*(1.0-response2),response2);
        if needs_surface && !water && !tile_macro {
            slope1 = slope0;
            if (surface.modes.x & 4u) != 0u {
                slope1 = orient(normal_slope(sample_second_normal(slot, uv1, gradient1), surface.response.x, (surface.modes.w & 2u) != 0u),rotation_first);
            }
            if (surface.modes.z & 1u) != 0u {
                s1 = sample_second_specular(slot, uv1, gradient1);
            }
            if surface.layers.x > 2u {
                slope2 = slope0;
                if (surface.modes.x & 8u) != 0u {
                    slope2 = orient(normal_slope(sample_third_normal(slot, layer_uv.zw, layer_gradient2), surface.response.x, (surface.modes.w & 4u) != 0u),rotation_second);
                }
                if (surface.modes.z & 2u) != 0u {
                    s2 = sample_third_specular(slot, layer_uv.zw, layer_gradient2);
                }
            }
        }
    }
    var slope = slope0*weights.x+slope1*weights.y+slope2*weights.z;
    var sg = s0*weights.x+s1*weights.y+s2*weights.z;
    if tile_macro && needs_surface && !water {
        var coating = vec2(0.0);
        var core = vec2(0.0);
        if (surface.modes.x & 4u) != 0u {
            coating = normal_slope(sample_second_normal(slot, uv1, gradient1), surface.response.x, (surface.modes.w & 2u) != 0u);
        }
        if (surface.modes.x & 8u) != 0u {
            core = normal_slope(sample_third_normal(slot, uv1, gradient1), surface.response.x, (surface.modes.w & 4u) != 0u);
        }
        slope = slope0 + core*weights.x + coating*weights.y;
        if surface.modes.y != 0u {
            s0 = sample_specular_map(slot, uv1, gradient1);
        }
        s1 = s0;
        if (surface.modes.z & 1u) != 0u {
            s1 = sample_second_specular(slot, uv1, gradient1);
        }
        sg = s0*weights.x+s1*weights.y;
    }
    if water {
        if needs_surface {
            let t = animation_time(slot);
            let wuv = in.uv*surface.water.z;
            let a = normal_slope(sample_normal_map(slot, wuv+t*surface.velocity.xy, gradient0*surface.water.z), surface.response.x, (surface.modes.w & 1u) != 0u);
            let b = normal_slope(sample_second_normal(slot, wuv*1.37+t*surface.velocity.zw, gradient0*(surface.water.z*1.37)), surface.response.x, (surface.modes.w & 2u) != 0u);
            slope = (a+b)*surface.water.x;
            sg = vec4(1.0,surface.water.y,0.0,0.0);
        }
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
            let gradient = mat2x2(dpdx(uv),dpdy(uv));
            samples[layer] = effect_sample(slot,layer,uv,gradient);
            coverage[layer] = samples[layer].a;
            if surface.effect.coverage_layers[layer] != layer {
                coverage[layer] = effect_sample(slot,surface.effect.coverage_layers[layer],uv,gradient).a;
            }
        }
        let resolved = effect_color(samples,coverage,surface.effect,facing);
        pbr.material.base_color = vec4(resolved.rgb*in.color.rgb,resolved.a*in.color.a);
        if (pbr.material.flags & STANDARD_MATERIAL_FLAGS_ALPHA_MODE_RESERVED_BITS) == 0u {
            pbr.material.base_color.a = 1.0;
        }
    }
    @if(VERTEX_TANGENTS) let tbn = calculate_tbn_mikktspace(in.world_normal, in.world_tangent);
    @if(VERTEX_TANGENTS) pbr.N = normalize(tbn * vec3(slope, 1.0)) * face_sign;
    pbr.material.reflectance = select(vec3(0.5),vec3(sqrt(0.02/0.16)),water);
    pbr.material.perceptual_roughness = clamp(sg.g,0.089,1.0);
    pbr.material.metallic = clamp(sg.b,0.0,1.0);
    pbr.material.clearcoat = clamp(sg.a,0.0,1.0);
    if (surface.modes.x & 128u) != 0u {
        let cavity = clamp(sg.r,0.0,1.0);
        pbr.diffuse_occlusion *= vec3(cavity);
        pbr.specular_occlusion *= cavity;
    }
    pbr.material.clearcoat_perceptual_roughness = surface.response.y;
    pbr.clearcoat_N = normalize(pbr.world_normal);
    pbr.material.base_color = alpha_discard(pbr.material.flags,pbr.material.alpha_cutoff,pbr.material.base_color);
    var out: FragmentOutput;
    if (pbr.material.flags & STANDARD_MATERIAL_FLAGS_UNLIT_BIT) != 0u {
        out.color = pbr.material.base_color;
    } else {
        out.color = apply_pbr_lighting(pbr);
    }
    @if(ORIGINS_DIAGNOSTIC) {
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
    }
    out.color = main_pass_post_lighting_processing(pbr, out.color);
    if (pbr.material.flags & STANDARD_MATERIAL_FLAGS_ALPHA_MODE_RESERVED_BITS) == STANDARD_MATERIAL_FLAGS_ALPHA_MODE_PREMULTIPLIED {
        out.color = vec4(out.color.rgb*out.color.a, out.color.a);
    }
    return out;
}

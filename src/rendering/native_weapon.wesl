import bevy_pbr::render::{forward_io::{VertexOutput, FragmentOutput}, pbr_types::{PbrInput, STANDARD_MATERIAL_FLAGS_DOUBLE_SIDED_BIT, STANDARD_MATERIAL_FLAGS_ALPHA_MODE_PREMULTIPLIED, STANDARD_MATERIAL_FLAGS_ALPHA_MODE_RESERVED_BITS}, pbr_functions::{calculate_tbn_mikktspace, alpha_discard, main_pass_post_lighting_processing}};
import origins::surface_bindings::{OriginsSurfaceData, NativeWeaponParameters, animation_time, diagnostic_mode, sample_normal_map, sample_second_normal, sample_specular_map, sample_second_specular, sample_control_map, sample_third_specular, sample_color_base_grad, sample_color_first_grad, sample_color_second_grad, sample_color_third_grad};
@if(ORIGINS_DIAGNOSTIC) import origins::surface_lighting::apply_pbr_lighting;
@if(!ORIGINS_DIAGNOSTIC) import bevy_pbr::render::pbr_functions::apply_pbr_lighting;

const LEGACY_DIELECTRIC_F0: f32 = 0.0482276048;

fn perceived_brightness(color: vec3<f32>) -> f32 {
    return sqrt(dot(color*color,vec3(0.299,0.587,0.114)));
}
fn legacy_metallic(diffuse: vec3<f32>, specular: vec3<f32>) -> f32 {
    let brightness = perceived_brightness(specular);
    if brightness <= LEGACY_DIELECTRIC_F0 { return 0.0; }
    let remaining = 1.0-max(max(specular.r,specular.g),specular.b);
    let b = perceived_brightness(diffuse)*remaining/(1.0-LEGACY_DIELECTRIC_F0)+brightness-2.0*LEGACY_DIELECTRIC_F0;
    let c = LEGACY_DIELECTRIC_F0-brightness;
    return clamp((-b+sqrt(max(b*b-4.0*LEGACY_DIELECTRIC_F0*c,0.0)))/(2.0*LEGACY_DIELECTRIC_F0),0.0,1.0);
}

fn native_slope(sampled: vec4<f32>) -> vec2<f32> {
    return (sampled.rg*4.015748-vec2(2.015748))*vec2(1.0,-1.0);
}
fn native_layer_uv(uv: vec2<f32>, layer: u32, native: NativeWeaponParameters, time: f32, parallax: vec2<f32>) -> vec2<f32> {
    let so = native.layer_scale_offset[layer];
    let sr = native.layer_scroll_rotation[layer];
    let od = native.layer_origin_depth[layer];
    let angle = sr.z*(1.0+sr.w*time)*6.283185;
    let cs = cos(angle);
    let sn = sin(angle);
    let v = (uv-od.xy+so.zw+parallax*od.z)*so.xy;
    var result = vec2(cs*v.x+sn*v.y,-sn*v.x+cs*v.y)+time*sr.xy*so.xy+od.xy;
    if native.kind.x == 10u { result *= native.camo.xy; }
    return result;
}
fn native_flicker(time: f32, seed: f32) -> f32 {
    var state = vec4(floor(time*10.0)*0.1+seed);
    let coefficients = vec4(81.239487,17.348024,37.349838,59.394840);
    state.x = fract(dot(state,coefficients));
    state.y = fract(dot(state,coefficients));
    state.z = fract(dot(state,coefficients));
    state.w = fract(dot(state,coefficients));
    state.x = fract(dot(state,coefficients));
    state.y = fract(dot(state,coefficients));
    state.z = fract(dot(state,coefficients));
    return fract(dot(state,coefficients));
}
fn native_fragment(in: VertexOutput, is_front: bool, slot: u32, surface: OriginsSurfaceData, input: PbrInput) -> FragmentOutput {
    var pbr = input;
    let native = surface.native;
    let flags = native.kind.y;
    let layered = native.kind.x == 7u || (native.kind.x == 10u && native.camo_flags.w != 0u);
    let time = animation_time(slot);
    let gradient = mat2x2(dpdx(in.uv),dpdy(in.uv));
    let face_sign = select(1.0,-1.0,!is_front && (pbr.material.flags & STANDARD_MATERIAL_FLAGS_DOUBLE_SIDED_BIT) != 0u);
    pbr.world_normal = in.world_normal*face_sign;
    pbr.N = normalize(pbr.world_normal);
    @if(VERTEX_TANGENTS) let tbn = calculate_tbn_mikktspace(in.world_normal,in.world_tangent);
    var normal_uv = in.uv;
    if layered { normal_uv *= native.staff.xy; }
    var slope = vec2(0.0);
    if native.kind.z != 0u { slope = native_slope(sample_normal_map(slot,normal_uv,mat2x2(dpdx(normal_uv),dpdy(normal_uv)))); }
    if native.kind.x == 10u { slope *= f32(native.camo_flags.y); }
    if (flags & 2u) != 0u {
        var detail_uv = in.uv*native.detail.xy;
        if native.kind.x == 10u { detail_uv = in.uv*native.camo.xy; }
        if layered { detail_uv *= native.staff.xy; }
        var strength = 1.0;
        if native.kind.x == 10u { strength = native.camo.w; }
        slope += native_slope(sample_second_normal(slot,detail_uv,mat2x2(dpdx(detail_uv),dpdy(detail_uv))))*strength;
    }
    if layered { slope *= native.staff.z; }
    @if(VERTEX_TANGENTS) pbr.N = normalize(tbn*vec3(slope,1.0))*face_sign;
    let color = sample_color_base_grad(slot,in.uv,gradient);
    var diffuse = color.rgb*in.color.rgb;
    if (flags & 512u) != 0u && native.kind.x != 4u { diffuse *= in.color.rgb; }
    var alpha = color.a*in.color.a;
    var sg = vec4(vec3(56.0/255.0),0.0);
    if (flags & 1u) != 0u { sg = sample_specular_map(slot,in.uv,gradient); }
    if (flags & 16u) != 0u { sg.a = sample_second_specular(slot,in.uv,gradient).r; }
    if (flags & 16384u) != 0u { sg.a = color.a; alpha = 1.0; }
    if native.kind.x == 10u && !layered {
        let camo_uv = in.uv*native.camo.xy;
        let camo_gradient = mat2x2(dpdx(camo_uv),dpdy(camo_uv));
        diffuse = sample_color_base_grad(slot,camo_uv,camo_gradient).rgb*in.color.rgb;
        let detail = sample_second_specular(slot,camo_uv,camo_gradient);
        sg = vec4(detail.rgb,mix(detail.a,sg.a,native.camo.z*f32(native.camo_flags.z)));
        if native.camo_flags.x != 0u {
            let weapon_color = sample_color_first_grad(slot,in.uv,gradient);
            diffuse = mix(diffuse,weapon_color.rgb*in.color.rgb,weapon_color.a);
        }
        alpha = 1.0;
    }
    sg = vec4(sg.rgb*native.flicker.w,sg.a);
    if native.kind.x == 2u || native.kind.x == 8u {
        alpha = sqrt(max(alpha,0.0));
    }
    var control = vec4(1.0);
    if (flags & 4u) != 0u {
        control = sample_control_map(slot,in.uv,gradient);
        pbr.diffuse_occlusion *= vec3(clamp(control.r,0.0,1.0));
        pbr.specular_occlusion *= clamp(control.g,0.0,1.0);
    }
    if (flags & 8u) != 0u {
        let uv = (in.uv-vec2(0.5))*native.camo_extra.z+native.camo_extra.xy+vec2(0.5);
        let detail = sample_third_specular(slot,uv,mat2x2(dpdx(uv),dpdy(uv)));
        let adjusted_specular = mix(clamp(sg.rgb+vec3(detail.a-0.5),vec3(0.0),vec3(1.0)),sg.rgb,color.a);
        sg = vec4(mix(sg.rgb,adjusted_specular,native.camo_extra.w),sg.a);
        let adjusted_color = mix(clamp(detail.rgb+vec3(control.b-0.5),vec3(0.0),vec3(1.0)),color.rgb,color.a);
        diffuse = mix(color.rgb,adjusted_color,native.camo_extra.w)*in.color.rgb;
    }
    var gloss = clamp(sg.a*native.response.z,0.0,1.0);
    var emission = vec3(0.0);
    if native.kind.x == 4u {
        let flicker = mix(native.flicker.x,native.flicker.y,native_flicker(time,native.ember.w));
        let ember_uv = in.uv*native.ember.x+fract(time*native.motion.xy*native.ember.x);
        let heat_uv = in.uv*native.ember.y+fract(time*native.motion.zw*native.ember.y);
        let ember = sample_color_first_grad(slot,ember_uv,gradient*native.ember.x);
        let heat = sample_color_second_grad(slot,heat_uv,gradient*native.ember.y);
        let reveal = sample_color_third_grad(slot,in.uv,gradient);
        diffuse = mix(diffuse,ember.rgb*heat.rgb*flicker*6.0,clamp(reveal.r*in.color.r,0.0,1.0))*in.color.rgb;
        emission = ember.rgb*heat.rgb*flicker*flicker*6.0*reveal.rgb*in.color.rgb*ember.a*reveal.a*in.color.a*native.response.w;
        emission *= emission;
    } else if (flags & 32u) != 0u {
        let glow = sample_color_first_grad(slot,in.uv,gradient);
        emission = glow.rgb*glow.a*native.flicker.z*native.flicker.z*native.response.w;
        emission *= emission;
    }
    if layered {
        let geometric = normalize(pbr.world_normal);
        var parallax = vec2(0.0);
        @if(VERTEX_TANGENTS) {
            let tangent = normalize(in.world_tangent.xyz);
            let bitangent = cross(geometric,tangent)*in.world_tangent.w;
            let facing = dot(pbr.V,geometric);
            if facing > 0.0 { parallax = -vec2(dot(pbr.V,tangent),dot(pbr.V,bitangent))/max(facing,0.001); }
        }
        var samples: array<vec4<f32>,3>;
        for (var layer=0u;layer<3u;layer++) {
            let uv = native_layer_uv(in.uv,layer,native,time,parallax);
            let g = mat2x2(dpdx(uv),dpdy(uv));
            if layer == 0u { samples[layer] = sample_color_base_grad(slot,uv,g); }
            if layer == 1u { samples[layer] = sample_color_first_grad(slot,uv,g); }
            if layer == 2u {
                samples[layer] = sample_color_second_grad(slot,uv,g);
                if native.kind.x == 7u && native.staff_emission.z >= native.staff.w { samples[layer] = sample_color_third_grad(slot,uv,g); }
            }
        }
        let third = samples[2].rgb*samples[2].a;
        let second = mix(third,samples[1].rgb,samples[1].a*native.layer_alpha.y);
        let resolved = mix(second,samples[0].rgb,samples[0].a*native.layer_alpha.x);
        let strength = select(native.staff_emission.x,native.staff_emission.y,native.kind.x == 7u && native.staff_emission.z >= native.staff.w);
        emission = resolved*in.color.rgb*strength;
        emission *= emission;
        diffuse = vec3(0.0);
        gloss = native.layer_alpha.w;
        alpha = 1.0;
    }
    if native.kind.x == 9u {
        var projected_uv = vec2(0.5);
        @if(VERTEX_TANGENTS) {
            let normal = normalize(pbr.world_normal);
            let tangent = normalize(in.world_tangent.xyz);
            let bitangent = cross(normal,tangent)*in.world_tangent.w;
            projected_uv += vec2(dot(pbr.V,tangent),-dot(pbr.V,bitangent))*native.camo.w;
        }
        let reticle = sample_color_base_grad(slot,projected_uv,mat2x2(dpdx(projected_uv),dpdy(projected_uv)));
        let noise_uv = in.uv*native.weapon_detail.x;
        let noise = sample_third_specular(slot,noise_uv,mat2x2(dpdx(noise_uv),dpdy(noise_uv))).r;
        diffuse = native.camo_extra.rgb*pow(max(reticle.g,0.0),native.weapon_detail.y)*noise+native.camo.rgb*reticle.r*(1.0-native.weapon_detail.z*noise);
        alpha = reticle.b*native.weapon_detail.w;
    }
    if (flags & 8192u) != 0u {
        diffuse = color.rgb;
        alpha = color.a;
        if (flags & 1024u) != 0u { alpha = sqrt(max(color.a,0.0)); }
        if (flags & 2048u) != 0u { diffuse *= color.a; }
        if (flags & 6144u) != 0u { alpha = 1.0; }
    }
    var specular_amount = max(native.response.x,0.0);
    if (flags & 8u) != 0u { specular_amount *= max(native.detail.z,0.0); }
    if native.kind.x == 8u {
        let grazing = 1.0-clamp(dot(normalize(pbr.world_normal),pbr.V),0.0,1.0);
        specular_amount *= smoothstep(0.0,max(native.weapon_detail.x,0.00001),grazing);
    }
    let f0 = clamp(sg.rgb*sg.rgb*specular_amount,vec3(0.0),vec3(1.0));
    let dielectric = (flags & 512u) != 0u || native.kind.x == 2u || native.kind.x == 8u;
    let diffuse_linear = diffuse*diffuse;
    var metallic = 0.0;
    if !dielectric && !layered { metallic = legacy_metallic(diffuse_linear,f0); }
    let remaining = 1.0-max(max(f0.r,f0.g),f0.b);
    let dielectric_f0 = select(min(f0,vec3(0.16)),vec3(LEGACY_DIELECTRIC_F0),metallic > 0.000001);
    let from_diffuse = diffuse_linear*remaining/max(1.0-max(max(dielectric_f0.r,dielectric_f0.g),dielectric_f0.b),0.000001)/max(1.0-metallic,0.000001);
    let from_specular = (f0-dielectric_f0*(1.0-metallic))/max(metallic,0.000001);
    let base_color = clamp(mix(from_diffuse,from_specular,metallic*metallic),vec3(0.0),vec3(1.0));
    pbr.material.reflectance = sqrt(dielectric_f0/0.16);
    pbr.material.metallic = metallic;
    if (flags & 16401u) != 0u || layered {
        pbr.material.perceptual_roughness = clamp(pow(2.0/(exp2(13.0*gloss)+2.0),0.25),0.089,1.0);
    }
    pbr.material.clearcoat = 0.0;
    pbr.material.base_color = alpha_discard(pbr.material.flags,pbr.material.alpha_cutoff,vec4(select(base_color,diffuse_linear,dielectric || layered),alpha));
    pbr.material.emissive = vec4(emission,0.0);
    var out: FragmentOutput;
    if native.kind.x == 3u || native.kind.x == 9u {
        out.color = vec4(diffuse*diffuse*native.response.w*native.response.w,alpha);
    } else { out.color = apply_pbr_lighting(pbr); }
    let mode = diagnostic_mode(slot);
    if mode != 0u && mode < 8u && mode != 6u {
        var debug = vec3(pbr.material.perceptual_roughness);
        if mode == 1u { debug = fract(vec3(surface.response.z)*vec3(0.618034,0.414214,0.732051)); }
        if mode == 3u { debug = vec3(metallic); }
        if mode == 4u { debug = pbr.N*0.5+0.5; }
        if mode == 5u { debug = vec3(alpha); }
        if mode == 7u { debug = normalize(pbr.world_normal)*0.5+0.5; }
        out.color = vec4(debug,1.0);
        return out;
    }
    out.color = main_pass_post_lighting_processing(pbr,out.color);
    if (pbr.material.flags & STANDARD_MATERIAL_FLAGS_ALPHA_MODE_RESERVED_BITS) == STANDARD_MATERIAL_FLAGS_ALPHA_MODE_PREMULTIPLIED { out.color = vec4(out.color.rgb*out.color.a,out.color.a); }
    return out;
}

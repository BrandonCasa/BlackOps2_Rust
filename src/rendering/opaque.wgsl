// Project-owned opaque response. Bevy retains PBR lights, shadows and post processing.
#import bevy_pbr::{
    forward_io::{VertexOutput, FragmentOutput},
    pbr_fragment::pbr_input_from_standard_material,
    pbr_functions::{main_pass_post_lighting_processing, calculate_tbn_mikktspace, alpha_discard},
    lighting::F_AB,
    mesh_view_bindings::view,
    pbr_types::{STANDARD_MATERIAL_FLAGS_ALPHA_MODE_PREMULTIPLIED, STANDARD_MATERIAL_FLAGS_ALPHA_MODE_RESERVED_BITS},
}
#import origins::baked_lighting::apply_pbr_lighting

#import origins::surface_bindings::{OriginsSurfaceData, animation_time, parameters_for, atlas_dimensions, reflection_levels, sample_reflection, sample_normal_map, sample_second_normal, sample_specular_map, sample_second_specular, sample_control_map, sample_packed_lightmap, sample_color0, sample_color1, sample_color2, sample_color3, sample_third_normal, sample_third_specular}
#import bevy_pbr::mesh_bindings::mesh

fn normal_slope(sampled: vec2<f32>, strength: f32) -> vec2<f32> {
    // Raw BC5 source slope convention, with DirectX Y reversed for glTF tangents.
    return (sampled * 4.015748 - 2.015748) * vec2(1.0, -1.0) * strength;
}

fn source_basis(direction: vec3<f32>) -> vec3<f32> {
    // Preparation uses (x,z,-y); SH and cube faces retain source Z-up.
    return vec3(direction.x, -direction.z, direction.y);
}

fn probe_diffuse(normal: vec3<f32>, surface: OriginsSurfaceData) -> vec3<f32> {
    let n = source_basis(normal);
    let k = dot(surface.sh_two, vec4(n.z*n.x, n.y*n.z, n.x*n.y, n.x*n.x-n.y*n.y))
        + dot(surface.sh_one, vec4(n, 1.0)) + surface.sh_zero.w*n.z*n.z;
    return max(surface.sh_zero.rgb * k, vec3(0.0));
}

fn atlas_sample(slot: u32, uv: vec2<f32>, band: f32) -> vec4<f32> {
    // Clamp within each band to avoid bilinear bleeding into its neighbour.
    let dimensions = vec2<f32>(atlas_dimensions(slot));
    let inset = vec2(0.5) / vec2(dimensions.x, dimensions.y / 3.0);
    let w = clamp(uv, inset, vec2(1.0) - inset);
    return sample_packed_lightmap(slot, vec2(w.x, (w.y+band)/3.0));
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
    #ifdef BINDLESS
    let slot = mesh[in.instance_index].material_and_lightmap_bind_group_slot & 0xffffu;
#else
    let slot = 0u;
#endif
    let surface = parameters_for(slot);
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
        let mask = sample_control_map(slot, in.uv).rg;
        weights = vec3(mask.g,mask.r,0.0)/max(mask.r+mask.g,0.0001);
        // Normal 0 describes the whole object's shape at authored UVs.
        // Normals 1/2 accompany coating/core (color 1/0), respectively.
        uv1 = in.uv * surface.response.w;
    }
    var slope0 = select(vec2(0.0), normal_slope(sample_normal_map(slot, uv0).rg, surface.response.x), (surface.modes.x & 2u) != 0u);
    var slope1 = select(vec2(0.0), normal_slope(sample_second_normal(slot, uv1).rg, surface.response.x), (surface.modes.x & 4u) != 0u);
    var slope2 = vec2(0.0);
    var s0 = select(vec4(0.5,0.5,0.5,0.36), sample_specular_map(slot, uv0), surface.modes.y != 0u);
    var s1 = select(s0, sample_second_specular(slot, uv1), (surface.modes.z & 1u) != 0u);
    var s2 = s0;
    if layered {
        uv1 = layer_uv.xy;
        uv2 = layer_uv.zw;
        let c0 = sample_color0(slot, uv0);
        let c1 = sample_color1(slot, uv1);
        let c2 = sample_color2(slot, uv2);
        let c3 = sample_color3(slot, layer3_uv);
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
        slope1 = select(slope0,orient(normal_slope(sample_second_normal(slot, uv1).rg, surface.response.x),rotation1),(surface.modes.x & 4u) != 0u);
        slope2 = select(slope0,orient(normal_slope(sample_third_normal(slot, uv2).rg, surface.response.x),rotation2),(surface.modes.x & 8u) != 0u);
        s1 = select(s0,sample_second_specular(slot, uv1),(surface.modes.z & 1u) != 0u);
        s2 = select(s0,sample_third_specular(slot, uv2),(surface.modes.z & 2u) != 0u);
    }
    var slope = slope0*weights.x+slope1*weights.y+slope2*weights.z;
    var sg = s0*weights.x+s1*weights.y+s2*weights.z;
    if tile_macro {
        let coating = select(vec2(0.0),normal_slope(sample_second_normal(slot, uv1).rg, surface.response.x),(surface.modes.x & 4u) != 0u);
        let core = select(vec2(0.0),normal_slope(sample_third_normal(slot, uv1).rg, surface.response.x),(surface.modes.x & 8u) != 0u);
        // Blend detail slopes onto the unscaled macro normal. The shared diffuse
        // bake already includes this mask's occlusion, so do not multiply it again.
        slope = slope0 + core*weights.x + coating*weights.y;
        s0 = select(vec4(0.5,0.5,0.5,0.36),sample_specular_map(slot, uv1),surface.modes.y != 0u);
        s1 = select(s0,sample_second_specular(slot, uv1),(surface.modes.z & 1u) != 0u);
        // Specular bindings follow coating/core order, like detail normals.
        sg = s0*weights.y+s1*weights.x;
    }
    if water {
        let t = animation_time(slot);
        let wuv = in.uv*surface.water.z;
        let a = normal_slope(sample_normal_map(slot, wuv+t*surface.velocity.xy).rg, surface.response.x);
        let b = normal_slope(sample_second_normal(slot, wuv*1.37+t*surface.velocity.zw).rg, surface.response.x);
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
        let ambient = atlas_sample(slot, in.uv_b, 0.0);
        let directional = atlas_sample(slot, in.uv_b, 1.0);
        let direction = atlas_sample(slot, in.uv_b, 2.0).rgb*2.0-1.0;
        let world_direction = vec3(direction.x, direction.z, -direction.y);
        pbr.lightmap_light = (ambient.rgb/max(ambient.a, 1.0/255.0)
            + directional.rgb/max(directional.a, 1.0/255.0)*clamp(dot(world_direction,pbr.N),0.0,1.0))*surface.radiance.x;
#endif
    } else if surface.modes.w == 2u {
        // Probe-only props have no retained local bake. A bounded share of the
        // probe's spherical mean reconstructs indirect bounce in dark SH lobes.
        // Atlas/vertex lighting and tile calibration retain their authored path.
        pbr.lightmap_light = max(probe_diffuse(pbr.N, surface), surface.lighting.rgb)*surface.radiance.x;
    } else if surface.modes.w == 3u {
        // Interpolated retained instance irradiance; alpha is not surface opacity.
        pbr.lightmap_light = (baked_color + probe_diffuse(pbr.N, surface)*surface.radiance.w)*surface.radiance.x;
    }
    var out: FragmentOutput;
    out.color = apply_pbr_lighting(pbr);
    if surface.lighting.w > 0.0 {
        let roughness = pbr.material.perceptual_roughness;
        let r = reflect(-pbr.V,pbr.N);
        let source = source_basis(r);
        // Retained DDS faces already use source sampling axes. Unlike a cube
        // authored for Bevy's world basis, this requires no additional Z flip.
        let radiance = sample_reflection(slot,
            source, roughness*f32(reflection_levels(slot)-1u)).rgb;
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

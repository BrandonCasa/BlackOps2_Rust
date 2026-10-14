import bevy_pbr::{
    render::{
        forward_io::{VertexOutput,FragmentOutput},
        pbr_fragment::pbr_input_from_standard_material,
        pbr_functions::{apply_pbr_lighting,main_pass_post_lighting_processing},
        view_transformations::{position_world_to_view,depth_ndc_to_view_z},
    },
    prepass::utils::prepass_depth,
};

struct Controls {
    domains:array<vec4<f32>,4>,
    color:vec4<f32>,
    parameters:vec4<f32>,
    time:vec4<f32>,
    kind:vec4<u32>,
}
@group(constants::MATERIAL_BIND_GROUP) @binding(100) var<uniform> controls:Controls;
@group(constants::MATERIAL_BIND_GROUP) @binding(101) var texture0:texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(102) var sampler0:sampler;
@group(constants::MATERIAL_BIND_GROUP) @binding(103) var texture1:texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(104) var sampler1:sampler;
@group(constants::MATERIAL_BIND_GROUP) @binding(105) var texture2:texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(106) var sampler2:sampler;
@group(constants::MATERIAL_BIND_GROUP) @binding(107) var texture3:texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(108) var sampler3:sampler;
@group(constants::MATERIAL_BIND_GROUP) @binding(109) var texture4:texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(110) var sampler4:sampler;
@group(constants::MATERIAL_BIND_GROUP) @binding(111) var texture5:texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(112) var sampler5:sampler;

fn scroll(uv:vec2<f32>,domain:vec4<f32>)->vec2<f32> {
    return uv*domain.xy+fract(controls.time.w*domain.zw*domain.xy);
}

fn flicker(time:f32)->f32 {
    let ten=time*10.0;
    let q=(ten-fract(ten))*0.1+controls.parameters.z;
    var state=vec4(q);
    let weights=vec4(81.239487,17.348024,37.349838,59.394840);
    state.x=fract(dot(state,weights));
    state.y=fract(dot(state,weights));
    state.z=fract(dot(state,weights));
    state.w=fract(dot(state,weights));
    state.x=fract(dot(state,weights));
    state.y=fract(dot(state,weights));
    state.z=fract(dot(state,weights));
    state.x=fract(dot(state,weights));
    return controls.parameters.x+(controls.parameters.y-controls.parameters.x)*state.x;
}

@fragment
fn fragment(in:VertexOutput,@builtin(front_facing) is_front:bool)->FragmentOutput {
    let uv=in.uv;
    let vertex=in.color;
    var pbr=pbr_input_from_standard_material(in,is_front);
    var out:FragmentOutput;
    if controls.kind.x==0u {
        let alpha_uv=scroll(uv,controls.domains[0]);
        let plus_uv=scroll(uv,controls.domains[1]);
        let minus_uv=scroll(uv,controls.domains[2]);
        let mask_uv=(uv-controls.domains[3].zw)*controls.domains[3].xy+controls.domains[3].zw;
        let plus_alpha=textureSample(texture0,sampler0,plus_uv).r;
        let alpha=textureSample(texture1,sampler1,alpha_uv).r;
        let mask=textureSample(texture2,sampler2,mask_uv).r;
        let minus_alpha=textureSample(texture3,sampler3,minus_uv).r;
        let plus_color=textureSample(texture4,sampler4,plus_uv).rgb;
        let minus_color=textureSample(texture5,sampler5,minus_uv).rgb;
        let depth=prepass_depth(in.position,0u);
        let scene_z=depth_ndc_to_view_z(depth);
        let fragment_z=position_world_to_view(in.world_position.xyz).z;
        let soft=clamp((fragment_z-scene_z)/(0.0254*max(controls.parameters.y,controls.parameters.z)),0.0,1.0);
        let density=alpha*mask*(plus_alpha+minus_alpha);
        let weight=density*vertex.a*vertex.a*soft*controls.color.a;
        let rgb=(plus_color+minus_color)*vertex.rgb*controls.color.rgb*weight*controls.parameters.x;
        out.color=main_pass_post_lighting_processing(pbr,vec4(rgb*rgb,1.0));
        return out;
    }
    let mask=textureSample(texture1,sampler1,uv);
    let domain=controls.domains[0];
    let ember_uv=uv*domain.x+fract(controls.time.w*domain.yz*domain.x);
    let ember=textureSample(texture2,sampler2,ember_uv);
    let diffuse=textureSample(texture3,sampler3,uv);
    let candidate=ember*flicker(controls.time.w)*mask.r*vec4(6.0,6.0,6.0,1.0);
    let mixed=mix(diffuse,candidate,mask.g)*vertex;
    if mixed.a<=0.0 { discard; }
    let slope=textureSample(texture0,sampler0,uv).xy*4.015748-vec2(2.015748);
    let geometric=normalize(in.world_normal);
    let tangent=normalize(in.world_tangent.xyz);
    let bitangent=cross(geometric,tangent)*in.world_tangent.w;
    let normal=normalize(geometric+slope.x*tangent+slope.y*bitangent);
    pbr.material.base_color=vec4(mixed.rgb*mixed.rgb,sqrt(clamp(mixed.a,0.0,1.0)));
    pbr.N=normal;
    pbr.world_normal=normal;
    var lit=apply_pbr_lighting(pbr);
    lit=vec4(lit.rgb*mixed.a,lit.a);
    out.color=main_pass_post_lighting_processing(pbr,lit);
    return out;
}

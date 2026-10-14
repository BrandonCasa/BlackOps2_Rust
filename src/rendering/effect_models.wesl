import bevy_pbr::{
    render::{
        forward_io::{VertexOutput, FragmentOutput},
        mesh_functions::{get_local_from_world,get_world_from_local},
        mesh_view_bindings::view,
        view_transformations::{frag_coord_to_ndc,position_ndc_to_world,position_world_to_view,depth_ndc_to_view_z},
        pbr_fragment::pbr_input_from_standard_material,
        pbr_functions::{apply_pbr_lighting,main_pass_post_lighting_processing},
    },
    prepass::utils::prepass_depth,
};

struct Controls {
    constants: array<vec4<f32>,16>,
    kind: vec4<u32>,
}

@group(constants::MATERIAL_BIND_GROUP) @binding(100) var<uniform> controls: Controls;
@group(constants::MATERIAL_BIND_GROUP) @binding(101) var texture0: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(102) var sampler0: sampler;
@group(constants::MATERIAL_BIND_GROUP) @binding(103) var texture1: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(104) var sampler1: sampler;
@group(constants::MATERIAL_BIND_GROUP) @binding(105) var texture2: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(106) var sampler2: sampler;
@group(constants::MATERIAL_BIND_GROUP) @binding(107) var texture3: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(108) var sampler3: sampler;
@group(constants::MATERIAL_BIND_GROUP) @binding(109) var texture4: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(110) var sampler4: sampler;
@group(constants::MATERIAL_BIND_GROUP) @binding(111) var texture5: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(112) var sampler5: sampler;
@group(constants::MATERIAL_BIND_GROUP) @binding(113) var texture6: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(114) var sampler6: sampler;

fn c(index:u32)->vec4<f32> { return controls.constants[index]; }

@fragment
fn fragment(vertex:VertexOutput,@builtin(front_facing) is_front:bool)->FragmentOutput {
    var in=vertex;
    let depth=prepass_depth(in.position,0u);
    if controls.kind.x==0u {
        if depth<=0.0 { discard; }
        var ndc=frag_coord_to_ndc(in.position);
        ndc.z=depth;
        let world=position_ndc_to_world(ndc);
        let local=(get_local_from_world(in.instance_index)*vec4(world,1.0)).xyz;
        let native=vec3(local.x,-local.z,local.y)/0.0254;
        if any(abs(native)>vec3(1.0)) { discard; }
        let uv=native.yz*0.5+vec2(0.5);
        let sample=textureSample(texture0,sampler0,uv);
        in.world_position=vec4(world,1.0);
        in.world_normal=normalize(get_world_from_local(in.instance_index)[0].xyz);
        var pbr=pbr_input_from_standard_material(in,is_front);
        pbr.material.base_color=vec4(sample.rgb*sample.rgb,sample.a*(1.0-abs(native.x)));
        pbr.material.reflectance=vec3(0.0);
        pbr.N=in.world_normal;
        pbr.world_normal=in.world_normal;
        var out:FragmentOutput;
        out.color=main_pass_post_lighting_processing(pbr,apply_pbr_lighting(pbr));
        return out;
    }
    var uv=vec2(0.0);
@if(VERTEX_UVS_A)
    uv=in.uv;
    var color=vec4(1.0);
@if(VERTEX_COLORS)
    color=in.color;
    let relative=(in.world_position.xyz-view.world_position.xyz)/0.0254;
    let horizontal=length(relative.xz);
    let fade=smoothstep(0.0,1.0,clamp((horizontal-c(15u).x)/(c(15u).y-c(15u).x),0.0,1.0))*c(14u).w;
    let offset=mix(c(10u).xy,c(15u).zw,fade);
    let time=c(4u).w;
    let distortion_uv=uv*c(9u).xy+fract(time*c(8u).yz*c(9u).xy);
    let distortion=textureSample(texture2,sampler2,distortion_uv).xy*2.0-vec2(1.0);
    let first_uv=uv+distortion*c(9u).z+offset;
    let alpha_uv=first_uv*c(5u).xy+fract(time*c(5u).zw*c(5u).xy);
    let first_alpha=textureSample(texture4,sampler4,alpha_uv).r;
    let normal=normalize(in.world_normal);
    let facing=clamp(dot(-normalize(relative),normal),0.0,1.0);
    let falloff=pow(facing,c(6u).y)*c(6u).z;
    let second_distortion_uv=uv*c(12u).zw+fract(time*c(12u).xy*c(12u).zw);
    let second_distortion=textureSample(texture0,sampler0,second_distortion_uv).xy*2.0-vec2(1.0);
    let second_uv=(uv+second_distortion*c(13u).x)*c(14u).xy+fract(time*c(13u).yz*c(14u).xy);
    let second_alpha=textureSample(texture6,sampler6,second_uv).r*falloff*c(14u).z;
    let second_color=textureSample(texture1,sampler1,second_uv).rgb;
    let mask=textureSample(texture5,sampler5,uv).r;
    let first_color_uv=first_uv*c(7u).zw+fract(time*c(7u).xy*c(7u).zw);
    let first_color=textureSample(texture3,sampler3,first_color_uv).rgb;
    let scene_z=depth_ndc_to_view_z(depth);
    let fragment_z=position_world_to_view(in.world_position.xyz).z;
    let depth_fade=clamp((fragment_z-scene_z)/(0.0254*max(c(8u).x,0.0001)),0.0,1.0);
    let alpha=(first_alpha*mask+second_alpha)*color.a*color.a*depth_fade*c(11u).w;
    let rgb=(first_color+second_color*c(14u).z)*color.rgb*c(11u).rgb*alpha*c(6u).x;
    var pbr=pbr_input_from_standard_material(in,is_front);
    var out:FragmentOutput;
    out.color=main_pass_post_lighting_processing(pbr,vec4(rgb*rgb,1.0));
    return out;
}

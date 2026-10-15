import bevy_pbr::render::{
    forward_io::{VertexOutput,FragmentOutput},
    mesh_view_bindings::view,
    mesh_functions::get_tag,
    pbr_fragment::pbr_input_from_standard_material,
    pbr_functions::{apply_pbr_lighting,main_pass_post_lighting_processing},
};

struct Controls {
    scale_offset:array<vec4<f32>,4>,
    origin_rotation:array<vec4<f32>,4>,
    scroll_depth_alpha:array<vec4<f32>,4>,
    script:vec4<f32>,
    time:vec4<f32>,
    emission:vec4<f32>,
    options:vec4<f32>,
    reflection:vec4<f32>,
    normal_decode:vec4<f32>,
}
@group(constants::MATERIAL_BIND_GROUP) @binding(100) var<uniform> material_controls:Controls;
@group(constants::MATERIAL_BIND_GROUP) @binding(111) var<storage,read> states:array<vec4<f32>>;
@group(constants::MATERIAL_BIND_GROUP) @binding(101) var first:texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(102) var first_sampler:sampler;
@group(constants::MATERIAL_BIND_GROUP) @binding(103) var second:texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(104) var second_sampler:sampler;
@group(constants::MATERIAL_BIND_GROUP) @binding(105) var third:texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(106) var third_sampler:sampler;
@group(constants::MATERIAL_BIND_GROUP) @binding(107) var fourth:texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(108) var fourth_sampler:sampler;
@group(constants::MATERIAL_BIND_GROUP) @binding(109) var normal_map:texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(110) var normal_sampler:sampler;

fn layer_uv(index:u32,uv:vec2<f32>,parallax:vec2<f32>,controls:Controls)->vec2<f32> {
    let scale=controls.scale_offset[index];
    let origin=controls.origin_rotation[index];
    let scroll=controls.scroll_depth_alpha[index];
    let centered=(uv-origin.xy+scale.zw+parallax*(scroll.z+controls.options.z))*scale.xy;
    let angle=origin.z*(1.0+origin.w*controls.time.w)*controls.emission.w;
    let sine=sin(angle);
    let cosine=cos(angle);
    return vec2(cosine*centered.x+sine*centered.y,-sine*centered.x+cosine*centered.y)
        +controls.time.w*scroll.xy*scale.xy+origin.xy;
}

@fragment
fn fragment(in:VertexOutput,@builtin(front_facing) is_front:bool)->FragmentOutput {
    var controls=material_controls;
    let slot=get_tag(in.instance_index)*2u;
    controls.script=states[slot];
    controls.time=states[slot+1u];
    let mode=u32(controls.options.x);
    let count=u32(controls.options.y);
    let n=normalize(in.world_normal);
    let v=normalize(view.world_position.xyz-in.world_position.xyz);
    var tangent=vec3(0.0);
    var bitangent=vec3(0.0);
@if(VERTEX_TANGENTS) {
    tangent=normalize(in.world_tangent.xyz);
    bitangent=cross(n,tangent)*in.world_tangent.w;
}
    var parallax=vec2(0.0);
    if dot(v,n)>0.0 {
        parallax=-vec2(dot(v,tangent),dot(v,bitangent))/dot(v,n);
    }
    var uv=vec2(0.0);
@if(VERTEX_UVS_B)
    uv=in.uv_b;
    var vertex=vec4(1.0);
@if(VERTEX_COLORS)
    vertex=in.color;
    var samples:array<vec4<f32>,4>;
    samples[0]=textureSample(first,first_sampler,layer_uv(0u,uv,parallax,controls));
    samples[1]=textureSample(second,second_sampler,layer_uv(1u,uv,parallax,controls));
    samples[2]=textureSample(third,third_sampler,layer_uv(2u,uv,parallax,controls));
    samples[3]=textureSample(fourth,fourth_sampler,layer_uv(3u,uv,parallax,controls));
    var weights:array<f32,4>;
    for(var i=0u;i<4u;i+=1u) {
        weights[i]=samples[i].a*controls.scroll_depth_alpha[i].w*(1.0-clamp(controls.script[i],0.0,1.0));
    }
    var color=samples[count-1u].rgb*samples[count-1u].a;
    var coverage=weights[count-1u];
    for(var i=i32(count)-2;i>=0;i-=1) {
        color+=weights[u32(i)]*(samples[u32(i)].rgb-color);
        coverage=clamp(coverage+weights[u32(i)],0.0,1.0);
    }
    if mode==4u || mode==5u || mode==7u {
        color*=coverage;
    }
    if mode>=2u && mode<=5u {
        color*=pow(clamp(dot(v,n),0.0,1.0),controls.emission.y)*controls.emission.z;
    }
    if mode==1u {
        color*=vertex.rgb*(1.0-controls.script.x);
    }
    if mode==5u {
        color*=vertex.rgb*vertex.a*vertex.a;
    }
    color*=controls.emission.x;
    var alpha=1.0;
    if mode==0u || mode==3u || mode==6u {
        alpha=sqrt(max(coverage,0.0));
    }
    var pbr=pbr_input_from_standard_material(in,is_front);
    var output=vec4(color*color*view.exposure,alpha);
    if controls.options.w!=0.0 {
        var bumped=n;
        if controls.normal_decode.z!=0.0 {
            let slope=(textureSample(normal_map,normal_sampler,uv*controls.reflection.yz).xy*controls.normal_decode.x
                +vec2(controls.normal_decode.y))*controls.reflection.w;
            bumped=normalize(n+tangent*slope.x+bitangent*slope.y);
        }
        pbr.N=bumped;
        pbr.world_normal=bumped;
        pbr.material.base_color=vec4(0.0,0.0,0.0,1.0);
        pbr.material.emissive=vec4(0.0);
        pbr.material.reflectance=vec3(sqrt(max(controls.options.w,0.0)/0.16));
        pbr.material.perceptual_roughness=clamp(1.0-controls.reflection.x,0.0,1.0);
        output=vec4(output.rgb+apply_pbr_lighting(pbr).rgb,alpha);
    }
    var out:FragmentOutput;
    out.color=main_pass_post_lighting_processing(pbr,output);
    return out;
}

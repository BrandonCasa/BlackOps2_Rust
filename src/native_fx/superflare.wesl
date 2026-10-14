import bevy_pbr::render::{
    forward_io::{Vertex,decompress_vertex},
    mesh_functions::get_world_from_local,
    mesh_view_bindings::view,
    view_transformations::position_world_to_clip,
};

struct Controls {
    flare:vec4<f32>,
    atlas:vec4<f32>,
    draw:vec4<u32>,
}
struct VertexOutput {
    @builtin(position) position:vec4<f32>,
    @location(0) sprite_uv:vec2<f32>,
    @location(1) grime_uv:vec2<f32>,
    @location(2) tint:vec3<f32>,
}
@group(constants::MATERIAL_BIND_GROUP) @binding(0) var<uniform> controls:Controls;
@group(constants::MATERIAL_BIND_GROUP) @binding(1) var sprite:texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(2) var sprite_sampler:sampler;
@group(constants::MATERIAL_BIND_GROUP) @binding(3) var grime:texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(4) var grime_sampler:sampler;

@vertex
fn vertex(input:Vertex)->VertexOutput {
    let vertex=decompress_vertex(input,input.instance_index);
    let world_from_local=get_world_from_local(input.instance_index);
    let center=world_from_local*vec4(vertex.position,1.0);
    let clip=position_world_to_clip(center.xyz);
    let tint=vec3(0.75,0.5,1.0)*controls.flare.x*controls.flare.z;
    var out:VertexOutput;
    out.position=vec4(-1000.0);
    out.sprite_uv=vec2(0.0);
    out.grime_uv=vec2(0.0);
    out.tint=vec3(0.0);
    if controls.draw.x==0u || floor(255.0*controls.flare.w+0.000001)>=1.0
        || clip.w<0.0 || dot(tint.xzy,vec3(0.333,0.333,0.334))<0.0038 {
        return out;
    }
    let uv=vertex.uv*controls.atlas.xy+controls.atlas.zw;
    let aspect=view.viewport.w/view.viewport.z;
    let extent=500.0*0.0254*controls.flare.y/clip.w;
    let offset=(2.0*uv-vec2(1.0))*vec2(aspect,-1.0)*extent;
    let ndc=clip.xy/clip.w+offset;
    out.position=vec4(ndc,0.0,1.0);
    out.sprite_uv=uv*0.25+vec2(0.25,0.0);
    out.grime_uv=ndc*vec2(0.5,-0.5)+vec2(0.5);
    out.tint=tint;
    return out;
}

@fragment
fn fragment(in:VertexOutput)->@location(0) vec4<f32> {
    let mask=textureSample(sprite,sprite_sampler,in.sprite_uv).g;
    let dirt=textureSample(grime,grime_sampler,in.grime_uv).rgb;
    let rgb=0.2*clamp(mask*in.tint*(1.25*dirt),vec3(0.0),vec3(1.0));
    return vec4(rgb,0.0);
}

import bevy_pbr::render::{
    forward_io::{VertexOutput,FragmentOutput},
    mesh_view_bindings::view,
    pbr_fragment::pbr_input_from_standard_material,
    pbr_functions::{apply_pbr_lighting,main_pass_post_lighting_processing},
};

struct Controls {
    domains:array<vec4<f32>,4>,
    slope:vec4<f32>,
    color_normal:vec4<f32>,
    color_surface:vec4<f32>,
    fresnel:vec4<f32>,
    time:vec4<f32>,
}
@group(constants::MATERIAL_BIND_GROUP) @binding(100) var<uniform> controls:Controls;
@group(constants::MATERIAL_BIND_GROUP) @binding(101) var low:texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(102) var low_sampler:sampler;
@group(constants::MATERIAL_BIND_GROUP) @binding(103) var high:texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(104) var high_sampler:sampler;

@fragment
fn fragment(in:VertexOutput,@builtin(front_facing) is_front:bool)->FragmentOutput {
    let source_xy=vec2(in.world_position.x,-in.world_position.z)/0.0254;
    var slope=vec2(0.0);
    for(var layer=0u;layer<4u;layer+=1u) {
        let domain=controls.domains[layer];
        let uv=source_xy*domain.x+controls.time.w*domain.yz;
        if layer<2u {
            slope+=textureSample(low,low_sampler,uv).xy*controls.slope.x+vec2(controls.slope.y);
        } else {
            slope+=textureSample(high,high_sampler,uv).xy*controls.slope.z+vec2(controls.slope.w);
        }
    }
    let normal=normalize(vec3(slope.x,1.0,-slope.y));
    let to_view=normalize(view.world_position.xyz-in.world_position.xyz);
    let facing=clamp(dot(normal,to_view),0.0,1.0);
    let color=mix(controls.color_normal,controls.color_surface,facing);
    let fresnel=controls.fresnel.z+controls.fresnel.w*pow(1.0-facing,5.0);
    var alpha=color.a;
@if(VERTEX_COLORS)
    alpha*=in.color.a;
    var pbr=pbr_input_from_standard_material(in,is_front);
    pbr.material.base_color=vec4(color.rgb,clamp(alpha,0.0,1.0));
    pbr.material.reflectance=vec3(sqrt(max(fresnel,0.0)/0.16));
    pbr.material.perceptual_roughness=0.0;
    pbr.N=normal;
    pbr.world_normal=normal;
    var out:FragmentOutput;
    out.color=main_pass_post_lighting_processing(pbr,apply_pbr_lighting(pbr));
    return out;
}

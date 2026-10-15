import bevy_pbr::render::{
    forward_io::{VertexOutput,FragmentOutput},
    mesh_view_bindings::view,
    pbr_fragment::pbr_input_from_standard_material,
    pbr_functions::{apply_pbr_lighting,main_pass_post_lighting_processing},
};

@fragment
fn fragment(in:VertexOutput,@builtin(front_facing) is_front:bool)->FragmentOutput {
    var pbr=pbr_input_from_standard_material(in,is_front);
    var vertex=vec4(1.0);
@if(VERTEX_COLORS)
    vertex=in.color;
    var tint=vec3(1.0);
    if constants::BAKED_MODE==1u || constants::BAKED_MODE==5u {
        tint=vertex.rgb*vertex.rgb;
    }
    if constants::BAKED_MODE==5u {
        tint*=vertex.a*vertex.a*vertex.a*vertex.a;
    }
    let power=bitcast<f32>(constants::BAKED_FALLOFF_BITS);
    if power!=0.0 {
        let v=normalize(view.world_position.xyz-in.world_position.xyz);
        tint*=pow(clamp(dot(v,normalize(in.world_normal)),0.0,1.0),2.0*power);
    }
    pbr.material.emissive=vec4(pbr.material.emissive.rgb*tint,pbr.material.emissive.a);
    var out:FragmentOutput;
    out.color=apply_pbr_lighting(pbr);
    if constants::BAKED_MODE==5u { out.color.a=1.0; }
    out.color=main_pass_post_lighting_processing(pbr,out.color);
    return out;
}

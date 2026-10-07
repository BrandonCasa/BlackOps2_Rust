#import bevy_pbr::{
    forward_io::{VertexOutput, FragmentOutput},
    pbr_fragment::pbr_input_from_standard_material,
    pbr_functions::alpha_discard,
}
#import origins::diagnostic_bindings::{material_diagnostic_mode, material_diagnostic_identity}
#import origins::diagnostic_lighting::apply_pbr_lighting

@fragment
fn fragment(in: VertexOutput, @builtin(front_facing) front: bool) -> FragmentOutput {
    var pbr = pbr_input_from_standard_material(in, front);
    pbr.material.base_color = alpha_discard(pbr.material, pbr.material.base_color);
    let view_mode = material_diagnostic_mode();
    var out: FragmentOutput;
    if view_mode == 6u {
        out.color = apply_pbr_lighting(pbr);
        return out;
    }
    var color = vec3(pbr.material.perceptual_roughness);
    if view_mode == 1u { color = fract(vec3(material_diagnostic_identity())*vec3(0.618034,0.414214,0.732051)); }
    if view_mode == 3u { color = vec3(pbr.material.metallic); }
    if view_mode == 4u { color = pbr.N*0.5+0.5; }
    if view_mode == 5u { color = vec3(pbr.material.base_color.a); }
    out.color = vec4(color, 1.0);
    return out;
}

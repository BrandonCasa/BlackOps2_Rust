

@group(constants::MATERIAL_BIND_GROUP) @binding(100)
var<uniform> diagnostic_parameters: vec4<f32>;

fn material_diagnostic_mode() -> u32 { return u32(diagnostic_parameters.x); }
fn material_diagnostic_identity() -> f32 { return diagnostic_parameters.y; }

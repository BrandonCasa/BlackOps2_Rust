#import origins::gi_sampling::{gi_coordinate, gi_physical}
struct Constants { center_spacing: vec4<f32>, grid_generation: vec4<u32>, batch_lamps: vec4<u32> }
@group(0) @binding(0) var<uniform> constants: Constants;
@group(0) @binding(1) var<storage> updates: array<vec4<f32>>;
@group(0) @binding(2) var output: texture_storage_3d<rgba16float, write>;
@group(0) @binding(3) var<storage> probes: array<vec4<i32>>;
@compute @workgroup_size(64)
fn main(@builtin(workgroup_id) group: vec3<u32>, @builtin(local_invocation_index) lane: u32) {
    let physical = gi_physical(probes[group.x].xyz, vec3<i32>(constants.grid_generation.xyz));
    for (var index = lane; index < 208u; index += 64u) {
        textureStore(output, gi_coordinate(physical, index), updates[group.x * 208u + index]);
    }
}

#define_import_path bevy_pbr::irradiance_volume
#import bevy_pbr::light_probe::transpose_affine_matrix
#import bevy_pbr::mesh_view_bindings::{irradiance_volumes, irradiance_volume, light_probes}
#import bevy_pbr::clustered_forward::ClusterableObjectIndexRanges

#ifdef IRRADIANCE_VOLUMES_ARE_USABLE
// SAMPLING_HELPER
fn irradiance_volume_light(world_position: vec3<f32>, normal: vec3<f32>, ranges: ptr<function, ClusterableObjectIndexRanges>) -> vec3<f32> {
    var total = vec3(0.0);
    var total_weight = 0.0;
    // Bevy 0.19.1's cluster assignment hard-codes light probes to layer 0.
    // Per-view probe gathering honors visibility/layers. Iterate that native
    // list so world and first-person receivers sample the identical field.
    for (var index = 0; index < light_probes.irradiance_volume_count; index++) {
        let probe = light_probes.irradiance_volumes[index];
        if probe.cubemap_index < 0 { continue; }
        let light_from_world = transpose_affine_matrix(probe.light_from_world_transposed);
        let position = (light_from_world * vec4(world_position, 1.0)).xyz;
        let falloff = max(probe.falloff, vec3(0.0001));
        let axis_weights = clamp((1.0 - 2.0 * abs(position)) / (2.0 * falloff), vec3(0.0), vec3(1.0));
        let boundary_weight = min(axis_weights.x, min(axis_weights.y, axis_weights.z));
        if boundary_weight <= 0.0 { continue; }
#ifdef MULTIPLE_LIGHT_PROBES_IN_ARRAY
        let field = irradiance_volumes[probe.cubemap_index];
#else
        let field = irradiance_volume;
#endif
        let grid = vec3<f32>(textureDimensions(field) / vec3(16u, 13u, 1u));
        let spacing = 1.0 / (length(light_from_world[0].xyz) * grid.x);
        // Bias into the receiving hemisphere, preserving thin-wall visibility.
        let unit_position = (light_from_world * vec4(world_position + normal * min(0.2, spacing * 0.1), 1.0)).xyz;
        let diagonal = vec3(light_from_world[0].x, light_from_world[1].y, light_from_world[2].z);
        let center = -light_from_world[3].xyz / diagonal;
        let sample = sample_volume(probe.cubemap_index, unit_position, normal, spacing, center);
        // Prefer the finer cascade, with the engine's smooth boundary weighting.
        let weight = boundary_weight * sample.weight / (spacing * spacing);
        total += sample.irradiance * probe.intensity * weight;
        total_weight += weight;
    }
    return total / max(total_weight, 0.000001);
}
#endif

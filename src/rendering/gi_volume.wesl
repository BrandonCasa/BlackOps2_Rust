import bevy_pbr::light_probe::transpose_affine_matrix;
import bevy_pbr::render::mesh_view_bindings::{irradiance_volumes, irradiance_volume, light_probes};
import bevy_pbr::render::clustered_forward::ClusterableObjectIndexRanges;


// SAMPLING_HELPER

// Match native LightProbes and its texture binding array. Keep only metadata
// and a rejected-index mask live instead of an eight-candidate register array.
@if(IRRADIANCE_VOLUMES_ARE_USABLE) const GI_MAX_VIEW_VOLUMES: i32 = 8;

@if(IRRADIANCE_VOLUMES_ARE_USABLE) fn gi_volume_candidate(index: i32, world_position: vec3<f32>) -> vec2<f32> {
    let probe = light_probes.irradiance_volumes[index];
    if probe.cubemap_index < 0 { return vec2(1000000.0, 0.0); }
    let light_from_world = transpose_affine_matrix(probe.light_from_world_transposed);
    let position = (light_from_world * vec4(world_position, 1.0)).xyz;
    let falloff = max(probe.falloff, vec3(0.0001));
    let axis_weights = clamp((1.0 - 2.0 * abs(position)) / (2.0 * falloff), vec3(0.0), vec3(1.0));
    let boundary = min(axis_weights.x, min(axis_weights.y, axis_weights.z));
    if boundary <= 0.0 { return vec2(1000000.0, 0.0); }
    @if(IRRADIANCE_VOLUMES_ARE_USABLE && MULTIPLE_LIGHT_PROBES_IN_ARRAY) let field = irradiance_volumes[probe.cubemap_index];
    @if(IRRADIANCE_VOLUMES_ARE_USABLE && !MULTIPLE_LIGHT_PROBES_IN_ARRAY) let field = irradiance_volume;
    let grid = vec3<f32>(textureDimensions(field) / vec3(16u, 13u, 1u));
    let spacing = 1.0 / (length(light_from_world[0].xyz) * grid.x);
    return vec2(spacing, boundary);
}

@if(IRRADIANCE_VOLUMES_ARE_USABLE) fn gi_sample_candidate(index: i32, world_position: vec3<f32>, normal: vec3<f32>, spacing: f32) -> GiSample {
    let probe = light_probes.irradiance_volumes[index];
    let light_from_world = transpose_affine_matrix(probe.light_from_world_transposed);
    // Keep the existing receiver bias, relocation and visibility-aware sampler.
    let unit_position = (light_from_world * vec4(world_position + normal * min(0.02, spacing * 0.01), 1.0)).xyz;
    let diagonal = vec3(light_from_world[0].x, light_from_world[1].y, light_from_world[2].z);
    let center = -light_from_world[3].xyz / diagonal;
    let sample = sample_volume(probe.cubemap_index, unit_position, normal, spacing, center);
    return GiSample(sample.irradiance * probe.intensity, sample.weight);
}

@if(IRRADIANCE_VOLUMES_ARE_USABLE) fn irradiance_volume_light(world_position: vec3<f32>, normal: vec3<f32>, ranges: ptr<function, ClusterableObjectIndexRanges>) -> vec3<f32> {
    // Native clustered assignment hard-codes probes to layer 0. Its per-view
    // list honors visibility/layers and is shared by world and first-person.
    let volume_count = clamp(light_probes.irradiance_volume_count, 0, GI_MAX_VIEW_VOLUMES);
    var rejected = 0u;
    for (var attempt = 0; attempt < volume_count; attempt++) {
        var finest_index = -1;
        var finest_spacing = 1000000.0;
        var finest_boundary = 0.0;
        for (var index = 0; index < volume_count; index++) {
            if (rejected & (1u << u32(index))) != 0u { continue; }
            let candidate = gi_volume_candidate(index, world_position);
            // Strict inequality retains the original first-index tie rule.
            if candidate.y > 0.0 && candidate.x < finest_spacing {
                finest_index = index;
                finest_spacing = candidate.x;
                finest_boundary = candidate.y;
            }
        }
        if finest_index < 0 { return vec3(0.0); }
        let finest = gi_sample_candidate(finest_index, world_position, normal, finest_spacing);
        if finest.weight <= 0.0 {
            rejected |= 1u << u32(finest_index);
            continue;
        }
        // Supported black probes retain ownership. A supported interior needs
        // one full field lookup regardless of overlapping coarse volumes.
        if finest_boundary == 1.0 { return finest.irradiance; }

        var total = vec3(0.0);
        var total_weight = 0.0;
        for (var index = 0; index < volume_count; index++) {
            if index == finest_index || (rejected & (1u << u32(index))) != 0u { continue; }
            let candidate = gi_volume_candidate(index, world_position);
            if candidate.y <= 0.0 { continue; }
            let sample = gi_sample_candidate(index, world_position, normal, candidate.x);
            if sample.weight <= 0.0 { continue; }
            let weight = candidate.y / (candidate.x * candidate.x);
            total += sample.irradiance * weight;
            total_weight += weight;
        }
        let coarse = total / max(total_weight, 0.000001);
        return mix(coarse, finest.irradiance, finest_boundary);
    }
    return vec3(0.0);
}

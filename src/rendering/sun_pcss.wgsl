// Orthographic directional PCSS: penumbra radius = blocker separation *
// tan(sun angular radius). Reverse-Z differences become world distances
// before filtering; they never use a camera-fitted perspective depth ratio.
fn sample_directional_shadow_map_pcss(
    light_local: vec2<f32>, depth: f32, array_index: i32,
    frag_coord_xy: vec2<f32>, texel_size: f32, angular_radius: f32,
    depth_world_scale: f32, search_distance: f32, world_plane: vec2<f32>,
) -> f32 {
    let tangent = tan(angular_radius);
    let blocker = search_for_blockers_in_shadow_map(
        light_local, depth, array_index, texel_size, tangent * search_distance);
    if blocker == 0.0 { return 1.0; }
    let separation = max(blocker - depth, 0.0) * depth_world_scale;
    let radius = max(separation * tangent, 0.5 * texel_size);
    let shadow_map_size = vec2<f32>(textureDimensions(view_bindings::directional_shadow_textures));
    let scale = radius / (texel_size * shadow_map_size);
    let rotation = random_rotation_matrix(world_plane * 64.0, false);
    var sum = 0.0;
    sum += sample_shadow_map_hardware(light_local + rotation * utils::SPIRAL_OFFSET_0_ * scale, depth, array_index);
    sum += sample_shadow_map_hardware(light_local + rotation * utils::SPIRAL_OFFSET_1_ * scale, depth, array_index);
    sum += sample_shadow_map_hardware(light_local + rotation * utils::SPIRAL_OFFSET_2_ * scale, depth, array_index);
    sum += sample_shadow_map_hardware(light_local + rotation * utils::SPIRAL_OFFSET_3_ * scale, depth, array_index);
    sum += sample_shadow_map_hardware(light_local + rotation * utils::SPIRAL_OFFSET_4_ * scale, depth, array_index);
    sum += sample_shadow_map_hardware(light_local + rotation * utils::SPIRAL_OFFSET_5_ * scale, depth, array_index);
    sum += sample_shadow_map_hardware(light_local + rotation * utils::SPIRAL_OFFSET_6_ * scale, depth, array_index);
    sum += sample_shadow_map_hardware(light_local + rotation * utils::SPIRAL_OFFSET_7_ * scale, depth, array_index);
    return sum * 0.125;
}

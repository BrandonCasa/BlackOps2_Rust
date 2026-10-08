
// Orthographic directional PCSS: penumbra radius = blocker separation *
// tan(sun angular radius). Reverse-Z differences become world distances
// before filtering; they never use a camera-fitted perspective depth ratio.
fn sample_directional_shadow_map_pcss(
    light_local: vec2<f32>, depth: f32, array_index: i32,
    frag_coord_xy: vec2<f32>, texel_size: f32, angular_radius: f32,
    depth_world_scale: f32, search_distance: f32, world_plane: vec2<f32>,
    receiver_gradient: vec2<f32>,
) -> f32 {
    let tangent = tan(angular_radius);
    let shadow_map_size = vec2<f32>(textureDimensions(view_bindings::directional_shadow_textures));
    // IGN is designed for integer screen pixels. Feeding continuous light-plane
    // coordinates to its linear hash creates coherent stripes on curved rigs.
    // Avalanche integer world cells instead; stable in motion, no linear bands.
    let cell = vec2<i32>(floor(world_plane * 1024.0));
    var hash = (bitcast<u32>(cell.x) * 0x9e3779b9u) ^ (bitcast<u32>(cell.y) * 0x85ebca6bu);
    hash = (hash ^ (hash >> 16u)) * 0x7feb352du;
    hash = (hash ^ (hash >> 15u)) * 0x846ca68bu;
    hash ^= hash >> 16u;
    let angle = f32(hash & 0x00ffffffu) * (2.0 * PI / 16777216.0);
    let rotation = mat2x2(cos(angle), -sin(angle), sin(angle), cos(angle));
    let search_scale = tangent * search_distance / (texel_size * shadow_map_size);
    var blockers = vec2(0.0);
    let max_correction = 2.0 * texel_size / depth_world_scale;
    let footprint_bias = min(dot(abs(receiver_gradient), 0.5 / shadow_map_size),
        0.5 * texel_size / depth_world_scale);
    // Raw nearest depths retain real blockers. Linear depth interpolation across
    // a silhouette invents blockers; comparing every tap to the center depth
    // also mistakes a tilted receiver for an occluder.
    // Include the center and keep blocker admission deterministic. Rotating a
    // sparse search lets neighboring receivers alternate between no blocker
    // (full sunlight) and a large penumbra underneath the same narrow caster.
    for (var i = 0u; i < 17u; i++) {
        var uv = light_local;
        if i > 0u {
            let a = f32(i-1u) * 2.39996323;
            let r = sqrt((f32(i)-0.5)/16.0);
            uv += vec2(cos(a),sin(a)) * r * search_scale;
        }
        let pixel = vec2<i32>(floor(uv * shadow_map_size));
        if any(pixel < vec2<i32>(0)) || any(pixel >= vec2<i32>(shadow_map_size)) { continue; }
        let offset = (vec2<f32>(pixel) + 0.5) / shadow_map_size - light_local;
        let receiver = depth + clamp(dot(receiver_gradient, offset), -max_correction, max_correction) + footprint_bias;
        let blocker = textureLoad(view_bindings::directional_shadow_textures, pixel, array_index, 0);
        let gap = blocker - receiver;
        if gap > 0.0 { blockers += vec2(gap, 1.0); }
    }
    // A search miss cannot override a real center comparison.
    if blockers.y == 0.0 {
        return sample_shadow_map_hardware(light_local, depth + footprint_bias, array_index);
    }
    let separation = blockers.x / blockers.y * depth_world_scale;
    let radius = max(separation * tangent, 0.5 * texel_size);
    let scale = radius / (texel_size * shadow_map_size);
    // Hardware PCF still covers neighboring texels around each corrected tap.
    // Cover that footprint in reverse Z without a global detached-shadow bias.
    var sum = 0.0;
    // Fixed low-discrepancy disk, with interleaved sixteen-tap admission.
    let offsets = array(
        vec2(0.0883883,0.0000000), vec2(-0.1128861,0.1034129), vec2(0.0172790,-0.1968856), vec2(0.1422856,0.1855864),
        vec2(-0.2611116,-0.0461870), vec2(0.2473477,-0.1573424), vec2(-0.0827330,0.3077625), vec2(-0.1577807,-0.3037972),
        vec2(0.3423211,0.1250151), vec2(-0.3561280,0.1470045), vec2(0.1716772,-0.3668643), vec2(0.1268651,0.4044660),
        vec2(-0.3823729,-0.2215929), vec2(0.4485670,-0.0986162), vec2(-0.2737535,0.3893861), vec2(-0.0632434,-0.4880448),
        vec2(0.3882524,0.3272195), vec2(-0.5224660,0.0216056), vec2(0.3810988,-0.3792442), vec2(-0.0254970,0.5513959),
        vec2(-0.3626168,-0.4345361), vec2(0.5744250,0.0772881), vec2(-0.4867091,0.3386395), vec2(0.1329969,-0.5911847),
        vec2(0.3076150,0.5368291), vec2(-0.6013578,-0.1918496), vec2(0.5841426,-0.2698887), vec2(-0.2530652,0.6046863),
        vec2(-0.2258551,-0.6279347), vec2(0.6009762,0.3158562), vec2(-0.6675333,0.1759596), vec2(0.3794305,-0.5901017),
        vec2(0.1206991,0.7023135), vec2(-0.5720078,-0.4429950), vec2(0.7317019,-0.0606200), vec2(-0.5057603,0.5467120),
        vec2(0.0036840,-0.7551814), vec2(0.5143049,0.5669461), vec2(-0.7722949,-0.0715761), vec2(0.6257873,-0.4749503),
        vec2(-0.1423806,0.7826495), vec2(-0.4288839,-0.6815395), vec2(0.7859201,0.2153881), vec2(-0.7334855,0.3764127),
        vec2(0.2898617,-0.7818521), vec2(0.3179115,0.7809416), vec2(-0.7702639,-0.3650424), vec2(0.8232633,-0.2538209),
        vec2(-0.4401565,0.7510491), vec2(-0.1846432,-0.8598514), vec2(0.7241774,0.5144217), vec2(-0.8901574,0.1109386),
        vec2(0.5870543,-0.6896954), vec2(0.0333199,0.9136888), vec2(-0.6477270,-0.6572764), vec2(0.9300141,0.0475522),
        vec2(-0.7243231,0.5984718), vec2(0.1309753,-0.9387667), vec2(0.5422049,0.7874493), vec2(-0.9396490,-0.2162110),
        vec2(0.8459368,-0.4792739), vec2(-0.3024919,0.9324356), vec2(-0.4100971,-0.8991011), vec2(0.9169759,0.3890280));
    for (var i = 0u; i < 16u; i++) {
        let offset = rotation * offsets[i*4u] * scale;
        sum += sample_shadow_map_hardware(light_local + offset,
            depth + clamp(dot(receiver_gradient, offset), -max_correction, max_correction) + footprint_bias, array_index);
    }
    // Spend additional taps only in a resolved penumbra. Interior shadow and
    // light remain exact endpoint comparisons; no history or image blur.
    if sum == 0.0 || sum == 16.0 { return sum * 0.0625; }
    for (var i = 0u; i < 64u; i++) {
        if i % 4u == 0u { continue; }
        let offset = rotation * offsets[i] * scale;
        sum += sample_shadow_map_hardware(light_local + offset,
            depth + clamp(dot(receiver_gradient, offset), -max_correction, max_correction) + footprint_bias, array_index);
    }
    return sum * 0.015625;
}

// Plane equation in the cascade's UV/depth coordinates. Use the geometric
// normal, never the texture normal, so detail cannot move shadow receivers.
fn directional_receiver_gradient(clip: mat4x4<f32>, normal: vec3<f32>) -> vec2<f32> {
    let x = vec3(clip[0].x, clip[1].x, clip[2].x);
    let y = vec3(clip[0].y, clip[1].y, clip[2].y);
    let z = vec3(clip[0].z, clip[1].z, clip[2].z);
    let cosine = dot(normalize(normal), normalize(z));
    // At a silhouette the tangent plane has no finite shadow-map projection.
    let denominator = select(-1.0, 1.0, cosine >= 0.0) * max(abs(cosine), 0.05);
    return vec2(-dot(normalize(normal), normalize(x)) / length(x),
        dot(normalize(normal), normalize(y)) / length(y)) * (2.0 * length(z) / denominator);
}

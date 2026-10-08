
// This software contains source code provided by NVIDIA Corporation.
// WGSL adaptation of RTXGI-DDGI f33e496: octahedral lookup and visibility.
struct GiSample { irradiance: vec3<f32>, weight: f32 }
fn gi_sign(v: vec2<f32>) -> vec2<f32> {
    return select(vec2(-1.0), vec2(1.0), v >= vec2(0.0));
}
fn gi_oct_direction(uv: vec2<f32>) -> vec3<f32> {
    var v = vec3(uv, 1.0 - abs(uv.x) - abs(uv.y));
    if v.z < 0.0 { v = vec3((1.0 - abs(v.yx)) * gi_sign(v.xy), v.z); }
    return normalize(v);
}
fn gi_oct_coordinates(direction: vec3<f32>) -> vec2<f32> {
    var uv = direction.xy / max(dot(abs(direction), vec3(1.0)), 0.000001);
    if direction.z < 0.0 { uv = (1.0 - abs(uv.yx)) * gi_sign(uv); }
    return uv * 0.5 + 0.5;
}
fn gi_wrapped_texel(input: vec2<i32>, size: i32) -> vec2<i32> {
    var p = input;
    if p.x < 0 { p = vec2(0, size - 1 - p.y); }
    if p.x >= size { p = vec2(size - 1, size - 1 - p.y); }
    if p.y < 0 { p = vec2(size - 1 - p.x, 0); }
    if p.y >= size { p = vec2(size - 1 - p.x, size - 1); }
    return p;
}
fn gi_physical(cell: vec3<i32>, grid: vec3<i32>) -> vec3<i32> {
    return ((cell % grid) + grid) % grid;
}
fn gi_coordinate(physical: vec3<i32>, index: u32) -> vec3<i32> {
    return vec3(physical.x * 16 + i32(index % 16u), physical.y * 13 + i32(index / 16u), physical.z);
}
fn gi_read(field: texture_3d<f32>, cell: vec3<i32>, index: u32) -> vec4<f32> {
    let grid = vec3<i32>(textureDimensions(field)) / vec3(16, 13, 1);
    return textureLoad(field, gi_coordinate(gi_physical(cell, grid), index), 0);
}
fn gi_direction_sample(field: texture_3d<f32>, cell: vec3<i32>, direction: vec3<f32>, visibility: bool) -> vec4<f32> {
    let size = select(8, 16, visibility);
    let coordinate = gi_oct_coordinates(direction) * f32(size) - 0.5;
    let base = vec2<i32>(floor(coordinate));
    let alpha = fract(coordinate);
    var result = vec4(0.0);
    for (var i = 0u; i < 4u; i++) {
        let bit = vec2<i32>(i32(i & 1u), i32(i >> 1u));
        let p = gi_wrapped_texel(base + bit, size);
        let index = u32(p.y * size + p.x);
        var value = vec4(0.0);
        if visibility {
            let packed = gi_read(field, cell, 64u + index / 2u);
            value = vec4(select(packed.xy, packed.zw, (index & 1u) != 0u), 0.0, 0.0);
        } else {
            let encoded = gi_read(field, cell, index);
            // Decode before either directional or spatial interpolation. Mixing
            // gamma/square-root RGB darkens colored transport and changes hue.
            value = vec4(pow(max(encoded.rgb, vec3(0.0)), vec3(5.0)), encoded.a);
        }
        let weight = select(1.0 - alpha, alpha, bit != vec2(0));
        result += value * weight.x * weight.y;
    }
    return result;
}
// Probe tiles store E/(2*pi) with gamma encoding, distance moments,
// world-cell ownership and relocation in spacing units.
fn sample_gi_field(field: texture_3d<f32>, unit_position: vec3<f32>, normal: vec3<f32>, spacing: f32, center: vec3<f32>) -> GiSample {
    let grid = vec3<i32>(textureDimensions(field)) / vec3(16, 13, 1);
    let world_position = center + unit_position * vec3<f32>(grid) * spacing;
    let coordinate = world_position / spacing - 0.5;
    let base = vec3<i32>(floor(coordinate));
    let fraction = fract(coordinate);
    let origin = vec3<i32>(round(center / spacing)) - grid / 2;
    var total = vec3(0.0);
    var weight_sum = 0.0;
    var support_sum = 0.0;
    for (var corner = 0u; corner < 8u; corner++) {
        let bit = vec3<i32>(i32(corner & 1u), i32((corner >> 1u) & 1u), i32((corner >> 2u) & 1u));
        let cell = base + bit;
        if any(cell < origin) || any(cell >= origin + grid) { continue; }
        let state = gi_read(field, cell, 192u);
        if state.w <= 0.0 || any(state.xyz != vec3<f32>(cell)) { continue; }
        let offset = gi_read(field, cell, 193u).xyz;
        let delta = coordinate - vec3<f32>(cell) - offset;
        let distance = length(delta);
        let direction = delta / max(distance, 0.000001);
        let moments = gi_direction_sample(field, cell, direction, true).xy;
        let variance = abs(moments.y - moments.x * moments.x);
        let difference = max(distance - moments.x, 0.0);
        let chebyshev = variance / max(variance + difference * difference, 0.000001);
        let visibility = select(1.0, chebyshev * chebyshev * chebyshev, distance > moments.x);
        let wrap = (dot(normal, -direction) + 1.0) * 0.5;
        let trilinear = max(vec3(0.001), select(1.0 - fraction, fraction, bit != vec3(0)));
        let support = (wrap * wrap + 0.2) * trilinear.x * trilinear.y * trilinear.z;
        support_sum += support;
        // A blocked probe is not ambient light. The SDK's visibility floor is
        // a fallback for missing probes; normalizing only that floor lets an
        // entirely occluded neighborhood illuminate the other side of a wall.
        var weight = (wrap * wrap + 0.2) * visibility;
        if weight < 0.2 { weight *= weight * weight / 0.04; }
        weight *= trilinear.x * trilinear.y * trilinear.z;
        let linear = gi_direction_sample(field, cell, normal, false).rgb;
        total += linear * weight;
        weight_sum += weight;
    }
    let irradiance = total / max(weight_sum, 0.000001);
    let confidence = smoothstep(0.01, 0.2, weight_sum / max(support_sum, 0.000001));
    // Bevy expects diffuse incident irradiance divided by pi.
    // Keep initialized support separate from visibility: black fine probes
    // must retain ownership instead of falling back to bright coarse probes.
    return GiSample(irradiance * 2.0 * confidence, support_sum);
}

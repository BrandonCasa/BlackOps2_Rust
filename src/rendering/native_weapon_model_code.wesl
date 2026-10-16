// Exact native code-target consumers: UI3D PS229e7ac2 and sonar PSfc8afb12.
// Every generated texture is an explicit input from its real producer.
import package::rendering::native_model::NativeModelScene;
struct NativeUi3dReticleConstants { source: array<vec4<f32>, 5> }
fn native_ui3d_reticle(uv: vec2<f32>, fragment_xy: vec2<f32>, target_size: vec2<f32>,
    window5: vec4<f32>, scene: NativeModelScene, material: NativeUi3dReticleConstants,
    ui3d: texture_2d<f32>, ui3d_sampler: sampler,
    noise_map: texture_2d<f32>, noise_sampler: sampler) -> vec4<f32> {
    let c = material.source;
    let mask = textureSample(ui3d, ui3d_sampler, fma(uv, window5.xy, window5.zw));
    let glow = fma(vec3<f32>(exp2(log2(mask.g) * c[1].y)), c[0].rgb,
        exp2(log2(mask.a) * c[1].y) * c[4].rgb);
    let noise = textureSample(noise_map, noise_sampler, (fragment_xy / target_size) * c[1].x).r;
    let transmission = fma(-c[1].z, noise, 1.0);
    let reticle = fma(vec3<f32>(mask.r), c[2].rgb, mask.b * c[3].rgb);
    let gamma = fma(reticle, vec3<f32>(transmission), glow * noise) * c[0].a;
    return vec4<f32>(sqrt((gamma * gamma) * scene.world.hdr_control0.x), 1.0);
}
struct NativeSonarConstants { source: array<vec4<f32>, 10> }
// VS c436f53e's deterministic eight-component noise generator. Only ZW reach PS.
fn native_sonar_noise(seconds: f32) -> vec4<f32> {
    let k = vec4<f32>(81.23948669433594, 17.348024368286133, 37.34983825683594, 59.394840240478516);
    var noise = vec4<f32>(seconds);
    noise.x = fract(dot(noise, k));
    noise.y = fract(dot(noise, k));
    noise.z = fract(dot(noise, k));
    noise.w = fract(dot(noise, k));
    noise.x = fract(dot(noise, k));
    noise.y = fract(dot(noise, k));
    noise.z = fract(dot(noise, k));
    noise.w = fract(dot(noise, k));
    return noise;
}
fn native_sonar_fragment(uv: vec2<f32>, fragment_xy: vec2<f32>, target_size: vec2<f32>,
    fog_color: vec3<f32>, fog_transmission: f32, noise_zw: vec2<f32>, seconds: f32,
    z_near: f32, weapon0: vec4<f32>, weapon4: vec4<f32>, scene: NativeModelScene,
    material: NativeSonarConstants,
    static_noise: texture_2d<f32>, static_sampler: sampler,
    lookup: texture_2d<f32>, lookup_sampler: sampler,
    float_z: texture_2d<f32>, float_z_sampler: sampler,
    sonar_color: texture_2d<f32>, sonar_color_sampler: sampler,
    sonar_depth: texture_depth_2d, sonar_depth_sampler: sampler) -> vec4<f32> {
    let c = material.source;
    let screen_uv = fragment_xy / target_size;
    let sonar_uv = (screen_uv - weapon4.xy) / weapon4.zw;
    let depth_sample = textureSample(sonar_depth, sonar_depth_sampler, sonar_uv);
    let color = textureSample(sonar_color, sonar_color_sampler, sonar_uv);
    let raw_sonar_depth = z_near / max(abs(depth_sample), 0.0);
    let sonar_distance = fma(select(0.0, 1.0, raw_sonar_depth >= 10000.0), -raw_sonar_depth, raw_sonar_depth);
    let phase = fract(seconds / c[4].x);
    let pulse_distance = (c[4].x / c[4].y) * c[1].x;
    let player_phase = fract((fma(pulse_distance, phase, -sonar_distance) / pulse_distance) + 0.5) - 0.5;
    let player_lookup = textureSample(lookup, lookup_sampler, vec2<f32>(player_phase / c[6].x + 0.5, 0.5));
    let player_visibility = min(color.a, max(player_lookup.g, c[6].z));
    let player_intensity = dot(color.rgb, vec3<f32>(0.33000001311302185));
    var player = (player_intensity * c[1].yzw) * c[5].z;
    let center_sample = textureSample(float_z, float_z_sampler, screen_uv).r;
    let distance = z_near / max(abs(center_sample), 0.0);
    let adjusted_distance = fma(select(0.0, 1.0, 10.0 >= distance), 100000.0 - distance, distance);
    player *= select(0.0, 1.0, sonar_distance >= adjusted_distance + 10.0);
    player *= player_visibility;
    let offset = c[4].z * 0.004999999888241291;
    let d0 = z_near / max(abs(textureSample(float_z, float_z_sampler, screen_uv + vec2<f32>(-offset, offset)).r), 0.0);
    let d1 = z_near / max(abs(textureSample(float_z, float_z_sampler, screen_uv + vec2<f32>(-offset, -offset)).r), 0.0);
    let d2 = z_near / max(abs(textureSample(float_z, float_z_sampler, screen_uv + vec2<f32>(offset, -offset)).r), 0.0);
    let d3 = z_near / max(abs(textureSample(float_z, float_z_sampler, screen_uv + vec2<f32>(offset, offset)).r), 0.0);
    let minimum = min(min(min(min(d1, min(d0, 10000000.0)), d2), d3), distance);
    let total = d3 + (d2 + (d1 + d0));
    let edge = clamp((abs(fma(-total, 0.25, distance)) * c[4].w) / distance, 0.0, 1.0);
    let shape_phase = fract((fma(pulse_distance, phase, -minimum) / pulse_distance) + 0.5) - 0.5;
    let pulse_lookup = textureSample(lookup, lookup_sampler, vec2<f32>(shape_phase / c[5].w + 0.5, 0.5));
    let edge_lookup = textureSample(lookup, lookup_sampler, vec2<f32>(shape_phase / c[6].y + 0.5, 0.5));
    let noise = textureSample(static_noise, static_sampler, uv / c[0].yz + noise_zw);
    let scatter = fma(noise.g - 0.5, c[2].z, 1.0);
    let line_transmission = 1.0 - dot(vec2<f32>(c[3].w), vec2<f32>(noise.b));
    let reciprocal_range = 1.0 / (c[2].x - c[1].x);
    let scattered_phase = clamp(fma(distance, scatter, -c[1].x) * reciprocal_range, 0.0, 1.0);
    let scattered_far = fma(-fma(scattered_phase, -2.0, 3.0), scattered_phase * scattered_phase, 1.0);
    let half_min_depth = c[0].w * 0.5;
    let near_phase = clamp(fma(distance, scatter, -half_min_depth) * (1.0 / half_min_depth), 0.0, 1.0);
    let near_curve = (near_phase * near_phase) * fma(near_phase, -2.0, 3.0);
    let pulse_weight = ((near_curve * scattered_far) * pulse_lookup.r) * select(0.0, 1.0, minimum >= 0.10000000149011612);
    let pulse = (pulse_weight * c[7].rgb) * c[9].x;
    let sonar_phase = clamp((sonar_distance - c[1].x) * reciprocal_range, 0.0, 1.0);
    let sonar_far = fma(-fma(sonar_phase, -2.0, 3.0), sonar_phase * sonar_phase, 1.0);
    let sonar_near_phase = clamp(fma(-c[0].w, 0.5, sonar_distance) * (1.0 / half_min_depth), 0.0, 1.0);
    let sonar_near = (sonar_near_phase * sonar_near_phase) * fma(sonar_near_phase, -2.0, 3.0);
    var emission = fma(player, vec3<f32>(sonar_near * sonar_far), pulse);
    var edge_cube = (edge * edge) * edge;
    edge_cube = fma(select(0.0, 1.0, c[5].x >= minimum), -edge_cube, edge_cube);
    edge_cube = fma(select(0.0, 1.0, minimum >= c[5].y), -edge_cube, edge_cube);
    let edge_color = (edge_cube * c[8].rgb) * c[8].a;
    emission = fma(edge_color, vec3<f32>(edge_lookup.b), emission);
    emission *= line_transmission;
    emission *= 1.0 - weapon0.y;
    emission *= c[2].w;
    let fogged = fma(vec3<f32>(fog_transmission), fma(emission, emission, -fog_color), fog_color);
    return vec4<f32>(sqrt(fogged * scene.world.hdr_control0.x), 1.0);
}

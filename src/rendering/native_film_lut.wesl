// Source VS/PS900b8491 and 0045e620: generate the graded 32^3 lattice.
struct FilmConstants {
    ramp_a: vec4<f32>, ramp_b: vec4<f32>,
    red_rows: array<vec4<f32>, 3>, green_rows: array<vec4<f32>, 3>, blue_rows: array<vec4<f32>, 3>,
    gamma: vec4<f32>, saturation: vec4<f32>, blend: vec4<f32>, banks: vec4<f32>,
    color_matrix: array<vec4<f32>, 3>,
}
@group(0) @binding(0) var authored_lut: texture_2d<f32>;
@group(0) @binding(1) var<uniform> film: FilmConstants;
struct Input { @builtin(position) position: vec4<f32>, @location(0) uv: vec2<f32> }
fn tonal(value: vec4<f32>, rows: array<vec4<f32>, 3>, weights: vec3<f32>) -> f32 {
    return clamp(dot(vec3<f32>(dot(value, rows[0]), dot(value, rows[1]), dot(value, rows[2])), weights), 0.0, 1.0);
}
@fragment fn generate(input: Input) -> @location(0) vec4<f32> {
    let lattice = vec2<i32>(input.position.xy);
    // Point/clamp source sampler0xe1 at texel centers is exactly textureLoad.
    let current = textureLoad(authored_lut, lattice + vec2<i32>(0, i32(film.banks.x) * 32), 0);
    let previous = textureLoad(authored_lut, lattice + vec2<i32>(0, i32(film.banks.y) * 32), 0);
    let raw = fma(current, vec4<f32>(film.banks.z), previous * film.banks.w);
    let y = dot(raw.rgb, vec3<f32>(0.21258544921875, 0.7151947021484375, 0.0722198486328125));
    let ramp = clamp(fma(vec4<f32>(y), film.ramp_a.xzyw, film.ramp_b.xzyw), vec4<f32>(0.0), vec4<f32>(1.0));
    let r = vec3<f32>(ramp.x, ramp.y * ramp.w, ramp.z);
    let eased = (r * r) * fma(r, vec3<f32>(-2.0), vec3<f32>(3.0));
    let weights = eased * (1.0 / dot(eased, vec3<f32>(1.0)));
    let linear = vec4<f32>(raw.rgb * raw.rgb, raw.a);
    let graded = vec3<f32>(tonal(linear, film.red_rows, weights), tonal(linear, film.green_rows, weights), tonal(linear, film.blue_rows, weights));
    let powered = exp2(log2(graded) * film.gamma.rgb);
    let grey = dot(powered, film.saturation.rgb);
    let saturated = fma(vec3<f32>(film.saturation.w), powered - vec3<f32>(grey), vec3<f32>(grey));
    let mixed = vec4<f32>(fma(film.blend.rgb, saturated - linear.rgb, linear.rgb), 1.0);
    let transformed = clamp(vec3<f32>(dot(mixed, film.color_matrix[0]), dot(mixed, film.color_matrix[1]), dot(mixed, film.color_matrix[2])), vec3<f32>(0.0), vec3<f32>(1.0));
    // The RGB10A2 attachment quantizes this before the later trilinear lookup.
    return vec4<f32>(sqrt(transformed), raw.a);
}

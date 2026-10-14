struct Parameters {
    flags: u32,
    count: u32,
    padding: vec2<u32>,
    parms: vec4<f32>,
    images: array<vec4<u32>, 7>,
}
struct Sample {
    uv01: vec4<f32>,
    uv2_color: vec4<f32>,
    transform: vec4<f32>,
    second_transform: vec4<f32>,
}
struct Pixel {
    color: vec4<f32>,
    normal: vec4<f32>,
    response: vec4<f32>,
    specular: vec4<f32>,
}
@group(0) @binding(0) var<uniform> recipe: Parameters;
@group(0) @binding(1) var<storage, read> images: array<vec4<f32>>;
@group(0) @binding(2) var<storage, read> samples: array<Sample>;
@group(0) @binding(3) var<storage, read_write> output: array<Pixel>;

fn flag(bit: u32) -> bool { return (recipe.flags & bit) != 0u; }
fn address(value: i32, size: i32, clamped: bool) -> u32 {
    if clamped { return u32(clamp(value, 0, size - 1)); }
    return u32(((value % size) + size) % size);
}
fn at(image: vec4<u32>, x: i32, y: i32) -> vec4<f32> {
    return images[image.z + address(y, i32(image.y), (image.w & 64u) != 0u) * image.x
        + address(x, i32(image.x), (image.w & 32u) != 0u)];
}
fn sample_image(index: u32, uv: vec2<f32>) -> vec4<f32> {
    let image = recipe.images[index];
    var u = uv.x - floor(uv.x);
    var v = uv.y - floor(uv.y);
    if (image.w & 32u) != 0u { u = clamp(uv.x, 0.0, 1.0); }
    if (image.w & 64u) != 0u { v = clamp(uv.y, 0.0, 1.0); }
    let x = u * f32(image.x) - 0.5;
    let y = v * f32(image.y) - 0.5;
    if (image.w & 7u) == 1u { return at(image, i32(floor(x + 0.5)), i32(floor(y + 0.5))); }
    let ix = i32(floor(x));
    let iy = i32(floor(y));
    let fx = x - floor(x);
    let fy = y - floor(y);
    return (at(image, ix, iy) * (1.0 - fx) + at(image, ix + 1, iy) * fx) * (1.0 - fy)
        + (at(image, ix, iy + 1) * (1.0 - fx) + at(image, ix + 1, iy + 1) * fx) * fy;
}
fn roughness(gloss: f32) -> f32 { return pow(2.0 / (pow(2.0, 13.0 * gloss) + 2.0), 0.25); }
fn decode(value: vec2<f32>) -> vec2<f32> { return value * bitcast<f32>(0x40808102u) + bitcast<f32>(0xc0010204u); }
fn normal(slope: vec2<f32>) -> vec4<f32> {
    let length = sqrt(slope.x * slope.x + slope.y * slope.y + 1.0);
    return vec4<f32>(slope.x / length, slope.y / length, 1.0 / length, 1.0);
}
fn brightness(rgb: vec3<f32>) -> f32 { return sqrt(rgb.x * rgb.x * 0.299 + rgb.y * rgb.y * 0.587 + rgb.z * rgb.z * 0.114); }


fn fit_response(value: Pixel, rgb: vec3<f32>, gloss: f32) -> Pixel {
    var pixel = value;
    let energy = 1.0 - max(0.0, max(rgb.x, max(rgb.y, rgb.z)));
    let specular_brightness = brightness(rgb);
    var metallic = 0.0;
    if specular_brightness >= 0.04 {
        let b = brightness(pixel.color.rgb) * energy / 0.96 + specular_brightness - 0.08;
        metallic = clamp((-b + sqrt(max(b * b - 0.16 * (0.04 - specular_brightness), 0.0))) / 0.08, 0.0, 1.0);
    }
    let from_diffuse = pixel.color.rgb * energy / 0.96 / max(1.0 - metallic, 0.000001);
    let from_specular = (rgb - vec3<f32>(0.04 * (1.0 - metallic))) / max(metallic, 0.000001);
    pixel.color = vec4<f32>(clamp(from_diffuse + (from_specular - from_diffuse) * metallic * metallic, vec3<f32>(0.0), vec3<f32>(1.0)), 1.0);
    pixel.response = vec4<f32>(1.0, roughness(gloss), metallic, 1.0);
    pixel.specular = vec4<f32>(rgb, gloss);
    return pixel;
}

@compute @workgroup_size(64)
fn bake(@builtin(global_invocation_id) invocation: vec3<u32>) {
    let index = invocation.x;
    if index >= recipe.count { return; }
    let sample = samples[index];
    let uv0 = sample.uv01.xy;
    let uv1 = sample.uv01.zw;
    let uv2 = sample.uv2_color.xy;
    let green = sample.uv2_color.z;
    let blue = sample.uv2_color.w;
    let base = sample_image(0u, uv0);
    let n0 = sample_image(1u, uv0);
    let layer = sample_image(select(2u, 1u, flag(256u)), uv1);
    var result: Pixel;
    result.response = vec4<f32>(1.0, 1.0, 0.0, 1.0);
    result.specular = vec4<f32>(0.0);
    if flag(2u) || flag(4u) || flag(8u) || flag(256u) {
        var second = vec4<f32>(1.0);
        if flag(1u) { second = sample_image(3u, uv2); }
        let tint = vec3<f32>(1.0) + green * (layer.rgb - vec3<f32>(1.0));
        var color: vec3<f32>;
        if flag(8u) {
            let first = base.rgb + green * layer.a * (layer.rgb - base.rgb);
            var weight = 0.0;
            if flag(1u) { weight = blue * second.a; }
            color = first + weight * (second.rgb - first);
        } else if flag(4u) {
            color = (base.rgb + layer.rgb * green * layer.a) * (vec3<f32>(1.0) + blue * (second.rgb - vec3<f32>(1.0)));
        }
        else { color = base.rgb * tint * (vec3<f32>(1.0) + blue * (second.rgb - vec3<f32>(1.0))); }
        result.color = vec4<f32>(color * color, 1.0);
        if flag(64u) {
            var gloss = base.a;
            if flag(4u) && flag(1u) {
                let second_tint = vec3<f32>(1.0) + blue * (second.rgb - vec3<f32>(1.0));
                gloss = base.a * (second_tint.x * 0.3 + second_tint.y * 0.6 + second_tint.z * 0.1);
            } else if !flag(4u) && !flag(8u) { gloss = base.a * (tint.x * 0.3 + tint.y * 0.6 + tint.z * 0.1); }
            result.response = vec4<f32>(1.0, roughness(gloss), 0.0, gloss);
        }
        result.normal = vec4<f32>(0.0, 0.0, 1.0, 1.0);
        if !flag(256u) {
            var slope = decode(n0.xy);
            if flag(1024u) {
                let s = decode(sample_image(4u, uv2).xy);
                let rotated = vec2<f32>(s.x * sample.transform.x + s.y * sample.transform.y, s.x * sample.transform.z + s.y * sample.transform.w);
                slope = slope + blue * second.a * (rotated - slope);
            }
            result.normal = normal(slope);
        }
        if flag(128u) {
            let specular = sample_image(recipe.padding.x - 1u, uv0);
            result = fit_response(result, specular.rgb * specular.rgb, specular.a);
        }
        output[index] = result;
        return;
    }
    let n1 = sample_image(3u, uv1);
    var weight = layer.a * green;
    if flag(16u) {
        let a = green * bitcast<f32>(0x3f7f7ceeu) + bitcast<f32>(0x3a83126fu);
        let q = 1.0 - a;
        let low = clamp(q - recipe.parms.x * exp2(log2(a) * recipe.parms.y), 0.0, 1.0);
        let high = clamp(q + recipe.parms.x * exp2(log2(q) * recipe.parms.y), 0.0, 1.0);
        if high <= low {
            result.color.x = bitcast<f32>(0x7fc00001u);
            output[index] = result;
            return;
        }
        weight = clamp((layer.a - low) / (high - low), 0.0, 1.0);
    }
    var decal = vec4<f32>(1.0);
    if flag(1u) { decal = sample_image(4u, uv2); }
    let first = base.rgb + weight * (layer.rgb - base.rgb);
    var color = first * (vec3<f32>(1.0) + blue * (decal.rgb - vec3<f32>(1.0)));
    if flag(512u) { color = first + blue * decal.a * decal.rgb; }
    else if flag(2048u) { color = first + blue * decal.a * (decal.rgb - first); }
    result.color = vec4<f32>(color * color, 1.0);
    let s0 = decode(n0.xy);
    let s1 = decode(n1.xy);
    let rotated = vec2<f32>(s1.x * sample.transform.x + s1.y * sample.transform.y, s1.x * sample.transform.z + s1.y * sample.transform.w);
    var slope = s0 + weight * (rotated - s0);
    if flag(2048u) {
        let s = decode(sample_image(5u, uv2).xy);
        let rotated_second = vec2<f32>(s.x * sample.second_transform.x + s.y * sample.second_transform.y,
            s.x * sample.second_transform.z + s.y * sample.second_transform.w);
        slope = slope + blue * decal.a * (rotated_second - slope);
    }
    result.normal = normal(slope);
    if flag(32u) || flag(128u) {
        let k = bitcast<f32>(0x3e4ccccdu);
        var base_gloss = 0.0;
        if flag(64u) { base_gloss = base.a; }
        var specular = vec4<f32>(k, k, k, base_gloss);
        if flag(128u) { specular = sample_image(recipe.padding.x - 1u, uv0); }
        if flag(32u) {
            let layer_specular = sample_image(select(4u, 5u, flag(1u)), uv1);
            specular = specular + weight * (layer_specular - specular);
        }
        let rgb = specular.rgb * specular.rgb;
        var gloss = specular.a;
        if flag(1u) && !flag(512u) {
            let tint = vec3<f32>(1.0) + blue * (decal.rgb - vec3<f32>(1.0));
            gloss = gloss * (tint.x * 0.3 + tint.y * 0.6 + tint.z * 0.1);
        }
        result = fit_response(result, rgb, gloss);
    } else if flag(64u) { result.response = vec4<f32>(1.0, roughness(base.a), 0.0, base.a); }
    output[index] = result;
}

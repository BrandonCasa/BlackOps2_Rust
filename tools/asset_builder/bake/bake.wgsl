const DECAL: u32 = 1u;
const MULTIPLY: u32 = 2u;
const ADDITIVE: u32 = 4u;
const BLEND_COLOR: u32 = 8u;
const REVEAL: u32 = 16u;
const LAYER_SPECULAR: u32 = 32u;
const BASE_ALPHA_RESPONSE: u32 = 64u;
const BASE_SPECULAR: u32 = 128u;
const GEOMETRIC_NORMAL: u32 = 256u;
const SECOND_ADDITIVE: u32 = 512u;
const SECOND_NORMAL_COLOR: u32 = 1024u;
const SECOND_NORMAL_LAYER: u32 = 2048u;
const SECOND_REVEAL: u32 = 4096u;
const SECOND_SPECULAR: u32 = 8192u;
const THIRD_MULTIPLY: u32 = 16384u;
const LAYER_CUTOUT: u32 = 32768u;
const TILE_BLEND: u32 = 65536u;
const WATER_REVEAL: u32 = 131072u;
const PHONG_EMISSIVE: u32 = 262144u;
const EMISSIVE_FLICKER: u32 = 524288u;
const EMISSIVE_FRITZ: u32 = 1048576u;
const EMISSIVE_SCRIPT_MULTI: u32 = 2097152u;
const EMISSIVE_TOMB: u32 = 4194304u;
const SPECIAL_SURFACE: u32 = 0x7f0000u;

struct Parameters {
    flags: u32,
    count: u32,
    image_count: u32,
    last_image: u32,
    parms: vec4<f32>,
    second_parms: vec4<f32>,
    special_parms: array<vec4<f32>,3>,
    slots: vec4<u32>,
    images: array<vec4<u32>, 9>,
}
struct Sample {
    uv01: vec4<f32>,
    uv2_color: vec4<f32>,
    transform: vec4<f32>,
    second_transform: vec4<f32>,
    third: vec4<f32>,
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


fn reveal_weight(alpha: f32, vertex: f32, parms: vec4<f32>) -> f32 {
    let a = vertex * bitcast<f32>(0x3f7f7ceeu) + bitcast<f32>(0x3a83126fu);
    let q = 1.0 - a;
    let low = clamp(q - parms.x * exp2(log2(a) * parms.y), 0.0, 1.0);
    let high = clamp(q + parms.x * exp2(log2(q) * parms.y), 0.0, 1.0);
    if high <= low { return -1.0; }
    return clamp((alpha - low) / (high - low), 0.0, 1.0);
}

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

fn special_surface(uv: vec2<f32>) -> Pixel {
    var result: Pixel;
    result.response = vec4<f32>(1.0, 1.0, 0.0, 1.0);
    result.specular = vec4<f32>(0.0);
    if flag(EMISSIVE_FRITZ) {
        let base = sample_image(0u,uv);
        var color = base;
        var response = sample_image(2u,uv);
        if recipe.special_parms[1].x > 0.0 {
            let warp = sample_image(4u,uv * recipe.second_parms.w).xy * 2.0 - vec2<f32>(1.0);
            let warped_uv = uv + warp * recipe.second_parms.z;
            let warped_color = sample_image(0u,warped_uv);
            let luminance = dot(warped_color.rgb,vec3<f32>(0.3,0.59,0.11));
            let luma = luminance * luminance * recipe.special_parms[0].x;
            color = vec4<f32>(base.rgb + recipe.special_parms[1].x * (vec3<f32>(luma) - base.rgb),warped_color.a);
            let warped_specular = sample_image(2u,warped_uv);
            let specular_luma = dot(warped_specular.rgb,vec3<f32>(0.3,0.59,0.11));
            response = vec4<f32>(response.rgb + recipe.special_parms[1].x * (vec3<f32>(specular_luma) - response.rgb),response.a);
        }
        result.color = vec4<f32>(color.rgb * color.rgb,1.0);
        result.normal = normal(decode(sample_image(1u,uv).xy));
        result = fit_response(result,response.rgb * response.rgb,response.a);
        let intensity = recipe.parms.z * recipe.special_parms[1].y;
        result.response.a = intensity * intensity;
        if recipe.special_parms[0].y != 0.0 {result.color.a = color.a;}
        return result;
    }
    if flag(PHONG_EMISSIVE) || flag(EMISSIVE_FLICKER) || flag(EMISSIVE_SCRIPT_MULTI) || flag(EMISSIVE_TOMB) {
        let base = sample_image(0u, uv);
        result.color = vec4<f32>(base.rgb * base.rgb, 1.0);
        result.normal = normal(decode(sample_image(1u, uv).xy));
        if flag(EMISSIVE_FLICKER) {
            result = fit_response(result,vec3<f32>(0.04),base.a);
            result.response.a = recipe.parms.x * recipe.parms.x;
        } else if flag(EMISSIVE_SCRIPT_MULTI) {
            let response = sample_image(2u,uv);
            result = fit_response(result,response.rgb * response.rgb,response.a);
            let emission = select(0.0,recipe.parms.w,recipe.special_parms[1].z >= recipe.second_parms.z);
            result.response.a = emission * emission;
            if recipe.second_parms.w != 0.0 {result.color.a = base.a;}
        } else {
            let response = sample_image(2u, uv);
            let specular = response.rgb * recipe.parms.x;
            result = fit_response(result,specular * specular,response.a * recipe.parms.y);
            let emission = select(recipe.parms.z * recipe.parms.z * recipe.parms.w,recipe.parms.w,flag(EMISSIVE_TOMB));
            result.response.a = emission * emission;
            if flag(PHONG_EMISSIVE) && recipe.special_parms[0].x != 0.0 {result.color.a = base.a;}
        }
        return result;
    }
    if flag(TILE_BLEND) {
        let first_uv = uv * recipe.parms.xy;
        let second_uv = uv * recipe.parms.zw;
        let macro_sample = sample_image(0u, uv);
        let weight = macro_sample.g * macro_sample.g;
        let slope = decode(sample_image(1u, uv).xy) * recipe.second_parms.y
            + decode(sample_image(2u, first_uv).xy) * ((1.0 - weight) * recipe.second_parms.x)
            + decode(sample_image(3u, second_uv).xy) * (weight * recipe.second_parms.w);
        let first = sample_image(4u, first_uv).rgb;
        let encoded = first + weight * (sample_image(5u, second_uv).rgb - first);
        result.color = vec4<f32>(encoded * encoded, 1.0);
        result.normal = normal(slope);
        if flag(BASE_SPECULAR) {
            let first_response = sample_image(6u, first_uv);
            let response = first_response + weight * (sample_image(7u, second_uv) - first_response);
            result = fit_response(result, response.gba * response.gba, response.r);
        }
        return result;
    }
    let time = recipe.special_parms[2].w;
    let motion = vec2<f32>(0.0, fract(time * bitcast<f32>(0xbe116d3fu)));
    let warp_uv = (uv * bitcast<f32>(0x4035c88eu) + motion)
        * vec2<f32>(bitcast<f32>(0x3f7f41f2u), bitcast<f32>(0xbdb851ecu));
    let power = sample_image(2u, warp_uv).r * 0.1 + 1.0;
    let water_uv = (pow(uv, vec2<f32>(power)) - vec2<f32>(0.5)) / recipe.parms.zw + vec2<f32>(0.5);
    let scroll = time * recipe.parms.x;
    let first_uv = water_uv - scroll * vec2<f32>(bitcast<f32>(0xbc11fcabu), bitcast<f32>(0x3f4cc98cu));
    let second_uv = water_uv - (scroll * vec2<f32>(bitcast<f32>(0xbc6d3a96u), bitcast<f32>(0x3fa663c2u)) + vec2<f32>(0.5));
    let slope_sum = decode(sample_image(3u, first_uv).xy) + decode(sample_image(3u, second_uv).xy);
    var water = normalize(vec3<f32>(slope_sum, 2.0));
    water /= water.z;
    let positive = dot(water.xy, vec2<f32>(0.5)) > 0.0;
    let response = sample_image(5u, uv);
    let bias = select(bitcast<f32>(0x3e92491bu), bitcast<f32>(0x3e5b6dcau), positive);
    let wet_specular = response.rgb + vec3<f32>(bias);
    let temporal = sample_image(0u, uv).r;
    let state = recipe.special_parms[1].z;
    var reveal = clamp((vec2<f32>(temporal) - (vec2<f32>(state) + vec2<f32>(bitcast<f32>(0x3d75c28fu),bitcast<f32>(0xbcf5c28fu))))
        * vec2<f32>(bitcast<f32>(0xc1055555u),bitcast<f32>(0x41855555u)), vec2<f32>(0.0),vec2<f32>(1.0));
    reveal = reveal * reveal * (vec2<f32>(3.0) - 2.0 * reveal);
    let encoded_specular = wet_specular + reveal.x * (response.rgb - wet_specular);
    let wet_gloss = select(1.0, response.a, positive);
    let gloss = wet_gloss + reveal.x * (response.a - wet_gloss);
    let base_slope = decode(sample_image(1u, uv).xy);
    let wet_slope = base_slope * water.z + water.xy * recipe.parms.y;
    result.normal = normal(wet_slope + reveal.x * (base_slope - wet_slope));
    let warped_color_uv = clamp(uv - water.xy * water.z,vec2<f32>(0.0),vec2<f32>(1.0));
    let wet_color = sample_image(4u, warped_color_uv).rgb * vec3<f32>(bitcast<f32>(0x3f3bc148u),bitcast<f32>(0x3f49c91du),bitcast<f32>(0x3f649247u));
    let altered = recipe.special_parms[0].rgb * sample_image(4u, uv).r;
    let encoded = wet_color + reveal.x * (altered - wet_color);
    result.color = vec4<f32>(encoded * encoded, 1.0);
    result = fit_response(result, encoded_specular * encoded_specular, gloss);
    result.response.a = reveal.x * reveal.y * reveal.x * reveal.y;
    return result;
}

@compute @workgroup_size(64)
fn bake(@builtin(global_invocation_id) invocation: vec3<u32>) {
    let index = invocation.x;
    if index >= recipe.count { return; }
    let sample = samples[index];
    let uv0 = sample.uv01.xy;
    if flag(SPECIAL_SURFACE) {
        output[index] = special_surface(uv0);
        return;
    }
    let uv1 = sample.uv01.zw;
    let uv2 = sample.uv2_color.xy;
    let green = sample.uv2_color.z;
    let blue = sample.uv2_color.w;
    let base = sample_image(0u, uv0);
    let n0 = sample_image(1u, uv0);
    let layer = sample_image(select(2u, 1u, flag(GEOMETRIC_NORMAL)), uv1);
    var result: Pixel;
    result.response = vec4<f32>(1.0, 1.0, 0.0, 1.0);
    result.specular = vec4<f32>(0.0);
    if flag(MULTIPLY) || flag(ADDITIVE) || flag(BLEND_COLOR) || flag(GEOMETRIC_NORMAL) {
        var second = vec4<f32>(1.0);
        if flag(DECAL) { second = sample_image(recipe.slots.w, uv2); }
        var second_weight = blue * second.a;
        if flag(SECOND_REVEAL) { second_weight = reveal_weight(second.a, blue, recipe.second_parms); }
        if second_weight < 0.0 {
            result.color.x = bitcast<f32>(0x7fc00001u);
            output[index] = result;
            return;
        }
        let tint = vec3<f32>(1.0) + green * (layer.rgb - vec3<f32>(1.0));
        var color: vec3<f32>;
        if flag(BLEND_COLOR) {
            let first = base.rgb + green * layer.a * (layer.rgb - base.rgb);
            var weight = 0.0;
            if flag(DECAL) { weight = second_weight; }
            color = first + weight * (second.rgb - first);
        } else if flag(ADDITIVE) {
            color = (base.rgb + layer.rgb * green * layer.a) * (vec3<f32>(1.0) + blue * (second.rgb - vec3<f32>(1.0)));
        }
        else { color = base.rgb * tint * (vec3<f32>(1.0) + blue * (second.rgb - vec3<f32>(1.0))); }
        result.color = vec4<f32>(color * color, 1.0);
        if flag(BASE_ALPHA_RESPONSE) {
            var gloss = base.a;
            if flag(ADDITIVE) && flag(DECAL) {
                let second_tint = vec3<f32>(1.0) + blue * (second.rgb - vec3<f32>(1.0));
                gloss = base.a * (second_tint.x * 0.3 + second_tint.y * 0.6 + second_tint.z * 0.1);
            } else if !flag(ADDITIVE) && !flag(BLEND_COLOR) { gloss = base.a * (tint.x * 0.3 + tint.y * 0.6 + tint.z * 0.1); }
            result.response = vec4<f32>(1.0, roughness(gloss), 0.0, gloss);
        }
        result.normal = vec4<f32>(0.0, 0.0, 1.0, 1.0);
        if !flag(GEOMETRIC_NORMAL) {
            var slope = decode(n0.xy);
            if flag(SECOND_NORMAL_COLOR) {
                let s = decode(sample_image(recipe.slots.z, uv2).xy);
                let rotated = vec2<f32>(s.x * sample.transform.x + s.y * sample.transform.y, s.x * sample.transform.z + s.y * sample.transform.w);
                slope = slope + second_weight * (rotated - slope);
            }
            result.normal = normal(slope);
        }
        if flag(BASE_SPECULAR) || flag(SECOND_SPECULAR) {
            let k = bitcast<f32>(0x3e4ccccdu);
            var specular = vec4<f32>(k, k, k, base.a);
            if flag(BASE_SPECULAR) { specular = sample_image(recipe.last_image, uv0); }
            if flag(SECOND_SPECULAR) {
                let second_specular = sample_image(recipe.slots.y, uv2);
                specular = specular + second_weight * (second_specular - specular);
            }
            result = fit_response(result, specular.rgb * specular.rgb, specular.a);
        }
        output[index] = result;
        return;
    }
    let n1 = sample_image(3u, uv1);
    var weight = layer.a * green;
    if flag(REVEAL) { weight = reveal_weight(layer.a, green, recipe.parms); }
    else if flag(LAYER_CUTOUT) { weight = select(0.0, 1.0, layer.a * green >= 0.5); }
    var decal = vec4<f32>(1.0);
    if flag(DECAL) { decal = sample_image(recipe.slots.w, uv2); }
    var second_weight = blue * decal.a;
    if flag(SECOND_REVEAL) { second_weight = reveal_weight(decal.a, blue, recipe.second_parms); }
    if weight < 0.0 || second_weight < 0.0 {
        result.color.x = bitcast<f32>(0x7fc00001u);
        output[index] = result;
        return;
    }
    let first = base.rgb + weight * (layer.rgb - base.rgb);
    var color = first * (vec3<f32>(1.0) + blue * (decal.rgb - vec3<f32>(1.0)));
    if flag(SECOND_ADDITIVE) { color = first + blue * decal.a * decal.rgb; }
    else if flag(SECOND_NORMAL_LAYER) { color = first + second_weight * (decal.rgb - first); }
    var third_tint = vec3<f32>(1.0);
    if flag(THIRD_MULTIPLY) {
        let third_color = sample_image(recipe.last_image, sample.third.xy);
        third_tint = vec3<f32>(1.0) + sample.third.z * (third_color.rgb - vec3<f32>(1.0));
    }
    color = color * third_tint;
    var alpha = 1.0;
    if flag(LAYER_CUTOUT) && weight == 1.0 { alpha = layer.a; }
    result.color = vec4<f32>(color * color, alpha);
    let s0 = decode(n0.xy);
    let s1 = decode(n1.xy);
    let rotated = vec2<f32>(s1.x * sample.transform.x + s1.y * sample.transform.y, s1.x * sample.transform.z + s1.y * sample.transform.w);
    var slope = s0 + weight * (rotated - s0);
    if flag(SECOND_NORMAL_LAYER) {
        let s = decode(sample_image(recipe.slots.z, uv2).xy);
        let rotated_second = vec2<f32>(s.x * sample.second_transform.x + s.y * sample.second_transform.y,
            s.x * sample.second_transform.z + s.y * sample.second_transform.w);
        slope = slope + second_weight * (rotated_second - slope);
    }
    result.normal = normal(slope);
    if flag(LAYER_SPECULAR) || flag(BASE_SPECULAR) || flag(SECOND_SPECULAR) {
        let k = bitcast<f32>(0x3e4ccccdu);
        var base_gloss = 0.0;
        if flag(BASE_ALPHA_RESPONSE) { base_gloss = base.a; }
        var specular = vec4<f32>(k, k, k, base_gloss);
        if flag(BASE_SPECULAR) { specular = sample_image(recipe.last_image, uv0); }
        if flag(LAYER_SPECULAR) {
            let layer_specular = sample_image(recipe.slots.x, uv1);
            specular = specular + weight * (layer_specular - specular);
        }
        if flag(SECOND_SPECULAR) {
            let second_specular = sample_image(recipe.slots.y, uv2);
            specular = specular + second_weight * (second_specular - specular);
        }
        let rgb = specular.rgb * specular.rgb;
        var gloss = specular.a;
        if flag(THIRD_MULTIPLY) { gloss = gloss * (third_tint.x * 0.3 + third_tint.y * 0.6 + third_tint.z * 0.1); }
        else if flag(DECAL) && !flag(SECOND_ADDITIVE) && !flag(SECOND_NORMAL_COLOR) && !flag(SECOND_NORMAL_LAYER) {
            let tint = vec3<f32>(1.0) + blue * (decal.rgb - vec3<f32>(1.0));
            gloss = gloss * (tint.x * 0.3 + tint.y * 0.6 + tint.z * 0.1);
        }
        result = fit_response(result, rgb, gloss);
    } else if flag(BASE_ALPHA_RESPONSE) { result.response = vec4<f32>(1.0, roughness(base.a), 0.0, base.a); }
    output[index] = result;
}

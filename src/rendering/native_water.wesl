import package::rendering::native_world::{NativeWorldScene, NativeWorldObject};

// Original wpc_cod7water_we876z2f, cameraRegion litOpaque, sortKey16.
// Native state: reverse-Z depth write, back cull, RGB write, alpha write off,
// no blending, no alpha test. There is no emissive or depth-prepass technique.
// Recovered rendering-20261012/source/all/zm_tomb/{techsets,techniques,shader_bin}.
// LIT VS fcb1a754 SHA256 e48812bea34bc7546f15864eb1d234ef613ac75532b184f49b2ab0f3f9df3bfb.
// LIT PS fcb1a754 SHA256 b4d0d1ffb68aa541b4293842cc190d0a5b7761081656c09ced8bdd391626f160.
// SUN PS362781fd SHA256 2188bbabb6422d6edb856fd39f97fd5fde4c408dacc2a4e199e9bda67b8e49ec.
// SPOT PSa4e65c9e SHA256 cc8aabf1cf3332bac0743a262e3f19aca4f5cc6d3d5de071745537663841b225.
// SQUARE PS760181ab SHA256 0420a50e8550940938f988d8eef1f683fef73d6209fb49b53049e20dcc385053.
// ROUND PSaa9d17db SHA256 f87154b53177f8cee0b083945be4c97b05bea7cfa0a6d8bda7039fc0487db9cc.
// OMNI PS787acaee SHA256 10b2f7ba8af8cdba5cf4545de3b941d7d62a3c51aa1f7727d54c19046cdd60cc.
// UNLIT VS4eaedd79 SHA256 f223df7a7d5e7de13df5174918a3b75639a220f1176ed212e73f03f5869943b5.
// UNLIT PS4eaedd79 SHA256 8065bcf3d0d8c4072af52d57160d425652c9679c2876696fe0c23ac08e4337d8.
// Techset aliases map sun/shadow/sun-dlight to SUN and corresponding spot/
// square/round/omni variants to their same PS. No shadow texture is read here.
// Host binds the original primary-light technique and actual view/light data.
// Refraction reads the resolvedPostSun snapshot before these water draws write
// scene color/depth; sampling the active color attachment is a different input.

struct NativeWaterScene {
    world: NativeWorldScene,
    // Source $Globals cb2[0]: [sin(2pi*frac(t)),cos(...),frac(t),t].
    // 0041ba60/004189a0/00467da0: refdef milliseconds -> float seconds;
    // 00477780 packs the vector, 00474870 subtracts actual material time offset.
    // VS consumes unwrapped W, never a chosen loop length or wall-clock delta.
    game_time: vec4<f32>,
    z_near: vec4<f32>, // Source cb0[58]; X is original native-inch near distance.
    render_target_size: vec4<f32>, // Source cb0[95], inverse width/height in ZW.
}

// Compact required material inputs; original $Globals slots are documented.
// SDK hash-only slot names resolve with its unchanged R_HashString:
// normalMap00=449481359, normalMap01=449481358;
// waterHiNormalScroll=27397098, waterLoNormalScroll=1855396200,
// waterNormalScrollSpeed=323802060, waterNormalPositionScale=3341333671,
// waterControlVar0=2815841959, waterControlVar1=2815841958,
// waterNormalScale=2251460126. waterColorN/S are already named in the export.
// Source waterColorF cb2[9] is unused by every selected program; it is omitted.
struct NativeWaterMaterial {
    normal_position_scale: vec4<f32>, // cb2[1]
    lo_normal_scroll: vec4<f32>, // cb2[2]
    hi_normal_scroll: vec4<f32>, // cb2[3]
    normal_scroll_speed: vec4<f32>, // cb2[4]
    control0: vec4<f32>, // cb2[5]
    control1: vec4<f32>, // cb2[6]
    color_n: vec4<f32>, // cb2[7]
    color_s: vec4<f32>, // cb2[8]
    normal_scale: vec4<f32>, // cb2[10]
}

struct NativeWaterSunLight {
    position: vec4<f32>, // Source cb0[18].xyz: authored source direction to sun.
    diffuse: vec4<f32>, // Source cb0[19].rgb.
}

struct NativeWaterPointLight {
    position: vec4<f32>, // Source cb0[77], eye-relative native inches/source axes.
    diffuse: vec4<f32>, // cb0[78]
    attenuation: vec4<f32>, // cb0[79], W scales the capped inverse-square falloff.
    falloff_a: vec4<f32>, // cb0[82]
    falloff_b: vec4<f32>, // cb0[83]
}

struct NativeWaterProjectedLight {
    light: NativeWaterPointLight,
    spot_matrix_rows: array<vec4<f32>, 3>, // Source cb0[84..86].
}

struct NativeWaterSpotCookie {
    spot_direction: vec4<f32>, // Source cb0[80], XYZ builds cookie U.
    cookie_slide: vec4<f32>, // Source cb0[91], XYZ builds cookie V.
}

// 0047ea30 binds the selected lightDef attenuation image as code texture15
// with sampler0x13 at SourceState+0x161b. Through 00432ba0 at quality1 this
// is anisotropic2, linear min/mag/mip, repeatUVW, zero bias and all mips.

struct NativeWaterSpotCone {
    control1: vec4<f32>, // Source cb0[88].
    control2: vec4<f32>, // Source cb0[89], XY are the two superellipse powers.
}

struct NativeWaterVertexInput {
    position: vec3<f32>,
    color: vec4<f32>,
    lightmap_uv: vec2<f32>,
}

struct NativeWaterUnlitVertexInput {
    position: vec3<f32>,
    uv: vec2<f32>,
}

struct NativeWaterVaryings {
    clip_position: vec4<f32>,
    position_lightmap: vec4<f32>,
    // Entry shader must use @interpolate(perspective, centroid), as in source
    // TEXCOORD1. W is homogeneous clip W in native inches, not metres or NDC Z.
    eye_and_clip_w: vec4<f32>,
    normal_uv0: vec4<f32>,
    normal_uv1: vec4<f32>,
    color: vec4<f32>,
    reflection_correction: vec3<f32>,
}

struct NativeWaterUnlitVaryings {
    clip_position: vec4<f32>,
    uv: vec2<f32>,
    normal_uv0: vec4<f32>,
    normal_uv1: vec4<f32>,
    reflection_correction: vec3<f32>,
}

struct NativeWaterVertexBase {
    clip_position: vec4<f32>,
    eye_to_vertex: vec3<f32>,
    normal_uv0: vec4<f32>,
    normal_uv1: vec4<f32>,
    reflection_correction: vec3<f32>,
}

fn native_water_vertex_base(
    position: vec3<f32>, scene: NativeWaterScene,
    material: NativeWaterMaterial, object: NativeWorldObject,
) -> NativeWaterVertexBase {
    let homogeneous = vec4<f32>(position, 1.0);
    let relative = vec4<f32>(dot(homogeneous, object.world_rows[0]),
        dot(homogeneous, object.world_rows[1]), dot(homogeneous, object.world_rows[2]),
        dot(homogeneous, object.world_rows[3]));
    let clip = vec4<f32>(dot(relative, scene.world.view_projection_rows[0]),
        dot(relative, scene.world.view_projection_rows[1]),
        dot(relative, scene.world.view_projection_rows[2]),
        dot(relative, scene.world.view_projection_rows[3]));
    // Scroll original local POSITION.xy, independently of the object/eye pose.
    let low_time = scene.game_time.w * material.lo_normal_scroll;
    let low_offset = low_time * material.normal_scroll_speed.xxyy;
    let low_uv = fma(position.xyxy, material.normal_position_scale.xxyy, low_offset);
    let high_time = scene.game_time.w * material.hi_normal_scroll;
    let high_offset = high_time * material.normal_scroll_speed.zzww;
    let high_uv = fma(position.xyxy, material.normal_position_scale.zzww, high_offset);
    // Original VS assumes a horizontal water normal. It does not sample or
    // transform a vertex normal/tangent and does not evaluate full normal SH.
    let sh = dot(object.reflection_lighting_sh1.zw, vec2<f32>(1.0))
        + object.reflection_lighting_sh0.w;
    let correction = vec3<f32>(1.0) / max(sh * object.reflection_lighting_sh0.rgb,
        vec3<f32>(0.10000000149011612));
    return NativeWaterVertexBase(clip, relative.xyz, low_uv, high_uv, correction);
}

// PACKED_WORLD primary stride36: POSITION at0, COLOR at16, lightmap UV at32;
// neither packed normal/tangent nor auxiliary stream is required by LIT.
fn native_water_vertex(
    input: NativeWaterVertexInput, scene: NativeWaterScene,
    material: NativeWaterMaterial, object: NativeWorldObject,
) -> NativeWaterVaryings {
    let base = native_water_vertex_base(input.position, scene, material, object);
    return NativeWaterVaryings(base.clip_position, vec4<f32>(input.position.xy, input.lightmap_uv),
        vec4<f32>(base.eye_to_vertex, base.clip_position.w), base.normal_uv0,
        base.normal_uv1, input.color, base.reflection_correction);
}

// UNLIT's separate source VS reads material UV at20 instead of lightmap UV.
fn native_water_unlit_vertex(
    input: NativeWaterUnlitVertexInput, scene: NativeWaterScene,
    material: NativeWaterMaterial, object: NativeWorldObject,
) -> NativeWaterUnlitVaryings {
    let base = native_water_vertex_base(input.position, scene, material, object);
    return NativeWaterUnlitVaryings(base.clip_position, input.uv, base.normal_uv0,
        base.normal_uv1, base.reflection_correction);
}

fn native_water_unlit_fragment(material: NativeWaterMaterial) -> vec4<f32> {
    // Original unlit PS has no scene HDR, fog, normals, refraction or lighting.
    return vec4<f32>(sqrt((material.color_n.rgb + material.color_s.rgb) * 0.5), material.color_n.a);
}

struct NativeWaterSurface {
    normal: vec3<f32>,
    incident: vec3<f32>,
    distance: f32,
    fresnel: f32,
    water_color: vec4<f32>,
    depth_amount: f32,
    refracted: vec3<f32>,
}

fn native_water_fresnel(cosine: f32, control: vec4<f32>) -> f32 {
    let complement = 1.0 - cosine;
    let squared = complement * complement;
    let fourth = squared * squared;
    return fma(control.w, fourth * complement, control.z);
}

// Native pairs: normalMap00 t0/s0, normalMap01 t1/s1, floatZ t2/s2,
// resolvedPostSun refraction t3/s5. All sampled color/normal views are UNORM.
// floatZ is source reverse-Z R32Float max resolve; its native0x61 code sampler
// is nearest, clamp UV/repeat W, no mip bias (00433fb0/0043d110).
// Refraction uses native0x62: R_InitCodeTextureBindings0043d230 writes
// DAT_0395ca75 next to TEXTURE_SRC_CODE_RESOLVED_POST_SUN (code texture9).
// 00467520 copies its sampler byte to SourceState+0x1615, 00475ed0 binds it
// to s5, and 00432ba0 resolves min/maglinear, mipnearest, clampUV/repeatW,
// anisotropy1, no comparison, all mips and zero bias at source quality1.
fn native_water_surface(
    input: NativeWaterVaryings, scene: NativeWaterScene, material: NativeWaterMaterial,
    normal00: texture_2d<f32>, normal_sampler00: sampler,
    normal01: texture_2d<f32>, normal_sampler01: sampler,
    float_z: texture_2d<f32>, float_z_sampler: sampler,
    refract_map: texture_2d<f32>, refract_sampler: sampler,
) -> NativeWaterSurface {
    let low0 = textureSample(normal00, normal_sampler00, input.normal_uv0.xy);
    let low1 = textureSample(normal00, normal_sampler00, input.normal_uv0.zw);
    let low = fma(vec4<f32>(low0.rg, low1.rg), vec4<f32>(material.normal_scale.x),
        vec4<f32>(material.normal_scale.y));
    let high0 = textureSample(normal01, normal_sampler01, input.normal_uv1.xy);
    let high1 = textureSample(normal01, normal_sampler01, input.normal_uv1.zw);
    let high = fma(vec4<f32>(high0.rg, high1.rg), vec4<f32>(material.normal_scale.z),
        vec4<f32>(material.normal_scale.w));
    let combined = low + high;
    let slopes = combined.xy + combined.zw;
    let mapped = vec3<f32>(slopes, 1.0);
    let normal = inverseSqrt(dot(mapped, mapped)) * mapped;
    let distance_squared = dot(input.eye_and_clip_w.xyz, input.eye_and_clip_w.xyz);
    let incident = inverseSqrt(distance_squared) * input.eye_and_clip_w.xyz;
    let facing = clamp(dot(normal, -incident), 0.0, 1.0);
    let fresnel = native_water_fresnel(facing, material.control0);
    let water_color = fma(vec4<f32>(facing), material.color_s - material.color_n, material.color_n);
    // Entry input.clip_position.xy is fragment SV_Position: pixel-centre XY,
    // independent of the centroid interpolated eye vector / clip W.
    let screen_uv = input.clip_position.xy * scene.render_target_size.zw;
    let z_sample = textureSample(float_z, float_z_sampler, screen_uv).r;
    let thickness = scene.z_near.x / max(abs(z_sample), 0.0) - input.eye_and_clip_w.w;
    let depth_amount = clamp(thickness * material.control1.w, 0.0, 1.0);
    let offset_pixels = normal.xy * 8.0;
    let refract_xy = fma(offset_pixels, vec2<f32>(sqrt(depth_amount)), input.clip_position.xy);
    let refract_sample = textureSample(refract_map, refract_sampler,
        refract_xy * scene.render_target_size.zw);
    let refracted = (refract_sample.rgb * refract_sample.rgb) / scene.world.hdr_control0.x;
    return NativeWaterSurface(normal, incident, sqrt(distance_squared), fresnel,
        water_color, depth_amount, refracted);
}

// Exact common two-band base lighting: ambient + one quarter directional RGB.
// Water does not use the third-band directional RGB as a lighting direction.
fn native_water_base_light(
    input: NativeWaterVaryings,
    lightmap: texture_2d<f32>, lightmap_sampler: sampler,
) -> vec3<f32> {
    let uv = input.position_lightmap.zw;
    let directed = textureSample(lightmap, lightmap_sampler,
        fma(uv, vec2<f32>(1.0, 0.3333333432674408), vec2<f32>(0.0, 0.3333333432674408)));
    let directed_color = (1.0 / (directed.a + 9.999999974752427e-07)) * directed.rgb;
    let quarter = directed_color * 0.25;
    let ambient = textureSample(lightmap, lightmap_sampler, uv * vec2<f32>(1.0, 0.3333333432674408));
    return fma(ambient.rgb, vec3<f32>(1.0 / (ambient.a + 9.999999974752427e-07)), quarter);
}

fn native_water_primary_visibility(
    input: NativeWaterVaryings,
    lightmap: texture_2d<f32>, lightmap_sampler: sampler,
) -> f32 {
    return textureSample(lightmap, lightmap_sampler,
        fma(input.position_lightmap.zw, vec2<f32>(1.0, 0.3333333432674408),
            vec2<f32>(0.0, 0.6666666865348816))).a;
}

fn native_water_reflected(
    input: NativeWaterVaryings, surface: NativeWaterSurface, base_light: vec3<f32>,
    reflection_probe: texture_cube<f32>, reflection_sampler: sampler,
) -> vec3<f32> {
    let direction = fma(surface.normal,
        vec3<f32>(-2.0 * dot(surface.incident, surface.normal)), surface.incident);
    // Original water uses explicit probe LOD0, independently of water roughness.
    let sample = textureSampleLevel(reflection_probe, reflection_sampler, direction, 0.0);
    let probe = (1.0 / (sample.a + 9.999999974752427e-07)) * sample.rgb;
    return surface.fresnel * ((base_light * probe) * input.reflection_correction);
}

fn native_water_compose(
    input: NativeWaterVaryings, surface: NativeWaterSurface,
    diffuse_light: vec3<f32>, reflected: vec3<f32>,
) -> vec3<f32> {
    let tint = (1.0 - surface.fresnel) * surface.water_color.rgb;
    let delta = fma(tint, diffuse_light, -surface.refracted);
    let opacity = (surface.water_color.a * input.color.a) * surface.depth_amount;
    let refracted_diffuse = fma(vec3<f32>(opacity), delta, surface.refracted);
    return fma(reflected, vec3<f32>(surface.depth_amount), refracted_diffuse);
}

// Every lit water PS computes fog per pixel and multiplies its contribution
// by water depth_amount. A vertex fog varying or ordinary opaque fog differs.
fn native_water_finish(
    input: NativeWaterVaryings, scene: NativeWaterScene,
    surface: NativeWaterSurface, lit: vec3<f32>,
) -> vec4<f32> {
    let sun_amount = clamp(fma(scene.world.sun_fog.y,
        dot(scene.world.sun_fog_dir.xyz, surface.incident), scene.world.sun_fog.x), 0.0, 1.0);
    let fog = fma(vec4<f32>(sun_amount), scene.world.sun_fog_color - scene.world.fog_color,
        scene.world.fog_color);
    let height_term = fma(scene.world.fog_consts.w, input.eye_and_clip_w.z, scene.world.fog_consts.x);
    let height_exp = exp2(min(height_term, 64.0) * 1.4426950216293335);
    let height_integral = select(height_term + 1.0, height_exp, height_term < 0.0);
    let height_delta = input.eye_and_clip_w.z * scene.world.fog_consts.w;
    let near_horizontal = abs(height_delta) < 9.999999747378752e-05;
    let denominator = select(height_delta, 1.0, near_horizontal);
    let integral = select((height_integral - scene.world.fog_consts2.x) / denominator,
        clamp(scene.world.fog_consts2.x, 0.0, 1.0), near_horizontal) * scene.world.fog_consts.y;
    let fog_amount = 1.0 - min(exp2(fma(integral, surface.distance, scene.world.fog_consts.z)), 1.0);
    let amount = (fog.a * fog_amount) * surface.depth_amount;
    let fogged = fma(vec3<f32>(amount), fog.rgb - lit, lit);
    return vec4<f32>(sqrt(fogged * scene.world.hdr_control0.x), 0.0);
}

fn native_water_fragment(
    input: NativeWaterVaryings, scene: NativeWaterScene, material: NativeWaterMaterial,
    normal00: texture_2d<f32>, normal_sampler00: sampler,
    normal01: texture_2d<f32>, normal_sampler01: sampler,
    float_z: texture_2d<f32>, float_z_sampler: sampler,
    refract_map: texture_2d<f32>, refract_sampler: sampler,
    lightmap: texture_2d<f32>, lightmap_sampler: sampler,
    reflection_probe: texture_cube<f32>, reflection_sampler: sampler,
) -> vec4<f32> {
    let base_light = native_water_base_light(input, lightmap, lightmap_sampler);
    let surface = native_water_surface(input, scene, material, normal00, normal_sampler00,
        normal01, normal_sampler01, float_z, float_z_sampler, refract_map, refract_sampler);
    let reflected = native_water_reflected(input, surface, base_light, reflection_probe, reflection_sampler);
    let lit = native_water_compose(input, surface, base_light, reflected);
    return native_water_finish(input, scene, surface, lit);
}

struct NativeWaterDirectLight {
    direction: vec3<f32>,
    radiance: vec3<f32>, // Before the third-band lightmap alpha visibility.
}

// Common direct-light specular instructions in SUN/SPOT/SQUARE/ROUND/OMNI.
// Original two power lobes, half-vector Fresnel and 1/(4 H.V); no epsilon or
// denominator replacement. Material control vectors supply all authored values.
fn native_water_direct_specular(
    surface: NativeWaterSurface, material: NativeWaterMaterial,
    direction: vec3<f32>, radiance: vec3<f32>, n_dot_l: f32,
) -> vec3<f32> {
    let half_vector = direction - surface.incident;
    let half_direction = inverseSqrt(dot(half_vector, half_vector)) * half_vector;
    let n_dot_h = clamp(dot(surface.normal, half_direction), 0.0, 1.0);
    let powers = exp2(vec2<f32>(log2(n_dot_h)) * material.control0.xy);
    let lobe = dot(powers, material.control1.xy);
    let h_dot_l = clamp(dot(half_direction, direction), 0.0, 1.0);
    let fresnel = native_water_fresnel(h_dot_l, material.control0);
    let denominator = 4.0 * clamp(dot(half_direction, -surface.incident), 0.0, 1.0);
    let factor = (((n_dot_l * fresnel) * (1.0 / denominator)) * lobe) * material.control1.z;
    return radiance * factor;
}

// This tail is shared by all five native direct-light programs. Shape, cookie
// and distance attenuation are computed in their separate functions below.
fn native_water_direct_fragment(
    input: NativeWaterVaryings, scene: NativeWaterScene, material: NativeWaterMaterial,
    light: NativeWaterDirectLight,
    normal00: texture_2d<f32>, normal_sampler00: sampler,
    normal01: texture_2d<f32>, normal_sampler01: sampler,
    float_z: texture_2d<f32>, float_z_sampler: sampler,
    refract_map: texture_2d<f32>, refract_sampler: sampler,
    lightmap: texture_2d<f32>, lightmap_sampler: sampler,
    reflection_probe: texture_cube<f32>, reflection_sampler: sampler,
) -> vec4<f32> {
    let base_light = native_water_base_light(input, lightmap, lightmap_sampler);
    let visibility = native_water_primary_visibility(input, lightmap, lightmap_sampler);
    let radiance = light.radiance * visibility;
    let surface = native_water_surface(input, scene, material, normal00, normal_sampler00,
        normal01, normal_sampler01, float_z, float_z_sampler, refract_map, refract_sampler);
    let n_dot_l = clamp(dot(surface.normal, light.direction), 0.0, 1.0);
    let diffuse_light = fma(radiance, vec3<f32>(n_dot_l), base_light);
    // The probe is corrected by base lighting; direct diffuse is excluded.
    let reflected = native_water_reflected(input, surface, base_light, reflection_probe, reflection_sampler);
    let composed = native_water_compose(input, surface, diffuse_light, reflected);
    let specular = native_water_direct_specular(surface, material, light.direction, radiance, n_dot_l);
    let lit = fma(specular, vec3<f32>(surface.depth_amount), composed);
    return native_water_finish(input, scene, surface, lit);
}

fn native_water_sun_fragment(
    input: NativeWaterVaryings, scene: NativeWaterScene, material: NativeWaterMaterial,
    sun: NativeWaterSunLight,
    normal00: texture_2d<f32>, normal_sampler00: sampler,
    normal01: texture_2d<f32>, normal_sampler01: sampler,
    float_z: texture_2d<f32>, float_z_sampler: sampler,
    refract_map: texture_2d<f32>, refract_sampler: sampler,
    lightmap: texture_2d<f32>, lightmap_sampler: sampler,
    reflection_probe: texture_cube<f32>, reflection_sampler: sampler,
) -> vec4<f32> {
    return native_water_direct_fragment(input, scene, material,
        NativeWaterDirectLight(sun.position.xyz, sun.diffuse.rgb),
        normal00, normal_sampler00, normal01, normal_sampler01, float_z, float_z_sampler,
        refract_map, refract_sampler, lightmap, lightmap_sampler, reflection_probe, reflection_sampler);
}

fn native_water_smooth_falloff(value: f32) -> f32 {
    return fma(-2.0, value, 3.0) * (value * value);
}

fn native_water_projected_position(input: NativeWaterVaryings, light: NativeWaterProjectedLight) -> vec3<f32> {
    let position = vec4<f32>(input.eye_and_clip_w.xyz, 1.0);
    let projected = vec3<f32>(dot(light.spot_matrix_rows[0], position),
        dot(light.spot_matrix_rows[1], position), dot(light.spot_matrix_rows[2], position));
    return vec3<f32>(projected.xy / projected.z, projected.z);
}

fn native_water_distance_attenuation(input: NativeWaterVaryings, light: NativeWaterPointLight) -> f32 {
    let delta = light.position.xyz - input.eye_and_clip_w.xyz;
    return clamp((1.0 / dot(delta, delta)) * light.attenuation.w, 0.0, 1.0);
}

fn native_water_light_sample(
    input: NativeWaterVaryings, light: NativeWaterPointLight, radiance: vec3<f32>,
) -> NativeWaterDirectLight {
    let delta = light.position.xyz - input.eye_and_clip_w.xyz;
    let direction = inverseSqrt(dot(delta, delta)) * delta;
    return NativeWaterDirectLight(direction, radiance);
}

fn native_water_cookie(
    projected: vec3<f32>, cookie: NativeWaterSpotCookie,
    attenuation_map: texture_2d<f32>, attenuation_sampler: sampler,
) -> vec3<f32> {
    let uv = vec2<f32>(dot(projected.xy, cookie.spot_direction.xy) + cookie.spot_direction.z,
        dot(projected.xy, cookie.cookie_slide.xy) + cookie.cookie_slide.z);
    let sample = textureSample(attenuation_map, attenuation_sampler, uv);
    return sample.rgb * sample.rgb;
}

// SPOT superellipse: cb0[88/89] shape, range-Z feather, capped inverse-square
// distance attenuation and squared attenuationSampler t12/s12 cookie color.
fn native_water_spot_fragment(
    input: NativeWaterVaryings, scene: NativeWaterScene, material: NativeWaterMaterial,
    light: NativeWaterProjectedLight, cone: NativeWaterSpotCone, cookie: NativeWaterSpotCookie,
    normal00: texture_2d<f32>, normal_sampler00: sampler,
    normal01: texture_2d<f32>, normal_sampler01: sampler,
    float_z: texture_2d<f32>, float_z_sampler: sampler,
    refract_map: texture_2d<f32>, refract_sampler: sampler,
    attenuation_map: texture_2d<f32>, attenuation_sampler: sampler,
    lightmap: texture_2d<f32>, lightmap_sampler: sampler,
    reflection_probe: texture_cube<f32>, reflection_sampler: sampler,
) -> vec4<f32> {
    let projected = native_water_projected_position(input, light);
    let range = clamp(fma(vec2<f32>(projected.z), light.light.falloff_a.zw,
        light.light.falloff_b.zw), vec2<f32>(0.0), vec2<f32>(1.0));
    let powers = exp2(log2(abs(projected.xxyy * cone.control1)) * cone.control2.x);
    let norms = exp2(log2(powers.zw + powers.xy) * cone.control2.y);
    let shape = clamp((norms.y - 1.0) / (norms.y - norms.x), 0.0, 1.0);
    let falloff = native_water_smooth_falloff(range.x * range.y) * native_water_smooth_falloff(shape);
    let factor = falloff * native_water_distance_attenuation(input, light.light);
    let cookie_color = native_water_cookie(projected, cookie, attenuation_map, attenuation_sampler);
    let sample = native_water_light_sample(input, light.light, (factor * cookie_color) * light.light.diffuse.rgb);
    return native_water_direct_fragment(input, scene, material, sample,
        normal00, normal_sampler00, normal01, normal_sampler01, float_z, float_z_sampler,
        refract_map, refract_sampler, lightmap, lightmap_sampler, reflection_probe, reflection_sampler);
}

// SQUARE uses separate absolute U/V feathers and range-Z; no superellipse.
fn native_water_square_fragment(
    input: NativeWaterVaryings, scene: NativeWaterScene, material: NativeWaterMaterial,
    light: NativeWaterProjectedLight, cookie: NativeWaterSpotCookie,
    normal00: texture_2d<f32>, normal_sampler00: sampler,
    normal01: texture_2d<f32>, normal_sampler01: sampler,
    float_z: texture_2d<f32>, float_z_sampler: sampler,
    refract_map: texture_2d<f32>, refract_sampler: sampler,
    attenuation_map: texture_2d<f32>, attenuation_sampler: sampler,
    lightmap: texture_2d<f32>, lightmap_sampler: sampler,
    reflection_probe: texture_cube<f32>, reflection_sampler: sampler,
) -> vec4<f32> {
    let projected = native_water_projected_position(input, light);
    let feathers = clamp(fma(vec4<f32>(abs(projected.xy), projected.zz), light.light.falloff_a,
        light.light.falloff_b), vec4<f32>(0.0), vec4<f32>(1.0));
    let falloff = (native_water_smooth_falloff(feathers.y) * native_water_smooth_falloff(feathers.x))
        * native_water_smooth_falloff(feathers.z * feathers.w);
    let factor = falloff * native_water_distance_attenuation(input, light.light);
    let cookie_color = native_water_cookie(projected, cookie, attenuation_map, attenuation_sampler);
    let sample = native_water_light_sample(input, light.light, (factor * cookie_color) * light.light.diffuse.rgb);
    return native_water_direct_fragment(input, scene, material, sample,
        normal00, normal_sampler00, normal01, normal_sampler01, float_z, float_z_sampler,
        refract_map, refract_sampler, lightmap, lightmap_sampler, reflection_probe, reflection_sampler);
}

// ROUND uses projected radial distance plus range-Z and no cookie texture.
fn native_water_round_fragment(
    input: NativeWaterVaryings, scene: NativeWaterScene, material: NativeWaterMaterial,
    light: NativeWaterProjectedLight,
    normal00: texture_2d<f32>, normal_sampler00: sampler,
    normal01: texture_2d<f32>, normal_sampler01: sampler,
    float_z: texture_2d<f32>, float_z_sampler: sampler,
    refract_map: texture_2d<f32>, refract_sampler: sampler,
    lightmap: texture_2d<f32>, lightmap_sampler: sampler,
    reflection_probe: texture_cube<f32>, reflection_sampler: sampler,
) -> vec4<f32> {
    let projected = native_water_projected_position(input, light);
    let radius = sqrt(dot(projected.xy, projected.xy));
    let feathers = clamp(fma(vec3<f32>(radius, projected.zz), light.light.falloff_a.xzw,
        light.light.falloff_b.xzw), vec3<f32>(0.0), vec3<f32>(1.0));
    let falloff = native_water_smooth_falloff(feathers.y * feathers.z) * native_water_smooth_falloff(feathers.x);
    let factor = falloff * native_water_distance_attenuation(input, light.light);
    let sample = native_water_light_sample(input, light.light, factor * light.light.diffuse.rgb);
    return native_water_direct_fragment(input, scene, material, sample,
        normal00, normal_sampler00, normal01, normal_sampler01, float_z, float_z_sampler,
        refract_map, refract_sampler, lightmap, lightmap_sampler, reflection_probe, reflection_sampler);
}

// OMNI uses actual light distance for its range, without projective matrices.
fn native_water_omni_fragment(
    input: NativeWaterVaryings, scene: NativeWaterScene, material: NativeWaterMaterial,
    light: NativeWaterPointLight,
    normal00: texture_2d<f32>, normal_sampler00: sampler,
    normal01: texture_2d<f32>, normal_sampler01: sampler,
    float_z: texture_2d<f32>, float_z_sampler: sampler,
    refract_map: texture_2d<f32>, refract_sampler: sampler,
    lightmap: texture_2d<f32>, lightmap_sampler: sampler,
    reflection_probe: texture_cube<f32>, reflection_sampler: sampler,
) -> vec4<f32> {
    let delta = light.position.xyz - input.eye_and_clip_w.xyz;
    let distance = sqrt(dot(delta, delta));
    let range = clamp(fma(vec2<f32>(distance), light.falloff_a.zw, light.falloff_b.zw),
        vec2<f32>(0.0), vec2<f32>(1.0));
    let factor = native_water_distance_attenuation(input, light) * native_water_smooth_falloff(range.y * range.x);
    let sample = native_water_light_sample(input, light, factor * light.diffuse.rgb);
    return native_water_direct_fragment(input, scene, material, sample,
        normal00, normal_sampler00, normal01, normal_sampler01, float_z, float_z_sampler,
        refract_map, refract_sampler, lightmap, lightmap_sampler, reflection_probe, reflection_sampler);
}

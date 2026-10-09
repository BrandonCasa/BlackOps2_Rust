
struct Effect {
    scale_scroll: array<vec4<f32>,3>,
    origin_offset: array<vec4<f32>,3>,
    rotation_depth: array<vec4<f32>,3>,
    alpha_hdr: vec4<f32>,
    falloff_response: vec4<f32>,
    coverage_layers: vec4<u32>,
}
fn effect_uv(uv: vec2<f32>, layer: u32, effect: Effect, time: f32, parallax: vec2<f32>) -> vec2<f32> {
    let transform = effect.scale_scroll[layer];
    let origin = effect.origin_offset[layer].xy;
    let offset = effect.origin_offset[layer].zw;
    let rotation = effect.rotation_depth[layer];
    let angle = rotation.x*(1.0+rotation.y*time)*6.28318530718;
    let c = cos(angle); let s = sin(angle);
    let local = (uv-origin+offset+parallax*rotation.z)*transform.xy;
    return mat2x2(vec2(c,s),vec2(-s,c))*local+origin+time*transform.zw*transform.xy;
}
fn effect_gradient(gradient: mat2x2<f32>, layer: u32, effect: Effect, time: f32) -> mat2x2<f32> {
    let rotation = effect.rotation_depth[layer];
    let angle = rotation.x*(1.0+rotation.y*time)*6.28318530718;
    let c = cos(angle); let s = sin(angle);
    let turn = mat2x2(vec2(c,s),vec2(-s,c));
    return mat2x2(turn*(gradient[0]*effect.scale_scroll[layer].xy),turn*(gradient[1]*effect.scale_scroll[layer].xy));
}
fn effect_color(samples: array<vec4<f32>,3>, coverage: vec3<f32>, effect: Effect, facing: f32) -> vec4<f32> {
    let second = coverage.y*effect.alpha_hdr.y;
    let first = coverage.x*effect.alpha_hdr.x;
    var rgb = samples[2].rgb*coverage.z;
    rgb += second*(samples[1].rgb-rgb);
    rgb += first*(samples[0].rgb-rgb);
    let alpha = clamp(coverage.z*effect.alpha_hdr.z+second+first,0.0,1.0);
    let falloff = pow(max(facing,0.00001),effect.falloff_response.x)*effect.falloff_response.y;
    return vec4(max(rgb,vec3(0.0))*alpha*falloff*effect.alpha_hdr.w,alpha);
}

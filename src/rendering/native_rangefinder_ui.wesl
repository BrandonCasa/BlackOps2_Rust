// UI3D atlas producer: native VS7ba1465e / PSb2aadbd8, source pixel coordinates.
struct Quad { rectangle: vec4<f32>, uv_origin_x: vec4<f32>, uv_y: vec4<f32> }
@group(0) @binding(0) var atlas: texture_2d<f32>;
@group(0) @binding(1) var atlas_sampler: sampler;
@group(0) @binding(2) var<storage, read> quads: array<Quad>;
struct Varyings { @builtin(position) position: vec4<f32>, @location(0) uv: vec2<f32> }
@vertex fn vertex(@builtin(vertex_index) vertex: u32, @builtin(instance_index) instance: u32) -> Varyings {
    let corners = array<vec2<f32>, 6>(vec2<f32>(0.0, 0.0), vec2<f32>(1.0, 0.0), vec2<f32>(1.0, 1.0),
        vec2<f32>(0.0, 0.0), vec2<f32>(1.0, 1.0), vec2<f32>(0.0, 1.0));
    let q = quads[instance];
    let p = corners[vertex];
    let pixel = fma(p, q.rectangle.zw, q.rectangle.xy);
    let uv = fma(vec2<f32>(p.x), q.uv_origin_x.zw, fma(vec2<f32>(p.y), q.uv_y.xy, q.uv_origin_x.xy));
    return Varyings(vec4<f32>(pixel.x / 256.0 - 1.0, 1.0 - pixel.y / 256.0, 0.0, 1.0), uv);
}
@fragment fn fragment(input: Varyings) -> @location(0) vec4<f32> {
    return textureSample(atlas, atlas_sampler, input.uv);
}

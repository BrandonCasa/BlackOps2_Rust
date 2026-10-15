// T6 pc_resolve_floatz1/2/4/8/16. Original 1x PS b02a7e95 copies depth;
// MSAA PS 3ea57976/f8ca70f1/af6565be/6c852a17 take the sample maximum.
// Reverse-Z depth is zNear/positiveViewZ, with zero representing infinity.
@if(MULTISAMPLED)
@group(0) @binding(0) var source_depth: texture_depth_multisampled_2d;
@else
@group(0) @binding(0) var source_depth: texture_depth_2d;

struct FragmentInput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@fragment
fn fragment(input: FragmentInput) -> @location(0) f32 {
    let pixel = vec2<i32>(input.position.xy);
    @if(MULTISAMPLED) {
        var depth = textureLoad(source_depth, pixel, 0);
        for (var sample_index = 1u; sample_index < textureNumSamples(source_depth); sample_index++) {
            depth = max(depth, textureLoad(source_depth, pixel, i32(sample_index)));
        }
        return depth;
    }
    @if(!MULTISAMPLED) {
        return textureLoad(source_depth, pixel, 0);
    }
}

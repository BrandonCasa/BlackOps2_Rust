import package::rendering::native_sky::{NativeSkyUniform, NativeSkyVaryings, native_sky_vertex, native_sky_fragment};

@group(0) @binding(0) var<uniform> sky: NativeSkyUniform;
@group(0) @binding(1) var cube: texture_cube<f32>;
@group(0) @binding(2) var cube_sampler: sampler;

struct Varyings {
    @builtin(position) position: vec4<f32>,
    @location(0) cube_directions: vec4<f32>,
    @location(1) far_position_transition: vec4<f32>,
    @location(2) fog_color_amount: vec4<f32>,
}

@vertex
fn vertex(@location(0) position: vec3<f32>) -> Varyings {
    let output = native_sky_vertex(position, sky);
    return Varyings(output.clip_position, output.cube_directions,
        output.far_position_transition, output.fog_color_amount);
}

@fragment
fn fragment(input: Varyings) -> @location(0) vec4<f32> {
    return native_sky_fragment(NativeSkyVaryings(input.position, input.cube_directions,
        input.far_position_transition, input.fog_color_amount), sky, cube, cube_sampler);
}

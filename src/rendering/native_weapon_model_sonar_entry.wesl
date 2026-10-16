// Native MMS lens VS c436f53e / PS fc8afb12. Generated textures remain explicit.
import package::rendering::native_model::{NativeModelScene, NativeModelObject};
import package::rendering::native_model_draw::NativeModelDrawMatrices;
import package::rendering::native_world::NativeWorldObject;
import package::rendering::native_world_plain::{NativeWorldGeometricVertexInput, native_world_position_fog};
import package::rendering::native_weapon_model_code::{NativeSonarConstants, native_sonar_noise, native_sonar_fragment};
@group(0) @binding(0) var<uniform> scene: NativeModelScene;
@group(1) @binding(0) var<uniform> object: NativeModelObject;
@group(1) @binding(2) var<uniform> weapon: NativeModelDrawMatrices;
@group(2) @binding(0) var static_noise: texture_2d<f32>;
@group(2) @binding(1) var static_sampler: sampler;
@group(2) @binding(2) var lookup: texture_2d<f32>;
@group(2) @binding(3) var lookup_sampler: sampler;
@group(2) @binding(26) var<uniform> material: NativeSonarConstants;
@group(3) @binding(0) var float_z: texture_2d<f32>;
@group(3) @binding(1) var float_z_sampler: sampler;
@group(3) @binding(2) var sonar_color: texture_2d<f32>;
@group(3) @binding(3) var sonar_color_sampler: sampler;
@group(3) @binding(4) var sonar_depth: texture_depth_2d;
@group(3) @binding(5) var sonar_depth_sampler: sampler;
struct NativeSonarCodeView { z_near: vec4<f32>, target_size: vec4<f32>, viewport: vec4<f32> }
@group(3) @binding(6) var<uniform> code: NativeSonarCodeView;
struct Input { @location(0) position: vec3<f32>, @location(2) uv: vec2<f32> }
struct Varyings {
    @builtin(position) position: vec4<f32>, @location(0) uv: vec2<f32>,
    @location(1) fog: vec4<f32>, @location(2) noise_zw: vec2<f32>,
}
@vertex fn vertex_sonar(input: Input) -> Varyings {
    let output = native_world_position_fog(
        NativeWorldGeometricVertexInput(input.position, vec4<f32>(1.0), input.uv, vec3<f32>(0.0), vec2<f32>(0.0)),
        scene.world, NativeWorldObject(object.world_rows, vec4<f32>(0.0), vec4<f32>(0.0), vec4<f32>(0.0), vec4<f32>(1.0)));
    return Varyings(output.clip_position, input.uv, vec4<f32>(output.fog_color, output.normal_fog.w), native_sonar_noise(scene.world.game_time.w).zw);
}
fn shade(input: Varyings) -> vec4<f32> {
    // weaponParam4 is supplied directly by its prepared producer, after viewport
    // selection; keeping it in the view group avoids a stale per-draw copy.
    return native_sonar_fragment(input.uv, input.position.xy, code.target_size.xy,
        input.fog.rgb, input.fog.a, input.noise_zw, scene.world.game_time.w,
        code.z_near.x, weapon.parameter0, code.viewport, scene, material,
        static_noise, static_sampler, lookup, lookup_sampler, float_z, float_z_sampler,
        sonar_color, sonar_color_sampler, sonar_depth, sonar_depth_sampler);
}
@fragment fn fragment_sonar(input: Varyings) -> @location(0) vec4<f32> { return shade(input); }
@fragment fn fragment_sonar_srgb(input: Varyings) -> @location(0) vec4<f32> {
    let encoded = shade(input);
    let linear = select(encoded.rgb / vec3<f32>(12.92), pow((encoded.rgb + vec3<f32>(0.055)) / vec3<f32>(1.055), vec3<f32>(2.4)), encoded.rgb > vec3<f32>(0.04045));
    return vec4<f32>(linear, encoded.a);
}

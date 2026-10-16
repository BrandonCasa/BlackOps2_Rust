// One per-view model matrix owner for reticles, objective surfaces and
// depth-projected decal volumes. Weapon component inputs remain with their rig.
struct NativeModelDrawMatrices {
    world_view: array<vec4<f32>, 4>,
    inverse_world_view: array<vec4<f32>, 4>,
    clip: array<vec4<f32>, 4>,
    inverse_transpose: array<vec4<f32>, 3>,
    parameter0: vec4<f32>,
    parameter4: vec4<f32>,
    ui3d_setup5: vec4<f32>,
    target_size: vec4<f32>,
}

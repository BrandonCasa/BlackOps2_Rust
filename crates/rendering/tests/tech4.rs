use bevy::{mesh::VertexAttributeValues, prelude::*};
use bo2_rendering::tech4::*;

fn fragment() -> Tech4Fragment {
    Tech4Fragment {
        normal: Vec3::Z,
        tangent: Vec3::X,
        bitangent: -Vec3::Y,
        camera_relative: -Vec3::Z,
        color: Vec3::ONE,
        inverse_sh: Vec3::ONE,
        fog: Vec4::new(0.0, 0.0, 0.0, 1.0),
    }
}
fn samples() -> Tech4Samples {
    Tech4Samples {
        base: Vec3::new(0.5, 0.25, 0.125),
        normal: Vec2::splat(2.015748 / 4.015748),
        specular_gloss: Vec4::ZERO,
        lightmap: [Vec4::ONE, Vec4::new(0.0, 0.0, 0.0, 1.0), Vec4::ONE],
        probe: Vec4::new(0.0, 0.0, 0.0, 1.0),
    }
}
#[test]
fn diffuse_squares_color_and_uses_alpha_radiance_without_extra_sun() {
    let mut f = fragment();
    f.color = Vec3::new(0.5, 1.0, 0.5);
    let s = samples();
    let actual = shade_samples(f, s, 1.0).unwrap();
    let expected = (s.base * f.color) / (1.0_f32 + 1e-6).sqrt();
    assert!(actual.truncate().abs_diff_eq(expected, 1e-6));
    assert_eq!(actual.w, 1.0);
}
#[test]
fn fog_blends_before_hdr_and_square_root() {
    let mut f = fragment();
    f.fog = Vec4::new(0.25, 0.5, 1.0, 0.0);
    assert!(
        shade_samples(f, samples(), 4.0)
            .unwrap()
            .abs_diff_eq(Vec4::new(1.0, 2.0_f32.sqrt(), 2.0, 1.0), 1e-6)
    );
    assert!(
        shade_samples(f, samples(), 0.0)
            .unwrap()
            .abs_diff_eq(Vec4::W, 1e-6)
    );
}
#[test]
fn mapped_direction_probe_modulation_and_sh_have_distinct_effects() {
    let mut f = fragment();
    let mut s = samples();
    s.base = Vec3::ZERO;
    s.specular_gloss = Vec4::ONE;
    s.probe = Vec4::new(0.5, 0.25, 0.125, 0.5);
    let a = shade_samples(f, s, 1.0).unwrap();
    f.inverse_sh = Vec3::splat(4.0);
    let b = shade_samples(f, s, 1.0).unwrap();
    assert!(b.truncate().abs_diff_eq(a.truncate() * 2.0, 1e-6));
    let globals = Tech4Globals {
        sh0: Vec4::new(2.0, 3.0, 4.0, 1.0),
        sh1: Vec4::ZERO,
        sh2: Vec4::ZERO,
        hdr_scale: 1.0,
    };
    assert_eq!(inverse_probe_sh(Vec3::X, globals), Vec3::splat(10.0));
    assert_eq!(
        inverse_probe_sh(Vec3::Z, globals),
        Vec3::new(0.5, 1.0 / 3.0, 0.25)
    );
    s.probe = Vec4::new(0.0, 0.0, 0.0, 1.0);
    s.base = Vec3::ONE;
    s.lightmap[0] = Vec4::new(0.0, 0.0, 0.0, 1.0);
    s.lightmap[1] = Vec4::ONE;
    s.lightmap[2] = Vec4::new(0.5, 0.5, 1.0, 1.0);
    let aligned = shade_samples(f, s, 1.0).unwrap();
    s.normal = Vec2::ONE;
    let tilted = shade_samples(f, s, 1.0).unwrap();
    assert!(tilted.x < aligned.x && tilted.x > 0.0);
}
#[test]
fn invalid_runtime_inputs_do_not_become_material_defaults() {
    let mut f = fragment();
    f.camera_relative = Vec3::ZERO;
    assert_eq!(
        shade_samples(f, samples(), 1.0),
        Err(Tech4Error::InvalidAttributes)
    );
    let mut globals = Tech4Globals {
        sh0: Vec4::ZERO,
        sh1: Vec4::ZERO,
        sh2: Vec4::ZERO,
        hdr_scale: 1.0,
    };
    globals.hdr_scale = f32::NAN;
    assert_eq!(globals.validate(), Err(Tech4Error::InvalidGlobals));
}
#[test]
fn mesh_adapts_all_direction_attributes_once_and_preserves_corner_contract() {
    let v = Tech4Vertex {
        position: [1.0, 2.0, 3.0],
        normal: [0.0, 0.0, 1.0],
        tangent: [1.0, 0.0, 0.0],
        handedness: -1.0,
        color: [0.5, 0.25, 0.125, 1.0],
        uv: [4.0, 5.0],
        lightmap_uv: [0.25, 0.75],
        fog_color_transmittance: [0.1, 0.2, 0.3, 0.4],
    };
    let mesh = tech4_mesh(&[v; 3]).unwrap();
    assert!(
        matches!(mesh.attribute(Mesh::ATTRIBUTE_POSITION),Some(VertexAttributeValues::Float32x3(p)) if p[0]==[1.0,3.0,-2.0])
    );
    assert!(
        matches!(mesh.attribute(Mesh::ATTRIBUTE_NORMAL),Some(VertexAttributeValues::Float32x3(p)) if p[0]==[0.0,1.0,-0.0])
    );
    assert!(
        matches!(mesh.attribute(Mesh::ATTRIBUTE_TANGENT),Some(VertexAttributeValues::Float32x4(p)) if p[0]==[1.0,0.0,-0.0,-1.0])
    );
    assert!(
        matches!(mesh.attribute(Mesh::ATTRIBUTE_UV_0),Some(VertexAttributeValues::Float32x2(p)) if p[0]==v.uv)
    );
    assert!(
        matches!(mesh.attribute(ATTRIBUTE_FOG),Some(VertexAttributeValues::Float32x4(p)) if p[0]==v.fog_color_transmittance)
    );
    assert!(tech4_mesh(&[v; 2]).is_err());
    let mut bad = v;
    bad.normal = [0.0; 3];
    assert!(tech4_mesh(&[bad; 3]).is_err());
}

#[test]
fn qualified_unorm_direction_remap_preserves_shader_wrap_boundary() {
    assert_eq!(
        decode_direction_input([0.25, 0.5, 0.75, 1.0]).unwrap(),
        [0.5, -1.0, -0.5]
    );
    let below = f32::from_bits(0.5f32.to_bits() - 1);
    assert!(decode_direction_input([below, 0.0, 0.0, 0.0]).unwrap()[0] > 0.99);
    assert!(decode_direction_input([0.0, 0.0, 0.0, 0.0]).is_err());
    assert!(decode_direction_input([f32::NAN, 0.0, 0.0, 1.0]).is_err());
}

#[test]
fn native_vertex_fog_handles_height_limit_and_sun_interpolation() {
    let fog = Tech4Fog {
        color: Vec4::new(0.25, 0.5, 0.75, 1.0),
        constants: Vec4::new(-1.0, -0.25, 0.0, 0.0),
        constants2: Vec4::new(0.5, 0.0, 0.0, 0.0),
        sun_direction: Vec4::Z,
        sun_color: Vec4::new(1.0, 0.0, 0.0, 0.5),
        sun_parameters: Vec4::new(0.0, 1.0, 0.0, 0.0),
        enabled: 1.0,
    };
    let horizontal = vertex_fog(Vec3::X * 8.0, fog).unwrap();
    assert!(horizontal.abs_diff_eq(Vec4::new(0.25, 0.5, 0.75, 0.5), 1e-6));
    let sun = vertex_fog(Vec3::Z * 8.0, fog).unwrap();
    assert!(sun.abs_diff_eq(Vec4::new(1.0, 0.0, 0.0, 0.75), 1e-6));
    assert!(vertex_fog(Vec3::ZERO, fog).is_err());
}

#[test]
fn captured_clip_retains_original_xy_and_w_and_adapts_viewport_depth() {
    let clip = CapturedClip {
        view_projection: Mat4::IDENTITY,
        camera_source: Vec3::new(10.0, 20.0, 30.0),
        enabled: 1.0,
        viewport_depth: Vec2::new(0.0, 0.75),
    };
    assert_eq!(
        captured_vertex_clip(Vec3::new(10.25, 19.5, 30.75), clip).unwrap(),
        Vec4::new(0.25, -0.5, 0.5625, 1.0)
    );
    let mut bad = clip;
    bad.camera_source.x = f32::NAN;
    assert!(captured_vertex_clip(Vec3::ZERO, bad).is_err());
}

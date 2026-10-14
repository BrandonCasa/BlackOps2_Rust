use super::{falloff::*, ordinary::*, unlit::*};

use super::*;

#[test]
#[ignore = "requires the authorized Steam installation"]
fn installed_reviewed_material_contracts() {
    let zones = crate::source::read_zones(&crate::source::installation().unwrap()).unwrap();
    let catalog = Catalog::new(&zones);
    let names = crate::geometry::material_names(&catalog, None).unwrap();
    let materials = read_used(&catalog, &names).unwrap();
    let mut checked = 0;
    let mut failures = Vec::new();
    for (name, material) in materials {
        let family = material["technique_set"].as_str().unwrap();
        let Some(recipe) = Recipe::lookup(family) else {
            continue;
        };
        if let Err(error) = recipe.validate(&name, &material) {
            failures.push(format!("{name} ({family}): {error}"));
            continue;
        }
        let index = recipe.technique();
        let mut changed = material.clone();
        let technique = changed["techniques"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|technique| technique["index"] == index)
            .unwrap();
        technique["passes"][0]["pixel_shader"] = json!("unreviewed.hlsl");
        assert!(
            recipe.validate(&name, &changed).is_err(),
            "{name}: changed shader accepted"
        );
        let mut changed = material.clone();
        let passes = changed["techniques"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|technique| technique["index"] == index)
            .unwrap()["passes"]
            .as_array_mut()
            .unwrap();
        passes.push(passes[0].clone());
        assert!(
            recipe.validate(&name, &changed).is_err(),
            "{name}: extra pass accepted"
        );
        checked += 1;
    }
    assert!(
        failures.is_empty(),
        "{} reviewed material contracts failed:\n{}",
        failures.len(),
        failures.join("\n")
    );
    assert!(checked > 0);
    println!(
        "Validated {checked} installed material contracts and rejected changed shaders/passes"
    );
}

#[test]
fn attachment_camo_uses_base_alpha_as_mask_and_keeps_gloss_independent() {
    let color = [0.2, 0.4, 0.6, 0.25];
    let specular = [0.1, 0.3, 0.8, 0.7];
    let detail = [0., 0.25, 1., 0.75];
    let result = camo_pixel(color, specular, detail, 1., 0.5);
    for (actual, expected) in result
        .into_iter()
        .flatten()
        .zip([0.3125, 0.53125, 0.75, 0.25, 0.19375, 0.39375, 0.875, 0.7])
    {
        assert!((actual - expected).abs() < 1e-6);
    }
    assert_eq!(
        camo_pixel(color, specular, detail, 1., 0.),
        [color, specular]
    );
    let protected = [0.2, 0.4, 0.6, 1.];
    for (actual, expected) in camo_pixel(protected, specular, detail, 1., 1.)
        .into_iter()
        .flatten()
        .zip([protected, specular].into_iter().flatten())
    {
        assert!((actual - expected).abs() < 1e-6);
    }
    let unmasked = [0.2, 0.4, 0.6, 0.];
    for (actual, expected) in camo_pixel(unmasked, specular, detail, 0.25, 1.)
        .into_iter()
        .flatten()
        .zip([0., 0., 0.75, 0., 0.35, 0.55, 1., 0.7])
    {
        assert!((actual - expected).abs() < 1e-6);
    }
}

#[test]
fn detail_normal_adds_weighted_slopes_before_normalization() {
    let base = texture::Image {
        width: 1,
        height: 1,
        depth: 1,
        faces: 1,
        format: 1,
        flags: 0,
        levels: vec![vec![255, 128, 0, 255]],
    };
    let detail = texture::Image {
        levels: vec![vec![128, 255, 255, 0]],
        ..base.clone()
    };
    let result = bake(&base, "normal", Some((&detail, 20, [8., 2., 0.5, 0.]))).unwrap();
    assert_eq!(result.levels[0], [232, 180, 180, 255]);
    assert_eq!(
        bake(&base, "normal", Some((&detail, 20, [8., 2., 0., 0.])))
            .unwrap()
            .levels,
        bake(&base, "normal", None).unwrap().levels
    );
    assert!(bake(&base, "normal", Some((&detail, 20, [8., f32::NAN, 1., 0.]))).is_err());
}
#[test]
fn implemented_dispatch_does_not_satisfy_converted_material_closure() {
    let native = BTreeMap::from([
        (
            "ordinary".into(),
            json!({"technique_set":"mc_lit_sm_r0c0n0_2z223015"}),
        ),
        (
            "layered".into(),
            json!({"technique_set":"lit_sm_r0c0n0_b1c1n1v1"}),
        ),
        (
            "unknown".into(),
            json!({"technique_set":"unreviewed_native_family"}),
        ),
    ]);
    assert_eq!(
        unimplemented(&native),
        ["unreviewed_native_family: 1 used materials, first unknown"]
    );
    assert_eq!(missing(&native, &BTreeMap::new()).len(), 3);
    let pending = BTreeMap::from([(
        "ordinary".into(),
        json!({"extras":{"surface":{"pending_runtime":"source fog operation"}}}),
    )]);
    assert_eq!(missing(&native, &pending).len(), 3);
    assert!(
        missing(&native, &pending)
            .iter()
            .any(|reason| reason == "ordinary: source fog operation")
    );
}

fn initial_additive_fixture() -> (Value, Value) {
    let state = (1 << 27) | (0x121_u32 << 16) | 0x8000 | 0x122;
    let native = json!({"name":"add","technique_set":"mc_unlit_add_ww82zw88",
        "techniques":[{"index":2,"passes":[{"pixel_shader":"pimp_shader_unlit_4b1d2e0b.hlsl",
            "vertex_shader":"pimp_shader_unlit_4876d431.hlsl",
            "vertex_routes":[{"source":0,"destination":0},{"source":1,"destination":2},{"source":2,"destination":5}],
            "arguments":[{"type":2,"value":2695565377_u32,"location":0,"size":1,"buffer":0},
                {"type":6,"value":3799286735_u32,"location":16,"size":4,"buffer":2}]}]}],
        "textures":[{"hash":2695565377_u32,"image":"add","sampler":20}],
        "constants":[{"hash":3799286735_u32,"literal":[1.,0.,0.,0.]}],
        "state_entries":[255,255,0],"state_bits":[[state,12]]});
    let image = texture::Image {
        width: 1,
        height: 1,
        depth: 1,
        faces: 1,
        format: 1,
        flags: 0,
        levels: vec![vec![128, 64, 32, 255]],
    };
    let converted = convert_unlit(
        "add",
        &native,
        unlit_recipe("mc_unlit_add_ww82zw88").unwrap(),
        &BTreeMap::from([("add".into(), image)]),
        &mut BTreeMap::new(),
        &mut Pack::default(),
    )
    .unwrap();
    (native, converted)
}

#[test]
fn native_rgb_write_bit_preserves_visible_materials() {
    let (mut native, converted) = initial_additive_fixture();
    assert_eq!(
        converted["extras"]["surface"]["presentation"]["visible"],
        true
    );
    let state = native["state_bits"][0][0].as_u64().unwrap();
    native["state_bits"][0][0] = json!(state & !(1 << 27));
    let image = texture::Image {
        width: 1,
        height: 1,
        depth: 1,
        faces: 1,
        format: 1,
        flags: 0,
        levels: vec![vec![128, 64, 32, 255]],
    };
    let converted = convert_unlit(
        "add",
        &native,
        unlit_recipe("mc_unlit_add_ww82zw88").unwrap(),
        &BTreeMap::from([("add".into(), image)]),
        &mut BTreeMap::new(),
        &mut Pack::default(),
    )
    .unwrap();
    assert_eq!(
        converted["extras"]["surface"]["presentation"]["visible"],
        false
    );
}

#[test]
fn initial_visual_acceptance_preserves_reported_limits_and_full_scope_gate() {
    let (native, converted) = initial_additive_fixture();
    let native = BTreeMap::from([("add".into(), native)]);
    let mut converted = BTreeMap::from([("add".into(), converted)]);
    let full = converted.clone();
    let reason = full["add"]["extras"]["surface"]["pending_runtime"].clone();
    let accepted = accept_initial_visual_approximations(&native, &mut converted).unwrap();
    assert_eq!(accepted.len(), 1);
    assert!(missing(&native, &converted).is_empty());
    assert_eq!(missing(&native, &full).len(), 1);
    let surface = &converted["add"]["extras"]["surface"];
    assert_eq!(surface["approximation_scope"], "first_playable_solo");
    assert!(
        surface["approximations"]
            .as_array()
            .unwrap()
            .contains(&reason)
    );
    assert_eq!(
        surface["channels"],
        full["add"]["extras"]["surface"]["channels"]
    );
    assert!(
        accept_initial_visual_approximations(&native, &mut converted)
            .unwrap()
            .is_empty()
    );
}

#[test]
fn initial_visual_acceptance_validates_each_generated_material_binding() {
    let (native, converted) = initial_additive_fixture();
    let native = BTreeMap::from([("add".into(), native)]);
    let variants = ["add", "add#atlas0", "add#periodic0", "add#atlas_invalid"];
    let original: BTreeMap<_, _> = variants
        .into_iter()
        .map(|name| {
            let mut value = converted.clone();
            value["name"] = json!(name);
            value["extras"]["surface"]["identity"] = json!(name);
            (name.to_owned(), value)
        })
        .collect();
    let mut converted = original.clone();
    assert_eq!(
        accept_initial_visual_approximations(&native, &mut converted)
            .unwrap()
            .len(),
        3
    );
    for name in &variants[..3] {
        let surface = &converted[*name]["extras"]["surface"];
        assert!(surface["pending_runtime"].is_null());
        assert_eq!(surface["identity"], *name);
        assert_eq!(surface["native"], native["add"]);
        assert_eq!(
            surface["channels"],
            original[*name]["extras"]["surface"]["channels"]
        );
    }
    assert!(converted["add#atlas_invalid"]["extras"]["surface"]["pending_runtime"].is_string());
    let mut mismatched = original.clone();
    mismatched.get_mut("add#atlas0").unwrap()["extras"]["surface"]["identity"] = json!("add");
    assert!(accept_initial_visual_approximations(&native, &mut mismatched).is_err());
    let mut missing = original.clone();
    missing.get_mut("add#periodic0").unwrap()["extras"]["surface"]["channels"] = json!([]);
    assert_eq!(
        accept_initial_visual_approximations(&native, &mut missing)
            .unwrap()
            .len(),
        2
    );
    assert!(missing["add#periodic0"]["extras"]["surface"]["pending_runtime"].is_string());
    let mut altered = original;
    altered.get_mut("add#atlas0").unwrap()["extras"]["surface"]["native"]["state_bits"][0][0] =
        json!(0);
    assert!(accept_initial_visual_approximations(&native, &mut altered).is_err());
}

#[test]
fn initial_visual_acceptance_never_fills_missing_payloads_or_source_state() {
    let (native, converted) = initial_additive_fixture();
    let native = BTreeMap::from([("add".into(), native)]);
    for field in [
        "channels",
        "source_effective_inputs",
        "source_color_matrix",
        "source_flag_motion",
        "source_reveal",
        "source_script_layers",
        "source_hero_lighting",
    ] {
        let mut converted = BTreeMap::from([("add".into(), converted.clone())]);
        let surface = &mut converted.get_mut("add").unwrap()["extras"]["surface"];
        surface[field] = if field == "channels" {
            json!([])
        } else {
            json!([169])
        };
        assert!(
            accept_initial_visual_approximations(&native, &mut converted)
                .unwrap()
                .is_empty()
        );
        assert_eq!(missing(&native, &converted).len(), 1);
        assert!(converted["add"]["extras"]["surface"]["pending_runtime"].is_string());
    }
    let mut altered = native.clone();
    altered.get_mut("add").unwrap()["techniques"][0]["passes"][0]["pixel_shader"] =
        json!("unreviewed_native_program");
    let mut converted = BTreeMap::from([("add".into(), converted)]);
    assert!(accept_initial_visual_approximations(&altered, &mut converted).is_err());
}

#[test]
fn unlit_scale_and_coverage_preserve_hdr_and_source_alpha_roles() {
    let image = texture::Image {
        width: 2,
        height: 1,
        depth: 1,
        faces: 1,
        format: 1,
        flags: 0,
        levels: vec![
            vec![128, 64, 32, 0, 255, 128, 64, 128],
            vec![128, 128, 128, 255],
        ],
    };
    let add = unlit_color(&image, 50., "add").unwrap();
    assert_eq!(add.format, 19);
    assert_eq!(add.levels.len(), 2);
    let pixels = texture::pixels(&add, 0).unwrap();
    assert!((pixels[0][0] - 12.59827).abs() < 0.008);
    assert_eq!(pixels[0][3], 1.);
    assert_eq!(pixels[1][0], 50.);
    assert_eq!(pixels[1][3], 1.);
    let blend = unlit_color(&image, 50., "blend").unwrap();
    let pixels = texture::pixels(&blend, 0).unwrap();
    assert_eq!(pixels[0], [0.; 4]);
    assert!((pixels[1][0] - 25.09804).abs() < 0.016);
    assert!((pixels[1][3] - 128. / 255.).abs() < 0.001);
    let replace = unlit_color(&image, 50., "opaque").unwrap();
    assert_eq!(texture::pixels(&replace, 0).unwrap()[0][3], 0.);
    let black = texture::Image {
        width: 1,
        height: 1,
        depth: 1,
        faces: 1,
        format: 1,
        flags: 0,
        levels: vec![vec![0, 0, 0, 255]],
    };
    assert_eq!(unlit_color(&black, 8., "opaque").unwrap().format, 1);
    assert!(unlit_color(&image, f32::NAN, "add").is_err());
}

#[test]
fn unlit_decal_multiply_keeps_transparent_neutral_and_opaque_black() {
    let image = texture::Image {
        width: 2,
        height: 1,
        depth: 1,
        faces: 1,
        format: 1,
        flags: 0,
        levels: vec![vec![32, 64, 128, 0, 0, 0, 0, 255]],
    };
    let result = unlit_color(&image, 1., "decal_multiply").unwrap();
    let pixels = texture::pixels(&result, 0).unwrap();
    assert_eq!(pixels[0], [1., 1., 1., 0.]);
    assert_eq!(pixels[1], [0., 0., 0., 1.]);
}

#[test]
fn unlit_additive_decal_weights_green_twice_red_coverage() {
    let image = texture::Image {
        width: 2,
        height: 1,
        depth: 1,
        faces: 1,
        format: 1,
        flags: 0,
        levels: vec![vec![255, 0, 0, 255, 0, 255, 0, 255]],
    };
    let result = unlit_color(&image, 1., "decal_add").unwrap();
    let pixels = texture::pixels(&result, 0).unwrap();
    assert!(pixels[0][3] > 0.);
    assert!((pixels[1][3] - 2. * pixels[0][3]).abs() <= 1. / 255.);
    assert_eq!(pixels[0][1], 0.);
    assert_eq!(pixels[1][0], 0.);
}

#[test]
fn unlit_multiply_ignores_unbound_scale_and_retains_native_value() {
    let native = json!({"technique_set":"mc_unlit_multiply_97923qz1",
        "techniques":[{"index":2,"passes":[{"pixel_shader":"pimp_shader_unlit_4f14dd58.hlsl",
            "vertex_shader":"pimp_shader_unlit_4876d431.hlsl",
            "vertex_routes":[{"source":0,"destination":0},{"source":1,"destination":2},{"source":2,"destination":5}],
            "arguments":[{"type":2,"value":2695565377_u32,"location":0,"size":1,"buffer":0}]}]}],
        "textures":[{"hash":2695565377_u32,"image":"multiply","sampler":20}],
        "constants":[{"hash":3799286735_u32,"literal":[0.,0.,0.,1.]}],
        "state_entries":[255,255,0],"state_bits":[[422086961_u32,28]]});
    let image = texture::Image {
        width: 1,
        height: 1,
        depth: 1,
        faces: 1,
        format: 1,
        flags: 0,
        levels: vec![vec![128, 128, 128, 255]],
    };
    assert_eq!(
        validate_unlit(
            "multiply",
            &native,
            unlit_recipe("mc_unlit_multiply_97923qz1").unwrap()
        )
        .unwrap(),
        1.
    );
    let output = unlit_color(&image, 0., "multiply").unwrap();
    assert_eq!(
        output.levels,
        unlit_color(&image, 50., "multiply").unwrap().levels
    );
    assert!(texture::pixels(&output, 0).unwrap()[0][0] > 0.);
    let mut pack = Pack::default();
    let converted = convert_unlit(
        "multiply",
        &native,
        unlit_recipe("mc_unlit_multiply_97923qz1").unwrap(),
        &BTreeMap::from([("multiply".into(), image)]),
        &mut BTreeMap::new(),
        &mut pack,
    )
    .unwrap();
    assert_eq!(
        converted["extras"]["surface"]["native"]["constants"],
        native["constants"]
    );
    assert!(
        converted["extras"]["surface"]["pending_runtime"]
            .as_str()
            .is_some()
    );
}

#[test]
fn falloff_last_layer_alpha_controls_coverage_without_changing_blended_rgb() {
    let samples = [[0., 0., 0., 0.], [0., 0., 0., 0.], [0.8, 0.4, 0.2, 0.5]];
    let low = falloff_pixel(samples, [1., 1.], Some(0.25), [0.; 4], "blend");
    let high = falloff_pixel(samples, [1., 1.], Some(0.75), [0.; 4], "blend");
    assert_eq!(low[..3], high[..3]);
    assert!((high[3] * high[3] - 3. * low[3] * low[3]).abs() < 1e-6);
    let opaque = falloff_pixel(samples, [1., 1.], None, [0., 0., 1., 0.], "opaque");
    assert_eq!(opaque[..3], low[..3]);
    assert_eq!(opaque[3], 1.);
    let hidden = falloff_pixel(samples, [1., 1.], Some(1.), [1.; 4], "add");
    assert_eq!(hidden, [0., 0., 0., 1.]);
}

#[test]
fn falloff_keeps_authored_mix_overshoot_and_saturates_only_script_inputs() {
    let samples = [[1., 0., 0., 1.], [0., 1., 0., 1.], [0., 0., 1., 1.]];
    let result = falloff_pixel(samples, [1.5, 1.], Some(1.), [0., 1., 0., 0.], "add");
    assert!(result[0] > 1.);
    assert!(result[2] < 0.);
    assert_eq!(result[1], 0.);
    assert_eq!(
        falloff_pixel(samples, [1.5, 1.], Some(1.), [-4., 5., 0.5, 0.], "add"),
        falloff_pixel(samples, [1.5, 1.], Some(1.), [0., 1., 0.5, 0.], "add")
    );
    let suppressed = falloff_pixel(samples, [1.5, 1.], Some(1.), [1., 1., 0., 0.], "blend");
    assert_eq!(suppressed, [0., 0., 1., 1.]);
}

#[test]
fn falloff_converted_texture_keeps_squared_hdr_overshoot_and_sqrt_alpha() {
    let source = |pixel| texture::Image {
        width: 1,
        height: 1,
        depth: 1,
        faces: 1,
        format: 1,
        flags: 0,
        levels: vec![pixel],
    };
    let red = source(vec![255, 0, 0, 255]);
    let clear = source(vec![0, 255, 0, 0]);
    let blue = source(vec![0, 0, 255, 255]);
    let mut layers = [UnlitLayer {
        scale: [1.; 2],
        offset: [0.; 2],
        origin: [0.5; 2],
        rotation: [0.; 2],
        scroll: [0.; 2],
        depth: 0.,
        alpha: Some(1.),
    }; 3];
    layers[0].alpha = Some(1.5);
    let output = falloff_image([&red, &clear, &blue], [20; 3], layers, [0.; 4], 2., "add").unwrap();
    assert_eq!(texture::pixels(&output, 0).unwrap()[0], [9., 0., 1., 1.]);
    layers[0].alpha = Some(0.);
    layers[2].alpha = Some(0.25);
    let output =
        falloff_image([&red, &clear, &blue], [20; 3], layers, [0.; 4], 1., "blend").unwrap();
    assert_eq!(texture::pixels(&output, 0).unwrap()[0], [0., 0., 1., 0.5]);
}

#[test]
fn four_layer_emission_keeps_last_alpha_in_coverage_and_unclamped_mix_weights() {
    let layers = [
        [1., 0., 0., 1.],
        [0., 1., 0., 1.],
        [0., 0., 1., 1.],
        [0.5, 0.25, 0.75, 1.],
    ];
    assert_eq!(
        four_layer_pixel(layers, [0., 0., 0., 1.], [0.; 4]),
        [0.5, 0.25, 0.75, 1.]
    );
    assert_eq!(
        four_layer_pixel(layers, [0., 0., 0., 1.], [0., 0., 0., 1.]),
        [0., 0., 0., 1.]
    );
    let overshoot = four_layer_pixel(layers, [2., 0., 0., 1.], [0.; 4]);
    assert!(overshoot[0] > 1.);
    assert!(overshoot[1] < 0. && overshoot[2] < 0.);
    assert_eq!(
        four_layer_pixel(layers, [-2.5, 1., 3., 1.], [0.; 4]),
        [0., 0., 0., 1.]
    );
    assert_eq!(
        four_layer_pixel(layers, [2., 0., 0., 1.], [-2., 3., 0., 0.]),
        four_layer_pixel(layers, [2., 0., 0., 1.], [0., 1., 0., 0.])
    );
}

#[test]
fn viewmodel_alpha_keeps_raw_coverage_when_emission_is_zero() {
    let image = texture::Image {
        width: 1,
        height: 1,
        depth: 1,
        faces: 1,
        format: 1,
        flags: 0,
        levels: vec![vec![255, 128, 64, 64]],
    };
    let alpha = unlit_color(&image, 0., "source_vm_alpha").unwrap();
    let pixel = texture::pixels(&alpha, 0).unwrap()[0];
    assert_eq!(pixel[..3], [0.; 3]);
    assert!((pixel[3] - 64. / 255.).abs() < 1e-6);
    let sqrt = unlit_color(&image, 0., "source_vm_blend").unwrap();
    assert!(texture::pixels(&sqrt, 0).unwrap()[0][3] > pixel[3]);
}

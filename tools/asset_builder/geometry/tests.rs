use super::*;

#[test]
fn flag_packed_basis_survives_triangle_remapping_and_gltf_packing() {
    let words = [
        [0x11223344_u32, 0x00755122],
        [0x211001ff, 0x05566778],
        [0x00155477, 0x01aa3322],
    ];
    let mut vertices = vec![0; 3 * 32];
    for (index, vertex) in vertices.chunks_exact_mut(32).enumerate() {
        vertex[..4].copy_from_slice(&(index as f32).to_le_bytes());
        vertex[12..16].copy_from_slice(&1_f32.to_le_bytes());
        vertex[24..28].copy_from_slice(&words[index][0].to_le_bytes());
        vertex[28..32].copy_from_slice(&words[index][1].to_le_bytes());
    }
    let indices: Vec<_> = [0_u16, 1, 2]
        .into_iter()
        .flat_map(u16::to_le_bytes)
        .collect();
    let surface = Surface::read("flag".into(), 0, &vertices, 32, &indices, "flag".into()).unwrap();
    assert_eq!(surface.packed_basis, [words[2], words[1], words[0]]);
    let selected = surface.select_triangles(&[0, 0]).unwrap();
    assert_eq!(
        selected.packed_basis,
        [words[2], words[1], words[0], words[2], words[1], words[0]]
    );
    let mut scene = crate::gltf::Scene::new("flag");
    let converted = std::collections::BTreeMap::from([(
        "flag".into(),
        serde_json::json!({"name":"flag","extras":{"surface":{"source_flag_motion":{}}}}),
    )]);
    let materials = scene.materials([&selected], &converted).unwrap();
    scene.mesh("flag", [&selected], &materials, false).unwrap();
    let index = scene.data["meshes"][0]["primitives"][0]["attributes"]["_ORIGINS_PACKED_BASIS"]
        .as_u64()
        .unwrap() as usize;
    let accessor = &scene.data["accessors"][index];
    assert_eq!(
        (
            accessor["componentType"].as_u64(),
            accessor["type"].as_str(),
            accessor["count"].as_u64()
        ),
        (Some(5125), Some("VEC2"), Some(6))
    );
    let start =
        scene.data["bufferViews"][accessor["bufferView"].as_u64().unwrap() as usize]["byteOffset"]
            .as_u64()
            .unwrap() as usize;
    let mut pack = crate::pack::Pack::default();
    scene.write("prepared/flag.gltf", &mut pack).unwrap();
    let bytes = pack.get("prepared/flag.bin").unwrap();
    let actual: Vec<[u32; 2]> = (0..6)
        .map(|i| {
            [
                u32_at(bytes, start + i * 8),
                u32_at(bytes, start + i * 8 + 4),
            ]
        })
        .collect();
    assert_eq!(actual, selected.packed_basis);
}

#[test]
fn unlit_vertex_alpha_matches_constant_rgb_source_interpolation() {
    let mut vertices = vec![0; 3 * 32];
    for (index, vertex) in vertices.chunks_exact_mut(32).enumerate() {
        vertex[..4].copy_from_slice(&(index as f32).to_le_bytes());
        vertex[12..16].copy_from_slice(&1_f32.to_le_bytes());
        vertex[24..28].copy_from_slice(&1_u32.to_le_bytes());
        vertex[28..32].copy_from_slice(&1024_u32.to_le_bytes());
    }
    let indices: Vec<_> = [0_u16, 1, 2]
        .into_iter()
        .flat_map(u16::to_le_bytes)
        .collect();
    let mut surface =
        Surface::read("unlit".into(), 0, &vertices, 32, &indices, "unlit".into()).unwrap();
    surface.colors = vec![[128, 64, 32, 0], [128, 64, 32, 128], [128, 64, 32, 255]];
    for (transfer, coverage) in [("gamma2_alpha", "unlit_blend"), ("gamma2", "unlit_add")] {
        let mut scene = crate::gltf::Scene::new("unlit");
        let converted = std::collections::BTreeMap::from([(
            "unlit".into(),
            serde_json::json!({"name":"unlit",
            "extras":{"surface":{"source_vertex_color_transfer":transfer,"source_coverage":coverage}}}),
        )]);
        let materials = scene.materials([&surface], &converted).unwrap();
        scene.mesh("unlit", [&surface], &materials, false).unwrap();
        let color_index = scene.data["meshes"][0]["primitives"][0]["attributes"]["COLOR_0"]
            .as_u64()
            .unwrap() as usize;
        let accessor = &scene.data["accessors"][color_index];
        assert_eq!(accessor["componentType"], 5126);
        let start = scene.data["bufferViews"][accessor["bufferView"].as_u64().unwrap() as usize]
            ["byteOffset"]
            .as_u64()
            .unwrap() as usize;
        let mut pack = crate::pack::Pack::default();
        scene.write("prepared/unlit.gltf", &mut pack).unwrap();
        let bytes = pack.get("prepared/unlit.bin").unwrap();
        let values: [[f32; 4]; 3] = std::array::from_fn(|i| {
            std::array::from_fn(|c| f32::from_bits(u32_at(bytes, start + i * 16 + c * 4)))
        });
        let weights = [0.2, 0.3, 0.5];
        let alpha = 0.3 * 128. / 255. + 0.5;
        for c in 0..3 {
            let interpolated: f32 = (0..3).map(|i| values[i][c] * weights[i]).sum();
            let tint = [128., 64., 32.][c] / 255.;
            let expected = tint * tint * if coverage == "unlit_blend" { alpha } else { 1. };
            assert!((interpolated - expected).abs() < 0.000001);
        }
        if coverage == "unlit_add" {
            assert!(values.iter().all(|value| value[3] == 1.));
        } else {
            assert_eq!(values.map(|value| value[3]), [0., 128. / 255., 1.]);
        }
    }
}

#[test]
fn native_binding_groups_and_weight_endpoints() {
    let words = [
        128_u16, 192, 64, 65535, 64, 128, 16384, 192, 32768, 0, 64, 8192, 128, 16384, 192, 32768,
    ];
    let bytes: Vec<_> = words.into_iter().flat_map(u16::to_le_bytes).collect();
    let values = blended_bindings([1; 4], &bytes, 4, 4).unwrap();
    assert_eq!(
        values[0],
        Influence {
            joints: [2, 0, 0, 0],
            weights: [1., 0., 0., 0.]
        }
    );
    assert_eq!(
        values[1],
        Influence {
            joints: [3, 1, 0, 0],
            weights: [0.0000152587890625, 0.9999847412109375, 0., 0.]
        }
    );
    assert_eq!(
        values[2],
        Influence {
            joints: [1, 2, 3, 0],
            weights: [0.25, 0.25, 0.5, 0.]
        }
    );
    assert_eq!(
        values[3],
        Influence {
            joints: [0, 1, 2, 3],
            weights: [0.125, 0.125, 0.25, 0.5]
        }
    );
    for offset in [65_u16, 256] {
        assert!(blended_bindings([1, 0, 0, 0], &offset.to_le_bytes(), 4, 1).is_err());
    }
    let overflow: Vec<_> = [0_u16, 64, 65535, 128, 2]
        .into_iter()
        .flat_map(u16::to_le_bytes)
        .collect();
    assert!(blended_bindings([0, 0, 1, 0], &overflow, 4, 1).is_err());
    assert!(blended_bindings([1; 4], &bytes[..bytes.len() - 1], 4, 4).is_err());
    assert!(blended_bindings([1; 4], &bytes, 4, 3).is_err());
    let rigid: Vec<_> = [192_u16, 2, 65535, 65535, 0, 0, 64, 1, 0, 0, 0, 0]
        .into_iter()
        .flat_map(u16::to_le_bytes)
        .collect();
    assert_eq!(
        rigid_bindings(&rigid, 4, 3)
            .unwrap()
            .iter()
            .map(|value| value.joints[0])
            .collect::<Vec<_>>(),
        [3, 3, 1]
    );
    assert!(rigid_bindings(&rigid, 4, 2).is_err());
}

#[test]
fn binding_remap_follows_raw_vertices_through_triangle_selection() {
    let mut vertices = vec![0; 3 * 32];
    for (index, vertex) in vertices.chunks_exact_mut(32).enumerate() {
        vertex[..4].copy_from_slice(&(index as f32).to_le_bytes());
        vertex[12..16].copy_from_slice(&1_f32.to_le_bytes());
        vertex[24..28].copy_from_slice(&1_u32.to_le_bytes());
        vertex[28..32].copy_from_slice(&1024_u32.to_le_bytes());
    }
    let indices: Vec<_> = [2_u16, 0, 1, 1, 2, 0]
        .into_iter()
        .flat_map(u16::to_le_bytes)
        .collect();
    let mut surface = Surface::read(
        "binding-test".into(),
        0,
        &vertices,
        32,
        &indices,
        "binding-test".into(),
    )
    .unwrap();
    surface.bindings = Some(Bindings {
        flags: 0x80,
        blended: surface
            .source_vertices
            .iter()
            .map(|&vertex| {
                Some(Influence {
                    joints: [vertex, 0, 0, 0],
                    weights: [1., 0., 0., 0.],
                })
            })
            .collect(),
        rigid: surface
            .source_vertices
            .iter()
            .map(|&vertex| {
                Some(Influence {
                    joints: [3 - vertex, 0, 0, 0],
                    weights: [1., 0., 0., 0.],
                })
            })
            .collect(),
    });
    let selected = surface.select_triangles(&[1, 0]).unwrap();
    let mut scene = crate::gltf::Scene::new("binding-test");
    let converted = std::collections::BTreeMap::from([(
        "binding-test".into(),
        serde_json::json!({"name":"binding-test"}),
    )]);
    let materials = scene.materials([&selected], &converted).unwrap();
    let mesh = scene
        .mesh("binding-test", [&selected], &materials, true)
        .unwrap();
    let owner = scene.node(serde_json::json!({"name":"binding-test","mesh":mesh}));
    let (identity, inverse) = bind_pose([0., 0., 0., 1.], [0.; 3], 2.);
    let joints = vec![
        Joint {
            name: None,
            parent: None,
            bind: identity,
            bind_local: identity,
            inverse_bind: inverse,
            default_rotation: [0., 0., 0., 1.],
            default_translation: [0.; 3],
        },
        Joint {
            name: Some("second-root".into()),
            parent: None,
            bind: identity,
            bind_local: identity,
            inverse_bind: inverse,
            default_rotation: [0., 0., 0., 1.],
            default_translation: [0.; 3],
        },
        Joint {
            name: Some(String::new()),
            parent: Some(0),
            bind: identity,
            bind_local: identity,
            inverse_bind: inverse,
            default_rotation: [0., 0., 0., 1.],
            default_translation: [1., 2., 3.],
        },
    ];
    scene.skin(owner, &joints).unwrap();
    let mut pack = crate::pack::Pack::default();
    scene
        .write("prepared/binding-test.gltf", &mut pack)
        .unwrap();
    let document: serde_json::Value =
        serde_json::from_slice(pack.get("prepared/binding-test.gltf").unwrap()).unwrap();
    let attributes = &document["meshes"][0]["primitives"][0]["attributes"];
    let accessor = &document["accessors"][attributes["JOINTS_0"].as_u64().unwrap() as usize];
    assert_eq!(accessor["componentType"], 5123);
    assert_eq!(accessor["count"], 6);
    assert_eq!(accessor["type"], "VEC4");
    let view = &document["bufferViews"][accessor["bufferView"].as_u64().unwrap() as usize];
    let bytes = pack.get("prepared/binding-test.bin").unwrap();
    let start = view["byteOffset"].as_u64().unwrap() as usize;
    assert_eq!(
        (0..6)
            .map(|vertex| u16_at(bytes, start + vertex * 8))
            .collect::<Vec<_>>(),
        [0, 2, 1, 1, 0, 2]
    );
    let weights = &document["accessors"][attributes["WEIGHTS_0"].as_u64().unwrap() as usize];
    assert_eq!(weights["componentType"], 5126);
    assert_eq!(weights["count"], 6);
    assert_eq!(weights["type"], "VEC4");
    assert_eq!(document["nodes"][owner]["skin"], 0);
    assert_eq!(document["skins"][0]["joints"], serde_json::json!([1, 2, 3]));
    assert_eq!(
        document["nodes"][owner]["children"],
        serde_json::json!([1, 2])
    );
    assert_eq!(document["nodes"][1]["children"], serde_json::json!([3]));
    assert!(document["nodes"][1].get("matrix").is_none());
    assert_eq!(
        document["nodes"][1]["translation"],
        serde_json::json!([0., 0., 0.])
    );
    assert_eq!(
        document["nodes"][1]["rotation"],
        serde_json::json!([0., 0., 0., 1.])
    );
    assert!(document["nodes"][1].get("name").is_none());
    assert_eq!(document["nodes"][2]["name"], "second-root");
    assert_eq!(document["nodes"][3]["name"], "");
    assert_eq!(
        document["nodes"][3]["translation"],
        serde_json::json!([0., 0., 0.])
    );
    assert_eq!(
        document["nodes"][3]["extras"]["native_local_default"]["translation"],
        serde_json::json!([1., 2., 3.])
    );
    let inverse = &document["accessors"][document["skins"][0]["inverseBindMatrices"]
        .as_u64()
        .unwrap() as usize];
    assert_eq!(inverse["componentType"], 5126);
    assert_eq!(inverse["count"], 3);
    assert_eq!(inverse["type"], "MAT4");
    let view = &document["bufferViews"][inverse["bufferView"].as_u64().unwrap() as usize];
    assert!(view.get("target").is_none());
    let start = view["byteOffset"].as_u64().unwrap() as usize;
    assert_eq!(
        (0..16)
            .map(|index| f32::from_bits(u32_at(bytes, start + index * 4)))
            .collect::<Vec<_>>(),
        identity
    );
    let bindings = selected.bindings.unwrap();
    assert_eq!(bindings.flags, 0x80);
    assert_eq!(
        bindings
            .blended
            .iter()
            .map(|value| value.unwrap().joints[0])
            .collect::<Vec<_>>(),
        [0, 2, 1, 1, 0, 2]
    );
    assert_eq!(
        bindings
            .rigid
            .iter()
            .map(|value| value.unwrap().joints[0])
            .collect::<Vec<_>>(),
        [3, 1, 2, 2, 3, 1]
    );
}

#[test]
fn bind_pose_converts_native_space_and_preserves_parent_offsets() {
    let q = [0., 0., 0.5_f32.sqrt(), 0.5_f32.sqrt()];
    let (parent, inverse) = bind_pose(q, [10., 20., 30.], 2.);
    let mapped: [f32; 3] = std::array::from_fn(|row| parent[12 + row] + parent[row] * 0.0254);
    for (a, b) in mapped.into_iter().zip([0.254, 0.762, -0.5334]) {
        assert!((a - b).abs() < 0.000001);
    }
    let restored: [f32; 3] = std::array::from_fn(|row| {
        inverse[12 + row]
            + (0..3)
                .map(|column| inverse[column * 4 + row] * mapped[column])
                .sum::<f32>()
    });
    for (a, b) in restored.into_iter().zip([0.0254, 0., 0.]) {
        assert!((a - b).abs() < 0.000001);
    }
    let (child, _) = bind_pose(q, [10., 25., 30.], 2.);
    let local = compose(inverse, child);
    for (a, b) in local[12..15].iter().copied().zip([0.127, 0., 0.]) {
        assert!((a - b).abs() < 0.000001);
    }
    let reconstructed = compose(parent, local);
    for (a, b) in reconstructed.into_iter().zip(child) {
        assert!((a - b).abs() < 0.000001);
    }
}

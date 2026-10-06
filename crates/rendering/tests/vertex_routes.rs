use bevy::mesh::{Mesh, VertexAttributeValues};
use bo2_assets::{
    full_scene::*,
    geometry_snapshot::read_geometry_snapshot,
    secondary_stream::{SourceSecondaryCorner, SourceTexcoord},
    shader_bank::{ShaderDeclaration, ShaderStage, VertexInput as ReflectedInput},
    vertex_signature::*,
};
use bo2_rendering::{original_material::SourceAttribute, vertex_routes::*};
fn declaration(inputs: &[(u32, u8)]) -> ShaderDeclaration {
    ShaderDeclaration {
        source_sha256: "0".repeat(64),
        file: "synthetic.wgsl".into(),
        sha256: "1".repeat(64),
        stage: ShaderStage::Vertex,
        entry_point: "main".into(),
        resources: vec![],
        inputs: inputs
            .iter()
            .map(|(location, components)| ReflectedInput {
                location: *location,
                components: *components,
            })
            .collect(),
    }
}
fn signature(inputs: &[(&str, u32, u32, u8)]) -> ShaderVertexSignature {
    ShaderVertexSignature {
        source_shader_sha256: "0".repeat(64),
        inputs: inputs
            .iter()
            .map(|(name, index, register, mask)| SourceVertexInput {
                semantic_name: (*name).into(),
                semantic_index: *index,
                register: *register,
                component_mask: *mask,
                used_mask: *mask,
                component_type: InputComponentType::Float32,
                system_value: InputSystemValue::None,
            })
            .collect(),
    }
}
fn scene(format: u32) -> FullSceneInput {
    let mut bytes = vec![0; 64];
    bytes[..8].copy_from_slice(b"BO2DGS\0\0");
    bytes[8..12].copy_from_slice(&1u32.to_le_bytes());
    for (o, v) in [(16, 3u64), (24, 6), (32, 1), (40, 172)] {
        bytes[o..o + 8].copy_from_slice(&v.to_le_bytes());
    }
    for p in [[0.0f32, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]] {
        for c in p {
            bytes.extend(c.to_le_bytes());
        }
    }
    for i in [0u32, 1, 2, 0, 2, 1] {
        bytes.extend(i.to_le_bytes());
    }
    bytes.extend(0u64.to_le_bytes());
    bytes.extend(6u64.to_le_bytes());
    bytes.extend(format.to_le_bytes());
    bytes.extend(0u32.to_le_bytes());
    for c in [0.0f32, 0.0, 0.0, 1.0, 1.0, 0.0] {
        bytes.extend(c.to_le_bytes());
    }
    let geometry =
        read_geometry_snapshot(&mut std::io::Cursor::new(&bytes), bytes.len() as u64).unwrap();
    FullSceneInput {
        geometry,
        corner_uv: vec![
            [0.0, 0.0],
            [0.0, 0.0],
            [0.0, 0.0],
            [0.5, 0.75],
            [0.0, 0.0],
            [0.0, 0.0],
        ],
        corner_attributes: Some(vec![
            SourceCornerAttributes {
                position_w: 0.25,
                color: [0.25; 4],
                normal_unorm: [0.5, 0.25, 0.75, 1.0],
                tangent_unorm: [0.5, 1.0, 0.5, 0.0],
                lightmap_uv: [0.0, 1.0]
            };
            6
        ]),
        surfaces: vec![SceneSurface {
            material: 0,
            source_vertex_format: format,
            lightmap: None,
            probe: None,
        }],
        images: vec![],
        materials: vec![],
        lightmaps: vec![],
        probes: vec![],
    }
}
#[test]
fn semantic_routes_follow_native_registers_and_reject_conflicts_or_missing_streams() {
    let d = declaration(&[(8, 4), (4, 2), (1, 4)]);
    let s = signature(&[
        ("POSITION", 0, 8, 15),
        ("TEXCOORD", 0, 4, 3),
        ("TEXCOORD", 3, 1, 15),
    ]);
    let routing = [[0, 0], [2, 5], [6, 8]];
    let result = route_vertex_signature(&d, &s, &routing, 1).unwrap();
    assert_eq!(result[0].location, 1);
    assert_eq!(result[0].attribute, SourceAttribute::SecondaryTexcoord0);
    assert_eq!(result[1].location, 4);
    assert_eq!(result[1].attribute, SourceAttribute::Uv0);
    assert_eq!(result[2].location, 8);
    assert_eq!(result[2].attribute, SourceAttribute::Position4);
    assert!(matches!(
        route_vertex_signature(&d, &s, &routing, 0),
        Err(VertexRouteError::MissingSource { .. })
    ));
    assert_eq!(
        route_vertex_signature(&d, &s, &[[0, 0], [1, 0]], 1),
        Err(VertexRouteError::ConflictingDestination)
    );
    assert_eq!(
        route_vertex_signature(&d, &s, &[[0, 255]], 1),
        Err(VertexRouteError::InvalidRoute)
    );
    assert!(matches!(
        route_vertex_signature(&d, &s, &[[0, 0]], 1),
        Err(VertexRouteError::MissingSemantic { .. })
    ));
    assert_eq!(
        route_vertex_signature(&d, &s, &routing, 6),
        Err(VertexRouteError::UnsupportedSourceFormat(6))
    );
    let mut bad = s.clone();
    bad.source_shader_sha256 = "2".repeat(64);
    assert_eq!(
        route_vertex_signature(&d, &bad, &routing, 1),
        Err(VertexRouteError::SignatureIdentity)
    );
    let bad = declaration(&[(8, 3), (4, 2), (1, 4)]);
    assert_eq!(
        route_vertex_signature(&bad, &s, &routing, 1),
        Err(VertexRouteError::ReflectionMismatch)
    );
}
#[test]
fn source_positions_secondary_uv_seams_and_direct3d_component_expansion_are_preserved() {
    let d = declaration(&[(0, 3), (2, 2), (6, 4)]);
    let s = signature(&[
        ("POSITION", 0, 0, 7),
        ("TEXCOORD", 0, 2, 3),
        ("TEXCOORD", 3, 6, 15),
    ]);
    let route = route_vertex_signature(&d, &s, &[[0, 0], [2, 5], [6, 8]], 1).unwrap();
    let input = scene(1);
    let mut secondary = vec![
        SourceSecondaryCorner {
            texcoord3: Some(SourceTexcoord::Float2([0.25, 0.75])),
            ..Default::default()
        };
        6
    ];
    secondary[3].texcoord3 = Some(SourceTexcoord::Float2([0.875, 0.125]));
    let mesh = routed_world_mesh(&input, 0, Some(&secondary), &route).unwrap();
    assert!(
        matches!(mesh.attribute(Mesh::ATTRIBUTE_POSITION),Some(VertexAttributeValues::Float32x3(p)) if p[2]==[0.0,0.0,-1.0] && p[0]==p[3])
    );
    assert!(
        matches!(mesh.attribute(SourceAttribute::Position3.attribute()),Some(VertexAttributeValues::Float32x3(p)) if p[2]==[0.0,1.0,0.0])
    );
    assert!(
        matches!(mesh.attribute(SourceAttribute::Position4.attribute()),Some(VertexAttributeValues::Float32x4(p)) if p[0][3]==0.25)
    );
    assert!(
        matches!(mesh.attribute(SourceAttribute::SecondaryTexcoord0.attribute()),Some(VertexAttributeValues::Float32x4(uv)) if uv[0]==[0.25,0.75,0.0,1.0] && uv[3]==[0.875,0.125,0.0,1.0])
    );
    assert!(
        matches!(mesh.attribute(SourceAttribute::Uv0.attribute()),Some(VertexAttributeValues::Float32x2(uv)) if uv[3]==[0.5,0.75])
    );
    assert_eq!(input.geometry.positions()[2], [0.0, 1.0, 0.0]);
    assert!(matches!(
        routed_world_mesh(&input, 0, None, &route),
        Err(VertexRouteError::MissingSecondary)
    ));
    secondary[0].texcoord3 = Some(SourceTexcoord::Float4([1.0; 4]));
    assert!(matches!(
        routed_world_mesh(&input, 0, Some(&secondary), &route),
        Err(VertexRouteError::InvalidAttributes)
    ));
}
#[test]
fn qualified_half4_color_and_second_uv_layouts_are_not_interchangeable() {
    let d = declaration(&[(0, 4), (6, 4), (7, 4), (8, 4)]);
    let s = signature(&[
        ("POSITION", 0, 0, 15),
        ("TEXCOORD", 3, 6, 15),
        ("TEXCOORD", 4, 7, 15),
        ("TEXCOORD", 5, 8, 15),
    ]);
    let route = route_vertex_signature(&d, &s, &[[0, 0], [6, 8], [7, 9], [8, 10]], 7).unwrap();
    let mut secondary = vec![
        SourceSecondaryCorner {
            texcoord3: Some(SourceTexcoord::Float4([0.1, 0.2, 0.3, 0.4])),
            texcoord4: Some([0.5, 0.6]),
            color5: Some([0.75; 4]),
            ..Default::default()
        };
        6
    ];
    let mesh = routed_world_mesh(&scene(7), 0, Some(&secondary), &route).unwrap();
    assert!(
        matches!(mesh.attribute(SourceAttribute::SecondaryTexcoord1.attribute()),Some(VertexAttributeValues::Float32x4(uv)) if uv[0]==[0.5,0.6,0.0,1.0])
    );
    assert!(
        matches!(mesh.attribute(SourceAttribute::SecondaryNormalTransform0.attribute()),Some(VertexAttributeValues::Float32x4(c)) if c[0]==[0.75;4])
    );
    secondary[0].color5 = Some([2.0; 4]);
    assert!(routed_world_mesh(&scene(7), 0, Some(&secondary), &route).is_err());
    assert!(matches!(
        route_vertex_signature(&d, &s, &[[0, 0], [6, 8], [7, 9], [8, 10]], 5),
        Err(VertexRouteError::MissingSource { .. })
    ));
    let d = declaration(&[(6, 4)]);
    let s = signature(&[("TEXCOORD", 3, 6, 15)]);
    assert!(route_vertex_signature(&d, &s, &[[6, 8]], 3).is_ok());
}

#[test]
#[ignore = "requires external semantic-admitted shader bank"]
fn external_all_native_shader_routes_have_qualified_source_formats() {
    let path = std::env::var_os("BO2_SHADER_BANK").expect("BO2_SHADER_BANK");
    let bank = bo2_assets::shader_bank::load_shader_bank(std::path::Path::new(&path)).unwrap();
    let mut hashes = std::collections::BTreeSet::new();
    let mut passes = 0;
    for family in &bank.families {
        for technique in &family.techniques {
            for pass in &technique.passes {
                hashes.insert(
                    bank.shaders[pass.vertex_shader]
                        .declaration
                        .source_sha256
                        .clone(),
                );
                assert!(
                    [0, 1, 2, 3, 4, 5, 7]
                        .into_iter()
                        .any(|format| world_vertex_route(&bank, pass, format).is_ok()),
                    "no qualified source format for native vertex signature"
                );
                passes += 1;
            }
        }
    }
    assert_eq!(hashes.len(), 51);
    assert_eq!(passes, 1404);
    println!("native route census:51 vertex identities/1404 passes have qualified format");
    let scene_path = std::env::var_os("BO2_FULL_SCENE").expect("BO2_FULL_SCENE");
    let secondary_path = std::env::var_os("BO2_SECONDARY_CORNERS").expect("BO2_SECONDARY_CORNERS");
    let scene = load_full_scene(std::path::Path::new(&scene_path)).unwrap();
    let secondary = bo2_assets::secondary_stream::load_secondary_corners(
        std::path::Path::new(&secondary_path),
        &scene.geometry,
        &bank.geometry_sha256,
    )
    .unwrap();
    let mut ready = 0usize;
    let mut failures = std::collections::BTreeMap::new();
    let mut meshes = std::collections::BTreeSet::new();
    for (surface, source) in scene.surfaces.iter().enumerate() {
        let material = bank
            .materials
            .iter()
            .find(|m| m.scene_material == source.material)
            .unwrap();
        for technique in &bank.families[material.family].techniques {
            if material.state_entries[usize::from(technique.index)] == u8::MAX {
                continue;
            }
            for pass in &technique.passes {
                match world_vertex_route(&bank, pass, source.source_vertex_format) {
                    Ok(route) => {
                        ready += 1;
                        if meshes.insert((pass.vertex_shader, source.source_vertex_format)) {
                            routed_world_mesh(&scene, surface, Some(&secondary), &route).unwrap();
                        }
                    }
                    Err(error) => {
                        *failures.entry(format!("{error:?}")).or_insert(0usize) += 1;
                    }
                }
            }
        }
    }
    println!(
        "actual world routing: {} surfaces/{ready} admitted pass uses/{} vertex-format meshes; unsupported {failures:?}",
        scene.surfaces.len(),
        meshes.len()
    );
    assert!(failures.is_empty());
}

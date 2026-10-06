use bo2_assets::{appearance::AppearanceError, shader_bank::*};
use sha2::{Digest, Sha256};
fn fixture() -> ShaderBankManifest {
    let shader = |stage, source: u32, file: &str| ShaderDeclaration {
        source_sha256: format!("{source:064x}"),
        file: file.into(),
        sha256: format!("{:x}", Sha256::digest(b"synthetic WGSL")),
        stage,
        entry_point: "main".into(),
        inputs: vec![VertexInput {
            location: 0,
            components: 4,
        }],
        resources: vec![],
    };
    ShaderBankManifest {
        version: 1,
        geometry_sha256: "0".repeat(64),
        scene_image_count: 0,
        vertex_signatures: None,
        shaders: vec![
            shader(ShaderStage::Vertex, 1, "vs.wgsl"),
            shader(ShaderStage::Fragment, 2, "ps.wgsl"),
        ],
        families: vec![TechniqueFamily {
            source_technique_set: "synthetic".into(),
            techniques: vec![Technique {
                index: 4,
                flags: 0,
                passes: vec![ShaderPass {
                    vertex_shader: 0,
                    pixel_shader: 1,
                    routing: vec![[0, 0]],
                    args: vec![PassArgument {
                        stage: ShaderStage::Vertex,
                        destination: ArgumentDestination::Uniform {
                            buffer: 3,
                            offset: 0,
                            size: 64,
                        },
                        source: ArgumentSource::CodeConstant {
                            index: 1,
                            first_row: 0,
                            row_count: 4,
                        },
                    }],
                    custom_sampler_flags: 0,
                    material_type: 0,
                    precompiled_index: 0,
                }],
            }],
        }],
        materials: vec![MaterialContract {
            scene_material: 0,
            family: 0,
            constants: vec![],
            textures: vec![],
            states: vec![[0, 0]],
            state_entries: vec![0; 36],
        }],
    }
}
#[test]
fn typed_stage_bindings_shadow_and_refs_are_explicit() {
    let mut m = fixture();
    m.shaders[1].resources = vec![
        ShaderResource {
            binding: 32,
            kind: ResourceKind::Texture {
                dimension: TextureDimension::D2,
                sample: TextureSample::Depth,
            },
        },
        ShaderResource {
            binding: 64,
            kind: ResourceKind::Sampler { comparison: true },
        },
    ];
    assert!(validate_bank(&m).is_ok());
    m.shaders[1].resources[0].kind = ResourceKind::Sampler { comparison: false };
    assert_eq!(validate_bank(&m), Err(AppearanceError::Unsupported));
    m.shaders[1].resources.clear();
    m.families[0].techniques[0].passes[0].vertex_shader = 1;
    assert_eq!(validate_bank(&m), Err(AppearanceError::Unsupported));
    m.families[0].techniques[0].passes[0].vertex_shader = 0;
    m.materials[0].family = 1;
    assert_eq!(validate_bank(&m), Err(AppearanceError::Reference));
}
#[test]
fn duplicate_identities_locations_limits_and_argument_union_reject() {
    let mut m = fixture();
    m.shaders[1].source_sha256 = m.shaders[0].source_sha256.clone();
    assert_eq!(validate_bank(&m), Err(AppearanceError::Identity));
    let mut m = fixture();
    let duplicate = m.shaders[0].inputs[0];
    m.shaders[0].inputs.push(duplicate);
    assert_eq!(validate_bank(&m), Err(AppearanceError::Attributes));
    let mut m = fixture();
    m.materials[0].state_entries.pop();
    assert_eq!(validate_bank(&m), Err(AppearanceError::Limit));
    let mut m = fixture();
    m.families[0].techniques[0].passes[0].args[0].destination =
        ArgumentDestination::TextureSampler {
            texture: 0,
            sampler: 1,
        };
    assert_eq!(validate_bank(&m), Err(AppearanceError::Unsupported));
}
#[test]
fn hash_bound_external_code_and_traversal_reject() {
    let root = std::env::temp_dir().join(format!("bo2-shader-bank-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    for n in ["vs.wgsl", "ps.wgsl"] {
        std::fs::write(root.join(n), b"synthetic WGSL").unwrap();
    }
    let mut m = fixture();
    let path = root.join("bank.json");
    let save =
        |m: &ShaderBankManifest| std::fs::write(&path, serde_json::to_vec(m).unwrap()).unwrap();
    save(&m);
    assert_eq!(load_shader_bank(&path).unwrap().shaders.len(), 2);
    std::fs::write(root.join("vs.wgsl"), b"changed").unwrap();
    assert!(matches!(
        load_shader_bank(&path),
        Err(AppearanceError::Identity)
    ));
    m.shaders[0].file = "../escape".into();
    save(&m);
    assert!(matches!(
        load_shader_bank(&path),
        Err(AppearanceError::Reference)
    ));
    std::fs::remove_dir_all(root).unwrap();
}
#[test]
#[ignore = "requires external BO2_SHADER_BANK; originals stay external"]
fn complete_external_shader_bank_admits_all_families() {
    let path = std::env::var("BO2_SHADER_BANK").unwrap();
    let bank = load_shader_bank(std::path::Path::new(&path)).unwrap();
    assert_eq!(bank.shaders.len(), 1113);
    assert_eq!(bank.families.len(), 56);
    assert_eq!(bank.materials.len(), 280);
    assert_eq!(bank.vertex_signatures.len(), 51);
    assert_eq!(
        bank.vertex_signatures
            .iter()
            .map(|s| s.inputs.len())
            .sum::<usize>(),
        274
    );
    for family in &bank.families {
        for technique in &family.techniques {
            for pass in &technique.passes {
                for input in &bank
                    .signature_for_shader(pass.vertex_shader)
                    .unwrap()
                    .inputs
                {
                    bo2_assets::vertex_signature::source_for_input(&pass.routing, input).unwrap();
                }
            }
        }
    }
    assert_eq!(
        bank.families
            .iter()
            .flat_map(|f| &f.techniques)
            .map(|t| t.passes.len())
            .sum::<usize>(),
        1404
    );
}

#[test]
fn semantic_metadata_is_hash_bound_complete_and_routes_by_identity() {
    use bo2_assets::{full_scene::SceneImage, vertex_signature::*};
    let root = std::env::temp_dir().join(format!("bo2-semantic-bank-test-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    for file in ["vs.wgsl", "ps.wgsl"] {
        std::fs::write(root.join(file), b"synthetic WGSL").unwrap();
    }
    let path = root.join("bank.json");
    let mut m = fixture();
    m.version = 2;
    m.families[0].techniques[0].passes[0].routing = vec![[0, 0]];
    assert_eq!(validate_bank(&m), Err(AppearanceError::Attributes));
    let signature = ShaderVertexSignature {
        source_shader_sha256: m.shaders[0].source_sha256.clone(),
        inputs: vec![SourceVertexInput {
            semantic_name: "POSITION".into(),
            semantic_index: 0,
            register: 0,
            component_mask: 15,
            used_mask: 15,
            component_type: InputComponentType::Float32,
            system_value: InputSystemValue::None,
        }],
    };
    let save = |m: &mut ShaderBankManifest, s: &[ShaderVertexSignature]| {
        let bytes = serde_json::to_vec(s).unwrap();
        std::fs::write(root.join("signature.json"), &bytes).unwrap();
        m.vertex_signatures = Some(SceneImage {
            file: "signature.json".into(),
            sha256: format!("{:x}", Sha256::digest(&bytes)),
        });
        std::fs::write(&path, serde_json::to_vec(m).unwrap()).unwrap();
    };
    save(&mut m, std::slice::from_ref(&signature));
    let bank = load_shader_bank(&path).unwrap();
    let input = &bank.signature_for_shader(0).unwrap().inputs[0];
    assert_eq!(
        source_for_input(&[[0, 0]], input),
        Ok(StreamSource::Position)
    );
    assert_eq!(
        source_for_input(&[[0, 0], [1, 0]], input),
        Err(AppearanceError::Attributes)
    );
    assert_eq!(
        decode_stream_route([9, 11]).unwrap().destination.semantic(),
        ("TEXCOORD", 6)
    );
    assert_eq!(
        decode_stream_route([11, 0]),
        Err(AppearanceError::Unsupported)
    );
    assert_eq!(
        decode_stream_route([0, 20]),
        Err(AppearanceError::Unsupported)
    );
    let mut wrong = signature.clone();
    wrong.inputs[0].register = 1;
    save(&mut m, &[wrong]);
    assert!(matches!(
        load_shader_bank(&path),
        Err(AppearanceError::Attributes)
    ));
    let mut wrong = signature.clone();
    wrong.inputs[0].component_mask = 7;
    save(&mut m, &[wrong]);
    assert!(matches!(
        load_shader_bank(&path),
        Err(AppearanceError::Attributes)
    ));
    let mut wrong = signature.clone();
    wrong.inputs[0].used_mask = 16;
    save(&mut m, &[wrong]);
    assert!(matches!(
        load_shader_bank(&path),
        Err(AppearanceError::Attributes)
    ));
    let mut wrong = signature.clone();
    wrong.inputs[0].semantic_name = "UNKNOWN".into();
    save(&mut m, &[wrong]);
    assert!(matches!(
        load_shader_bank(&path),
        Err(AppearanceError::Unsupported)
    ));
    let mut wrong = signature.clone();
    wrong.inputs[0].system_value = InputSystemValue::VertexId;
    save(&mut m, &[wrong]);
    assert!(matches!(
        load_shader_bank(&path),
        Err(AppearanceError::Unsupported)
    ));
    let mut wrong = signature.clone();
    wrong.inputs[0].component_type = InputComponentType::Sint32;
    save(&mut m, &[wrong]);
    assert!(matches!(
        load_shader_bank(&path),
        Err(AppearanceError::Unsupported)
    ));
    let mut wrong = signature.clone();
    wrong.inputs.push(wrong.inputs[0].clone());
    save(&mut m, &[wrong]);
    assert!(matches!(
        load_shader_bank(&path),
        Err(AppearanceError::Attributes)
    ));
    save(&mut m, &[]);
    assert!(matches!(
        load_shader_bank(&path),
        Err(AppearanceError::Attributes)
    ));
    save(&mut m, &[signature.clone(), signature.clone()]);
    assert!(matches!(
        load_shader_bank(&path),
        Err(AppearanceError::Reference)
    ));
    save(&mut m, std::slice::from_ref(&signature));
    std::fs::write(root.join("signature.json"), b"changed").unwrap();
    assert!(matches!(
        load_shader_bank(&path),
        Err(AppearanceError::Identity)
    ));
    save(&mut m, std::slice::from_ref(&signature));
    m.families[0].techniques[0].passes[0].routing.clear();
    std::fs::write(&path, serde_json::to_vec(&m).unwrap()).unwrap();
    assert!(matches!(
        load_shader_bank(&path),
        Err(AppearanceError::Attributes)
    ));
    m.families[0].techniques[0].passes[0].routing = vec![[0, 0], [1, 0]];
    assert_eq!(validate_bank(&m), Err(AppearanceError::Attributes));
    m.families[0].techniques[0].passes[0].routing = vec![[0, 20]];
    assert_eq!(validate_bank(&m), Err(AppearanceError::Unsupported));
    m.families[0].techniques[0].passes[0].routing.clear();
    m.version = 1;
    m.vertex_signatures = None;
    std::fs::write(&path, serde_json::to_vec(&m).unwrap()).unwrap();
    let bank = load_shader_bank(&path).unwrap();
    assert!(matches!(
        bank.signature_for_shader(0),
        Err(AppearanceError::Unsupported)
    ));
    std::fs::remove_dir_all(root).unwrap();
}

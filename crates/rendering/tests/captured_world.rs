use bevy::{mesh::VertexAttributeValues, prelude::*, render::render_resource::*, shader::Shader};
use bo2_assets::shader_bank::{ShaderStage, TextureDimension};
use bo2_assets::{captured_frame::*, full_scene::*, shader_bank::*, vertex_signature::*};
use bo2_rendering::{captured_world::*, original_material::SourceAttribute};
use std::collections::BTreeMap;
fn state() -> PipelineState {
    let attachment = Attachment {
        identity: "synthetic".into(),
        view_format: "R8G8B8A8_UNORM".into(),
        width: 16,
        height: 16,
        samples: 4,
        producer_event: 0,
    };
    PipelineState {
        viewport: [0., 0., 16., 16., 0., 0.984375],
        scissor: [-1, -2, 8192, 8192],
        scissor_enabled: true,
        cull: 2,
        front_ccw: false,
        depth_clip: true,
        depth_bias: 256,
        slope_bias: 1.,
        bias_clamp: 0.,
        alpha_to_coverage: false,
        independent_blend: false,
        conservative_raster: 0,
        forced_samples: 0,
        multisample: false,
        antialiased_lines: false,
        sample_mask: u32::MAX,
        depth_enabled: true,
        depth_write: true,
        depth_compare: 5,
        stencil_enabled: false,
        stencil_reference: 0,
        stencil_read_mask: 255,
        stencil_write_mask: 255,
        stencil: [[0; 4]; 2],
        blend_enabled: true,
        blend_color: [1, 7, 0],
        blend_alpha: [9, 1, 0],
        blend_constant: [1.; 4],
        write_mask: 7,
        depth: Some(Attachment {
            view_format: "D32S8".into(),
            ..attachment.clone()
        }),
        color: attachment,
    }
}
#[test]
fn native_state_mapping_and_viewport_scissor_do_not_use_d3d_enum_numbers() {
    let mut p = state();
    let (r, replay) = native_raster(&p).unwrap();
    assert_eq!(r.depth_compare, CompareFunction::GreaterEqual);
    assert_eq!(r.front_face, FrontFace::Cw);
    assert_eq!(
        r.color_writes,
        ColorWrites::RED | ColorWrites::GREEN | ColorWrites::BLUE
    );
    assert_eq!(
        r.blend.unwrap().color.dst_factor,
        BlendFactor::OneMinusSrcAlpha
    );
    assert_eq!(replay.depth_bias, 256);
    let d = native_dispatch(3, 99, &p).unwrap();
    assert_eq!(d.scissor, [0, 0, 16, 16]);
    assert_eq!(d.viewport[5], 0.984375);
    p.scissor = [4, 5, 6, 7];
    assert_eq!(native_dispatch(0, 0, &p).unwrap().scissor, [4, 5, 6, 7]);
    p.blend_color[2] = 1;
    assert!(native_raster(&p).is_err());
    p.blend_color[2] = 0;
    p.stencil_enabled = true;
    assert!(native_raster(&p).is_err());
    p.stencil_enabled = false;
    p.color.samples = 1;
    assert!(native_raster(&p).is_err());
    p.viewport[5] = 1.1;
    assert!(native_dispatch(0, 0, &p).is_err());
    assert!(native_compare(8).is_err());
}
#[test]
fn native_sampler_keeps_comparison_anisotropy_and_rejects_unmapped_features() {
    let mut s = SamplerBinding {
        stage: ShaderStage::Fragment,
        register: 15,
        address: [3, 3, 0],
        filter: [2, 2, 2, 1],
        comparison: 4,
        max_anisotropy: 0.,
        min_lod: -f32::MAX,
        max_lod: f32::MAX,
        mip_bias: 0.,
        border: [0.; 4],
    };
    let d = native_sampler(&s).unwrap();
    assert_eq!(d.compare, Some(bevy::image::ImageCompareFunction::Greater));
    assert_eq!(d.lod_min_clamp, 0.);
    s.filter = [4, 4, 4, 0];
    s.max_anisotropy = 4.;
    assert_eq!(native_sampler(&s).unwrap().anisotropy_clamp, 4);
    s.mip_bias = 1.;
    assert!(native_sampler(&s).is_err());
    s.mip_bias = 0.;
    s.address[0] = 2;
    assert!(native_sampler(&s).is_err());
}
fn fixture() -> (FullSceneInput, ShaderBankInput, ShaderPass, WorldDraw) {
    let mut bytes = vec![0; 64];
    bytes[..8].copy_from_slice(b"BO2DGS\0\0");
    bytes[8..12].copy_from_slice(&1u32.to_le_bytes());
    for (o, v) in [(16, 3u64), (24, 6), (32, 1), (40, 172)] {
        bytes[o..o + 8].copy_from_slice(&v.to_le_bytes());
    }
    for p in [[0.0f32, 0., 0.], [1., 0., 0.], [0., 1., 0.]] {
        for c in p {
            bytes.extend(c.to_le_bytes());
        }
    }
    for i in [0u32, 1, 2, 0, 2, 1] {
        bytes.extend(i.to_le_bytes());
    }
    bytes.extend(0u64.to_le_bytes());
    bytes.extend(6u64.to_le_bytes());
    bytes.extend(0u32.to_le_bytes());
    bytes.extend(0u32.to_le_bytes());
    for c in [0.0f32, 0., 0., 1., 1., 0.] {
        bytes.extend(c.to_le_bytes());
    }
    let scene = FullSceneInput {
        geometry: bo2_assets::geometry_snapshot::read_geometry_snapshot(
            &mut std::io::Cursor::new(&bytes),
            bytes.len() as u64,
        )
        .unwrap(),
        corner_uv: vec![
            [0., 0.],
            [0., 0.],
            [0., 0.],
            [0.5, 0.75],
            [0., 0.],
            [0., 0.],
        ],
        corner_attributes: Some(vec![
            SourceCornerAttributes {
                position_w: 0.25,
                color: [0.25; 4],
                normal_unorm: [0.5; 4],
                tangent_unorm: [0.5; 4],
                lightmap_uv: [0.; 2]
            };
            6
        ]),
        surfaces: vec![SceneSurface {
            material: 0,
            source_vertex_format: 0,
            lightmap: None,
            probe: None,
        }],
        images: vec![],
        materials: vec![],
        lightmaps: vec![],
        probes: vec![],
    };
    let hash = "0".repeat(64);
    let frag = "1".repeat(64);
    let declaration = ShaderDeclaration {
        source_sha256: hash.clone(),
        file: "synthetic".into(),
        sha256: hash.clone(),
        stage: ShaderStage::Vertex,
        entry_point: "main".into(),
        inputs: vec![VertexInput {
            location: 0,
            components: 4,
        }],
        resources: vec![],
    };
    let pass = ShaderPass {
        vertex_shader: 0,
        pixel_shader: 1,
        routing: vec![[0, 0]],
        args: vec![],
        custom_sampler_flags: 0,
        material_type: 0,
        precompiled_index: 0,
    };
    let bank = ShaderBankInput {
        geometry_sha256: hash.clone(),
        scene_image_count: 0,
        vertex_signatures: vec![ShaderVertexSignature {
            source_shader_sha256: hash.clone(),
            inputs: vec![SourceVertexInput {
                semantic_name: "POSITION".into(),
                semantic_index: 0,
                register: 0,
                component_mask: 15,
                used_mask: 15,
                component_type: InputComponentType::Float32,
                system_value: InputSystemValue::None,
            }],
        }],
        shaders: vec![
            ShaderInput {
                declaration: declaration.clone(),
                wgsl: String::new(),
            },
            ShaderInput {
                declaration: ShaderDeclaration {
                    source_sha256: frag.clone(),
                    stage: ShaderStage::Fragment,
                    inputs: vec![],
                    ..declaration
                },
                wgsl: String::new(),
            },
        ],
        families: vec![TechniqueFamily {
            source_technique_set: "synthetic".into(),
            techniques: vec![Technique {
                index: 0,
                flags: 0,
                passes: vec![pass.clone()],
            }],
        }],
        materials: vec![MaterialContract {
            scene_material: 0,
            family: 0,
            constants: vec![],
            textures: vec![],
            states: vec![],
            state_entries: vec![0],
        }],
    };
    let draw = WorldDraw {
        slices: vec![
            SurfaceSlice {
                surface: 0,
                first_corner: 3,
                corners: 3,
            },
            SurfaceSlice {
                surface: 0,
                first_corner: 0,
                corners: 3,
            },
        ],
        runtime_corners: Payload {
            file: "synthetic".into(),
            sha256: hash.clone(),
        },
        runtime_secondary: None,
        vertex_shader_sha256: hash,
        fragment_shader_sha256: frag,
        uniforms: vec![],
        textures: vec![],
        samplers: vec![],
        pipeline: state(),
        object_matrix: [0.; 16],
        object_register: 3,
        object_offset: 0,
    };
    (scene, bank, pass, draw)
}
#[test]
fn captured_slices_and_order_keep_corner_inputs_and_canonical_positions() {
    let (scene, mut bank, pass, draw) = fixture();
    let mesh = captured_mesh(&scene, &[], &bank, &pass, &draw).unwrap();
    assert!(
        matches!(mesh.attribute(SourceAttribute::Uv0.attribute()),Some(VertexAttributeValues::Float32x2(v)) if v[0]==[0.5,0.75])
    );
    assert!(
        matches!(mesh.attribute(Mesh::ATTRIBUTE_POSITION),Some(VertexAttributeValues::Float32x3(v)) if v[1]==[0.,0.,-1.])
    );
    assert!(
        matches!(mesh.attribute(SourceAttribute::Position4.attribute()),Some(VertexAttributeValues::Float32x4(v)) if v[1]==[0.,1.,0.,0.25])
    );
    let mut invalid = draw.clone();
    invalid.slices[0].corners = 4;
    assert!(captured_mesh(&scene, &[], &bank, &pass, &invalid).is_err());
    let mut shaders = Assets::<Shader>::default();
    let handle = shaders.add(Shader::from_wgsl("", "synthetic"));
    let handles = [handle.clone(), handle];
    let mut frame = CapturedFrameInput {
        manifest: FrameManifest {
            version: 1,
            capture_sha256: "0".repeat(64),
            geometry_sha256: "0".repeat(64),
            shader_bank_sha256: "0".repeat(64),
            source_corners: draw.runtime_corners.clone(),
            source_secondary: draw.runtime_corners.clone(),
            draws: vec![
                DrawReceipt {
                    event: 20,
                    dispatch: Dispatch::World {
                        draw: Box::new(draw.clone()),
                    },
                },
                DrawReceipt {
                    event: 22,
                    dispatch: Dispatch::NonWorld {
                        reason: "explicit exclusion".into(),
                    },
                },
                DrawReceipt {
                    event: 30,
                    dispatch: Dispatch::World {
                        draw: Box::new(draw),
                    },
                },
            ],
            initial_attachments: vec![],
            camera_origin: [0.; 3],
            camera_view_projection: [0.; 16],
            producer_dependencies: vec![],
            excluded_operations: vec![],
        },
        draws: vec![
            LoadedWorldDraw {
                event: 20,
                uniforms: vec![],
                textures: vec![],
            },
            LoadedWorldDraw {
                event: 30,
                uniforms: vec![],
                textures: vec![],
            },
        ],
        initial_attachments: vec![],
    };
    let images = BTreeMap::new();
    let result = prepare_captured_world(
        &frame,
        &scene,
        &[],
        &bank,
        CapturedHandles {
            shaders: &handles,
            images: &images,
        },
        &WgpuLimits::default(),
    )
    .unwrap();
    assert_eq!(
        result
            .iter()
            .map(|d| (d.dispatch.ordinal, d.dispatch.event))
            .collect::<Vec<_>>(),
        vec![(0, 20), (1, 30)]
    );
    assert!(
        result
            .iter()
            .all(|d| d.material.descriptor().replay.is_some())
    );
    let extra = TextureBinding {
        stage: ShaderStage::Vertex,
        register: 0,
        bytes: Payload {
            file: "synthetic-unread".into(),
            sha256: "2".repeat(64),
        },
        width: 1,
        height: 1,
        mips: 1,
        cube: false,
        view_format: "R8G8B8A8_UNORM".into(),
        first_mip: 0,
        view_mips: 1,
        first_slice: 0,
        slices: 1,
        producer_event: 0,
    };
    if let Dispatch::World { draw } = &mut frame.manifest.draws[0].dispatch {
        draw.textures.push(extra.clone());
    }
    frame.draws[0]
        .textures
        .push(std::sync::Arc::from(vec![0u8]));
    assert!(
        prepare_captured_world(
            &frame,
            &scene,
            &[],
            &bank,
            CapturedHandles {
                shaders: &handles,
                images: &images
            },
            &WgpuLimits::default()
        )
        .is_ok()
    );
    bank.shaders[1].declaration.resources.push(ShaderResource {
        binding: 32,
        kind: ResourceKind::Texture {
            dimension: TextureDimension::D2,
            sample: TextureSample::Float,
        },
    });
    if let Dispatch::World { draw } = &mut frame.manifest.draws[0].dispatch {
        draw.textures.push(TextureBinding {
            stage: ShaderStage::Fragment,
            ..extra
        });
    }
    frame.draws[0]
        .textures
        .push(std::sync::Arc::from(vec![0u8]));
    assert!(matches!(
        prepare_captured_world(
            &frame,
            &scene,
            &[],
            &bank,
            CapturedHandles {
                shaders: &handles,
                images: &images
            },
            &WgpuLimits::default()
        ),
        Err(CapturedWorldError::MissingImage)
    ));
    bank.shaders[1].declaration.resources.clear();
    frame.draws.remove(1);
    assert!(matches!(
        prepare_captured_world(
            &frame,
            &scene,
            &[],
            &bank,
            CapturedHandles {
                shaders: &handles,
                images: &images
            },
            &WgpuLimits::default()
        ),
        Err(CapturedWorldError::MissingDraw)
    ));
}

#[test]
#[ignore = "requires admitted external captured frame/scene/bank/secondary; CPU descriptors only"]
fn external_all_694_captured_world_draws_prepare() {
    fn path(key: &str) -> std::path::PathBuf {
        std::env::var_os(key).expect(key).into()
    }
    let bank = load_shader_bank(&path("BO2_SHADER_BANK")).unwrap();
    let scene = load_full_scene(&path("BO2_FULL_SCENE")).unwrap();
    let secondary = bo2_assets::secondary_stream::load_secondary_corners(
        &path("BO2_SECONDARY_CORNERS"),
        &scene.geometry,
        &bank.geometry_sha256,
    )
    .unwrap();
    let bank_hash = std::env::var("BO2_SHADER_BANK_SHA256").unwrap();
    let frame =
        load_captured_frame(&path("BO2_CAPTURED_FRAME"), &scene, &bank, &bank_hash).unwrap();
    let mut shaders = Assets::<Shader>::default();
    let shaders: Vec<_> = bank
        .shaders
        .iter()
        .map(|s| {
            shaders.add(Shader::from_wgsl(
                s.wgsl.clone(),
                s.declaration.file.clone(),
            ))
        })
        .collect();
    // Strong synthetic image handles exercise complete binding assembly. Actual
    // typed image preparation/upload/readiness is a separate GPU integration gate.
    let mut image_assets = Assets::<Image>::default();
    let mut images = BTreeMap::new();
    for receipt in &frame.manifest.draws {
        if let Dispatch::World { draw } = &receipt.dispatch {
            for texture in &draw.textures {
                images
                    .entry((texture.bytes.sha256.clone(), texture.view_format.clone()))
                    .or_insert_with(|| image_assets.add(Image::default()));
            }
        }
    }
    let draws = prepare_captured_world(
        &frame,
        &scene,
        &secondary,
        &bank,
        CapturedHandles {
            shaders: &shaders,
            images: &images,
        },
        &WgpuLimits::default(),
    )
    .unwrap();
    assert_eq!(draws.len(), 694);
    assert!(
        draws
            .windows(2)
            .all(|d| d[0].dispatch.ordinal < d[1].dispatch.ordinal
                && d[0].dispatch.event < d[1].dispatch.event)
    );
    println!(
        "captured CPU assembly: {} draws/{} exact texture identities, all native states and slices admitted",
        draws.len(),
        images.len()
    );
}

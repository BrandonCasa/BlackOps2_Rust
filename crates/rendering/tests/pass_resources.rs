use bevy::{prelude::*, render::render_resource::*, shader::Shader};
use bo2_assets::shader_bank::ShaderStage;
use bo2_assets::shader_bank::*;
use bo2_rendering::{
    original_material::{
        BindingKind, BindingSpec, OriginalMaterialDescriptor, OriginalRasterState,
        OriginalResource, SourceAttribute, VertexInput as RoutedInput,
    },
    pass_resources::*,
};
fn descriptor(bindings: Vec<BindingSpec>) -> OriginalMaterialDescriptor {
    let mut shaders = Assets::<Shader>::default();
    let shader = shaders.add(Shader::from_wgsl("", "synthetic"));
    OriginalMaterialDescriptor {
        vertex_shader: shader.clone(),
        fragment_shader: shader,
        vertex_entry: "main".into(),
        fragment_entry: "main".into(),
        bindings,
        attributes: vec![RoutedInput {
            attribute: SourceAttribute::Position4,
            location: 0,
        }],
        raster: OriginalRasterState {
            front_face: FrontFace::Cw,
            cull_mode: Some(Face::Back),
            depth_write: true,
            depth_compare: CompareFunction::GreaterEqual,
            blend: None,
            color_writes: ColorWrites::ALL,
        },
    }
}
fn uniform(binding: u32, size: u64) -> BindingSpec {
    BindingSpec {
        binding,
        visibility: if binding < 16 {
            ShaderStages::VERTEX
        } else {
            ShaderStages::FRAGMENT
        },
        kind: BindingKind::Uniform { minimum_size: size },
    }
}
fn pass(args: Vec<PassArgument>) -> ShaderPass {
    ShaderPass {
        vertex_shader: 0,
        pixel_shader: 1,
        routing: vec![],
        args,
        custom_sampler_flags: 0,
        material_type: 0,
        precompiled_index: 0,
    }
}
fn material() -> MaterialContract {
    MaterialContract {
        scene_material: 0,
        family: 0,
        constants: vec![MaterialConstant {
            name_hash: 7,
            value: [1.0, 2.0, 3.0, 4.0],
        }],
        textures: vec![],
        states: vec![],
        state_entries: vec![],
    }
}
fn argument(stage: ShaderStage, offset: u16, size: u16, source: ArgumentSource) -> PassArgument {
    PassArgument {
        stage,
        destination: ArgumentDestination::Uniform {
            buffer: 0,
            offset,
            size,
        },
        source,
    }
}
fn read(resources: &[OriginalResource], binding: u32, offset: usize) -> f32 {
    let OriginalResource::Uniform { bytes, .. } =
        resources.iter().find(|r| r.binding() == binding).unwrap()
    else {
        panic!("uniform")
    };
    f32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap())
}
#[test]
fn partial_material_writes_and_code_matrix_rows_keep_byte_offsets_and_stage_buffers() {
    let descriptor = descriptor(vec![uniform(0, 80), uniform(16, 16)]);
    let mut context = PassResourceContext::default();
    context.initial_uniforms.insert(0, vec![0x55; 80]);
    context.initial_uniforms.insert(16, vec![0x66; 16]);
    context.code_constants.insert(
        213,
        vec![[10.0; 4], [20.0; 4], [30.0; 4], [40.0; 4], [50.0; 4]],
    );
    let pass = pass(vec![
        argument(
            ShaderStage::Vertex,
            4,
            8,
            ArgumentSource::MaterialConstant { name_hash: 7 },
        ),
        argument(
            ShaderStage::Vertex,
            16,
            64,
            ArgumentSource::CodeConstant {
                index: 213,
                first_row: 1,
                row_count: 4,
            },
        ),
        argument(
            ShaderStage::Fragment,
            0,
            4,
            ArgumentSource::Literal {
                value: [9.0, 0.0, 0.0, 0.0],
            },
        ),
    ]);
    let resources = assemble_pass_resources(&descriptor, &pass, &material(), &context).unwrap();
    assert_eq!(read(&resources, 0, 4), 1.0);
    assert_eq!(read(&resources, 0, 8), 2.0);
    assert_eq!(read(&resources, 0, 16), 20.0);
    assert_eq!(read(&resources, 0, 64), 50.0);
    assert_eq!(read(&resources, 16, 0), 9.0);
}
#[test]
fn missing_codes_hashes_and_unwritten_globals_are_explicit_errors() {
    let d = descriptor(vec![uniform(0, 16)]);
    let mut context = PassResourceContext::default();
    let mut p = pass(vec![argument(
        ShaderStage::Vertex,
        0,
        16,
        ArgumentSource::CodeConstant {
            index: 229,
            first_row: 0,
            row_count: 1,
        },
    )]);
    assert!(matches!(
        assemble_pass_resources(&d, &p, &material(), &context),
        Err(PassResourceError::MissingCodeConstant(229))
    ));
    p.args[0].source = ArgumentSource::MaterialConstant { name_hash: 999 };
    assert!(matches!(
        assemble_pass_resources(&d, &p, &material(), &context),
        Err(PassResourceError::MissingMaterialConstant(999))
    ));
    p.args[0].source = ArgumentSource::MaterialConstant { name_hash: 7 };
    p.args[0].destination = ArgumentDestination::Uniform {
        buffer: 0,
        offset: 0,
        size: 4,
    };
    assert!(matches!(
        assemble_pass_resources(&d, &p, &material(), &context),
        Err(PassResourceError::UninitializedUniform {
            binding: 0,
            offset: 4
        })
    ));
    context.initial_uniforms.insert(0, vec![0; 16]);
    assert!(assemble_pass_resources(&d, &p, &material(), &context).is_ok());
    p.args.push(argument(
        ShaderStage::Vertex,
        0,
        4,
        ArgumentSource::Literal { value: [5.0; 4] },
    ));
    assert!(matches!(
        assemble_pass_resources(&d, &p, &material(), &context),
        Err(PassResourceError::ConflictingUniform {
            binding: 0,
            offset: _
        })
    ));
}
fn image_binding(binding: u32) -> BindingSpec {
    BindingSpec {
        binding,
        visibility: ShaderStages::FRAGMENT,
        kind: BindingKind::Texture {
            dimension: TextureViewDimension::D2,
            sample_type: TextureSampleType::Float { filterable: true },
        },
    }
}
fn sampler_binding(binding: u32) -> BindingSpec {
    BindingSpec {
        binding,
        visibility: ShaderStages::FRAGMENT,
        kind: BindingKind::Sampler {
            kind: SamplerBindingType::Filtering,
        },
    }
}
#[test]
fn material_hash_images_builtin_errors_and_independent_sampler_registers() {
    let d = descriptor(vec![image_binding(32), sampler_binding(65)]);
    let p = pass(vec![PassArgument {
        stage: ShaderStage::Fragment,
        destination: ArgumentDestination::TextureSampler {
            texture: 0,
            sampler: 1,
        },
        source: ArgumentSource::MaterialTexture { name_hash: 42 },
    }]);
    let mut m = material();
    m.textures.push(MaterialTextureContract {
        name_hash: 42,
        semantic: 2,
        sampler: 20,
        image: MaterialImage::SceneImage { index: 3 },
    });
    let mut c = PassResourceContext::default();
    assert!(matches!(
        assemble_pass_resources(&d, &p, &m, &c),
        Err(PassResourceError::MissingSceneImage(3))
    ));
    let mut images = Assets::<Image>::default();
    let image = images.add(Image::default());
    c.scene_images.insert(3, image.clone());
    let resources = assemble_pass_resources(&d, &p, &m, &c).unwrap();
    assert!(matches!(&resources[0],OriginalResource::Image{binding:32,image:i} if *i==image));
    assert!(matches!(
        &resources[1],
        OriginalResource::Sampler { binding: 65, .. }
    ));
    m.textures[0].image = MaterialImage::UnresolvedBuiltin {
        source_sha256: "synthetic-builtin".into(),
    };
    assert!(matches!(
        assemble_pass_resources(&d, &p, &m, &c),
        Err(PassResourceError::MissingBuiltin(_))
    ));
    c.builtin_images.insert("synthetic-builtin".into(), image);
    assert!(assemble_pass_resources(&d, &p, &m, &c).is_ok());
}
#[test]
fn shared_sampler_conflicts_and_missing_code_textures_are_rejected() {
    let d = descriptor(vec![
        image_binding(32),
        image_binding(33),
        sampler_binding(64),
    ]);
    let p = pass(
        (0..2)
            .map(|t| PassArgument {
                stage: ShaderStage::Fragment,
                destination: ArgumentDestination::TextureSampler {
                    texture: t,
                    sampler: 0,
                },
                source: ArgumentSource::CodeTexture {
                    index: u32::from(t),
                },
            })
            .collect(),
    );
    let mut c = PassResourceContext::default();
    assert!(matches!(
        assemble_pass_resources(&d, &p, &material(), &c),
        Err(PassResourceError::MissingCodeTexture(0))
    ));
    let mut images = Assets::<Image>::default();
    let image = images.add(Image::default());
    c.code_textures.insert(
        0,
        RuntimeTexture {
            image: image.clone(),
            sampler: default(),
        },
    );
    let sampler = bevy::image::ImageSamplerDescriptor {
        lod_max_clamp: 1.0,
        ..default()
    };
    c.code_textures.insert(1, RuntimeTexture { image, sampler });
    assert!(matches!(
        assemble_pass_resources(&d, &p, &material(), &c),
        Err(PassResourceError::ConflictingBinding(64))
    ));
}
#[test]
fn native_custom_lighting_slots_are_explicit_and_unknown_flags_fail() {
    let mut p = pass(vec![]);
    p.custom_sampler_flags = 3;
    p.material_type = 5;
    let mut c = PassResourceContext::default();
    let d = descriptor(vec![]);
    assert!(matches!(
        assemble_pass_resources(&d, &p, &material(), &c),
        Err(PassResourceError::UnsupportedPassFlags { .. })
    ));
    assert!(QualifiedImplicitBindings::world_lighting(&p, None, None).is_err());
    p.custom_sampler_flags = 4;
    assert!(QualifiedImplicitBindings::world_lighting(&p, None, None).is_err());
    p.custom_sampler_flags = 0;
    c.qualified_implicit = Some(QualifiedImplicitBindings::world_lighting(&p, None, None).unwrap());
    assert!(assemble_pass_resources(&d, &p, &material(), &c).is_ok());
}

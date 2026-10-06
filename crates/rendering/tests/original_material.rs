use bevy::{prelude::*, render::render_resource::*, shader::Shader};
use bo2_rendering::original_material::*;

fn descriptor() -> OriginalMaterialDescriptor {
    let mut shaders = Assets::<Shader>::default();
    let shader = shaders.add(Shader::from_wgsl("", "synthetic"));
    OriginalMaterialDescriptor {
        replay: None,
        vertex_shader: shader.clone(),
        fragment_shader: shader,
        vertex_entry: "main".into(),
        fragment_entry: "main".into(),
        bindings: vec![BindingSpec {
            binding: 0,
            visibility: ShaderStages::VERTEX,
            kind: BindingKind::Uniform { minimum_size: 32 },
        }],
        attributes: vec![VertexInput {
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

#[test]
fn native_float32_filtering_requires_enabled_device_feature() {
    let mut images = Assets::<Image>::default();
    let mut image = Image::default();
    image.texture_descriptor.format = TextureFormat::R32Float;
    image.data = Some(0.25f32.to_le_bytes().to_vec());
    let image = images.add(image);
    for required in [true, false] {
        let mut d = descriptor();
        d.bindings = vec![BindingSpec {
            binding: 32,
            visibility: ShaderStages::FRAGMENT,
            kind: BindingKind::Texture {
                dimension: TextureViewDimension::D2,
                sample_type: TextureSampleType::Float {
                    filterable: required,
                },
            },
        }];
        let material = OriginalMaterial::new(
            d,
            vec![OriginalResource::Image {
                binding: 32,
                image: image.clone(),
            }],
            &WgpuLimits::default(),
        )
        .unwrap();
        assert_eq!(
            material.validate_images(&images),
            if required {
                Err(OriginalMaterialError::WrongResource)
            } else {
                Ok(())
            }
        );
        assert_eq!(
            material.validate_images_with_features(&images, WgpuFeatures::FLOAT32_FILTERABLE),
            Ok(())
        );
    }
}
fn uniform(bytes: usize) -> OriginalResource {
    OriginalResource::Uniform {
        binding: 0,
        bytes: vec![0; bytes],
    }
}
#[test]
fn exact_resources_and_uniform_extent_are_required() {
    let limits = WgpuLimits::default();
    assert!(matches!(
        OriginalMaterial::new(descriptor(), vec![], &limits),
        Err(OriginalMaterialError::MissingBinding)
    ));
    assert!(matches!(
        OriginalMaterial::new(descriptor(), vec![uniform(16)], &limits),
        Err(OriginalMaterialError::InvalidUniform)
    ));
    assert!(matches!(
        OriginalMaterial::new(descriptor(), vec![uniform(33)], &limits),
        Err(OriginalMaterialError::InvalidUniform)
    ));
    let mut material = OriginalMaterial::new(descriptor(), vec![uniform(32)], &limits).unwrap();
    assert!(material.update_uniform(1, vec![0; 32]).is_err());
    assert!(material.update_uniform(0, vec![0; 16]).is_err());
    material.update_uniform(0, vec![1; 32]).unwrap();
    assert_eq!(
        material.descriptor().raster.depth_compare,
        CompareFunction::GreaterEqual
    );
}
#[test]
fn duplicate_bindings_attributes_and_device_limits_fail_before_gpu() {
    let mut d = descriptor();
    d.bindings.push(d.bindings[0].clone());
    assert_eq!(d.validate(), Err(OriginalMaterialError::DuplicateBinding));
    let mut d = descriptor();
    d.attributes.push(d.attributes[0].clone());
    assert_eq!(d.validate(), Err(OriginalMaterialError::InvalidAttributes));
    let limits = WgpuLimits {
        max_uniform_buffers_per_shader_stage: 0,
        ..default()
    };
    assert_eq!(
        descriptor().validate_limits(&limits),
        Err(OriginalMaterialError::DeviceLimit)
    );
}
#[test]
fn reflection_merges_only_identical_shared_resource_types() {
    use bo2_assets::shader_bank::{
        ResourceKind, ShaderDeclaration, ShaderResource, ShaderStage, VertexInput as Input,
    };
    let declaration = |stage| ShaderDeclaration {
        source_sha256: "0".repeat(64),
        file: "synthetic.wgsl".into(),
        sha256: "1".repeat(64),
        stage,
        entry_point: "main".into(),
        inputs: if stage == ShaderStage::Vertex {
            vec![Input {
                location: 0,
                components: 4,
            }]
        } else {
            vec![]
        },
        resources: vec![ShaderResource {
            binding: 0,
            kind: ResourceKind::Uniform { size: 32 },
        }],
    };
    let d = descriptor();
    let vs = declaration(ShaderStage::Vertex);
    let mut ps = declaration(ShaderStage::Fragment);
    let merged = translated_descriptor(
        &vs,
        &ps,
        d.vertex_shader.clone(),
        d.fragment_shader.clone(),
        d.attributes.clone(),
        d.raster.clone(),
    )
    .unwrap();
    assert_eq!(merged.bindings.len(), 1);
    assert_eq!(merged.bindings[0].visibility, ShaderStages::VERTEX_FRAGMENT);
    ps.resources[0].kind = ResourceKind::Uniform { size: 64 };
    assert!(matches!(
        translated_descriptor(
            &vs,
            &ps,
            d.vertex_shader,
            d.fragment_shader,
            d.attributes,
            d.raster
        ),
        Err(OriginalMaterialError::WrongResource)
    ));
}

#[test]
fn color_images_cannot_satisfy_depth_or_cube_bindings() {
    let mut images = Assets::<Image>::default();
    let image = images.add(Image::default());
    for (dimension, sample_type) in [
        (TextureViewDimension::D2, TextureSampleType::Depth),
        (
            TextureViewDimension::Cube,
            TextureSampleType::Float { filterable: true },
        ),
    ] {
        let mut d = descriptor();
        d.bindings = vec![BindingSpec {
            binding: 32,
            visibility: ShaderStages::FRAGMENT,
            kind: BindingKind::Texture {
                dimension,
                sample_type,
            },
        }];
        let material = OriginalMaterial::new(
            d,
            vec![OriginalResource::Image {
                binding: 32,
                image: image.clone(),
            }],
            &WgpuLimits::default(),
        )
        .unwrap();
        assert_eq!(
            material.validate_images(&images),
            Err(OriginalMaterialError::WrongResource)
        );
    }
}

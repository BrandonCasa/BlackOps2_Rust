use bo2_assets::{appearance::AppearanceError, builtin_images::*};

#[test]
fn initializer_measurement_is_exact_and_source_identity_is_required() {
    let d = definition_for_source(IDENTITY_NORMAL_SOURCE_SHA256).unwrap();
    assert_eq!(
        (d.width, d.height, d.mips, d.srgb, d.rgba8),
        (1, 1, 1, false, [128, 128, 255, 128])
    );
    for unknown in [
        "",
        "identity-normal",
        "df04ff1564886176b0257baa1e44483e9add8571251fd8f12dea9e3e97014de1",
    ] {
        assert_eq!(
            definition_for_source(unknown),
            Err(AppearanceError::Unsupported)
        );
    }
    let mut changed = IDENTITY_NORMAL_SOURCE_SHA256.to_owned();
    changed.replace_range(0..1, "4");
    assert_eq!(
        definition_for_source(&changed),
        Err(AppearanceError::Unsupported)
    );
    let mut dds = vec![0u8; 128];
    dds[..4].copy_from_slice(b"DDS ");
    for (offset, value) in [
        (4, 124u32),
        (12, 1),
        (16, 1),
        (28, 1),
        (76, 32),
        (80, 0x41),
        (88, 32),
        (92, 0xff),
        (96, 0xff00),
        (100, 0xff0000),
        (104, 0xff000000),
    ] {
        dds[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    }
    dds.extend([128, 128, 255, 128]);
    validate_external_builtin(IDENTITY_NORMAL_SOURCE_SHA256, &dds).unwrap();
    dds[131] = 255;
    assert_eq!(
        validate_external_builtin(IDENTITY_NORMAL_SOURCE_SHA256, &dds),
        Err(AppearanceError::Identity)
    );
    dds[131] = 128;
    dds[28..32].copy_from_slice(&2u32.to_le_bytes());
    assert_eq!(
        validate_external_builtin(IDENTITY_NORMAL_SOURCE_SHA256, &dds),
        Err(AppearanceError::Malformed)
    );
}

#[test]
#[ignore = "requires licensed external scene and shader bank"]
fn measured_builtin_resolves_all_identity_references_and_preserves_source_samplers() {
    use bo2_assets::{
        full_scene::{DdsFormat, load_full_scene},
        shader_bank::{MaterialImage, load_shader_bank},
    };
    let scene = load_full_scene(std::path::Path::new(
        &std::env::var("BO2_TEXTURE_SCENE").unwrap(),
    ))
    .unwrap();
    let bank = load_shader_bank(std::path::Path::new(
        &std::env::var("BO2_SHADER_BANK").unwrap(),
    ))
    .unwrap();
    assert!((261..=262).contains(&scene.images.len()));
    assert_eq!(bank.scene_image_count, scene.images.len());
    let image = &scene.images[260];
    assert_eq!(image.metadata.format, DdsFormat::Rgba8);
    assert_eq!(
        (
            image.metadata.width,
            image.metadata.height,
            image.metadata.mips,
            image.metadata.cube
        ),
        (1, 1, 1, false)
    );
    assert_eq!(&image.dds[128..], [128, 128, 255, 128]);
    validate_external_builtin(IDENTITY_NORMAL_SOURCE_SHA256, &image.dds).unwrap();
    let mut changed = image.dds.clone();
    changed[128] = 127;
    assert_eq!(
        validate_external_builtin(IDENTITY_NORMAL_SOURCE_SHA256, &changed),
        Err(AppearanceError::Identity)
    );
    let mut normal_bindings = 0;
    let mut shadow_bindings = 0;
    for material in &bank.materials {
        for texture in &material.textures {
            match &texture.image {
                MaterialImage::SceneImage { index: 260 } => {
                    normal_bindings += 1;
                    let s = scene.materials[material.scene_material]
                        .bindings
                        .iter()
                        .find(|b| b.name_hash == texture.name_hash)
                        .unwrap();
                    assert_eq!(
                        (s.image, s.semantic, s.sampler),
                        (260, texture.semantic, texture.sampler)
                    );
                }
                MaterialImage::SceneImage { index: 261 } => {
                    shadow_bindings += 1;
                }
                MaterialImage::UnresolvedBuiltin { source_sha256 } => {
                    assert_ne!(source_sha256, IDENTITY_NORMAL_SOURCE_SHA256);
                    shadow_bindings += 1;
                }
                _ => {}
            }
        }
    }
    assert_eq!(normal_bindings, 41);
    assert_eq!(shadow_bindings, 1);
}

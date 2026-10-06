use bo2_assets::{appearance::AppearanceError, builtin_images::*, full_scene::SceneImage};
#[test]
fn shared_source_contract_and_missing_unknown_identity_errors_are_explicit() {
    let d = external_definition_for_source(SHADOW_MATERIAL_SOURCE_SHA256).unwrap();
    assert_eq!(
        (d.width, d.height, d.mips, d.format),
        (256, 256, 9, bo2_assets::full_scene::DdsFormat::Bc1)
    );
    assert_eq!(
        external_definition_for_source(IDENTITY_NORMAL_SOURCE_SHA256),
        Err(AppearanceError::Unsupported)
    );
    let root =
        std::env::temp_dir().join(format!("bo2-shadow-material-test-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    let mut image = SceneImage {
        file: "missing".into(),
        sha256: "0".repeat(64),
    };
    assert!(matches!(
        load_external_builtin(&root, SHADOW_MATERIAL_SOURCE_SHA256, &image),
        Err(AppearanceError::Identity)
    ));
    image.sha256 = d.dds_sha256.into();
    assert!(matches!(
        load_external_builtin(&root, SHADOW_MATERIAL_SOURCE_SHA256, &image),
        Err(AppearanceError::Io(std::io::ErrorKind::NotFound))
    ));
    std::fs::write(root.join("missing"), [0; 128]).unwrap();
    assert!(matches!(
        load_external_builtin(&root, SHADOW_MATERIAL_SOURCE_SHA256, &image),
        Err(AppearanceError::Identity)
    ));
    std::fs::remove_dir_all(root).unwrap();
}
#[test]
#[ignore = "requires licensed shared-zone image export"]
fn actual_shared_dependency_resolves_exact_material_binding() {
    use bo2_assets::{
        full_scene::load_full_scene,
        shader_bank::{ArgumentDestination, ArgumentSource, MaterialImage, load_shader_bank},
    };
    let scene_path = std::path::PathBuf::from(std::env::var("BO2_TEXTURE_SCENE").unwrap());
    let scene = load_full_scene(&scene_path).unwrap();
    let bank = load_shader_bank(std::path::Path::new(
        &std::env::var("BO2_SHADER_BANK").unwrap(),
    ))
    .unwrap();
    assert_eq!((scene.images.len(), bank.scene_image_count), (262, 262));
    let image = SceneImage {
        file: "shared-shadow.dds".into(),
        sha256: external_definition_for_source(SHADOW_MATERIAL_SOURCE_SHA256)
            .unwrap()
            .dds_sha256
            .into(),
    };
    let loaded = load_external_builtin(
        scene_path.parent().unwrap(),
        SHADOW_MATERIAL_SOURCE_SHA256,
        &image,
    )
    .unwrap();
    assert_eq!(loaded.dds, scene.images[261].dds);
    let material = &bank.materials[264];
    let texture = material
        .textures
        .iter()
        .find(|t| t.name_hash == 2695565377)
        .unwrap();
    assert!(matches!(
        texture.image,
        MaterialImage::SceneImage { index: 261 }
    ));
    assert_eq!((texture.semantic, texture.sampler), (2, 20));
    let binding = scene.materials[264]
        .bindings
        .iter()
        .find(|t| t.name_hash == texture.name_hash)
        .unwrap();
    assert_eq!(
        (binding.image, binding.sampler, binding.semantic),
        (261, 20, 2)
    );
    let family = &bank.families[material.family];
    for n in [2, 3] {
        let t = family.techniques.iter().find(|t| t.index == n).unwrap();
        let a = t.passes[0]
            .args
            .iter()
            .find(|a| {
                matches!(
                    a.source,
                    ArgumentSource::MaterialTexture {
                        name_hash: 2695565377
                    }
                )
            })
            .unwrap();
        assert!(matches!(
            a.destination,
            ArgumentDestination::TextureSampler {
                texture: 0,
                sampler: 0
            }
        ));
    }
    assert!(
        bank.materials
            .iter()
            .flat_map(|m| &m.textures)
            .all(|t| !matches!(t.image, MaterialImage::UnresolvedBuiltin { .. }))
    );
}

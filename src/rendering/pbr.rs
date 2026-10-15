use bevy::{
    asset::{AssetId, LoadState},
    gltf::GltfMaterialExtras,
    image::{
        ImageAddressMode, ImageFilterMode, ImageLoaderSettings, ImageSampler,
        ImageSamplerDescriptor,
    },
    prelude::*,
};
use serde::Deserialize;
use std::collections::HashMap;

const SHADOW_ONLY_LAYER: usize = 31;

pub(super) fn install(app: &mut App) {
    super::hybrid::install(app);
    app.add_systems(PreUpdate, parse_metadata);
}

#[derive(Component)]
pub(super) struct SurfaceMetadata(pub serde_json::Value);

pub(super) fn parse_metadata(
    mut commands: Commands,
    changed: Query<(Entity, &GltfMaterialExtras), Changed<GltfMaterialExtras>>,
    mut removed: RemovedComponents<GltfMaterialExtras>,
    mut failure: ResMut<crate::assets::LoadFailure>,
    mut next: ResMut<NextState<crate::GameState>>,
) {
    for entity in removed.read() {
        if changed.get(entity).is_err() {
            if let Ok(mut entity) = commands.get_entity(entity) {
                entity.remove::<SurfaceMetadata>();
            }
        }
    }
    for (entity, extras) in &changed {
        match serde_json::from_str(&extras.value) {
            Ok(value) => {
                commands.entity(entity).insert(SurfaceMetadata(value));
            }
            Err(error) => {
                failure.0 = format!("Invalid material metadata: {error}");
                next.set(crate::GameState::LoadError);
                return;
            }
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
pub(super) struct PreparedTexture {
    pub uri: String,
    pub role: String,
    #[serde(default)]
    pub layer: u64,
    pub color_space: String,
    pub sampler: u8,
}

#[derive(Component)]
pub(crate) struct PbrReady;

pub(super) fn textures_ready<'a>(
    server: &AssetServer,
    images: impl IntoIterator<Item = &'a Handle<Image>>,
) -> Result<bool, String> {
    let mut ready = true;
    for image in images {
        if let Some(LoadState::Failed(error)) = server.get_load_state(image.id()) {
            return Err(format!("Material texture {:?}: {error}", image.path()));
        }
        ready &= server.is_loaded_with_dependencies(image.id());
    }
    Ok(ready)
}

pub(super) fn apply_channels(
    material: &mut StandardMaterial,
    channel: impl Fn(&str) -> Option<Handle<Image>>,
) {
    material.base_color_texture = channel("color");
    material.normal_map_texture = channel("normal");
    material.metallic_roughness_texture = channel("response");
    material.occlusion_texture = channel("occlusion").or_else(|| channel("response"));
    material.emissive_texture = channel("emissive");
    material.specular_tint_texture = channel("reflectance").or_else(|| channel("specular_gloss"));
}

pub(super) fn textures(value: &serde_json::Value) -> Result<Vec<PreparedTexture>, String> {
    let textures: Vec<PreparedTexture> = serde_json::from_value(value.clone())
        .map_err(|error| format!("Invalid material channels: {error}"))?;
    let mut roles = std::collections::HashSet::new();
    for texture in &textures {
        crate::origins::validate_asset_path(&texture.uri).map_err(|error| error.to_string())?;
        if !matches!(texture.color_space.as_str(), "srgb" | "linear")
            || texture.layer != 0
            || !matches!(
                texture.role.as_str(),
                "color"
                    | "normal"
                    | "response"
                    | "occlusion"
                    | "emissive"
                    | "reflectance"
                    | "specular_gloss"
            )
            || !roles.insert((texture.role.clone(), texture.layer))
        {
            return Err(format!(
                "Invalid or duplicate material channel {}: {}",
                texture.role, texture.uri
            ));
        }
    }
    Ok(textures)
}

pub(crate) fn validate(surface: &serde_json::Value) -> Result<(), String> {
    if surface["version"] != 3 || surface["identity"].as_str().is_none_or(str::is_empty) {
        return Err("Unsupported prepared material version or missing identity".into());
    }
    if let Some(reason) = surface["pending_runtime"].as_str() {
        return Err(format!("Unresolved material runtime binding: {reason}"));
    }
    textures(&surface["channels"])?;
    super::hybrid::validate(surface)?;
    if surface["source_script_layers"]["consumer"] == "native_live_layers" {
        return Err("Native animated material has no static ray average".into());
    }
    if !surface["source_shadow_only"].is_null() {
        let shadow = &surface["source_shadow_only"];
        if surface["native"]["technique_set"] != "wpc_shadowcaster_wj6w5j60"
            || shadow["visible_lit_pass"] != false
            || shadow["cast_shadows"] != true
            || shadow["depth_prepass"]["color_write"] != false
            || shadow["shadowmap"]["color_write"] != false
            || surface["presentation"]["visible"] != false
            || surface["presentation"]["alpha"] != "opaque"
        {
            return Err("Invalid native shadow-only material binding".into());
        }
    }
    uv_transform(&surface["standard"])?;
    let state = &surface["presentation"];
    if !matches!(
        state["alpha"].as_str(),
        Some("opaque" | "mask" | "blend" | "premultiplied" | "add" | "multiply")
    ) || !matches!(state["cull"].as_str(), Some("none" | "front" | "back"))
        || !matches!(
            state["fog_mode"].as_str(),
            None | Some("color" | "attenuation")
        )
    {
        return Err("Unsupported prepared material presentation state".into());
    }
    if state["alpha"] == "mask"
        && state["cutoff"]
            .as_f64()
            .is_none_or(|value| !(0. ..=1.).contains(&value))
    {
        return Err("Invalid material alpha cutoff".into());
    }
    Ok(())
}

pub(super) fn uv_transform(factors: &serde_json::Value) -> Result<bevy::math::Affine2, String> {
    let Some(value) = factors.get("uv_transform") else {
        return Ok(bevy::math::Affine2::IDENTITY);
    };
    let values: [f32; 6] = serde_json::from_value(value.clone())
        .map_err(|error| format!("Invalid prepared UV transform: {error}"))?;
    let transform = bevy::math::Affine2::from_cols_array(&values);
    let determinant = transform.matrix2.determinant();
    if values.iter().any(|value| !value.is_finite())
        || !determinant.is_finite()
        || determinant == 0.
    {
        return Err("Prepared UV transform is non-finite or singular".into());
    }
    Ok(transform)
}

pub(super) fn refresh(
    mut commands: Commands,
    surfaces: Query<Entity, (With<PbrReady>, Changed<SurfaceMetadata>)>,
) {
    for entity in &surfaces {
        let mut entity = commands.entity(entity);
        super::opaque::reset_binding(&mut entity);
    }
}

pub(super) fn prepare_image_dimensions(
    mut events: MessageReader<AssetEvent<Image>>,
    server: Res<AssetServer>,
    mut images: ResMut<Assets<Image>>,
) {
    use bevy::render::render_resource::TextureDimension;
    for event in events.read() {
        let (AssetEvent::Added { id } | AssetEvent::Modified { id }) = event else {
            continue;
        };
        if !server.get_path(*id).is_some_and(|path| {
            path.path()
                .extension()
                .is_some_and(|extension| extension == "ktx2")
        }) {
            continue;
        }
        if let Some(image) = images.get_mut_untracked(*id) {
            if image.texture_descriptor.dimension == TextureDimension::D1 && image.height() == 1 {
                image.texture_descriptor.dimension = TextureDimension::D2;
            }
            retain_texture_mips(image, 2048);
        }
    }
}

fn retain_texture_mips(image: &mut Image, maximum_dimension: u32) {
    use bevy::render::render_resource::{TextureDataOrder, TextureDimension};
    let descriptor = &mut image.texture_descriptor;
    if descriptor.dimension != TextureDimension::D2
        || descriptor.size.depth_or_array_layers != 1
        || descriptor.mip_level_count < 2
        || image.data_order != TextureDataOrder::MipMajor
        || image.texture_view_descriptor.is_some()
    {
        return;
    }
    let Some(data) = image.data.as_mut() else {
        return;
    };
    let Some(block_bytes) = descriptor.format.block_copy_size(None) else {
        return;
    };
    let (block_width, block_height) = descriptor.format.block_dimensions();
    let mut width = descriptor.size.width;
    let mut height = descriptor.size.height;
    let mut skipped = 0;
    let mut offset = 0;
    while width.max(height) > maximum_dimension && skipped + 1 < descriptor.mip_level_count {
        offset +=
            (width.div_ceil(block_width) * height.div_ceil(block_height) * block_bytes) as usize;
        width = (width / 2).max(1);
        height = (height / 2).max(1);
        skipped += 1;
    }
    if skipped == 0 || offset >= data.len() {
        return;
    }
    *data = data[offset..].to_vec();
    descriptor.size.width = width;
    descriptor.size.height = height;
    descriptor.mip_level_count -= skipped;
}

pub(super) fn load(server: &AssetServer, texture: &PreparedTexture) -> Handle<Image> {
    let srgb = texture.color_space == "srgb";
    let native = texture.sampler;
    server
        .load_builder()
        .with_settings(move |settings: &mut ImageLoaderSettings| {
            settings.is_srgb = srgb;
            let point = native & 7 == 1;
            let filtered_mips = !point && native & 24 == 16;
            settings.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
                address_mode_u: if native & 32 != 0 {
                    ImageAddressMode::ClampToEdge
                } else {
                    ImageAddressMode::Repeat
                },
                address_mode_v: if native & 64 != 0 {
                    ImageAddressMode::ClampToEdge
                } else {
                    ImageAddressMode::Repeat
                },
                mag_filter: if point {
                    ImageFilterMode::Nearest
                } else {
                    ImageFilterMode::Linear
                },
                min_filter: if point {
                    ImageFilterMode::Nearest
                } else {
                    ImageFilterMode::Linear
                },
                mipmap_filter: if filtered_mips {
                    ImageFilterMode::Linear
                } else {
                    ImageFilterMode::Nearest
                },
                lod_max_clamp: if native & 24 == 0 { 0. } else { 32. },
                anisotropy_clamp: if filtered_mips {
                    match native & 7 {
                        3 => 2,
                        4 => 4,
                        5 => 8,
                        6 => 16,
                        _ => 1,
                    }
                } else {
                    1
                },
                ..ImageSamplerDescriptor::linear()
            });
        })
        .load(texture.uri.clone())
}

type PreparedHandles = HashMap<AssetId<StandardMaterial>, Vec<(PreparedTexture, Handle<Image>)>>;

pub(super) fn prepare(
    mut commands: Commands,
    entities: Query<
        (Entity, &MeshMaterial3d<StandardMaterial>, &SurfaceMetadata),
        (Without<PbrReady>, Without<crate::assets::WorldCollider>),
    >,
    server: Res<AssetServer>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut pending: Local<PreparedHandles>,
    mut failure: ResMut<crate::assets::LoadFailure>,
    mut next: ResMut<NextState<crate::GameState>>,
    mut library: Option<ResMut<super::opaque::SurfaceLibrary>>,
) {
    if library.as_ref().is_some_and(|library| library.is_added()) {
        pending.clear();
    }
    for (entity, handle, extras) in &entities {
        let surface = &extras.0["surface"];
        if surface.is_null() {
            commands.entity(entity).insert(PbrReady);
            continue;
        }
        if let Err(error) = validate(surface) {
            failure.0 = format!("Material {}: {error}", surface["identity"]);
            next.set(crate::GameState::LoadError);
            return;
        }
        let channels = pending.entry(handle.0.id()).or_insert_with(|| {
            textures(&surface["channels"]).unwrap()
                .into_iter()
                .map(|texture| {
                    let image = load(&server, &texture);
                    (texture, image)
                })
                .collect()
        });
        match textures_ready(&server, channels.iter().map(|(_, image)| image)) {
            Ok(true) => {}
            Ok(false) => continue,
            Err(error) => {
                failure.0 = error;
                next.set(crate::GameState::LoadError);
                return;
            }
        }
        let Some(mut material) = materials.get_mut(&handle.0) else {
            continue;
        };
        let channel = |role: &str| {
            channels
                .iter()
                .find(|(texture, _)| texture.role == role && texture.layer == 0)
                .map(|(_, h)| h.clone())
        };
        apply_channels(&mut material, channel);
        material.flip_normal_map_y = false;
        material.clearcoat = 0.;
        if material.metallic_roughness_texture.is_some() {
            material.metallic = 1.;
            material.perceptual_roughness = 1.;
        }
        super::opaque::apply_factors(&mut material, &surface["standard"]);
        material.opaque_render_method = if material.unlit || material.specular_transmission > 0. {
            bevy::material::OpaqueRendererMethod::Forward
        } else {
            bevy::material::OpaqueRendererMethod::Deferred
        };
        let state = &surface["presentation"];
        material.fog_enabled = state["fog"].as_bool().unwrap_or(true);
        material.double_sided = state["cull"].as_str() == Some("none");
        material.cull_mode = match state["cull"].as_str() {
            Some("none") => None,
            Some("front") => Some(bevy::render::render_resource::Face::Front),
            _ => Some(bevy::render::render_resource::Face::Back),
        };
        material.alpha_mode = match state["alpha"].as_str() {
            Some("mask") => AlphaMode::Mask(state["cutoff"].as_f64().unwrap_or(0.5) as f32),
            Some("blend") => AlphaMode::Blend,
            Some("premultiplied") => AlphaMode::Premultiplied,
            Some("add") => AlphaMode::Add,
            Some("multiply") => AlphaMode::Multiply,
            _ => AlphaMode::Opaque,
        };
        if !surface["source_shadow_only"].is_null() {
            material.base_color = Color::BLACK;
            material.reflectance = 0.;
            material.emissive = LinearRgba::BLACK;
            material.unlit = false;
            commands.entity(entity).insert((
                bevy::camera::visibility::RenderLayers::layer(SHADOW_ONLY_LAYER),
                Visibility::Inherited,
            ));
        } else if state["visible"].as_bool() == Some(false) {
            commands.entity(entity).insert(Visibility::Hidden);
        }
        if super::shadows::excludes_caster(surface) || surface["bake"]["script_layers"] == true {
            commands.entity(entity).insert(bevy::light::NotShadowCaster);
        }
        if let Some(ref mut library) = library {
            if let Err(error) = super::opaque::register_prepared(library, surface, channels) {
                failure.0 = format!("Material {}: {error}", surface["identity"]);
                next.set(crate::GameState::LoadError);
                return;
            }
        }
        pending.remove(&handle.0.id());
        commands.entity(entity).insert(PbrReady);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn texture_residency_keeps_existing_compressed_mips_and_their_dimensions() {
        use bevy::render::render_resource::{
            Extent3d, TextureDataOrder, TextureDimension, TextureFormat,
        };
        let mut image = Image::new_uninit(
            Extent3d {
                width: 64,
                height: 32,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            TextureFormat::Bc7RgbaUnorm,
            bevy::asset::RenderAssetUsages::default(),
        );
        image.texture_descriptor.mip_level_count = 7;
        image.data_order = TextureDataOrder::MipMajor;
        let mut data = Vec::new();
        for mip in 0..7 {
            let size = (64u32 >> mip).max(1).div_ceil(4) * (32u32 >> mip).max(1).div_ceil(4) * 16;
            data.extend(std::iter::repeat_n(mip as u8, size as usize));
        }
        let expected = data[16 * 8 * 16 + 8 * 4 * 16..].to_vec();
        image.data = Some(data);
        retain_texture_mips(&mut image, 16);
        assert_eq!(image.texture_descriptor.size.width, 16);
        assert_eq!(image.texture_descriptor.size.height, 8);
        assert_eq!(image.texture_descriptor.mip_level_count, 5);
        assert_eq!(image.data.as_ref().unwrap(), &expected);
        retain_texture_mips(&mut image, 16);
        assert_eq!(image.data.unwrap(), expected);
    }

    #[test]
    fn corrupt_material_metadata_reports_the_dependency() {
        let mut surface = json!({"version":3,"identity":"test",
            "channels":[{"uri":"../outside.ktx2","role":"color","color_space":"srgb","sampler":0}],
            "presentation":{"alpha":"opaque","cull":"back"}});
        assert!(validate(&surface).is_err());
        surface["channels"][0]["uri"] = json!("prepared/test.ktx2");
        assert!(validate(&surface).is_ok());
        surface["presentation"]["alpha"] = json!("unsupported");
        assert!(validate(&surface).is_err());
        surface["presentation"]["alpha"] = json!("mask");
        assert!(validate(&surface).is_err());
        surface["presentation"]["cutoff"] = json!(128. / 255.);
        assert!(validate(&surface).is_ok());
        surface["pending_runtime"] = json!("missing source consumer");
        assert!(
            validate(&surface)
                .unwrap_err()
                .contains("missing source consumer")
        );
    }

    #[test]
    fn prepared_uv_transform_maps_source_domain_and_rejects_invalid_inputs() {
        let factors = json!({"uv_transform":[0.5,0.,0.,0.25,-1.,2.]});
        let transform = uv_transform(&factors).unwrap();
        assert_eq!(transform.transform_point2(Vec2::new(2., -8.)), Vec2::ZERO);
        assert_eq!(transform.transform_point2(Vec2::new(4., -4.)), Vec2::ONE);
        let mut material = StandardMaterial::default();
        super::super::opaque::apply_factors(&mut material, &factors);
        assert_eq!(material.uv_transform, transform);
        super::super::opaque::apply_factors(&mut material, &json!({}));
        assert_eq!(material.uv_transform, bevy::math::Affine2::IDENTITY);
        for values in [
            json!([0., 0., 0., 1., 0., 0.]),
            json!([1., 0., 0., 1., 0.]),
            json!([1e100, 0., 0., 1., 0., 0.]),
        ] {
            assert!(uv_transform(&json!({"uv_transform":values})).is_err());
        }
    }
    #[test]
    #[ignore = "requires the installed locked Bevy shader sources"]
    fn remaining_shaders_compose_with_locked_bevy() {
        use bevy::asset::AssetId;
        use bevy::shader::{
            Shader, ShaderCache, ShaderCacheError, ShaderCacheSource, ShaderDefVal,
        };
        use std::path::Path;
        fn install(
            cache: &mut ShaderCache<(), ()>,
            root: &Path,
            directory: &Path,
            crate_name: &str,
            index: &mut u128,
        ) {
            for item in std::fs::read_dir(directory).unwrap() {
                let path = item.unwrap().path();
                if path.is_dir() {
                    install(cache, root, &path, crate_name, index);
                } else if path
                    .extension()
                    .is_some_and(|extension| extension == "wesl")
                {
                    *index += 1;
                    let relative = path
                        .strip_prefix(root)
                        .unwrap()
                        .to_string_lossy()
                        .replace('\\', "/");
                    cache.set_shader(
                        AssetId::Uuid {
                            uuid: bevy::asset::uuid::Uuid::from_u128(*index),
                        },
                        Shader::from_wesl(
                            std::fs::read_to_string(&path).unwrap(),
                            format!("embedded://{crate_name}/{relative}"),
                        ),
                    );
                }
            }
        }
        let mut cache = ShaderCache::new((), |_, source, _| {
            let ShaderCacheSource::Wgsl(source) = source else {
                panic!("Expected composed WGSL");
            };
            let module = wgpu::naga::front::wgsl::parse_str(&source).map_err(|error| {
                ShaderCacheError::CreateShaderModule(error.emit_to_string(&source))
            })?;
            wgpu::naga::valid::Validator::new(
                wgpu::naga::valid::ValidationFlags::all(),
                wgpu::naga::valid::Capabilities::all(),
            )
            .validate(&module)
            .map_err(|error| ShaderCacheError::CreateShaderModule(format!("{error:#?}")))?;
            Ok(())
        });
        let home = std::env::var_os("CARGO_HOME")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| {
                std::path::PathBuf::from(
                    std::env::var_os("USERPROFILE")
                        .or_else(|| std::env::var_os("HOME"))
                        .unwrap(),
                )
                .join(".cargo")
            });
        let registries = std::fs::read_dir(home.join("registry/src"))
            .unwrap()
            .map(|item| item.unwrap().path())
            .collect::<Vec<_>>();
        let mut index = 0;
        for name in ["bevy_pbr", "bevy_render", "bevy_core_pipeline"] {
            let path = registries
                .iter()
                .map(|root| root.join(format!("{name}-0.20.0/src")))
                .find(|root| root.is_dir())
                .unwrap();
            install(&mut cache, &path, &path, name, &mut index);
        }
        let definitions = [
            ShaderDefVal::UInt(
                "MATERIAL_BIND_GROUP".into(),
                bevy::pbr::MATERIAL_BIND_GROUP_INDEX as u32,
            ),
            ShaderDefVal::UInt(
                "MAX_DIRECTIONAL_LIGHTS".into(),
                bevy::pbr::MAX_DIRECTIONAL_LIGHTS as u32,
            ),
            ShaderDefVal::UInt(
                "MAX_CASCADES_PER_LIGHT".into(),
                bevy::pbr::MAX_CASCADES_PER_LIGHT as u32,
            ),
            ShaderDefVal::UInt("MAX_RECT_LIGHTS".into(), bevy::pbr::MAX_RECT_LIGHTS as u32),
            ShaderDefVal::UInt("PER_OBJECT_BUFFER_BATCH_SIZE".into(), 1),
            ShaderDefVal::Bool("VERTEX_POSITIONS".into(), true),
            ShaderDefVal::Bool("VERTEX_OUTPUT_INSTANCE_INDEX".into(), true),
            ShaderDefVal::Bool("VERTEX_UVS_A".into(), true),
            ShaderDefVal::Bool("VERTEX_COLORS".into(), true),
            ShaderDefVal::Bool("VERTEX_NORMALS".into(), true),
            ShaderDefVal::Bool("VERTEX_TANGENTS".into(), true),
            ShaderDefVal::Bool("DEPTH_PREPASS".into(), true),
            ShaderDefVal::Bool("VIEW_PROJECTION_PERSPECTIVE".into(), true),
        ];
        let hybrid = AssetId::Uuid {
            uuid: bevy::asset::uuid::Uuid::from_u128(index + 3),
        };
        cache.set_shader(
            hybrid,
            Shader::from_wesl(
                include_str!("hybrid.wesl"),
                "embedded://origins/rendering/hybrid.wesl",
            ),
        );
        let mut hybrid_definitions = definitions.to_vec();
        hybrid_definitions.push(ShaderDefVal::Bool("VERTEX_UVS_B".into(), true));
        cache.get(2, hybrid, &hybrid_definitions).unwrap();
        let flare = AssetId::Uuid {
            uuid: bevy::asset::uuid::Uuid::from_u128(index + 5),
        };
        cache.set_shader(
            flare,
            Shader::from_wesl(
                include_str!("../native_fx/superflare.wesl"),
                "embedded://origins/native_fx/superflare.wesl",
            ),
        );
        cache.get(4, flare, &definitions).unwrap();
    }
}

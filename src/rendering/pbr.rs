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

#[derive(Clone, bevy::render::render_resource::ShaderType)]
struct WaterControls {
    domains: [Vec4; 4],
    slope: Vec4,
    color_normal: Vec4,
    color_surface: Vec4,
    fresnel: Vec4,
    time: Vec4,
}

#[derive(Asset, bevy::render::render_resource::AsBindGroup, TypePath, Clone)]
pub(super) struct WaterExtension {
    #[uniform(100)]
    controls: WaterControls,
    #[texture(101)]
    #[sampler(102)]
    #[dependency]
    low: Handle<Image>,
    #[texture(103)]
    #[sampler(104)]
    #[dependency]
    high: Handle<Image>,
}

impl bevy::pbr::MaterialExtension for WaterExtension {
    fn fragment_shader() -> bevy::shader::ShaderRef {
        "embedded://origins/rendering/water.wesl".into()
    }
    fn enable_prepass() -> bool {
        false
    }
    fn enable_shadows() -> bool {
        false
    }
    fn enable_oit() -> bool {
        false
    }
}

type WaterMaterial = bevy::pbr::ExtendedMaterial<StandardMaterial, WaterExtension>;

pub(super) fn install(app: &mut App) {
    bevy::shader::load_shader_library!(app, "water.wesl");
    app.add_plugins(MaterialPlugin::<WaterMaterial>::default())
        .add_systems(PostUpdate, advance_water.after(super::MaterialAnimation));
}

fn advance_water(
    clock: Res<super::opaque::NativeClock>,
    mut materials: ResMut<Assets<WaterMaterial>>,
) {
    let Some(milliseconds) = clock.render_milliseconds() else {
        return;
    };
    let Ok(time) = super::opaque::game_time(milliseconds, 0.) else {
        return;
    };
    for (_, material) in materials.iter_mut() {
        material.extension.controls.time = Vec4::from_array(time);
    }
}

fn water_controls(
    surface: &serde_json::Value,
) -> Result<Option<(WaterControls, [PreparedTexture; 2])>, String> {
    let water = &surface["source_water"];
    if water.is_null() {
        return Ok(None);
    }
    #[derive(Deserialize)]
    struct Layer {
        uri: String,
        sampler: u8,
        position_scale: f32,
        velocity: [f32; 2],
        slope_decode: [f32; 2],
        coordinates: String,
        normal_space: String,
    }
    let layers: [Layer; 4] = serde_json::from_value(water["layers"].clone())
        .map_err(|error| format!("Invalid native water layers: {error}"))?;
    if water["consumer"] != "native_world_normals_and_palette_shared_pbr"
        || surface["native"]["technique_set"] != "wpc_cod7water_we876z2f"
        || water["game_time_code"] != 25
        || water["game_time_component"] != 3
        || layers.iter().any(|layer| {
            layer.sampler != 20
                || layer.coordinates != "source_position_xy"
                || layer.normal_space != "source_world_xy"
                || !layer.position_scale.is_finite()
                || !layer
                    .velocity
                    .iter()
                    .chain(layer.slope_decode.iter())
                    .all(|value| value.is_finite())
        })
        || layers[0].uri != layers[1].uri
        || layers[2].uri != layers[3].uri
        || layers[0].slope_decode != layers[1].slope_decode
        || layers[2].slope_decode != layers[3].slope_decode
        || !textures(&surface["channels"])?.iter().any(|texture| {
            texture.role == "normal"
                && texture.uri == layers[0].uri
                && texture.color_space == "linear"
                && texture.sampler == 20
        })
    {
        return Err("Unsupported native water domains or time binding".into());
    }
    for layer in &layers {
        crate::origins::validate_asset_path(&layer.uri).map_err(|error| error.to_string())?;
    }
    let vector = |name: &str| -> Result<Vec4, String> {
        let value: [f32; 4] = serde_json::from_value(water[name].clone())
            .map_err(|error| format!("Invalid native water {name}: {error}"))?;
        if value.iter().any(|value| !value.is_finite()) {
            return Err(format!("Non-finite native water {name}"));
        }
        Ok(Vec4::from_array(value))
    };
    let textures = [0, 2].map(|index| PreparedTexture {
        uri: layers[index].uri.clone(),
        role: if index == 0 {
            "water_low"
        } else {
            "water_high"
        }
        .into(),
        layer: 0,
        color_space: "linear".into(),
        sampler: layers[index].sampler,
    });
    Ok(Some((
        WaterControls {
            domains: std::array::from_fn(|index| {
                Vec4::new(
                    layers[index].position_scale,
                    layers[index].velocity[0],
                    layers[index].velocity[1],
                    0.,
                )
            }),
            slope: Vec4::new(
                layers[0].slope_decode[0],
                layers[0].slope_decode[1],
                layers[2].slope_decode[0],
                layers[2].slope_decode[1],
            ),
            color_normal: vector("color_normal")?,
            color_surface: vector("color_surface")?,
            fresnel: vector("control0")?,
            time: Vec4::ZERO,
        },
        textures,
    )))
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
    water_controls(surface)?;
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
    surfaces: Query<Entity, (With<PbrReady>, Changed<GltfMaterialExtras>)>,
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
        if let Some(image) = images.get_mut_untracked(*id)
            && image.texture_descriptor.dimension == TextureDimension::D1
            && image.height() == 1
        {
            image.texture_descriptor.dimension = TextureDimension::D2;
        }
    }
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
        (
            Entity,
            &MeshMaterial3d<StandardMaterial>,
            &GltfMaterialExtras,
        ),
        Without<PbrReady>,
    >,
    server: Res<AssetServer>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut water_materials: ResMut<Assets<WaterMaterial>>,
    mut water_cache: Local<HashMap<AssetId<StandardMaterial>, Handle<WaterMaterial>>>,
    mut pending: Local<PreparedHandles>,
    mut failure: ResMut<crate::assets::LoadFailure>,
    mut next: ResMut<NextState<crate::GameState>>,
    mut library: Option<ResMut<super::opaque::SurfaceLibrary>>,
) {
    if library.as_ref().is_some_and(|library| library.is_added()) {
        pending.clear();
        water_cache.clear();
    }
    for (entity, handle, extras) in &entities {
        let value: serde_json::Value = match serde_json::from_str(&extras.value) {
            Ok(value) => value,
            Err(error) => {
                failure.0 = format!("Invalid standard material metadata: {error}");
                next.set(crate::GameState::LoadError);
                return;
            }
        };
        let surface = &value["surface"];
        if surface.is_null() {
            commands.entity(entity).insert(PbrReady);
            continue;
        }
        if let Err(error) = validate(surface) {
            failure.0 = format!("Material {}: {error}", surface["identity"]);
            next.set(crate::GameState::LoadError);
            return;
        }
        let water = water_controls(surface).expect("Validated native water controls");
        let channels = pending.entry(handle.0.id()).or_insert_with(|| {
            let mut textures = textures(&surface["channels"]).unwrap();
            if let Some((_, inputs)) = &water {
                textures.extend(inputs.iter().cloned());
            }
            textures
                .into_iter()
                .map(|texture| {
                    let image = load(&server, &texture);
                    (texture, image)
                })
                .collect()
        });
        for (texture, image) in channels.iter() {
            if let Some(LoadState::Failed(error)) = server.get_load_state(image.id()) {
                failure.0 = format!("Standard material texture {}: {error}", texture.uri);
                next.set(crate::GameState::LoadError);
                return;
            }
        }
        if channels
            .iter()
            .any(|(_, image)| !server.is_loaded_with_dependencies(image.id()))
        {
            continue;
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
        material.base_color_texture = channel("color");
        material.normal_map_texture = channel("normal");
        material.metallic_roughness_texture = channel("response");
        material.occlusion_texture = channel("occlusion").or_else(|| channel("response"));
        material.emissive_texture = channel("emissive");
        material.specular_tint_texture =
            channel("reflectance").or_else(|| channel("specular_gloss"));
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
                super::SurfaceReady,
            ));
        } else if state["visible"].as_bool() == Some(false) {
            commands.entity(entity).insert(Visibility::Hidden);
        }
        if super::shadows::excludes_caster(surface) {
            commands.entity(entity).insert(bevy::light::NotShadowCaster);
        }
        if let Some((controls, _)) = water {
            let water = water_cache
                .entry(handle.0.id())
                .or_insert_with(|| {
                    let mut base = material.clone();
                    base.normal_map_texture = None;
                    base.opaque_render_method = bevy::material::OpaqueRendererMethod::Forward;
                    water_materials.add(bevy::pbr::ExtendedMaterial {
                        base,
                        extension: WaterExtension {
                            controls,
                            low: channel("water_low").unwrap(),
                            high: channel("water_high").unwrap(),
                        },
                    })
                })
                .clone();
            commands
                .entity(entity)
                .remove::<MeshMaterial3d<StandardMaterial>>()
                .insert((
                    MeshMaterial3d(water),
                    super::SurfaceReady,
                    super::opaque::StandardReady,
                    Visibility::Inherited,
                    bevy::light::NotShadowCaster,
                ));
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
}

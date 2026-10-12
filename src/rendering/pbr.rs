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

#[derive(Clone, Deserialize)]
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
    mut pending: Local<PreparedHandles>,
    mut failure: ResMut<crate::assets::LoadFailure>,
    mut next: ResMut<NextState<crate::GameState>>,
    mut library: Option<ResMut<super::opaque::SurfaceLibrary>>,
) {
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
        let channels = pending.entry(handle.0.id()).or_insert_with(|| {
            serde_json::from_value::<Vec<PreparedTexture>>(surface["channels"].clone())
                .expect("Source-built material channels")
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
        material.occlusion_texture = channel("response").or_else(|| channel("occlusion"));
        material.emissive_texture = channel("emissive");
        material.specular_tint_texture =
            channel("reflectance").or_else(|| channel("specular_gloss"));
        if surface["version"] != 3 {
            material.flip_normal_map_y = channels.iter().any(|(texture, _)| {
                texture.role == "normal" && surface["native_weapon"].is_object()
            });
        }
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
        if state["visible"].as_bool() == Some(false) {
            commands.entity(entity).insert(Visibility::Hidden);
        }
        if super::shadows::excludes_caster(surface) {
            commands.entity(entity).insert(bevy::light::NotShadowCaster);
        }
        if let Some(ref mut library) = library {
            super::opaque::register_prepared(library, surface, channels);
        }
        pending.remove(&handle.0.id());
        commands.entity(entity).insert(PbrReady);
    }
}

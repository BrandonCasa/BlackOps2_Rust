use bevy::{
    asset::{AssetId, LoadState, uuid_handle},
    gltf::GltfMaterialExtras,
    image::{
        ImageAddressMode, ImageFilterMode, ImageLoaderSettings, ImageSampler,
        ImageSamplerDescriptor,
    },
    pbr::{ExtendedMaterial, MaterialExtension},
    prelude::*,
    render::render_resource::AsBindGroup,
    shader::{Shader, ShaderRef},
};
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Clone, Deserialize)]
pub(super) struct PreparedTexture {
    pub uri: String,
    pub role: String,
    pub layer: u64,
    pub color_space: String,
    pub sampler: u8,
}

#[derive(Clone, Copy)]
pub(super) enum NormalEncoding {
    Rgb,
    Rg,
}

impl NormalEncoding {
    pub(super) fn for_image(image: &Image) -> Self {
        use bevy::render::render_resource::TextureFormat;
        match image.texture_descriptor.format {
            TextureFormat::Rg8Unorm
            | TextureFormat::Rg16Unorm
            | TextureFormat::Bc5RgUnorm
            | TextureFormat::EacRg11Unorm => Self::Rg,
            _ => Self::Rgb,
        }
    }

    pub(super) fn flag(self, layer: u32) -> u32 {
        match self {
            Self::Rgb => 0,
            Self::Rg => 1 << layer,
        }
    }
}

#[derive(Component)]
pub(crate) struct PbrReady;

#[derive(Component)]
pub(super) struct PendingMaterialVisibility(Visibility);

#[derive(Resource, Default)]
pub(super) struct MaterialDiagnostic(pub u32);

#[derive(Asset, TypePath, AsBindGroup, Clone)]
pub(super) struct DiagnosticExtension {
    #[uniform(100)]
    parameters: Vec4,
}

impl MaterialExtension for DiagnosticExtension {
    fn fragment_shader() -> ShaderRef {
        DIAGNOSTIC_SHADER.into()
    }
}

pub(super) type DiagnosticMaterial = ExtendedMaterial<StandardMaterial, DiagnosticExtension>;
const DIAGNOSTIC_SHADER: Handle<Shader> = uuid_handle!("1aa8e217-dae4-4976-9e40-f625773e9a20");
const DIAGNOSTIC_BINDINGS: Handle<Shader> = uuid_handle!("604f94e4-d362-448b-bd18-5eb2d912ad20");
const DIAGNOSTIC_LIGHTING: Handle<Shader> = uuid_handle!("ab5c9d0c-e0fa-4f55-81f8-2929d20a8415");

pub(super) fn lighting_diagnostics(source: String, mode: &str) -> String {
    let source = source
        .replacen("    var direct_light: vec3<f32> = vec3<f32>(0.0);",
            "    var origins_sun = vec3(0.0);\n    var origins_shadow = 1.0;\n    var origins_local_shadow = 0.0;\n    var origins_local_weight = 0.0;\n    var direct_light: vec3<f32> = vec3<f32>(0.0);", 1)
        .replacen("@if(DIRECTIONAL_LIGHT_SHADOW_MAP_DEBUG_CASCADES)",
            &format!("        if {mode}() == 9u {{ origins_sun += light_contrib * shadow; }}\n        if {mode}() == 10u {{ origins_shadow = min(origins_shadow, shadow); }}\n@if(DIRECTIONAL_LIGHT_SHADOW_MAP_DEBUG_CASCADES)"), 1);
    let local = format!(
        "\n        if {mode}() == 12u {{\n            let origins_weight = dot(max(light_contrib, vec3(0.0)), vec3(0.2126, 0.7152, 0.0722));\n            origins_local_shadow += origins_weight * shadow;\n            origins_local_weight += origins_weight;\n        }}"
    );
    let source = source
        .replace("        let light_contrib = lighting::point_light(light_id, &lighting_input, enable_diffuse, true);", &format!("        let light_contrib = lighting::point_light(light_id, &lighting_input, enable_diffuse, true);{local}"))
        .replace("        let light_contrib = lighting::spot_light(light_id, &lighting_input, enable_diffuse);", &format!("        let light_contrib = lighting::spot_light(light_id, &lighting_input, enable_diffuse);{local}"));
    source
        .replacen("    // Total light", &format!(
            "    if {mode}() == 8u {{ return vec4(emissive_light, 1.0); }}\n    if {mode}() == 9u {{ return vec4(origins_sun * view_bindings::view.exposure, 1.0); }}\n    if {mode}() == 10u {{ return vec4(vec3(origins_shadow), 1.0); }}\n    if {mode}() == 11u {{ return vec4(indirect_light * view_bindings::view.exposure, 1.0); }}\n    if {mode}() == 12u {{ return vec4(vec3(select(1.0, origins_local_shadow / max(origins_local_weight, 0.000001), origins_local_weight > 0.0)), 1.0); }}\n    // Total light"), 1)
        .replacen("    indirect_light += environment_light.specular * specular_occlusion;",
            &format!("    if {mode}() != 11u {{ indirect_light += environment_light.specular * specular_occlusion; }}"), 1)
}

pub(super) fn diagnostic_shaders(shaders: &mut Assets<Shader>, engine: &str) {
    let hook = "    indirect_light += environment_light.diffuse * diffuse_occlusion;";
    assert!(
        engine.contains(hook),
        "Pinned Bevy material diagnostic hook changed"
    );
    let lighting = format!(
        "import origins::diagnostic_bindings::material_diagnostic_mode;\n{}",
        engine.replace("package::", "bevy_pbr::")
    ).replacen(hook, "    if material_diagnostic_mode() == 6u { return vec4(environment_light.specular * specular_occlusion * view_bindings::view.exposure, 1.0); }\n    indirect_light += environment_light.diffuse * diffuse_occlusion;", 1);
    let lighting = lighting_diagnostics(lighting, "material_diagnostic_mode");
    for (handle, source, path) in [
        (
            DIAGNOSTIC_SHADER,
            include_str!("diagnostic.wesl").to_owned(),
            "embedded://origins/material_diagnostic.wesl",
        ),
        (
            DIAGNOSTIC_BINDINGS,
            include_str!("diagnostic_bindings.wesl").to_owned(),
            "embedded://origins/diagnostic_bindings.wesl",
        ),
        (
            DIAGNOSTIC_LIGHTING,
            lighting,
            "embedded://origins/diagnostic_lighting.wesl",
        ),
    ] {
        shaders
            .insert(handle.id(), Shader::from_wesl(source, path))
            .expect("Valid shader UUID");
    }
}

#[derive(Component)]
pub(super) struct OriginalMaterial(Handle<StandardMaterial>);

pub(super) fn diagnostics(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    options: Res<crate::assets::LaunchOptions>,
    mut mode: ResMut<MaterialDiagnostic>,
    sources: Res<Assets<StandardMaterial>>,
    mut diagnostic_materials: ResMut<Assets<DiagnosticMaterial>>,
    mut shaders: ResMut<Assets<Shader>>,
    ready: Option<Res<super::opaque::MaterialShadersReady>>,
    ordinary: Query<
        (
            Entity,
            &MeshMaterial3d<StandardMaterial>,
            &GltfMaterialExtras,
        ),
        (With<PbrReady>, Without<crate::assets::WorldCollider>),
    >,
    active: Query<(
        Entity,
        &OriginalMaterial,
        &MeshMaterial3d<DiagnosticMaterial>,
    )>,
    mut cache: Local<HashMap<AssetId<StandardMaterial>, Handle<DiagnosticMaterial>>>,
) {
    let requested = if options.material_debug != 0
        || options.verify_viewmodel_shadows
        || options.verify_actor_shadows
    {
        options.material_debug
    } else if keys.just_pressed(KeyCode::F11) {
        (mode.0 + 1) % 13
    } else {
        mode.0
    };
    if requested != mode.0 {
        mode.0 = requested;
        info!(
            "Material diagnostic: {} (0 lit, 1 identity, 2 roughness, 3 metal, 4 normal, 5 coverage, 6 reflection, 7 geometry normal, 8 emission, 9 sun, 10 sun shadow, 11 indirect diffuse, 12 local shadow)",
            mode.0
        );
    }
    if mode.0 != 0 && ready.is_none() {
        if !super::opaque::prepare_diagnostic_shaders(&mut shaders) {
            return;
        }
        commands.insert_resource(super::opaque::MaterialShadersReady);
    }
    for (entity, original, handle) in &active {
        if mode.0 == 0 {
            commands
                .entity(entity)
                .remove::<(MeshMaterial3d<DiagnosticMaterial>, OriginalMaterial)>()
                .insert(MeshMaterial3d(original.0.clone()));
        } else if let Some(mut material) = diagnostic_materials.get_mut(&handle.0)
            && material.extension.parameters.x != mode.0 as f32
        {
            material.extension.parameters.x = mode.0 as f32;
        }
    }
    if mode.0 == 0 {
        return;
    }
    for (entity, original, extras) in &ordinary {
        let Some(source) = sources.get(&original.0) else {
            continue;
        };
        let handle = cache.entry(original.0.id()).or_insert_with(|| {
            let identity: serde_json::Value =
                serde_json::from_str(&extras.value).unwrap_or_default();
            let name = identity["surface"]["identity"]
                .as_str()
                .unwrap_or(&extras.value);
            let hash = name.bytes().fold(2166136261u32, |value, byte| {
                (value ^ byte as u32).wrapping_mul(16777619)
            });
            diagnostic_materials.add(DiagnosticMaterial {
                base: source.clone(),
                extension: DiagnosticExtension {
                    parameters: Vec4::new(mode.0 as f32, (hash % 65535) as f32, 0., 0.),
                },
            })
        });
        if let Some(mut material) = diagnostic_materials.get_mut(&*handle) {
            material.extension.parameters.x = mode.0 as f32;
        }
        commands
            .entity(entity)
            .remove::<MeshMaterial3d<StandardMaterial>>()
            .insert((
                OriginalMaterial(original.0.clone()),
                MeshMaterial3d(handle.clone()),
            ));
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
            let filtered_mips = !point && native & 24 != 0;
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
                anisotropy_clamp: if filtered_mips { 16 } else { 1 },
                ..ImageSamplerDescriptor::linear()
            });
        })
        .load(texture.uri.clone())
}

type PreparedHandles = HashMap<AssetId<StandardMaterial>, Vec<(PreparedTexture, Handle<Image>)>>;

pub(super) struct CachedMaterial {
    hidden: bool,
    material: StandardMaterial,
}
impl CachedMaterial {
    fn maps_match(&self, material: &StandardMaterial) -> bool {
        let prepared = &self.material;
        material.base_color_texture == prepared.base_color_texture
            && material.normal_map_texture == prepared.normal_map_texture
            && material.metallic_roughness_texture == prepared.metallic_roughness_texture
            && material.occlusion_texture == prepared.occlusion_texture
            && material.emissive_texture == prepared.emissive_texture
            && material.clearcoat_texture == prepared.clearcoat_texture
    }
}

pub(super) fn prepare(
    mut commands: Commands,
    entities: Query<
        (
            Entity,
            &MeshMaterial3d<StandardMaterial>,
            &GltfMaterialExtras,
            Option<&Visibility>,
            Option<&PendingMaterialVisibility>,
            Has<bevy::mesh::skinning::SkinnedMesh>,
        ),
        Without<PbrReady>,
    >,
    server: Res<AssetServer>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut pending: Local<PreparedHandles>,
    mut prepared: Local<HashMap<AssetId<StandardMaterial>, CachedMaterial>>,
    mut failure: ResMut<crate::assets::LoadFailure>,
    mut next: ResMut<NextState<crate::GameState>>,
    profile: Res<super::profile::VisualProfile>,
    mut library: Option<ResMut<super::opaque::SurfaceLibrary>>,
) {
    for (entity, handle, extras, visibility, pending_visibility, skinned) in &entities {
        if let Some(cached) = prepared.get(&handle.0.id()) {
            let Some(mut material) = materials.get_mut(&handle.0) else {
                continue;
            };
            if !cached.maps_match(&material) {
                *material = cached.material.clone();
            }
            if cached.hidden {
                commands.entity(entity).insert(Visibility::Hidden);
            } else if let Some(original) = pending_visibility {
                commands.entity(entity).insert(original.0);
            }
            commands
                .entity(entity)
                .remove::<PendingMaterialVisibility>()
                .insert(PbrReady);
            continue;
        }
        let channels = pending.entry(handle.0.id()).or_insert_with(|| {
            let value: serde_json::Value = serde_json::from_str(&extras.value).unwrap_or_default();
            if value["surface"]["version"] != 2 {
                return Vec::new();
            }
            serde_json::from_value::<Vec<PreparedTexture>>(value["surface"]["channels"].clone())
                .expect("Validated PBR channel metadata")
                .into_iter()
                .map(|texture| {
                    let image = load(&server, &texture);
                    (texture, image)
                })
                .collect()
        });
        for (texture, image) in channels.iter() {
            if let Some(LoadState::Failed(error)) = server.get_load_state(image.id()) {
                failure.0 = format!("Prepared material texture {}: {error}", texture.uri);
                error!("{}", failure.0);
                next.set(crate::GameState::LoadError);
                return;
            }
        }
        if channels
            .iter()
            .any(|(_, handle)| !server.is_loaded_with_dependencies(handle.id()))
        {
            if skinned
                && pending_visibility.is_none()
                && let Some(visibility) = visibility
            {
                commands
                    .entity(entity)
                    .insert((PendingMaterialVisibility(*visibility), Visibility::Hidden));
            }
            continue;
        }
        let Some(mut material) = materials.get_mut(&handle.0) else {
            continue;
        };
        let channel = |role: &str| {
            channels
                .iter()
                .find(|(t, _)| t.role == role && t.layer == 0)
                .map(|(_, h)| h.clone())
        };
        let value: serde_json::Value = serde_json::from_str(&extras.value).unwrap_or_default();
        if let Some(ref mut library) = library {
            super::opaque::register_prepared(library, &value["surface"], channels);
        }
        if value["surface"]["version"] == 2 {
            material.unlit = value["surface"]["family"].as_str() == Some("unlit");
            material.emissive = LinearRgba::BLACK;
            material.emissive_texture = None;
        }
        if !channels.is_empty() {
            material.base_color_texture = channel("color");
            material.normal_map_texture = channel("normal");
            material.metallic_roughness_texture = channel("response");
            material.occlusion_texture = channel("response");
            if material.metallic_roughness_texture.is_some() {
                material.perceptual_roughness = 1.;
                material.metallic = 1.;
            }
            material.reflectance = 0.5;
            material.clearcoat_texture = channel("coating");
            material.clearcoat = if material.clearcoat_texture.is_some() {
                1.
            } else {
                0.
            };
            material.clearcoat_perceptual_roughness = value["surface"]["coating_roughness"]
                .as_f64()
                .unwrap_or(0.12) as f32;
            material.flip_normal_map_y = false;
        }
        let state = &value["surface"]["presentation"];
        if !state.is_null() {
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
                Some("premultiplied") if !channels.is_empty() => AlphaMode::Blend,
                Some("premultiplied") => AlphaMode::Premultiplied,
                Some("add") => AlphaMode::Add,
                Some("multiply") => AlphaMode::Multiply,
                _ => AlphaMode::Opaque,
            };
            if state["emissive"].as_bool() == Some(true) {
                material.emissive_texture = channel("emissive").or_else(|| channel("color"));
                material.emissive = LinearRgba::WHITE * profile.emissive_nits;
                material.emissive_exposure_weight = 1.;
            }
            if state["visible"].as_bool() == Some(false) {
                commands.entity(entity).insert(Visibility::Hidden);
            } else if let Some(original) = pending_visibility {
                commands.entity(entity).insert(original.0);
            }
        } else if let Some(original) = pending_visibility {
            commands.entity(entity).insert(original.0);
        }
        pending.remove(&handle.0.id());
        prepared.insert(
            handle.0.id(),
            CachedMaterial {
                hidden: state["visible"].as_bool() == Some(false),
                material: material.clone(),
            },
        );
        commands
            .entity(entity)
            .remove::<PendingMaterialVisibility>()
            .insert(PbrReady);
    }
}

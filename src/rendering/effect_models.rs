use super::{opaque, pbr};
use bevy::{
    gltf::GltfMaterialExtras,
    mesh::MeshVertexBufferLayoutRef,
    pbr::{ExtendedMaterial, MaterialExtension, MaterialExtensionKey, MaterialExtensionPipeline},
    prelude::*,
    render::render_resource::{
        AsBindGroup, CompareFunction, RenderPipelineDescriptor, ShaderType,
        SpecializedMeshPipelineError,
    },
    shader::ShaderRef,
};
use std::collections::HashMap;

#[derive(Clone, ShaderType, Default)]
struct Controls {
    constants: [Vec4; 16],
    kind: UVec4,
}

#[derive(Asset, AsBindGroup, TypePath, Clone)]
struct EffectExtension {
    #[uniform(100)]
    controls: Controls,
    #[texture(101)]
    #[sampler(102)]
    #[dependency]
    texture0: Option<Handle<Image>>,
    #[texture(103)]
    #[sampler(104)]
    #[dependency]
    texture1: Option<Handle<Image>>,
    #[texture(105)]
    #[sampler(106)]
    #[dependency]
    texture2: Option<Handle<Image>>,
    #[texture(107)]
    #[sampler(108)]
    #[dependency]
    texture3: Option<Handle<Image>>,
    #[texture(109)]
    #[sampler(110)]
    #[dependency]
    texture4: Option<Handle<Image>>,
    #[texture(111)]
    #[sampler(112)]
    #[dependency]
    texture5: Option<Handle<Image>>,
    #[texture(113)]
    #[sampler(114)]
    #[dependency]
    texture6: Option<Handle<Image>>,
}

impl MaterialExtension for EffectExtension {
    fn fragment_shader() -> ShaderRef {
        "embedded://origins/rendering/effect_models.wesl".into()
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
    fn specialize(
        _pipeline: &MaterialExtensionPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        _layout: &MeshVertexBufferLayoutRef,
        _key: MaterialExtensionKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        if let Some(depth) = &mut descriptor.depth_stencil {
            depth.depth_compare = Some(CompareFunction::Always);
            depth.depth_write_enabled = Some(false);
        }
        if let Some(fragment) = &mut descriptor.fragment {
            fragment
                .shader_defs
                .push("VERTEX_OUTPUT_INSTANCE_INDEX".into());
        }
        descriptor
            .vertex
            .shader_defs
            .push("VERTEX_OUTPUT_INSTANCE_INDEX".into());
        Ok(())
    }
}

type EffectMaterial = ExtendedMaterial<StandardMaterial, EffectExtension>;

#[derive(Component)]
struct Checked;

struct Pending {
    textures: Vec<Handle<Image>>,
    visibility: Visibility,
}

pub(super) fn install(app: &mut App) {
    bevy::shader::load_shader_library!(app, "effect_models.wesl");
    app.add_plugins(MaterialPlugin::<EffectMaterial>::default())
        .add_systems(
            Update,
            (prepare, advance).chain().after(super::SurfacePreparation),
        );
}

fn prepare(
    mut commands: Commands,
    surfaces: Query<
        (
            Entity,
            &MeshMaterial3d<StandardMaterial>,
            &GltfMaterialExtras,
            Option<&Visibility>,
        ),
        (With<pbr::PbrReady>, Without<Checked>),
    >,
    server: Res<AssetServer>,
    originals: Res<Assets<StandardMaterial>>,
    mut materials: ResMut<Assets<EffectMaterial>>,
    mut pending: Local<HashMap<Entity, Pending>>,
    mut status: ResMut<opaque::OpaqueSurfaceStatus>,
    mut failure: ResMut<crate::assets::LoadFailure>,
    mut next: ResMut<NextState<crate::GameState>>,
) {
    pending.retain(|entity, _| surfaces.contains(*entity));
    for (entity, original, extras, visibility) in &surfaces {
        let result = (|| -> Result<Option<(EffectMaterial, Visibility)>, String> {
            let metadata: serde_json::Value =
                serde_json::from_str(&extras.value).map_err(|e| e.to_string())?;
            let effect = &metadata["surface"]["source_effect_model"];
            if effect.is_null() {
                return Ok(None);
            }
            let kind = match effect["kind"].as_str() {
                Some("projected_blood") => 0,
                Some("electric_disc") => 1,
                _ => return Err("Unknown native effect-model consumer".into()),
            };
            let inputs = effect["inputs"]
                .as_array()
                .ok_or("Native effect-model inputs are absent")?
                .iter()
                .map(|input| {
                    pbr::textures(&serde_json::json!([input])).map(|mut inputs| inputs.remove(0))
                })
                .collect::<Result<Vec<_>, _>>()?;
            if inputs.len() != if kind == 0 { 1 } else { 7 } {
                return Err("Native effect-model image count is invalid".into());
            }
            let constants: [[f32; 4]; 16] =
                serde_json::from_value(effect["constants"].clone()).map_err(|e| e.to_string())?;
            if constants.iter().flatten().any(|v| !v.is_finite()) {
                return Err("Native effect-model constants are non-finite".into());
            }
            let handles = pending.entry(entity).or_insert_with(|| {
                commands.entity(entity).insert(Visibility::Hidden);
                Pending {
                    textures: inputs
                        .iter()
                        .map(|input| pbr::load(&server, input))
                        .collect(),
                    visibility: visibility.copied().unwrap_or_default(),
                }
            });
            for image in &handles.textures {
                if let Some(bevy::asset::LoadState::Failed(error)) =
                    server.get_load_state(image.id())
                {
                    return Err(format!(
                        "Native effect-model image {:?}: {error}",
                        image.path()
                    ));
                }
            }
            let Some(mut base) = originals.get(&original.0).cloned() else {
                status.pending += 1;
                return Ok(None);
            };
            if handles
                .textures
                .iter()
                .any(|image| !server.is_loaded_with_dependencies(image.id()))
            {
                status.pending += 1;
                return Ok(None);
            }
            base.opaque_render_method = bevy::material::OpaqueRendererMethod::Forward;
            base.fog_enabled = false;
            base.cull_mode = None;
            base.reflectance = 0.;
            let texture = |index| handles.textures.get(index).cloned();
            let extension = EffectExtension {
                controls: Controls {
                    constants: constants.map(Vec4::from_array),
                    kind: UVec4::new(kind, 0, 0, 0),
                },
                texture0: texture(0),
                texture1: texture(1),
                texture2: texture(2),
                texture3: texture(3),
                texture4: texture(4),
                texture5: texture(5),
                texture6: texture(6),
            };
            Ok(Some((
                ExtendedMaterial { base, extension },
                handles.visibility,
            )))
        })();
        match result {
            Ok(Some((material, visibility))) => {
                commands
                    .entity(entity)
                    .remove::<MeshMaterial3d<StandardMaterial>>()
                    .insert((
                        MeshMaterial3d(materials.add(material)),
                        visibility,
                        Checked,
                        bevy::light::NotShadowCaster,
                    ));
                pending.remove(&entity);
            }
            Ok(None) => {
                if !pending.contains_key(&entity) {
                    commands.entity(entity).insert(Checked);
                }
            }
            Err(error) => {
                failure.0 = format!("Native effect-model: {error}");
                next.set(crate::GameState::LoadError);
                return;
            }
        }
    }
}

fn advance(clock: Res<opaque::NativeClock>, mut materials: ResMut<Assets<EffectMaterial>>) {
    let Some(milliseconds) = clock.render_milliseconds() else {
        return;
    };
    let Ok(time) = opaque::game_time(milliseconds, 0.) else {
        return;
    };
    for (_, material) in materials.iter_mut() {
        material.extension.controls.constants[4] = Vec4::from_array(time);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[ignore = "requires the installed locked Bevy shader sources"]
    fn effect_model_shader_composes_with_locked_bevy() {
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
        let id = AssetId::Uuid {
            uuid: bevy::asset::uuid::Uuid::from_u128(index + 1),
        };
        cache.set_shader(
            id,
            Shader::from_wesl(
                include_str!("effect_models.wesl"),
                "embedded://origins/rendering/effect_models.wesl",
            ),
        );
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
            ShaderDefVal::Bool("VERTEX_OUTPUT_INSTANCE_INDEX".into(), true),
            ShaderDefVal::Bool("VERTEX_UVS_A".into(), true),
            ShaderDefVal::Bool("VERTEX_COLORS".into(), true),
            ShaderDefVal::Bool("VERTEX_NORMALS".into(), true),
            ShaderDefVal::Bool("VERTEX_TANGENTS".into(), true),
            ShaderDefVal::Bool("DEPTH_PREPASS".into(), true),
            ShaderDefVal::Bool("VIEW_PROJECTION_PERSPECTIVE".into(), true),
        ];
        cache.get(0, id, &definitions).unwrap();
        let water = AssetId::Uuid {
            uuid: bevy::asset::uuid::Uuid::from_u128(index + 2),
        };
        cache.set_shader(
            water,
            Shader::from_wesl(
                include_str!("water.wesl"),
                "embedded://origins/rendering/water.wesl",
            ),
        );
        cache.get(1, water, &definitions).unwrap();
    }
}

use super::{
    opaque::{AnimatedMaterial, NativeArtInputs, NativeClock, StandardReady, SurfaceLibrary},
    pbr::{self, PreparedTexture},
};
use bevy::{gltf::GltfMaterialName, mesh::MeshTag, prelude::*, render::storage::ShaderBuffer};
use serde::Deserialize;
use std::collections::{HashMap, HashSet};

#[derive(Resource, Default)]
struct Instances {
    buffer: Handle<ShaderBuffer>,
    slots: HashMap<Entity, u32>,
    free: Vec<u32>,
    data: Vec<[f32; 4]>,
}

#[derive(Component)]
struct Live {
    slot: u32,
    base: Vec4,
}

#[derive(Component)]
pub(super) struct RayOwner(pub Entity);

pub(super) fn install(app: &mut App) {
    bevy::shader::load_shader_library!(app, "hybrid.wesl");
    app.init_resource::<Instances>()
        .add_plugins(MaterialPlugin::<LayerMaterial>::default())
        .add_systems(
            PostUpdate,
            (prepare, upload)
                .chain()
                .after(super::SurfacePreparation)
                .after(super::MaterialAnimation)
                .run_if(super::opaque::material_scene_active),
        )
        .add_systems(
            PostUpdate,
            sync_rays
                .after(super::opaque::animate_flags)
                .before(super::solari::prepare_meshes),
        );
}

fn prepare(
    mut commands: Commands,
    query: Query<
        (
            Entity,
            &MeshMaterial3d<StandardMaterial>,
            &GltfMaterialName,
            &Mesh3d,
            &GlobalTransform,
            &InheritedVisibility,
            Option<&bevy::mesh::skinning::SkinnedMesh>,
            Option<&AnimatedMaterial>,
        ),
        (With<StandardReady>, Without<Live>, Without<RayOwner>),
    >,
    library: Res<SurfaceLibrary>,
    sources: Res<Assets<StandardMaterial>>,
    server: Res<AssetServer>,
    mut buffers: ResMut<Assets<ShaderBuffer>>,
    mut materials: ResMut<Assets<LayerMaterial>>,
    mut instances: ResMut<Instances>,
    mut cache: Local<HashMap<String, Handle<LayerMaterial>>>,
    mut pending: Local<HashMap<String, Vec<Handle<Image>>>>,
    mut status: ResMut<super::opaque::OpaqueSurfaceStatus>,
    mut failure: ResMut<crate::assets::LoadFailure>,
    mut next: ResMut<NextState<crate::GameState>>,
) {
    if instances.buffer == Handle::default() {
        instances.data.resize(2, [0.; 4]);
        let mut buffer = ShaderBuffer::from(instances.data.clone());
        buffer.label = "Origins native material state".into();
        instances.buffer = buffers.add(buffer);
    }
    for (entity, source, name, mesh, transform, visibility, skin, animation) in &query {
        let Some(definition) = library.definitions.get(&name.0) else {
            continue;
        };
        let metadata = &definition.metadata;
        let parsed = layer_controls(metadata);
        let (controls, inputs) = match parsed {
            Ok(Some(value)) => value,
            Ok(None) => continue,
            Err(error) => {
                failure.0 = error;
                next.set(crate::GameState::LoadError);
                return;
            }
        };
        let Some(base) = sources.get(&source.0) else {
            status.pending += 1;
            continue;
        };
        let identity = metadata["native"]["name"]
            .as_str()
            .unwrap_or(&name.0)
            .to_owned();
        let material = if let Some(material) = cache.get(&identity) {
            material.clone()
        } else {
            let images = pending.entry(identity.clone()).or_insert_with(|| {
                inputs
                    .iter()
                    .map(|input| pbr::load(&server, input))
                    .collect()
            });
            if let Some(error) =
                images
                    .iter()
                    .find_map(|image| match server.get_load_state(image.id()) {
                        Some(bevy::asset::LoadState::Failed(error)) => Some(error.to_string()),
                        _ => None,
                    })
            {
                failure.0 = error;
                next.set(crate::GameState::LoadError);
                return;
            }
            if images
                .iter()
                .any(|image| !server.is_loaded_with_dependencies(image.id()))
            {
                status.pending += 1;
                continue;
            }
            let channel = |role: &str, layer: u64| {
                inputs
                    .iter()
                    .zip(images.iter())
                    .find(|(input, _)| input.role == role && input.layer == layer)
                    .map(|(_, image)| image.clone())
            };
            let layer = |index: usize| {
                channel(
                    &format!(
                        "native_layer_{}",
                        index.min(controls.options.y as usize - 1)
                    ),
                    0,
                )
                .unwrap()
            };
            let mut base = base.clone();
            base.base_color_texture = None;
            base.normal_map_texture = None;
            base.metallic_roughness_texture = None;
            base.emissive_texture = None;
            base.occlusion_texture = None;
            base.specular_tint_texture = None;
            base.unlit = controls.options.w == 0.;
            base.opaque_render_method = bevy::material::OpaqueRendererMethod::Forward;
            let material = materials.add(bevy::pbr::ExtendedMaterial {
                base,
                extension: LayerExtension {
                    first: layer(0),
                    second: layer(1),
                    third: layer(2),
                    fourth: layer(3),
                    normal: channel("native_layer_normal", 0).unwrap_or_else(|| layer(0)),
                    controls: controls.clone(),
                    states: instances.buffer.clone(),
                },
            });
            cache.insert(identity.clone(), material.clone());
            pending.remove(&identity);
            material
        };
        let slot = instances.free.pop().unwrap_or_else(|| {
            let slot = instances.data.len() as u32 / 2;
            instances.data.extend([[0.; 4]; 2]);
            slot
        });
        instances.slots.insert(entity, slot);
        if metadata["hybrid"]["ray_visible"] == true {
            let mut ray = commands.spawn((
                ChildOf(entity),
                Mesh3d(mesh.0.clone()),
                MeshMaterial3d(source.0.clone()),
                Transform::IDENTITY,
                *transform,
                Visibility::Inherited,
                *visibility,
                bevy::camera::visibility::RenderLayers::none(),
                RayOwner(entity),
                name.clone(),
                StandardReady,
            ));
            if let Some(skin) = skin {
                ray.insert(skin.clone());
            }
            if let Some(animation) = animation {
                ray.insert(animation.clone());
            }
        }
        commands
            .entity(entity)
            .remove::<(MeshMaterial3d<StandardMaterial>, AnimatedMaterial)>()
            .insert((
                MeshMaterial3d(material),
                MeshTag::with_type::<Live>(slot),
                Live {
                    slot,
                    base: controls.script,
                },
            ));
    }
}

fn upload(
    query: Query<(Entity, &Live)>,
    parents: Query<&ChildOf>,
    rigs: Query<&crate::presentation::Rig>,
    inputs: Query<&NativeArtInputs>,
    clock: Res<NativeClock>,
    mut instances: ResMut<Instances>,
    mut buffers: ResMut<Assets<ShaderBuffer>>,
) {
    let mut live = HashSet::new();
    for (entity, surface) in &query {
        live.insert(entity);
        let mut current = Some(entity);
        let mut owner = None;
        while let Some(entity) = current {
            if let Ok(rig) = rigs.get(entity) {
                owner = inputs.get(rig.owner).ok();
            }
            if owner.is_none() {
                owner = inputs.get(entity).ok();
            }
            if owner.is_some() {
                break;
            }
            current = parents.get(entity).ok().map(ChildOf::parent);
        }
        let script = owner
            .filter(|input| input.script_available & 1 != 0)
            .map_or(surface.base.to_array(), |input| input.script_vectors[0]);
        let time = owner
            .and_then(|input| input.shader_time)
            .or_else(|| clock.render_milliseconds().map(|time| (time, 0.)))
            .and_then(|(time, start)| super::opaque::game_time(time, start).ok())
            .unwrap_or([0.; 4]);
        instances.data[surface.slot as usize * 2] = script;
        instances.data[surface.slot as usize * 2 + 1] = time;
    }
    let removed: Vec<_> = instances
        .slots
        .keys()
        .copied()
        .filter(|entity| !live.contains(entity))
        .collect();
    for entity in removed {
        if let Some(slot) = instances.slots.remove(&entity) {
            instances.free.push(slot);
        }
    }
    if let Some(mut buffer) = buffers.get_mut(&instances.buffer) {
        buffer.clear();
        buffer.extend_from_slice(&instances.data);
        buffer.resize_buffer((instances.data.len().next_power_of_two() * 16) as u64);
    }
}

fn sync_rays(
    mut commands: Commands,
    sources: Query<(&Mesh3d, Option<&bevy::mesh::skinning::SkinnedMesh>), Without<RayOwner>>,
    mut rays: Query<(
        Entity,
        &RayOwner,
        &mut Mesh3d,
        Option<&bevy::mesh::skinning::SkinnedMesh>,
    )>,
) {
    for (entity, owner, mut mesh, skin) in &mut rays {
        let Ok((source, source_skin)) = sources.get(owner.0) else {
            continue;
        };
        if mesh.0 != source.0 {
            mesh.0 = source.0.clone();
        }
        if let Some(source_skin) = source_skin {
            if skin.is_none_or(|skin| {
                skin.joints != source_skin.joints
                    || skin.inverse_bindposes != source_skin.inverse_bindposes
            }) {
                commands.entity(entity).insert(source_skin.clone());
            }
        } else if skin.is_some() {
            commands
                .entity(entity)
                .remove::<bevy::mesh::skinning::SkinnedMesh>();
        }
    }
}

#[derive(Clone, bevy::render::render_resource::ShaderType)]
struct LayerControls {
    scale_offset: [Vec4; 4],
    origin_rotation: [Vec4; 4],
    scroll_depth_alpha: [Vec4; 4],
    script: Vec4,
    time: Vec4,
    emission: Vec4,
    options: Vec4,
    reflection: Vec4,
    normal_decode: Vec4,
}

#[derive(Asset, bevy::render::render_resource::AsBindGroup, TypePath, Clone)]
pub(super) struct LayerExtension {
    #[uniform(100)]
    controls: LayerControls,
    #[texture(101)]
    #[sampler(102)]
    #[dependency]
    first: Handle<Image>,
    #[texture(103)]
    #[sampler(104)]
    #[dependency]
    second: Handle<Image>,
    #[texture(105)]
    #[sampler(106)]
    #[dependency]
    third: Handle<Image>,
    #[texture(107)]
    #[sampler(108)]
    #[dependency]
    fourth: Handle<Image>,
    #[texture(109)]
    #[sampler(110)]
    #[dependency]
    normal: Handle<Image>,
    #[storage(111, read_only)]
    #[dependency]
    states: Handle<ShaderBuffer>,
}

impl bevy::pbr::MaterialExtension for LayerExtension {
    fn fragment_shader() -> bevy::shader::ShaderRef {
        "embedded://origins/rendering/hybrid.wesl".into()
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

type LayerMaterial = bevy::pbr::ExtendedMaterial<StandardMaterial, LayerExtension>;

pub(super) fn validate(surface: &serde_json::Value) -> Result<(), String> {
    layer_controls(surface).map(|_| ())
}

fn layer_controls(
    surface: &serde_json::Value,
) -> Result<Option<(LayerControls, Vec<PreparedTexture>)>, String> {
    let script = &surface["source_script_layers"];
    if script["consumer"] != "native_hybrid_layers" {
        return Ok(None);
    }
    let mode = match (
        surface["native"]["technique_set"].as_str(),
        script["mode"].as_str(),
    ) {
        (
            Some("wpc_sw4_3d_unlit_1layer_8691j77q" | "mc_sw4_3d_unlit_1layer_nofog_91f00f80"),
            Some("single"),
        ) => 0.,
        (
            Some(
                "mc_sw4_3d_unlit_3layer_normal_script_q0zez5ju"
                | "mlv_sw4_3d_unlit_3layer_normal_script_766fz50u",
            ),
            Some("normal_script"),
        ) => 1.,
        (Some("wpc_sw4_3d_unlit_3layer_normal_script_q1z1f6w1"), Some("normal_script_unlit")) => 1.,
        (Some("mc_sw4_3d_unlit_3layer_nofog_falloff_5z943qe4"), Some("opaque")) => 2.,
        (Some("mc_sw4_3d_unlit_3layer_nofog_falloff_5z909874"), Some("blend")) => 3.,
        (
            Some(
                "mc_sw4_3d_unlit_3layer_nofog_falloff_w840w777"
                | "mlv_sw4_3d_unlit_3layer_nofog_falloff_fjz6ze0u",
            ),
            Some("add"),
        ) => 4.,
        (
            Some(
                "mc_sw4_3d_unlit_3layer_nofog_falloff_zfeather_wq4jwzz6"
                | "mlv_sw4_3d_unlit_3layer_nofog_falloff_zfeather_z0zw1090",
            ),
            Some("zfeather"),
        ) => 5.,
        (Some("mc_sw4_3d_unlit_3layer_nofog_qw71w785"), Some("layer_blend")) => 6.,
        (
            Some(
                "mc_sw4_3d_unlit_4layer_nofog_eee768jw" | "mlv_sw4_3d_unlit_4layer_nofog_2j0ezz67",
            ),
            Some("four_add"),
        ) => 7.,
        (Some("mc_sw4_3d_unlit_4layer_jw0z1973"), Some("four_opaque")) => 8.,
        _ => return Err("Unsupported native live layer family or mode".into()),
    };
    #[derive(Deserialize)]
    struct Layer {
        uri: String,
        sampler: u8,
        color_space: String,
        layer: usize,
        scale: [f32; 2],
        offset: [f32; 2],
        origin: [f32; 2],
        rotation: [f32; 2],
        scroll: [f32; 2],
        depth: f32,
        alpha: Option<f32>,
    }
    let layers: Vec<Layer> = serde_json::from_value(surface["source_layers"].clone())
        .map_err(|error| format!("Invalid native live layers: {error}"))?;
    let count = if mode == 0. {
        1
    } else if mode >= 7. {
        4
    } else {
        3
    };
    if script["code_constant"] != 169
        || script["base_state"] != serde_json::json!([0., 0., 0., 0.])
        || surface["source_layer_uv"]["game_time_code"] != 25
        || surface["source_layer_uv"]["game_time_component"] != 3
        || layers.len() != count
    {
        return Err("Unsupported native live layer state, clock or count".into());
    }
    let mut textures = Vec::new();
    for (index, layer) in layers.iter().enumerate() {
        crate::origins::validate_asset_path(&layer.uri).map_err(|error| error.to_string())?;
        if layer.color_space != "native_encoded"
            || layer.layer != index
            || !layer
                .scale
                .iter()
                .chain(layer.offset.iter())
                .chain(layer.origin.iter())
                .chain(layer.rotation.iter())
                .chain(layer.scroll.iter())
                .chain([&layer.depth, &layer.alpha.unwrap_or(0.)])
                .all(|value| value.is_finite())
        {
            return Err("Invalid native live layer input or parameter".into());
        }
        textures.push(PreparedTexture {
            uri: layer.uri.clone(),
            role: format!("native_layer_{index}"),
            layer: 0,
            color_space: "linear".into(),
            sampler: layer.sampler,
        });
    }
    let number = |value: &serde_json::Value, fallback: f32| -> Result<f32, String> {
        if value.is_null() {
            return Ok(fallback);
        }
        let value = value.as_f64().ok_or("Invalid native layer scalar")? as f32;
        value
            .is_finite()
            .then_some(value)
            .ok_or("Non-finite native layer scalar".into())
    };
    let normal = &surface["source_normal"];
    let mut normal_scale: [f32; 2] = [1., 1.];
    let mut decode: [f32; 2] = [1., 0.];
    if !normal.is_null() {
        let uri = normal["uri"]
            .as_str()
            .ok_or("Missing native layer normal image")?;
        crate::origins::validate_asset_path(uri).map_err(|error| error.to_string())?;
        normal_scale = serde_json::from_value(normal["scale"].clone())
            .map_err(|error| format!("Invalid native layer normal scale: {error}"))?;
        decode = serde_json::from_value(normal["decode"].clone())
            .map_err(|error| format!("Invalid native layer normal decode: {error}"))?;
        if !normal_scale
            .iter()
            .chain(decode.iter())
            .all(|value| value.is_finite())
        {
            return Err("Non-finite native layer normal parameters".into());
        }
        textures.push(PreparedTexture {
            uri: uri.into(),
            role: "native_layer_normal".into(),
            layer: 0,
            color_space: "linear".into(),
            sampler: normal["sampler"]
                .as_u64()
                .filter(|value| *value <= 255)
                .ok_or("Invalid native layer normal sampler")? as u8,
        });
    }
    let state: [f32; 4] = if script["state"].is_null() {
        [0.; 4]
    } else {
        serde_json::from_value(script["state"].clone())
            .map_err(|error| format!("Invalid native layer state: {error}"))?
    };
    if !state.iter().all(|value| value.is_finite()) {
        return Err("Non-finite native layer state".into());
    }
    let gloss = number(&surface["source_reflection"]["gloss"], 0.)?;
    let reflect =
        number(&surface["source_reflection"]["amount"], 0.)? * (0.75 + 0.25 * gloss).clamp(0., 1.);
    let controls = LayerControls {
        scale_offset: std::array::from_fn(|index| {
            let layer = &layers[index.min(count - 1)];
            Vec4::new(
                layer.scale[0],
                layer.scale[1],
                layer.offset[0],
                layer.offset[1],
            )
        }),
        origin_rotation: std::array::from_fn(|index| {
            let layer = &layers[index.min(count - 1)];
            Vec4::new(
                layer.origin[0],
                layer.origin[1],
                layer.rotation[0],
                layer.rotation[1],
            )
        }),
        scroll_depth_alpha: std::array::from_fn(|index| {
            let layer = &layers[index.min(count - 1)];
            Vec4::new(
                layer.scroll[0],
                layer.scroll[1],
                layer.depth,
                layer.alpha.unwrap_or(0.),
            )
        }),
        script: Vec4::from_array(state),
        time: Vec4::ZERO,
        emission: Vec4::new(
            number(&surface["source_emissive"]["amount"], 1.)?,
            number(&surface["source_falloff"]["power"], 0.)?,
            number(&surface["source_falloff"]["boost"], 1.)?,
            number(&surface["source_layer_uv"]["rotation_tau"], 1.)?,
        ),
        options: Vec4::new(
            mode,
            count as f32,
            number(&surface["source_layer_uv"]["depth_epsilon"], 0.)?,
            reflect,
        ),
        reflection: Vec4::new(
            gloss,
            normal_scale[0],
            normal_scale[1],
            number(&normal["height"], 0.)?,
        ),
        normal_decode: Vec4::new(decode[0], decode[1], (!normal.is_null()) as u8 as f32, 0.),
    };
    Ok(Some((controls, textures)))
}

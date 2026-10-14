use super::{opaque, pbr};
use bevy::{
    gltf::GltfMaterialExtras,
    pbr::{ExtendedMaterial, MaterialExtension},
    prelude::*,
    render::render_resource::{AsBindGroup, ShaderType},
    shader::ShaderRef,
};
use serde_json::Value;
use std::collections::HashMap;

#[derive(Clone, Default, ShaderType)]
struct Controls {
    domains: [Vec4; 4],
    color: Vec4,
    parameters: Vec4,
    time: Vec4,
    kind: UVec4,
}

#[derive(Asset, AsBindGroup, TypePath, Clone)]
pub(super) struct AnimatedExtension {
    #[uniform(100)]
    controls: Controls,
    #[texture(101)]
    #[sampler(102)]
    #[dependency]
    texture0: Handle<Image>,
    #[texture(103)]
    #[sampler(104)]
    #[dependency]
    texture1: Handle<Image>,
    #[texture(105)]
    #[sampler(106)]
    #[dependency]
    texture2: Handle<Image>,
    #[texture(107)]
    #[sampler(108)]
    #[dependency]
    texture3: Handle<Image>,
    #[texture(109)]
    #[sampler(110)]
    #[dependency]
    texture4: Option<Handle<Image>>,
    #[texture(111)]
    #[sampler(112)]
    #[dependency]
    texture5: Option<Handle<Image>>,
}

impl MaterialExtension for AnimatedExtension {
    fn fragment_shader() -> ShaderRef {
        "embedded://origins/rendering/animated_surfaces.wesl".into()
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

type AnimatedMaterial = ExtendedMaterial<StandardMaterial, AnimatedExtension>;

#[derive(Component)]
pub(super) struct Checked;

type Pending = HashMap<Entity, Vec<(pbr::PreparedTexture, Handle<Image>)>>;

pub(super) fn install(app: &mut App) {
    bevy::shader::load_shader_library!(app, "animated_surfaces.wesl");
    app.add_plugins(MaterialPlugin::<AnimatedMaterial>::default())
        .add_systems(
            PostUpdate,
            advance
                .after(super::MaterialAnimation)
                .run_if(opaque::material_scene_active),
        );
}

fn controls(surface: &Value) -> Result<Option<(Controls, Vec<pbr::PreparedTexture>)>, String> {
    let smoke = &surface["source_smoke"];
    let ember = &surface["source_embers"];
    let smoke_active = smoke["consumer"] == "native_layers_vertex_time_depth_forward";
    let ember_active = ember["consumer"] == "native_layers_vertex_time_shared_pbr";
    if !smoke_active && !ember_active {
        if !smoke.is_null() || !ember.is_null() {
            return Err("Native smoke or ember consumer is absent".into());
        }
        return Ok(None);
    }
    if smoke_active && ember_active {
        return Err("Conflicting native animated surface consumers".into());
    }
    let scalar = |value: &Value| -> Result<f32, String> {
        let result = value.as_f64().ok_or("Missing native animated scalar")? as f32;
        if !result.is_finite() {
            return Err("Non-finite native animated scalar".into());
        }
        Ok(result)
    };
    let vector = |value: &Value| -> Result<Vec4, String> {
        let components: [f32; 4] = serde_json::from_value(value.clone())
            .map_err(|error| format!("Invalid native animated vector: {error}"))?;
        if components.iter().any(|value| !value.is_finite()) {
            return Err("Non-finite native animated vector".into());
        }
        Ok(Vec4::from_array(components))
    };
    let mut result = Controls::default();
    let expected: &[(u32, &str, u8)] = if smoke_active {
        let uv = &surface["source_uv_animation"];
        let depth = &surface["source_depth_fade"];
        if surface["native"]["technique_set"] != "mc_sw4_3d_zmb_smoke_pillar_cheap_w6w7e738"
            || uv["game_time_code"] != 25
            || uv["component"] != 3
            || uv["independent_domains"] != true
            || uv["evaluation"] != "fragment"
            || depth["code_image"] != 18
            || depth["render_target_size_code"] != 17
            || depth["near_plane_scene_byte"] != 928
            || surface["source_vertex_color_transfer"]
                != "linear_primary_vertex_rgb_and_squared_primary_vertex_alpha"
            || surface["presentation"]["alpha"] != "add"
            || surface["presentation"]["fog"] != false
        {
            return Err("Unsupported native smoke surface contract".into());
        }
        let scales: [[f32; 2]; 4] = serde_json::from_value(uv["scale"].clone())
            .map_err(|error| format!("Invalid smoke scales: {error}"))?;
        let speeds: [[f32; 2]; 3] = serde_json::from_value(uv["speed"].clone())
            .map_err(|error| format!("Invalid smoke speeds: {error}"))?;
        let pivot: [f32; 2] = serde_json::from_value(uv["mask_offset"].clone())
            .map_err(|error| format!("Invalid smoke mask pivot: {error}"))?;
        if scales
            .iter()
            .flatten()
            .chain(speeds.iter().flatten())
            .chain(&pivot)
            .any(|value| !value.is_finite())
        {
            return Err("Non-finite smoke UV domain".into());
        }
        for index in 0..3 {
            result.domains[index] = Vec4::new(
                scales[index][0],
                scales[index][1],
                speeds[index][0],
                speeds[index][1],
            );
        }
        result.domains[3] = Vec4::new(scales[3][0], scales[3][1], pivot[0], pivot[1]);
        result.color = vector(&smoke["color"])?;
        result.parameters = Vec4::new(
            scalar(&smoke["hdr_amount"])?,
            scalar(&depth["factor"])?,
            scalar(&depth["fade_epsilon"])?,
            0.,
        );
        &[
            (170923560, "alpha_plus", 2),
            (1345510170, "alpha", 66),
            (2515352718, "alpha_mask", 226),
            (2564398276, "alpha_minus", 2),
            (2695565377, "color_plus", 2),
            (2816785293, "color_minus", 2),
        ]
    } else {
        if surface["native"]["technique_set"] != "wpc_sw4_3d_burning_embers_nuketown_74j6971w"
            || ember["game_time_code"] != 25
            || ember["game_time_component"] != 3
            || ember["sample_rate_hz"] != 10.
            || !ember["repeat_period"].is_null()
            || ember["flicker_updates"] != serde_json::json!([0, 1, 2, 3, 0, 1, 2, 0])
            || ember["flicker_weights"]
                != serde_json::json!([81.239487_f32, 17.348024_f32, 37.349838_f32, 59.39484_f32])
            || ember["candidate_rgb_scale"] != 6.
            || ember["has_additive_emission"] != false
            || ember["vertex_color"] != "multiply_encoded_mixed_rgba_then_square_rgb"
            || ember["hero_lighting"]["rows"] != serde_json::json!([114, 115, 116])
            || surface["presentation"]["alpha"] != "blend"
            || surface["presentation"]["fog"] != true
        {
            return Err("Unsupported native ember surface contract".into());
        }
        let direction: [f32; 2] = serde_json::from_value(ember["direction"].clone())
            .map_err(|error| format!("Invalid ember direction: {error}"))?;
        if direction.iter().any(|value| !value.is_finite()) {
            return Err("Non-finite ember direction".into());
        }
        result.domains[0] = Vec4::new(scalar(&ember["scale"])?, direction[0], direction[1], 0.);
        result.parameters = Vec4::new(
            scalar(&ember["flicker_min"])?,
            scalar(&ember["flicker_max"])?,
            scalar(&ember["seed"])?,
            0.,
        );
        result.kind.x = 1;
        &[
            (2485960688, "normal", 20),
            (2799170835, "mask", 20),
            (3617419774, "ember", 20),
            (4030327853, "diffuse", 20),
        ]
    };
    if surface["presentation"]["cull"] != "back" {
        return Err("Unsupported native animated surface culling".into());
    }
    let layers = surface["source_layers"]
        .as_array()
        .ok_or("Missing native animated layers")?;
    if layers.len() != expected.len() {
        return Err("Incorrect native animated layer count".into());
    }
    let textures = layers
        .iter()
        .zip(expected)
        .map(|(layer, &(hash, role, sampler))| {
            if layer["hash"] != hash
                || layer["role"] != role
                || layer["sampler"] != sampler
                || layer["color_space"] != "native_encoded"
            {
                return Err(format!("Unsupported native animated layer {role}"));
            }
            let uri = layer["uri"]
                .as_str()
                .ok_or("Missing native animated texture URI")?;
            crate::origins::validate_asset_path(uri).map_err(|error| error.to_string())?;
            Ok(pbr::PreparedTexture {
                uri: uri.into(),
                role: role.into(),
                layer: 0,
                color_space: "linear".into(),
                sampler,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    Ok(Some((result, textures)))
}

pub(super) fn validate(surface: &Value) -> Result<(), String> {
    controls(surface).map(|_| ())
}

pub(super) fn prepare(
    mut commands: Commands,
    surfaces: Query<
        (
            Entity,
            &MeshMaterial3d<StandardMaterial>,
            &GltfMaterialExtras,
            &Mesh3d,
        ),
        Without<Checked>,
    >,
    originals: Res<Assets<StandardMaterial>>,
    meshes: Res<Assets<Mesh>>,
    server: Res<AssetServer>,
    mut materials: ResMut<Assets<AnimatedMaterial>>,
    mut pending: Local<Pending>,
    mut library: ResMut<opaque::SurfaceLibrary>,
    mut failure: ResMut<crate::assets::LoadFailure>,
    mut next: ResMut<NextState<crate::GameState>>,
) {
    pending.retain(|entity, _| surfaces.contains(*entity));
    for (entity, original, extras, mesh) in &surfaces {
        let result = (|| -> Result<Option<AnimatedMaterial>, String> {
            let metadata: Value =
                serde_json::from_str(&extras.value).map_err(|error| error.to_string())?;
            let surface = &metadata["surface"];
            let Some((controls, inputs)) = controls(surface)? else {
                commands.entity(entity).insert(Checked);
                return Ok(None);
            };
            pbr::validate(surface)?;
            let channels = pending.entry(entity).or_insert_with(|| {
                inputs
                    .into_iter()
                    .map(|input| {
                        let handle = pbr::load(&server, &input);
                        (input, handle)
                    })
                    .collect()
            });
            opaque::register_prepared(&mut library, surface, channels)?;
            for (input, handle) in channels.iter() {
                if let Some(bevy::asset::LoadState::Failed(error)) =
                    server.get_load_state(handle.id())
                {
                    return Err(format!("Native animated image {}: {error}", input.uri));
                }
            }
            if channels
                .iter()
                .any(|(_, handle)| !server.is_loaded_with_dependencies(handle.id()))
            {
                return Ok(None);
            }
            let Some(mesh) = meshes.get(&mesh.0) else {
                return Ok(None);
            };
            if !mesh.contains_attribute(Mesh::ATTRIBUTE_UV_0)
                || !mesh.contains_attribute(Mesh::ATTRIBUTE_COLOR)
                || (controls.kind.x == 1
                    && (!mesh.contains_attribute(Mesh::ATTRIBUTE_NORMAL)
                        || !mesh.contains_attribute(Mesh::ATTRIBUTE_TANGENT)))
            {
                return Err("Native animated surface is missing authored vertex inputs".into());
            }
            let Some(mut base) = originals.get(&original.0).cloned() else {
                return Ok(None);
            };
            opaque::apply_factors(&mut base, &surface["standard"]);
            base.base_color_texture = None;
            base.normal_map_texture = None;
            base.metallic_roughness_texture = None;
            base.emissive_texture = None;
            base.emissive = LinearRgba::BLACK;
            base.opaque_render_method = bevy::material::OpaqueRendererMethod::Forward;
            base.cull_mode = Some(bevy::render::render_resource::Face::Back);
            base.double_sided = false;
            base.fog_enabled = controls.kind.x == 1;
            base.alpha_mode = if controls.kind.x == 0 {
                AlphaMode::Add
            } else {
                AlphaMode::Blend
            };
            let texture = |index: usize| channels.get(index).map(|(_, handle)| handle.clone());
            Ok(Some(ExtendedMaterial {
                base,
                extension: AnimatedExtension {
                    controls,
                    texture0: texture(0).unwrap(),
                    texture1: texture(1).unwrap(),
                    texture2: texture(2).unwrap(),
                    texture3: texture(3).unwrap(),
                    texture4: texture(4),
                    texture5: texture(5),
                },
            }))
        })();
        match result {
            Ok(Some(material)) => {
                commands
                    .entity(entity)
                    .remove::<MeshMaterial3d<StandardMaterial>>()
                    .insert((
                        MeshMaterial3d(materials.add(material)),
                        Checked,
                        super::SurfaceReady,
                        opaque::StandardReady,
                        pbr::PbrReady,
                        bevy::light::NotShadowCaster,
                    ));
                pending.remove(&entity);
            }
            Ok(None) => {}
            Err(error) => {
                failure.0 = format!("Native animated surface: {error}");
                next.set(crate::GameState::LoadError);
                return;
            }
        }
    }
}

pub(super) fn pending(
    surfaces: Query<
        &GltfMaterialExtras,
        (With<MeshMaterial3d<StandardMaterial>>, Without<Checked>),
    >,
    mut status: ResMut<opaque::OpaqueSurfaceStatus>,
) {
    for extras in &surfaces {
        if let Ok(metadata) = serde_json::from_str::<Value>(&extras.value)
            && (metadata["surface"]["source_smoke"]["consumer"]
                == "native_layers_vertex_time_depth_forward"
                || metadata["surface"]["source_embers"]["consumer"]
                    == "native_layers_vertex_time_shared_pbr")
        {
            status.pending += 1;
        }
    }
}

fn advance(
    clock: Res<opaque::NativeClock>,
    surfaces: Query<(Entity, &MeshMaterial3d<AnimatedMaterial>)>,
    parents: Query<&ChildOf>,
    inputs: Query<&opaque::NativeArtInputs>,
    starts: Query<&opaque::NativeShaderStart>,
    mut materials: ResMut<Assets<AnimatedMaterial>>,
    mut status: ResMut<opaque::OpaqueSurfaceStatus>,
    mut failure: ResMut<crate::assets::LoadFailure>,
    mut next: ResMut<NextState<crate::GameState>>,
) {
    let Some(milliseconds) = clock.render_milliseconds() else {
        status.pending += surfaces.iter().count();
        return;
    };
    for (entity, handle) in &surfaces {
        let Some(material) = materials.get_mut(&handle.0) else {
            continue;
        };
        let mut current = Some(entity);
        let mut owner = None;
        while let Some(ancestor) = current {
            if inputs.contains(ancestor) || starts.contains(ancestor) {
                owner = Some(ancestor);
                break;
            }
            current = parents.get(ancestor).ok().map(ChildOf::parent);
        }
        let source = if let Some(owner) = owner {
            inputs
                .get(owner)
                .ok()
                .and_then(|input| input.shader_time)
                .ok_or_else(|| {
                    "Native animated shader-time owner has no published input".to_owned()
                })
        } else if material.extension.controls.kind.x == 1 {
            Ok((milliseconds, 0.))
        } else {
            Err("Native smoke model has no shader-time owner".into())
        };
        match source.and_then(|(milliseconds, start)| opaque::game_time(milliseconds, start)) {
            Ok(time) => material.extension.controls.time = Vec4::from_array(time),
            Err(error) => {
                failure.0 = error;
                next.set(crate::GameState::LoadError);
                return;
            }
        }
    }
}

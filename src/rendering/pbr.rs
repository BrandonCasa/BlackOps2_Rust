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
}

impl bevy::pbr::MaterialExtension for LayerExtension {
    fn fragment_shader() -> bevy::shader::ShaderRef {
        "embedded://origins/rendering/script_layers.wesl".into()
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

#[derive(Component)]
struct LiveLayers {
    base: Vec4,
}

fn layer_controls(
    surface: &serde_json::Value,
) -> Result<Option<(LayerControls, Vec<PreparedTexture>)>, String> {
    let script = &surface["source_script_layers"];
    if script["consumer"] != "native_live_layers" {
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

fn advance_layers(
    clock: Res<super::opaque::NativeClock>,
    surfaces: Query<(Entity, &LiveLayers, &MeshMaterial3d<LayerMaterial>)>,
    parents: Query<&ChildOf>,
    inputs: Query<&super::opaque::NativeArtInputs>,
    mut materials: ResMut<Assets<LayerMaterial>>,
) {
    for (entity, live, handle) in &surfaces {
        let mut ancestor = Some(entity);
        let mut input = None;
        while let Some(current) = ancestor {
            if let Ok(found) = inputs.get(current) {
                input = Some(found);
                break;
            }
            ancestor = parents.get(current).ok().map(ChildOf::parent);
        }
        let script = input
            .filter(|input| input.script_available & 1 != 0)
            .map_or(live.base, |input| Vec4::from_array(input.script_vectors[0]));
        let time = input
            .and_then(|input| input.shader_time)
            .or_else(|| {
                clock
                    .render_milliseconds()
                    .map(|milliseconds| (milliseconds, 0.))
            })
            .and_then(|(milliseconds, start)| super::opaque::game_time(milliseconds, start).ok());
        if let Some(mut material) = materials.get_mut(&handle.0) {
            material.extension.controls.script = script;
            if let Some(time) = time {
                material.extension.controls.time = Vec4::from_array(time);
            }
        }
    }
}

pub(super) fn install(app: &mut App) {
    bevy::shader::load_shader_library!(app, "water.wesl");
    bevy::shader::load_shader_library!(app, "script_layers.wesl");
    app.add_plugins(MaterialPlugin::<WaterMaterial>::default())
        .add_plugins(MaterialPlugin::<LayerMaterial>::default())
        .add_systems(
            PostUpdate,
            (advance_water, advance_layers).after(super::MaterialAnimation),
        );
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
    layer_controls(surface)?;
    super::animated_surfaces::validate(surface)?;
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
        if let Some(image) = images.get_mut_untracked(*id) {
            if image.texture_descriptor.dimension == TextureDimension::D1 && image.height() == 1 {
                image.texture_descriptor.dimension = TextureDimension::D2;
            }
            retain_texture_mips(image, 4096);
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
    { return; }
    let Some(data) = image.data.as_mut() else { return; };
    let Some(block_bytes) = descriptor.format.block_copy_size(None) else { return; };
    let (block_width, block_height) = descriptor.format.block_dimensions();
    let mut width = descriptor.size.width;
    let mut height = descriptor.size.height;
    let mut skipped = 0;
    let mut offset = 0;
    while width.max(height) > maximum_dimension && skipped + 1 < descriptor.mip_level_count {
        offset += (width.div_ceil(block_width) * height.div_ceil(block_height) * block_bytes) as usize;
        width = (width / 2).max(1);
        height = (height / 2).max(1);
        skipped += 1;
    }
    if skipped == 0 || offset >= data.len() { return; }
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
        (
            Entity,
            &MeshMaterial3d<StandardMaterial>,
            &GltfMaterialExtras,
        ),
        (Without<PbrReady>, Without<crate::assets::WorldCollider>),
    >,
    server: Res<AssetServer>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut water_materials: ResMut<Assets<WaterMaterial>>,
    mut layer_materials: ResMut<Assets<LayerMaterial>>,
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
        if surface["source_smoke"]["consumer"] == "native_layers_vertex_time_depth_forward"
            || surface["source_embers"]["consumer"] == "native_layers_vertex_time_shared_pbr"
        {
            continue;
        }
        let water = water_controls(surface).expect("Validated native water controls");
        let layers = layer_controls(surface).expect("Validated native layer controls");
        let channels = pending.entry(handle.0.id()).or_insert_with(|| {
            let mut textures = textures(&surface["channels"]).unwrap();
            if let Some((_, inputs)) = &water {
                textures.extend(inputs.iter().cloned());
            }
            if let Some((_, inputs)) = &layers {
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
        if let Some((controls, inputs)) = layers {
            let layer = |index: usize| {
                channel(&format!(
                    "native_layer_{}",
                    index.min(controls.options.y as usize - 1)
                ))
                .unwrap()
            };
            let mut base = material.clone();
            base.base_color_texture = None;
            base.normal_map_texture = None;
            base.unlit = controls.options.w == 0.;
            base.opaque_render_method = bevy::material::OpaqueRendererMethod::Forward;
            let live = LiveLayers {
                base: controls.script,
            };
            let layered = layer_materials.add(bevy::pbr::ExtendedMaterial {
                base,
                extension: LayerExtension {
                    first: layer(0),
                    second: layer(1),
                    third: layer(2),
                    fourth: layer(3),
                    normal: if inputs
                        .iter()
                        .any(|input| input.role == "native_layer_normal")
                    {
                        channel("native_layer_normal").unwrap()
                    } else {
                        layer(0)
                    },
                    controls,
                },
            });
            commands
                .entity(entity)
                .remove::<MeshMaterial3d<StandardMaterial>>()
                .insert((
                    MeshMaterial3d(layered),
                    live,
                    super::SurfaceReady,
                    super::opaque::StandardReady,
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
    fn texture_residency_keeps_existing_compressed_mips_and_their_dimensions() {
        use bevy::render::render_resource::{Extent3d, TextureDataOrder, TextureDimension, TextureFormat};
        let mut image = Image::new_uninit(Extent3d { width: 64, height: 32, depth_or_array_layers: 1 },
            TextureDimension::D2, TextureFormat::Bc7RgbaUnorm, bevy::asset::RenderAssetUsages::default());
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
    fn live_layers_keep_object_vectors_and_unwrapped_owner_time_separate() {
        use super::super::opaque::{NativeArtInputs, NativeClock};
        let surface = json!({"native":{"technique_set":"wpc_sw4_3d_unlit_1layer_8691j77q"},
            "source_script_layers":{"consumer":"native_live_layers","mode":"single","code_constant":169,"base_state":[0.,0.,0.,0.]},
            "source_layers":[{"uri":"prepared/source.ktx2","layer":0,"sampler":20,"color_space":"native_encoded",
                "scale":[1.,1.],"offset":[0.,0.],"origin":[0.,0.],"rotation":[0.,0.],"scroll":[0.,0.],"depth":0.,"alpha":1.}],
            "source_layer_uv":{"game_time_code":25,"game_time_component":3,"depth_epsilon":0.00001,"rotation_tau":6.283185},
            "source_emissive":{"amount":1.}});
        let (controls, _) = layer_controls(&surface).unwrap().unwrap();
        let mut app = App::new();
        app.init_resource::<NativeClock>()
            .init_resource::<Assets<LayerMaterial>>()
            .add_systems(Update, advance_layers);
        let mut objects = Vec::new();
        for value in [0.65, 0.1] {
            let owner = app
                .world_mut()
                .spawn(NativeArtInputs {
                    script_vectors: [[value, 0., 0., 0.], [1., 2., 3., 4.], [0.; 4], [0.; 4]],
                    script_available: 3,
                    shader_time: Some((12500, 2.)),
                    ..default()
                })
                .id();
            let handle = app.world_mut().resource_mut::<Assets<LayerMaterial>>().add(
                bevy::pbr::ExtendedMaterial {
                    base: StandardMaterial::default(),
                    extension: LayerExtension {
                        controls: controls.clone(),
                        first: default(),
                        second: default(),
                        third: default(),
                        fourth: default(),
                        normal: default(),
                    },
                },
            );
            let entity = app
                .world_mut()
                .spawn((
                    ChildOf(owner),
                    LiveLayers { base: Vec4::ZERO },
                    MeshMaterial3d(handle.clone()),
                ))
                .id();
            objects.push((owner, entity, handle));
        }
        app.update();
        let materials = app.world().resource::<Assets<LayerMaterial>>();
        assert_eq!(
            materials
                .get(&objects[0].2)
                .unwrap()
                .extension
                .controls
                .script
                .x,
            0.65
        );
        assert_eq!(
            materials
                .get(&objects[1].2)
                .unwrap()
                .extension
                .controls
                .script
                .x,
            0.1
        );
        assert!(
            (materials
                .get(&objects[0].2)
                .unwrap()
                .extension
                .controls
                .time
                .w
                - 10.5)
                .abs()
                < 0.000002
        );
        app.world_mut()
            .entity_mut(objects[0].1)
            .insert(ChildOf(objects[1].0));
        app.update();
        assert_eq!(
            app.world()
                .resource::<Assets<LayerMaterial>>()
                .get(&objects[0].2)
                .unwrap()
                .extension
                .controls
                .script
                .x,
            0.1
        );
        app.world_mut()
            .get_mut::<NativeArtInputs>(objects[1].0)
            .unwrap()
            .script_available = 2;
        app.update();
        for (_, _, handle) in objects {
            assert_eq!(
                app.world()
                    .resource::<Assets<LayerMaterial>>()
                    .get(&handle)
                    .unwrap()
                    .extension
                    .controls
                    .script,
                Vec4::ZERO
            );
        }
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
}

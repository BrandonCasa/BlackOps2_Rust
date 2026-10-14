use super::{
    SurfaceReady,
    pbr::{PbrReady, PreparedTexture},
};
use bevy::{
    gltf::{GltfExtras, GltfMaterialName},
    prelude::*,
    render::render_resource::AsBindGroup,
};
use serde_json::Value;
use std::collections::{HashMap, HashSet};

pub(crate) fn bindless_supported(device: &bevy::render::renderer::RenderDevice) -> bool {
    StandardMaterial::bindless_supported(device)
}

#[derive(Resource, Default, serde::Serialize)]
pub(crate) struct OpaqueSurfaceStatus {
    world: usize,
    static_models: usize,
    normal: usize,
    response_maps: usize,
    masked: usize,
    transparent: usize,
    water: usize,
    pub pending: usize,
    shared_materials: usize,
}
impl OpaqueSurfaceStatus {
    pub(crate) fn ready(&self) -> bool {
        self.world > 0 && self.static_models > 0 && self.pending == 0
    }
}

pub(super) struct SurfaceDefinition {
    pub presentation: Value,
    channels: HashMap<(String, u64), Handle<Image>>,
    metadata: Value,
    frames: Vec<Frame>,
}
#[derive(Resource, Default)]
pub(crate) struct SurfaceLibrary {
    pub(super) definitions: HashMap<String, SurfaceDefinition>,
    pub(super) prepared_meshes: HashSet<AssetId<Mesh>>,
    pub(super) live_meshes: HashSet<AssetId<Mesh>>,
}
impl SurfaceLibrary {
    pub(crate) fn textures_ready(&self, server: &AssetServer) -> Result<bool, String> {
        let mut ready = true;
        for definition in self.definitions.values() {
            for image in definition.channels.values().chain(
                definition
                    .frames
                    .iter()
                    .flat_map(|frame| frame.channels.values()),
            ) {
                if let Some(bevy::asset::LoadState::Failed(error)) =
                    server.get_load_state(image.id())
                {
                    return Err(format!(
                        "Standard material texture {:?}: {error}",
                        image.path()
                    ));
                }
                ready &= server.is_loaded_with_dependencies(image.id());
            }
        }
        Ok(ready)
    }
}

pub(super) fn initialize(mut commands: Commands) {
    commands.insert_resource(SurfaceLibrary::default());
    commands.insert_resource(MaterialTime::default());
    commands.insert_resource(OpaqueSurfaceStatus::default());
}

#[derive(Resource, Default)]
pub(super) struct MaterialTime(f64);

pub(super) fn advance_time(time: Res<Time>, mut clock: ResMut<MaterialTime>) {
    clock.0 += time.delta_secs_f64();
}

pub(super) fn register_prepared(
    library: &mut SurfaceLibrary,
    surface: &Value,
    channels: &[(PreparedTexture, Handle<Image>)],
) -> Result<(), String> {
    let Some(identity) = surface["identity"].as_str() else {
        return Err("Prepared surface identity is missing".into());
    };
    validate_animation(surface)?;
    if let Some(existing) = library.definitions.get(identity) {
        return if existing.metadata == *surface {
            Ok(())
        } else {
            Err(format!(
                "Conflicting packed definitions for material {identity}"
            ))
        };
    }
    library.definitions.insert(
        identity.into(),
        SurfaceDefinition {
            presentation: surface["presentation"].clone(),
            channels: channels
                .iter()
                .map(|(texture, image)| ((texture.role.clone(), texture.layer), image.clone()))
                .collect(),
            metadata: surface.clone(),
            frames: Vec::new(),
        },
    );
    Ok(())
}

pub(super) fn apply_factors(material: &mut StandardMaterial, factors: &Value) {
    let scalar = |key: &str, default: f32| factors[key].as_f64().map_or(default, |v| v as f32);
    material.perceptual_roughness = scalar("roughness", material.perceptual_roughness);
    material.metallic = scalar("metallic", material.metallic);
    material.reflectance = scalar("reflectance", material.reflectance);
    material.specular_transmission = scalar("specular_transmission", 0.);
    material.diffuse_transmission = scalar("diffuse_transmission", 0.);
    material.ior = scalar("ior", 1.5);
    material.thickness = scalar("thickness", 0.);
    material.unlit = factors["unlit"].as_bool().unwrap_or(false);
    material.specular_tint =
        if let Some(color) = factors["specular_tint"].as_array().filter(|v| v.len() == 3) {
            Color::linear_rgb(
                color[0].as_f64().unwrap_or(1.) as f32,
                color[1].as_f64().unwrap_or(1.) as f32,
                color[2].as_f64().unwrap_or(1.) as f32,
            )
        } else {
            Color::WHITE
        };
    if let Some(color) = factors["base_color"].as_array().filter(|v| v.len() == 4) {
        material.base_color = Color::linear_rgba(
            color[0].as_f64().unwrap_or(1.) as f32,
            color[1].as_f64().unwrap_or(1.) as f32,
            color[2].as_f64().unwrap_or(1.) as f32,
            color[3].as_f64().unwrap_or(1.) as f32,
        );
    }
    if let Some(color) = factors["emissive"].as_array().filter(|v| v.len() == 3) {
        material.emissive = LinearRgba::rgb(
            color[0].as_f64().unwrap_or(0.) as f32,
            color[1].as_f64().unwrap_or(0.) as f32,
            color[2].as_f64().unwrap_or(0.) as f32,
        );
    }
}

#[derive(Component)]
pub(super) struct StandardReady;

pub(crate) fn reset_binding(entity: &mut bevy::ecs::system::EntityCommands) {
    entity.remove::<(
        PbrReady,
        StandardReady,
        AnimatedMaterial,
        ScalarAnimation,
        UvAnimation,
    )>();
}

pub(super) fn refresh_owners(
    mut surfaces: Query<
        (
            Entity,
            Option<&mut AnimatedMaterial>,
            Option<&mut ScalarAnimation>,
            Option<&mut UvAnimation>,
        ),
        Or<(
            With<AnimatedMaterial>,
            With<ScalarAnimation>,
            With<UvAnimation>,
        )>,
    >,
    parents: Query<&ChildOf>,
    rigs: Query<&crate::presentation::Rig>,
    stones: Query<(), With<crate::staff_puzzles::IceStone>>,
    inputs: Query<(), With<NativeArtInputs>>,
) {
    for (entity, animated, scalar, uv) in &mut surfaces {
        let mut ancestor = Some(entity);
        let mut owner = None;
        while let Some(current) = ancestor {
            if inputs.contains(current) || stones.contains(current) {
                owner = Some(current);
                break;
            }
            if let Ok(rig) = rigs.get(current) {
                owner = Some(rig.owner);
                break;
            }
            ancestor = parents.get(current).ok().map(ChildOf::parent);
        }
        if let Some(mut animation) = animated {
            animation.owner = owner;
        }
        if let Some(mut animation) = scalar {
            animation.owner = owner;
        }
        if let Some(mut animation) = uv {
            animation.owner = owner;
        }
    }
}

#[derive(Clone)]
struct Frame {
    channels: HashMap<String, Handle<Image>>,
    factors: Value,
}
#[derive(Component)]
pub(super) struct AnimatedMaterial {
    driver: String,
    fps: f32,
    time_frames: usize,
    state_threshold: f32,
    owner: Option<Entity>,
    frames: Vec<Frame>,
    selected: usize,
}

#[derive(Component, Default)]
pub(crate) struct NativeArtInputs {
    pub script_vectors: [[f32; 4]; 4],
    pub time_offset: f32,
}

#[derive(Component, serde::Deserialize)]
pub(super) struct ScalarAnimation {
    lookup: Vec<f32>,
    sampler: u32,
    speed: f32,
    minimum: f32,
    maximum: f32,
    script_vector: usize,
    script_component: usize,
    #[serde(skip)]
    owner: Option<Entity>,
}

#[derive(Component, serde::Deserialize)]
pub(super) struct UvAnimation {
    translation: [f32; 2],
    time_mode: String,
    #[serde(skip)]
    owner: Option<Entity>,
}

fn validate_animation(surface: &Value) -> Result<(), String> {
    if !surface["animation"].is_null() {
        let animation = &surface["animation"];
        let frames = animation["frames"]
            .as_array()
            .filter(|frames| !frames.is_empty())
            .ok_or("Material animation has no frames")?;
        match animation["driver"].as_str() {
            Some("time" | "staff_charge") => {
                if animation["fps"]
                    .as_f64()
                    .is_none_or(|fps| !fps.is_finite() || fps <= 0.)
                {
                    return Err("Material animation has invalid source frame rate".into());
                }
                if animation["driver"] == "staff_charge" {
                    let count = animation["time_frames"]
                        .as_u64()
                        .filter(|count| *count > 0)
                        .ok_or("Staff animation has no time frame count")?;
                    if count.checked_mul(2) != Some(frames.len() as u64)
                        || animation["state_threshold"].as_f64().is_none()
                    {
                        return Err("Staff material animation has invalid state banks".into());
                    }
                }
            }
            Some("frozen_stone") => {}
            _ => return Err("Unknown source material animation driver".into()),
        }
        for frame in frames {
            super::pbr::textures(&frame["channels"])?;
        }
    }
    if !surface["scalar_animation"].is_null() {
        let animation: ScalarAnimation =
            serde_json::from_value(surface["scalar_animation"].clone())
                .map_err(|error| format!("Invalid scalar material animation: {error}"))?;
        if animation.lookup.is_empty()
            || animation.script_vector >= 4
            || animation.script_component >= 4
            || !animation
                .lookup
                .iter()
                .chain([&animation.speed, &animation.minimum, &animation.maximum])
                .all(|value| value.is_finite())
        {
            return Err("Invalid scalar material animation domain".into());
        }
    }
    if !surface["uv_animation"].is_null() {
        let animation: UvAnimation = serde_json::from_value(surface["uv_animation"].clone())
            .map_err(|error| format!("Invalid UV material animation: {error}"))?;
        if !matches!(
            animation.time_mode.as_str(),
            "elapsed" | "fractional_second"
        ) || !animation.translation.iter().all(|value| value.is_finite())
        {
            return Err("Unsupported UV material animation controls".into());
        }
    }
    Ok(())
}

pub(super) fn convert(
    mut commands: Commands,
    query: Query<
        (
            Entity,
            &MeshMaterial3d<StandardMaterial>,
            &GltfMaterialName,
            &Mesh3d,
            Has<super::DynamicSurface>,
        ),
        (With<PbrReady>, With<SurfaceReady>, Without<StandardReady>),
    >,
    extras: Query<&GltfExtras>,
    parents: Query<&ChildOf>,
    rigs: Query<&crate::presentation::Rig>,
    stones: Query<(), With<crate::staff_puzzles::IceStone>>,
    art_inputs: Query<(), With<NativeArtInputs>>,
    mut library: ResMut<SurfaceLibrary>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    server: Res<AssetServer>,
    mut status: ResMut<OpaqueSurfaceStatus>,
    mut failure: ResMut<crate::assets::LoadFailure>,
    mut next: ResMut<NextState<crate::GameState>>,
) {
    status.pending = 0;
    for (entity, original, name, mesh, dynamic) in &query {
        let Some(base) = materials.get(&original.0).cloned() else {
            status.pending += 1;
            continue;
        };
        let Some(definition) = library.definitions.get_mut(&name.0) else {
            commands.entity(entity).insert(StandardReady);
            status.world += 1;
            continue;
        };
        let mut current = Some(entity);
        let mut owner = None;
        let mut is_static = false;
        while let Some(ancestor) = current {
            if owner.is_none()
                && let Ok(rig) = rigs.get(ancestor)
            {
                owner = Some(rig.owner);
            }
            if owner.is_none() && stones.contains(ancestor) {
                owner = Some(ancestor);
            }
            if owner.is_none() && art_inputs.contains(ancestor) {
                owner = Some(ancestor);
            }
            is_static |= extras
                .get(ancestor)
                .ok()
                .and_then(|v| serde_json::from_str::<Value>(&v.value).ok())
                .is_some_and(|v| v["static_instance"] == true);
            current = parents.get(ancestor).ok().map(ChildOf::parent);
        }
        let animation = &definition.metadata["animation"];
        let live_mesh = dynamic || !definition.metadata["uv_animation"].is_null();
        if definition.frames.is_empty()
            && let Some(source_frames) = animation["frames"].as_array()
        {
            for frame in source_frames {
                let textures = super::pbr::textures(&frame["channels"]).unwrap();
                definition.frames.push(Frame {
                    channels: textures
                        .into_iter()
                        .map(|texture| (texture.role.clone(), super::pbr::load(&server, &texture)))
                        .collect(),
                    factors: frame["standard"].clone(),
                });
            }
        }
        let frames = &definition.frames;
        if let Some((image, error)) = frames
            .iter()
            .flat_map(|frame| frame.channels.values())
            .find_map(|image| match server.get_load_state(image.id()) {
                Some(bevy::asset::LoadState::Failed(error)) => Some((image, error)),
                _ => None,
            })
        {
            failure.0 = format!(
                "Material animation {} texture {:?}: {error}",
                name.0,
                image.path()
            );
            next.set(crate::GameState::LoadError);
            return;
        }
        if frames
            .iter()
            .flat_map(|f| f.channels.values())
            .any(|image| !server.is_loaded_with_dependencies(image.id()))
        {
            status.pending += 1;
            continue;
        }
        if !frames.is_empty()
            || !definition.metadata["scalar_animation"].is_null()
            || !definition.metadata["uv_animation"].is_null()
        {
            commands
                .entity(entity)
                .insert(MeshMaterial3d(materials.add(base.clone())));
        }
        if !definition.metadata["scalar_animation"].is_null() {
            let mut animation: ScalarAnimation =
                serde_json::from_value(definition.metadata["scalar_animation"].clone())
                    .expect("Source-built scalar material animation");
            animation.owner = owner;
            commands.entity(entity).insert(animation);
        }
        if !definition.metadata["uv_animation"].is_null() {
            let mut animation: UvAnimation =
                serde_json::from_value(definition.metadata["uv_animation"].clone())
                    .expect("Source-built UV material animation");
            animation.owner = owner;
            commands.entity(entity).insert(animation);
        }
        if !frames.is_empty() {
            commands.entity(entity).insert(AnimatedMaterial {
                driver: animation["driver"].as_str().unwrap().into(),
                fps: animation["fps"].as_f64().unwrap_or(0.) as f32,
                time_frames: animation["time_frames"].as_u64().unwrap_or(1).max(1) as usize,
                state_threshold: animation["state_threshold"].as_f64().unwrap_or(1.) as f32,
                owner,
                frames: frames.clone(),
                selected: usize::MAX,
            });
        }
        status.static_models += is_static as usize;
        status.world += (!is_static) as usize;
        status.normal += base.normal_map_texture.is_some() as usize;
        status.response_maps += base.metallic_roughness_texture.is_some() as usize;
        status.masked += matches!(base.alpha_mode, AlphaMode::Mask(_)) as usize;
        status.transparent +=
            (!matches!(base.alpha_mode, AlphaMode::Opaque | AlphaMode::Mask(_))) as usize;
        status.water += (definition.metadata["family"] == "water") as usize;
        commands.entity(entity).insert(StandardReady);
        library.prepared_meshes.insert(mesh.0.id());
        if live_mesh {
            library.live_meshes.insert(mesh.0.id());
        }
    }
    status.shared_materials = materials.len();
}

pub(super) fn animate(
    time: Res<MaterialTime>,
    inputs: Query<&NativeArtInputs>,
    server: Res<AssetServer>,
    inventories: Query<&crate::weapons::WeaponInventory>,
    stones: Query<&crate::staff_puzzles::FrozenStone>,
    mut entities: Query<(&mut AnimatedMaterial, &MeshMaterial3d<StandardMaterial>)>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for (mut animation, handle) in &mut entities {
        let count = animation.frames.len();
        let seconds = time.0
            - animation
                .owner
                .and_then(|owner| inputs.get(owner).ok())
                .map_or(0., |input| input.time_offset as f64);
        let phase = |count: usize| {
            (seconds * animation.fps as f64)
                .floor()
                .rem_euclid(count as f64) as usize
        };
        let selected = match animation.driver.as_str() {
            "staff_charge" => {
                let charged = animation
                    .owner
                    .and_then(|owner| inventories.get(owner).ok())
                    .is_some_and(|inventory| {
                        inventory.charge_level as f32 >= animation.state_threshold
                    });
                let phase = phase(animation.time_frames);
                (charged as usize * animation.time_frames + phase).min(count - 1)
            }
            "frozen_stone" => ((animation
                .owner
                .and_then(|owner| stones.get(owner).ok())
                .map_or(0., |stone| stone.amount)
                .clamp(0., 1.)
                * (count - 1) as f32)
                .round() as usize)
                .min(count - 1),
            _ => phase(count),
        };
        if animation.selected == selected {
            continue;
        }
        if animation.frames[selected]
            .channels
            .values()
            .any(|image| !server.is_loaded_with_dependencies(image.id()))
        {
            continue;
        }
        if let Some(mut material) = materials.get_mut(&handle.0) {
            let frame = &animation.frames[selected];
            material.base_color_texture = frame.channels.get("color").cloned();
            material.normal_map_texture = frame.channels.get("normal").cloned();
            material.metallic_roughness_texture = frame.channels.get("response").cloned();
            material.occlusion_texture = frame
                .channels
                .get("occlusion")
                .or_else(|| frame.channels.get("response"))
                .cloned();
            material.emissive_texture = frame.channels.get("emissive").cloned();
            material.specular_tint_texture = frame
                .channels
                .get("reflectance")
                .or_else(|| frame.channels.get("specular_gloss"))
                .cloned();
            apply_factors(&mut material, &frame.factors);
            animation.selected = selected;
        }
    }
}

pub(super) fn animate_uvs(
    time: Res<MaterialTime>,
    inputs: Query<&NativeArtInputs>,
    entities: Query<(&UvAnimation, &MeshMaterial3d<StandardMaterial>)>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for (animation, handle) in &entities {
        let input = animation.owner.and_then(|owner| inputs.get(owner).ok());
        let seconds = time.0 - input.map_or(0., |input| input.time_offset as f64);
        let phase = if animation.time_mode == "fractional_second" {
            seconds - seconds.floor()
        } else {
            seconds
        };
        let uv_transform = bevy::math::Affine2::from_translation(
            Vec2::from_array(animation.translation) * phase as f32,
        );
        if let Some(material) = materials.get(&handle.0)
            && material.uv_transform != uv_transform
            && let Some(mut material) = materials.get_mut(&handle.0)
        {
            material.uv_transform = uv_transform;
        }
    }
}

pub(super) fn animate_scalars(
    time: Res<MaterialTime>,
    inputs: Query<&NativeArtInputs>,
    entities: Query<(&ScalarAnimation, &MeshMaterial3d<StandardMaterial>)>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for (animation, handle) in &entities {
        let count = animation.lookup.len();
        if count == 0 {
            continue;
        }
        let input = animation.owner.and_then(|owner| inputs.get(owner).ok());
        let seconds = time.0 - input.map_or(0., |input| input.time_offset as f64);
        let coordinate = seconds * animation.speed as f64;
        let coordinate = if animation.sampler & 64 != 0 {
            coordinate.clamp(0., 1.)
        } else {
            coordinate.rem_euclid(1.)
        };
        let texel = (coordinate * count as f64 - 0.5) as f32;
        let at = |index: i64| {
            let index = if animation.sampler & 64 != 0 {
                index.clamp(0, count as i64 - 1)
            } else {
                index.rem_euclid(count as i64)
            };
            animation.lookup[index as usize]
        };
        let value = if animation.sampler & 7 == 1 {
            at((texel + 0.5).floor() as i64)
        } else {
            let lower = texel.floor();
            let fraction = texel - lower;
            at(lower as i64) * (1. - fraction) + at(lower as i64 + 1) * fraction
        };
        let script = input
            .and_then(|input| input.script_vectors.get(animation.script_vector))
            .and_then(|vector| vector.get(animation.script_component))
            .copied()
            .unwrap_or(0.);
        let intensity = (animation.minimum + (animation.maximum - animation.minimum) * value)
            * (1. - script).clamp(0., 1.);
        let emission = intensity * intensity;
        let emissive = LinearRgba::rgb(emission, emission, emission);
        if let Some(material) = materials.get(&handle.0)
            && material.emissive != emissive
            && let Some(mut material) = materials.get_mut(&handle.0)
        {
            material.emissive = emissive;
        }
    }
}

#[derive(Component)]
struct QuestLightMaterial {
    brightness: f32,
    color: Vec3,
    handle: AssetId<StandardMaterial>,
}

pub(crate) fn install_quest_lights(app: &mut App) {
    app.add_systems(
        Update,
        quest_lights
            .after(super::SurfacePreparation)
            .after(crate::staff_upgrades::lights::LightPresentation),
    );
}
fn quest_lights(
    mut commands: Commands,
    entities: Query<(
        Entity,
        &crate::staff_upgrades::lights::LightSurface,
        &MeshMaterial3d<StandardMaterial>,
        Option<&QuestLightMaterial>,
    )>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for (entity, surface, current, cloned) in &entities {
        let Some(source) = materials.get(&current.0) else {
            continue;
        };
        let brightness = cloned
            .filter(|c| c.handle == current.0.id())
            .map_or(source.emissive.red.max(1.), |c| c.brightness);
        let color = surface.color * surface.color * brightness;
        if cloned.is_none_or(|c| c.handle != current.0.id()) {
            let mut source = source.clone();
            source.emissive = LinearRgba::rgb(color.x, color.y, color.z);
            let handle = materials.add(source);
            commands.entity(entity).insert((
                MeshMaterial3d(handle.clone()),
                QuestLightMaterial {
                    brightness,
                    color: surface.color,
                    handle: handle.id(),
                },
            ));
        } else if cloned.is_some_and(|c| c.color != surface.color) {
            if let Some(mut material) = materials.get_mut(&current.0) {
                material.emissive = LinearRgba::rgb(color.x, color.y, color.z);
            }
            commands.entity(entity).insert(QuestLightMaterial {
                brightness,
                color: surface.color,
                handle: current.0.id(),
            });
        }
    }
}

#[cfg(feature = "dev-tools")]
pub(crate) fn quest_light_snapshot(world: &mut World) -> Value {
    let materials = world.resource::<Assets<StandardMaterial>>();
    Value::Array(world.iter_entities().filter_map(|entity| {
        let light = entity.get::<QuestLightMaterial>()?;
        let material = materials.get(light.handle)?;
        Some(serde_json::json!({"entity": entity.id().to_bits(), "kind": "standard", "color": light.color.to_array(), "emissive": [material.emissive.red, material.emissive.green, material.emissive.blue]}))
    }).collect())
}

#[cfg(feature = "dev-tools")]
pub(crate) fn weapon_material_snapshot(world: &mut World) -> Value {
    let materials = world.resource::<Assets<StandardMaterial>>();
    let server = world.resource::<AssetServer>();
    let mut first_person = Vec::new();
    let mut models = Vec::new();
    let mut pending = 0;
    for entity in world.iter_entities() {
        let Some(name) = entity.get::<GltfMaterialName>() else {
            continue;
        };
        let mut current = Some(entity.id());
        let mut first = entity.contains::<super::FirstPersonSurface>();
        let mut weapon = first;
        while let Some(ancestor) = current {
            first |= world.get::<crate::presentation::Rig>(ancestor).is_some();
            weapon |= first
                || world
                    .get::<crate::presentation::camo::WeaponMaterialContext>(ancestor)
                    .is_some();
            current = world.get::<ChildOf>(ancestor).map(ChildOf::parent);
        }
        if !weapon {
            continue;
        }
        let material = entity
            .get::<MeshMaterial3d<StandardMaterial>>()
            .and_then(|h| materials.get(&h.0));
        let ready = material.is_some_and(|m| {
            [
                &m.base_color_texture,
                &m.normal_map_texture,
                &m.metallic_roughness_texture,
                &m.emissive_texture,
            ]
            .into_iter()
            .flatten()
            .all(|h| server.is_loaded_with_dependencies(h.id()))
        });
        pending += (!ready) as usize;
        let row = serde_json::json!({"entity":entity.id().to_bits(), "material":name.0, "native":entity.contains::<StandardReady>(), "textures_ready":ready, "lighting_model":"solari_standard", "visible":entity.get::<InheritedVisibility>().is_some_and(|v| v.get())});
        if first {
            first_person.push(row);
        } else {
            models.push(row);
        }
    }
    serde_json::json!({"pending":pending,"first_person":first_person,"world":models})
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn repeated_surface_registration_retains_loaded_frames_and_rejects_conflicts() {
        let mut library = SurfaceLibrary::default();
        let surface = json!({"identity":"test", "presentation":{}, "channels":[]});
        register_prepared(&mut library, &surface, &[]).unwrap();
        library
            .definitions
            .get_mut("test")
            .unwrap()
            .frames
            .push(Frame {
                channels: HashMap::new(),
                factors: json!({}),
            });
        register_prepared(&mut library, &surface, &[]).unwrap();
        assert_eq!(library.definitions["test"].frames.len(), 1);
        let mut conflict = surface;
        conflict["standard"] = json!({"emissive":[1.,0.,0.]});
        assert!(register_prepared(&mut library, &conflict, &[]).is_err());
    }

    #[test]
    fn unsupported_animation_controls_are_rejected_before_playback() {
        for animation in [
            json!({"driver":"unknown", "fps":1., "frames":[{"channels":[]}]}),
            json!({"driver":"time", "fps":0., "frames":[{"channels":[]}]}),
            json!({"driver":"staff_charge", "fps":1., "time_frames":2,
                "state_threshold":1., "frames":[{"channels":[]}]}),
        ] {
            assert!(validate_animation(&json!({"animation":animation})).is_err());
        }
        assert!(
            validate_animation(&json!({"scalar_animation":{"lookup":[],"sampler":0,
            "speed":1.,"minimum":0.,"maximum":1.,"script_vector":0,"script_component":0}}))
            .is_err()
        );
    }

    #[test]
    fn frame_selection_respects_per_object_offsets_without_mutating_shared_materials() {
        use bevy::asset::AssetApp;
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, bevy::asset::AssetPlugin::default()))
            .init_asset::<Image>()
            .init_asset::<StandardMaterial>()
            .insert_resource(MaterialTime(1.))
            .add_systems(Update, (refresh_owners, animate).chain());
        let original = app
            .world_mut()
            .resource_mut::<Assets<StandardMaterial>>()
            .add(StandardMaterial::default());
        let mut objects = Vec::new();
        for offset in [0., 1.] {
            let owner = app
                .world_mut()
                .spawn(NativeArtInputs {
                    time_offset: offset,
                    ..default()
                })
                .id();
            let material = app
                .world_mut()
                .resource_mut::<Assets<StandardMaterial>>()
                .add(StandardMaterial::default());
            let entity = app
                .world_mut()
                .spawn((
                    ChildOf(owner),
                    MeshMaterial3d(material.clone()),
                    AnimatedMaterial {
                        driver: "time".into(),
                        fps: 1.,
                        time_frames: 2,
                        state_threshold: 1.,
                        owner: Some(owner),
                        frames: vec![
                            Frame {
                                channels: HashMap::new(),
                                factors: json!({"emissive":[1.,0.,0.]}),
                            },
                            Frame {
                                channels: HashMap::new(),
                                factors: json!({"emissive":[9.,0.,0.]}),
                            },
                        ],
                        selected: usize::MAX,
                    },
                ))
                .id();
            objects.push((entity, material, owner));
        }
        app.update();
        {
            let materials = app.world().resource::<Assets<StandardMaterial>>();
            assert_eq!(materials.get(&objects[0].1).unwrap().emissive.red, 9.);
            assert_eq!(materials.get(&objects[1].1).unwrap().emissive.red, 1.);
            assert_eq!(materials.get(&original).unwrap().emissive.red, 0.);
        }
        app.world_mut()
            .entity_mut(objects[0].0)
            .insert(ChildOf(objects[1].2));
        app.update();
        let materials = app.world().resource::<Assets<StandardMaterial>>();
        assert_eq!(materials.get(&objects[0].1).unwrap().emissive.red, 1.);
    }
}

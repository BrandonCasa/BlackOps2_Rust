use crate::{GameState, gameplay::SessionEntity};
use bevy::{
    asset::{AssetLoader, LoadContext, io::Reader},
    math::Affine2,
    mesh::VertexAttributeValues,
    prelude::*,
    world_serialization::{WorldAsset, WorldAssetRoot, WorldInstance, WorldInstanceSpawner},
};
use serde::Deserialize;
use std::{
    collections::{BTreeMap, BTreeSet},
    io,
};

#[derive(Clone, Deserialize)]
struct VisualState {
    color: [f32; 4],
    rotation_total: f32,
    size: [f32; 2],
    scale: f32,
}
#[derive(Clone, Deserialize)]
struct StateSample {
    base: VisualState,
    amplitude: VisualState,
}
#[derive(Clone, Deserialize)]
struct VelocityFrame {
    delta: [[f32; 3]; 2],
    velocity: [[f32; 3]; 2],
}
#[derive(Clone, Deserialize)]
struct VelocitySample {
    local: VelocityFrame,
    world: VelocityFrame,
}
#[derive(Clone, Deserialize)]
struct Element {
    #[serde(rename = "type")]
    kind: u8,
    flags: i32,
    looping: bool,
    spawn: [i32; 2],
    delay: [f32; 2],
    life: [f32; 2],
    origin: [[f32; 2]; 3],
    radius: [f32; 2],
    height: [f32; 2],
    angles: [[f32; 2]; 3],
    angular_velocity: [[f32; 2]; 3],
    rotation: [f32; 2],
    gravity: [f32; 2],
    atlas: [u32; 7],
    velocities: Vec<VelocitySample>,
    states: Vec<StateSample>,
    visuals: Vec<String>,
    death: String,
    emitted: String,
    attached: String,
    impact: String,
    sound: String,
    pivot: [f32; 2],
}
#[derive(Clone, Deserialize)]
struct Effect {
    elements: Vec<Element>,
    loop_life: i32,
    oneshot_life: i32,
}
#[derive(Deserialize)]
struct NativeMaterial {
    texture: String,
    sampler: u32,
    presentation: serde_json::Value,
    columns: u32,
    rows: u32,
}
#[derive(Deserialize)]
struct Manifest {
    effects: BTreeMap<String, Effect>,
    materials: BTreeMap<String, NativeMaterial>,
    models: BTreeMap<String, String>,
}
struct FxMaterial {
    texture: Handle<Image>,
    blend: AlphaMode,
    columns: u32,
    rows: u32,
}
#[derive(Asset, TypePath)]
pub(crate) struct NativeFxPack {
    effects: BTreeMap<String, Effect>,
    materials: BTreeMap<String, FxMaterial>,
    #[dependency]
    textures: Vec<Handle<Image>>,
    #[dependency]
    models: BTreeMap<String, Handle<WorldAsset>>,
}
#[derive(Default, TypePath)]
struct NativeFxLoader;
impl AssetLoader for NativeFxLoader {
    type Asset = NativeFxPack;
    type Settings = ();
    type Error = io::Error;
    async fn load(
        &self,
        reader: &mut dyn Reader,
        _: &(),
        context: &mut LoadContext<'_>,
    ) -> Result<NativeFxPack, io::Error> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        let manifest: Manifest = serde_json::from_slice(&bytes).map_err(io::Error::other)?;
        let mut textures = Vec::new();
        let mut materials = BTreeMap::new();
        for (name, material) in manifest.materials {
            let native = material.sampler;
            let texture = context
                .load_builder()
                .with_settings(move |settings: &mut bevy::image::ImageLoaderSettings| {
                    use bevy::image::{
                        ImageAddressMode, ImageFilterMode, ImageSampler, ImageSamplerDescriptor,
                    };
                    settings.is_srgb = true;
                    let point = native & 7 == 1;
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
                        mipmap_filter: if !point && native & 24 == 16 {
                            ImageFilterMode::Linear
                        } else {
                            ImageFilterMode::Nearest
                        },
                        lod_max_clamp: if native & 24 == 0 { 0. } else { 32. },
                        ..ImageSamplerDescriptor::linear()
                    });
                })
                .load(material.texture);
            textures.push(texture.clone());
            let blend = match material.presentation["alpha"].as_str() {
                Some("opaque") => AlphaMode::Opaque,
                Some("mask") => AlphaMode::Mask(
                    material.presentation["cutoff"]
                        .as_f64()
                        .ok_or_else(|| io::Error::other("Prepared FX alpha cutoff is missing"))?
                        as f32,
                ),
                Some("add") => AlphaMode::Add,
                Some("multiply") => AlphaMode::Multiply,
                Some("premultiplied") => AlphaMode::Premultiplied,
                Some("blend") => AlphaMode::Blend,
                _ => return Err(io::Error::other("Prepared FX blend state is invalid")),
            };
            if material.columns == 0 || material.rows == 0 {
                return Err(io::Error::other("Prepared FX atlas dimensions are invalid"));
            }
            materials.insert(
                name,
                FxMaterial {
                    texture,
                    blend,
                    columns: material.columns,
                    rows: material.rows,
                },
            );
        }
        Ok(NativeFxPack {
            effects: manifest.effects,
            materials,
            textures,
            models: manifest
                .models
                .into_iter()
                .map(|(name, path)| (name, context.load(path)))
                .collect(),
        })
    }
    fn extensions(&self) -> &[&str] {
        &["nativefx"]
    }
}
#[derive(Resource)]
pub(crate) struct NativeFxAssets(pub Handle<NativeFxPack>);
#[derive(Resource, Default)]
struct Diagnostics(BTreeSet<String>);
#[derive(Component)]
struct Emitter {
    name: String,
    owner: Option<Entity>,
    offset: Transform,
    elapsed: f32,
    next: Vec<f32>,
    counts: Vec<u32>,
    limits: Vec<u32>,
    depth: u8,
    random: u64,
}
#[derive(Component)]
struct FxParent(Entity);
#[derive(Component)]
struct Particle {
    element: Element,
    pose: Transform,
    age: f32,
    fresh: bool,
    life: f32,
    random: f32,
    rotation: f32,
    material: Option<Handle<StandardMaterial>>,
    columns: u32,
    rows: u32,
    atlas_start: u32,
    depth: u8,
}
pub(crate) struct NativeFxPlugin;
impl Plugin for NativeFxPlugin {
    fn build(&self, app: &mut App) {
        app.init_asset::<NativeFxPack>()
            .init_asset_loader::<NativeFxLoader>()
            .init_resource::<Diagnostics>()
            .add_systems(
                OnEnter(GameState::Loading),
                load.after(crate::assets::begin_load),
            )
            .add_systems(
                Update,
                (spawn, animate)
                    .chain()
                    .before(crate::audio::AudioPlayback)
                    .run_if(in_state(GameState::Playing)),
            );
    }
}
fn load(
    mut commands: Commands,
    server: Res<AssetServer>,
    existing: Option<Res<NativeFxAssets>>,
    packs: Res<Assets<NativeFxPack>>,
    art: Option<Res<crate::assets::prepared::PreparedArt>>,
    mut failure: ResMut<crate::assets::LoadFailure>,
    mut next: ResMut<NextState<GameState>>,
) {
    let Some(art) = art else { return };
    let Some(uri) = art.effects_uri() else {
        failure.0 = "Prepared native effects manifest is missing".into();
        next.set(GameState::LoadError);
        return;
    };
    if let Some(existing) = existing {
        if matches!(
            server.get_load_state(existing.0.id()),
            Some(bevy::asset::LoadState::Failed(_))
        ) {
            server.reload(uri.to_owned());
        }
        if let Some(pack) = packs.get(&existing.0) {
            for texture in &pack.textures {
                if matches!(
                    server.get_load_state(texture.id()),
                    Some(bevy::asset::LoadState::Failed(_))
                ) && let Some(path) = server.get_path(texture.id())
                {
                    server.reload(path);
                }
            }
            for model in pack.models.values() {
                if matches!(
                    server.get_recursive_dependency_load_state(model.id()),
                    Some(bevy::asset::RecursiveDependencyLoadState::Failed(_))
                ) && let Some(path) = server.get_path(model.id())
                {
                    server.reload(path);
                }
            }
        }
    } else {
        commands.insert_resource(NativeFxAssets(server.load(uri.to_owned())));
    }
}
pub(crate) fn emit(
    commands: &mut Commands,
    name: impl Into<String>,
    transform: Transform,
) -> Entity {
    create(commands, name.into(), None, transform, 0)
}
pub(crate) fn attach(
    commands: &mut Commands,
    name: impl Into<String>,
    owner: Entity,
    offset: Transform,
) -> Entity {
    create(commands, name.into(), Some(owner), offset, 0)
}
fn create(
    commands: &mut Commands,
    name: String,
    owner: Option<Entity>,
    offset: Transform,
    depth: u8,
) -> Entity {
    let entity = commands.spawn_empty().id();
    commands.entity(entity).insert((
        SessionEntity,
        Name::new(format!("Native FX {name}")),
        offset,
        Emitter {
            name,
            owner,
            offset,
            elapsed: 0.,
            next: Vec::new(),
            counts: Vec::new(),
            limits: Vec::new(),
            depth,
            random: entity.to_bits().wrapping_add(1),
        },
    ));
    entity
}
fn native(v: [f32; 3]) -> Vec3 {
    Vec3::new(v[0], v[2], -v[1]) * 0.0254
}
fn random(seed: &mut u64) -> f32 {
    *seed = seed
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    (*seed >> 40) as f32 / 16777216.
}
fn range(value: [f32; 2], random: f32) -> f32 {
    value[0] + value[1] * random
}
fn diagnose(diagnostics: &mut Diagnostics, key: String, message: &str) {
    if diagnostics.0.insert(key.clone()) {
        warn!(effect = key, "Native FX: {message}");
    }
}
fn play_sound(
    commands: &mut Commands,
    game: &crate::assets::GameDefinition,
    diagnostics: &mut Diagnostics,
    effect: &str,
    alias: &str,
    position: Vec3,
) {
    if crate::audio::resolve_alias(game, alias).is_some() {
        crate::audio::emit(commands, alias, crate::audio::SoundSource::World(position));
    } else {
        diagnose(
            diagnostics,
            format!("{effect}:audio:{alias}"),
            "original effect sound alias is absent from the recovered native soundbanks",
        );
    }
}
fn spawn(
    mut commands: Commands,
    time: Res<Time>,
    handle: Res<NativeFxAssets>,
    packs: Res<Assets<NativeFxPack>>,
    game: Res<crate::assets::GameDefinition>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut diagnostics: ResMut<Diagnostics>,
    mut emitters: Query<(Entity, &mut Emitter, &mut Transform, Option<&FxParent>)>,
    parents: Query<&FxParent>,
    live: Query<(), With<Emitter>>,
    transforms: Query<&GlobalTransform>,
) {
    let Some(pack) = packs.get(&handle.0) else {
        return;
    };
    let mut owned = BTreeSet::new();
    for parent in &parents {
        owned.insert(parent.0);
    }
    for (entity, mut emitter, mut pose, parent) in &mut emitters {
        if parent.is_some_and(|parent| !live.contains(parent.0)) {
            commands.entity(entity).try_despawn();
            continue;
        }
        if let Some(owner) = emitter.owner {
            let Ok(owner) = transforms.get(owner) else {
                commands.entity(entity).try_despawn();
                continue;
            };
            *pose = owner.mul_transform(emitter.offset).compute_transform();
        }
        let Some(effect) = pack.effects.get(emitter.name.trim_start_matches(',')) else {
            diagnose(
                &mut diagnostics,
                emitter.name.clone(),
                "original definition is absent",
            );
            commands.entity(entity).try_despawn();
            continue;
        };
        if emitter.depth > 12 {
            diagnose(
                &mut diagnostics,
                emitter.name.clone(),
                "nested runner depth exceeds 12",
            );
            commands.entity(entity).try_despawn();
            continue;
        }
        if emitter.next.is_empty() {
            for element in &effect.elements {
                let r = random(&mut emitter.random);
                emitter.next.push(range(element.delay, r).max(0.) * 0.001);
                emitter.counts.push(0);
                let limit = if element.looping {
                    element.spawn[1].max(0) as u32
                } else {
                    let amplitude = (random(&mut emitter.random)
                        * (element.spawn[1].max(0) as f32 + 1.))
                        .floor();
                    (element.spawn[0] as f32 + amplitude).max(0.) as u32
                };
                emitter.limits.push(limit);
            }
        }
        emitter.elapsed += time.delta_secs();
        let mut active = false;
        for (index, element) in effect.elements.iter().enumerate() {
            let looping_life = effect.loop_life.max(0) as f32 * 0.001;
            let permitted = !element.looping || emitter.elapsed <= looping_life;
            let limit = emitter.limits[index];
            if !permitted || emitter.counts[index] >= limit {
                continue;
            }
            active = true;
            let mut frame_count = 0;
            while emitter.next[index] <= emitter.elapsed
                && emitter.counts[index] < limit
                && frame_count < 64
            {
                emitter.counts[index] += 1;
                frame_count += 1;
                emitter.next[index] += if element.looping {
                    element.spawn[0].max(1) as f32 * 0.001
                } else {
                    0.
                };
                let r = random(&mut emitter.random);
                let mut particle_pose = *pose;
                let position = native(std::array::from_fn(|i| {
                    range(element.origin[i], random(&mut emitter.random))
                }));
                particle_pose.translation += pose.rotation * position;
                let angle = std::array::from_fn::<_, 3, _>(|i| {
                    range(element.angles[i], random(&mut emitter.random))
                });
                particle_pose.rotation *=
                    Quat::from_euler(EulerRot::YXZ, -angle[1], angle[0], -angle[2]);
                if element.flags & 0x30 != 0 {
                    let theta = random(&mut emitter.random) * std::f32::consts::TAU;
                    let radius = range(element.radius, random(&mut emitter.random));
                    let height = range(element.height, random(&mut emitter.random));
                    particle_pose.translation += pose.rotation
                        * native([radius * theta.cos(), radius * theta.sin(), height]);
                    diagnose(
                        &mut diagnostics,
                        format!("{}:offset", emitter.name),
                        "native cylindrical/spherical offset uses a reconstructed distribution",
                    );
                }
                let visual = element
                    .visuals
                    .get((random(&mut emitter.random) * element.visuals.len() as f32) as usize);
                if !element.sound.is_empty() {
                    play_sound(
                        &mut commands,
                        &game,
                        &mut diagnostics,
                        &emitter.name,
                        &element.sound,
                        particle_pose.translation,
                    );
                }
                if element.kind == 12 {
                    if let Some(visual) = visual {
                        let child = create(
                            &mut commands,
                            visual.trim_start_matches(',').into(),
                            None,
                            particle_pose,
                            emitter.depth + 1,
                        );
                        commands.entity(child).insert(FxParent(entity));
                    }
                    continue;
                }
                if element.kind == 10 {
                    if let Some(visual) = visual {
                        play_sound(
                            &mut commands,
                            &game,
                            &mut diagnostics,
                            &emitter.name,
                            visual,
                            particle_pose.translation,
                        );
                    }
                    continue;
                }
                if element.kind == 9 || element.kind == 11 {
                    diagnose(
                        &mut diagnostics,
                        format!("{}:type{}", emitter.name, element.kind),
                        "native spot-light or projected-decal element requires additional native light/projection data and is not rendered",
                    );
                    continue;
                }
                if element.kind == 5 || element.kind == 6 {
                    diagnose(
                        &mut diagnostics,
                        format!("{}:type{}", emitter.name, element.kind),
                        "native trail/cloud geometry is approximated with its original textured visual and sampled curves",
                    );
                }
                if !element.impact.is_empty()
                    || !element.emitted.is_empty()
                    || !element.attached.is_empty()
                {
                    diagnose(
                        &mut diagnostics,
                        format!("{}:secondary", emitter.name),
                        "impact collision and continuous emitted/attached subeffects require native emission-distance interpretation",
                    );
                }
                let life = range(element.life, random(&mut emitter.random)).max(1.) * 0.001;
                let mut particle = Particle {
                    element: element.clone(),
                    pose: particle_pose,
                    age: 0.,
                    fresh: true,
                    life,
                    random: r,
                    rotation: range(element.rotation, r),
                    material: None,
                    columns: 1,
                    rows: 1,
                    atlas_start: element.atlas[1],
                    depth: emitter.depth,
                };
                if element.kind == 8 {
                    commands.spawn((
                        SessionEntity,
                        FxParent(entity),
                        particle,
                        particle_pose,
                        PointLight {
                            shadow_maps_enabled: false,
                            ..default()
                        },
                    ));
                } else if element.kind == 7 {
                    if let Some(model) =
                        visual.and_then(|name| pack.models.get(name.trim_start_matches(',')))
                    {
                        commands.spawn((
                            SessionEntity,
                            FxParent(entity),
                            particle,
                            particle_pose,
                            WorldAssetRoot(model.clone()),
                        ));
                    } else {
                        diagnose(
                            &mut diagnostics,
                            format!("{}:model", emitter.name),
                            "original model scene is absent",
                        );
                    }
                } else if let Some(source) = visual.and_then(|name| pack.materials.get(name)) {
                    particle.columns = source.columns;
                    particle.rows = source.rows;
                    if element.atlas[0] & 1 != 0 {
                        particle.atlas_start = (r * (source.columns * source.rows) as f32) as u32;
                    }
                    let material = materials.add(StandardMaterial {
                        base_color_texture: Some(source.texture.clone()),
                        alpha_mode: source.blend,
                        unlit: true,
                        cull_mode: None,
                        ..default()
                    });
                    particle.material = Some(material.clone());
                    let mut mesh = Mesh::from(Rectangle::new(1., 1.));
                    if let Some(VertexAttributeValues::Float32x3(vertices)) =
                        mesh.attribute_mut(Mesh::ATTRIBUTE_POSITION)
                    {
                        for vertex in vertices {
                            vertex[0] -= element.pivot[0];
                            vertex[1] -= element.pivot[1];
                        }
                    }
                    commands.spawn((
                        SessionEntity,
                        FxParent(entity),
                        particle,
                        particle_pose,
                        Mesh3d(meshes.add(mesh)),
                        MeshMaterial3d(material),
                        bevy::light::NotShadowCaster,
                        bevy::light::NotShadowReceiver,
                    ));
                }
            }
        }
        if !active
            && emitter.elapsed >= effect.oneshot_life.max(0) as f32 * 0.001
            && !owned.contains(&entity)
        {
            commands.entity(entity).try_despawn();
        }
    }
}
fn sample_indices(length: usize, age: f32) -> (usize, usize, f32) {
    let coordinate = age.clamp(0., 1.) * length.saturating_sub(1) as f32;
    let index = coordinate.floor() as usize;
    (
        index,
        (index + 1).min(length.saturating_sub(1)),
        coordinate.fract(),
    )
}
fn delta(frame: &VelocityFrame, random: f32) -> Vec3 {
    native(std::array::from_fn(|i| {
        frame.delta[0][i] + random * frame.delta[1][i]
    }))
}
fn velocity(frame: &VelocityFrame, random: f32) -> Vec3 {
    native(std::array::from_fn(|i| {
        frame.velocity[0][i] + random * frame.velocity[1][i]
    }))
}
fn visual(sample: &StateSample, random: f32) -> ([f32; 4], Vec2, f32, f32) {
    (
        std::array::from_fn(|i| {
            sample.base.color[i] + random * (sample.amplitude.color[i] - sample.base.color[i])
        }),
        Vec2::from_array(std::array::from_fn(|i| {
            sample.base.size[i] + random * sample.amplitude.size[i]
        })),
        sample.base.rotation_total + random * sample.amplitude.rotation_total,
        sample.base.scale + random * sample.amplitude.scale,
    )
}
fn retire_particle(
    commands: &mut Commands,
    spawner: &mut WorldInstanceSpawner,
    entity: Entity,
    instance: Option<&WorldInstance>,
) {
    if let Some(instance) = instance {
        spawner.unregister_instance(**instance);
    }
    commands
        .entity(entity)
        .try_remove::<WorldAssetRoot>()
        .try_despawn();
}
fn animate(
    mut commands: Commands,
    time: Res<Time>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut spawner: ResMut<WorldInstanceSpawner>,
    cameras: Query<&GlobalTransform, With<crate::rendering::GameCamera>>,
    emitters: Query<(), With<Emitter>>,
    mut particles: Query<(
        Entity,
        &FxParent,
        &mut Particle,
        &mut Transform,
        Option<&mut PointLight>,
        Option<&WorldInstance>,
    )>,
) {
    let camera = cameras
        .iter()
        .next()
        .map(GlobalTransform::compute_transform);
    for (entity, parent, mut particle, mut pose, light, instance) in &mut particles {
        if !emitters.contains(parent.0) {
            if let Some(material) = &particle.material {
                materials.remove(material.id());
            }
            retire_particle(&mut commands, &mut spawner, entity, instance);
            continue;
        }
        let ready = particle.element.kind != 7
            || instance.is_some_and(|instance| spawner.instance_is_ready(**instance));
        if ready {
            if particle.fresh {
                particle.fresh = false;
            } else {
                particle.age += time.delta_secs();
            }
        }
        if particle.age >= particle.life {
            if !particle.element.death.is_empty() {
                let child = create(
                    &mut commands,
                    particle.element.death.trim_start_matches(',').into(),
                    None,
                    *pose,
                    particle.depth + 1,
                );
                commands.entity(child).insert(FxParent(parent.0));
            }
            if let Some(material) = &particle.material {
                materials.remove(material.id());
            }
            retire_particle(&mut commands, &mut spawner, entity, instance);
            continue;
        }
        let normalized = particle.age / particle.life;
        let element = &particle.element;
        let mut direction = Vec3::Y;
        let mut displacement = Vec3::ZERO;
        if !element.velocities.is_empty() {
            let (a, b, t) = sample_indices(element.velocities.len(), normalized);
            let a = &element.velocities[a];
            let b = &element.velocities[b];
            displacement = particle.pose.rotation
                * delta(&a.local, particle.random).lerp(delta(&b.local, particle.random), t)
                + delta(&a.world, particle.random).lerp(delta(&b.world, particle.random), t);
            direction = particle.pose.rotation
                * velocity(&a.local, particle.random).lerp(velocity(&b.local, particle.random), t)
                + velocity(&a.world, particle.random).lerp(velocity(&b.world, particle.random), t);
            displacement *= particle.life * 1000.;
        }
        pose.translation = particle.pose.translation + displacement;
        pose.translation.y -=
            range(element.gravity, particle.random) * particle.age * particle.age * 9.81 * 0.5;
        let mut size = Vec2::ONE;
        let mut color = [255.; 4];
        let mut rotation = particle.rotation;
        let mut scale = 1.;
        if !element.states.is_empty() {
            let (a, b, t) = sample_indices(element.states.len(), normalized);
            let a = visual(&element.states[a], particle.random);
            let b = visual(&element.states[b], particle.random);
            color = std::array::from_fn(|i| a.0[i] + t * (b.0[i] - a.0[i]));
            size = a.1.lerp(b.1, t);
            rotation += (a.2 + t * (b.2 - a.2)) * particle.life * 1000.;
            scale = a.3 + t * (b.3 - a.3);
        }
        pose.scale = particle.pose.scale
            * Vec3::new(
                size.x.abs().max(0.001) * 0.0508,
                size.y.abs().max(0.001) * 0.0508,
                1.,
            );
        if element.kind == 7 {
            pose.scale = particle.pose.scale * scale.max(0.001);
        }
        let angular = native(std::array::from_fn(|i| {
            range(element.angular_velocity[i], particle.random)
        })) / 0.0254
            * particle.age
            * 1000.;
        if matches!(element.kind, 1 | 2 | 7) {
            pose.rotation = particle.pose.rotation
                * Quat::from_euler(EulerRot::YXZ, angular.y, angular.x, angular.z + rotation);
        } else if let Some(camera) = camera {
            pose.rotation = camera.rotation * Quat::from_rotation_z(rotation);
            if matches!(element.kind, 3 | 4 | 5) && direction.length_squared() > 0.000001 {
                let y = direction.normalize();
                let view = (camera.translation - pose.translation).normalize_or_zero();
                let x = y.cross(view).normalize_or_zero();
                if x.length_squared() > 0.5 {
                    pose.rotation = Quat::from_mat3(&Mat3::from_cols(x, y, x.cross(y)));
                }
            }
        }
        if let Some(mut light) = light {
            light.color = Color::srgb(color[0] / 255., color[1] / 255., color[2] / 255.);
            light.intensity = color[3] / 255. * 1000.;
            light.range = size.x.abs() * 0.0254;
            light.radius = scale.abs() * 0.0254;
        }
        if let Some(mut material) = particle
            .material
            .as_ref()
            .and_then(|h| materials.get_mut(h))
        {
            material.base_color = Color::srgba(
                color[0] / 255.,
                color[1] / 255.,
                color[2] / 255.,
                color[3] / 255.,
            );
            let count = particle.columns * particle.rows;
            let frame = if element.atlas[2] > 0 {
                (particle.age * element.atlas[2] as f32) as u32
            } else {
                (normalized * count as f32) as u32
            };
            let frame = if element.atlas[3] == 0 {
                frame.min(count - 1)
            } else {
                frame % count
            };
            let frame = (particle.atlas_start + frame) % count;
            material.uv_transform = Affine2::from_scale_angle_translation(
                Vec2::new(1. / particle.columns as f32, 1. / particle.rows as f32),
                0.,
                Vec2::new(
                    (frame % particle.columns) as f32 / particle.columns as f32,
                    (frame / particle.columns) as f32 / particle.rows as f32,
                ),
            );
        }
    }
}

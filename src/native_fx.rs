//! Origins source-authored effect timelines and original native draw inputs.
mod cloud;
pub(crate) mod decal;
mod geometry;
mod impact;
pub(crate) mod lighting;
mod material;
#[cfg(test)]
mod tests;
mod visuals;
use crate::rendering::native_effect::{
    NativeCloudObject, NativeEffectDraw, NativeEffectGeometry, NativeEffectMaterial,
    NativeEffectProgram, NativeEffectTransform, NativeOutdoorMap,
};
use crate::{GameState, assets::GameDefinition, gameplay::SessionEntity};
use bevy::{asset::RecursiveDependencyLoadState, math::Affine2, prelude::*};
use serde::Deserialize;
use std::collections::{BTreeMap, HashSet};
#[derive(Clone, Copy, Deserialize)]
struct Range {
    base: f32,
    amplitude: f32,
}
impl Range {
    fn sample(self, random: f32) -> f32 {
        self.base + self.amplitude * random
    }
}
#[derive(Clone, Deserialize)]
struct VectorRange {
    base: [f32; 3],
    amplitude: [f32; 3],
}
impl VectorRange {
    fn sample(&self, seed: u64, catalog: &Catalog) -> Vec3 {
        native(std::array::from_fn(|i| {
            self.base[i] + self.amplitude[i] * channel(catalog, seed, i as i32 - 6)
        }))
    }
}
#[derive(Clone, Deserialize)]
struct VelocityFrame {
    total_delta: VectorRange,
    velocity: VectorRange,
}
#[derive(Clone, Deserialize)]
struct VelocitySample {
    local: VelocityFrame,
    world: VelocityFrame,
}
#[derive(Clone, Deserialize)]
struct VisualState {
    color_rgba8: [f32; 4],
    rotation_total: f32,
    size: [f32; 2],
    scale: f32,
}
#[derive(Clone, Deserialize)]
struct VisualSample {
    base: VisualState,
    amplitude: VisualState,
}
#[derive(Clone, Deserialize)]
struct Atlas {
    behavior: u32,
    column_bits: u32,
    row_bits: u32,
    fps: u32,
    index: u32,
    loop_count: u32,
    entry_count_and_range: u32,
}
#[derive(Clone, Copy, Deserialize)]
#[serde(untagged)]
enum Spawn {
    Once { count: Range },
    Loop { count: u32, interval_ms: u32 },
}
#[derive(Clone, Deserialize)]
struct Visual {
    material: Option<String>,
    model: Option<String>,
    effect: Option<String>,
    sound: Option<String>,
    decal_materials: Option<[String; 2]>,
}
#[derive(Clone, Deserialize)]
struct TrailVertex {
    normal: [f32; 2],
    position: [f32; 2],
    texcoord: f32,
}
#[derive(Clone, Deserialize)]
struct Trail {
    vertices: Vec<TrailVertex>,
    indices: Vec<u32>,
    split_distance: f32,
    repeat_distance: f32,
    scroll_ms: f32,
}
#[derive(Clone, Copy, Deserialize)]
struct BillboardTrim {
    bottom_width: f32,
    top_width: f32,
}
impl BillboardTrim {
    fn widths(self) -> [f32; 2] {
        [self.bottom_width, self.top_width]
    }
}
#[derive(Clone, Deserialize)]
struct Element {
    #[serde(rename = "type")]
    kind: u8,
    flags: i32,
    spawn: Spawn,
    delay_ms: Range,
    fade_in_range: Range,
    fade_out_range: Range,
    life_ms: Range,
    origin: [Range; 3],
    angles: [Range; 3],
    angular_velocity: [Range; 3],
    initial_rotation: Range,
    gravity: Range,
    offset_radius: Range,
    offset_height: Range,
    atlas: Atlas,
    velocity_samples: Vec<VelocitySample>,
    visual_samples: Vec<VisualSample>,
    visuals: Vec<Option<Visual>>,
    effect_attached: Option<String>,
    effect_emitted: Option<String>,
    effect_on_death: Option<String>,
    effect_on_impact: Option<String>,
    spawn_sound: Option<String>,
    emit_distance: Range,
    emit_distance_variance: Range,
    billboard_pivot: [f32; 2],
    billboard_trim: Option<BillboardTrim>,
    lighting_fraction: i8,
    alpha_fade_ms: u16,
    trail: Option<Trail>,
    cloud_density: Option<Range>,
}
#[derive(Deserialize)]
struct SourceEffect {
    flags: u16,
    elements: Vec<Element>,
    looping_count: usize,
    looping_life_ms: f32,
    non_looping_life_ms: f32,
}
#[derive(Deserialize)]
struct SourceMaterial {}
#[derive(Resource, Deserialize)]
pub(crate) struct Catalog {
    effects: BTreeMap<String, SourceEffect>,
    materials: BTreeMap<String, SourceMaterial>,
    random_table: Vec<f32>,
    #[serde(flatten)]
    impacts: impact::ImpactEffects,
}
#[derive(Component)]
struct Emitter {
    name: String,
    elapsed: f32,
    next: Vec<f32>,
    counts: Vec<u32>,
    limits: Vec<u32>,
    seed: u64,
    previous: Option<Transform>,
    lighting: Vec3,
    trail_points: Vec<Option<Vec3>>,
    trail_distance: Vec<f32>,
}
#[derive(Component)]
struct Owned(Entity);
#[derive(Component)]
struct AttachedSource(Entity);
#[derive(Component)]
struct EffectParent(Entity);
#[derive(Component)]
struct Stopped;
#[derive(Component)]
struct Particle {
    element: usize,
    initial: Transform,
    age: f32,
    life: f32,
    random: f32,
    seed: u64,
    trail_start: Option<Vec3>,
    rotation: f32,
    image: Option<Handle<Image>>,
    scene: Option<Handle<bevy::world_serialization::WorldAsset>>,
    atlas_start: u32,
    emitted_at: Vec3,
    emit_step: f32,
    cloud: Option<cloud::State>,
}
pub(crate) struct NativeFxPlugin;
impl Plugin for NativeFxPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            OnEnter(GameState::Loading),
            load_catalog.after(crate::assets::begin_load),
        );
        app.add_systems(
            Update,
            (follow_source_attachments, spawn, animate, decal::fade)
                .chain()
                .run_if(in_state(GameState::Playing)),
        );
        app.add_systems(
            PostUpdate,
            object_constants
                .after(bevy::transform::TransformSystems::Propagate)
                .after(bevy::camera::CameraUpdateSystems),
        );
    }
}
fn load_catalog(
    mut commands: Commands,
    options: Res<crate::assets::LaunchOptions>,
    server: Res<AssetServer>,
    mut materials: ResMut<Assets<NativeEffectMaterial>>,
    mut geometries: ResMut<Assets<NativeEffectGeometry>>,
) {
    let catalog: Catalog = crate::origins::read(&options.asset_root.join("fx_catalog.json"));
    catalog.impacts.validate();
    commands.insert_resource(decal::Materials::load(&options.asset_root, &catalog));
    commands.insert_resource(material::Materials::load(
        &options.asset_root,
        catalog.materials.keys(),
        &server,
        &mut materials,
    ));
    commands.insert_resource(catalog);
    commands.insert_resource(cloud::geometry(&mut geometries));
}

///004fa310 ->005c6f00(flag0) ->005c6ba0 ->005c61c0 stops this effect
/// and its registered child effects.005c5740 clears the active bit; unlike
///005c6310's kill path, it leaves existing particle lists to finish naturally.
pub(crate) fn stop(commands: &mut Commands, entity: Entity) {
    commands.queue(move |world: &mut World| {
        let mut descendants: BTreeMap<Entity, Vec<Entity>> = BTreeMap::new();
        let mut query = world.query::<(Entity, &EffectParent)>();
        for (child, parent) in query.iter(world) {
            descendants.entry(parent.0).or_default().push(child);
        }
        let mut pending = vec![entity];
        while let Some(effect) = pending.pop() {
            if let Ok(mut entity) = world.get_entity_mut(effect) {
                if entity.contains::<Emitter>() {
                    entity.insert(Stopped);
                }
            }
            if let Some(children) = descendants.remove(&effect) {
                pending.extend(children);
            }
        }
    });
}

fn emit_child(
    commands: &mut Commands,
    name: impl Into<String>,
    pose: Transform,
    parent: Entity,
) -> Entity {
    let child = emit(commands, name, pose);
    commands.entity(child).insert(EffectParent(parent));
    child
}
pub(crate) fn emit(
    commands: &mut Commands,
    name: impl Into<String>,
    transform: Transform,
) -> Entity {
    let entity = commands
        .spawn((SessionEntity, transform, Visibility::default()))
        .id();
    commands.entity(entity).insert(Emitter {
        name: name.into().trim_start_matches(',').to_owned(),
        elapsed: 0.,
        next: vec![],
        counts: vec![],
        limits: vec![],
        seed: entity.to_bits().wrapping_add(1),
        previous: None,
        lighting: Vec3::ZERO,
        trail_points: vec![],
        trail_distance: vec![],
    });
    entity
}
pub(crate) fn attach(
    commands: &mut Commands,
    name: impl Into<String>,
    owner: Entity,
    offset: Transform,
) -> Entity {
    let entity = emit(commands, name, offset);
    commands.entity(owner).add_child(entity);
    entity
}
fn native(v: [f32; 3]) -> Vec3 {
    Vec3::new(v[0], v[2], -v[1]) * 0.0254
}
// Recovered native table channels start at 00b3ed08; the catalog
// includes the six preceding velocity/angular channels (00b3ecf0).
fn channel(catalog: &Catalog, seed: u64, offset: i32) -> f32 {
    catalog.random_table[(seed as i32 + 6 + offset) as usize]
}
fn integer_range(range: Range, catalog: &Catalog, seed: u64, offset: i32) -> f32 {
    let bits = channel(catalog, seed, offset).to_bits() & 0xffff;
    range.base + ((bits * (range.amplitude as u32 + 1)) >> 16) as f32
}

// Source attached FX retain their initial orientation and only follow xyz.
fn follow_source_attachments(
    mut commands: Commands,
    owners: Query<&Transform, (With<Particle>, Without<Emitter>)>,
    mut attached: Query<
        (Entity, &AttachedSource, &mut Transform),
        (With<Emitter>, Without<Particle>),
    >,
) {
    for (entity, source, mut pose) in &mut attached {
        if let Ok(owner) = owners.get(source.0) {
            pose.translation = owner.translation;
        } else {
            stop(&mut commands, entity);
        }
    }
}
fn spawn(
    mut commands: Commands,
    time: Res<Time>,
    catalog: Option<Res<Catalog>>,
    server: Res<AssetServer>,
    game: Res<GameDefinition>,
    materials: Res<material::Materials>,
    native_materials: Res<Assets<NativeEffectMaterial>>,
    cloud_geometry: Res<cloud::Geometry>,
    outdoor: Option<Res<NativeOutdoorMap>>,
    lighting: (
        Res<crate::rendering::model_lighting::ModelLightingSource>,
        Res<lighting::Source>,
        Res<crate::environment::weather::Weather>,
    ),
    mut geometries: ResMut<Assets<NativeEffectGeometry>>,
    mut decals: (
        Option<Res<decal::World>>,
        ResMut<decal::Materials>,
        ResMut<Assets<crate::rendering::native::NativeWorldMaterial>>,
        ResMut<Assets<crate::rendering::native::NativeWorldGeometry>>,
        Query<&GlobalTransform>,
    ),
    mut emitters: Query<(
        Entity,
        &mut Emitter,
        &Transform,
        Has<ChildOf>,
        Option<&Stopped>,
    )>,
    transforms: bevy::transform::helper::TransformHelper,
    particles: Query<&Owned>,
    mut occupied: Local<HashSet<Entity>>,
) {
    let Some(catalog) = catalog else {
        return;
    };
    occupied.clear();
    occupied.extend(particles.iter().map(|owner| owner.0));
    for (entity, mut emitter, local, parented, stopped) in &mut emitters {
        if stopped.is_some() {
            if !occupied.contains(&entity) {
                commands.entity(entity).despawn();
            }
            continue;
        }
        let Some(effect) = catalog.effects.get(&emitter.name) else {
            panic!(
                "{}",
                format!(
                    "Required source effect definition missing: {}",
                    emitter.name
                ),
            );
        };
        // Update runs before transform propagation. New explosions and runners
        // already own their world pose; attached FX need the current hierarchy.
        let parent = if parented {
            transforms
                .compute_global_transform(entity)
                .expect("source FX transform hierarchy")
                .compute_transform()
        } else {
            *local
        };
        if emitter.next.is_empty() {
            if effect.flags & 1 != 0 {
                emitter.lighting = lighting
                    .1
                    .sample(&lighting.0, &lighting.2, parent.translation);
            }
            let start_ms = (time.elapsed_secs() * 1000.) as u32;
            emitter.seed = u64::from(
                (((start_ms.wrapping_mul(0x343fd).wrapping_add(0x269ec3)) >> 17) * 479) >> 15,
            );
            for e in &effect.elements {
                let delay = integer_range(e.delay_ms, &catalog, emitter.seed, 12) * 0.001;
                let limit = match e.spawn {
                    Spawn::Loop { count, .. } => count,
                    Spawn::Once { count } => {
                        integer_range(count, &catalog, emitter.seed, 13).max(0.) as u32
                    }
                };
                emitter.next.push(delay);
                emitter.counts.push(0);
                emitter.limits.push(limit);
                emitter.trail_points.push(None);
                emitter.trail_distance.push(0.);
            }
        }
        emitter.elapsed += time.delta_secs();
        for (i, e) in effect.elements.iter().enumerate() {
            let prior_count = emitter.counts[i];
            let births = schedule_element(&mut emitter, effect, e, i, parent, time.delta_secs());
            for (birth_index, born) in births.into_iter().enumerate() {
                let particle_seed = ((emitter.seed & 0xffff)
                    + (u64::from(prior_count + birth_index as u32) & 255) * 0x128
                    + (born * 1000.) as u64)
                    % 479;
                let r = channel(&catalog, particle_seed, 17);
                let mut pose = parent;
                if e.kind == 5 {
                    if let Some(previous) = emitter.previous {
                        let phase = ((born - (emitter.elapsed - time.delta_secs()))
                            / time.delta_secs().max(f32::EPSILON))
                        .clamp(0., 1.);
                        pose.translation = previous.translation.lerp(parent.translation, phase);
                        pose.rotation = previous.rotation.slerp(parent.rotation, phase);
                    }
                }
                let mut position = native(std::array::from_fn(|axis| {
                    e.origin[axis].sample(channel(&catalog, particle_seed, axis as i32))
                }));
                match e.flags & 0x30 {
                    0 => {}
                    0x10 => {
                        let z = channel(&catalog, particle_seed, 4) * 2. - 1.;
                        let angle = channel(&catalog, particle_seed, 3) * std::f32::consts::TAU;
                        let radial = (1. - z * z).sqrt();
                        position += native([radial * angle.cos(), radial * angle.sin(), z])
                            * e.offset_radius.sample(channel(&catalog, particle_seed, 5));
                    }
                    _ => {
                        let radius = e.offset_radius.sample(channel(&catalog, particle_seed, 5));
                        let angle = channel(&catalog, particle_seed, 3) * std::f32::consts::TAU;
                        position += native([
                            radius * angle.cos(),
                            radius * angle.sin(),
                            e.offset_height.sample(channel(&catalog, particle_seed, 4)),
                        ]);
                    }
                }
                pose.translation += if e.flags & 2 != 0 {
                    parent.rotation * position
                } else {
                    position
                };
                let angles: [f32; 3] = std::array::from_fn(|axis| {
                    e.angles[axis].sample(channel(&catalog, particle_seed, 6 + axis as i32))
                });
                pose.rotation *= Quat::from_euler(EulerRot::YXZ, -angles[1], angles[0], -angles[2]);
                if let Some(alias) = &e.spawn_sound {
                    if let Err(error) =
                        visuals::sound(&mut commands, &game, alias, pose.translation)
                    {
                        panic!("{}", error);
                    }
                }
                let visual = e
                    .visuals
                    .get(
                        (((channel(&catalog, particle_seed, 15).to_bits() & 0xffff) as u64
                            * e.visuals.len() as u64)
                            >> 16) as usize,
                    )
                    .and_then(Option::as_ref);
                if e.kind == 12 {
                    if let Some(name) = visual.and_then(|v| v.effect.as_ref()) {
                        emit_child(&mut commands, name, pose, entity);
                    } else {
                        panic!(
                            "{}",
                            format!("Source runner has no effect: {}", emitter.name),
                        );
                    }
                    continue;
                }
                if e.kind == 10 {
                    if let Some(alias) = visual.and_then(|v| v.sound.as_ref()) {
                        if let Err(error) =
                            visuals::sound(&mut commands, &game, alias, pose.translation)
                        {
                            panic!("{}", error);
                        }
                    }
                    continue;
                }
                let life = integer_range(e.life_ms, &catalog, particle_seed, 11) * 0.001;
                let particle_entity = commands
                    .spawn((SessionEntity, Owned(entity), pose, Visibility::default()))
                    .id();
                occupied.insert(entity);
                let mut particle = Particle {
                    element: i,
                    initial: pose,
                    // animate advances every particle after this system's
                    // births. Subtract this frame once to retain source age.
                    age: emitter.elapsed - born - time.delta_secs(),
                    life,
                    random: r,
                    rotation: e
                        .initial_rotation
                        .sample(channel(&catalog, particle_seed, 18)),
                    seed: particle_seed,
                    trail_start: None,
                    image: None,
                    scene: None,
                    atlas_start: e.atlas.index,
                    emitted_at: pose.translation,
                    emit_step: (e.emit_distance.sample(channel(&catalog, particle_seed, 14))
                        + e.emit_distance_variance
                            .sample(channel(&catalog, particle_seed, 14)))
                        * 0.0254,
                    cloud: None,
                };
                if !matches!(e.kind, 6..=9) {
                    let entries = e.atlas.entry_count_and_range & 0x1ff;
                    particle.atlas_start = match e.atlas.behavior & 3 {
                        0 => e.atlas.index,
                        1 => {
                            (((channel(&catalog, particle_seed, 16).to_bits() & 0xffff) as u64
                                * u64::from(entries))
                                >> 16) as u32
                        }
                        2 => (prior_count + birth_index as u32) & (entries - 1),
                        _ => 0,
                    };
                }
                if e.kind == 8 {
                    commands
                        .entity(particle_entity)
                        .insert(crate::rendering::native_light::FxOmniLight::default());
                } else if e.kind == 9 {
                    panic!(
                        "Original FX {} element {i}: native FX light submission (005b7af0/005b7c80) and receiver shading are required before rendering type {}",
                        emitter.name, e.kind
                    );
                } else if e.kind == 7 {
                    let Some(model) = visual.and_then(|v| v.model.as_ref()) else {
                        panic!(
                            "{}",
                            format!("Source model effect has no model: {}", emitter.name),
                        );
                    };
                    match visuals::model(&mut commands, &server, particle_entity, model) {
                        Ok(scene) => {
                            particle.scene = Some(scene);
                        }
                        Err(error) => {
                            panic!("{}", error);
                        }
                    }
                } else {
                    let name = visual.and_then(|v| {
                        if e.kind == 11 {
                            v.decal_materials.as_ref().map(|pair| &pair[1])
                        } else {
                            v.material.as_ref()
                        }
                    });
                    let Some(name) = name else {
                        panic!(
                            "{}",
                            format!(
                                "Source effect type {} has no visual: {}",
                                e.kind, emitter.name
                            ),
                        );
                    };
                    if e.kind == 11 {
                        let (color, size, _, _) = self::visual(
                            e.visual_samples
                                .first()
                                .expect("Original mark visual sample"),
                            r,
                            particle_seed,
                            &catalog,
                        );
                        decal::spawn_world(
                            &mut commands,
                            particle_entity,
                            pose,
                            size.x,
                            particle.rotation,
                            color,
                            f32::from(e.alpha_fade_ms) * 0.001,
                            name,
                            decals.0.as_deref().expect("Original world decal receivers"),
                            &mut decals.1,
                            &server,
                            &mut decals.2,
                            &mut decals.3,
                            &decals.4,
                        );
                    } else {
                        let handle = materials.get(name).unwrap_or_else(|error| {
                            panic!("Original FX {} element {i}: {error}", emitter.name)
                        });
                        let native_material = native_materials
                            .get(&handle)
                            .expect("FX material was inserted during loading");
                        particle.image = Some(native_material.color_map.clone());
                        if e.kind == 6 {
                            assert!(
                                !native_material.program.cloud_outdoor() || outdoor.is_some(),
                                "Original outdoor particle clouds require the world's outdoorImage and outdoorLookupMatrix"
                            );
                            assert!(
                                native_material.program.cloud(),
                                "Original cloud element requires its selected PARTICLECLOUD program"
                            );
                            particle.cloud = Some(cloud::State {
                                program: native_material.program,
                                size: Vec2::ZERO,
                                velocity: Vec3::ZERO,
                                color: [0.; 4],
                            });
                            let (first_index, index_count) =
                                cloud_range(e, &catalog, particle_seed);
                            commands.entity(particle_entity).insert((
                                bevy::camera::primitives::Aabb::from_min_max(
                                    Vec3::splat(-1.),
                                    Vec3::ONE,
                                ),
                                NativeEffectDraw {
                                    geometry: cloud_geometry.0.clone(),
                                    material: handle,
                                    source_world_vertices: false,
                                    vertex_byte_offset: 0,
                                    first_index,
                                    index_count,
                                    sort_center: pose.translation,
                                },
                            ));
                        } else {
                            assert!(
                                matches!(e.kind, 3 | 5)
                                    || native_material.program
                                        != NativeEffectProgram::LocalAdditiveFeather,
                                "Original local-normal FX requires directional or trail source geometry"
                            );
                            assert!(
                                matches!(
                                    native_material.program,
                                    NativeEffectProgram::Zfeather
                                        | NativeEffectProgram::ZfeatherDissolve
                                        | NativeEffectProgram::SimpleFog
                                        | NativeEffectProgram::SimpleFogDissolve
                                        | NativeEffectProgram::Additive
                                        | NativeEffectProgram::AdditiveFeather
                                        | NativeEffectProgram::LocalAdditiveFeather
                                        | NativeEffectProgram::OutdoorFeather
                                        | NativeEffectProgram::OutdoorFogFeather
                                        | NativeEffectProgram::SimpleFeather
                                        | NativeEffectProgram::EyeFeather
                                        | NativeEffectProgram::EyeAdditive
                                        | NativeEffectProgram::SimpleAdditive
                                ),
                                "Original sprite element requires its selected packed particle program"
                            );
                            if native_material.program.outdoor() {
                                assert!(
                                    outdoor.is_some(),
                                    "Original outdoor-feather FX requires the world's outdoorImage and outdoorLookupMatrix"
                                );
                            }
                            if e.kind == 5 {
                                if e.trail.is_none() {
                                    panic!(
                                        "{}",
                                        format!(
                                            "Required source trail geometry missing: {}",
                                            emitter.name
                                        ),
                                    );
                                };
                                let start = emitter.trail_points[i].unwrap_or(pose.translation);
                                emitter.trail_points[i] = Some(pose.translation);
                                particle.trail_start = Some(start);
                            }
                            let (geometry, bounds, index_count) = geometry::quad(
                                pose.rotation * Vec3::Z,
                                [127, 127, 127, 255],
                                Affine2::IDENTITY,
                                e.billboard_trim
                                    .expect("billboard source trim is required")
                                    .widths(),
                            );
                            commands.entity(particle_entity).insert((
                                bounds,
                                NativeEffectDraw {
                                    geometry: geometries.add(geometry),
                                    material: handle,
                                    source_world_vertices: matches!(e.kind, 3 | 5),
                                    vertex_byte_offset: 0,
                                    first_index: 0,
                                    index_count,
                                    sort_center: pose.translation,
                                },
                            ));
                        }
                    }
                }
                commands.entity(particle_entity).insert(particle);
                if let Some(child) = &e.effect_attached {
                    let attached = emit_child(&mut commands, child, pose, entity);
                    commands
                        .entity(attached)
                        .insert(AttachedSource(particle_entity));
                }
            }
        }
        emitter.previous = Some(parent);
        let finished = effect.elements.iter().enumerate().all(|(i, _)| {
            emitter.counts[i] >= emitter.limits[i]
                || (i < effect.looping_count
                    && effect.looping_life_ms > 0.
                    && emitter.next[i] > effect.looping_life_ms * 0.001)
        });
        if finished
            && emitter.elapsed >= effect.non_looping_life_ms * 0.001
            && !occupied.contains(&entity)
        {
            commands.entity(entity).despawn();
        }
    }
}
// Native trail emission samples cumulative distance, while other elements
// schedule by source milliseconds (005c7ae0 / 005c7ef0).
fn schedule_element(
    emitter: &mut Emitter,
    effect: &SourceEffect,
    element: &Element,
    index: usize,
    current: Transform,
    delta: f32,
) -> Vec<f32> {
    let mut births = Vec::new();
    if element.kind == 5 {
        if emitter.elapsed < emitter.next[index] {
            return births;
        }
        let Some(trail) = &element.trail else {
            return births;
        };
        if emitter.previous.is_none() {
            births.push(emitter.elapsed);
            emitter.counts[index] += 1;
            return births;
        }
        let previous = emitter.previous.unwrap();
        let distance = previous.translation.distance(current.translation);
        let step = trail.split_distance * 0.0254;
        if step <= 0. || distance <= 0. {
            return births;
        }
        let from = emitter.trail_distance[index];
        let to = from + distance;
        let mut crossing = (from / step).floor() * step + step;
        while crossing <= to {
            births.push(emitter.elapsed - delta + delta * (crossing - from) / distance);
            emitter.counts[index] += 1;
            crossing += step;
        }
        emitter.trail_distance[index] = to;
        return births;
    }
    while emitter.next[index] <= emitter.elapsed && emitter.counts[index] < emitter.limits[index] {
        if index < effect.looping_count
            && effect.looping_life_ms > 0.
            && emitter.next[index] > effect.looping_life_ms * 0.001
        {
            break;
        }
        births.push(emitter.next[index]);
        emitter.counts[index] += 1;
        emitter.next[index] += match element.spawn {
            Spawn::Loop { interval_ms, .. } => interval_ms as f32 * 0.001,
            _ => 0.,
        };
        if matches!(element.spawn, Spawn::Loop { interval_ms: 0, .. }) {
            break;
        }
    }
    births
}
fn indices(length: usize, age: f32) -> (usize, usize, f32) {
    let f = age.clamp(0., 1.) * length.saturating_sub(1) as f32;
    let a = f.floor() as usize;
    (a, (a + 1).min(length.saturating_sub(1)), f.fract())
}

fn cloud_range(element: &Element, catalog: &Catalog, seed: u64) -> (u32, u32) {
    //005b7490/00473970 choose a contiguous range in the source shared buffers.
    let count = integer_range(
        element
            .cloud_density
            .expect("original cloud density is required"),
        catalog,
        seed,
        23,
    )
    .clamp(1., 1024.) as u32;
    let selection = channel(catalog, seed, 23).to_bits() & 0xffff;
    let start = ((1025 - count) * selection) >> 16;
    let distribution = if element.flags & (1 << 22) != 0 {
        1024
    } else {
        0
    };
    ((start + distribution) * 6, count * 6)
}

fn distance_fade(element: &Element, distance: f32) -> f32 {
    // Literal005b7390 return: min(first falling ramp, second rising ramp).
    let falling = |range: Range| (1. - (distance - range.base) / range.amplitude).clamp(0., 1.);
    let first = if element.fade_in_range.amplitude == 0. {
        1.
    } else {
        falling(element.fade_in_range)
    };
    let second = if element.fade_out_range.amplitude == 0. {
        1.
    } else {
        1. - falling(element.fade_out_range)
    };
    first.min(second)
}
fn visual(s: &VisualSample, r: f32, seed: u64, catalog: &Catalog) -> ([f32; 4], Vec2, f32, f32) {
    (
        std::array::from_fn(|i| {
            s.base.color_rgba8[i] + r * (s.amplitude.color_rgba8[i] - s.base.color_rgba8[i])
        }),
        Vec2::from_array(std::array::from_fn(|i| {
            s.base.size[i] + channel(catalog, seed, 20 + i as i32) * s.amplitude.size[i]
        })),
        s.base.rotation_total + channel(catalog, seed, 19) * s.amplitude.rotation_total,
        s.base.scale + channel(catalog, seed, 22) * s.amplitude.scale,
    )
}
// Exact native atlas stepping recovered from 005b5130.
fn atlas_transform(atlas: &Atlas, start: u32, age: f32, life: f32) -> Result<Affine2, String> {
    let entries = atlas.entry_count_and_range & 0x1ff;
    if entries == 1 {
        return Ok(Affine2::IDENTITY);
    }
    let range = if atlas.behavior & 3 == 3 {
        atlas.entry_count_and_range >> 9
    } else {
        entries
    };
    let advance = if atlas.behavior & 4 != 0 {
        (age / life * range as f32) as u32
    } else {
        atlas.fps * (age * 1000.) as u32 / 1000
    };
    let mut frame = start + advance;
    if atlas.behavior & 8 != 0 && frame >= atlas.loop_count * range {
        frame = range - 1;
    }
    if atlas.behavior & 3 == 3 {
        frame = frame + atlas.index - (frame / range) * range;
    }
    frame &= entries - 1;
    let columns = 1u32 << atlas.column_bits;
    let rows = 1u32 << atlas.row_bits;
    Ok(Affine2::from_scale_angle_translation(
        Vec2::new(1. / columns as f32, 1. / rows as f32),
        0.,
        Vec2::new(
            (frame & (columns - 1)) as f32 / columns as f32,
            (frame >> atlas.column_bits) as f32 / rows as f32,
        ),
    ))
}
fn animate(
    mut commands: Commands,
    time: Res<Time>,
    server: Res<AssetServer>,
    catalog: Res<Catalog>,
    mut geometries: ResMut<Assets<NativeEffectGeometry>>,
    collision: avian3d::prelude::SpatialQuery,
    emitters: Query<&Emitter>,
    cameras: Query<
        (
            &GlobalTransform,
            &crate::rendering::native::NativeWorldScene,
        ),
        With<crate::rendering::GameCamera>,
    >,
    mut particles: Query<(
        Entity,
        &Owned,
        &mut Particle,
        &mut Transform,
        Option<&mut NativeEffectDraw>,
        Option<&mut bevy::camera::primitives::Aabb>,
        Option<&mut crate::rendering::native_light::FxOmniLight>,
    )>,
) {
    let view = cameras.iter().next();
    let exposure = view.map_or(1., |(_, scene)| scene.hdr_control0.x);
    let camera = view.map(|(pose, _)| pose.compute_transform());
    for (entity, owner, mut p, mut pose, draw, bounds, light) in &mut particles {
        let Ok(emitter) = emitters.get(owner.0) else {
            commands.entity(entity).try_despawn();
            continue;
        };
        let e = &catalog.effects[&emitter.name].elements[p.element];
        let state = if let Some(image) = &p.image {
            server.get_recursive_dependency_load_state(image.id())
        } else if let Some(scene) = &p.scene {
            server.get_recursive_dependency_load_state(scene.id())
        } else {
            Some(RecursiveDependencyLoadState::Loaded)
        };
        match state {
            Some(RecursiveDependencyLoadState::Failed(error)) => {
                panic!(
                    "{}",
                    format!("Required source effect dependency failed: {error}"),
                );
            }
            _ => {}
        }
        p.age += time.delta_secs();
        if p.age >= p.life {
            if let Some(child) = &e.effect_on_death {
                emit_child(&mut commands, child, *pose, owner.0);
            }
            commands.entity(entity).try_despawn();
            continue;
        }
        // Source mark geometry is created at birth (005c6f40), retained by
        // its receiver, and alpha-faded by 005bcb60. It is not a billboard.
        if e.kind == 11 {
            continue;
        }
        let normalized = p.age / p.life;
        let mut displacement = Vec3::ZERO;
        let mut direction = Vec3::ZERO;
        if !e.velocity_samples.is_empty() {
            let (a, b, t) = indices(e.velocity_samples.len(), normalized);
            let a = &e.velocity_samples[a];
            let b = &e.velocity_samples[b];
            let intervals = e.velocity_samples.len().saturating_sub(1).max(1) as f32;
            let step = p.life * 1000. / intervals;
            displacement = (p.initial.rotation
                * a.local
                    .total_delta
                    .sample(p.seed, &catalog)
                    .lerp(b.local.total_delta.sample(p.seed, &catalog), t)
                + a.world
                    .total_delta
                    .sample(p.seed, &catalog)
                    .lerp(b.world.total_delta.sample(p.seed, &catalog), t))
                * step;
            direction = p.initial.rotation
                * a.local
                    .velocity
                    .sample(p.seed, &catalog)
                    .lerp(b.local.velocity.sample(p.seed, &catalog), t)
                + a.world
                    .velocity
                    .sample(p.seed, &catalog)
                    .lerp(b.world.velocity.sample(p.seed, &catalog), t);
        }
        let previous_position = pose.translation;
        pose.translation = p.initial.translation + displacement;
        pose.translation.y -=
            e.gravity.sample(channel(&catalog, p.seed, 9)) * p.age * p.age * 20.32 * 0.5;
        if let Some(child) = &e.effect_on_impact {
            let travel = pose.translation - previous_position;
            if let Ok(direction) = Dir3::new(travel) {
                let filter = avian3d::prelude::SpatialQueryFilter::default()
                    .with_excluded_entities([entity]);
                if let Some(hit) =
                    collision.cast_ray(previous_position, direction, travel.length(), true, &filter)
                {
                    let mut impact = *pose;
                    impact.translation = previous_position + *direction * hit.distance;
                    emit_child(&mut commands, child, impact, owner.0);
                    commands.entity(entity).try_despawn();
                    continue;
                }
            }
        }
        if let Some(child) = &e.effect_emitted {
            if p.emit_step <= 0. {
                panic!("{}", "Source emitted effect has invalid distance interval",);
            }
            let mut distance = pose.translation.distance(p.emitted_at);
            while distance >= p.emit_step {
                let direction = (pose.translation - p.emitted_at).normalize_or_zero();
                let emit_step = p.emit_step;
                p.emitted_at += direction * emit_step;
                let mut emitted = *pose;
                emitted.translation = p.emitted_at;
                emit_child(&mut commands, child, emitted, owner.0);
                distance = pose.translation.distance(p.emitted_at);
            }
        }
        let (mut color, mut size, mut rotation, mut scale) = ([255.; 4], Vec2::ONE, p.rotation, 1.);
        if !e.visual_samples.is_empty() {
            let (a, b, t) = indices(e.visual_samples.len(), normalized);
            let a = visual(&e.visual_samples[a], p.random, p.seed, &catalog);
            let b = visual(&e.visual_samples[b], p.random, p.seed, &catalog);
            color = std::array::from_fn(|i| a.0[i] + (b.0[i] - a.0[i]) * t);
            size = a.1.lerp(b.1, t);
            rotation += (a.2 + (b.2 - a.2) * t) * p.life * 1000.;
            scale = a.3 + (b.3 - a.3) * t;
        }
        if let Some(mut light) = light {
            let fade = camera.map_or(1., |camera| {
                distance_fade(e, camera.translation.distance(pose.translation) / 0.0254)
            });
            color[3] *= fade;
            *light = crate::rendering::native_light::FxOmniLight::evaluate(
                if fade > 0.015686275 { size.x } else { 0. },
                color,
                e.flags & 0x800 != 0,
            );
            continue;
        }
        pose.scale = p.initial.scale * Vec3::new(size.x.abs() * 0.0508, size.y.abs() * 0.0508, 1.);
        let angular: [f32; 3] = std::array::from_fn(|i| {
            e.angular_velocity[i].sample(channel(&catalog, p.seed, i as i32 - 3)) * p.age * 1000.
        });
        if e.kind == 3 {
            // 005b6a80 bakes its view-relative basis, rotation, size, and pivot
            // into source-world vertices. Keep their unit normal untransformed.
            pose.rotation = Quat::IDENTITY;
            pose.scale = Vec3::ONE;
        } else if e.kind == 6 {
            pose.rotation = p.initial.rotation
                * Quat::from_euler(EulerRot::YXZ, -angular[1], angular[0], -angular[2]);
        } else if matches!(e.kind, 1 | 2 | 7) {
            pose.rotation = p.initial.rotation
                * Quat::from_euler(
                    EulerRot::YXZ,
                    -angular[1],
                    angular[0],
                    -angular[2] + rotation,
                );
        } else if let Some(camera) = camera {
            pose.rotation = camera.rotation * Quat::from_rotation_z(rotation);
            if matches!(e.kind, 4 | 5) && direction.length_squared() > 1e-8 {
                let y = direction.normalize();
                let view = (camera.translation - pose.translation).normalize_or_zero();
                let x = y.cross(view).normalize_or_zero();
                if x.length_squared() > 0.5 {
                    pose.rotation = Quat::from_mat3(&Mat3::from_cols(x, y, x.cross(y)));
                }
            }
        }
        if matches!(e.kind, 0..=2 | 4) && e.flags & 0x200000 != 0 {
            let pivot = pose.rotation
                * Vec3::new(
                    e.billboard_pivot[0] * size.x * 0.0254,
                    -e.billboard_pivot[1] * size.y * 0.0254,
                    0.,
                );
            pose.translation += pivot;
        }
        if e.kind == 5 {
            pose.rotation = p.initial.rotation;
            pose.scale = p.initial.scale;
        }
        if e.kind == 7 {
            pose.scale = p.initial.scale * scale;
        }
        let Some(mut draw) = draw else {
            continue;
        };
        if e.kind == 6 {
            pose.scale = p.initial.scale * (scale * 0.0254);
            let seed = p.seed;
            let age = p.age;
            let state = p
                .cloud
                .as_mut()
                .expect("cloud source program validated at birth");
            state.size = size;
            state.velocity =
                direction * (1000. * e.velocity_samples.len().saturating_sub(1).max(1) as f32);
            state.velocity.y -= e.gravity.sample(channel(&catalog, seed, 9)) * age * 20.32;
            state.color = color;
            let fade = camera.map_or(1., |camera| {
                distance_fade(e, camera.translation.distance(pose.translation) / 0.0254)
            });
            state.color[3] *= fade;
            let (first_index, index_count) = cloud_range(e, &catalog, seed);
            draw.first_index = first_index;
            let alpha = state.color[3].clamp(0., 255.) as u8;
            let threshold = if e.flags & 0x40000 != 0 { 0 } else { 6 };
            draw.index_count = if scale == 0. || fade <= 0.015686275 || alpha <= threshold {
                0
            } else {
                index_count
            };
            draw.sort_center = pose.translation;
            if let Some(mut bounds) = bounds {
                let extent = if scale == 0. {
                    1.
                } else {
                    1. + size.abs().max_element() / scale.abs()
                };
                *bounds = bevy::camera::primitives::Aabb::from_min_max(
                    Vec3::splat(-extent),
                    Vec3::splat(extent),
                );
            }
            continue;
        }
        let mut uv = match atlas_transform(&e.atlas, p.atlas_start, p.age, p.life) {
            Ok(uv) => uv,
            Err(error) => {
                panic!("{}", error);
            }
        };
        if let Some(trail) = &e.trail {
            if trail.scroll_ms != 0. {
                uv.translation.x -= p.age * 1000. / trail.scroll_ms;
            }
        }
        let color = lighting::pack(color, e.lighting_fraction as u8, emitter.lighting, exposure);
        let geometry = if e.kind == 5 {
            geometry::trail(
                e.trail.as_ref().expect("trail source validated at birth"),
                p.initial,
                p.trail_start.expect("trail start recorded at birth"),
                size,
                rotation,
                color,
                uv,
            )
        } else if e.kind == 3 {
            let camera = camera.expect("original directional sprite requires the game camera");
            // 005b8860 scales source sizes only when the element requests it.
            let size = if e.flags & 0x8000 != 0 {
                size * p.initial.scale.truncate()
            } else {
                size
            };
            geometry::directional(
                pose.translation,
                camera.translation,
                direction,
                size,
                rotation,
                color,
                uv,
                e.billboard_trim
                    .expect("billboard source trim is required")
                    .widths(),
                if e.flags & 0x200000 != 0 {
                    Vec2::from_array(e.billboard_pivot)
                } else {
                    Vec2::ZERO
                },
            )
        } else {
            Ok(geometry::quad(
                pose.rotation * Vec3::Z,
                color,
                uv,
                e.billboard_trim
                    .expect("billboard source trim is required")
                    .widths(),
            ))
        };
        let (updated, updated_bounds, index_count) = geometry.unwrap_or_else(|error| {
            panic!(
                "Original FX {} element {}: {error}",
                emitter.name, p.element
            )
        });
        *geometries
            .get_mut(&draw.geometry)
            .expect("particle owns its native geometry") = updated;
        draw.index_count = index_count;
        draw.sort_center = pose.translation;
        if let Some(mut bounds) = bounds {
            *bounds = updated_bounds;
        }
    }
}

fn object_constants(
    mut commands: Commands,
    materials: Res<Assets<NativeEffectMaterial>>,
    cameras: Query<Ref<GlobalTransform>, With<crate::rendering::GameCamera>>,
    mut particles: Query<
        (
            Entity,
            Ref<GlobalTransform>,
            &Particle,
            &NativeEffectDraw,
            Option<&mut NativeEffectTransform>,
            Option<&mut NativeCloudObject>,
        ),
        With<NativeEffectDraw>,
    >,
    mut previous_eye: Local<Option<Vec3>>,
    outdoor: Option<Res<NativeOutdoorMap>>,
) {
    let Ok(camera) = cameras.single() else {
        return;
    };
    let eye = camera.translation();
    let eye_changed = *previous_eye != Some(eye);
    *previous_eye = Some(eye);
    let source_from_bevy = crate::rendering::world::source_frame().inverse();
    let to_eye = source_from_bevy * Mat4::from_translation(-eye);
    for (entity, transform, particle, draw, constants, cloud_constants) in &mut particles {
        if let Some(state) = &particle.cloud {
            let updated = cloud::constants(&transform, &camera, outdoor.as_deref(), state);
            if let Some(mut constants) = cloud_constants {
                *constants = updated;
            } else {
                commands.entity(entity).insert(updated);
            }
            continue;
        }
        let needs_outdoor = materials
            .get(&draw.material)
            .expect("particle owns its native material")
            .program
            .outdoor();
        if !eye_changed
            && !(needs_outdoor && camera.is_changed())
            && !transform.is_changed()
            && !(needs_outdoor && outdoor.as_ref().is_some_and(|map| map.is_changed()))
            && constants.is_some()
        {
            continue;
        }
        // Source directional/trail vertices already carry their orientation
        // and size. Apply only the particle-origin translation here.
        let input_world = if draw.source_world_vertices {
            Mat4::from_translation(transform.translation())
                * crate::rendering::world::source_frame()
        } else {
            transform.to_matrix()
        };
        let matrix = to_eye * input_world;
        let outdoor_matrix = if needs_outdoor {
            let map = outdoor
                .as_ref()
                .expect("outdoor-feather source inputs are required");
            let lookup = Mat4::from_cols_array_2d(&map.lookup_rows.map(|row| row.to_array()));
            // 0046b3b0 uses the absolute source transform and view-away bias.
            let bias =
                source_from_bevy.transform_vector3(camera.rotation() * Vec3::Z * (32. * 0.0254));
            lookup * Mat4::from_translation(bias) * source_from_bevy * input_world
        } else {
            Mat4::ZERO
        };
        let updated = NativeEffectTransform {
            world_rows: std::array::from_fn(|i| matrix.row(i)),
            outdoor_rows: std::array::from_fn(|i| outdoor_matrix.row(i)),
        };
        if let Some(mut constants) = constants {
            *constants = updated;
        } else {
            commands.entity(entity).insert(updated);
        }
    }
}

//! Origins source-authored presentation using stock Bevy assets and materials.
mod visuals;
use crate::{
    GameState,
    assets::{GameDefinition, LoadFailure},
    gameplay::SessionEntity,
};
use bevy::{asset::RecursiveDependencyLoadState, math::Affine2, prelude::*};
use serde::Deserialize;
use std::collections::BTreeMap;
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
    decal_materials: Option<Vec<String>>,
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
#[derive(Clone, Deserialize)]
struct Element {
    #[serde(rename = "type")]
    kind: u8,
    flags: i32,
    spawn: Spawn,
    delay_ms: Range,
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
    trail: Option<Trail>,
    cloud_density: Option<Range>,
}
#[derive(Deserialize)]
struct SourceEffect {
    elements: Vec<Element>,
    looping_count: usize,
    looping_life_ms: f32,
    non_looping_life_ms: f32,
}
#[derive(Deserialize)]
struct SourceMaterial {
    image: String,
    alpha: String,
    alpha_cutoff: Option<f32>,
}
#[derive(Resource, Deserialize)]
struct Catalog {
    effects: BTreeMap<String, SourceEffect>,
    materials: BTreeMap<String, SourceMaterial>,
    random_table: Vec<f32>,
}
#[derive(Resource)]
struct CloudPoints {
    uniform: Vec<Vec3>,
    gaussian: Vec<Vec3>,
}
impl CloudPoints {
    fn native_initial_state() -> Self {
        fn crt(state: &mut u32) -> f32 {
            *state = state.wrapping_mul(214013).wrapping_add(2531011);
            ((*state >> 16) & 32767) as f32
        }
        fn gaussian(state: &mut u32) -> (f32, f32) {
            loop {
                let x = 2. * crt(state) / 32768. - 1.;
                let y = 2. * crt(state) / 32768. - 1.;
                let square = x * x + y * y;
                if square <= 1. {
                    let factor = (-2. * square.ln() / square).sqrt();
                    return (x * factor, y * factor);
                }
            }
        }
        fn quantize(v: Vec3) -> Vec3 {
            v.map(|axis| ((axis.clamp(-1., 1.) * 32767.) as i16) as f32 / 32767.)
        }
        // 0043f4c0 / 00748c70; CRT's documented initial holdrand is 1.
        let mut state = 1u32;
        let mut permutation = 0x1ffu16;
        let mut point_index = 0usize;
        let mut uniform = vec![Vec3::ZERO; 1024];
        let mut normal = Vec::with_capacity(1024);
        for i in 0..8 {
            for j in 0..8 {
                for k in 0..16 {
                    permutation = (0u16.wrapping_sub(permutation & 1) & 0x240) ^ (permutation >> 1);
                    let point = Vec3::new(
                        (crt(&mut state) / 32767. + i as f32) * 0.25 - 1.,
                        (crt(&mut state) / 32767. + j as f32) * 0.25 - 1.,
                        (crt(&mut state) / 32767. + k as f32) * 0.125 - 1.,
                    );
                    let (x, y) = gaussian(&mut state);
                    let (z, _) = gaussian(&mut state);
                    uniform[point_index] = quantize(point);
                    normal.push(quantize(Vec3::new(x, y, z) * 0.25));
                    point_index = permutation as usize;
                }
            }
        }
        Self {
            uniform,
            gaussian: normal,
        }
    }
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
    trail_points: Vec<Option<Vec3>>,
    trail_distance: Vec<f32>,
}
#[derive(Component)]
struct Owned(Entity);
#[derive(Component)]
struct AttachedSource(Entity);
#[derive(Component)]
struct Stopped;
#[derive(Component)]
struct Particle {
    source: Element,
    initial: Transform,
    age: f32,
    life: f32,
    random: f32,
    seed: u64,
    trail_start: Option<Vec3>,
    rotation: f32,
    image: Option<Handle<Image>>,
    scene: Option<Handle<bevy::world_serialization::WorldAsset>>,
    material: Option<Handle<StandardMaterial>>,
    decal: Option<Handle<bevy::pbr::decal::ForwardDecalMaterial<StandardMaterial>>>,
    atlas_start: u32,
    emitted_at: Vec3,
    emit_step: f32,
}
pub(crate) struct NativeFxPlugin;
impl Plugin for NativeFxPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::Loading), load_catalog);
        app.add_systems(
            Update,
            (follow_source_attachments, spawn, animate)
                .chain()
                .run_if(in_state(GameState::Playing)),
        );
    }
}
fn load_catalog(
    mut commands: Commands,
    mut failure: ResMut<LoadFailure>,
    mut next: ResMut<NextState<GameState>>,
) {
    match serde_json::from_str::<Catalog>(include_str!("../gameplay/origins/fx_catalog.json")) {
        Ok(catalog) => {
            if catalog.random_table.len() < 512
                || catalog
                    .random_table
                    .iter()
                    .any(|value| !value.is_finite() || !(0. ..=1.).contains(value))
            {
                fail(
                    &mut failure,
                    &mut next,
                    "Invalid recovered native FX random table".into(),
                );
                return;
            }
            commands.insert_resource(catalog);
            commands.insert_resource(CloudPoints::native_initial_state());
        }
        Err(error) => {
            fail(
                &mut failure,
                &mut next,
                format!("Invalid source FX catalog: {error}"),
            );
        }
    }
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

fn fail(failure: &mut LoadFailure, next: &mut NextState<GameState>, text: String) {
    failure.0 = text;
    next.set(GameState::LoadError);
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
            commands.entity(entity).insert(Stopped);
        }
    }
}
fn spawn(
    mut commands: Commands,
    time: Res<Time>,
    catalog: Option<Res<Catalog>>,
    server: Res<AssetServer>,
    game: Res<GameDefinition>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut decals: ResMut<Assets<bevy::pbr::decal::ForwardDecalMaterial<StandardMaterial>>>,
    mut emitters: Query<(
        Entity,
        &mut Emitter,
        &GlobalTransform,
        &Transform,
        Option<&AttachedSource>,
        Option<&Stopped>,
    )>,
    particles: Query<&Owned>,
    mut failure: ResMut<LoadFailure>,
    mut next: ResMut<NextState<GameState>>,
) {
    let Some(catalog) = catalog else {
        return;
    };
    for (entity, mut emitter, global, local, attachment, stopped) in &mut emitters {
        if stopped.is_some() {
            if !particles.iter().any(|owner| owner.0 == entity) {
                commands.entity(entity).despawn();
            }
            continue;
        }
        let Some(effect) = catalog.effects.get(&emitter.name) else {
            fail(
                &mut failure,
                &mut next,
                format!(
                    "Required source effect definition missing: {}",
                    emitter.name
                ),
            );
            return;
        };
        if emitter.next.is_empty() {
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
        let parent = if attachment.is_some() {
            *local
        } else {
            global.compute_transform()
        };
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
                        fail(&mut failure, &mut next, error);
                        return;
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
                        emit(&mut commands, name, pose);
                    } else {
                        fail(
                            &mut failure,
                            &mut next,
                            format!("Source runner has no effect: {}", emitter.name),
                        );
                        return;
                    }
                    continue;
                }
                if e.kind == 10 {
                    if let Some(alias) = visual.and_then(|v| v.sound.as_ref()) {
                        if let Err(error) =
                            visuals::sound(&mut commands, &game, alias, pose.translation)
                        {
                            fail(&mut failure, &mut next, error);
                            return;
                        }
                    }
                    continue;
                }
                let life = integer_range(e.life_ms, &catalog, particle_seed, 11) * 0.001;
                if !life.is_finite() || life <= 0. {
                    fail(
                        &mut failure,
                        &mut next,
                        format!("Invalid source effect element lifetime: {}", emitter.name),
                    );
                    return;
                }
                let particle_entity = commands
                    .spawn((SessionEntity, Owned(entity), pose, Visibility::default()))
                    .id();
                let mut particle = Particle {
                    source: e.clone(),
                    initial: pose,
                    age: (emitter.elapsed - born).max(0.),
                    life,
                    random: r,
                    rotation: e
                        .initial_rotation
                        .sample(channel(&catalog, particle_seed, 18)),
                    seed: particle_seed,
                    trail_start: None,
                    image: None,
                    scene: None,
                    material: None,
                    decal: None,
                    atlas_start: e.atlas.index,
                    emitted_at: pose.translation,
                    emit_step: (e.emit_distance.sample(channel(&catalog, particle_seed, 14))
                        + e.emit_distance_variance
                            .sample(channel(&catalog, particle_seed, 14)))
                        * 0.0254,
                };
                if !matches!(e.kind, 7 | 8) {
                    let entries = e.atlas.entry_count_and_range & 0x1ff;
                    if entries == 0 {
                        fail(
                            &mut failure,
                            &mut next,
                            "Source FX atlas has no entries".into(),
                        );
                        return;
                    }
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
                    visuals::light(&mut commands, particle_entity);
                } else if e.kind == 7 {
                    let Some(model) = visual.and_then(|v| v.model.as_ref()) else {
                        fail(
                            &mut failure,
                            &mut next,
                            format!("Source model effect has no model: {}", emitter.name),
                        );
                        return;
                    };
                    match visuals::model(&mut commands, &server, particle_entity, model) {
                        Ok(scene) => {
                            particle.scene = Some(scene);
                        }
                        Err(error) => {
                            fail(&mut failure, &mut next, error);
                            return;
                        }
                    }
                } else {
                    let name = visual.and_then(|v| {
                        if e.kind == 11 {
                            v.decal_materials.as_ref().and_then(|m| m.first())
                        } else {
                            v.material.as_ref()
                        }
                    });
                    let Some(name) = name else {
                        fail(
                            &mut failure,
                            &mut next,
                            format!(
                                "Source effect type {} has no visual: {}",
                                e.kind, emitter.name
                            ),
                        );
                        return;
                    };
                    let Some(source) = catalog.materials.get(name.trim_start_matches(',')) else {
                        fail(
                            &mut failure,
                            &mut next,
                            format!("Required source effect material missing: {name}"),
                        );
                        return;
                    };
                    let image = server.load(source.image.clone());
                    particle.image = Some(image.clone());
                    let material = StandardMaterial {
                        base_color_texture: Some(image),
                        unlit: true,
                        cull_mode: None,
                        alpha_mode: match source.alpha.as_str() {
                            "add" => AlphaMode::Add,
                            "blend" => AlphaMode::Blend,
                            "mask" => AlphaMode::Mask(match source.alpha_cutoff {
                                Some(value) => value,
                                None => {
                                    fail(
                                        &mut failure,
                                        &mut next,
                                        format!(
                                            "Source masked FX material has no alpha cutoff: {name}"
                                        ),
                                    );
                                    return;
                                }
                            }),
                            "opaque" => AlphaMode::Opaque,
                            other => {
                                fail(
                                    &mut failure,
                                    &mut next,
                                    format!("Unsupported source effect alpha mode: {other}"),
                                );
                                return;
                            }
                        },
                        ..default()
                    };
                    if e.kind == 11 {
                        particle.decal = Some(visuals::decal(
                            &mut commands,
                            &mut decals,
                            particle_entity,
                            material,
                        ));
                    } else {
                        let handle = materials.add(material);
                        particle.material = Some(handle.clone());
                        let mesh = if e.kind == 5 {
                            let Some(trail) = &e.trail else {
                                fail(
                                    &mut failure,
                                    &mut next,
                                    format!(
                                        "Required source trail geometry missing: {}",
                                        emitter.name
                                    ),
                                );
                                return;
                            };
                            let start = emitter.trail_points[i].unwrap_or(pose.translation);
                            emitter.trail_points[i] = Some(pose.translation);
                            particle.trail_start = Some(start);
                            match trail_mesh(trail, pose, start, Vec2::ZERO) {
                                Ok(mesh) => mesh,
                                Err(error) => {
                                    fail(&mut failure, &mut next, error);
                                    return;
                                }
                            }
                        } else {
                            Mesh::from(Rectangle::new(1., 1.))
                        };
                        commands
                            .entity(particle_entity)
                            .insert((Mesh3d(meshes.add(mesh)), MeshMaterial3d(handle)));
                    }
                }
                commands.entity(particle_entity).insert(particle);
                if let Some(child) = &e.effect_attached {
                    let attached = emit(&mut commands, child, pose);
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
            && !particles.iter().any(|owner| owner.0 == entity)
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
fn trail_mesh(source: &Trail, pose: Transform, start: Vec3, size: Vec2) -> Result<Mesh, String> {
    use bevy::mesh::{Indices, PrimitiveTopology};
    if source.vertices.is_empty()
        || source.indices.len() % 2 != 0
        || source
            .indices
            .iter()
            .any(|i| *i as usize >= source.vertices.len())
    {
        return Err("Invalid source trail cross section".into());
    }
    let local_start = pose.rotation.inverse() * (start - pose.translation);
    let length = local_start.length();
    let sections = if source.split_distance > 0. {
        (length / (source.split_distance * 0.0254)).ceil().max(1.) as usize
    } else {
        1
    };
    let mut vertices = Vec::new();
    let mut normals = Vec::new();
    let mut uv = Vec::new();
    let mut indices = Vec::new();
    for ring in 0..=sections {
        let fraction = ring as f32 / sections as f32;
        let center = local_start * (1. - fraction);
        for v in &source.vertices {
            vertices.push([
                center.x + v.position[0] * size.x * 0.0254,
                center.y + v.position[1] * size.y * 0.0254,
                center.z,
            ]);
            normals.push([v.normal[0], v.normal[1], 0.]);
            uv.push([
                v.texcoord,
                if source.repeat_distance > 0. {
                    fraction * length / (source.repeat_distance * 0.0254)
                } else {
                    fraction
                },
            ]);
        }
    }
    let count = source.vertices.len() as u32;
    for ring in 0..sections as u32 {
        for edge in source.indices.chunks_exact(2) {
            let a = ring * count + edge[0];
            let b = ring * count + edge[1];
            let c = a + count;
            let d = b + count;
            indices.extend([a, b, c, b, d, c]);
        }
    }
    let mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        bevy::asset::RenderAssetUsages::default(),
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, vertices)
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uv)
    .with_inserted_indices(Indices::U32(indices));
    Ok(mesh)
}
fn cloud_mesh(points: &[Vec3], size: Vec2, scale: f32, source_to_view: Quat) -> Mesh {
    use bevy::mesh::{Indices, PrimitiveTopology};
    let mut positions = Vec::with_capacity(points.len() * 4);
    let mut normals = Vec::with_capacity(points.len() * 4);
    let mut uv = Vec::with_capacity(points.len() * 4);
    let mut indices = Vec::with_capacity(points.len() * 6);
    for point in points {
        let center = source_to_view * native(point.to_array()) * scale;
        let first = positions.len() as u32;
        for corner in [
            Vec2::new(-1., -1.),
            Vec2::new(1., -1.),
            Vec2::new(-1., 1.),
            Vec2::new(1., 1.),
        ] {
            positions.push(
                (center + Vec3::new(corner.x * size.x * 0.0254, corner.y * size.y * 0.0254, 0.))
                    .to_array(),
            );
            normals.push([0., 0., 1.]);
            uv.push([(corner.x + 1.) * 0.5, (corner.y + 1.) * 0.5]);
        }
        indices.extend([first, first + 1, first + 2, first + 2, first + 1, first + 3]);
    }
    Mesh::new(
        PrimitiveTopology::TriangleList,
        bevy::asset::RenderAssetUsages::default(),
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uv)
    .with_inserted_indices(Indices::U32(indices))
}
fn indices(length: usize, age: f32) -> (usize, usize, f32) {
    let f = age.clamp(0., 1.) * length.saturating_sub(1) as f32;
    let a = f.floor() as usize;
    (a, (a + 1).min(length.saturating_sub(1)), f.fract())
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
    if entries == 0 {
        return Err("Source FX atlas has zero entries".into());
    }
    if entries == 1 {
        return Ok(Affine2::IDENTITY);
    }
    let range = if atlas.behavior & 3 == 3 {
        atlas.entry_count_and_range >> 9
    } else {
        entries
    };
    if range == 0 {
        return Err("Source FX atlas has zero index range".into());
    }
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
    cloud_points: Res<CloudPoints>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut mesh_assets: ResMut<Assets<Mesh>>,
    collision: avian3d::prelude::SpatialQuery,
    mut decals: ResMut<Assets<bevy::pbr::decal::ForwardDecalMaterial<StandardMaterial>>>,
    emitters: Query<(), With<Emitter>>,
    cameras: Query<&GlobalTransform, With<crate::rendering::GameCamera>>,
    mut particles: Query<(
        Entity,
        &Owned,
        &mut Particle,
        &mut Transform,
        Option<&mut PointLight>,
        Option<&Mesh3d>,
    )>,
    mut failure: ResMut<LoadFailure>,
    mut next: ResMut<NextState<GameState>>,
) {
    let camera = cameras
        .iter()
        .next()
        .map(GlobalTransform::compute_transform);
    for (entity, owner, mut p, mut pose, light, mesh) in &mut particles {
        if !emitters.contains(owner.0) {
            commands.entity(entity).try_despawn();
            continue;
        }
        let state = if let Some(image) = &p.image {
            server.get_recursive_dependency_load_state(image.id())
        } else if let Some(scene) = &p.scene {
            server.get_recursive_dependency_load_state(scene.id())
        } else {
            Some(RecursiveDependencyLoadState::Loaded)
        };
        match state {
            Some(RecursiveDependencyLoadState::Failed(error)) => {
                fail(
                    &mut failure,
                    &mut next,
                    format!("Required source effect dependency failed: {error}"),
                );
                return;
            }
            Some(RecursiveDependencyLoadState::Loaded) => {}
            _ => continue,
        }
        p.age += time.delta_secs();
        if p.age >= p.life {
            if let Some(child) = &p.source.effect_on_death {
                emit(&mut commands, child, *pose);
            }
            if let Some(m) = &p.material {
                materials.remove(m.id());
            }
            if let Some(m) = &p.decal {
                decals.remove(m.id());
            }
            commands.entity(entity).try_despawn();
            continue;
        }
        let e = p.source.clone();
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
                    emit(&mut commands, child, impact);
                    commands.entity(entity).try_despawn();
                    continue;
                }
            }
        }
        if let Some(child) = &e.effect_emitted {
            if p.emit_step <= 0. {
                fail(
                    &mut failure,
                    &mut next,
                    "Source emitted effect has invalid distance interval".into(),
                );
                return;
            }
            let mut distance = pose.translation.distance(p.emitted_at);
            while distance >= p.emit_step {
                let direction = (pose.translation - p.emitted_at).normalize_or_zero();
                let emit_step = p.emit_step;
                p.emitted_at += direction * emit_step;
                let mut emitted = *pose;
                emitted.translation = p.emitted_at;
                emit(&mut commands, child, emitted);
                distance = pose.translation.distance(p.emitted_at);
            }
        }
        let e = p.source.clone();
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
        pose.scale = p.initial.scale * Vec3::new(size.x.abs() * 0.0508, size.y.abs() * 0.0508, 1.);
        let angular: [f32; 3] = std::array::from_fn(|i| {
            e.angular_velocity[i].sample(channel(&catalog, p.seed, i as i32 - 3)) * p.age * 1000.
        });
        if matches!(e.kind, 1 | 2 | 7) {
            pose.rotation = p.initial.rotation
                * Quat::from_euler(
                    EulerRot::YXZ,
                    -angular[1],
                    angular[0],
                    -angular[2] + rotation,
                );
        } else if let Some(camera) = camera {
            pose.rotation = camera.rotation * Quat::from_rotation_z(rotation);
            if matches!(e.kind, 3 | 4 | 5) && direction.length_squared() > 1e-8 {
                let y = direction.normalize();
                let view = (camera.translation - pose.translation).normalize_or_zero();
                let x = y.cross(view).normalize_or_zero();
                if x.length_squared() > 0.5 {
                    pose.rotation = Quat::from_mat3(&Mat3::from_cols(x, y, x.cross(y)));
                }
            }
        }
        if matches!(e.kind, 0..=4) && e.flags & 0x200000 != 0 {
            let pivot = pose.rotation
                * Vec3::new(
                    e.billboard_pivot[0] * size.x * 0.0254,
                    e.billboard_pivot[1] * size.y * 0.0254,
                    0.,
                );
            pose.translation += pivot;
        }
        if e.kind == 5 {
            pose.rotation = p.initial.rotation;
            pose.scale = p.initial.scale;
            if let (Some(source), Some(start), Some(mesh)) = (&e.trail, p.trail_start, mesh) {
                match trail_mesh(source, p.initial, start, size) {
                    Ok(updated) => {
                        if let Some(mut existing) = mesh_assets.get_mut(&mesh.0) {
                            *existing = updated;
                        }
                    }
                    Err(error) => {
                        fail(&mut failure, &mut next, error);
                        return;
                    }
                }
            }
        }
        if e.kind == 6 {
            let Some(density) = e.cloud_density else {
                fail(
                    &mut failure,
                    &mut next,
                    "Source cloud has no density range".into(),
                );
                return;
            };
            let count = integer_range(density, &catalog, p.seed, 23).clamp(1., 1024.) as usize;
            let selection = channel(&catalog, p.seed, 23).to_bits() & 0xffff;
            let start = ((1025 - count) * selection as usize) >> 16;
            let points = if e.flags & (1 << 22) != 0 {
                &cloud_points.gaussian
            } else {
                &cloud_points.uniform
            };
            if let Some(mesh) = mesh {
                let updated = cloud_mesh(
                    &points[start..start + count],
                    size,
                    scale,
                    pose.rotation.inverse() * p.initial.rotation,
                );
                if let Some(mut existing) = mesh_assets.get_mut(&mesh.0) {
                    *existing = updated;
                }
            }
            pose.scale = p.initial.scale;
        }
        if e.kind == 7 {
            pose.scale = p.initial.scale * scale;
        }
        if let Some(mut light) = light {
            // Native 005b7af0/00410c30 submits linear RGB * normalized alpha * 32.
            // Native brightness has no recovered SI unit. Treat it as candela
            // and convert to Bevy lumens (4*pi); native falloff parity is not implied.
            light.color = Color::srgb(color[0] / 255., color[1] / 255., color[2] / 255.)
                .to_linear()
                .into();
            light.intensity = (color[3] / 255. * 32.) * (4. * std::f32::consts::PI);
            light.range = size.x.clamp(0., 10000.) * 0.0254;
            light.radius = 0.;
        }
        if p.material.is_none() && p.decal.is_none() {
            continue;
        }
        let mut uv = match atlas_transform(&e.atlas, p.atlas_start, p.age, p.life) {
            Ok(uv) => uv,
            Err(error) => {
                fail(&mut failure, &mut next, error);
                return;
            }
        };
        if let Some(trail) = &e.trail {
            if trail.scroll_ms != 0. {
                uv.translation.y += p.age * 1000. / trail.scroll_ms;
            }
        }
        if let Some(mut m) = p.material.as_ref().and_then(|h| materials.get_mut(h)) {
            m.base_color = Color::srgba(
                color[0] / 255.,
                color[1] / 255.,
                color[2] / 255.,
                color[3] / 255.,
            );
            m.uv_transform = uv;
        }
        if let Some(mut m) = p.decal.as_ref().and_then(|h| decals.get_mut(h)) {
            m.base.base_color = Color::srgba(
                color[0] / 255.,
                color[1] / 255.,
                color[2] / 255.,
                color[3] / 255.,
            );
            m.base.uv_transform = uv;
        }
    }
}

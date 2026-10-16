//! Weapon-owned source flash/ejection FX and original TracerDef beams.
use super::*;
use crate::{origins::content::WeaponId, presentation::Rig};
use bevy::{camera::primitives::Aabb, world_serialization::WorldInstanceReady};
use std::{collections::HashMap, sync::LazyLock};

#[derive(Message)]
pub(crate) struct Shot {
    pub owner: Entity,
    pub weapon: WeaponId,
    pub view_weapon: WeaponId,
    pub left: bool,
    pub origin: Vec3,
    pub end: Option<Vec3>,
    pub muzzle: bool,
    pub last: bool,
}

#[derive(Component, Default)]
struct Sockets {
    flash: Option<Entity>,
    alternate: Option<Entity>,
    brass: Option<Entity>,
}

#[derive(Deserialize)]
pub(super) struct TracerDefinition {
    kind: String,
    material: String,
    draw_interval: u8,
    speed: f32,
    beam_length: f32,
    beam_width: f32,
    screw_radius: f32,
    screw_distance: f32,
    fade_time: f32,
    fade_scale: f32,
    tex_repeat_rate: f32,
    colors: [[f32; 4]; 5],
}

pub(super) fn tracers() -> &'static BTreeMap<String, TracerDefinition> {
    static DEFINITIONS: LazyLock<BTreeMap<String, TracerDefinition>> = LazyLock::new(|| {
        serde_json::from_str(include_str!("../../gameplay/origins/tracers.json"))
            .expect("original weapon TracerDef records")
    });
    &DEFINITIONS
}

pub(super) fn materials() -> impl Iterator<Item = &'static String> {
    tracers().values().map(|tracer| &tracer.material)
}

#[derive(Component)]
struct Tracer {
    definition: String,
    start: Vec3,
    end: Vec3,
    age: f32,
}

pub(super) fn install(app: &mut App) {
    app.add_message::<Shot>()
        .add_observer(bind_sockets)
        .add_systems(
            Update,
            (shots, animate_tracers)
                .chain()
                .after(crate::presentation::PresentationAnimation)
                .before(super::spawn)
                .run_if(in_state(GameState::Playing)),
        );
}

fn bind_sockets(
    event: On<WorldInstanceReady>,
    mut commands: Commands,
    rigs: Query<&Rig>,
    children: Query<&Children>,
    names: Query<&Name>,
) {
    let Ok(rig) = rigs.get(event.entity) else {
        return;
    };
    if rig.arms || rig.knife {
        return;
    }
    let mut sockets = Sockets::default();
    for child in children.iter_descendants(event.entity) {
        match names.get(child).map(Name::as_str) {
            Ok("tag_flash") => sockets.flash = Some(child),
            Ok("tag_flash1") if rig.left => sockets.flash = Some(child),
            Ok("tag_flash_alt") => sockets.alternate = Some(child),
            Ok("tag_brass") => sockets.brass = Some(child),
            Ok("tag_brass1") if rig.left => sockets.brass = Some(child),
            _ => {}
        }
    }
    commands.entity(event.entity).insert(sockets);
}

fn socket_pose(
    entity: Option<Entity>,
    poses: &Query<&GlobalTransform>,
    offset: [f32; 3],
) -> Option<Transform> {
    let matrix = poses.get(entity?).ok()?.to_matrix();
    if matrix.determinant() == 0. {
        return None;
    }
    // The exported tag includes source inches and Z-up conversion. FX already
    // convert their own inputs, so keep only the tag's world placement here.
    let mut pose =
        Transform::from_matrix(matrix * crate::rendering::world::source_frame().inverse());
    pose.translation += pose.rotation * native(offset);
    Some(pose)
}

fn shots(
    mut commands: Commands,
    mut events: MessageReader<Shot>,
    game: Res<GameDefinition>,
    rigs: Query<(&Rig, &Sockets, &Visibility)>,
    poses: Query<&GlobalTransform>,
    players: Query<(), With<crate::player::Player>>,
    materials: Res<material::Materials>,
    mut geometries: ResMut<Assets<NativeEffectGeometry>>,
    mut counters: Local<HashMap<Entity, u8>>,
) {
    if events.is_empty() {
        return;
    }
    counters.retain(|owner, _| players.contains(*owner));
    for shot in events.read() {
        let weapon = game
            .content
            .weapons
            .iter()
            .find(|weapon| weapon.id == shot.weapon)
            .expect("shot owns its source weapon definition");
        let effects = &weapon.effects;
        let sockets = rigs
            .iter()
            .find(|(rig, sockets, visible)| {
                rig.owner == shot.owner
                    && rig.ready
                    && rig.attached
                    && **visible != Visibility::Hidden
                    && rig.weapon == shot.view_weapon
                    && rig.left == shot.left
                    && (sockets.flash.is_some()
                        || sockets.alternate.is_some()
                        || sockets.brass.is_some())
            })
            .map(|(_, sockets, _)| sockets);
        let flash = sockets.and_then(|sockets| {
            socket_pose(
                if effects.alternate_flash {
                    sockets.alternate.or(sockets.flash)
                } else {
                    sockets.flash
                },
                &poses,
                effects.flash_offset,
            )
        });
        if shot.muzzle {
            if let (Some(effect), Some(pose)) = (&effects.flash, flash) {
                super::emit(&mut commands, effect, pose);
            }
            let shell = if shot.last {
                effects.last_shell.as_ref()
            } else {
                effects.shell.as_ref()
            };
            if let (Some(effect), Some(mut pose)) = (
                shell,
                sockets
                    .and_then(|sockets| socket_pose(sockets.brass, &poses, effects.shell_offset)),
            ) {
                let [pitch, yaw, roll] = effects.shell_rotation.map(f32::to_radians);
                pose.rotation *= Quat::from_euler(EulerRot::YXZ, -yaw, pitch, -roll);
                super::emit(&mut commands, effect, pose);
            }
        }
        let (Some(name), Some(end)) = (&effects.tracer, shot.end) else {
            continue;
        };
        let definition = tracers()
            .get(name)
            .expect("required original weapon tracer");
        // 007c7450 owns one countdown per shooter, including weapon switches.
        let count = counters.entry(shot.owner).or_default();
        if *count == 0 || *count > definition.draw_interval {
            *count = definition.draw_interval;
        }
        *count = count.saturating_sub(1);
        if *count != 0 {
            continue;
        }
        let start = flash.map_or(shot.origin, |pose| pose.translation);
        if start.distance(end) <= 5. * 0.0254 {
            continue;
        }
        commands.spawn((
            SessionEntity,
            Tracer {
                definition: name.clone(),
                start,
                end,
                age: 0.,
            },
            Transform::from_translation(start),
            Visibility::Inherited,
            Aabb::default(),
            NativeEffectDraw {
                geometry: geometries.add(NativeEffectGeometry {
                    layout: crate::rendering::native_effect::NativeEffectVertexLayout::Packed32,
                    vertices: Vec::new(),
                    indices: Vec::new(),
                }),
                material: materials
                    .get(&definition.material)
                    .expect("original tracer material"),
                vertex_byte_offset: 0,
                first_index: 0,
                index_count: 0,
                sort_center: start,
            },
        ));
    }
}

fn animate_tracers(
    mut commands: Commands,
    time: Res<Time>,
    cameras: Query<&GlobalTransform, With<crate::rendering::GameCamera>>,
    mut tracers_query: Query<(
        Entity,
        &mut Tracer,
        &mut Transform,
        &mut Aabb,
        &mut NativeEffectDraw,
    )>,
    mut geometries: ResMut<Assets<NativeEffectGeometry>>,
) {
    let Ok(camera) = cameras.single() else { return };
    for (entity, mut tracer, mut pose, mut bounds, mut draw) in &mut tracers_query {
        let definition = &tracers()[&tracer.definition];
        let delta = tracer.end - tracer.start;
        let distance = delta.length();
        let direction = delta / distance;
        let smoke = definition.kind == "Smoke";
        let life = if smoke {
            definition.fade_time
        } else {
            distance / (definition.speed * 0.0254)
        };
        // Render the initial beam even when its flight is shorter than a frame.
        if !tracer.is_added() {
            tracer.age += time.delta_secs();
        }
        if tracer.age >= life {
            commands.entity(entity).despawn();
            continue;
        }
        let travelled = tracer.age
            * if smoke {
                0.1 * 0.0254
            } else {
                definition.speed * 0.0254
            };
        let start = tracer.start
            + direction * travelled
            + if smoke {
                Vec3::Y * (tracer.age * 2. * 0.0254)
            } else {
                Vec3::ZERO
            };
        let length = (distance - travelled).min(definition.beam_length * 0.0254);
        let fade = if smoke { 1. - tracer.age / life } else { 1. };
        let width =
            definition.beam_width.min(4.) * ((definition.fade_scale - 1.) * (1. - fade) + 1.);
        let geometry = beam(
            definition,
            direction,
            length,
            camera.translation() - start,
            width,
            fade,
            smoke,
        );
        pose.translation = start;
        *bounds = geometry.1;
        draw.index_count = (geometry.0.indices.len() / 2) as u32;
        draw.sort_center = start + direction * (length * 0.5);
        *geometries
            .get_mut(&draw.geometry)
            .expect("tracer owns its geometry") = geometry.0;
    }
}

fn beam(
    definition: &TracerDefinition,
    direction: Vec3,
    length: f32,
    eye: Vec3,
    width: f32,
    fade: f32,
    smoke: bool,
) -> (NativeEffectGeometry, Aabb) {
    let right = direction
        .cross(eye)
        .try_normalize()
        .unwrap_or_else(|| direction.any_orthonormal_vector());
    let up = right.cross(direction);
    // 007c72b0 splits laser beams eight times more finely than smoke beams,
    // with the original 255-section limit and five packed vertex colors.
    let sections = ((length / 0.0254 * if smoke { 1. } else { 8. }) / definition.screw_distance)
        .floor()
        .clamp(1., 255.) as usize;
    let mut geometry = NativeEffectGeometry {
        layout: crate::rendering::native_effect::NativeEffectVertexLayout::Packed32,
        vertices: Vec::with_capacity((sections + 1) * 64),
        indices: Vec::with_capacity(sections * 12),
    };
    let mut low = Vec3::splat(f32::INFINITY);
    let mut high = Vec3::splat(f32::NEG_INFINITY);
    for i in 0..=sections {
        let fraction = i as f32 / sections as f32;
        let phase =
            fraction * length / (definition.screw_distance * 0.0254) * std::f32::consts::TAU;
        let center = direction * (fraction * length)
            + (right * phase.cos() + up * phase.sin()) * (definition.screw_radius * 0.0254);
        let sample = fraction * 4.;
        let index = (sample.floor() as usize).min(3);
        let color: [u8; 4] = std::array::from_fn(|channel| {
            let value = definition.colors[index][channel]
                + (definition.colors[index + 1][channel] - definition.colors[index][channel])
                    * (sample - index as f32);
            (value * if channel == 3 { fade } else { 1. } * 255.).clamp(0., 255.) as u8
        });
        for (side, v) in [(-1., 0.), (1., 1.)] {
            let point = center + right * (side * width * 0.0254 * 0.5);
            low = low.min(point);
            high = high.max(point);
            super::geometry::vertex(
                &mut geometry.vertices,
                Vec3::new(point.x, -point.z, point.y) / 0.0254,
                color,
                Vec2::new(fraction * definition.tex_repeat_rate, v),
                0x7f7fffff,
            );
        }
        if i > 0 {
            let a = ((i - 1) * 2) as u16;
            for index in [a, a + 1, a + 2, a + 1, a + 3, a + 2] {
                geometry.indices.extend_from_slice(&index.to_le_bytes());
            }
        }
    }
    (geometry, Aabb::from_min_max(low, high))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tracer_geometry_stays_finite_when_firing_along_the_camera_axis() {
        let definition = &tracers()["pistol"];
        let (geometry, bounds) = beam(
            definition,
            Vec3::NEG_Z,
            1.,
            Vec3::Z,
            definition.beam_width,
            1.,
            false,
        );
        assert!(!geometry.indices.is_empty());
        assert!(bounds.center.is_finite() && bounds.half_extents.is_finite());
        for vertex in geometry.vertices.chunks_exact(32) {
            for coordinate in vertex[..12].chunks_exact(4) {
                assert!(f32::from_le_bytes(coordinate.try_into().unwrap()).is_finite());
            }
        }
        let vertices = geometry.vertices.len() / 32;
        assert!(
            geometry
                .indices
                .chunks_exact(2)
                .all(|index| usize::from(u16::from_le_bytes(index.try_into().unwrap())) < vertices)
        );
    }

    #[test]
    fn smoke_tracer_fades_its_authored_colors_without_leaving_invalid_draw_ranges() {
        let definition = &tracers()["ballista_tracer"];
        let (geometry, _) = beam(
            definition,
            Vec3::X,
            10000.,
            Vec3::Y,
            definition.beam_width,
            0.5,
            true,
        );
        assert_eq!(geometry.vertices.len() / 32, 512);
        assert_eq!(geometry.indices.len() / 2, 255 * 6);
        let last = &geometry.vertices[geometry.vertices.len() - 32..];
        assert_eq!(last[19], (definition.colors[4][3] * 0.5 * 255.) as u8);
    }
}

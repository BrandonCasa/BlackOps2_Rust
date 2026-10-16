//! Source mark projection: 00446950 / 004451d0 / 00446b30 / 005bf460.
use crate::rendering::{
    material::MaterialDefinition,
    native::*,
    world::{WorldSurfaceColor, WorldSurfaceLighting},
};
use bevy::{camera::primitives::Aabb, prelude::*};
use std::{collections::BTreeMap, sync::Arc};

#[derive(Clone, Copy, Default)]
struct ClipVertex {
    position: Vec3,
    weights: Vec3,
}

#[derive(Clone, Copy, Default)]
struct Vertex {
    position: Vec3,
    lightmap: Vec2,
    normal: Vec3,
}

#[derive(Default)]
struct Polygon {
    vertices: [Vertex; 14],
    count: usize,
}

struct Projection {
    origin: Vec3,
    axes: [Vec3; 3],
    radius: f32,
}

impl Projection {
    fn clip(&self, triangle: [Vertex; 3]) -> Polygon {
        // 00446b30 tests clockwise source triangles: (a-b) x (c-b).
        let cross = (triangle[0].position - triangle[1].position)
            .cross(triangle[2].position - triangle[1].position);
        let facing = self.axes[0].dot(cross);
        if facing < 0. || facing * facing < cross.length_squared() * 0.25 {
            return Polygon::default();
        }
        let mut polygon = [ClipVertex::default(); 14];
        let mut scratch = polygon;
        for i in 0..3 {
            polygon[i] = ClipVertex {
                position: triangle[i].position,
                weights: Vec3::AXES[i],
            };
        }
        let mut count = 3;
        for axis in self.axes {
            for normal in [axis, -axis] {
                count = clip_plane(
                    &polygon[..count],
                    &mut scratch,
                    normal,
                    normal.dot(self.origin) - self.radius,
                );
                if count == 0 {
                    return Polygon::default();
                }
                std::mem::swap(&mut polygon, &mut scratch);
            }
        }
        let mut output = Polygon { count, ..default() };
        for (vertex, v) in output.vertices.iter_mut().zip(&polygon[..count]) {
            *vertex = Vertex {
                position: v.position,
                lightmap: triangle[0].lightmap * v.weights.x
                    + triangle[1].lightmap * v.weights.y
                    + triangle[2].lightmap * v.weights.z,
                normal: triangle[0].normal * v.weights.x
                    + triangle[1].normal * v.weights.y
                    + triangle[2].normal * v.weights.z,
            };
        }
        output
    }

    fn vertex(&self, vertex: Vertex, bytes: &mut Vec<u8>) {
        let delta = vertex.position - self.origin;
        let tangent = self.axes[2];
        let uv = Vec2::new(tangent.dot(delta), tangent.cross(vertex.normal).dot(delta))
            * (0.5 / self.radius)
            + Vec2::splat(0.5);
        for value in [vertex.position.x, vertex.position.y, vertex.position.z, -1.] {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        bytes.extend_from_slice(&[255; 4]);
        for value in uv.to_array() {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        bytes.extend_from_slice(&super::geometry::pack_normal(vertex.normal).to_le_bytes());
        bytes.extend_from_slice(&super::geometry::pack_normal(tangent).to_le_bytes());
        for value in vertex.lightmap.to_array() {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
    }
}

// Keep the source half-inch classification, including rejection when every
// vertex lies on the plane. A conventional epsilon clip changes this behavior.
fn clip_plane(
    input: &[ClipVertex],
    output: &mut [ClipVertex; 14],
    normal: Vec3,
    distance: f32,
) -> usize {
    let mut distances = [0.; 14];
    let mut sides = [0; 14];
    let mut counts = [0; 3];
    for (i, v) in input.iter().enumerate() {
        let d = normal.dot(v.position) - distance;
        distances[i] = d;
        sides[i] = if d > 0.5 {
            0
        } else if d < -0.5 {
            1
        } else {
            2
        };
        counts[sides[i]] += 1;
    }
    if counts[0] == 0 {
        return 0;
    }
    if counts[1] == 0 {
        output[..input.len()].copy_from_slice(input);
        return input.len();
    }
    let mut count = 0;
    for i in 0..input.len() {
        let j = (i + 1) % input.len();
        if sides[i] != 1 {
            output[count] = input[i];
            count += 1;
        }
        if sides[i] != 2 && sides[j] != 2 && sides[i] != sides[j] {
            let t = distances[i] / (distances[i] - distances[j]);
            output[count] = ClipVertex {
                position: input[i].position + (input[j].position - input[i].position) * t,
                weights: input[i].weights + (input[j].weights - input[i].weights) * t,
            };
            count += 1;
        }
    }
    count
}

pub(crate) struct Receiver {
    pub entity: Entity,
    pub bounds: [Vec3; 2],
    pub vertex_offset: usize,
    pub first_index: usize,
    pub index_count: usize,
    pub surface_type_bits: u32,
    pub lightmap_index: usize,
    pub probe_index: usize,
    pub primary_light_index: u8,
    pub lightmap: Option<Handle<Image>>,
    pub probe: Handle<Image>,
    pub lighting: [Vec4; 3],
}

#[derive(Resource)]
pub(crate) struct World {
    pub primary: Arc<[u8]>,
    pub indices: Arc<[u8]>,
    // One group per source world/brush model; its surfaces share a transform.
    pub receivers: Vec<Vec<Receiver>>,
}

struct Definition {
    source: MaterialDefinition,
    bound: BTreeMap<(usize, usize), (Handle<NativeWorldMaterial>, NativeWorldState)>,
}

#[derive(Resource)]
pub(super) struct Materials(BTreeMap<String, Definition>);

impl Materials {
    pub(super) fn load(root: &std::path::Path, catalog: &super::Catalog) -> Self {
        let names: std::collections::BTreeSet<_> = catalog
            .effects
            .values()
            .flat_map(|effect| &effect.elements)
            .filter(|element| element.kind == 11)
            .flat_map(|element| &element.visuals)
            .flatten()
            .filter_map(|visual| visual.decal_materials.as_ref())
            .map(|pair| pair[1].trim_start_matches(','))
            .collect();
        Self(
            names
                .into_iter()
                .map(|name| {
                    (
                        name.to_owned(),
                        Definition {
                            source: MaterialDefinition::read(
                                &root.join(format!("materials/{name}.json")),
                            )
                            .unwrap_or_else(|error| panic!("World decal {name}: {error}")),
                            bound: BTreeMap::new(),
                        },
                    )
                })
                .collect(),
        )
    }
}

#[derive(Component)]
#[relationship(relationship_target = Draws)]
pub(super) struct Owner(Entity);

#[derive(Component, Default)]
#[relationship_target(relationship = Owner, linked_spawn)]
pub(super) struct Draws(Vec<Entity>);

#[derive(Component)]
pub(super) struct Color {
    value: [u8; 4],
    fade_seconds: f32,
}

impl Color {
    fn at(&self, age: f32, life: f32) -> Vec4 {
        let mut value = self.value;
        let fade_start = life - self.fade_seconds;
        if self.fade_seconds > 0. && age > fade_start {
            let fade = 1. - ((age - fade_start) / self.fade_seconds).clamp(0., 1.);
            value[3] = (f32::from(value[3]) * fade) as u8;
        }
        Vec4::from_array(value.map(|v| f32::from(v) / 255.))
    }
}

fn source_vertex(bytes: &[u8], offset: usize) -> Vertex {
    let f = |at| f32::from_le_bytes(bytes[offset + at..offset + at + 4].try_into().unwrap());
    let normal = u32::from_le_bytes(bytes[offset + 24..offset + 28].try_into().unwrap());
    let signed = |shift: u32| {
        (((normal >> shift) & 1023u32) as i32
            - if normal & (512u32 << shift) != 0 {
                1024
            } else {
                0
            }) as f32
            / 511.
    };
    let uv = |at| {
        f32::from(u16::from_le_bytes(
            bytes[offset + at..offset + at + 2].try_into().unwrap(),
        )) / 65535.
    };
    Vertex {
        position: Vec3::new(f(0), f(4), f(8)),
        normal: Vec3::new(signed(0), signed(10), signed(20)),
        lightmap: Vec2::new(uv(32), uv(34)),
    }
}

pub(super) fn spawn_world(
    commands: &mut Commands,
    owner: Entity,
    pose: Transform,
    radius: f32,
    rotation: f32,
    color: [f32; 4],
    fade_seconds: f32,
    name: &str,
    world: &World,
    definitions: &mut Materials,
    server: &AssetServer,
    materials: &mut Assets<NativeWorldMaterial>,
    geometries: &mut Assets<NativeWorldGeometry>,
    transforms: &Query<&GlobalTransform>,
) {
    if radius < 0.1 {
        return;
    }
    let definition = definitions
        .0
        .get_mut(name.trim_start_matches(','))
        .expect("Loaded source world mark material");
    let lighting = definition.source.mark_lighting();
    let world_normal = pose.rotation * Vec3::X;
    let world_tangent =
        Quat::from_axis_angle(world_normal, rotation) * (pose.rotation * Vec3::NEG_Z);
    let mut total_triangles = 0;
    let mut total_vertices = 0;
    commands.entity(owner).insert(Color {
        value: color.map(|value| value.clamp(0., 255.) as u8),
        fade_seconds,
    });
    for receivers in &world.receivers {
        let Ok(transform) = transforms.get(receivers[0].entity) else {
            continue;
        };
        let inverse = transform.to_matrix().inverse();
        let origin = inverse.transform_point3(pose.translation);
        let normal = inverse.transform_vector3(world_normal).normalize();
        let tangent = inverse.transform_vector3(world_tangent).normalize();
        let projection = Projection {
            origin,
            axes: [normal, tangent.cross(normal), tangent],
            radius,
        };
        let extent =
            (projection.axes[0].abs() + projection.axes[1].abs() + projection.axes[2].abs())
                * radius;
        for receiver in receivers {
            if lighting != (receiver.lightmap_index != 31, receiver.probe_index != 0)
                || receiver.surface_type_bits & definition.source.surface_type_bits
                    != definition.source.surface_type_bits
            {
                continue;
            }
            if (origin + extent).cmplt(receiver.bounds[0]).any()
                || (origin - extent).cmpgt(receiver.bounds[1]).any()
            {
                continue;
            }
            let mut vertices = Vec::new();
            let mut indices = Vec::new();
            let mut minimum = Vec3::splat(f32::INFINITY);
            let mut maximum = Vec3::splat(f32::NEG_INFINITY);
            for triangle in world.indices
                [receiver.first_index * 2..(receiver.first_index + receiver.index_count) * 2]
                .chunks_exact(6)
            {
                let source = std::array::from_fn(|i| {
                    source_vertex(
                        &world.primary,
                        receiver.vertex_offset
                            + usize::from(u16::from_le_bytes([
                                triangle[i * 2],
                                triangle[i * 2 + 1],
                            ])) * 36,
                    )
                });
                let clipped = projection.clip(source);
                let polygon = &clipped.vertices[..clipped.count];
                if polygon.len() < 3 {
                    continue;
                }
                if total_triangles + polygon.len() - 2 > 255 || total_vertices + polygon.len() > 765
                {
                    warn!("Original world decal fragment capacity reached for {name}");
                    break;
                }
                let base = (vertices.len() / 44) as u16;
                for i in 2..polygon.len() {
                    for index in [base + i as u16 - 1, base + i as u16, base] {
                        indices.extend_from_slice(&index.to_le_bytes());
                    }
                }
                total_triangles += polygon.len() - 2;
                total_vertices += polygon.len();
                for &vertex in polygon {
                    minimum = minimum.min(vertex.position);
                    maximum = maximum.max(vertex.position);
                    projection.vertex(vertex, &mut vertices);
                }
            }
            if indices.is_empty() {
                continue;
            }
            let (material, state) = definition
                .bound
                .entry((receiver.lightmap_index, receiver.probe_index))
                .or_insert_with(|| {
                    let (material, state) = definition
                        .source
                        .world(server, receiver.lightmap.clone(), receiver.probe.clone())
                        .unwrap_or_else(|error| panic!("World decal {name}: {error}"));
                    (materials.add(material), state)
                });
            let count = (indices.len() / 2) as u32;
            let geometry = geometries.add(NativeWorldGeometry {
                primary: vertices.into(),
                auxiliary: Vec::new(),
                indices: indices.into(),
                vertex_stride: 44,
            });
            commands.spawn((
                Owner(owner),
                ChildOf(receiver.entity),
                Transform::IDENTITY,
                Visibility::Inherited,
                Aabb::from_min_max(minimum, maximum),
                WorldSurfaceLighting(receiver.lighting),
                WorldSurfaceColor(Vec4::ONE),
                NativeWorldSurface {
                    pass: NativeWorldPass::Translucent,
                    sort_key: (u64::from(definition.source.sort_key) << 58)
                        | (u64::from(receiver.primary_light_index) << 44)
                        | ((receiver.probe_index as u64) << 25),
                    source_surface_index: owner.index_u32(),
                    geometry,
                    material: material.clone(),
                    state: *state,
                    primary_byte_offset: 0,
                    auxiliary_byte_offset: 0,
                    first_index: 0,
                    index_count: count,
                },
                Name::new(name.to_owned()),
            ));
        }
    }
}

pub(super) fn fade(
    marks: Query<(&super::Particle, &Color, &Draws)>,
    mut colors: Query<&mut WorldSurfaceColor>,
) {
    for (particle, color, draws) in &marks {
        let value = color.at(particle.age, particle.life);
        for draw in &draws.0 {
            if let Ok(mut target) = colors.get_mut(*draw) {
                if target.0 != value {
                    target.0 = value;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fade_preserves_birth_color_until_its_final_interval_and_truncates_alpha() {
        let color = Color {
            value: [32, 64, 128, 129],
            fade_seconds: 2.,
        };
        assert_eq!(color.at(0., 5.), color.at(3., 5.));
        assert_eq!(color.at(4., 5.).truncate(), color.at(0., 5.).truncate());
        assert_eq!(color.at(4., 5.).w, 64. / 255.);
        assert_eq!(color.at(5., 5.).w, 0.);
        let instant = Color {
            value: color.value,
            fade_seconds: 0.,
        };
        assert_eq!(instant.at(4., 5.).w, 129. / 255.);
    }

    #[test]
    fn clipping_preserves_source_half_inch_boundary_rules() {
        let mut output = [ClipVertex::default(); 14];
        let vertices = [-0.5, 0., 0.5].map(|x| ClipVertex {
            position: Vec3::new(x, 0., 0.),
            weights: Vec3::X,
        });
        assert_eq!(clip_plane(&vertices, &mut output, Vec3::X, 0.), 0);
        let mut vertices = vertices;
        vertices[2].position.x = 0.5001;
        assert_eq!(clip_plane(&vertices, &mut output, Vec3::X, 0.), 3);
        assert_eq!(output[0].position, vertices[0].position);
    }

    #[test]
    fn clipped_triangle_retains_area_winding_and_interpolated_inputs() {
        let projection = Projection {
            origin: Vec3::ZERO,
            axes: [Vec3::Z, Vec3::X, Vec3::Y],
            radius: 1.,
        };
        let triangle = [
            Vec3::new(-2., -2., 0.),
            Vec3::new(0., 2., 0.),
            Vec3::new(2., -2., 0.),
        ]
        .map(|position| Vertex {
            position,
            lightmap: position.truncate() * 0.1 + Vec2::splat(0.4),
            normal: Vec3::new(position.x * 0.1, position.y * 0.1, 0.8),
        });
        let polygon = projection.clip(triangle);
        assert_eq!(polygon.count, 6);
        let vertices = &polygon.vertices[..polygon.count];
        let mut twice_area = 0.;
        for (i, vertex) in vertices.iter().enumerate() {
            assert!(
                vertex
                    .lightmap
                    .abs_diff_eq(vertex.position.truncate() * 0.1 + Vec2::splat(0.4), 1e-6)
            );
            assert!(vertex.normal.abs_diff_eq(
                Vec3::new(vertex.position.x * 0.1, vertex.position.y * 0.1, 0.8),
                1e-6
            ));
            twice_area += vertex
                .position
                .cross(vertices[(i + 1) % vertices.len()].position)
                .z;
        }
        assert!((twice_area * 0.5 + 3.5).abs() < 1e-6);
        assert_eq!(
            projection
                .clip([triangle[2], triangle[1], triangle[0]])
                .count,
            0
        );
    }

    #[test]
    fn projection_rejects_triangles_facing_more_than_sixty_degrees_away() {
        let triangle = [
            Vec3::new(-2., -2., 0.),
            Vec3::new(0., 2., 0.),
            Vec3::new(2., -2., 0.),
        ]
        .map(|position| Vertex {
            position,
            ..default()
        });
        for (angle, expected) in [(59.9_f32, 3), (60.1, 0), (180., 0)] {
            let rotation = Quat::from_rotation_x(angle.to_radians());
            let projection = Projection {
                origin: Vec3::ZERO,
                axes: [rotation * Vec3::Z, rotation * Vec3::X, rotation * Vec3::Y],
                radius: 10.,
            };
            assert_eq!(projection.clip(triangle).count, expected);
        }
    }

    #[test]
    fn mark_vertices_keep_full_precision_uv_and_lightmap_coordinates() {
        let projection = Projection {
            origin: Vec3::ZERO,
            axes: [Vec3::Z, Vec3::NEG_Y, Vec3::X],
            radius: 7.,
        };
        let vertex = Vertex {
            position: Vec3::new(1.234567, 2.345678, 0.),
            lightmap: Vec2::new(0.1234567, 0.7654321),
            normal: Vec3::Z * 0.8,
        };
        let mut bytes = Vec::new();
        projection.vertex(vertex, &mut bytes);
        assert_eq!(bytes.len(), 44);
        let f = |offset| f32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap());
        assert_eq!(f(12), -1.);
        assert_eq!(&bytes[16..20], &[255; 4]);
        assert_eq!(f(20), vertex.position.x * (0.5 / 7.) + 0.5);
        assert_eq!(f(24), -vertex.position.y * 0.8 * (0.5 / 7.) + 0.5);
        assert_eq!(f(36), vertex.lightmap.x);
        assert_eq!(f(40), vertex.lightmap.y);
    }

    #[test]
    fn expiring_mark_removes_draws_without_removing_receiver() {
        let mut world = bevy::prelude::World::new();
        let receiver = world.spawn_empty().id();
        let mark = world.spawn_empty().id();
        let draw = world.spawn((Owner(mark), ChildOf(receiver))).id();
        assert_eq!(world.get::<Draws>(mark).unwrap().0, vec![draw]);
        world.despawn(mark);
        assert!(world.get_entity(draw).is_err());
        assert!(world.get_entity(receiver).is_ok());
    }

    #[test]
    #[ignore = "requires the authorized local Origins export"]
    fn exported_dirt_floor_receives_a_clockwise_mark() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("target/assets");
        let world: serde_json::Value =
            serde_json::from_slice(&std::fs::read(root.join("world.json")).unwrap()).unwrap();
        let surface = &world["surfaces"][347];
        assert_eq!(surface["sourceSurfaceIndex"], 347);
        assert_eq!(
            surface["material"],
            "*5n_34n(wpc/zm_tm_dirt01:wpc/zm_tm_decal_gibs_02_blend)"
        );
        let primary = std::fs::read(root.join(world["vertexDataFile0"].as_str().unwrap())).unwrap();
        let indices = std::fs::read(root.join(world["indexFile"].as_str().unwrap())).unwrap();
        let first = surface["baseIndex"].as_u64().unwrap() as usize * 2;
        let offset = surface["vertexDataOffset0"].as_u64().unwrap() as usize;
        let triangle = std::array::from_fn(|i| {
            source_vertex(
                &primary,
                offset
                    + usize::from(u16::from_le_bytes(
                        indices[first + i * 2..first + i * 2 + 2]
                            .try_into()
                            .unwrap(),
                    )) * 36,
            )
        });
        assert_eq!(triangle[0].position, Vec3::new(2989., 5276., -380.));
        for vertex in triangle {
            assert!(vertex.normal.abs_diff_eq(Vec3::Z, 0.01));
        }
        let center = (triangle[0].position + triangle[1].position + triangle[2].position) / 3.;
        let projection = Projection {
            origin: center,
            axes: [Vec3::Z, Vec3::NEG_Y, Vec3::X],
            radius: 8.,
        };
        let polygon = projection.clip(triangle);
        assert_eq!(polygon.count, 4);
        let vertices = &polygon.vertices[..polygon.count];
        let twice_area: f32 = vertices
            .iter()
            .enumerate()
            .map(|(i, vertex)| {
                (vertex.position - center)
                    .cross(vertices[(i + 1) % vertices.len()].position - center)
                    .z
            })
            .sum();
        assert!((twice_area * 0.5 + 256.).abs() < 0.02);
        let frame = crate::rendering::world::source_frame();
        assert!(
            frame
                .inverse()
                .transform_point3(frame.transform_point3(center))
                .abs_diff_eq(center, 0.001)
        );
    }
}

//! Pose-correct ray contributors using Bevy's joint poses and native Solari BLAS.
//! Immutable bind data is shared; each live skin owns only its deformed provider.
use bevy::{
    asset::{AssetId, RenderAssetUsages},
    mesh::{
        Indices, PrimitiveTopology, VertexAttributeValues,
        skinning::{SkinnedMesh, SkinnedMeshInverseBindposes},
    },
    prelude::*,
};
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

#[derive(Resource, Default)]
pub(super) struct Sources(pub HashMap<AssetId<Mesh>, Arc<Source>>);

pub(super) struct Source {
    accessor_vertices: usize,
    original_vertices: Vec<u32>,
    positions: Vec<[f32; 3]>,
    normals: Vec<[f32; 3]>,
    tangents: Vec<[f32; 4]>,
    uv: Vec<[f32; 2]>,
    indices: Vec<u32>,
    joints: Vec<[u16; 4]>,
    weights: Vec<[f32; 4]>,
    used_joints: Vec<usize>,
    maximum_joint: usize,
}

impl Source {
    /// Joint membership for the native raster primitive, without copying its
    /// geometry or depending on whether the material can enter the ray scene.
    pub(super) fn indexed_joints(mesh: &Mesh) -> Option<Vec<usize>> {
        let VertexAttributeValues::Float32x3(positions) =
            mesh.attribute(Mesh::ATTRIBUTE_POSITION)?
        else {
            return None;
        };
        let VertexAttributeValues::Uint16x4(joints) =
            mesh.attribute(Mesh::ATTRIBUTE_JOINT_INDEX)?
        else {
            return None;
        };
        let VertexAttributeValues::Float32x4(weights) =
            mesh.attribute(Mesh::ATTRIBUTE_JOINT_WEIGHT)?
        else {
            return None;
        };
        if positions.is_empty()
            || positions.len() != joints.len()
            || positions.len() != weights.len()
            || positions.iter().flatten().any(|n| !n.is_finite())
            || weights.iter().any(|w| {
                let total = w.iter().sum::<f32>();
                w.iter().any(|n| !n.is_finite() || *n < 0.) || !total.is_finite() || total <= 0.
            })
        {
            return None;
        }
        let mut used_joints = HashSet::new();
        let mut referenced = 0;
        let mut collect_vertex = |index: usize| -> Option<()> {
            for (joint, weight) in joints.get(index)?.iter().zip(weights.get(index)?) {
                if *weight > 0. {
                    used_joints.insert(*joint as usize);
                }
            }
            referenced += 1;
            Some(())
        };
        if let Some(indices) = mesh.indices() {
            for index in indices.iter() {
                collect_vertex(index)?;
            }
        } else {
            for index in 0..positions.len() {
                collect_vertex(index)?;
            }
        }
        if referenced == 0 {
            return None;
        }
        let mut used_joints: Vec<_> = used_joints.into_iter().collect();
        used_joints.sort_unstable();
        Some(used_joints)
    }

    pub(super) fn read(mesh: &Mesh) -> Option<Self> {
        if mesh.primitive_topology() != PrimitiveTopology::TriangleList {
            return None;
        }
        let VertexAttributeValues::Float32x3(positions) =
            mesh.attribute(Mesh::ATTRIBUTE_POSITION)?
        else {
            return None;
        };
        let VertexAttributeValues::Float32x3(normals) = mesh.attribute(Mesh::ATTRIBUTE_NORMAL)?
        else {
            return None;
        };
        let VertexAttributeValues::Float32x4(tangents) = mesh.attribute(Mesh::ATTRIBUTE_TANGENT)?
        else {
            return None;
        };
        let VertexAttributeValues::Float32x2(uv) = mesh.attribute(Mesh::ATTRIBUTE_UV_0)? else {
            return None;
        };
        let VertexAttributeValues::Uint16x4(joints) =
            mesh.attribute(Mesh::ATTRIBUTE_JOINT_INDEX)?
        else {
            return None;
        };
        let VertexAttributeValues::Float32x4(weights) =
            mesh.attribute(Mesh::ATTRIBUTE_JOINT_WEIGHT)?
        else {
            return None;
        };
        if [
            normals.len(),
            tangents.len(),
            uv.len(),
            joints.len(),
            weights.len(),
        ]
        .iter()
        .any(|n| *n != positions.len())
        {
            return None;
        }
        let indices: Vec<_> = mesh.indices()?.iter().map(|i| i as u32).collect();
        if positions.is_empty()
            || indices.is_empty()
            || !indices.len().is_multiple_of(3)
            || indices.iter().any(|i| *i as usize >= positions.len())
            || positions.iter().flatten().any(|n| !n.is_finite())
            || normals.iter().flatten().any(|n| !n.is_finite())
            || tangents.iter().flatten().any(|n| !n.is_finite())
            || uv.iter().flatten().any(|n| !n.is_finite())
            || weights.iter().any(|w| {
                let total = w.iter().sum::<f32>();
                w.iter().any(|n| !n.is_finite() || *n < 0.) || !total.is_finite() || total <= 0.
            })
        {
            return None;
        }
        // Material surfaces can share a whole rig's vertex accessor while
        // indexing disjoint triangles. Keep every triangle in source order,
        // but deform and upload only the vertices that it references. Original
        // vertex IDs still address the authored surface-attribute sidecar.
        let used_joints = Self::indexed_joints(mesh)?;
        let maximum_joint = *used_joints.last()?;
        let mut remap = vec![u32::MAX; positions.len()];
        let mut original_vertices = Vec::new();
        let indices = indices
            .into_iter()
            .map(|index| {
                let mapped = &mut remap[index as usize];
                if *mapped == u32::MAX {
                    *mapped = original_vertices.len() as u32;
                    original_vertices.push(index);
                }
                *mapped
            })
            .collect();
        Some(Self {
            accessor_vertices: positions.len(),
            positions: original_vertices
                .iter()
                .map(|&i| positions[i as usize])
                .collect(),
            normals: original_vertices
                .iter()
                .map(|&i| normals[i as usize])
                .collect(),
            tangents: original_vertices
                .iter()
                .map(|&i| tangents[i as usize])
                .collect(),
            uv: original_vertices.iter().map(|&i| uv[i as usize]).collect(),
            indices,
            joints: original_vertices
                .iter()
                .map(|&i| joints[i as usize])
                .collect(),
            weights: original_vertices
                .iter()
                .map(|&i| weights[i as usize])
                .collect(),
            original_vertices,
            used_joints,
            maximum_joint,
        })
    }

    pub(super) fn mesh(&self, vertex_offset: u32) -> Mesh {
        self.deformed_mesh(
            self.positions.clone(),
            self.normals.clone(),
            self.tangents.clone(),
            vertex_offset,
        )
    }

    fn deformed_mesh(
        &self,
        positions: Vec<[f32; 3]>,
        normals: Vec<[f32; 3]>,
        tangents: Vec<[f32; 4]>,
        vertex_offset: u32,
    ) -> Mesh {
        let mut mesh = Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::RENDER_WORLD,
        );
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
        mesh.insert_attribute(Mesh::ATTRIBUTE_TANGENT, tangents);
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, self.uv.clone());
        mesh.insert_indices(Indices::U32(self.indices.clone()));
        mesh.enable_raytracing = true;
        super::ray_surface::encode_mapped_vertices(
            &mut mesh,
            vertex_offset,
            &self.original_vertices,
        );
        mesh
    }
}

#[derive(Component)]
pub(super) struct Skin {
    pub owner: Entity,
    pub source: Arc<Source>,
    pub mesh: Handle<Mesh>,
    pub previous: Vec<Mat4>,
    pub vertex_offset: u32,
}

#[derive(Component)]
pub(super) struct Provider {
    pub owner: Entity,
    pub mesh: Handle<Mesh>,
}

pub(super) fn visibility(
    mut commands: Commands,
    owners: Query<&InheritedVisibility>,
    providers: Query<(
        Entity,
        &Provider,
        Option<&bevy::solari::scene::RaytracingMesh3d>,
        Option<&Skin>,
    )>,
    mut revision: ResMut<Revision>,
    mut stats: ResMut<RayContributorStats>,
) {
    stats.admitted_providers = 0;
    stats.admitted_skins = 0;
    stats.active_skin_source_vertices = 0;
    stats.active_skin_vertices = 0;
    for (entity, provider, ray, skin) in &providers {
        let visible = owners
            .get(provider.owner)
            .is_ok_and(|visible| visible.get())
            // Never expose an uninitialized bind pose or a pose whose joints
            // vanished to the world-lighting scene.
            && skin.is_none_or(|skin| !skin.previous.is_empty());
        stats.admitted_providers += usize::from(visible);
        stats.admitted_skins += usize::from(visible && skin.is_some());
        if visible && let Some(skin) = skin {
            stats.active_skin_source_vertices += skin.source.accessor_vertices;
            stats.active_skin_vertices += skin.source.positions.len();
        }
        if visible && ray.is_none_or(|ray| ray.0.id() != provider.mesh.id()) {
            commands
                .entity(entity)
                .insert(bevy::solari::scene::RaytracingMesh3d(provider.mesh.clone()));
            revision.0 = revision.0.wrapping_add(1);
        } else if !visible && ray.is_some() {
            commands
                .entity(entity)
                .remove::<bevy::solari::scene::RaytracingMesh3d>();
            revision.0 = revision.0.wrapping_add(1);
        }
    }
}

#[derive(Resource, Default)]
pub(super) struct Revision(pub u64);

#[derive(Resource, Default, serde::Serialize)]
pub(crate) struct RayContributorStats {
    admitted_providers: usize,
    admitted_skins: usize,
    pose_updates: u64,
    ignored_joint_pose_updates: u64,
    active_skin_source_vertices: usize,
    active_skin_vertices: usize,
    deformed_source_vertices: u64,
    deformed_vertices: u64,
}

pub(super) fn deform(
    owners: Query<(&SkinnedMesh, &GlobalTransform, &InheritedVisibility)>,
    joints: Query<&GlobalTransform>,
    inverse_binds: Res<Assets<SkinnedMeshInverseBindposes>>,
    mut providers: Query<(&mut Skin, &mut Provider)>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut revision: ResMut<Revision>,
    mut sources: ResMut<Sources>,
    mut pose: Local<Vec<Mat4>>,
    mut stats: ResMut<RayContributorStats>,
) {
    let _span = info_span!("ray_skin_deformation").entered();
    for (mut provider, mut owner) in &mut providers {
        let Ok((skin, transform, visibility)) = owners.get(provider.owner) else {
            provider.previous.clear();
            continue;
        };
        if !visibility.get() {
            continue;
        }
        let Some(inverse) = inverse_binds.get(&skin.inverse_bindposes) else {
            provider.previous.clear();
            continue;
        };
        if skin.joints.len() != inverse.len() || provider.source.maximum_joint >= skin.joints.len()
        {
            provider.previous.clear();
            continue;
        }
        let world_from_mesh = transform.to_matrix();
        if !world_from_mesh.is_finite() || world_from_mesh.determinant().abs() <= 1e-8 {
            provider.previous.clear();
            continue;
        }
        let mesh_from_world = world_from_mesh.inverse();
        // One retained scratch allocation services every provider. Rest poses
        // do not allocate an intermediate vector or emit a mesh asset event.
        pose.clear();
        for (joint, bind) in skin.joints.iter().zip(inverse.iter()) {
            let Ok(joint) = joints.get(*joint) else { break };
            let matrix = mesh_from_world * joint.to_matrix() * *bind;
            if !matrix.is_finite() {
                break;
            }
            pose.push(matrix);
        }
        if pose.len() != skin.joints.len() {
            provider.previous.clear();
            continue;
        }
        // A rig's dial can animate while its housing uses only the root joint.
        // Compare actual mesh-space matrices for nonzero influences, so parent
        // motion still invalidates children but unrelated joints do not upload
        // an identical mesh or rebuild its native acceleration structure.
        if pose.len() == provider.previous.len()
            && provider
                .source
                .used_joints
                .iter()
                .all(|&joint| pose[joint] == provider.previous[joint])
        {
            if *pose != provider.previous {
                provider.previous.copy_from_slice(&pose);
                stats.ignored_joint_pose_updates += 1;
            }
            continue;
        }
        let source = &provider.source;
        let mut positions = Vec::with_capacity(source.positions.len());
        let mut normals = Vec::with_capacity(source.positions.len());
        let mut tangents = Vec::with_capacity(source.positions.len());
        for index in 0..source.positions.len() {
            let mut matrix = Mat4::ZERO;
            for lane in 0..4 {
                let weight = source.weights[index][lane];
                if weight > 0. {
                    matrix += pose[source.joints[index][lane] as usize] * weight;
                }
            }
            let position =
                (matrix * Vec3::from_array(source.positions[index]).extend(1.)).truncate();
            // Hidden animation bones can collapse to a singular scale. Keep
            // finite normals for their degenerate triangles instead of feeding
            // NaNs to native acceleration-structure preparation.
            let source_normal = Vec3::from_array(source.normals[index]);
            let linear = Mat3::from_mat4(matrix);
            let cofactor = Mat3::from_cols(
                linear.y_axis.cross(linear.z_axis),
                linear.z_axis.cross(linear.x_axis),
                linear.x_axis.cross(linear.y_axis),
            );
            let determinant = linear.x_axis.dot(cofactor.x_axis);
            let normal = if determinant.abs() > 1e-8 {
                cofactor * source_normal * determinant.signum()
            } else {
                source_normal
            }
            .normalize_or_zero();
            let tangent =
                (linear * Vec4::from_array(source.tangents[index]).truncate()).normalize_or_zero();
            positions.push(position.to_array());
            normals.push(normal.to_array());
            tangents.push(tangent.extend(source.tangents[index][3]).to_array());
        }
        let mesh = source.deformed_mesh(positions, normals, tangents, provider.vertex_offset);
        stats.deformed_source_vertices += source.accessor_vertices as u64;
        stats.deformed_vertices += source.positions.len() as u64;
        // Bevy 0.19.1's pending compaction queue identifies BLAS by asset ID.
        // Replacing that ID while compaction is pending can prepare the newest
        // BLAS twice. Immutable pose generations retain native compaction and
        // asset retirement without aliasing a queued acceleration structure.
        let handle = meshes.add(mesh);
        owner.mesh = handle.clone();
        provider.mesh = handle;
        provider.previous.clear();
        provider.previous.extend_from_slice(&pose);
        revision.0 = revision.0.wrapping_add(1);
        stats.pose_updates += 1;
    }
    // Despawned actors release their extra immutable bind-data copy. Late
    // instances can recreate it from the retained skeletal source mesh.
    sources.0.retain(|_, source| Arc::strong_count(source) > 1);
}

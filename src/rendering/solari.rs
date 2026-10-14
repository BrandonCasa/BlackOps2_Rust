use bevy::{
    anti_alias::dlss::{DlssRayReconstructionSupported, DlssSuperResolutionSupported},
    asset::RenderAssetUsages,
    mesh::{
        Indices, VertexAttributeValues,
        skinning::{SkinnedMesh, SkinnedMeshInverseBindposes},
    },
    prelude::*,
    solari::prelude::RaytracingMesh3d,
};
use std::collections::HashMap;

pub(super) struct OriginsSolariPlugin;

impl Plugin for OriginsSolariPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(bevy::solari::SolariPlugins)
            .add_systems(Startup, require_dlss)
            .add_systems(
                PostUpdate,
                prepare_meshes
                    .after(super::MaterialAnimation)
                    .after(bevy::transform::TransformSystems::Propagate)
                    .after(super::SurfacePreparation)
                    .after(bevy::camera::visibility::VisibilitySystems::VisibilityPropagate),
            );
    }
}

fn require_dlss(
    sr: Option<Res<DlssSuperResolutionSupported>>,
    rr: Option<Res<DlssRayReconstructionSupported>>,
) {
    assert!(
        sr.is_some() && rr.is_some(),
        "Origins requires NVIDIA DLSS Super Resolution and Ray Reconstruction on Vulkan. Install the DLSS runtime and use a supported NVIDIA RTX GPU and driver."
    );
}

#[derive(Component)]
struct RayMeshSource {
    source: AssetId<Mesh>,
    skinned: bool,
    pose: Vec<Mat4>,
    uv_transform: bevy::math::Affine2,
}

fn prepare_meshes(
    mut commands: Commands,
    surfaces: Query<
        (
            Entity,
            &Mesh3d,
            &GlobalTransform,
            &InheritedVisibility,
            Option<&SkinnedMesh>,
            Option<&RaytracingMesh3d>,
            Option<&RayMeshSource>,
            &MeshMaterial3d<StandardMaterial>,
        ),
        (
            With<MeshMaterial3d<StandardMaterial>>,
            Without<crate::assets::WorldCollider>,
        ),
    >,
    transforms: Query<&GlobalTransform>,
    bindposes: Res<Assets<SkinnedMeshInverseBindposes>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut cache: Local<HashMap<AssetId<Mesh>, Handle<Mesh>>>,
    materials: Res<Assets<StandardMaterial>>,
    mut events: MessageReader<AssetEvent<Mesh>>,
) {
    let changed: std::collections::HashSet<_> = events
        .read()
        .filter_map(|event| match event {
            AssetEvent::Modified { id } => meshes
                .get(*id)
                .filter(|mesh| mesh.try_attributes().is_ok())
                .map(|_| *id),
            AssetEvent::Removed { id } => Some(*id),
            _ => None,
        })
        .collect();
    cache.retain(|id, _| !changed.contains(id) && meshes.contains(*id));
    for (entity, raster, transform, visibility, skin, current, prepared, material) in &surfaces {
        let Some(material) = materials.get(&material.0) else {
            continue;
        };
        if !visibility.get()
            || material.unlit
            || material.specular_transmission > 0.
            || !matches!(material.alpha_mode, AlphaMode::Opaque | AlphaMode::Mask(_))
        {
            if current.is_some() {
                commands.entity(entity).remove::<RaytracingMesh3d>();
            }
            continue;
        }
        if let Some(prepared) = prepared
            && prepared.source == raster.0.id()
            && !prepared.skinned
            && skin.is_none()
            && prepared.uv_transform == material.uv_transform
            && !changed.contains(&raster.0.id())
            && current.is_some()
        {
            continue;
        }
        if skin.is_none()
            && material.uv_transform == bevy::math::Affine2::IDENTITY
            && let Some(handle) = cache.get(&raster.0.id())
        {
            commands.entity(entity).insert((
                RaytracingMesh3d(handle.clone()),
                RayMeshSource {
                    source: raster.0.id(),
                    skinned: false,
                    pose: Vec::new(),
                    uv_transform: material.uv_transform,
                },
            ));
            continue;
        }
        let matrices = if let Some(skin) = skin {
            let Some(bindposes) = bindposes.get(&skin.inverse_bindposes) else {
                continue;
            };
            let inverse = transform.affine().inverse();
            let Some(matrices) = skin
                .joints
                .iter()
                .zip(bindposes.iter())
                .map(|(joint, bind)| {
                    transforms
                        .get(*joint)
                        .ok()
                        .map(|global| Mat4::from(inverse * global.affine()) * *bind)
                })
                .collect::<Option<Vec<_>>>()
            else {
                continue;
            };
            if current.is_some()
                && prepared.is_some_and(|p| {
                    p.source == raster.0.id()
                        && p.pose == matrices
                        && p.uv_transform == material.uv_transform
                })
                && !changed.contains(&raster.0.id())
            {
                continue;
            }
            matrices
        } else {
            Vec::new()
        };
        let Some(source) = meshes.get(&raster.0) else {
            continue;
        };
        let Some(VertexAttributeValues::Float32x3(positions)) =
            source.attribute(Mesh::ATTRIBUTE_POSITION)
        else {
            continue;
        };
        let count = positions.len();
        let mut positions = positions.clone();
        let mut normals = match source.attribute(Mesh::ATTRIBUTE_NORMAL) {
            Some(VertexAttributeValues::Float32x3(values)) => values.clone(),
            _ => vec![[0., 1., 0.]; count],
        };
        let mut uv = match source.attribute(Mesh::ATTRIBUTE_UV_0) {
            Some(VertexAttributeValues::Float32x2(values)) => values.clone(),
            _ => vec![[0.; 2]; count],
        };
        for value in &mut uv {
            *value = material
                .uv_transform
                .transform_point2(Vec2::from_array(*value))
                .to_array();
        }
        let mut tangents = match source.attribute(Mesh::ATTRIBUTE_TANGENT) {
            Some(VertexAttributeValues::Float32x4(values)) => values.clone(),
            _ => vec![[1., 0., 0., 1.]; count],
        };
        let indices: Vec<u32> = source
            .indices()
            .map(|indices| indices.iter().map(|i| i as u32).collect())
            .unwrap_or_else(|| (0..count as u32).collect());
        if skin.is_some() {
            let Some(VertexAttributeValues::Uint16x4(joints)) =
                source.attribute(Mesh::ATTRIBUTE_JOINT_INDEX)
            else {
                continue;
            };
            let Some(VertexAttributeValues::Float32x4(weights)) =
                source.attribute(Mesh::ATTRIBUTE_JOINT_WEIGHT)
            else {
                continue;
            };
            for i in 0..count {
                let mut matrix = Mat4::ZERO;
                for influence in 0..4 {
                    let weight = weights[i][influence];
                    if weight != 0. {
                        let Some(joint) = matrices.get(joints[i][influence] as usize) else {
                            continue;
                        };
                        matrix += *joint * weight;
                    }
                }
                positions[i] = matrix
                    .transform_point3(Vec3::from_array(positions[i]))
                    .to_array();
                let normal_matrix = Mat3::from_mat4(matrix).inverse().transpose();
                normals[i] = (normal_matrix * Vec3::from_array(normals[i]))
                    .normalize_or(Vec3::Y)
                    .to_array();
                let tangent = matrix
                    .transform_vector3(Vec3::new(tangents[i][0], tangents[i][1], tangents[i][2]))
                    .normalize_or(Vec3::X);
                tangents[i] = [tangent.x, tangent.y, tangent.z, tangents[i][3]];
            }
        }
        let handle = if let Some(current) = current.filter(|_| {
            prepared.is_some_and(|p| {
                p.source == raster.0.id()
                    && (p.skinned || p.uv_transform != bevy::math::Affine2::IDENTITY)
            })
        }) {
            let Some(mut proxy) = meshes.get_mut(&current.0) else {
                continue;
            };
            proxy.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
            proxy.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
            proxy.insert_attribute(Mesh::ATTRIBUTE_TANGENT, tangents);
            proxy.insert_attribute(Mesh::ATTRIBUTE_UV_0, uv);
            current.0.clone()
        } else {
            let mut proxy = Mesh::new(source.primitive_topology(), RenderAssetUsages::default());
            proxy.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
            proxy.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
            proxy.insert_attribute(Mesh::ATTRIBUTE_UV_0, uv);
            proxy.insert_attribute(Mesh::ATTRIBUTE_TANGENT, tangents);
            proxy.insert_indices(Indices::U32(indices));
            proxy.enable_raytracing = true;
            meshes.add(proxy)
        };
        if skin.is_none() && material.uv_transform == bevy::math::Affine2::IDENTITY {
            cache.insert(raster.0.id(), handle.clone());
        }
        commands.entity(entity).insert((
            RaytracingMesh3d(handle),
            RayMeshSource {
                source: raster.0.id(),
                skinned: skin.is_some(),
                pose: matrices,
                uv_transform: material.uv_transform,
            },
        ));
    }
}

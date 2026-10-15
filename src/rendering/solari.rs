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
use std::collections::{HashMap, HashSet};

pub(super) struct OriginsSolariPlugin;

impl Plugin for OriginsSolariPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(bevy::solari::SolariPlugins)
            .add_systems(Startup, require_dlss)
            .add_systems(
                PostUpdate,
                prepare_meshes
                    .after(super::MaterialAnimation)
                    .after(super::opaque::animate_flags)
                    .after(super::lod::select)
                    .after(bevy::transform::TransformSystems::Propagate)
                    .after(super::SurfacePreparation)
                    .after(bevy::camera::visibility::VisibilitySystems::VisibilityPropagate)
                    .before(bevy::asset::AssetEventSystems),
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
pub(super) struct RayMeshSource {
    source: AssetId<Mesh>,
    proxy: Handle<Mesh>,
    ownership: ProxyOwnership,
    skinned: bool,
    deformed: bool,
    pose: Vec<Mat4>,
    flag_time: Option<f32>,
    uv_transform: bevy::math::Affine2,
    invalidated: bool,
}

impl RayMeshSource {
    pub(super) fn source_id(&self) -> AssetId<Mesh> {
        self.source
    }

    pub(super) fn ready_for(&self, source: AssetId<Mesh>, meshes: &Assets<Mesh>) -> bool {
        self.source == source && !self.invalidated && meshes.contains(self.proxy.id())
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ProxyOwnership {
    Shared,
    Private,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub(super) struct RayMeshKey {
    source: AssetId<Mesh>,
    uv: [u32; 6],
}

impl RayMeshKey {
    fn new(source: AssetId<Mesh>, uv: bevy::math::Affine2) -> Self {
        Self {
            source,
            uv: [
                uv.matrix2.x_axis.x,
                uv.matrix2.x_axis.y,
                uv.matrix2.y_axis.x,
                uv.matrix2.y_axis.y,
                uv.translation.x,
                uv.translation.y,
            ]
            .map(|value| if value == 0. { 0 } else { value.to_bits() }),
        }
    }
}

#[derive(Default)]
pub(super) struct RayMeshScratch {
    changed: HashSet<AssetId<Mesh>>,
    live: HashSet<RayMeshKey>,
    pose: Vec<Mat4>,
    normal_pose: Vec<Mat3>,
    positions: Vec<[f32; 3]>,
    normals: Vec<[f32; 3]>,
    tangents: Vec<[f32; 4]>,
    uv: Vec<[f32; 2]>,
}

pub(super) fn prepare_meshes(
    mut commands: Commands,
    mut surfaces: Query<
        (
            Entity,
            &Mesh3d,
            &GlobalTransform,
            &InheritedVisibility,
            Option<&SkinnedMesh>,
            Option<&RaytracingMesh3d>,
            Option<&mut RayMeshSource>,
            &MeshMaterial3d<StandardMaterial>,
            Option<&super::opaque::FlagMotion>,
            Has<super::opaque::UvAnimation>,
            Has<super::opaque::AnimatedMaterial>,
            Has<super::DynamicSurface>,
            Option<&super::lod::NativeLod>,
            Option<&super::hybrid::RayOwner>,
        ),
        (
            With<MeshMaterial3d<StandardMaterial>>,
            Without<crate::assets::WorldCollider>,
            Without<super::opaque::ScalarRayPrimary>,
        ),
    >,
    transforms: Query<&GlobalTransform>,
    lods: Query<&super::lod::NativeLod>,
    bindposes: Res<Assets<SkinnedMeshInverseBindposes>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut cache: Local<HashMap<RayMeshKey, Handle<Mesh>>>,
    mut scratch: Local<RayMeshScratch>,
    materials: Res<Assets<StandardMaterial>>,
    mut events: MessageReader<AssetEvent<Mesh>>,
) {
    let scratch = &mut *scratch;
    scratch.changed.clear();
    scratch.live.clear();
    scratch.changed.extend(events.read().filter_map(|event| {
        match event {
            AssetEvent::Modified { id } => meshes
                .get(*id)
                .filter(|mesh| mesh.try_attributes().is_ok())
                .map(|_| *id),
            AssetEvent::Removed { id } => Some(*id),
            _ => None,
        }
    }));
    cache.retain(|key, proxy| {
        !scratch.changed.contains(&key.source)
            && meshes.contains(key.source)
            && meshes.contains(proxy.id())
    });
    for (
        entity,
        raster,
        transform,
        visibility,
        skin,
        current,
        mut prepared,
        material,
        flag,
        uv_animation,
        animated_material,
        dynamic,
        native_lod,
        ray_owner,
    ) in &mut surfaces
    {
        let source_id = raster.0.id();
        if !meshes.contains(source_id) {
            commands
                .entity(entity)
                .remove::<(RaytracingMesh3d, RayMeshSource)>();
            continue;
        }
        if prepared.as_deref().is_some_and(|prepared| {
            prepared.source != source_id || (flag.is_none() && scratch.changed.contains(&source_id))
        }) {
            if prepared
                .as_deref()
                .is_some_and(|prepared| !prepared.invalidated)
            {
                prepared.as_deref_mut().unwrap().invalidated = true;
            }
            if current.is_some() {
                commands.entity(entity).remove::<RaytracingMesh3d>();
            }
        }
        if let Some(prepared) = prepared.as_deref()
            && prepared.ownership == ProxyOwnership::Shared
            && !prepared.invalidated
        {
            scratch
                .live
                .insert(RayMeshKey::new(prepared.source, prepared.uv_transform));
        }
        let native_lod = native_lod.or_else(|| ray_owner.and_then(|owner| lods.get(owner.0).ok()));
        if native_lod.is_some_and(|lod| {
            if skin.is_some() {
                !lod.is_ray_selected()
            } else {
                !lod.is_selected()
            }
        }) {
            if current.is_some() {
                commands.entity(entity).remove::<RaytracingMesh3d>();
            }
            continue;
        }
        let Some(material) = materials.get(&material.0) else {
            if current.is_some() {
                commands.entity(entity).remove::<RaytracingMesh3d>();
            }
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
        let ray_uv = if uv_animation || animated_material || dynamic {
            prepared
                .as_deref()
                .map_or(material.uv_transform, |prepared| prepared.uv_transform)
        } else {
            material.uv_transform
        };
        let shared = skin.is_none() && flag.is_none();
        let key = RayMeshKey::new(source_id, ray_uv);
        let flag_time = flag.and_then(super::opaque::FlagMotion::deformation_time);
        scratch.pose.clear();
        if let Some(skin) = skin {
            let Some(bindposes) = bindposes.get(&skin.inverse_bindposes) else {
                continue;
            };
            let inverse = transform.affine().inverse();
            let mut ready = true;
            for (joint, bind) in skin.joints.iter().zip(bindposes.iter()) {
                let Ok(global) = transforms.get(*joint) else {
                    ready = false;
                    break;
                };
                scratch
                    .pose
                    .push(Mat4::from(inverse * global.affine()) * *bind);
            }
            if !ready {
                continue;
            }
        } else if let Some(flag) = flag {
            scratch.pose.extend_from_slice(flag.deformation_pose());
        }
        if let Some(prepared) = prepared.as_deref()
            && !prepared.invalidated
            && prepared.source == source_id
            && prepared.skinned == skin.is_some()
            && prepared.deformed == flag.is_some()
            && prepared.flag_time == flag_time
            && prepared.pose == scratch.pose
            && prepared.uv_transform == ray_uv
            && (prepared.ownership == ProxyOwnership::Private || shared)
            && meshes.contains(prepared.proxy.id())
        {
            if current.is_none_or(|current| current.0 != prepared.proxy) {
                commands
                    .entity(entity)
                    .insert(RaytracingMesh3d(prepared.proxy.clone()));
            }
            continue;
        }
        let cached = shared.then(|| cache.get(&key).cloned()).flatten();
        let handle = if let Some(handle) = cached {
            handle
        } else {
            let source = meshes.get(source_id).unwrap();
            if source.try_attributes().is_err() {
                continue;
            }
            let Some(VertexAttributeValues::Float32x3(positions)) =
                source.attribute(Mesh::ATTRIBUTE_POSITION)
            else {
                continue;
            };
            let count = positions.len();
            scratch.positions.clone_from(positions);
            match source.attribute(Mesh::ATTRIBUTE_NORMAL) {
                Some(VertexAttributeValues::Float32x3(values)) => {
                    scratch.normals.clone_from(values)
                }
                _ => {
                    scratch.normals.clear();
                    scratch.normals.resize(count, [0., 1., 0.]);
                }
            }
            match source.attribute(Mesh::ATTRIBUTE_UV_0) {
                Some(VertexAttributeValues::Float32x2(values)) => scratch.uv.clone_from(values),
                _ => {
                    scratch.uv.clear();
                    scratch.uv.resize(count, [0.; 2]);
                }
            }
            for value in &mut scratch.uv {
                *value = ray_uv.transform_point2(Vec2::from_array(*value)).to_array();
            }
            match source.attribute(Mesh::ATTRIBUTE_TANGENT) {
                Some(VertexAttributeValues::Float32x4(values)) => {
                    scratch.tangents.clone_from(values)
                }
                _ => {
                    scratch.tangents.clear();
                    scratch.tangents.resize(count, [1., 0., 0., 1.]);
                }
            }
            if skin.is_some() {
                let RayMeshScratch {
                    pose, normal_pose, ..
                } = scratch;
                normal_pose.clear();
                normal_pose.extend(
                    pose.iter()
                        .map(|matrix| Mat3::from_mat4(*matrix).inverse().transpose()),
                );
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
                    let mut normal_matrix = Mat3::ZERO;
                    for influence in 0..4 {
                        let weight = weights[i][influence];
                        if weight != 0. {
                            let Some(joint) = scratch.pose.get(joints[i][influence] as usize)
                            else {
                                continue;
                            };
                            matrix += *joint * weight;
                            normal_matrix +=
                                scratch.normal_pose[joints[i][influence] as usize] * weight;
                        }
                    }
                    scratch.positions[i] = matrix
                        .transform_point3(Vec3::from_array(scratch.positions[i]))
                        .to_array();
                    scratch.normals[i] = (normal_matrix * Vec3::from_array(scratch.normals[i]))
                        .normalize_or(Vec3::Y)
                        .to_array();
                    let tangent = matrix
                        .transform_vector3(Vec3::new(
                            scratch.tangents[i][0],
                            scratch.tangents[i][1],
                            scratch.tangents[i][2],
                        ))
                        .normalize_or(Vec3::X);
                    scratch.tangents[i] = [tangent.x, tangent.y, tangent.z, scratch.tangents[i][3]];
                }
            }
            let mut proxy = Mesh::new(source.primitive_topology(), RenderAssetUsages::default());
            let indices: Vec<u32> = source
                .indices()
                .map(|indices| indices.iter().map(|i| i as u32).collect())
                .unwrap_or_else(|| (0..count as u32).collect());
            proxy.insert_attribute(
                Mesh::ATTRIBUTE_POSITION,
                std::mem::take(&mut scratch.positions),
            );
            proxy.insert_attribute(Mesh::ATTRIBUTE_NORMAL, std::mem::take(&mut scratch.normals));
            proxy.insert_attribute(Mesh::ATTRIBUTE_UV_0, std::mem::take(&mut scratch.uv));
            proxy.insert_attribute(
                Mesh::ATTRIBUTE_TANGENT,
                std::mem::take(&mut scratch.tangents),
            );
            proxy.insert_indices(Indices::U32(indices));
            proxy.enable_raytracing = true;
            if let Some(handle) = prepared
                .as_deref()
                .filter(|prepared| !shared && prepared.ownership == ProxyOwnership::Private)
                .map(|prepared| prepared.proxy.clone())
                && let Some(mut existing) = meshes.get_mut(&handle)
            {
                let mut previous = std::mem::replace(&mut *existing, proxy);
                if let Some(VertexAttributeValues::Float32x3(values)) =
                    previous.remove_attribute(Mesh::ATTRIBUTE_POSITION)
                {
                    scratch.positions = values;
                }
                if let Some(VertexAttributeValues::Float32x3(values)) =
                    previous.remove_attribute(Mesh::ATTRIBUTE_NORMAL)
                {
                    scratch.normals = values;
                }
                if let Some(VertexAttributeValues::Float32x2(values)) =
                    previous.remove_attribute(Mesh::ATTRIBUTE_UV_0)
                {
                    scratch.uv = values;
                }
                if let Some(VertexAttributeValues::Float32x4(values)) =
                    previous.remove_attribute(Mesh::ATTRIBUTE_TANGENT)
                {
                    scratch.tangents = values;
                }
                handle
            } else {
                meshes.add(proxy)
            }
        };
        let ownership = if shared {
            ProxyOwnership::Shared
        } else {
            ProxyOwnership::Private
        };
        if shared {
            cache.insert(key, handle.clone());
            scratch.live.insert(key);
        }
        if current.is_none_or(|current| current.0 != handle)
            || prepared.as_deref().is_some_and(|p| p.invalidated)
        {
            commands
                .entity(entity)
                .insert(RaytracingMesh3d(handle.clone()));
        }
        if let Some(prepared) = prepared.as_deref_mut() {
            prepared.source = source_id;
            prepared.proxy = handle;
            prepared.ownership = ownership;
            prepared.skinned = skin.is_some();
            prepared.deformed = flag.is_some();
            prepared.flag_time = flag_time;
            prepared.pose.clone_from(&scratch.pose);
            prepared.uv_transform = ray_uv;
            prepared.invalidated = false;
        } else {
            commands.entity(entity).insert(RayMeshSource {
                source: source_id,
                proxy: handle,
                ownership,
                skinned: skin.is_some(),
                deformed: flag.is_some(),
                pose: scratch.pose.clone(),
                flag_time,
                uv_transform: ray_uv,
                invalidated: false,
            });
        }
    }
    cache.retain(|key, _| scratch.live.contains(key));
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::{asset::AssetApp, ecs::message::MessageCursor, mesh::PrimitiveTopology};

    #[derive(Resource, Default)]
    struct ChangedProxies(Vec<Entity>);

    #[derive(Resource, Default)]
    struct ChangedRaySources(Vec<Entity>);

    fn record_changed(
        query: Query<Entity, Changed<RaytracingMesh3d>>,
        sources: Query<Entity, Changed<RayMeshSource>>,
        mut changed: ResMut<ChangedProxies>,
        mut changed_sources: ResMut<ChangedRaySources>,
    ) {
        changed.0.clear();
        changed.0.extend(&query);
        changed_sources.0.clear();
        changed_sources.0.extend(&sources);
    }

    fn app() -> App {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, bevy::asset::AssetPlugin::default()))
            .init_asset::<Mesh>()
            .init_asset::<StandardMaterial>()
            .init_asset::<SkinnedMeshInverseBindposes>()
            .init_resource::<ChangedProxies>()
            .init_resource::<ChangedRaySources>()
            .add_systems(
                PostUpdate,
                (prepare_meshes, record_changed)
                    .chain()
                    .after(super::super::lod::select)
                    .before(bevy::asset::AssetEventSystems),
            );
        app
    }

    fn triangle() -> Mesh {
        let mut mesh = Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        );
        mesh.insert_attribute(
            Mesh::ATTRIBUTE_POSITION,
            vec![[0., 0., 0.], [1., 0., 0.], [0., 1., 0.]],
        );
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, vec![[0., 0.], [1., 0.], [0., 1.]]);
        mesh.insert_attribute(
            Mesh::ATTRIBUTE_JOINT_INDEX,
            VertexAttributeValues::Uint16x4(vec![[0u16; 4]; 3]),
        );
        mesh.insert_attribute(Mesh::ATTRIBUTE_JOINT_WEIGHT, vec![[1., 0., 0., 0.]; 3]);
        mesh.insert_indices(Indices::U16(vec![0, 1, 2]));
        mesh
    }

    fn surface(app: &mut App, mesh: &Handle<Mesh>, material: &Handle<StandardMaterial>) -> Entity {
        app.world_mut()
            .spawn((
                Mesh3d(mesh.clone()),
                MeshMaterial3d(material.clone()),
                GlobalTransform::IDENTITY,
                InheritedVisibility::VISIBLE,
            ))
            .id()
    }

    fn material(app: &mut App, uv_transform: bevy::math::Affine2) -> Handle<StandardMaterial> {
        app.world_mut()
            .resource_mut::<Assets<StandardMaterial>>()
            .add(StandardMaterial {
                uv_transform,
                ..default()
            })
    }

    fn proxy(app: &App, entity: Entity) -> Handle<Mesh> {
        app.world()
            .get::<RaytracingMesh3d>(entity)
            .unwrap()
            .0
            .clone()
    }

    fn modifications(
        app: &App,
        cursor: &mut MessageCursor<AssetEvent<Mesh>>,
        id: AssetId<Mesh>,
    ) -> usize {
        cursor
            .read(app.world().resource::<Messages<AssetEvent<Mesh>>>())
            .filter(|event| matches!(event, AssetEvent::Modified { id: changed } if *changed == id))
            .count()
    }

    #[test]
    fn native_lods_control_first_frame_ray_admission_without_frustum_culling() {
        let mut app = app();
        app.init_resource::<crate::assets::LoadFailure>()
            .insert_resource(NextState::<crate::GameState>::Unchanged)
            .add_plugins(super::super::lod::PropLodPlugin);
        let camera = app
            .world_mut()
            .spawn((
                super::super::GameCamera,
                Projection::Perspective(PerspectiveProjection {
                    fov: std::f32::consts::FRAC_PI_2,
                    ..default()
                }),
                GlobalTransform::IDENTITY,
            ))
            .id();
        let source = app
            .world_mut()
            .resource_mut::<Assets<Mesh>>()
            .add(triangle());
        let second_source = app
            .world_mut()
            .resource_mut::<Assets<Mesh>>()
            .add(triangle());
        let material = material(&mut app, bevy::math::Affine2::IDENTITY);
        let ordinary = surface(&mut app, &source, &material);
        app.add_systems(
            SpawnScene,
            move |mut commands: Commands, mut spawned: Local<bool>| {
                if *spawned {
                    return;
                }
                *spawned = true;
                let parent = commands
                    .spawn(bevy::gltf::GltfExtras {
                        value: serde_json::json!({"native_lod": {
                            "distances": [20., 40.], "scale": 1., "cull_distance": 100.
                        }})
                        .to_string(),
                    })
                    .id();
                for (level, source) in [&source, &second_source].into_iter().enumerate() {
                    commands.spawn((
                        Mesh3d(source.clone()),
                        MeshMaterial3d(material.clone()),
                        GlobalTransform::from_translation(Vec3::X * 10.),
                        InheritedVisibility::VISIBLE,
                        ViewVisibility::HIDDEN,
                        ChildOf(parent),
                        bevy::gltf::GltfExtras {
                            value: serde_json::json!({"native_lod_level": level}).to_string(),
                        },
                    ));
                }
            },
        );
        app.update();
        let mut query = app
            .world_mut()
            .query_filtered::<Entity, With<super::super::lod::NativeLod>>();
        let mut levels: Vec<_> = query.iter(app.world()).collect();
        levels.sort_by_key(|entity| {
            let extras = app.world().get::<bevy::gltf::GltfExtras>(*entity).unwrap();
            serde_json::from_str::<serde_json::Value>(&extras.value).unwrap()["native_lod_level"]
                .as_u64()
                .unwrap()
        });
        assert_eq!(levels.len(), 2);
        assert!(proxy_or_none(&app, levels[0]).is_none());
        let selected_proxy = proxy(&app, levels[1]);
        assert!(proxy_or_none(&app, ordinary).is_some());
        assert!(!app.world().get::<ViewVisibility>(levels[1]).unwrap().get());
        app.update();
        assert!(app.world().resource::<ChangedProxies>().0.is_empty());

        for entity in &levels {
            *app.world_mut().get_mut::<GlobalTransform>(*entity).unwrap() =
                GlobalTransform::from_translation(Vec3::X);
        }
        app.update();
        assert!(proxy_or_none(&app, levels[0]).is_some());
        assert!(proxy_or_none(&app, levels[1]).is_none());
        app.world_mut()
            .entity_mut(levels[0])
            .insert(InheritedVisibility::HIDDEN);
        app.update();
        assert!(
            levels
                .iter()
                .all(|entity| proxy_or_none(&app, *entity).is_none())
        );
        app.world_mut()
            .entity_mut(levels[0])
            .insert(InheritedVisibility::VISIBLE);
        app.update();
        assert!(proxy_or_none(&app, levels[0]).is_some());

        for distance in [100., 10.] {
            for entity in &levels {
                *app.world_mut().get_mut::<GlobalTransform>(*entity).unwrap() =
                    GlobalTransform::from_translation(Vec3::X * distance);
            }
            app.update();
            assert!(proxy_or_none(&app, levels[0]).is_none());
            if distance == 100. {
                assert!(proxy_or_none(&app, levels[1]).is_none());
            } else {
                assert_eq!(proxy(&app, levels[1]), selected_proxy);
            }
            assert!(proxy_or_none(&app, ordinary).is_some());
        }
        app.world_mut()
            .entity_mut(camera)
            .remove::<super::super::GameCamera>();
        app.update();
        assert!(
            levels
                .iter()
                .all(|entity| proxy_or_none(&app, *entity).is_none())
        );
        assert!(proxy_or_none(&app, ordinary).is_some());
        app.world_mut()
            .entity_mut(camera)
            .insert(super::super::GameCamera);
        app.update();
        assert_eq!(proxy(&app, levels[1]), selected_proxy);
    }

    fn proxy_or_none(app: &App, entity: Entity) -> Option<Handle<Mesh>> {
        app.world()
            .get::<RaytracingMesh3d>(entity)
            .map(|proxy| proxy.0.clone())
    }

    #[test]
    fn immutable_uv_proxies_share_geometry_without_changing_material_ownership() {
        let mut app = app();
        let source = app
            .world_mut()
            .resource_mut::<Assets<Mesh>>()
            .add(triangle());
        let uv = bevy::math::Affine2::from_scale_angle_translation(
            Vec2::new(0.5, 0.25),
            0.,
            Vec2::new(1., -2.),
        );
        let first_material = material(&mut app, uv);
        let second_material = material(&mut app, uv);
        let different_material = material(&mut app, bevy::math::Affine2::IDENTITY);
        let first = surface(&mut app, &source, &first_material);
        let second = surface(&mut app, &source, &second_material);
        let different = surface(&mut app, &source, &different_material);
        app.update();
        let shared = proxy(&app, first);
        assert_eq!(shared, proxy(&app, second));
        assert_ne!(shared, proxy(&app, different));
        assert_eq!(
            app.world()
                .get::<MeshMaterial3d<StandardMaterial>>(first)
                .unwrap()
                .0,
            first_material
        );
        assert_eq!(
            app.world()
                .get::<MeshMaterial3d<StandardMaterial>>(second)
                .unwrap()
                .0,
            second_material
        );
        let meshes = app.world().resource::<Assets<Mesh>>();
        let mesh = meshes.get(&shared).unwrap();
        assert_eq!(
            mesh.attribute(Mesh::ATTRIBUTE_UV_0),
            Some(&VertexAttributeValues::Float32x2(vec![
                [1., -2.],
                [1.5, -2.],
                [1., -1.75]
            ]))
        );
        assert_eq!(
            mesh.attribute(Mesh::ATTRIBUTE_NORMAL),
            Some(&VertexAttributeValues::Float32x3(vec![[0., 1., 0.]; 3]))
        );
        assert_eq!(
            mesh.attribute(Mesh::ATTRIBUTE_TANGENT),
            Some(&VertexAttributeValues::Float32x4(vec![[1., 0., 0., 1.]; 3]))
        );
        let mut zero = bevy::math::Affine2::IDENTITY;
        zero.translation = Vec2::new(-0., 0.);
        assert!(
            RayMeshKey::new(source.id(), zero)
                == RayMeshKey::new(source.id(), bevy::math::Affine2::IDENTITY)
        );
        let mut cursor = app
            .world()
            .resource::<Messages<AssetEvent<Mesh>>>()
            .get_cursor_current();
        app.update();
        assert_eq!(modifications(&app, &mut cursor, shared.id()), 0);
        assert!(app.world().resource::<ChangedProxies>().0.is_empty());
        assert!(app.world().resource::<ChangedRaySources>().0.is_empty());
    }

    #[test]
    fn animated_uvs_do_not_rebuild_ray_geometry() {
        let mut app = app();
        let source = app
            .world_mut()
            .resource_mut::<Assets<Mesh>>()
            .add(triangle());
        let first_material = material(&mut app, bevy::math::Affine2::IDENTITY);
        let second_material = material(&mut app, bevy::math::Affine2::IDENTITY);
        let first = surface(&mut app, &source, &first_material);
        let second = surface(&mut app, &source, &second_material);
        let animation: super::super::opaque::UvAnimation = serde_json::from_value(
            serde_json::json!({"translation":[1.,0.],"time_mode":"elapsed"}),
        )
        .unwrap();
        app.world_mut()
            .entity_mut(first)
            .insert((super::super::DynamicSurface, animation));
        app.update();
        let shared = proxy(&app, first);
        assert_eq!(shared, proxy(&app, second));
        let mut cursor = app
            .world()
            .resource::<Messages<AssetEvent<Mesh>>>()
            .get_cursor_current();
        for offset in 1..12 {
            app.world_mut()
                .resource_mut::<Assets<StandardMaterial>>()
                .get_mut(&first_material)
                .unwrap()
                .uv_transform = bevy::math::Affine2::from_translation(Vec2::new(offset as f32, 0.));
            app.update();
            assert_eq!(proxy(&app, first), shared);
            assert_eq!(proxy(&app, second), shared);
            assert!(!cursor.read(app.world().resource::<Messages<AssetEvent<Mesh>>>())
                .any(|event| matches!(event, AssetEvent::Modified { id } | AssetEvent::Added { id } if *id == shared.id())));
            assert!(app.world().resource::<ChangedProxies>().0.is_empty());
            assert!(app.world().resource::<ChangedRaySources>().0.is_empty());
        }
    }

    #[test]
    fn skinning_detaches_shared_geometry_and_catches_up_after_hidden_frames() {
        let mut app = app();
        let source = app
            .world_mut()
            .resource_mut::<Assets<Mesh>>()
            .add(triangle());
        let material = material(&mut app, bevy::math::Affine2::IDENTITY);
        let first = surface(&mut app, &source, &material);
        let second = surface(&mut app, &source, &material);
        app.update();
        let shared = proxy(&app, second);
        let joint = app
            .world_mut()
            .spawn(GlobalTransform::from_translation(Vec3::new(2., 0., 0.)))
            .id();
        let inverse_bindposes = app
            .world_mut()
            .resource_mut::<Assets<SkinnedMeshInverseBindposes>>()
            .add(SkinnedMeshInverseBindposes::from(vec![Mat4::IDENTITY]));
        app.world_mut().entity_mut(first).insert(SkinnedMesh {
            inverse_bindposes,
            joints: vec![joint],
        });
        app.update();
        let private = proxy(&app, first);
        assert_ne!(private, shared);
        let meshes = app.world().resource::<Assets<Mesh>>();
        assert_eq!(
            meshes
                .get(&private)
                .unwrap()
                .attribute(Mesh::ATTRIBUTE_POSITION),
            Some(&VertexAttributeValues::Float32x3(vec![
                [2., 0., 0.],
                [3., 0., 0.],
                [2., 1., 0.]
            ]))
        );
        assert_eq!(
            meshes
                .get(&shared)
                .unwrap()
                .attribute(Mesh::ATTRIBUTE_POSITION),
            meshes
                .get(&source)
                .unwrap()
                .attribute(Mesh::ATTRIBUTE_POSITION)
        );
        let mut cursor = app
            .world()
            .resource::<Messages<AssetEvent<Mesh>>>()
            .get_cursor_current();
        app.update();
        assert_eq!(modifications(&app, &mut cursor, private.id()), 0);
        assert!(app.world().resource::<ChangedProxies>().0.is_empty());
        assert!(app.world().resource::<ChangedRaySources>().0.is_empty());
        app.world_mut()
            .entity_mut(first)
            .insert(InheritedVisibility::HIDDEN);
        app.update();
        assert!(app.world().get::<RaytracingMesh3d>(first).is_none());
        assert_eq!(
            app.world().get::<RayMeshSource>(first).unwrap().proxy,
            private
        );
        app.world_mut()
            .entity_mut(joint)
            .insert(GlobalTransform::from_translation(Vec3::new(4., 0., 0.)));
        app.update();
        assert_eq!(modifications(&app, &mut cursor, private.id()), 0);
        app.world_mut()
            .entity_mut(first)
            .insert(InheritedVisibility::VISIBLE);
        app.update();
        let resumed = proxy(&app, first);
        assert_eq!(resumed, private);
        let events: Vec<_> = cursor
            .read(app.world().resource::<Messages<AssetEvent<Mesh>>>())
            .collect();
        assert!(
            events
                .iter()
                .any(|event| matches!(event, AssetEvent::Modified { id } if *id == resumed.id()))
        );
        assert!(
            !events
                .iter()
                .any(|event| matches!(event, AssetEvent::Added { id } if *id == private.id()))
        );
        assert_eq!(proxy(&app, second), shared);
        let meshes = app.world().resource::<Assets<Mesh>>();
        assert_eq!(
            meshes
                .get(&resumed)
                .unwrap()
                .attribute(Mesh::ATTRIBUTE_POSITION),
            Some(&VertexAttributeValues::Float32x3(vec![
                [4., 0., 0.],
                [5., 0., 0.],
                [4., 1., 0.]
            ]))
        );
    }

    #[test]
    fn hidden_static_proxy_survives_source_cpu_release_and_final_owner_despawn() {
        let mut app = app();
        let source = app
            .world_mut()
            .resource_mut::<Assets<Mesh>>()
            .add(triangle());
        let material = material(&mut app, bevy::math::Affine2::from_translation(Vec2::X));
        let entity = surface(&mut app, &source, &material);
        app.update();
        let proxy_id = proxy(&app, entity).id();
        app.world_mut()
            .entity_mut(entity)
            .insert(InheritedVisibility::HIDDEN);
        app.update();
        app.world_mut()
            .resource_mut::<Assets<Mesh>>()
            .get_mut(&source)
            .unwrap()
            .take_gpu_data()
            .unwrap();
        app.update();
        app.world_mut()
            .entity_mut(entity)
            .insert(InheritedVisibility::VISIBLE);
        app.update();
        assert_eq!(proxy(&app, entity).id(), proxy_id);
        assert!(app.world().resource::<ChangedRaySources>().0.is_empty());
        assert_eq!(app.world().resource::<Assets<Mesh>>().len(), 2);
        app.world_mut().despawn(entity);
        for _ in 0..3 {
            app.update();
        }
        assert!(!app.world().resource::<Assets<Mesh>>().contains(proxy_id));
        assert_eq!(app.world().resource::<Assets<Mesh>>().len(), 1);
    }

    #[test]
    fn source_topology_changes_while_hidden_invalidate_retained_geometry() {
        let mut app = app();
        let source = app
            .world_mut()
            .resource_mut::<Assets<Mesh>>()
            .add(triangle());
        let material = material(&mut app, bevy::math::Affine2::IDENTITY);
        let entity = surface(&mut app, &source, &material);
        app.update();
        let original_proxy = proxy(&app, entity);
        app.world_mut()
            .entity_mut(entity)
            .insert(InheritedVisibility::HIDDEN);
        app.update();
        let mut replacement = Mesh::new(
            PrimitiveTopology::TriangleStrip,
            RenderAssetUsages::default(),
        );
        replacement.insert_attribute(
            Mesh::ATTRIBUTE_POSITION,
            vec![[0., 0., 0.], [1., 0., 0.], [0., 1., 0.], [1., 1., 0.]],
        );
        replacement.insert_indices(Indices::U16(vec![0, 2, 1, 3]));
        *app.world_mut()
            .resource_mut::<Assets<Mesh>>()
            .get_mut(&source)
            .unwrap() = replacement;
        app.update();
        app.update();
        assert!(
            app.world()
                .get::<RayMeshSource>(entity)
                .unwrap()
                .invalidated
        );
        app.world_mut()
            .entity_mut(entity)
            .insert(InheritedVisibility::VISIBLE);
        app.update();
        let replacement_proxy = proxy(&app, entity);
        assert_ne!(replacement_proxy, original_proxy);
        let meshes = app.world().resource::<Assets<Mesh>>();
        let mesh = meshes.get(&replacement_proxy).unwrap();
        assert_eq!(mesh.primitive_topology(), PrimitiveTopology::TriangleStrip);
        assert_eq!(
            mesh.indices().unwrap().iter().collect::<Vec<_>>(),
            vec![0, 2, 1, 3]
        );
        app.world_mut()
            .resource_mut::<Assets<Mesh>>()
            .remove(source.id());
        app.update();
        assert!(app.world().get::<RaytracingMesh3d>(entity).is_none());
        assert!(app.world().get::<RayMeshSource>(entity).is_none());
    }
}

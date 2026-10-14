use super::opaque::{OpaqueSurfaceStatus, SurfaceLibrary};
use super::solari::RayMeshSource;
use crate::{GameState, assets::WorldCollider};
use bevy::{asset::RenderAssetUsages, prelude::*};
use std::collections::HashSet;

pub(super) struct ResidencyPlugin;
impl Plugin for ResidencyPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            PostUpdate,
            release
                .after(super::solari::prepare_meshes)
                .before(bevy::asset::AssetEventSystems),
        );
    }
}

fn release(
    mut commands: Commands,
    state: Res<State<GameState>>,
    status: Res<OpaqueSurfaceStatus>,
    lights: Query<(&GlobalTransform, &PointLight, &ViewVisibility)>,
    cameras: Query<&GlobalTransform, With<super::GameCamera>>,
    colliders: Query<(Entity, &Mesh3d), With<WorldCollider>>,
    surfaces: Query<
        (
            &Mesh3d,
            &MeshMaterial3d<StandardMaterial>,
            Option<&RayMeshSource>,
        ),
        (
            Without<WorldCollider>,
            With<GlobalTransform>,
            With<InheritedVisibility>,
        ),
    >,
    changed: Query<&RayMeshSource, Changed<RayMeshSource>>,
    server: Res<AssetServer>,
    library: Option<Res<SurfaceLibrary>>,
    materials: Res<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut finished: Local<bool>,
    mut mesh_ids: Local<HashSet<AssetId<Mesh>>>,
) {
    if **state == GameState::Loading {
        *finished = false;
        mesh_ids.clear();
        return;
    }
    let Some(library) = library else {
        return;
    };
    mesh_ids.extend(changed.iter().map(RayMeshSource::source_id).filter(|id| {
        library.prepared_meshes.contains(id)
            && !library.live_meshes.contains(id)
            && server.get_path(*id).is_some()
    }));
    if !status.ready() || (*finished && mesh_ids.is_empty()) {
        return;
    }
    mesh_ids.retain(|id| {
        library.prepared_meshes.contains(id)
            && !library.live_meshes.contains(id)
            && server.get_path(*id).is_some()
    });
    if !*finished && let Ok(camera) = cameras.single() {
        info!(
            "Lantern allocation: {} total, {} view-visible, {} affect camera position; camera {:?}",
            lights.iter().count(),
            lights
                .iter()
                .filter(|(_, _, visible)| visible.get())
                .count(),
            lights
                .iter()
                .filter(
                    |(pose, light, _)| pose.translation().distance(camera.translation())
                        < light.range
                )
                .count(),
            camera.translation()
        );
    }
    let mut image_bytes = 0;
    let mut image_count = 0;
    if !*finished {
        let image_ids: Vec<_> = images
            .iter()
            .filter_map(|(id, image)| {
                (server.get_path(id).is_some()
                    && image.texture_descriptor.size.depth_or_array_layers == 1
                    && image.data.is_some()
                    && image.asset_usage.contains(RenderAssetUsages::MAIN_WORLD))
                .then_some(id)
            })
            .collect();
        for id in image_ids {
            if let Some(mut image) = images.get_mut(id) {
                image_bytes += image.data.as_ref().map_or(0, Vec::len);
                image_count += 1;
                image.asset_usage = RenderAssetUsages::RENDER_WORLD;
            }
        }
        mesh_ids.extend(
            library
                .prepared_meshes
                .iter()
                .copied()
                .filter(|id| !library.live_meshes.contains(id) && server.get_path(*id).is_some()),
        );
        for (entity, _) in &colliders {
            commands
                .entity(entity)
                .remove::<(Mesh3d, MeshMaterial3d<StandardMaterial>)>()
                .remove_parent_in_place();
        }
    }
    let (mesh_count, mesh_bytes) =
        release_meshes(&mut mesh_ids, surfaces.iter(), &materials, &mut meshes);
    info!(
        "Released preparation data: {image_count} images ({image_bytes} CPU bytes), {mesh_count} meshes ({mesh_bytes} CPU bytes)"
    );
    *finished = true;
}

fn release_meshes<'a>(
    mesh_ids: &mut HashSet<AssetId<Mesh>>,
    surfaces: impl Iterator<
        Item = (
            &'a Mesh3d,
            &'a MeshMaterial3d<StandardMaterial>,
            Option<&'a RayMeshSource>,
        ),
    >,
    materials: &Assets<StandardMaterial>,
    meshes: &mut Assets<Mesh>,
) -> (usize, usize) {
    for (raster, material, proxy) in surfaces {
        let id = raster.0.id();
        if !mesh_ids.contains(&id) {
            continue;
        }
        let ready = materials.get(&material.0).is_some_and(|material| {
            material.unlit
                || material.specular_transmission > 0.
                || !matches!(material.alpha_mode, AlphaMode::Opaque | AlphaMode::Mask(_))
                || proxy.is_some_and(|proxy| proxy.ready_for(id, meshes))
        });
        if !ready {
            mesh_ids.remove(&id);
        }
    }
    let mut mesh_bytes = 0;
    let mut mesh_count = 0;
    for id in mesh_ids.drain() {
        let Some(mesh) = meshes.get(id) else {
            continue;
        };
        if !mesh.asset_usage.contains(RenderAssetUsages::MAIN_WORLD)
            || mesh.try_attributes().is_err()
            || mesh.attribute(Mesh::ATTRIBUTE_JOINT_INDEX).is_some()
        {
            continue;
        }
        let Some(mut mesh) = meshes.get_mut(id) else {
            continue;
        };
        mesh_bytes += mesh.try_attributes().map_or(0, |attributes| {
            attributes
                .map(|(_, values)| values.get_bytes().len())
                .sum::<usize>()
        });
        mesh_bytes += mesh
            .try_indices_option()
            .ok()
            .flatten()
            .map_or(0, |indices| match indices {
                bevy::mesh::Indices::U16(values) => values.len() * 2,
                bevy::mesh::Indices::U32(values) => values.len() * 4,
            });
        mesh_count += 1;
        mesh.asset_usage = RenderAssetUsages::RENDER_WORLD;
    }
    (mesh_count, mesh_bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::{asset::AssetApp, gltf::GltfExtras, mesh::PrimitiveTopology};

    #[derive(Resource)]
    struct StaticSource(AssetId<Mesh>);

    #[derive(Resource, Default)]
    struct Released(usize);

    fn release_static(
        source: Res<StaticSource>,
        surfaces: Query<(
            &Mesh3d,
            &MeshMaterial3d<StandardMaterial>,
            Option<&RayMeshSource>,
        )>,
        changed: Query<&RayMeshSource, Changed<RayMeshSource>>,
        materials: Res<Assets<StandardMaterial>>,
        mut meshes: ResMut<Assets<Mesh>>,
        mut initial: Local<bool>,
        mut released: ResMut<Released>,
    ) {
        if *initial && !changed.iter().any(|proxy| proxy.source_id() == source.0) {
            return;
        }
        *initial = true;
        let mut ids = HashSet::from([source.0]);
        released.0 += release_meshes(&mut ids, surfaces.iter(), &materials, &mut meshes).0;
    }

    #[test]
    fn initially_inactive_lod_keeps_cpu_data_until_all_shared_references_have_proxies() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, bevy::asset::AssetPlugin::default()))
            .init_asset::<Mesh>()
            .init_asset::<StandardMaterial>()
            .init_asset::<bevy::mesh::skinning::SkinnedMeshInverseBindposes>()
            .init_resource::<crate::assets::LoadFailure>()
            .init_resource::<Released>()
            .insert_resource(NextState::<GameState>::Unchanged)
            .add_plugins(super::super::lod::PropLodPlugin)
            .add_systems(
                PostUpdate,
                (super::super::solari::prepare_meshes, release_static)
                    .chain()
                    .after(super::super::lod::select)
                    .before(bevy::asset::AssetEventSystems),
            );
        let mut mesh = Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        );
        mesh.insert_attribute(
            Mesh::ATTRIBUTE_POSITION,
            vec![[0., 0., 0.], [1., 0., 0.], [0., 1., 0.]],
        );
        let source = app.world_mut().resource_mut::<Assets<Mesh>>().add(mesh);
        app.insert_resource(StaticSource(source.id()));
        let material = app
            .world_mut()
            .resource_mut::<Assets<StandardMaterial>>()
            .add(StandardMaterial::default());
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
        let mut levels = Vec::new();
        for level in 0..2 {
            let value = serde_json::json!({
                "native_lod_level": level,
                "native_lod": {"distances": [20., 40.], "scale": 1., "cull_distance": 100.}
            })
            .to_string();
            levels.push(
                app.world_mut()
                    .spawn((
                        Mesh3d(source.clone()),
                        MeshMaterial3d(material.clone()),
                        GlobalTransform::from_translation(Vec3::X),
                        InheritedVisibility::VISIBLE,
                        GltfExtras { value },
                    ))
                    .id(),
            );
        }
        for _ in 0..2 {
            app.update();
            assert!(app.world().get::<RayMeshSource>(levels[0]).is_some());
            assert!(app.world().get::<RayMeshSource>(levels[1]).is_none());
            let meshes = app.world().resource::<Assets<Mesh>>();
            let mesh = meshes.get(&source).unwrap();
            assert!(mesh.asset_usage.contains(RenderAssetUsages::MAIN_WORLD));
            assert!(mesh.try_attributes().is_ok());
            assert_eq!(app.world().resource::<Released>().0, 0);
        }
        app.world_mut()
            .entity_mut(camera)
            .insert(GlobalTransform::from_translation(Vec3::X * 11.));
        app.update();
        assert!(app.world().get::<RayMeshSource>(levels[1]).is_some());
        assert!(
            app.world()
                .get::<bevy::solari::prelude::RaytracingMesh3d>(levels[1])
                .is_some()
        );
        assert_eq!(app.world().resource::<Released>().0, 1);
        let mut meshes = app.world_mut().resource_mut::<Assets<Mesh>>();
        let mut mesh = meshes.get_mut(&source).unwrap();
        assert_eq!(mesh.asset_usage, RenderAssetUsages::RENDER_WORLD);
        mesh.take_gpu_data().unwrap();
        assert!(mesh.try_attributes().is_err());
        drop(mesh);
        drop(meshes);
        app.world_mut()
            .entity_mut(camera)
            .insert(GlobalTransform::IDENTITY);
        app.update();
        assert!(
            app.world()
                .get::<bevy::solari::prelude::RaytracingMesh3d>(levels[0])
                .is_some()
        );
    }
}

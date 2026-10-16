//! On-demand source geometry shared by animated model instances.
use super::material::MaterialDefinition;
use bevy::{
    asset::{AssetLoader, LoadContext, io::Reader},
    camera::{
        primitives::Aabb,
        visibility::{NoFrustumCulling, RenderLayers},
    },
    mesh::skinning::{SkinnedMesh, SkinnedMeshInverseBindposes},
    prelude::*,
    render::{RenderApp, extract_component::ExtractComponent, sync_world::RenderEntity},
    world_serialization::WorldInstanceReady,
};
use serde::Deserialize;
use std::{
    collections::BTreeMap,
    io::{Error, ErrorKind},
    path::Path,
    sync::Arc,
};

#[derive(Clone, Asset, TypePath)]
pub(crate) struct AnimatedModel {
    pub definition: ModelDefinition,
    pub vertices: Vec<u8>,
    pub indices: Vec<u8>,
    pub blends: Vec<u8>,
    pub bind: Vec<u8>,
    pub materials: Arc<BTreeMap<String, MaterialDefinition>>,
}

#[derive(Clone, Deserialize, TypePath)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ModelDefinition {
    pub bounds: [[f32; 3]; 2],
    pub bone_names: Vec<String>,
    pub root_bone_count: usize,
    pub lod_ramp_type: usize,
    pub lods: Vec<ModelLod>,
    vertices_file: String,
    indices_file: String,
    blends_file: String,
    bind_file: String,
}

#[derive(Clone, Deserialize, TypePath)]
pub(crate) struct ModelLod {
    pub dist: f32,
    pub surfaces: Vec<ModelSurface>,
}

#[derive(Clone, Deserialize, TypePath)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ModelSurface {
    pub vertex_byte_offset: u64,
    pub first_index: u32,
    pub vertex_count: u32,
    pub tri_count: u32,
    pub material: String,
    pub material_file: String,
    pub material_draw_surf: u64,
    pub rigid_lists: Vec<RigidList>,
    pub blend_vertex_counts: [u32; 4],
    pub blend_byte_offset: u64,
    pub blend_byte_length: u64,
    pub tension_byte_length: u64,
}

#[derive(Clone, Deserialize, TypePath)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RigidList {
    pub bone_offset: u32,
    pub vert_count: u32,
}

#[derive(Default, TypePath)]
struct AnimatedModelLoader;
impl AssetLoader for AnimatedModelLoader {
    type Asset = AnimatedModel;
    type Settings = ();
    type Error = Error;

    async fn load(
        &self,
        reader: &mut dyn Reader,
        _: &(),
        context: &mut LoadContext<'_>,
    ) -> Result<AnimatedModel, Error> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        let definition: ModelDefinition =
            serde_json::from_slice(&bytes).map_err(|e| Error::new(ErrorKind::InvalidData, e))?;
        let mut read = async |path: &str| {
            context
                .read_asset_bytes(path.to_owned())
                .await
                .map_err(Error::other)
        };
        let vertices = read(&definition.vertices_file).await?;
        let indices = read(&definition.indices_file).await?;
        let blends = read(&definition.blends_file).await?;
        let bind = read(&definition.bind_file).await?;
        let mut materials = BTreeMap::new();
        for surface in definition.lods.iter().flat_map(|lod| &lod.surfaces) {
            if !materials.contains_key(&surface.material_file) {
                let bytes = read(&surface.material_file).await?;
                let material =
                    MaterialDefinition::from_bytes(Path::new(&surface.material_file), &bytes)
                        .map_err(Error::other)?;
                materials.insert(surface.material_file.clone(), material);
            }
        }
        let model = AnimatedModel {
            definition,
            vertices,
            indices,
            blends,
            bind,
            materials: Arc::new(materials),
        };
        model.validate()?;
        Ok(model)
    }

    fn extensions(&self) -> &[&str] {
        &["native_model"]
    }
}

impl AnimatedModel {
    fn validate(&self) -> Result<(), Error> {
        let invalid = |message| Error::new(ErrorKind::InvalidData, message);
        let bones = self.definition.bone_names.len();
        if self.definition.lod_ramp_type > 1
            || self.definition.lods.is_empty()
            || self
                .definition
                .lods
                .iter()
                .any(|lod| !lod.dist.is_finite() || lod.dist <= 0.)
        {
            return Err(invalid("invalid native model LOD ramp or distances"));
        }
        if bones == 0 || self.bind.len() != bones * 32 || self.definition.root_bone_count > bones {
            return Err(invalid("incomplete native animated model skeleton"));
        }
        if self.vertices.is_empty()
            || !self.vertices.len().is_multiple_of(32)
            || !self.indices.len().is_multiple_of(2)
        {
            return Err(invalid("incomplete native PACKED32 geometry"));
        }
        for lod in &self.definition.lods {
            for surface in &lod.surfaces {
                if !surface.vertex_byte_offset.is_multiple_of(32)
                    || surface.vertex_byte_offset + u64::from(surface.vertex_count) * 32
                        > self.vertices.len() as u64
                    || u64::from(surface.first_index) * 2 + u64::from(surface.tri_count) * 6
                        > self.indices.len() as u64
                    || surface.blend_byte_offset + surface.blend_byte_length
                        > self.blends.len() as u64
                {
                    return Err(invalid("native animated surface exceeds its source buffer"));
                }
                if surface.tension_byte_length != 0 {
                    return Err(invalid(
                        "native tension deformation must be implemented for this model",
                    ));
                }
                let rigid: u64 = surface
                    .rigid_lists
                    .iter()
                    .map(|r| u64::from(r.vert_count))
                    .sum();
                let blended: u64 = surface
                    .blend_vertex_counts
                    .iter()
                    .map(|&n| u64::from(n))
                    .sum();
                if rigid + blended != u64::from(surface.vertex_count) {
                    return Err(invalid("native skin ranges do not cover the surface"));
                }
                if surface
                    .rigid_lists
                    .iter()
                    .any(|r| r.bone_offset % 64 != 0 || r.bone_offset as usize / 64 >= bones)
                {
                    return Err(invalid("native rigid range references a missing bone"));
                }
                let expected: u64 = surface
                    .blend_vertex_counts
                    .iter()
                    .enumerate()
                    .map(|(i, &n)| u64::from(n) * (2 * i as u64 + 1) * 2)
                    .sum();
                if expected != surface.blend_byte_length
                    || !surface.blend_byte_offset.is_multiple_of(2)
                {
                    return Err(invalid(
                        "native blend buckets do not match the blend stream",
                    ));
                }
                let mut cursor = surface.blend_byte_offset as usize;
                for (i, &count) in surface.blend_vertex_counts.iter().enumerate() {
                    for _ in 0..count {
                        for word in std::iter::once(0).chain((1..=i).map(|j| j * 2 - 1)) {
                            let at = cursor + word * 2;
                            let bone =
                                u16::from_le_bytes(self.blends[at..at + 2].try_into().unwrap());
                            if bone % 64 != 0 || usize::from(bone) / 64 >= bones {
                                return Err(invalid("native blend references a missing bone"));
                            }
                        }
                        cursor += (2 * i + 1) * 2;
                    }
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scene_lod_preserves_source_units_scale_and_viewmodel_override() {
        let mut app = App::new();
        app.init_resource::<Assets<AnimatedModel>>()
            .init_resource::<super::super::culling::SourceVisibility>()
            .add_systems(Update, select_lods);
        let model = app
            .world_mut()
            .resource_mut::<Assets<AnimatedModel>>()
            .add(AnimatedModel {
                definition: ModelDefinition {
                    bounds: [[-1.; 3], [1.; 3]],
                    bone_names: vec!["tag_origin".into()],
                    root_bone_count: 1,
                    lod_ramp_type: 1,
                    lods: vec![
                        ModelLod {
                            dist: 100.,
                            surfaces: vec![],
                        },
                        ModelLod {
                            dist: 200.,
                            surfaces: vec![],
                        },
                    ],
                    vertices_file: String::new(),
                    indices_file: String::new(),
                    blends_file: String::new(),
                    bind_file: String::new(),
                },
                vertices: vec![],
                indices: vec![],
                blends: vec![],
                bind: vec![],
                materials: Arc::default(),
            });
        app.world_mut().spawn((
            super::super::GameCamera,
            GlobalTransform::IDENTITY,
            Camera {
                computed: bevy::camera::ComputedCameraValues {
                    clip_from_view: Mat4::from_diagonal(Vec4::new(2.118673, 2.118673, 1., 1.)),
                    ..default()
                },
                ..default()
            },
        ));
        let pose = |distance, scale| {
            GlobalTransform::from(
                Transform::from_xyz(distance * crate::assets::SOURCE_UNIT_METERS, 0., 0.)
                    .with_scale(Vec3::splat(scale * crate::assets::SOURCE_UNIT_METERS)),
            )
        };
        let mesh = app.world_mut().spawn(pose(150., 1.)).id();
        let instance = app
            .world_mut()
            .spawn((
                AnimatedModelInstance {
                    model,
                    lod: Some(0),
                },
                NativeBoneBinding {
                    mesh,
                    joints: vec![],
                    inverse_bindposes: Handle::default(),
                    bounds_meshes: vec![],
                    bounds: Aabb::from_min_max(Vec3::splat(-1.), Vec3::ONE),
                },
            ))
            .id();
        app.update();
        assert_eq!(
            app.world()
                .get::<AnimatedModelInstance>(instance)
                .unwrap()
                .lod,
            Some(1)
        );
        app.world_mut().entity_mut(mesh).insert(pose(150., 2.));
        app.update();
        assert_eq!(
            app.world()
                .get::<AnimatedModelInstance>(instance)
                .unwrap()
                .lod,
            Some(0)
        );
        app.world_mut().entity_mut(mesh).insert(pose(300., 1.));
        app.update();
        assert_eq!(
            app.world()
                .get::<AnimatedModelInstance>(instance)
                .unwrap()
                .lod,
            None
        );
        app.world_mut().entity_mut(mesh).insert(pose(50., 1.));
        app.update();
        assert_eq!(
            app.world()
                .get::<AnimatedModelInstance>(instance)
                .unwrap()
                .lod,
            Some(0)
        );
        app.world_mut()
            .entity_mut(mesh)
            .insert((pose(300., 1.), NoFrustumCulling));
        app.update();
        assert_eq!(
            app.world()
                .get::<AnimatedModelInstance>(instance)
                .unwrap()
                .lod,
            Some(0)
        );
    }

    #[test]
    fn native_palette_cancels_bind_pose_and_instance_scale() {
        let axes = Mat4::from_cols(Vec4::X, -Vec4::Z, Vec4::Y, Vec4::W);
        let owner = Mat4::from_scale_rotation_translation(
            Vec3::splat(0.0254),
            Quat::from_rotation_y(0.7),
            Vec3::new(13., 2., -8.),
        );
        let bind =
            Mat4::from_rotation_translation(Quat::from_rotation_z(0.3), Vec3::new(4., 7., 11.));
        let palette = source_skin_matrix(
            owner.inverse(),
            owner * axes * bind,
            (axes * bind).inverse(),
        );
        assert!(palette.abs_diff_eq(Mat4::IDENTITY, 0.0001));
    }

    #[test]
    fn native_bounds_follow_animation_and_include_every_primitive() {
        let mut app = App::new();
        app.add_systems(Update, update_native_bounds);
        let pose = GlobalTransform::from(
            Transform::from_xyz(90., 30., -40.)
                .with_scale(Vec3::splat(crate::assets::SOURCE_UNIT_METERS)),
        );
        let torso = app
            .world_mut()
            .spawn((
                Mesh3d::default(),
                pose,
                Aabb::from_min_max(Vec3::splat(-1.), Vec3::ONE),
            ))
            .id();
        let hand = app
            .world_mut()
            .spawn((
                Mesh3d::default(),
                pose,
                Aabb::from_min_max(Vec3::new(9., 19., -31.), Vec3::new(11., 21., -29.)),
            ))
            .id();
        let instance = app
            .world_mut()
            .spawn(NativeBoneBinding {
                mesh: torso,
                joints: vec![],
                inverse_bindposes: Handle::default(),
                bounds_meshes: vec![torso, hand],
                bounds: Aabb::default(),
            })
            .id();
        app.update();
        let bounds = app
            .world()
            .get::<NativeBoneBinding>(instance)
            .unwrap()
            .bounds;
        assert!(Vec3::from(bounds.min()).abs_diff_eq(Vec3::splat(-1.), 0.001));
        assert!(Vec3::from(bounds.max()).abs_diff_eq(Vec3::new(11., 31., 21.), 0.001));

        // An animated limb moves outside the bind pose while the mesh stays still.
        app.world_mut().entity_mut(hand).insert(Aabb::from_min_max(
            Vec3::new(-51., 19., -31.),
            Vec3::new(-49., 21., -29.),
        ));
        app.update();
        let bounds = app
            .world()
            .get::<NativeBoneBinding>(instance)
            .unwrap()
            .bounds;
        assert!(Vec3::from(bounds.min()).abs_diff_eq(Vec3::new(-51., -1., -1.), 0.001));
        assert!(Vec3::from(bounds.max()).abs_diff_eq(Vec3::new(1., 31., 21.), 0.001));
    }

    #[test]
    fn native_palette_retains_source_inches_and_bone_motion() {
        let axes = Mat4::from_cols(Vec4::X, -Vec4::Z, Vec4::Y, Vec4::W);
        let owner = Mat4::from_scale_rotation_translation(
            Vec3::new(0.0254, 0.0508, 0.0254),
            Quat::from_rotation_x(-0.4),
            Vec3::new(-3., 1., 6.),
        );
        let bind = Mat4::from_translation(Vec3::new(5., 0., 0.));
        let pose = Mat4::from_rotation_translation(
            Quat::from_rotation_z(std::f32::consts::FRAC_PI_2),
            Vec3::new(5., 0., 8.),
        );
        let palette = source_skin_matrix(
            owner.inverse(),
            owner * axes * pose,
            (axes * bind).inverse(),
        );
        let point = palette.transform_point3(Vec3::new(6., 0., 0.));
        assert!(point.abs_diff_eq(Vec3::new(5., 1., 8.), 0.0001));
    }

    #[test]
    fn hidden_native_palette_does_not_invert_zero_scale_mesh() {
        let mut app = App::new();
        app.insert_resource(Assets::<SkinnedMeshInverseBindposes>::default())
            .add_systems(Update, update_native_palettes);
        let mesh = app
            .world_mut()
            .spawn(GlobalTransform::from(Transform::from_scale(Vec3::ZERO)))
            .id();
        let instance = app
            .world_mut()
            .spawn((
                NativeBoneBinding {
                    mesh,
                    joints: vec![],
                    inverse_bindposes: Handle::default(),
                    bounds_meshes: vec![],
                    bounds: Aabb::default(),
                },
                NativeSkinPalette(vec![Mat4::IDENTITY]),
                InheritedVisibility::HIDDEN,
                AnimatedModelInstance {
                    model: Handle::default(),
                    lod: Some(0),
                },
            ))
            .id();
        app.update();
        assert_eq!(
            app.world().get::<NativeSkinPalette>(instance).unwrap().0,
            vec![Mat4::IDENTITY]
        );
    }

    #[test]
    fn returning_from_culling_refreshes_a_stationary_skin_palette() {
        let mut app = App::new();
        app.init_resource::<Assets<SkinnedMeshInverseBindposes>>()
            .add_systems(Update, update_native_palettes);
        let bind = app
            .world_mut()
            .resource_mut::<Assets<SkinnedMeshInverseBindposes>>()
            .add(SkinnedMeshInverseBindposes::from(vec![Mat4::IDENTITY]));
        let mesh = app.world_mut().spawn(GlobalTransform::IDENTITY).id();
        let joint = app.world_mut().spawn(GlobalTransform::IDENTITY).id();
        let instance = app
            .world_mut()
            .spawn((
                NativeBoneBinding {
                    mesh,
                    joints: vec![joint],
                    inverse_bindposes: bind,
                    bounds_meshes: vec![],
                    bounds: Aabb::default(),
                },
                NativeSkinPalette(vec![Mat4::IDENTITY]),
                InheritedVisibility::VISIBLE,
                AnimatedModelInstance {
                    model: Handle::default(),
                    lod: Some(0),
                },
            ))
            .id();
        app.update();
        app.world_mut()
            .get_mut::<AnimatedModelInstance>(instance)
            .unwrap()
            .lod = None;
        let moved = Mat4::from_translation(Vec3::new(2., 3., 4.));
        app.world_mut()
            .entity_mut(joint)
            .insert(GlobalTransform::from(Transform::from_matrix(moved)));
        app.update();
        assert_eq!(
            app.world().get::<NativeSkinPalette>(instance).unwrap().0,
            [Mat4::IDENTITY]
        );
        app.update();
        app.world_mut()
            .get_mut::<AnimatedModelInstance>(instance)
            .unwrap()
            .lod = Some(0);
        app.update();
        assert_eq!(
            app.world().get::<NativeSkinPalette>(instance).unwrap().0,
            [source_skin_matrix(Mat4::IDENTITY, moved, Mat4::IDENTITY)]
        );
    }

    #[test]
    fn rigid_native_model_binds_without_a_scene_skin() {
        let mut app = App::new();
        app.init_resource::<Assets<AnimatedModel>>()
            .init_resource::<Assets<SkinnedMeshInverseBindposes>>()
            .add_systems(Update, (bind_native_bones, update_native_palettes).chain());
        let model = app
            .world_mut()
            .resource_mut::<Assets<AnimatedModel>>()
            .add(AnimatedModel {
                definition: ModelDefinition {
                    bounds: [[-1.; 3], [1.; 3]],
                    bone_names: vec!["tag_weapon".into()],
                    root_bone_count: 1,
                    lod_ramp_type: 0,
                    lods: vec![ModelLod {
                        dist: 100.,
                        surfaces: vec![],
                    }],
                    vertices_file: String::new(),
                    indices_file: String::new(),
                    blends_file: String::new(),
                    bind_file: String::new(),
                },
                vertices: vec![],
                indices: vec![],
                blends: vec![],
                bind: vec![],
                materials: Arc::default(),
            });
        let instance = app
            .world_mut()
            .spawn((
                AnimatedModelInstance {
                    model,
                    lod: Some(0),
                },
                Transform::from_xyz(10., 2., -3.),
                InheritedVisibility::VISIBLE,
            ))
            .id();
        let mesh = app
            .world_mut()
            .spawn((ChildOf(instance), Mesh3d::default(), Transform::IDENTITY))
            .id();
        app.update();
        assert_eq!(
            app.world().get::<NativeBoneBinding>(instance).unwrap().mesh,
            mesh
        );
        assert_eq!(
            app.world().get::<NativeSkinPalette>(instance).unwrap().0,
            vec![Mat4::IDENTITY]
        );
        app.world_mut()
            .get_mut::<Transform>(instance)
            .unwrap()
            .translation += Vec3::X;
        app.update();
        assert_eq!(
            app.world().get::<NativeSkinPalette>(instance).unwrap().0,
            vec![Mat4::IDENTITY]
        );
    }

    #[test]
    #[ignore = "requires the authorized local native model export"]
    fn exported_native_model_materials_bind() {
        use std::collections::BTreeSet;
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("target/assets");
        let mut pending = vec![root.join("model_export")];
        let mut material_paths = BTreeSet::new();
        let mut models = 0;
        while let Some(path) = pending.pop() {
            if path.is_dir() {
                pending.extend(
                    std::fs::read_dir(path)
                        .unwrap()
                        .map(|file| file.unwrap().path()),
                );
            } else if path.extension().and_then(|e| e.to_str()) == Some("native_model") {
                let definition: ModelDefinition =
                    serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
                models += 1;
                material_paths.extend(definition.lods.into_iter().flat_map(|lod| {
                    lod.surfaces
                        .into_iter()
                        .map(|surface| surface.material_file)
                }));
            }
        }
        assert!(
            models > 0 && !material_paths.is_empty(),
            "native model export is required"
        );
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            bevy::asset::AssetPlugin {
                file_path: root.to_string_lossy().into_owned(),
                ..default()
            },
        ))
        .init_asset::<Image>();
        let server = app.world().resource::<AssetServer>();
        let mut failures = Vec::new();
        let mut sky_materials = 0;
        let mut invisible_materials = 0;
        for path in &material_paths {
            match MaterialDefinition::read(&root.join(path)) {
                Ok(material) => {
                    if !material.has_camera_pass() {
                        invisible_materials += 1;
                        continue;
                    }
                    if material.technique_set() == "mc_skycubemaphdr_1f117w7u" {
                        // NativeSky validates and draws the GfxWorld sky model.
                        sky_materials += 1;
                        continue;
                    }
                    match material.model(server, Handle::default()) {
                        Ok((bound, _)) => {
                            for image in bound.images() {
                                if let Some(image_path) = image.path()
                                    && !root.join(image_path.path()).is_file()
                                {
                                    failures.push(format!(
                                        "Material {path}: missing image {}",
                                        image_path.path().display()
                                    ));
                                }
                            }
                        }
                        Err(error) => failures.push(error),
                    }
                }
                Err(error) => failures.push(error),
            }
        }
        println!(
            "Checked {} unique materials across {models} exported models; {sky_materials} dedicated sky, {invisible_materials} without camera pass; {} binding failures",
            material_paths.len(),
            failures.len()
        );
        assert!(failures.is_empty(), "{}", failures.join("\n"));
    }

    #[test]
    #[ignore = "requires the authorized local native model export"]
    fn exported_native_models_validate() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("target/assets");
        let mut count = 0;
        let mut pending = vec![root.join("model_export")];
        while let Some(path) = pending.pop() {
            if path.is_dir() {
                pending.extend(
                    std::fs::read_dir(&path)
                        .unwrap()
                        .map(|file| file.unwrap().path()),
                );
                continue;
            }
            if path.extension().and_then(|e| e.to_str()) != Some("native_model") {
                continue;
            }
            let definition: ModelDefinition =
                serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
            let model = AnimatedModel {
                vertices: std::fs::read(root.join(&definition.vertices_file)).unwrap(),
                indices: std::fs::read(root.join(&definition.indices_file)).unwrap(),
                blends: std::fs::read(root.join(&definition.blends_file)).unwrap(),
                bind: std::fs::read(root.join(&definition.bind_file)).unwrap(),
                definition,
                materials: Arc::default(),
            };
            model
                .validate()
                .unwrap_or_else(|e| panic!("{}: {e}", path.display()));
            count += 1;
        }
        assert!(count > 0, "native model export is required");
        println!("Validated {count} native model assets");
    }
}

#[derive(Component)]
pub(crate) struct AnimatedModelInstance {
    pub model: Handle<AnimatedModel>,
    pub lod: Option<usize>,
}

#[derive(Component)]
struct NativeBoneBinding {
    mesh: Entity,
    joints: Vec<Entity>,
    inverse_bindposes: Handle<SkinnedMeshInverseBindposes>,
    bounds_meshes: Vec<Entity>,
    bounds: Aabb,
}

#[derive(Component, Default)]
pub(crate) struct NativeSkinPalette(pub Vec<Mat4>);

/// 004597d0/0043eef0 select scene-model LOD by source-origin distance divided
/// by the instance scale. 00792bd0 selects the model's rigid/skinned FOV ramp.
fn select_lods(
    cameras: Query<(&Camera, &GlobalTransform), With<super::GameCamera>>,
    models: Res<Assets<AnimatedModel>>,
    mut instances: Query<(
        &mut AnimatedModelInstance,
        &NativeBoneBinding,
        Option<&super::native_sonar::NativeSonarActor>,
    )>,
    meshes: Query<(&GlobalTransform, Has<NoFrustumCulling>)>,
    world: Option<Res<super::culling::VisibilityWorld>>,
    visible: Res<super::culling::SourceVisibility>,
    mut history: Local<[Option<f32>; 2]>,
) {
    let Ok((camera, transform)) = cameras.single() else {
        return;
    };
    let frame_inverse = super::world::source_frame().inverse();
    let eye = frame_inverse.transform_point3(transform.translation());
    let requested = super::models::lod_fov_scale(camera);
    assert!(requested.is_finite() && requested > 0.);
    for (ramp, limit) in [1.25, 0.4].into_iter().enumerate() {
        let previous = history[ramp].unwrap_or(requested);
        history[ramp] = Some(if previous <= requested || previous / requested > limit {
            requested
        } else {
            previous
        });
    }
    let axes = Mat4::from_rotation_x(-std::f32::consts::FRAC_PI_2);
    for (mut instance, binding, sonar) in &mut instances {
        let (Some(model), Ok((mesh, no_culling))) =
            (models.get(&instance.model), meshes.get(binding.mesh))
        else {
            continue;
        };
        // Source viewmodels use their explicitly bound view LOD.
        if no_culling {
            continue;
        }
        let pose = frame_inverse * mesh.to_matrix() * axes;
        let scale = pose.to_scale_rotation_translation().0.abs().max_element();
        let center = pose.transform_point3(binding.bounds.center.into());
        let extents = Mat3::from_mat4(pose).abs() * Vec3::from(binding.bounds.half_extents);
        let bounds = [(center - extents).to_array(), (center + extents).to_array()];
        let selected = if scale <= 0.
            || (sonar.is_none_or(|actor| !actor.active)
                && world
                    .as_ref()
                    .is_some_and(|world| !visible.dynamic_visible(world, &bounds)))
        {
            None
        } else {
            assert!(scale.is_finite(), "Original scene model scale");
            let raw = eye.distance(pose.transform_point3(Vec3::ZERO)) / scale;
            let adjusted = raw * history[model.definition.lod_ramp_type].unwrap();
            super::models::source_lod(
                model.definition.lods.iter().map(|lod| lod.dist),
                adjusted,
                raw,
            )
        };
        if instance.lod != selected {
            instance.lod = selected;
        }
    }
}

fn bind_native_bones(
    mut commands: Commands,
    instances: Query<(Entity, &AnimatedModelInstance), Without<NativeBoneBinding>>,
    models: Res<Assets<AnimatedModel>>,
    inverse_bindposes: Res<Assets<SkinnedMeshInverseBindposes>>,
    children: Query<&Children>,
    skins: Query<&SkinnedMesh>,
    meshes: Query<(), With<Mesh3d>>,
    names: Query<&Name>,
) {
    for (entity, instance) in &instances {
        let Some(model) = models.get(&instance.model) else {
            continue;
        };
        let lod = instance.lod.expect("initial native scene LOD");
        assert!(
            lod < model.definition.lods.len(),
            "native scene LOD is missing"
        );
        let skinned = children
            .iter_descendants(entity)
            .find_map(|e| skins.get(e).ok().map(|skin| (e, skin)));
        let Some((mesh, skin)) = skinned else {
            // Rigid GLB exports bake the bind pose into their vertices and omit
            // a skin. Their original PACKED32 streams use an identity palette.
            assert!(
                model.definition.lods[lod]
                    .surfaces
                    .iter()
                    .all(|surface| surface.blend_vertex_counts == [0; 4]),
                "blended native model must retain its Bevy skin"
            );
            let mesh = children
                .iter_descendants(entity)
                .find(|&e| meshes.contains(e))
                .expect("native model instance must retain its Bevy mesh");
            commands.entity(entity).insert((
                NativeBoneBinding {
                    mesh,
                    joints: Vec::new(),
                    inverse_bindposes: Handle::default(),
                    bounds_meshes: Vec::new(),
                    bounds: Aabb::from_min_max(
                        model.definition.bounds[0].into(),
                        model.definition.bounds[1].into(),
                    ),
                },
                NativeSkinPalette(vec![Mat4::IDENTITY; model.definition.bone_names.len()]),
            ));
            continue;
        };
        let Some(bind) = inverse_bindposes.get(&skin.inverse_bindposes) else {
            continue;
        };
        assert_eq!(
            skin.joints.len(),
            model.definition.bone_names.len(),
            "native bone count must match the scene skin"
        );
        assert_eq!(
            bind.len(),
            skin.joints.len(),
            "incomplete scene inverse bind poses"
        );
        for (&joint, source_name) in skin.joints.iter().zip(&model.definition.bone_names) {
            assert_eq!(
                names.get(joint).expect("native joint name").as_str(),
                source_name,
                "native blend indices must use the scene's bone order"
            );
        }
        commands.entity(entity).insert((
            NativeBoneBinding {
                mesh,
                joints: skin.joints.clone(),
                inverse_bindposes: skin.inverse_bindposes.clone(),
                bounds_meshes: children
                    .iter_descendants(entity)
                    .filter(|&e| meshes.contains(e))
                    .collect(),
                bounds: Aabb::from_min_max(
                    model.definition.bounds[0].into(),
                    model.definition.bounds[1].into(),
                ),
            },
            NativeSkinPalette(vec![Mat4::IDENTITY; skin.joints.len()]),
        ));
    }
}

fn update_native_bounds(
    mut instances: Query<&mut NativeBoneBinding>,
    meshes: Query<(Ref<GlobalTransform>, Ref<Aabb>), With<Mesh3d>>,
) {
    for mut binding in &mut instances {
        if binding.bounds_meshes.is_empty() {
            continue;
        }
        let (mesh, _) = meshes.get(binding.mesh).expect("bound native mesh bounds");
        // Hidden attachment groups can collapse their mesh transform to zero.
        // Retain the bounds until the mesh has an invertible pose again.
        if mesh.affine().matrix3.determinant() == 0. {
            continue;
        }
        if !binding.is_added()
            && !binding.bounds_meshes.iter().any(|&entity| {
                let (pose, bounds) = meshes.get(entity).expect("bound native primitive bounds");
                pose.is_changed() || bounds.is_changed()
            })
        {
            continue;
        }
        // Bevy already updates every GLB primitive's skinned bounds. Combine
        // all primitives in the PACKED32 draw frame, including offscreen limbs.
        let to_native =
            Mat4::from_rotation_x(std::f32::consts::FRAC_PI_2) * mesh.to_matrix().inverse();
        let mut minimum = Vec3::splat(f32::INFINITY);
        let mut maximum = Vec3::splat(f32::NEG_INFINITY);
        for &entity in &binding.bounds_meshes {
            let (pose, bounds) = meshes.get(entity).expect("bound native primitive bounds");
            let matrix = to_native * pose.to_matrix();
            let center = matrix.transform_point3(bounds.center.into());
            let extents = Mat3::from_mat4(matrix).abs() * Vec3::from(bounds.half_extents);
            minimum = minimum.min(center - extents);
            maximum = maximum.max(center + extents);
        }
        let bounds = Aabb::from_min_max(minimum, maximum);
        if binding.bounds != bounds {
            binding.bounds = bounds;
        }
    }
}

fn source_skin_matrix(mesh_inverse: Mat4, joint: Mat4, inverse_bind: Mat4) -> Mat4 {
    // GLB vertices and inverse binds use C=(x,z,-y), while PACKED32 is source
    // inches. Bevy's mesh-relative skin matrix removes instance scale/placement;
    // conjugate only its axis conversion, never apply the inch scale twice.
    let axes = Mat4::from_cols(Vec4::X, -Vec4::Z, Vec4::Y, Vec4::W);
    axes.transpose() * mesh_inverse * joint * inverse_bind * axes
}

fn update_native_palettes(
    mut instances: Query<(
        &NativeBoneBinding,
        &mut NativeSkinPalette,
        Ref<InheritedVisibility>,
        Ref<AnimatedModelInstance>,
    )>,
    transforms: Query<Ref<GlobalTransform>>,
    inverse_bindposes: Res<Assets<SkinnedMeshInverseBindposes>>,
) {
    for (binding, mut palette, visibility, instance) in &mut instances {
        if !visibility.get() || instance.lod.is_none() || binding.joints.is_empty() {
            continue;
        }
        let mesh = transforms.get(binding.mesh).expect("bound native mesh");
        if !palette.is_added()
            && !visibility.is_changed()
            && !instance.is_changed()
            && !inverse_bindposes.is_changed()
            && !mesh.is_changed()
            && !binding.joints.iter().any(|&joint| {
                transforms
                    .get(joint)
                    .expect("bound native bone")
                    .is_changed()
            })
        {
            continue;
        }
        let inverse_mesh = mesh.to_matrix().inverse();
        let bind = inverse_bindposes
            .get(&binding.inverse_bindposes)
            .expect("bound inverse poses");
        for (index, &joint) in binding.joints.iter().enumerate() {
            let matrix = source_skin_matrix(
                inverse_mesh,
                transforms
                    .get(joint)
                    .expect("bound native bone")
                    .to_matrix(),
                bind[index],
            );
            assert!(
                matrix.is_finite(),
                "native skin transform must be invertible"
            );
            if palette.0[index] != matrix {
                palette.0[index] = matrix;
            }
        }
    }
}

pub(crate) struct AnimatedModelPlugin;

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum AnimatedModelSystems {
    SelectLods,
    BindDraws,
}
impl Plugin for AnimatedModelPlugin {
    fn build(&self, app: &mut App) {
        app.init_asset::<AnimatedModel>()
            .init_asset_loader::<AnimatedModelLoader>()
            .add_observer(load_native_instance)
            .add_systems(Update, bind_native_bones)
            .add_systems(
                PostUpdate,
                update_native_bounds
                    .after(bevy::camera::visibility::VisibilitySystems::CalculateBounds)
                    .before(select_lods),
            )
            .add_systems(
                PostUpdate,
                select_lods
                    .in_set(AnimatedModelSystems::SelectLods)
                    .after(bevy::transform::TransformSystems::Propagate)
                    .after(bevy::camera::CameraUpdateSystems)
                    .after(super::culling::update_visibility)
                    .before(bind_animated_draws),
            )
            .add_systems(
                PostUpdate,
                bind_animated_draws
                    .in_set(AnimatedModelSystems::BindDraws)
                    .after(bevy::transform::TransformSystems::Propagate)
                    .after(bevy::camera::visibility::VisibilitySystems::VisibilityPropagate)
                    .before(bevy::camera::visibility::VisibilitySystems::CheckVisibility)
                    .before(super::world::object_constants),
            )
            .add_systems(
                PostUpdate,
                update_native_palettes
                    .after(bevy::transform::TransformSystems::Propagate)
                    .after(bevy::camera::visibility::VisibilitySystems::VisibilityPropagate),
            );
    }
}

fn load_native_instance(
    event: On<WorldInstanceReady>,
    roots: Query<&WorldAssetRoot>,
    children: Query<&Children>,
    meshes: Query<(), With<Mesh3d>>,
    server: Res<AssetServer>,
    definition: Res<crate::assets::GameDefinition>,
    mut commands: Commands,
) {
    let Ok(root) = roots.get(event.entity) else {
        return;
    };
    if !children
        .iter_descendants(event.entity)
        .any(|e| meshes.contains(e))
    {
        return;
    }
    let Some(path) = server.get_path(root.0.id()) else {
        return;
    };
    let path = path.path().to_string_lossy();
    let Some(name) = path
        .strip_prefix("model_export/")
        .and_then(|p| p.strip_suffix(".glb"))
    else {
        return;
    };
    let (name, lod) = if let Some((model, lod)) = name.rsplit_once("_lod") {
        (model, lod.parse().expect("native model scene LOD number"))
    } else {
        (
            definition
                .scene_models
                .get(path.as_ref())
                .unwrap_or_else(|| panic!("Missing original model binding for {path}"))
                .as_str(),
            0,
        )
    };
    commands
        .entity(event.entity)
        .remove::<(
            NativeBoneBinding,
            NativeSkinPalette,
            AnimatedModelInstance,
            AnimatedDraws,
            DynamicLightingOrigin,
        )>()
        .insert(AnimatedModelInstance {
            model: server.load(format!("model_export/{name}.native_model")),
            lod: Some(lod),
        });
}

#[derive(Component, Clone, Copy, ExtractComponent)]
#[extract_app(RenderApp)]
#[extract_component_filter(Or<(Changed<Self>, Changed<RenderEntity>)>)]
pub(crate) struct DynamicLightingOrigin(pub Vec3);

#[derive(Component)]
struct AnimatedDraws {
    model: AssetId<AnimatedModel>,
    lod: usize,
    probe: usize,
    surfaces: Vec<Entity>,
}

fn bind_animated_draws(
    mut commands: Commands,
    models: Res<Assets<AnimatedModel>>,
    probes: Option<Res<super::reflection::ReflectionWorld>>,
    server: Res<AssetServer>,
    mut materials: ResMut<Assets<super::native_model::NativeModelMaterial>>,
    mut instances: Query<(
        Entity,
        &AnimatedModelInstance,
        Ref<NativeBoneBinding>,
        &GlobalTransform,
        &InheritedVisibility,
        Option<&AnimatedDraws>,
        Option<&mut DynamicLightingOrigin>,
    )>,
    sonar_actors: Query<&super::native_sonar::NativeSonarActor>,
    meshes: Query<(
        &GlobalTransform,
        Option<&RenderLayers>,
        Has<NoFrustumCulling>,
    )>,
    mut draw_bounds: Query<&mut Aabb>,
) {
    let Some(probes) = probes else {
        return;
    };
    let source_frame = super::world::source_frame().inverse();
    let axes = Transform::from_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2));
    for (entity, instance, binding, root_transform, visibility, draws, lighting_origin) in
        &mut instances
    {
        if !visibility.get() {
            continue;
        }
        let Some(lod) = instance.lod else {
            if let Some(draws) = draws {
                for &surface in &draws.surfaces {
                    commands.entity(surface).try_despawn();
                }
                commands.entity(entity).remove::<AnimatedDraws>();
            }
            continue;
        };
        let Some(model) = models.get(&instance.model) else {
            continue;
        };
        let Ok((mesh_transform, layers, no_culling)) = meshes.get(binding.mesh) else {
            continue;
        };
        let origin = source_frame.transform_point3(root_transform.translation());
        let origin_changed = lighting_origin
            .as_ref()
            .is_none_or(|current| current.0 != origin);
        if origin_changed {
            if let Some(mut current) = lighting_origin {
                current.0 = origin;
            } else {
                commands
                    .entity(entity)
                    .insert(DynamicLightingOrigin(origin));
            }
        }
        let probe = draws
            .filter(|_| !origin_changed && !probes.is_changed())
            .map_or_else(|| probes.select(origin), |draws| draws.probe);
        if binding.is_changed()
            && let Some(draws) = draws
        {
            for &surface in &draws.surfaces {
                draw_bounds
                    .get_mut(surface)
                    .expect("native draw bounds")
                    .set_if_neq(binding.bounds);
            }
        }
        if draws.is_some_and(|d| d.model == instance.model.id() && d.lod == lod && d.probe == probe)
            && !models.is_changed()
            && !probes.is_changed()
        {
            continue;
        }
        if let Some(draws) = draws {
            for &surface in &draws.surfaces {
                commands.entity(surface).try_despawn();
            }
        }
        let source_probe = &probes.probes[probe];
        let probe_image = super::material::image(&server, &source_probe.reflection_image);
        let sh =
            std::array::from_fn(|i| Vec4::from_slice(&source_probe.lighting_sh[i * 4..i * 4 + 4]));
        let mut surfaces = Vec::new();
        for (index, surface) in model.definition.lods[lod].surfaces.iter().enumerate() {
            let source = &model.materials[&surface.material_file];
            if !source.has_camera_pass() {
                continue;
            }
            let (material, state) =
                source
                    .model(&server, probe_image.clone())
                    .unwrap_or_else(|e| {
                        panic!(
                            "Animated model {:?} surface {}: {e}",
                            instance.model.id(),
                            surface.material
                        )
                    });
            let mut draw = commands.spawn((
                ChildOf(binding.mesh),
                axes,
                mesh_transform.mul_transform(axes),
                Visibility::Inherited,
                layers.cloned().unwrap_or_default(),
                binding.bounds,
                super::world::WorldSurfaceLighting(sh),
                super::native_model::NativeModelSurface {
                    pass: source.world_pass(),
                    sort_key: (surface.material_draw_surf, index as u32),
                    vertex_lighting: None,
                    geometry: super::native_model::ModelGeometry::Animated {
                        model: instance.model.clone(),
                        instance: entity,
                        lod,
                        surface: index,
                    },
                    material: materials.add(material),
                    state,
                    primary_byte_offset: 0,
                    first_index: surface.first_index,
                    index_count: surface.tri_count * 3,
                    placement: entity,
                    colors_index: 0,
                    visibility: 0,
                },
            ));
            if let Ok(actor) = sonar_actors.get(entity) {
                draw.insert(super::native_sonar::NativeSonarSurfaceOwner(actor.owner));
            }
            if no_culling {
                draw.insert(NoFrustumCulling);
            }
            surfaces.push(draw.id());
        }
        commands.entity(entity).insert(AnimatedDraws {
            model: instance.model.id(),
            lod,
            probe,
            surfaces,
        });
    }
}

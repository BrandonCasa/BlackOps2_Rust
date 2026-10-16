//! Source weapon shader inputs, owned by each first-person rig/attachment.
use super::native::{NativeWorldObject, NativeWorldScene};
use bevy::{
    platform::collections::HashMap,
    prelude::*,
    render::{
        Extract, ExtractSchedule, RenderApp, render_resource::ShaderType, sync_world::MainEntity,
    },
};

/// Source cb1 weaponParam0/4 and UI3D window5 setup. Non-weapon models retain
/// the zeroed native command-source defaults; attachment owners supply values.
#[derive(Component, Clone, Copy, Default, PartialEq)]
pub(crate) struct NativeWeaponInputs {
    pub parameter0: Vec4,
    pub parameter4: Vec4,
    pub ui3d_setup5: Vec4,
}
#[derive(Component)]
pub(crate) struct NativeWeaponViewOwner(pub Entity);
#[derive(Resource, Default)]
pub(crate) struct NativeWeaponOwners(pub HashMap<MainEntity, NativeWeaponInputs>);
#[derive(Clone, Copy, ShaderType)]
pub(crate) struct NativeModelDrawMatrices {
    pub world_view: [Vec4; 4],
    pub inverse_world_view: [Vec4; 4],
    pub clip: [Vec4; 4],
    pub inverse_transpose: [Vec4; 3],
    pub parameter0: Vec4,
    pub parameter4: Vec4,
    pub ui3d_setup5: Vec4,
    pub target_size: Vec4,
}
pub(crate) struct NativeWeaponViewPlugin;
impl Plugin for NativeWeaponViewPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            PostUpdate,
            (attach_weapon_owners, update_weapon_inputs)
                .chain()
                .after(super::native_rangefinder_ui::RangefinderUiSystems::Update),
        );
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };
        render_app
            .init_resource::<NativeWeaponOwners>()
            .add_systems(ExtractSchedule, extract_weapon_inputs);
    }
}
fn attach_weapon_owners(
    mut commands: Commands,
    models: Query<Entity, Added<super::animated_models::AnimatedModelInstance>>,
    parents: Query<&ChildOf>,
    owners: Query<&NativeWeaponViewOwner>,
) {
    for entity in &models {
        let source = owners.get(entity).ok().or_else(|| {
            parents
                .iter_ancestors(entity)
                .find_map(|ancestor| owners.get(ancestor).ok())
        });
        if let Some(owner) = source {
            commands
                .entity(entity)
                .insert(NativeWeaponViewOwner(owner.0));
        }
    }
}
fn update_weapon_inputs(
    mut commands: Commands,
    owners: Query<(Entity, &NativeWeaponViewOwner, Option<&NativeWeaponInputs>)>,
    inventories: Query<&crate::weapons::WeaponInventory>,
    rangefinder: Res<super::native_rangefinder_ui::NativeRangefinderUi>,
) {
    for (entity, owner, previous) in &owners {
        let Ok(inventory) = inventories.get(owner.0) else {
            continue;
        };
        let Some(weapon) = inventory.active() else {
            continue;
        };
        let mut inputs = previous.copied().unwrap_or_default();
        // 005393e0 -> 0052bf90: X=GetClipAmmo; W=playerState[0x1e0] ADS.
        // Y/Z are separately owned source suppression/alternate-color flags.
        inputs.parameter0.x = weapon.magazine as f32;
        inputs.parameter0.w = inventory.ads;
        if rangefinder.owner == Some(owner.0) {
            inputs.parameter0.z = if rangefinder.alternate { 1. } else { 0. };
            inputs.ui3d_setup5 = Vec4::new(1., 1., 0., 0.);
        } else {
            inputs.parameter0.z = 0.;
            inputs.ui3d_setup5 = Vec4::ZERO;
        }
        if previous != Some(&inputs) {
            commands.entity(entity).insert(inputs);
        }
    }
}
fn extract_weapon_inputs(
    mut owners: ResMut<NativeWeaponOwners>,
    source: Extract<Query<(Entity, &NativeWeaponInputs), Changed<NativeWeaponInputs>>>,
    mut removed: Extract<RemovedComponents<NativeWeaponInputs>>,
) {
    for entity in removed.read() {
        owners.0.remove(&MainEntity::from(entity));
    }
    for (entity, inputs) in &source {
        owners.0.insert(entity.into(), *inputs);
    }
}
pub(crate) fn draw_constants(
    object: &NativeWorldObject,
    scene: &NativeWorldScene,
    target_size: UVec2,
    inputs: NativeWeaponInputs,
) -> NativeModelDrawMatrices {
    let world = Mat4::from_cols(
        object.world_rows[0],
        object.world_rows[1],
        object.world_rows[2],
        object.world_rows[3],
    )
    .transpose();
    let projection = Mat4::from_cols(
        scene.view_projection_rows[0],
        scene.view_projection_rows[1],
        scene.view_projection_rows[2],
        scene.view_projection_rows[3],
    )
    .transpose();
    // Shared native cb0[40..42]: RIGHT, UP, positive FORWARD.
    let rows = scene.inverse_view_rows;
    let camera = Mat4::from_cols(rows[0], rows[1], rows[2], Vec4::W).transpose();
    let world_view = camera * world;
    let inverse_world_view = world_view.inverse();
    let inverse_transpose = inverse_world_view.transpose();
    let clip = projection * world;
    NativeModelDrawMatrices {
        world_view: std::array::from_fn(|i| world_view.row(i)),
        inverse_world_view: std::array::from_fn(|i| inverse_world_view.row(i)),
        clip: std::array::from_fn(|i| clip.row(i)),
        inverse_transpose: std::array::from_fn(|i| inverse_transpose.row(i)),
        parameter0: inputs.parameter0,
        parameter4: inputs.parameter4,
        ui3d_setup5: inputs.ui3d_setup5,
        target_size: Vec4::new(target_size.x as f32, target_size.y as f32, 0., 0.),
    }
}

#[cfg(test)]
mod model_matrix_tests {
    use super::*;

    #[test]
    fn model_draw_matrices_reconstruct_rotated_scaled_decal_local_position() {
        let rotation = Quat::from_euler(EulerRot::YXZ, 0.1, -0.2, 0.05);
        let world = Mat4::from_scale_rotation_translation(
            Vec3::new(2., 3., 4.),
            Quat::from_euler(EulerRot::ZYX, 0.3, -0.4, 0.2),
            Vec3::new(5., 70., -3.),
        );
        let object = NativeWorldObject {
            world_rows: std::array::from_fn(|i| world.row(i)),
            reflection_lighting_sh0: Vec4::ZERO,
            reflection_lighting_sh1: Vec4::ZERO,
            reflection_lighting_sh2: Vec4::ZERO,
            color: Vec4::ONE,
        };
        let scene = NativeWorldScene {
            view_projection_rows: [Vec4::X, Vec4::Y, Vec4::Z, Vec4::W],
            inverse_view_rows: crate::rendering::source_inverse_view_rows(rotation),
            fog_color: Vec4::ZERO,
            fog_consts: Vec4::ZERO,
            fog_consts2: Vec4::ZERO,
            sun_fog_dir: Vec4::ZERO,
            sun_fog_color: Vec4::ZERO,
            sun_fog: Vec4::ZERO,
            hdr_control0: Vec4::ZERO,
            game_time: Vec4::ZERO,
            sun_position: Vec4::ZERO,
            sun_diffuse: Vec4::ZERO,
            omni_lights: Default::default(),
        };
        let inputs = NativeWeaponInputs {
            parameter0: Vec4::new(7., 0., 0., 0.5),
            parameter4: Vec4::splat(0.25),
            ..default()
        };
        let draw = draw_constants(&object, &scene, UVec2::new(1280, 720), inputs);
        let rows = |values: [Vec4; 4]| {
            Mat4::from_cols(values[0], values[1], values[2], values[3]).transpose()
        };
        let local = Vec3::new(0.25, -0.5, 0.7);
        let view_position = rows(draw.world_view).transform_point3(local);
        assert!(view_position.z > 0.);
        // Source floatZ stores zNear / positive view depth; its PS rebuilds the ray.
        let near = 2.;
        let stored_z = near / view_position.z;
        let depth = near / stored_z;
        let ray = view_position.truncate() / view_position.z;
        let reconstructed = Vec3::new(depth * ray.x, depth * ray.y, depth);
        let recovered = rows(draw.inverse_world_view).transform_point3(reconstructed);
        assert!(recovered.abs_diff_eq(local, 1e-5));
        let n = Vec3::new(
            draw.inverse_transpose[0].x,
            draw.inverse_transpose[1].x,
            draw.inverse_transpose[2].x,
        );
        let tangent = rows(draw.world_view).transform_vector3(Vec3::Y);
        assert!(n.dot(tangent).abs() < 1e-6);
        assert_eq!(draw.parameter0, inputs.parameter0);
        assert_eq!(draw.parameter4, inputs.parameter4);
        assert_eq!(draw.target_size, Vec4::new(1280., 720., 0., 0.));
    }
}

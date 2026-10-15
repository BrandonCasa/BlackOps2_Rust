use crate::{GameState, assets::GameDefinition, player::Player};
use bevy::prelude::*;
pub(crate) mod fog;
pub(crate) mod frame;
pub(crate) mod lighting;
pub(crate) mod material;
pub(crate) mod model_lighting;
pub mod native;
pub(crate) mod native_effect;
pub(crate) mod native_model;
pub(crate) mod world;
pub(crate) const BASE_FOV_DEGREES: f32 = 55.;
#[derive(Component)]
pub struct GameCamera;
pub struct GameRenderingPlugin;
impl Plugin for GameRenderingPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            frame::NativeFramePlugin,
            native::NativeWorldPlugin,
            native_effect::NativeEffectPlugin,
            model_lighting::ModelLightingPlugin,
            native_model::NativeModelPlugin,
        ))
        .add_systems(Startup, setup)
        .add_systems(Update, camera_follow)
        .add_systems(
            PostUpdate,
            (source_camera_constants, world::object_constants)
                .after(bevy::camera::CameraUpdateSystems)
                .after(bevy::transform::TransformSystems::Propagate),
        );
    }
}
pub(crate) fn setup(mut commands: Commands) {
    commands.spawn((
        Camera3d {
            depth_load_op: bevy::camera::Camera3dDepthLoadOp::Clear(0.),
            ..default()
        },
        fog::CameraFog::default(),
        frame::NativeRenderView,
        GameCamera,
        bevy::camera::visibility::RenderLayers::from_layers(&[0, 1]),
        Projection::Perspective(PerspectiveProjection {
            fov: BASE_FOV_DEGREES.to_radians(),
            // Native r_znear defaults to four inches (0041ad50 / 0074b550).
            near: 4. * crate::assets::SOURCE_UNIT_METERS,
            ..default()
        }),
        Transform::from_xyz(0., 2., 0.),
        bevy::audio::SpatialListener::new(0.2),
        Name::new("Origins camera"),
    ));
}

fn source_camera_constants(
    mut commands: Commands,
    world: Option<Res<fog::WorldFogDefinition>>,
    weather: Res<crate::environment::weather::Weather>,
    clock: Option<Res<crate::session::SessionClock>>,
    time: Res<Time<Virtual>>,
    mut cameras: Query<(Entity, &Camera, &GlobalTransform, &mut fog::CameraFog), With<GameCamera>>,
) {
    let (Some(world), Some(clock)) = (world, clock) else {
        return;
    };
    let bank = match weather.presented {
        crate::environment::weather::WeatherKind::Clear => 1,
        crate::environment::weather::WeatherKind::Rain => 2,
        crate::environment::weather::WeatherKind::Snow => 4,
    };
    // Native eye-relative inches -> Bevy eye-relative Y-up metres.
    let basis = world::source_frame();
    for (entity, camera, transform, mut fog) in &mut cameras {
        let (_, rotation, eye) = transform.to_scale_rotation_translation();
        let projection = camera.clip_from_view() * Mat4::from_quat(rotation.inverse()) * basis;
        // Native 00425750 clears depth to zero. Its depth-state table maps the
        // material's logical less/less_equal names to Greater/GreaterEqual.
        // Keep Bevy's reverse-Z projection in the same source convention.
        // Homogeneous clip W is also a source shader input: feathered particles
        // use it as positive view depth in inches. Cancel the boundary's metre
        // scale on every row, preserving NDC and the original clip-Z = zNear.
        let rows = std::array::from_fn(|i| projection.row(i) / crate::assets::SOURCE_UNIT_METERS);
        let mut scene = native::NativeWorldScene {
            view_projection_rows: rows,
            fog_color: Vec4::ZERO,
            fog_consts: Vec4::ZERO,
            fog_consts2: Vec4::ZERO,
            sun_fog_dir: Vec4::ZERO,
            sun_fog_color: Vec4::ZERO,
            sun_fog: Vec4::ZERO,
            hdr_control0: Vec4::ZERO,
        };
        fog.write_scene(
            &world,
            Vec3::new(eye.x, -eye.z, eye.y) / crate::assets::SOURCE_UNIT_METERS,
            bank,
            clock.milliseconds(&time),
            clock.is_changed() || world.is_changed(),
            &mut scene,
        );
        commands.entity(entity).insert(scene);
    }
}
fn camera_follow(
    players: Query<
        (
            &Player,
            &Transform,
            &crate::player::PreviousTranslation,
            &crate::weapons::WeaponInventory,
            &crate::movement::Mobility,
            &crate::movement::MotionFeedback,
        ),
        Without<GameCamera>,
    >,
    time: Res<Time<Fixed>>,
    mut camera: Query<(&mut Transform, &mut Projection), With<GameCamera>>,
    definition: Option<Res<GameDefinition>>,
    state: Res<State<GameState>>,
) {
    if !matches!(
        **state,
        GameState::StartingSession | GameState::Playing | GameState::Paused | GameState::GameOver
    ) {
        return;
    }
    let (
        Ok((player, body, previous, inventory, mobility, feedback)),
        Some(definition),
        Ok((mut camera, mut projection)),
    ) = (players.single(), definition, camera.single_mut())
    else {
        return;
    };
    let eye = if **state == GameState::Playing {
        mobility
            .previous_eye
            .lerp(player.eye_height, time.overstep_fraction())
    } else {
        player.eye_height
    };
    let bob = feedback
        .previous_bob
        .lerp(feedback.bob, time.overstep_fraction());
    let target = Transform {
        translation: (if **state == GameState::Playing {
            previous.0.lerp(body.translation, time.overstep_fraction())
        } else {
            body.translation
        }) + Vec3::Y * (eye - player.height * 0.5)
            + Quat::from_rotation_y(player.yaw) * Vec3::new(bob.x, bob.y, 0.),
        rotation: Quat::from_rotation_y(player.yaw + inventory.recoil.x + feedback.rumble.x)
            * Quat::from_rotation_x(
                (player.pitch + inventory.recoil.y + feedback.rumble.y).clamp(-1.5, 1.5),
            ),
        ..*camera
    };
    camera.set_if_neq(target);
    if **state == GameState::Playing
        && let Projection::Perspective(perspective) = &mut *projection
    {
        perspective.fov = inventory
            .definition(&definition.content.weapons)
            .map_or(BASE_FOV_DEGREES.to_radians(), |w| {
                inventory.ads_fov(BASE_FOV_DEGREES, w)
            });
    }
}

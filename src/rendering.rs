use crate::{GameState, assets::GameDefinition, player::Player};
use bevy::prelude::*;
pub(crate) mod culling;
pub(crate) mod display;
mod dlss;
pub(crate) use dlss::reset_history;
pub(crate) mod fog;
pub(crate) mod frame;
pub(crate) mod light_grid;
pub(crate) mod lighting;
pub(crate) mod material;
pub(crate) mod model_lighting;
pub(crate) mod models;
pub mod native;
pub(crate) mod native_effect;
mod native_film;
pub(crate) mod native_light;
pub(crate) mod native_model;
pub(crate) mod native_rangefinder_ui;
pub(crate) mod native_sky;
pub(crate) mod native_sonar;
mod native_vision;
pub(crate) mod native_water;
pub(crate) mod native_weapon_view;
pub(crate) mod reflection;
pub(crate) mod shadow;
pub(crate) mod skinning;
pub(crate) mod world;
pub(crate) const BASE_FOV_DEGREES: f32 = 55.;
#[derive(Component)]
pub struct GameCamera;
pub(crate) mod animated_models;
pub struct GameRenderingPlugin;
impl Plugin for GameRenderingPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            dlss::NativeDlssPlugin,
            frame::NativeFramePlugin,
            native::NativeWorldPlugin,
            native_effect::NativeEffectPlugin,
            model_lighting::ModelLightingPlugin,
            native_model::NativeModelPlugin,
            native_weapon_view::NativeWeaponViewPlugin,
            native_rangefinder_ui::NativeRangefinderUiPlugin,
            native_sky::NativeSkyPlugin,
            native_sonar::NativeSonarPlugin,
            skinning::NativeSkinningPlugin,
            animated_models::AnimatedModelPlugin,
            native_water::NativeWaterPlugin,
            shadow::ShadowPlugin,
        ))
        .add_plugins((
            native_film::NativeFilmPlugin,
            native_vision::NativeVisionPlugin,
            display::NativeDisplayPlugin,
        ))
        .init_resource::<culling::SourceVisibility>()
        .add_systems(Startup, setup)
        .add_systems(
            PostUpdate,
            culling::update_visibility
                .after(bevy::camera::visibility::VisibilitySystems::UpdateFrusta)
                .after(bevy::transform::TransformSystems::Propagate)
                .before(models::select_lods)
                .before(bevy::camera::visibility::VisibilitySystems::CheckVisibility),
        )
        .add_systems(
            PostUpdate,
            culling::filter_visibility
                .after(bevy::camera::visibility::VisibilitySystems::CheckVisibility),
        )
        .add_systems(Update, (camera_follow, dlss::toggle_neural_rendering))
        .add_systems(
            PostUpdate,
            (source_camera_constants, world::object_constants)
                .after(culling::filter_visibility)
                .after(bevy::camera::CameraUpdateSystems)
                .after(bevy::camera::visibility::VisibilitySystems::UpdateFrusta)
                .after(frame::maintain_float_z)
                .after(bevy::transform::TransformSystems::Propagate),
        )
        .add_systems(
            PostUpdate,
            models::select_lods
                .after(bevy::camera::CameraUpdateSystems)
                .after(bevy::transform::TransformSystems::Propagate)
                .before(bevy::camera::visibility::VisibilitySystems::CheckVisibility),
        );
    }
}
pub(crate) fn setup(
    mut commands: Commands,
    dlss_supported: Option<Res<bevy::anti_alias::dlss::DlssSuperResolutionSupported>>,
    neural_rendering: Option<
        Res<bevy::anti_alias::dlss::neural_rendering::DlssNeuralRenderingStatus>,
    >,
) {
    let mut camera = commands.spawn((
        Camera3d {
            depth_load_op: bevy::camera::Camera3dDepthLoadOp::Clear(0.),
            ..default()
        },
        fog::CameraFog::default(),
        frame::NativeRenderView,
        bevy::core_pipeline::tonemapping::Tonemapping::None,
        bevy::core_pipeline::tonemapping::DebandDither::Disabled,
        GameCamera,
        Msaa::Off,
        bevy::core_pipeline::prepass::NoBackgroundMotionVectors,
        bevy::camera::visibility::RenderLayers::from_layers(&[0, 1]),
        Projection::Perspective(PerspectiveProjection {
            fov: BASE_FOV_DEGREES.to_radians(),
            // Native r_znear defaults to four inches (0041ad50 / 0074b550).
            near: 4. * crate::assets::SOURCE_UNIT_METERS,
            far: culling::CULL_DISTANCE * crate::assets::SOURCE_UNIT_METERS,
            ..default()
        }),
        Transform::from_xyz(0., 2., 0.),
        bevy::audio::SpatialListener::new(0.2),
        Name::new("Origins camera"),
    ));
    if dlss_supported.is_some() {
        camera.insert(bevy::anti_alias::dlss::Dlss::<
            bevy::anti_alias::dlss::DlssSuperResolutionFeature,
        > {
            perf_quality_mode: bevy::anti_alias::dlss::DlssPerfQualityMode::UltraPerformance,
            reset: true,
            ..default()
        });
        info!("DLSS Super Resolution: Ultra Performance");
    } else {
        warn!("DLSS Super Resolution unavailable; rendering at native resolution");
    }
    use bevy::anti_alias::dlss::neural_rendering::{
        DlssNeuralRendering, DlssNeuralRenderingStatus,
    };
    if matches!(
        neural_rendering.as_deref(),
        Some(DlssNeuralRenderingStatus::Ready)
    ) {
        camera.insert(DlssNeuralRendering {
            style: 2, // Cinematic (third style).
            ..default()
        });
        info!("DLSS Neural Rendering enabled (F11 toggles)");
    }
}

fn source_camera_constants(
    mut commands: Commands,
    world: Option<Res<fog::WorldFogDefinition>>,
    weather: Res<crate::environment::weather::Weather>,
    clock: Option<Res<crate::session::SessionClock>>,
    water_lights: Option<Res<world::WorldWaterLighting>>,
    shadow_volumes: Option<Res<shadow::ShadowWorld>>,
    time: Res<Time<Virtual>>,
    omni_lights: Query<(&GlobalTransform, &native_light::FxOmniLight)>,
    mut cameras: Query<
        (
            Entity,
            Ref<Camera>,
            Ref<GlobalTransform>,
            &bevy::camera::primitives::Frustum,
            &mut fog::CameraFog,
            Option<&frame::NativeSceneImage>,
            Option<&mut native_water::NativeWaterView>,
            Option<&shadow::ShadowViewSettings>,
            Option<&shadow::sun::SunShadowViews>,
        ),
        With<GameCamera>,
    >,
) {
    let (Some(world), Some(clock)) = (world, clock) else {
        return;
    };
    let bank = match weather.vision {
        crate::environment::weather::WeatherKind::Clear => 1,
        crate::environment::weather::WeatherKind::Rain => 2,
        crate::environment::weather::WeatherKind::Snow => 4,
    };
    // Native eye-relative inches -> Bevy eye-relative Y-up metres.
    let basis = world::source_frame();
    let mut sun = world.initial_sun_light();
    sun.diffuse_color = weather.sun_diffuse();
    for (
        entity,
        camera,
        transform,
        frustum,
        mut fog,
        resolved_scene,
        water,
        shadow_settings,
        sun_views,
    ) in &mut cameras
    {
        let (_, rotation, eye) = transform.to_scale_rotation_translation();
        let eye_native = Vec3::new(eye.x, -eye.z, eye.y) / crate::assets::SOURCE_UNIT_METERS;
        let mut submissions = native_light::OmniSubmissions::default();
        for (transform, light) in &omni_lights {
            let position = transform.translation();
            if light.radius > 0.
                && frustum.intersects_sphere(
                    &bevy::camera::primitives::Sphere {
                        center: position.into(),
                        radius: light.radius * crate::assets::SOURCE_UNIT_METERS,
                    },
                    !light.ignore_far_plane,
                )
            {
                submissions.submit(
                    Vec3::new(position.x, -position.z, position.y)
                        / crate::assets::SOURCE_UNIT_METERS,
                    light,
                );
            }
        }
        if let Some(volumes) = &shadow_volumes
            && (shadow_settings.is_none() || transform.is_changed() || volumes.is_changed())
        {
            let settings = volumes.settings(eye_native);
            if shadow_settings != Some(&settings) {
                commands.entity(entity).insert(settings);
            }
        }
        if let (Some(lights), Some(image)) = (&water_lights, resolved_scene) {
            // 0041ba60 converts current refdef milliseconds to float seconds.
            let seconds = clock.milliseconds(&time) as f32 * 0.001;
            if let Some(mut water) = water {
                water.game_time_seconds = seconds;
                water.eye_position = eye_native;
                if !std::sync::Arc::ptr_eq(&water.lights, &lights.0) {
                    water.lights = lights.0.clone();
                }
            } else {
                commands
                    .entity(entity)
                    .insert(native_water::NativeWaterView {
                        resolved_post_sun: image.0.clone(),
                        game_time_seconds: seconds,
                        eye_position: eye_native,
                        lights: lights.0.clone(),
                    });
            }
        }
        let projection = camera.clip_from_view() * Mat4::from_quat(rotation.inverse()) * basis;
        // Native 00425750 clears depth to zero. Its depth-state table maps the
        // material's logical less/less_equal names to Greater/GreaterEqual.
        // Keep Bevy's reverse-Z projection in the same source convention.
        // Homogeneous clip W is also a source shader input: feathered particles
        // use it as positive view depth in inches. Cancel the boundary's metre
        // scale on every row, preserving NDC and the original clip-Z = zNear.
        let rows = std::array::from_fn(|i| projection.row(i) / crate::assets::SOURCE_UNIT_METERS);
        if let Some(shadows) = &shadow_volumes
            && (sun_views.is_none()
                || transform.is_changed()
                || camera.is_changed()
                || shadows.is_changed()
                || world.is_changed())
        {
            let views = shadows.sun_views(
                eye_native,
                sun.dir,
                Mat4::from_cols(rows[0], rows[1], rows[2], rows[3]).transpose(),
            );
            if sun_views != Some(&views) {
                commands.entity(entity).insert(views);
            }
        }
        let mut scene = native::NativeWorldScene {
            view_projection_rows: rows,
            inverse_view_rows: source_inverse_view_rows(rotation),
            fog_color: Vec4::ZERO,
            fog_consts: Vec4::ZERO,
            fog_consts2: Vec4::ZERO,
            sun_fog_dir: Vec4::ZERO,
            sun_fog_color: Vec4::ZERO,
            sun_fog: Vec4::ZERO,
            hdr_control0: Vec4::ZERO,
            game_time: native::shader_game_time(clock.milliseconds(&time) as f32 * 0.001),
            sun_position: sun.dir.extend(0.0),
            sun_diffuse: sun.diffuse_color,
            omni_lights: submissions.view(eye_native),
        };
        fog.write_scene(
            &world,
            eye_native,
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
        rotation: Quat::from_rotation_y(
            player.yaw
                + inventory.recoil.x
                + feedback.rumble.x
                + feedback.view_angles.y.to_radians(),
        ) * Quat::from_rotation_x(
            (player.pitch + inventory.recoil.y + feedback.rumble.y
                - feedback.view_angles.x.to_radians())
            .clamp(-1.5, 1.5),
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

// 0074b5a0 constructs source view columns (-refdef.axis[1], axis[2], axis[0]);
// 0046b5b0 supplies code0xd8, its inverse, directly to 00475c50's row upload.
// The three register rows are source-space camera right, up and positive-Z forward.
fn source_inverse_view_rows(rotation: Quat) -> [Vec4; 3] {
    [Vec3::X, Vec3::Y, -Vec3::Z].map(|axis| {
        let direction = rotation * axis;
        Vec3::new(direction.x, -direction.z, direction.y)
            .normalize()
            .extend(0.)
    })
}

#[cfg(test)]
mod source_camera_matrix_tests {
    use super::*;

    #[test]
    fn source_inverse_view_preserves_original_axis_order_and_forward_sign() {
        assert_eq!(
            source_inverse_view_rows(Quat::IDENTITY),
            [Vec4::X, Vec4::Z, Vec4::Y]
        );
        let position = Vec3::new(64., -12., 9.);
        for rotation in [
            Quat::IDENTITY,
            Quat::from_rotation_y(0.7),
            Quat::from_euler(EulerRot::YXZ, -1.2, 0.4, 0.1),
        ] {
            let rows = source_inverse_view_rows(rotation);
            for row in rows {
                assert!((row.truncate().length() - 1.).abs() < 1e-6);
                assert_eq!(row.w, 0.);
            }
            let expected = rotation.inverse() * world::source_frame().transform_vector3(position)
                / crate::assets::SOURCE_UNIT_METERS;
            let actual = Vec3::new(
                rows[0].truncate().dot(position),
                rows[1].truncate().dot(position),
                rows[2].truncate().dot(position),
            );
            assert!(actual.abs_diff_eq(Vec3::new(expected.x, expected.y, -expected.z), 1e-5));
            let forward = rows[2].truncate() * 100.;
            assert!((rows[2].truncate().dot(forward) - 100.).abs() < 1e-4);
        }
    }
}

#![forbid(unsafe_code)]
use bevy::{
    camera::CameraProjection,
    input::{
        ButtonState, InputPlugin,
        keyboard::{Key, KeyboardFocusLost, KeyboardInput, NativeKey},
    },
    prelude::*,
    time::{TimePlugin, TimeUpdateStrategy},
};
use bo2_geometry_viewer::{
    camera::{InspectionCamera, InspectionCameraPlugin},
    synthetic_snapshot,
};
use std::time::Duration;

fn fixture() -> (InspectionCamera, Transform) {
    let mut controller = InspectionCamera::new(Vec3::ZERO, 10.0).unwrap();
    let transform = controller.home(4.0 / 3.0);
    (controller, transform)
}

#[test]
fn forward_strafe_and_up_have_explicit_diagnostic_directions() {
    let (_, initial) = fixture();
    for (key, direction) in [
        (KeyCode::KeyW, *initial.forward()),
        (KeyCode::KeyD, *initial.right()),
        (KeyCode::KeyE, Vec3::Y),
    ] {
        let (mut controller, mut transform) = fixture();
        let home = transform;
        let mut keys = ButtonInput::default();
        keys.press(key);
        controller.step(&mut transform, &keys, 0.05, 4.0 / 3.0);
        assert!(
            transform
                .translation
                .abs_diff_eq(home.translation + direction * 0.5, 0.00001)
        );
    }
}

#[test]
fn opposing_keys_cancel_diagonals_normalize_and_speed_modifiers_work() {
    let (mut controller, mut transform) = fixture();
    let home = transform;
    let mut keys = ButtonInput::default();
    for key in [
        KeyCode::KeyW,
        KeyCode::KeyS,
        KeyCode::KeyA,
        KeyCode::KeyD,
        KeyCode::KeyE,
        KeyCode::KeyQ,
        KeyCode::ArrowLeft,
        KeyCode::ArrowRight,
        KeyCode::ArrowUp,
        KeyCode::ArrowDown,
    ] {
        keys.press(key);
    }
    controller.step(&mut transform, &keys, 0.1, 4.0 / 3.0);
    assert_eq!(transform.translation, home.translation);
    assert_eq!(transform.rotation, home.rotation);
    for (modifier, expected) in [
        (None, 1.0),
        (Some(KeyCode::ShiftLeft), 5.0),
        (Some(KeyCode::ControlLeft), 0.1),
    ] {
        let (mut controller, mut transform) = fixture();
        let mut keys = ButtonInput::default();
        keys.press(KeyCode::KeyW);
        keys.press(KeyCode::KeyD);
        if let Some(modifier) = modifier {
            keys.press(modifier);
        }
        controller.step(&mut transform, &keys, 0.1, 4.0 / 3.0);
        assert!((transform.translation.distance(home.translation) - expected).abs() < 0.00001);
    }
}

#[test]
fn look_is_orthonormal_pitch_bounded_and_time_is_finite_and_capped() {
    let (mut controller, mut transform) = fixture();
    let home = transform;
    let mut keys = ButtonInput::default();
    keys.press(KeyCode::ArrowLeft);
    keys.press(KeyCode::ArrowUp);
    for _ in 0..1000 {
        controller.step(&mut transform, &keys, 10_000.0, 4.0 / 3.0);
    }
    assert!(transform.rotation.is_finite());
    assert!((transform.rotation.length() - 1.0).abs() < 0.00001);
    assert!(transform.forward().dot(Vec3::Y).abs() <= 89.0_f32.to_radians().sin() + 0.00001);
    assert_eq!(transform.translation, home.translation);
    for dt in [f32::NAN, f32::INFINITY, -1.0, 0.0] {
        let before = transform;
        controller.step(&mut transform, &keys, dt, 4.0 / 3.0);
        assert_eq!(transform, before);
    }
    let (mut controller, mut transform) = fixture();
    let mut keys = ButtonInput::default();
    keys.press(KeyCode::KeyW);
    controller.step(&mut transform, &keys, 1000.0, 4.0 / 3.0);
    assert!((transform.translation.distance(home.translation) - 1.0).abs() < 0.00001);
    for _ in 0..20_000 {
        controller.step(&mut transform, &keys, 1000.0, 4.0 / 3.0);
    }
    assert!(
        transform.translation.is_finite() && transform.translation.abs().max_element() <= 1000.0
    );
}

#[test]
fn reset_overrides_motion_and_frames_asymmetric_geometry_in_portrait_and_landscape() {
    let snapshot = synthetic_snapshot().unwrap();
    let center = Vec3::new(0.5, 0.5, 2.0);
    let radius = Vec3::new(2.5, 1.5, 2.0).length();
    for aspect in [0.08, 0.5, 1.0, 4.0 / 3.0, 16.0] {
        let mut camera = InspectionCamera::new(center, radius).unwrap();
        let expected = camera.home(aspect);
        let mut transform = Transform::from_translation(Vec3::splat(99.0));
        let mut keys = ButtonInput::default();
        keys.press(KeyCode::KeyW);
        keys.press(KeyCode::KeyR);
        camera.step(&mut transform, &keys, 1.0, aspect);
        assert_eq!(transform, expected);
        let mut projection = camera.projection();
        projection.update(720.0 * aspect, 720.0);
        let matrix = projection.get_clip_from_view() * transform.to_matrix().inverse();
        for vertex in snapshot.positions() {
            let clip = matrix.project_point3(Vec3::from_array(*vertex));
            assert!(
                clip.is_finite()
                    && clip.x.abs() < 1.0
                    && clip.y.abs() < 1.0
                    && clip.z > 0.0
                    && clip.z < 1.0
            );
        }
    }
    assert!(InspectionCamera::new(Vec3::splat(f32::MAX), 10.0).is_none());
    assert!(InspectionCamera::new(Vec3::ZERO, f32::NAN).is_none());
    assert!(InspectionCamera::new(Vec3::splat(1_000_000.0), 0.001).is_none());
}

fn key(app: &mut App, window: Entity, key_code: KeyCode, state: ButtonState) {
    app.world_mut().write_message(KeyboardInput {
        key_code,
        logical_key: Key::Unidentified(NativeKey::Unidentified),
        state,
        text: None,
        repeat: false,
        window,
    });
}

#[test]
fn bevy_keyboard_messages_drive_camera_reset_focus_loss_and_escape() {
    let mut app = App::new();
    app.add_plugins((TimePlugin, InputPlugin, InspectionCameraPlugin))
        .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_millis(
            50,
        )));
    let window = app.world_mut().spawn_empty().id();
    let (controller, home) = fixture();
    let entity = app.world_mut().spawn((controller, home)).id();
    app.update();
    key(&mut app, window, KeyCode::KeyW, ButtonState::Pressed);
    app.update();
    let moved = *app.world().get::<Transform>(entity).unwrap();
    assert!((moved.translation.distance(home.translation) - 0.5).abs() < 0.00001);
    key(&mut app, window, KeyCode::KeyW, ButtonState::Released);
    key(&mut app, window, KeyCode::ArrowLeft, ButtonState::Pressed);
    app.update();
    let looked = *app.world().get::<Transform>(entity).unwrap();
    assert!(looked.rotation.angle_between(home.rotation) > 0.01);
    key(&mut app, window, KeyCode::KeyR, ButtonState::Pressed);
    app.update();
    assert_eq!(*app.world().get::<Transform>(entity).unwrap(), home);
    app.world_mut().write_message(KeyboardFocusLost);
    app.update();
    assert_eq!(*app.world().get::<Transform>(entity).unwrap(), home);
    key(&mut app, window, KeyCode::Escape, ButtonState::Pressed);
    app.update();
    assert_eq!(app.should_exit(), Some(AppExit::Success));
}

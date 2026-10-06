//! Tool-local keyboard flight controls. Axes, speed and timing are diagnostic choices.
use bevy::prelude::*;

/// Camera frame derived from the already-admitted raw geometry bounds sphere.
#[derive(Component, Clone)]
pub struct InspectionCamera {
    center: Vec3,
    radius: f32,
    yaw: f32,
    pitch: f32,
    home_pose: Option<Transform>,
}

impl InspectionCamera {
    pub fn new(center: Vec3, radius: f32) -> Option<Self> {
        if !center.is_finite()
            || center.abs().max_element() > 10_000_000.0
            || !radius.is_finite()
            || !(0.001..=1_000_000.0).contains(&radius)
            || radius < center.abs().max_element() * 0.00001
        {
            return None;
        }
        let mut camera = Self {
            center,
            radius,
            yaw: 0.0,
            pitch: 0.0,
            home_pose: None,
        };
        camera.home(4.0 / 3.0);
        Some(camera)
    }

    pub fn at_pose(pose: Transform, speed: f32) -> Option<Self> {
        if !pose.translation.is_finite()
            || !pose.rotation.is_finite()
            || (pose.rotation.length_squared() - 1.0).abs() > 0.0001
            || pose.scale != Vec3::ONE
        {
            return None;
        }
        let mut camera = Self::new(Vec3::ZERO, speed)?;
        camera.center = pose.translation;
        camera.home_pose = Some(pose);
        camera.home(16.0 / 9.0);
        Some(camera)
    }

    /// Sphere fits the shorter perspective dimension with 25% angular margin.
    pub fn home(&mut self, aspect: f32) -> Transform {
        if let Some(pose) = self.home_pose {
            let forward = *pose.forward();
            self.yaw = (-forward.x).atan2(-forward.z);
            self.pitch = forward.y.asin();
            return pose;
        }
        let aspect = if aspect.is_finite() {
            aspect.clamp(0.05, 20.0)
        } else {
            1.0
        };
        let vertical = std::f32::consts::FRAC_PI_6;
        let horizontal = (vertical.tan() * aspect).atan();
        let distance = self.radius * 1.25 / vertical.min(horizontal).sin();
        let transform = Transform::from_translation(self.center + Vec3::ONE.normalize() * distance)
            .looking_at(self.center, Vec3::Y);
        let forward = *transform.forward();
        self.yaw = (-forward.x).atan2(-forward.z);
        self.pitch = forward.y.asin();
        transform
    }

    pub fn projection(&self) -> PerspectiveProjection {
        PerspectiveProjection {
            fov: std::f32::consts::FRAC_PI_3,
            near: (self.radius * 0.0001).max(0.000_001),
            far: self.radius * 400.0,
            ..default()
        }
    }

    /// Bounded dt prevents a stall from teleporting the inspection camera.
    /// Opposing keys cancel; diagonal movement has the same speed as one axis.
    pub fn step(
        &mut self,
        transform: &mut Transform,
        keys: &ButtonInput<KeyCode>,
        dt: f32,
        aspect: f32,
    ) {
        if keys.just_pressed(KeyCode::KeyR) {
            *transform = self.home(aspect);
            return;
        }
        if !dt.is_finite() || dt <= 0.0 {
            return;
        }
        let dt = dt.min(0.1);
        let axis = |positive, negative| {
            i32::from(keys.pressed(positive)) as f32 - i32::from(keys.pressed(negative)) as f32
        };
        let yaw = axis(KeyCode::ArrowLeft, KeyCode::ArrowRight);
        let pitch = axis(KeyCode::ArrowUp, KeyCode::ArrowDown);
        if yaw != 0.0 || pitch != 0.0 {
            self.yaw = (self.yaw + yaw * dt).rem_euclid(std::f32::consts::TAU);
            self.pitch =
                (self.pitch + pitch * dt).clamp(-89.0_f32.to_radians(), 89.0_f32.to_radians());
            transform.rotation =
                Quat::from_rotation_y(self.yaw) * Quat::from_rotation_x(self.pitch);
        }
        let direction = *transform.forward() * axis(KeyCode::KeyW, KeyCode::KeyS)
            + *transform.right() * axis(KeyCode::KeyD, KeyCode::KeyA)
            + Vec3::Y * axis(KeyCode::KeyE, KeyCode::KeyQ);
        if direction == Vec3::ZERO {
            return;
        }
        let multiplier =
            if keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight) {
                0.1
            } else if keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight) {
                5.0
            } else {
                1.0
            };
        let candidate =
            transform.translation + direction.normalize_or_zero() * self.radius * multiplier * dt;
        if candidate.is_finite() {
            // Limit wandering to a finite region relative to the unchanged geometry.
            transform.translation = self.center
                + (candidate - self.center).clamp(
                    Vec3::splat(-100.0 * self.radius),
                    Vec3::splat(100.0 * self.radius),
                );
        }
    }
}

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct InspectionControlSystems;

pub struct InspectionCameraPlugin;
impl Plugin for InspectionCameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (control_camera, escape).in_set(InspectionControlSystems),
        );
    }
}

fn control_camera(
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
    mut cameras: Query<(&mut InspectionCamera, &mut Transform)>,
) {
    let aspect = windows.iter().next().map_or(4.0 / 3.0, |window| {
        window.width() / window.height().max(1.0)
    });
    for (mut camera, mut transform) in &mut cameras {
        camera.step(&mut transform, &keys, time.delta_secs(), aspect);
    }
}

fn escape(keys: Res<ButtonInput<KeyCode>>, mut exit: MessageWriter<AppExit>) {
    if keys.just_pressed(KeyCode::Escape) {
        exit.write(AppExit::Success);
    }
}

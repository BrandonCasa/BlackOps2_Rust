use avian3d::prelude::*;
use bevy::prelude::*;

#[derive(Clone, Copy, serde::Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct PhysicsTuning {
    pub ground_acceleration: f32,
    pub crouch_acceleration: f32,
    pub prone_acceleration: f32,
    pub air_acceleration: f32,
    pub friction: f32,
    pub stop_speed: f32,
    pub strafe_scale: f32,
    pub backward_scale: f32,
    pub sprint_strafe_scale: f32,
    pub ground_normal_min: f32,
    pub ground_probe: f32,
    pub step_down: f32,
    pub skin_width: f32,
    pub step_slowdown_height: f32,
    pub step_speed_min: f32,
}

#[derive(Resource, Default)]
pub struct PhysicsEvidence {
    pub samples: Vec<serde_json::Value>,
    pub projectiles: Vec<serde_json::Value>,
}

pub struct MotionOutput {
    pub slide: MoveAndSlideOutput,
    pub step_adjustment: f32,
    pub step_rise: Option<f32>,
    pub step_normal: Option<Vec3>,
    pub step_surface: Option<(Vec3, f32)>,
}
impl Default for PhysicsTuning {
    fn default() -> Self {
        Self {
            ground_acceleration: 9.,
            crouch_acceleration: 12.,
            prone_acceleration: 19.,
            air_acceleration: 1.,
            friction: 5.5,
            stop_speed: 100. * 0.0254,
            strafe_scale: 0.8,
            backward_scale: 0.7,
            sprint_strafe_scale: 0.667,
            ground_normal_min: 0.7,
            ground_probe: 0.0254,
            step_down: 9. * 0.0254,
            skin_width: 0.00635,
            step_slowdown_height: 0.5 * 0.0254,
            step_speed_min: 1. - 0.8,
        }
    }
}
impl PhysicsTuning {
    pub fn valid(&self) -> bool {
        [
            self.ground_acceleration,
            self.crouch_acceleration,
            self.prone_acceleration,
            self.air_acceleration,
            self.friction,
            self.stop_speed,
            self.strafe_scale,
            self.backward_scale,
            self.sprint_strafe_scale,
            self.ground_probe,
            self.step_down,
            self.skin_width,
            self.step_slowdown_height,
            self.step_speed_min,
        ]
        .iter()
        .all(|v| v.is_finite() && *v > 0.)
            && self.ground_normal_min.is_finite()
            && self.ground_normal_min > 0.
            && self.ground_normal_min < 1.
            && self.skin_width < self.ground_probe
            && self.step_speed_min < 1.
    }

    pub fn wish_velocity(
        &self,
        input: Vec2,
        yaw: f32,
        speed: f32,
        sprint: bool,
        grounded: bool,
    ) -> Vec3 {
        let input = input.clamp(Vec2::splat(-1.), Vec2::ONE);
        let forward = input.y.abs()
            * if input.y < 0. {
                self.backward_scale
            } else {
                1.
            };
        let strafe = input.x.abs() * self.strafe_scale;
        let scale = if grounded {
            forward.max(strafe)
        } else {
            input.abs().max_element()
        };
        let direction = Vec3::new(
            input.x * if sprint { self.sprint_strafe_scale } else { 1. },
            0.,
            -input.y,
        );
        Quat::from_rotation_y(yaw) * direction.normalize_or_zero() * speed * scale
    }

    pub fn velocity(
        &self,
        velocity: Vec3,
        wish: Vec3,
        ground_normal: Option<Vec3>,
        prone: bool,
        crouched: bool,
        dt: f32,
    ) -> Vec3 {
        let mut velocity = velocity;
        let mut wish = wish;
        if let Some(normal) = ground_normal {
            velocity = ground_velocity(velocity, normal);
            wish = ground_velocity(wish, normal);
            let speed = velocity.xz().length();
            velocity = if speed < 0.0254 {
                Vec3::ZERO
            } else {
                velocity
                    * ((speed - speed.max(self.stop_speed) * self.friction * dt).max(0.) / speed)
            };
        }
        let speed = wish.length();
        if speed > 0. {
            let direction = wish / speed;
            let acceleration = if ground_normal.is_none() {
                self.air_acceleration
            } else if prone {
                self.prone_acceleration
            } else if crouched {
                self.crouch_acceleration
            } else {
                self.ground_acceleration
            };
            let addition = (speed - velocity.dot(direction)).max(0.);
            velocity += direction * addition.min(acceleration * speed.max(self.stop_speed) * dt);
        }
        velocity
    }

    pub fn slide_config(&self) -> MoveAndSlideConfig {
        MoveAndSlideConfig {
            skin_width: self.skin_width,
            plane_similarity_dot_threshold: 0.999,
            ..default()
        }
    }

    pub fn step_scale(&self, adjustment: f32, height: f32, step_height: f32) -> f32 {
        if adjustment.abs() <= self.step_slowdown_height {
            1.
        } else {
            (1. - height.abs() / step_height).max(0.) * (1. - self.step_speed_min)
                + self.step_speed_min
        }
    }
}

pub fn ground_velocity(velocity: Vec3, normal: Vec3) -> Vec3 {
    if velocity.xz().length_squared() == 0. {
        return velocity;
    }
    let tangent = velocity.with_y(-velocity.xz().dot(normal.xz()) / normal.y);
    let scale = velocity.length() / tangent.length();
    if scale < 1. || tangent.y < 0. || velocity.y > 0. {
        tangent * scale
    } else {
        velocity
    }
}

pub fn walkable_surface(
    query: &SpatialQuery,
    point: Vec3,
    normal: Vec3,
    filter: &SpatialQueryFilter,
    tuning: &PhysicsTuning,
) -> Option<(Vec3, f32)> {
    if normal.y >= tuning.ground_normal_min {
        return Some((normal, point.y));
    }
    if normal.y <= 0. {
        return None;
    }
    let origin = point - normal.with_y(0.).normalize_or_zero() * tuning.skin_width
        + Vec3::Y * tuning.ground_probe;
    let surface = query.cast_ray(origin, Dir3::NEG_Y, tuning.ground_probe * 2., false, filter)?;
    let height = origin.y - surface.distance;
    (surface.normal.y >= tuning.ground_normal_min
        && (height - point.y).abs() <= tuning.ground_probe)
        .then_some((surface.normal, height))
}

pub fn move_and_slide(
    slide: &MoveAndSlide,
    query: &SpatialQuery,
    motor: crate::player::MotionShape<'_>,
    start: Vec3,
    velocity: Vec3,
    delta: std::time::Duration,
    filter: &SpatialQueryFilter,
    tuning: &PhysicsTuning,
) -> MotionOutput {
    let mut config = tuning.slide_config();
    config.skin_width = config.skin_width.min(motor.radius * 0.1);
    let mut result = slide.move_and_slide(
        motor.collider,
        start,
        Quat::IDENTITY,
        velocity,
        delta,
        &config,
        filter,
        |hit| {
            if motor.grounded
                && hit.normal.y > 0.
                && hit.normal.y < tuning.ground_normal_min
                && let Ok(wall) = Dir3::new(hit.normal.as_vec3().with_y(0.))
            {
                *hit.normal = wall;
            }
            MoveAndSlideHitResponse::Accept
        },
    );
    let unstepped_height = result.position.y;
    let mut step_rise = None;
    let mut step_normal = None;
    let mut step_surface = None;
    let travel = velocity.xz() * delta.as_secs_f32();
    let actual = result.position.xz() - start.xz();
    if motor.grounded && travel.length_squared() > 0.000001 {
        if actual.dot(travel) + 0.000001 < travel.length_squared() {
            let up = slide.move_and_slide(
                motor.collider,
                start,
                Quat::IDENTITY,
                Vec3::Y * motor.step_height,
                std::time::Duration::from_secs(1),
                &config,
                filter,
                |_| MoveAndSlideHitResponse::Accept,
            );
            let rise = (up.position.y - start.y).clamp(0., motor.step_height);
            step_rise = Some(rise);
            if rise > config.skin_width {
                let mut stepped = slide.move_and_slide(
                    motor.collider,
                    start + Vec3::Y * rise,
                    Quat::IDENTITY,
                    velocity,
                    delta,
                    &config,
                    filter,
                    |_| MoveAndSlideHitResponse::Accept,
                );
                let down = query.cast_shape(
                    motor.collider,
                    stepped.position,
                    Quat::IDENTITY,
                    Dir3::NEG_Y,
                    &ShapeCastConfig::from_max_distance(rise + tuning.ground_probe),
                    filter,
                );
                step_normal = down.map(|hit| hit.normal1);
                step_surface = down.and_then(|hit| {
                    walkable_surface(query, hit.point1, hit.normal1, filter, tuning)
                });
                if let Some(hit) = down
                    && let Some((normal, height)) = step_surface
                    && height - motor.collider.aabb(start, Quat::IDENTITY, 0.).min.y
                        <= motor.step_height + config.skin_width
                    && (stepped.position.xz() - start.xz()).dot(travel)
                        > actual.dot(travel) + 0.000001
                {
                    stepped.position.y -= (hit.distance - config.skin_width).max(0.);
                    stepped.projected_velocity =
                        ground_velocity(stepped.projected_velocity, normal);
                    result = stepped;
                }
            }
        }
        if let Some(hit) = query.cast_shape(
            motor.collider,
            result.position,
            Quat::IDENTITY,
            Dir3::NEG_Y,
            &ShapeCastConfig::from_max_distance(tuning.step_down),
            filter,
        ) && let Some((normal, _)) =
            walkable_surface(query, hit.point1, hit.normal1, filter, tuning)
        {
            result.position.y -= (hit.distance - config.skin_width).max(0.);
            result.projected_velocity = ground_velocity(result.projected_velocity, normal);
        }
    }
    MotionOutput {
        step_adjustment: result.position.y - unstepped_height,
        step_rise,
        step_normal,
        step_surface,
        slide: result,
    }
}

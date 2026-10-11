use bevy::prelude::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HandAction {
    Crafting,
    Station,
    Portal,
}

#[derive(Component)]
pub struct HandReservation {
    pub action: HandAction,
    remaining: Option<f32>,
}

impl HandReservation {
    pub fn held(action: HandAction) -> Self {
        Self {
            action,
            remaining: None,
        }
    }

    pub fn timed(action: HandAction, seconds: f32) -> Self {
        Self {
            action,
            remaining: Some(seconds),
        }
    }
}

#[derive(Message)]
pub(crate) struct Rumble {
    pub origin: Vec3,
    pub radius: f32,
    pub seconds: f32,
    pub strength: f32,
}
#[derive(Component)]
struct CameraImpulse {
    remaining: f32,
    seconds: f32,
    strength: f32,
}
pub struct ActionPlugin;

impl Plugin for ActionPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<Rumble>()
            .add_systems(
                FixedUpdate,
                feedback
                    .before(crate::gameplay::GameStep::Movement)
                    .run_if(in_state(crate::GameState::Playing)),
            )
            .add_systems(
                FixedUpdate,
                tick.before(crate::gameplay::GameStep::Movement)
                    .run_if(in_state(crate::GameState::Playing)),
            );
    }
}

fn tick(
    mut commands: Commands,
    time: Res<Time<Fixed>>,
    mut reservations: Query<(Entity, &crate::life::Vitality, &mut HandReservation)>,
) {
    for (entity, health, mut reservation) in &mut reservations {
        let expired = reservation.remaining.as_mut().is_some_and(|remaining| {
            *remaining -= time.delta_secs();
            *remaining <= 0.
        });
        if expired || health.state() != crate::life::LifeState::Standing {
            commands.entity(entity).remove::<HandReservation>();
        }
    }
}

fn feedback(
    mut commands: Commands,
    time: Res<Time<Fixed>>,
    mut events: MessageReader<Rumble>,
    mut players: Query<
        (
            Entity,
            &Transform,
            &mut crate::movement::MotionFeedback,
            Option<&mut CameraImpulse>,
        ),
        With<crate::player::Player>,
    >,
) {
    let events: Vec<_> = events.read().collect();
    for (entity, pose, mut feedback, impulse) in &mut players {
        feedback.rumble = Vec2::ZERO;
        if let Some(mut impulse) = impulse {
            impulse.remaining = (impulse.remaining - time.delta_secs()).max(0.);
            let elapsed = impulse.seconds - impulse.remaining;
            let strength = impulse.strength * (impulse.remaining / impulse.seconds);
            feedback.rumble =
                Vec2::new((elapsed * 71.).sin(), (elapsed * 91.).sin()) * strength * 0.006;
            if impulse.remaining == 0. {
                commands.entity(entity).remove::<CameraImpulse>();
            }
        }
        for event in &events {
            let distance = pose.translation.distance(event.origin);
            if distance < event.radius && event.seconds > 0. {
                commands.entity(entity).insert(CameraImpulse {
                    remaining: event.seconds,
                    seconds: event.seconds,
                    strength: event.strength * (1. - distance / event.radius),
                });
            }
        }
    }
}

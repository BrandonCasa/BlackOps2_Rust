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

pub struct ActionPlugin;

impl Plugin for ActionPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
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

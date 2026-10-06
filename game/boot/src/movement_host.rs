//! Assembly-facing explicit input consumption. Fixed-rate scheduling is host policy,
//! not evidence of a native command duration.
use crate::movement_input::{OrderedHostInput, PendingMovementInput};
use bevy::prelude::*;

#[derive(Resource, Default)]
pub struct ConsumedMovementInput(pub Vec<OrderedHostInput>);

/// Run immediately before command translation, once per requested command batch.
/// A zero-fixed-update render frame never runs this system or removes raw input.
pub fn consume_movement_input(
    mut pending: ResMut<PendingMovementInput>,
    mut consumed: ResMut<ConsumedMovementInput>,
) {
    consumed.0.extend(pending.drain());
}

pub struct MovementHostPlugin;
impl Plugin for MovementHostPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(crate::movement_input::MovementInputPlugin)
            .init_resource::<ConsumedMovementInput>();
        // Command owner explicitly schedules consume_movement_input. Avoid choosing
        // an original timing policy or draining events on every render update.
    }
}

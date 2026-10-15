use bevy::prelude::*;

/// Original ScriptVector0..3 values owned by the gameplay actor or rig.
/// The native renderer reads these exact slots through the surface's owner.
#[derive(Component, Default)]
pub(crate) struct CueParameters {
    pub script_vectors: [[f32; 4]; 4],
    pub script_available: u8,
    // Retained producer metadata; shader programs select their native slots.
    pub luminous_channel: Option<(usize, usize)>,
}

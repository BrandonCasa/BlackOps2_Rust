//! Shared admission limits for retained renderer work. Effects consume these
//! bounds through ECS resources rather than independently choosing frame rates.
use bevy::prelude::Resource;
use std::time::Duration;

#[derive(Resource)]
pub(super) struct RenderBudget {
    /// Desired maximum age; admit a refresh on the next available frame.
    pub reflection_refresh_interval: Duration,
    /// Admit a fresh capture when the camera exceeds this anchor displacement.
    pub reflection_anchor_distance: f32,
}

impl Default for RenderBudget {
    fn default() -> Self {
        Self {
            reflection_refresh_interval: Duration::from_millis(100),
            reflection_anchor_distance: 0.25,
        }
    }
}

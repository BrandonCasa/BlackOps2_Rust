//! Native, current-frame post processing after world/viewmodel compositing.
use super::profile::VisualProfile;
use bevy::{
    anti_alias::smaa::{Smaa, SmaaPreset},
    post_process::bloom::Bloom,
};

pub(crate) fn bloom(profile: &VisualProfile) -> Bloom {
    Bloom {
        intensity: profile.bloom_intensity,
        // NATURAL's 0.7 low-frequency boost spreads lantern/sky energy over
        // much of the frame. Keep their glow while preserving local contrast
        // in masonry and weapon detail. Retain energy conservation and the
        // unthresholded HDR input, so exposure cannot toggle a hard glow edge.
        low_frequency_boost: 0.2,
        low_frequency_boost_curvature: 0.5,
        high_pass_frequency: 0.75,
        ..Bloom::NATURAL
    }
}

pub(crate) fn smaa() -> Smaa {
    Smaa {
        // Extend long-edge searches without temporal accumulation or filtering
        // texture interiors. The native diagonal/corner handling stays active.
        preset: SmaaPreset::Ultra,
    }
}

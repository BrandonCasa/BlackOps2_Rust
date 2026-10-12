use super::profile::VisualProfile;
use bevy::post_process::bloom::Bloom;

pub(crate) fn bloom(profile: &VisualProfile) -> Bloom {
    Bloom {
        intensity: profile.bloom_intensity,
        low_frequency_boost: 0.2,
        low_frequency_boost_curvature: 0.5,
        high_pass_frequency: 0.75,
        ..Bloom::NATURAL
    }
}

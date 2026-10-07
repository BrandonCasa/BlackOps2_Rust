//! Inspect sinks and lifecycle in the actual game; receipts stay outside the pack.
use super::*;
use bevy::audio::AudioSinkPlayback;
#[derive(Resource, Default)]
struct Coverage {
    phase: u8,
    age: f32,
    paused: bool,
    resumed: bool,
    settings: bool,
}
pub fn install(app: &mut App) {
    app.init_resource::<Coverage>()
        .add_systems(Update, exercise.after(crate::audio::AudioPlayback))
        .add_systems(Last, record);
}
fn exercise(
    verification: Option<Res<Verification>>,
    time: Res<Time>,
    state: Res<State<GameState>>,
    mut next: ResMut<NextState<GameState>>,
    mut coverage: ResMut<Coverage>,
    mut settings: ResMut<crate::audio::AudioSettings>,
    spatial: Query<&SpatialAudioSink>,
    regular: Query<&AudioSink>,
    evidence: Res<crate::audio::AudioEvidence>,
) {
    let Some(v) = verification else {
        return;
    };
    if !v.barriers {
        return;
    }
    coverage.age += time.delta_secs();
    match coverage.phase {
        0 if **state == GameState::Playing
            && coverage.age > 1.
            && evidence.started > 0
            && !regular.is_empty() =>
        {
            next.set(GameState::Paused);
            coverage.phase = 1;
            coverage.age = 0.;
        }
        1 if **state == GameState::Paused
            && coverage.age > 0.2
            && !regular.is_empty()
            && spatial.iter().all(|s| s.is_paused())
            && regular.iter().all(|s| s.is_paused()) =>
        {
            coverage.paused = spatial.iter().all(|s| s.is_paused())
                && regular.iter().all(|s| s.is_paused())
                && !regular.is_empty();
            settings.music = 0.35;
            settings.effects = 0.5;
            settings.subtitles = false;
            coverage.settings = true;
            next.set(GameState::Playing);
            coverage.phase = 2;
            coverage.age = 0.;
        }
        2 if **state == GameState::Playing
            && coverage.age > 0.2
            && !regular.is_empty()
            && spatial.iter().all(|s| !s.is_paused())
            && regular.iter().all(|s| !s.is_paused()) =>
        {
            coverage.resumed =
                spatial.iter().all(|s| !s.is_paused()) && regular.iter().all(|s| !s.is_paused());
            *settings = default();
            coverage.phase = 3;
        }
        _ => {}
    }
}
fn record(
    options: Res<LaunchOptions>,
    mut exits: MessageReader<AppExit>,
    evidence: Res<crate::audio::AudioEvidence>,
    coverage: Res<Coverage>,
    spatial: Query<&SpatialAudioSink>,
    regular: Query<&AudioSink>,
) {
    if exits.read().next().is_none() {
        return;
    }
    let Some(output) = &options.verify else {
        return;
    };
    let lifecycle = if options.verify_barriers {
        coverage.paused && coverage.resumed && coverage.settings
    } else {
        true
    };
    let passed =
        evidence.started > 0 && !evidence.sinks.is_empty() && evidence.peak <= 64 && lifecycle;
    let receipt = serde_json::json!({"passed":passed,"requested":evidence.requested,"started":evidence.started,"rejected":evidence.rejected,"peak":evidence.peak,"paused":evidence.paused,"resumed":evidence.resumed,"owner_stops":evidence.owner_stops,"events":evidence.events,"sinks":evidence.sinks,"remaining_sinks":regular.iter().count()+spatial.iter().count(),"pause_exercised":coverage.paused,"resume_exercised":coverage.resumed,"settings_exercised":coverage.settings,"speech_transcripts_available":false});
    let _ = std::fs::write(output.join("audio.json"), receipt.to_string());
}

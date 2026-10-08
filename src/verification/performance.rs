//! Bounded actual-window measurements; loading, view changes and captures are excluded.
use super::Verification;
use crate::assets::LaunchOptions;
use bevy::{diagnostic::DiagnosticsStore, prelude::*};
use std::collections::BTreeMap;

#[derive(Default)]
pub(super) struct Samples {
    frames: BTreeMap<usize, Vec<f64>>,
    passes: BTreeMap<String, Vec<f64>>,
    last: BTreeMap<String, bevy::platform::time::Instant>,
    written: bool,
    view: Option<usize>,
    shadow_lights: BTreeMap<usize, (usize, usize)>,
    combat: Vec<f64>,
    load_seconds: Option<f64>,
}

pub(super) fn record(
    verification: Option<Res<Verification>>,
    options: Res<LaunchOptions>,
    time: Res<Time<Real>>,
    diagnostics: Res<DiagnosticsStore>,
    windows: Query<&Window>,
    mut samples: Local<Samples>,
    adapter: Option<Res<bevy::render::renderer::RenderAdapterInfo>>,
    lights: Query<&PointLight>,
    shadows: Res<crate::rendering::ShadowCacheStats>,
    state: Res<State<crate::GameState>>,
    profile: Res<crate::rendering::profile::VisualProfile>,
    preprocessing: Res<crate::rendering::PreprocessingStatus>,
    images: Res<Assets<Image>>,
    generators: Option<Res<super::generators::Coverage>>,
    shared_lighting: (
        Res<crate::rendering::StaticGiStats>,
        Res<crate::rendering::RayContributorStats>,
        Res<crate::rendering::RaySurfaceStats>,
        Res<crate::audio::AudioDecodeStats>,
    ),
) {
    let (shared_gi, contributors, ray_feedback, audio_decode) = shared_lighting;
    let Some(verification) = verification else {
        return;
    };
    if samples.load_seconds.is_none() && **state == crate::GameState::Playing {
        samples.load_seconds = Some(verification.started.elapsed().as_secs_f64());
    }
    if (options.verify_generators
        || (!options.benchmark || options.benchmark_combat)
            && verification.stage == 0
            && verification.playing >= 2.)
        && **state == crate::GameState::Playing
    {
        let _zone = bevy::log::info_span!("benchmark_sample", view = 15).entered();
        samples.combat.push(time.delta_secs_f64() * 1000.);
    }
    if samples.view != Some(verification.survey) {
        if let Some(view) = samples.view
            && let Some(frames) = samples.frames.get(&view)
        {
            info!("Benchmark view {view}: {}", summary(frames, true));
        }
        samples.view = Some(verification.survey);
    }
    let sample_end = if verification.moving_view { 60. } else { 12. };
    if options.benchmark
        && verification.stage == 1
        && (4.0..sample_end).contains(&verification.playing)
    {
        let _zone = bevy::log::info_span!("benchmark_sample", view = verification.survey).entered();
        let frames = samples.frames.entry(verification.survey).or_default();
        if frames.is_empty() {
            info!("Benchmark sampling ready: view {}", verification.survey);
        }
        frames.push(time.delta_secs_f64() * 1000.);
        let count = lights
            .iter()
            .filter(|light| light.shadow_maps_enabled)
            .count();
        samples
            .shadow_lights
            .entry(verification.survey)
            .and_modify(|range| {
                range.0 = range.0.min(count);
                range.1 = range.1.max(count);
            })
            .or_insert((count, count));
        for diagnostic in diagnostics.iter() {
            let Some(measurement) = diagnostic.measurement() else {
                continue;
            };
            let path = diagnostic.path().to_string();
            if !(path.ends_with("elapsed_gpu") || path.ends_with("elapsed_cpu")) {
                continue;
            }
            if samples.last.get(&path) == Some(&measurement.time) {
                continue;
            }
            samples.last.insert(path.clone(), measurement.time);
            samples
                .passes
                .entry(format!("view-{}/{}", verification.survey, path))
                .or_default()
                .push(measurement.value);
        }
    }
    if (matches!(verification.stage, 2 | 3)
        || generators
            .as_ref()
            .is_some_and(|coverage| coverage.finished()))
        && !samples.written
    {
        let views: BTreeMap<_, _> = samples
            .frames
            .iter()
            .map(|(view, values)| (*view, summary(values, true)))
            .collect();
        let passes: BTreeMap<_, _> = samples
            .passes
            .iter()
            .map(|(name, values)| (name, summary(values, false)))
            .collect();
        let report = serde_json::json!({
            "physical_resolution": windows.single().ok().map(|window| [window.resolution.physical_width(), window.resolution.physical_height()]),
            "logical_resolution": windows.single().ok().map(|window| [window.resolution.width(), window.resolution.height()]),
            "adapter": adapter.as_ref().map(|info| format!("{:?}", info.0)),
            "vsync": !options.benchmark, "sample_virtual_seconds_per_view": [4.,sample_end],
            "continuous_window_updates": options.benchmark,
            "winit_continuous": options.verify.is_some(),
            "initial_load_wall_seconds": samples.load_seconds,
            "loaded_image_cpu_payload_bytes": images.iter().filter_map(|(_, image)| image.data.as_ref()).map(Vec::len).sum::<usize>(),
            "antialiasing": profile.antialiasing,
            "lantern_shadow_map_size": profile.lantern_shadow_map_size,
            "continuous_camera_motion": verification.moving_view,
            "profile_tracy": cfg!(feature="profile-tracy"), "profile_chrome": cfg!(feature="profile-chrome"),
            "build_profile": if cfg!(debug_assertions) { "debug" } else { "release" },
            "logging_enabled": cfg!(debug_assertions),
            "workload": if options.verify_generators { "generator lifecycle including restart" } else if options.benchmark_combat { "combat" } else { "render survey" },
            "views": views, "render_passes": passes, "frame_ms": samples.frames,
            "shadowed_point_lights_min_max": samples.shadow_lights,
            "shadow_cache_enabled": !options.uncached_shadows,
            "shadow_cache": shadows.report(),
            "shared_lighting_scene_refreshes": shared_gi.scene_refreshes(),
            "shared_lighting": { "ddgi_batches": shared_gi.updates(), "reflection_frames": shared_gi.reflections() },
            "ray_contributors": &*contributors,
            "ray_feedback": &*ray_feedback,
            "audio_decode": &*audio_decode,
            "mesh_preprocessing": preprocessing.report(),
            "combat": if samples.combat.is_empty() { serde_json::Value::Null } else { summary(&samples.combat, true) },
        });
        match std::fs::write(
            verification.output.join("performance.json"),
            report.to_string(),
        ) {
            Ok(()) => {
                samples.written = true;
                info!("Benchmark frame summary: {views:?}");
            }
            Err(error) => error!("Cannot write performance receipt: {error}"),
        }
    }
}

pub(super) fn summary(values: &[f64], frames: bool) -> serde_json::Value {
    let mut sorted = values.to_vec();
    sorted.sort_by(f64::total_cmp);
    let mean = values.iter().sum::<f64>() / values.len() as f64;
    let percentile =
        |fraction: f64| sorted[((sorted.len() - 1) as f64 * fraction).round() as usize];
    serde_json::json!({"samples": values.len(), "mean_ms": mean, "median_ms": percentile(0.5),
        "sample_elapsed_seconds": if frames { Some(values.iter().sum::<f64>()/1000.) } else { None },
        "p95_ms": percentile(0.95), "p99_ms": percentile(0.99), "max_ms": sorted.last(),
        "fps": if frames { Some(1000./mean) } else { None }})
}

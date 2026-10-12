use super::Verification;
use crate::assets::LaunchOptions;
use bevy::prelude::*;
use std::collections::BTreeMap;

#[derive(Default)]
pub(super) struct Samples {
    frames: BTreeMap<usize, Vec<f64>>,
    written: bool,
    view: Option<usize>,
    shadow_lights: BTreeMap<usize, (usize, usize)>,
    combat: Vec<f64>,
    combat_shadow_lights: Option<(usize, usize)>,
    load_seconds: Option<f64>,
}

pub(super) fn record(
    verification: Option<Res<Verification>>,
    options: Res<LaunchOptions>,
    time: Res<Time<Real>>,
    windows: Query<&Window>,
    mut samples: Local<Samples>,
    adapter: Option<Res<bevy::render::renderer::RenderAdapterInfo>>,
    lights: Query<&PointLight>,
    state: Res<State<crate::GameState>>,
    profile: Res<crate::rendering::profile::VisualProfile>,
    preprocessing: Res<crate::rendering::PreprocessingStatus>,
    images: Res<Assets<Image>>,
    generators: Option<Res<super::generators::Coverage>>,
    environment: Res<crate::run_environment::RunEnvironment>,
    audio_decode: Res<crate::audio::AudioDecodeStats>,
) {
    if options.test_combat {
        return;
    }
    let Some(verification) = verification else {
        return;
    };
    let sample_end = if verification.moving_view { 60. } else { 12. };
    let admitted = options.benchmark
        && verification.stage == 1
        && (4.0..sample_end).contains(&verification.playing);
    let combat_admitted = (options.verify_generators
        || (!options.benchmark || options.benchmark_combat)
            && verification.stage == 0
            && verification.playing >= 2.)
        && **state == crate::GameState::Playing;
    if samples.load_seconds.is_none() && **state == crate::GameState::Playing {
        samples.load_seconds = Some(verification.started.elapsed().as_secs_f64());
    }
    if combat_admitted {
        let _zone = bevy::log::info_span!("benchmark_sample", view = 15).entered();
        samples.combat.push(time.delta_secs_f64() * 1000.);
        let count = lights
            .iter()
            .filter(|light| light.shadow_maps_enabled)
            .count();
        let range = samples.combat_shadow_lights.get_or_insert((count, count));
        range.0 = range.0.min(count);
        range.1 = range.1.max(count);
    }
    if samples.view != Some(verification.survey) {
        if let Some(view) = samples.view
            && let Some(frames) = samples.frames.get(&view)
        {
            info!("Benchmark view {view}: {}", summary(frames, true));
        }
        samples.view = Some(verification.survey);
    }
    if admitted {
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
        let report = serde_json::json!({
            "physical_resolution": windows.single().ok().map(|window| [window.resolution.physical_width(), window.resolution.physical_height()]),
            "logical_resolution": windows.single().ok().map(|window| [window.resolution.width(), window.resolution.height()]),
            "adapter": adapter.as_ref().map(|info| format!("{:?}", &***info)),
            "vsync": !options.benchmark, "sample_virtual_seconds_per_view": [4.,sample_end],
            "continuous_window_updates": environment.continuous_window_updates,
            "winit_continuous": environment.continuous_window_updates,
            "run_environment": &*environment,
            "initial_load_wall_seconds": samples.load_seconds,
            "loaded_image_cpu_payload_bytes": images.iter().filter_map(|(_, image)| image.data.as_ref()).map(Vec::len).sum::<usize>(),
            "antialiasing": profile.antialiasing,
            "lantern_shadow_map_size": profile.lantern_shadow_map_size,
            "continuous_camera_motion": verification.moving_view,
            "profile_tracy": cfg!(feature="profile-tracy"), "profile_chrome": cfg!(feature="profile-chrome"),
            "build_profile": if cfg!(debug_assertions) { "debug" } else { "release" },
            "logging_enabled": cfg!(debug_assertions),
            "workload": if options.verify_generators { "generator lifecycle including restart" } else if options.benchmark_combat { "combat" } else { "render survey" },
            "views": views, "frame_ms": samples.frames,
            "timing_scope": "Application frame intervals; no CPU/GPU pass attribution",
            "shadowed_point_lights_min_max": samples.shadow_lights,
            "combat_shadowed_point_lights_min_max": samples.combat_shadow_lights,
            "shared_lighting": { "method": "solari", "ray_tracing": true, "local_light_limit": profile.max_local_lights },
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
    if values.is_empty() {
        return serde_json::Value::Null;
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(f64::total_cmp);
    let mean = values.iter().sum::<f64>() / values.len() as f64;
    let percentile =
        |fraction: f64| sorted[((sorted.len() - 1) as f64 * fraction).round() as usize];
    serde_json::json!({"samples": values.len(), "mean_ms": mean, "median_ms": percentile(0.5),
        "sample_elapsed_seconds": if frames { Some(values.iter().sum::<f64>()/1000.) } else { None },
        "p95_ms": percentile(0.95), "p99_ms": percentile(0.99), "max_ms": sorted.last(),
        "fps": if frames { Some(1000./mean) } else { None },
        "frames_over_10ms": if frames { Some(values.iter().filter(|value| **value > 10.).count()) } else { None }})
}

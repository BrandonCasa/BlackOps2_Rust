//! Bounded actual-window measurements; loading, view changes and captures are excluded.
use super::Verification;
use crate::assets::LaunchOptions;
use bevy::{
    diagnostic::FrameCount,
    prelude::*,
    render::diagnostic::{
        RenderDiagnosticFrame, RenderDiagnosticsFrameContext, RenderDiagnosticsFrames,
    },
};
use std::collections::BTreeMap;

// Raw records are bounded independently of the recorder's readback queues. Keep
// complete frame totals after this limit, while explicitly reporting omitted raw
// invocations. Serializing receipts occurs only after the sampling interval.
const MAX_RAW_RENDER_SPANS: usize = 262_144;
const MAX_RAW_RENDER_FRAMES: usize = 4096;
const COMBAT_RENDER_WORKLOAD: u64 = u64::MAX;

#[derive(Default)]
pub(super) struct Samples {
    frames: BTreeMap<usize, Vec<f64>>,
    passes: BTreeMap<String, Vec<f64>>,
    render_frames: Vec<RenderDiagnosticFrame>,
    raw_spans: usize,
    omitted_raw_frames: u64,
    dropped_render_frames: u64,
    admitted_render_frames: BTreeMap<u64, u64>,
    received_render_frames: BTreeMap<u64, u64>,
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
    mut diagnostics: (
        Option<ResMut<RenderDiagnosticsFrames>>,
        Option<ResMut<RenderDiagnosticsFrameContext>>,
        Res<FrameCount>,
    ),
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
    environment: Res<crate::run_environment::RunEnvironment>,
    shared_lighting: (
        Res<crate::rendering::StaticGiStats>,
        Res<crate::rendering::RayContributorStats>,
        Res<crate::rendering::RaySurfaceStats>,
        Res<crate::audio::AudioDecodeStats>,
        Res<crate::rendering::FirstPersonDepthStats>,
        Res<bevy::light::ShadowVisibilityWorkCounts>,
    ),
) {
    let (
        shared_gi,
        contributors,
        ray_feedback,
        audio_decode,
        first_person_depth,
        shadow_visibility,
    ) = shared_lighting;
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
    let workload = if admitted {
        Some(verification.survey as u64)
    } else if combat_admitted {
        Some(COMBAT_RENDER_WORKLOAD)
    } else {
        None
    };
    if let Some(context) = diagnostics.1.as_deref_mut() {
        context.source_frame = u64::from(diagnostics.2.0);
        context.workload = workload;
        if let Some(view) = workload {
            *samples.admitted_render_frames.entry(view).or_default() += 1;
        }
    }
    if let Some(completed) = diagnostics.0.as_deref_mut() {
        samples.dropped_render_frames = completed.dropped_frames;
        for frame in completed.drain() {
            collect_render_frame(&mut samples, frame);
        }
    }
    if samples.load_seconds.is_none() && **state == crate::GameState::Playing {
        samples.load_seconds = Some(verification.started.elapsed().as_secs_f64());
    }
    if combat_admitted {
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
        let passes: BTreeMap<_, _> = samples
            .passes
            .iter()
            .map(|(name, values)| (name, summary(values, false)))
            .collect();
        let mut report = serde_json::json!({
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
            "views": views, "render_passes": passes, "frame_ms": samples.frames,
            "render_attribution": {
                "instrumented": diagnostics.0.is_some(),
                "admitted_frames": samples.admitted_render_frames.iter().map(|(&workload, &count)| (render_workload_label(workload), count)).collect::<BTreeMap<_,_>>(),
                "received_frames": samples.received_render_frames.iter().map(|(&workload, &count)| (render_workload_label(workload), count)).collect::<BTreeMap<_,_>>(),
                "dropped_completed_frames": samples.dropped_render_frames,
                "omitted_raw_frames": samples.omitted_raw_frames,
                "raw_span_limit": MAX_RAW_RENDER_SPANS,
                "scope": "Recorded pass intervals and queue windows; not whole CPU/GPU frame coverage. Parent timings are inclusive. Compaction queue windows include intervening uploads and idle/submission gaps. Missing intervals are absent evidence.",
                "frames": samples.render_frames.iter().map(render_frame_report).collect::<Vec<_>>(),
            },
            "shadowed_point_lights_min_max": samples.shadow_lights,
            "shadow_cache_enabled": !options.uncached_shadows,
            "shadow_cache": shadows.report(),
            "shared_lighting_scene_refreshes": shared_gi.scene_refreshes(),
            "shared_lighting": { "ddgi_enabled": cfg!(feature = "ddgi"), "ddgi_batches": shared_gi.updates(), "reflection_frames": shared_gi.reflections() },
            "ray_contributors": &*contributors,
            "ray_feedback": &*ray_feedback,
            "audio_decode": &*audio_decode,
            "mesh_preprocessing": preprocessing.report(),
            "first_person_depth": first_person_depth.report(),
            "combat": if samples.combat.is_empty() { serde_json::Value::Null } else { summary(&samples.combat, true) },
        });
        report["shadow_visibility"] = serde_json::json!({
            "enabled": shadow_visibility.enabled,
            "tracked_casters": shadow_visibility.tracked_casters,
            "global_casters": shadow_visibility.global_casters,
            "occupied_cells": shadow_visibility.occupied_cells,
            "membership_changes": shadow_visibility.membership_changes,
            "candidate_queries": shadow_visibility.candidate_queries,
            "candidate_casters": shadow_visibility.candidate_casters,
            "exhaustive_queries": shadow_visibility.exhaustive_queries,
            "exhaustive_casters": shadow_visibility.exhaustive_casters,
            "possible_caster_visits": shadow_visibility.possible_caster_visits,
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

fn collect_render_frame(samples: &mut Samples, frame: RenderDiagnosticFrame) {
    let Some(workload) = frame.context.workload else {
        return;
    };
    *samples.received_render_frames.entry(workload).or_default() += 1;
    let label = render_workload_label(workload);
    for diagnostic in &frame.diagnostics {
        let path = diagnostic.path.to_string();
        if path.ends_with("elapsed_gpu") || path.ends_with("elapsed_cpu") {
            samples
                .passes
                .entry(format!("{label}/{path}"))
                .or_default()
                .push(diagnostic.value);
        }
    }
    if samples.render_frames.len() < MAX_RAW_RENDER_FRAMES
        && frame.spans.len() <= MAX_RAW_RENDER_SPANS.saturating_sub(samples.raw_spans)
    {
        samples.raw_spans += frame.spans.len();
        samples.render_frames.push(frame);
    } else {
        samples.omitted_raw_frames += 1;
    }
}

fn render_workload_label(workload: u64) -> String {
    if workload == COMBAT_RENDER_WORKLOAD {
        "combat".into()
    } else {
        format!("view-{workload}")
    }
}

fn render_frame_report(frame: &RenderDiagnosticFrame) -> serde_json::Value {
    let coverage = &frame.coverage;
    serde_json::json!({
        "frame_id": frame.frame_id,
        "source_frame": frame.context.source_frame,
        "workload": frame.context.workload,
        "workload_label": frame.context.workload.map(render_workload_label),
        "recording_wall_ms": frame.end.duration_since(frame.start).as_secs_f64() * 1000.,
        "coverage": {
            "spans": coverage.spans,
            "gpu_spans": coverage.gpu_spans,
            "unsupported_spans": coverage.unsupported_spans,
            "capacity_exhausted_spans": coverage.capacity_exhausted_spans,
            "readback_failed_spans": coverage.readback_failed_spans,
            "readback_backpressure_spans": coverage.readback_backpressure_spans,
            "incomplete_spans": coverage.incomplete_spans,
            "timestamp_queries_used": coverage.timestamp_queries_used,
            "timestamp_query_capacity": coverage.timestamp_query_capacity,
            "pipeline_statistics_queries_used": coverage.pipeline_statistics_queries_used,
            "pipeline_statistics_query_capacity": coverage.pipeline_statistics_query_capacity,
            "pipeline_statistics_capacity_exhausted": coverage.pipeline_statistics_capacity_exhausted,
            "value_readback_failures": coverage.value_readback_failures,
            "value_readback_backpressure": coverage.value_readback_backpressure,
        },
        "recorded_values": frame.values.iter().map(|value| serde_json::json!({
            "path": value.path.to_string(),
            "suffix": value.suffix,
            "value": value.value,
        })).collect::<Vec<_>>(),
        "invocations": frame.spans.iter().map(|span| serde_json::json!({
            "invocation_id": span.invocation_id,
            "parent_invocation_id": span.parent_invocation_id,
            "thread": format!("{:?}", span.thread_id),
            "path": span.path.to_string(),
            "view": span.context.view.map(|entity| entity.to_bits()),
            "subview": span.context.subview,
            "submission_scope": span.context.submission,
            "cpu_interval_ns": span.cpu_interval.map(|interval| [interval.start_ns, interval.end_ns]),
            "gpu_interval_ns": span.gpu_interval.map(|interval| [interval.start_ns, interval.end_ns]),
            "cpu_exclusive_ms": frame.exclusive_cpu_ms(span),
            "gpu_exclusive_ms": frame.exclusive_gpu_ms(span),
            "gpu_status": format!("{:?}", span.gpu_status),
        })).collect::<Vec<_>>(),
    })
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

#[cfg(test)]
mod rendering_tests {
    use super::*;

    #[test]
    fn delayed_render_frames_keep_their_original_workload() {
        let now = bevy::platform::time::Instant::now();
        let mut samples = Samples {
            view: Some(9),
            ..Default::default()
        };
        collect_render_frame(
            &mut samples,
            RenderDiagnosticFrame {
                frame_id: 12,
                context: RenderDiagnosticsFrameContext {
                    source_frame: 42,
                    workload: Some(8),
                },
                start: now,
                end: now,
                spans: Vec::new(),
                values: Vec::new(),
                diagnostics: Vec::new(),
                coverage: Default::default(),
            },
        );
        assert_eq!(samples.received_render_frames.get(&8), Some(&1));
        assert!(!samples.received_render_frames.contains_key(&9));
        assert_eq!(samples.render_frames[0].context.source_frame, 42);
    }

    #[test]
    fn frame_target_misses_remain_visible_despite_good_average() {
        let measured = summary(&[1., 1., 10., 11.], true);
        assert!(measured["fps"].as_f64().unwrap() > 100.);
        assert_eq!(measured["frames_over_10ms"], 1);
        assert_eq!(measured["max_ms"], 11.);
        assert!(summary(&[], true).is_null());
    }

    #[test]
    fn combat_and_survey_fifteen_have_distinct_render_workloads() {
        assert_ne!(
            render_workload_label(15),
            render_workload_label(COMBAT_RENDER_WORKLOAD)
        );
        assert_eq!(render_workload_label(COMBAT_RENDER_WORKLOAD), "combat");
    }
}

//! Completed, frame-associated render records. Invocation metadata is kept out of diagnostic paths.

use alloc::{collections::VecDeque, vec::Vec};
use bevy_diagnostic::DiagnosticPath;
use bevy_ecs::{entity::Entity, resource::Resource};
use bevy_platform::{collections::HashMap, time::Instant};
use std::thread::ThreadId;

use crate::extract_resource::ExtractResource;

use super::internal::PassKind;

/// Bounded recorder allocation settings, read when the render device is initialized.
///
/// Timestamp capacity is rounded down to a multiple of 32 for resolve-buffer alignment.
/// Multiple query sets are used when capacity exceeds wgpu's per-set limit. A zero
/// capacity explicitly disables that query kind; missing GPU data remains `None`.
#[derive(Debug, Clone, Resource)]
pub struct RenderDiagnosticsConfig {
    pub timestamp_queries: u32,
    pub pipeline_statistics_queries: u32,
    pub completed_frame_capacity: usize,
    pub max_in_flight_frames: usize,
}

impl Default for RenderDiagnosticsConfig {
    fn default() -> Self {
        Self {
            timestamp_queries: 8192,
            pipeline_statistics_queries: 128,
            completed_frame_capacity: 64,
            max_in_flight_frames: 8,
        }
    }
}

impl RenderDiagnosticsConfig {
    pub(crate) fn bounded(&self) -> Self {
        Self {
            timestamp_queries: self.timestamp_queries.min(65536) / 32 * 32,
            pipeline_statistics_queries: self
                .pipeline_statistics_queries
                .min(wgpu::QUERY_SET_MAX_QUERIES),
            completed_frame_capacity: self.completed_frame_capacity.clamp(1, 256),
            max_in_flight_frames: self.max_in_flight_frames.clamp(2, 32),
        }
    }
}

/// Main-world workload identity extracted before render preparation begins.
///
/// This is independent of the recorder ordinal and remains attached through delayed
/// GPU readback. Applications can register `ExtractResourcePlugin` for this resource.
#[derive(Debug, Clone, Copy, Default, Resource, ExtractResource)]
#[extract_app(crate::RenderApp)]
pub struct RenderDiagnosticsFrameContext {
    pub source_frame: u64,
    pub workload: Option<u64>,
}

/// Optional invocation identity; neither field creates a new diagnostics-store path.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DiagnosticSpanContext {
    pub view: Option<Entity>,
    pub subview: Option<u32>,
    pub submission: Option<u64>,
}

/// A measured interval in nanoseconds. CPU intervals are relative to frame start;
/// GPU intervals use the queue's timestamp clock and cannot be compared to CPU time.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RenderDiagnosticInterval {
    pub start_ns: f64,
    pub end_ns: f64,
}

impl RenderDiagnosticInterval {
    pub fn duration_ms(self) -> f64 {
        (self.end_ns - self.start_ns).max(0.0) / 1e6
    }
}

/// Why GPU timing is available or missing for a particular invocation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderDiagnosticGpuStatus {
    Available,
    Unsupported,
    CapacityExhausted,
    ReadbackFailed,
    ReadbackBackpressure,
    Incomplete,
}

/// One invocation before repeated paths are aggregated. IDs are local to `frame_id`.
#[derive(Debug, Clone)]
pub struct RenderDiagnosticSpan {
    pub invocation_id: u64,
    pub parent_invocation_id: Option<u64>,
    pub thread_id: ThreadId,
    pub path: DiagnosticPath,
    pub pass_kind: Option<PassKind>,
    pub context: DiagnosticSpanContext,
    pub cpu_interval: Option<RenderDiagnosticInterval>,
    pub gpu_interval: Option<RenderDiagnosticInterval>,
    pub gpu_status: RenderDiagnosticGpuStatus,
    pub cpu_exclusive_ms: Option<f64>,
    pub gpu_exclusive_ms: Option<f64>,
}

/// Query and completion coverage. Uninstrumented passes are not implied to be covered.
#[derive(Debug, Clone, Default)]
pub struct RenderDiagnosticCoverage {
    pub spans: u32,
    pub gpu_spans: u32,
    pub unsupported_spans: u32,
    pub capacity_exhausted_spans: u32,
    pub readback_failed_spans: u32,
    pub readback_backpressure_spans: u32,
    pub incomplete_spans: u32,
    pub timestamp_queries_used: u32,
    pub timestamp_query_capacity: u32,
    pub pipeline_statistics_queries_used: u32,
    pub pipeline_statistics_query_capacity: u32,
    pub pipeline_statistics_capacity_exhausted: u32,
    pub value_readback_failures: u32,
    pub value_readback_backpressure: u32,
}

/// A completed measurement, not yet stored in `DiagnosticsStore`.
#[derive(Debug, Clone, Resource)]
pub struct RenderDiagnostic {
    pub path: DiagnosticPath,
    pub suffix: &'static str,
    pub value: f64,
}

/// Completed raw frame plus stable per-path aggregates. The timestamps are original
/// recording timestamps rather than readback-completion timestamps.
#[derive(Debug, Clone)]
pub struct RenderDiagnosticFrame {
    pub frame_id: u64,
    pub context: RenderDiagnosticsFrameContext,
    pub start: Instant,
    pub end: Instant,
    pub spans: Vec<RenderDiagnosticSpan>,
    pub values: Vec<RenderDiagnostic>,
    pub diagnostics: Vec<RenderDiagnostic>,
    pub coverage: RenderDiagnosticCoverage,
}

impl RenderDiagnosticFrame {
    /// Exclusive measured CPU time after the union of direct children is removed.
    /// This is recording time on one thread, not total CPU frame preparation time.
    pub fn exclusive_cpu_ms(&self, span: &RenderDiagnosticSpan) -> Option<f64> {
        span.cpu_exclusive_ms
    }

    /// Exclusive measured GPU time. `None` means the parent or one of its direct
    /// children lacks a GPU interval; incomplete coverage must not look like zero work.
    pub fn exclusive_gpu_ms(&self, span: &RenderDiagnosticSpan) -> Option<f64> {
        span.gpu_exclusive_ms
    }
}

pub(crate) fn populate_exclusive_timings(frame: &mut RenderDiagnosticFrame) {
    struct Children {
        cpu: Vec<RenderDiagnosticInterval>,
        gpu: Vec<RenderDiagnosticInterval>,
        cpu_complete: bool,
        gpu_complete: bool,
    }
    let threads: HashMap<_, _> = frame
        .spans
        .iter()
        .map(|span| (span.invocation_id, span.thread_id))
        .collect();
    let mut children: HashMap<u64, Children> = HashMap::default();
    for span in &frame.spans {
        let Some(parent) = span.parent_invocation_id else {
            continue;
        };
        let children = children.entry(parent).or_insert_with(|| Children {
            cpu: Vec::new(),
            gpu: Vec::new(),
            cpu_complete: true,
            gpu_complete: true,
        });
        if threads.get(&parent) == Some(&span.thread_id) {
            if let Some(interval) = span.cpu_interval {
                children.cpu.push(interval);
            } else {
                children.cpu_complete = false;
            }
        }
        if let Some(interval) = span.gpu_interval {
            children.gpu.push(interval);
        } else {
            children.gpu_complete = false;
        }
    }
    for span in &mut frame.spans {
        if let Some(children) = children.get(&span.invocation_id) {
            span.cpu_exclusive_ms = span
                .cpu_interval
                .filter(|_| children.cpu_complete)
                .map(|interval| exclusive_interval_ms(interval, children.cpu.iter().copied()));
            span.gpu_exclusive_ms = span
                .gpu_interval
                .filter(|_| children.gpu_complete)
                .map(|interval| exclusive_interval_ms(interval, children.gpu.iter().copied()));
        } else {
            span.cpu_exclusive_ms = span.cpu_interval.map(RenderDiagnosticInterval::duration_ms);
            span.gpu_exclusive_ms = span.gpu_interval.map(RenderDiagnosticInterval::duration_ms);
        }
    }
}

/// Bounded main-world queue, ordered by recorder frame ID. Draining never waits for
/// an earlier in-flight frame. Overflow drops the oldest retained frame and is counted.
#[derive(Debug, Clone, Resource)]
pub struct RenderDiagnosticsFrames {
    frames: VecDeque<RenderDiagnosticFrame>,
    capacity: usize,
    pub dropped_frames: u64,
}

impl Default for RenderDiagnosticsFrames {
    fn default() -> Self {
        Self::new(RenderDiagnosticsConfig::default().completed_frame_capacity)
    }
}

impl RenderDiagnosticsFrames {
    pub fn new(capacity: usize) -> Self {
        Self {
            frames: VecDeque::with_capacity(capacity.clamp(1, 256)),
            capacity: capacity.clamp(1, 256),
            dropped_frames: 0,
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = &RenderDiagnosticFrame> {
        self.frames.iter()
    }

    pub fn drain(&mut self) -> impl Iterator<Item = RenderDiagnosticFrame> + '_ {
        self.frames.drain(..)
    }

    pub fn len(&self) -> usize {
        self.frames.len()
    }

    pub fn is_empty(&self) -> bool {
        self.frames.is_empty()
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }

    pub(crate) fn push(&mut self, frame: RenderDiagnosticFrame) {
        let index = self
            .frames
            .iter()
            .position(|queued| queued.frame_id > frame.frame_id)
            .unwrap_or(self.frames.len());
        self.frames.insert(index, frame);
        if self.frames.len() > self.capacity {
            self.frames.pop_front();
            self.dropped_frames = self.dropped_frames.saturating_add(1);
        }
    }
}

pub(crate) fn aggregate_diagnostics(
    diagnostics: impl IntoIterator<Item = RenderDiagnostic>,
) -> Vec<RenderDiagnostic> {
    let mut aggregates: Vec<RenderDiagnostic> = Vec::new();
    let mut indices: HashMap<DiagnosticPath, usize> = HashMap::default();
    for diagnostic in diagnostics {
        if let Some(&index) = indices.get(&diagnostic.path) {
            aggregates[index].value += diagnostic.value;
        } else {
            indices.insert(diagnostic.path.clone(), aggregates.len());
            aggregates.push(diagnostic);
        }
    }
    aggregates
}

fn exclusive_interval_ms(
    parent: RenderDiagnosticInterval,
    children: impl IntoIterator<Item = RenderDiagnosticInterval>,
) -> f64 {
    let mut intervals: Vec<_> = children
        .into_iter()
        .filter_map(|child| {
            let start = child.start_ns.max(parent.start_ns);
            let end = child.end_ns.min(parent.end_ns);
            (end > start).then_some((start, end))
        })
        .collect();
    intervals.sort_unstable_by(|a, b| a.0.total_cmp(&b.0));
    let mut covered = 0.0;
    let mut cursor = parent.start_ns;
    for (start, end) in intervals {
        covered += (end - start.max(cursor)).max(0.0);
        cursor = cursor.max(end);
    }
    ((parent.end_ns - parent.start_ns) - covered).max(0.0) / 1e6
}

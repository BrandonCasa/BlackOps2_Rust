use alloc::{borrow::Cow, sync::Arc};
use core::{
    ops::{DerefMut, Range},
    sync::atomic::{AtomicU8, Ordering},
};
use std::{
    sync::Mutex,
    thread::{self, ThreadId},
};

use bevy_diagnostic::{Diagnostic, DiagnosticMeasurement, DiagnosticPath, DiagnosticsStore};
use bevy_ecs::{
    resource::Resource,
    system::{Res, ResMut},
};
use bevy_platform::time::Instant;
use wgpu::{
    Buffer, BufferDescriptor, BufferSlice, BufferUsages, CommandEncoder, Device,
    Features, MapMode, PipelineStatisticsTypes, QuerySet, QuerySetDescriptor, QueryType,
};

use crate::renderer::{RenderAdapterInfo, RenderDevice, RenderQueue, wgpu_wrapper};

use super::{
    RecordDiagnostics,
    frame::{
        DiagnosticSpanContext, RenderDiagnostic, RenderDiagnosticCoverage, RenderDiagnosticFrame,
        RenderDiagnosticGpuStatus, RenderDiagnosticInterval, RenderDiagnosticSpan,
        RenderDiagnosticsConfig, RenderDiagnosticsFrameContext, RenderDiagnosticsFrames,
        aggregate_diagnostics, populate_exclusive_timings,
    },
    internal::{Pass, PassKind, WritePipelineStatistics, WriteTimestamp},
};

const TIMESTAMP_SIZE: u64 = 8;
const PIPELINE_STATISTICS_SIZE: u64 = 40;
const MAP_PENDING: u8 = 0;
const MAP_READY: u8 = 1;
const MAP_FAILED: u8 = 2;

fn supports_timestamp_site(
    features: Features,
    inside_pass: bool,
    encoder_suppressed: bool,
) -> bool {
    features.contains(Features::TIMESTAMP_QUERY)
        && if inside_pass {
            features.contains(Features::TIMESTAMP_QUERY_INSIDE_PASSES)
        } else {
            !encoder_suppressed && features.contains(Features::TIMESTAMP_QUERY_INSIDE_ENCODERS)
        }
}

struct DiagnosticsRecorderInternal {
    timestamp_period_ns: f32,
    features: Features,
    config: RenderDiagnosticsConfig,
    next_frame_id: u64,
    gpu_frames_allocated: usize,
    current_frame: Mutex<FrameData>,
    submitted_frames: Vec<FrameData>,
    finished_frames: Vec<FrameData>,
    #[cfg(feature = "tracing-tracy")]
    tracy_gpu_context: Option<tracy_client::GpuContext>,
}

wgpu_wrapper!(struct WgpuDiagnosticsRecorderInternal(DiagnosticsRecorderInternal));

/// Records bounded GPU query pairs and raw invocation identities. Recording is inert
/// when `RenderDiagnosticsPlugin` is omitted and this resource is not installed.
#[derive(Resource)]
pub struct DiagnosticsRecorder(WgpuDiagnosticsRecorderInternal);

impl DiagnosticsRecorder {
    pub fn new(
        adapter_info: &RenderAdapterInfo,
        device: &RenderDevice,
        queue: &RenderQueue,
    ) -> Self {
        Self::new_with_config(
            adapter_info,
            device,
            queue,
            &RenderDiagnosticsConfig::default(),
        )
    }

    pub fn new_with_config(
        adapter_info: &RenderAdapterInfo,
        device: &RenderDevice,
        queue: &RenderQueue,
        config: &RenderDiagnosticsConfig,
    ) -> Self {
        let features = device.features();
        let config = config.bounded();
        #[cfg(feature = "tracing-tracy")]
        let tracy_gpu_context =
            super::tracy_gpu::new_tracy_gpu_context(adapter_info, device, queue);
        let _ = adapter_info;
        let frame = FrameData::new(
            device,
            features,
            &config,
            true,
            #[cfg(feature = "tracing-tracy")]
            tracy_gpu_context.clone(),
        );
        let gpu_frames_allocated = 1;
        Self(WgpuDiagnosticsRecorderInternal::new(DiagnosticsRecorderInternal {
            timestamp_period_ns: queue.get_timestamp_period(),
            features,
            config,
            next_frame_id: 1,
            gpu_frames_allocated,
            current_frame: Mutex::new(frame),
            submitted_frames: Vec::new(),
            finished_frames: Vec::new(),
            #[cfg(feature = "tracing-tracy")]
            tracy_gpu_context,
        }))
    }

    fn current_frame_mut(&mut self) -> &mut FrameData {
        self.0.current_frame.get_mut().expect("lock poisoned")
    }

    fn current_frame_lock(&self) -> impl DerefMut<Target = FrameData> + '_ {
        self.0.current_frame.lock().expect("lock poisoned")
    }

    pub fn current_frame_id(&self) -> u64 {
        self.current_frame_lock().frame_id
    }

    pub fn begin_frame(&mut self) {
        self.begin_frame_with_context(RenderDiagnosticsFrameContext::default());
    }

    pub fn begin_frame_with_context(&mut self, context: RenderDiagnosticsFrameContext) {
        let internal = &mut self.0;
        let timestamp_period_ns = internal.timestamp_period_ns;
        let mut index = 0;
        while index < internal.submitted_frames.len() {
            if internal.submitted_frames[index].run_mapped_callback(timestamp_period_ns) {
                let frame = internal.submitted_frames.swap_remove(index);
                internal.finished_frames.push(frame);
            } else {
                index += 1;
            }
        }
        let frame_id = internal.next_frame_id;
        internal.next_frame_id = internal.next_frame_id.wrapping_add(1);
        internal
            .current_frame
            .get_mut()
            .expect("lock poisoned")
            .begin(frame_id, context);
    }

    pub fn resolve(&mut self, encoder: &mut CommandEncoder) {
        self.current_frame_mut().resolve(encoder);
    }

    /// Ends the frame after all measured command buffers (and the resolve encoder)
    /// are submitted. The callback retains the original frame/workload identity.
    pub fn finish_frame(
        &mut self,
        device: &RenderDevice,
        callback: impl FnOnce(RenderDiagnosticFrame) + Send + Sync + 'static,
    ) {
        let internal = &mut self.0;
        internal
            .current_frame
            .get_mut()
            .expect("lock poisoned")
            .finish(callback);

        // Prefer reusable GPU storage. Once all bounded GPU slots are in flight,
        // recording continues on CPU-only scratch rather than growing query buffers.
        let reusable_gpu = internal
            .finished_frames
            .iter()
            .rposition(|frame| frame.gpu_enabled);
        let new_frame = if let Some(index) = reusable_gpu {
            internal.finished_frames.swap_remove(index)
        } else if internal.gpu_frames_allocated < internal.config.max_in_flight_frames {
            let frame = FrameData::new(
                device,
                internal.features,
                &internal.config,
                true,
                #[cfg(feature = "tracing-tracy")]
                internal.tracy_gpu_context.clone(),
            );
            internal.gpu_frames_allocated += 1;
            frame
        } else if let Some(frame) = internal.finished_frames.pop() {
            frame
        } else {
            FrameData::new(
                device,
                internal.features,
                &internal.config,
                false,
                #[cfg(feature = "tracing-tracy")]
                internal.tracy_gpu_context.clone(),
            )
        };
        let old = core::mem::replace(
            internal.current_frame.get_mut().expect("lock poisoned"),
            new_frame,
        );
        internal.submitted_frames.push(old);
    }
}

impl RecordDiagnostics for DiagnosticsRecorder {
    fn record_f32<N: Into<Cow<'static, str>>>(
        &self,
        encoder: &mut CommandEncoder,
        buffer: &BufferSlice,
        name: N,
    ) {
        assert_value_buffer(buffer);
        self.current_frame_lock()
            .record_value(encoder, buffer, name.into(), true);
    }

    fn record_u32<N: Into<Cow<'static, str>>>(
        &self,
        encoder: &mut CommandEncoder,
        buffer: &BufferSlice,
        name: N,
    ) {
        assert_value_buffer(buffer);
        self.current_frame_lock()
            .record_value(encoder, buffer, name.into(), false);
    }

    fn begin_time_span<E: WriteTimestamp>(&self, encoder: &mut E, name: Cow<'static, str>) {
        self.begin_time_span_with_context(encoder, name, DiagnosticSpanContext::default());
    }

    fn begin_time_span_with_context<E: WriteTimestamp>(
        &self,
        encoder: &mut E,
        name: Cow<'static, str>,
        context: DiagnosticSpanContext,
    ) {
        self.current_frame_lock()
            .begin_time_span(encoder, name, context);
    }

    fn end_time_span<E: WriteTimestamp>(&self, encoder: &mut E) {
        self.current_frame_lock().end_time_span(encoder);
    }

    fn begin_pass_span<P: Pass>(&self, pass: &mut P, name: Cow<'static, str>) {
        self.begin_pass_span_with_context(pass, name, DiagnosticSpanContext::default());
    }

    fn begin_pass_span_with_context<P: Pass>(
        &self,
        pass: &mut P,
        name: Cow<'static, str>,
        context: DiagnosticSpanContext,
    ) {
        self.current_frame_lock().begin_pass(pass, name, context);
    }

    fn end_pass_span<P: Pass>(&self, pass: &mut P) {
        self.current_frame_lock().end_pass(pass);
    }
}

fn assert_value_buffer(buffer: &BufferSlice) {
    assert_eq!(
        buffer.size(),
        4,
        "Diagnostic value slice must be 4 bytes"
    );
    assert!(
        buffer.buffer().usage().contains(BufferUsages::COPY_SRC),
        "Diagnostic value buffer needs COPY_SRC"
    );
}

#[derive(Default)]
struct TimestampQueryPairs {
    capacity: u32,
    used: u32,
}

impl TimestampQueryPairs {
    fn reserve(&mut self) -> Option<(u32, u32)> {
        if self.capacity.saturating_sub(self.used) < 2 {
            return None;
        }
        let pair = (self.used, self.used + 1);
        self.used += 2;
        Some(pair)
    }
}

struct SpanRecord {
    invocation_id: u64,
    parent_invocation_id: Option<u64>,
    thread_id: ThreadId,
    path_range: Range<usize>,
    pass_kind: Option<PassKind>,
    context: DiagnosticSpanContext,
    timestamp_pair: Option<(u32, u32)>,
    gpu_status: RenderDiagnosticGpuStatus,
    begin_instant: Instant,
    end_instant: Option<Instant>,
    pipeline_statistics_index: Option<u32>,
}

/// Only completed entries may be resolved: a reserved endpoint has not necessarily
/// been written, and resolving an unwritten Vulkan query can wait indefinitely.
/// Keep adjacent entries together, but never cross a native query-set boundary.
fn completed_query_ranges(
    completed: &[bool],
    queries_per_entry: u32,
) -> impl Iterator<Item = Range<u32>> + '_ {
    let mut cursor = 0;
    core::iter::from_fn(move || {
        while cursor < completed.len() && !completed[cursor] {
            cursor += 1;
        }
        if cursor == completed.len() {
            return None;
        }
        let start = cursor as u32 * queries_per_entry;
        let bank_end = (start / wgpu::QUERY_SET_MAX_QUERIES + 1) * wgpu::QUERY_SET_MAX_QUERIES;
        while cursor < completed.len()
            && completed[cursor]
            && (cursor as u32 * queries_per_entry) < bank_end
        {
            cursor += 1;
        }
        Some(start..cursor as u32 * queries_per_entry)
    })
}

fn span_gpu_interval(
    span: &SpanRecord,
    completed_pairs: &[bool],
    timestamps: &[u64],
    timestamp_period_ns: f32,
    map_failed: bool,
) -> (RenderDiagnosticGpuStatus, Option<RenderDiagnosticInterval>) {
    if span.end_instant.is_none() {
        return (RenderDiagnosticGpuStatus::Incomplete, None);
    }
    let Some((begin, end)) = span.timestamp_pair else {
        return (span.gpu_status, None);
    };
    if map_failed {
        return (RenderDiagnosticGpuStatus::ReadbackFailed, None);
    }
    if completed_pairs.get(begin as usize / 2) != Some(&true) {
        return (RenderDiagnosticGpuStatus::Incomplete, None);
    }
    match (timestamps.get(begin as usize), timestamps.get(end as usize)) {
        (Some(&begin), Some(&end)) if end >= begin => (
            RenderDiagnosticGpuStatus::Available,
            Some(RenderDiagnosticInterval {
                start_ns: begin as f64 * timestamp_period_ns as f64,
                end_ns: end as f64 * timestamp_period_ns as f64,
            }),
        ),
        _ => (RenderDiagnosticGpuStatus::Incomplete, None),
    }
}

fn completed_statistics_index(span: &SpanRecord, completed: &[bool]) -> Option<u32> {
    span.end_instant?;
    let index = span.pipeline_statistics_index?;
    (completed.get(index as usize) == Some(&true)).then_some(index)
}

struct ValueReadback {
    buffer: Buffer,
    name: Cow<'static, str>,
    is_f32: bool,
    state: Arc<AtomicU8>,
}

struct FrameData {
    device: Device,
    frame_id: u64,
    context: RenderDiagnosticsFrameContext,
    start: Instant,
    end: Instant,
    timestamps_query_sets: Vec<QuerySet>,
    timestamp_pairs: TimestampQueryPairs,
    completed_timestamp_pairs: Vec<bool>,
    supports_timestamps_inside_passes: bool,
    supports_timestamps_inside_encoders: bool,
    gpu_enabled: bool,
    pipeline_statistics_query_set: Option<QuerySet>,
    pipeline_statistics_capacity: u32,
    num_pipeline_statistics: u32,
    completed_pipeline_statistics: Vec<bool>,
    pipeline_statistics_capacity_exhausted: u32,
    pipeline_statistics_buffer_offset: u64,
    resolve_buffer: Option<Buffer>,
    read_buffer: Option<Buffer>,
    path_components: Vec<Cow<'static, str>>,
    open_spans: Vec<SpanRecord>,
    closed_spans: Vec<SpanRecord>,
    next_invocation_id: u64,
    value_buffers: Vec<ValueReadback>,
    value_readback_backpressure: u32,
    readback_state: Arc<AtomicU8>,
    callback: Option<Box<dyn FnOnce(RenderDiagnosticFrame) + Send + Sync + 'static>>,
    #[cfg(feature = "tracing-tracy")]
    tracy_gpu_context: Option<tracy_client::GpuContext>,
}

impl FrameData {
    fn new(
        device: &RenderDevice,
        features: Features,
        config: &RenderDiagnosticsConfig,
        gpu_enabled: bool,
        #[cfg(feature = "tracing-tracy")] tracy_gpu_context: Option<tracy_client::GpuContext>,
    ) -> Self {
        let wgpu_device = device.wgpu_device();
        let mut buffer_size = 0;
        let mut timestamps_query_sets = Vec::new();
        let supports_timestamps_inside_passes = supports_timestamp_site(features, true, false);
        let supports_timestamps_inside_encoders =
            supports_timestamp_site(features, false, cfg!(target_os = "macos"));
        if gpu_enabled && (supports_timestamps_inside_passes || supports_timestamps_inside_encoders)
        {
            let mut remaining = config.timestamp_queries;
            while remaining > 0 {
                let count = remaining.min(wgpu::QUERY_SET_MAX_QUERIES);
                timestamps_query_sets.push(wgpu_device.create_query_set(&QuerySetDescriptor {
                    label: Some("render_timestamps_query_bank"),
                    ty: QueryType::Timestamp,
                    count,
                }));
                remaining -= count;
            }
            buffer_size = u64::from(config.timestamp_queries) * TIMESTAMP_SIZE;
        }
        let pipeline_statistics_buffer_offset = buffer_size;
        let pipeline_statistics_query_set = if gpu_enabled
            && config.pipeline_statistics_queries > 0
            && features.contains(Features::PIPELINE_STATISTICS_QUERY)
        {
            buffer_size += u64::from(config.pipeline_statistics_queries) * PIPELINE_STATISTICS_SIZE;
            Some(wgpu_device.create_query_set(&QuerySetDescriptor {
                label: Some("render_pipeline_statistics_query_set"),
                ty: QueryType::PipelineStatistics(PipelineStatisticsTypes::all()),
                count: config.pipeline_statistics_queries,
            }))
        } else {
            None
        };
        let (resolve_buffer, read_buffer) = if buffer_size > 0 {
            (
                Some(wgpu_device.create_buffer(&BufferDescriptor {
                    label: Some("render_statistics_resolve_buffer"),
                    size: buffer_size,
                    usage: BufferUsages::QUERY_RESOLVE | BufferUsages::COPY_SRC,
                    mapped_at_creation: false,
                })),
                Some(wgpu_device.create_buffer(&BufferDescriptor {
                    label: Some("render_statistics_read_buffer"),
                    size: buffer_size,
                    usage: BufferUsages::COPY_DST | BufferUsages::MAP_READ,
                    mapped_at_creation: false,
                })),
            )
        } else {
            (None, None)
        };
        let now = Instant::now();
        Self {
            device: wgpu_device.clone(),
            frame_id: 0,
            context: RenderDiagnosticsFrameContext::default(),
            start: now,
            end: now,
            timestamps_query_sets,
            timestamp_pairs: TimestampQueryPairs {
                capacity: config.timestamp_queries,
                used: 0,
            },
            completed_timestamp_pairs: vec![false; config.timestamp_queries as usize / 2],
            supports_timestamps_inside_passes,
            supports_timestamps_inside_encoders,
            gpu_enabled,
            pipeline_statistics_query_set,
            pipeline_statistics_capacity: config.pipeline_statistics_queries,
            num_pipeline_statistics: 0,
            completed_pipeline_statistics: vec![false; config.pipeline_statistics_queries as usize],
            pipeline_statistics_capacity_exhausted: 0,
            pipeline_statistics_buffer_offset,
            resolve_buffer,
            read_buffer,
            path_components: Vec::new(),
            open_spans: Vec::new(),
            closed_spans: Vec::new(),
            next_invocation_id: 1,
            value_buffers: Vec::new(),
            value_readback_backpressure: 0,
            readback_state: Arc::new(AtomicU8::new(MAP_PENDING)),
            callback: None,
            #[cfg(feature = "tracing-tracy")]
            tracy_gpu_context,
        }
    }

    fn begin(&mut self, frame_id: u64, context: RenderDiagnosticsFrameContext) {
        self.frame_id = frame_id;
        self.context = context;
        self.start = Instant::now();
        self.end = self.start;
        self.timestamp_pairs.used = 0;
        self.completed_timestamp_pairs.fill(false);
        self.num_pipeline_statistics = 0;
        self.completed_pipeline_statistics.fill(false);
        self.pipeline_statistics_capacity_exhausted = 0;
        self.path_components.clear();
        self.open_spans.clear();
        self.closed_spans.clear();
        self.next_invocation_id = 1;
        self.value_readback_backpressure = 0;
        self.readback_state.store(MAP_PENDING, Ordering::Release);
    }

    fn reserve_timestamps(
        &mut self,
        inside_pass: bool,
    ) -> (Option<(u32, u32)>, RenderDiagnosticGpuStatus) {
        if !self.gpu_enabled {
            return (None, RenderDiagnosticGpuStatus::ReadbackBackpressure);
        }
        let supported = if inside_pass {
            self.supports_timestamps_inside_passes
        } else {
            self.supports_timestamps_inside_encoders
        };
        if !supported || self.timestamps_query_sets.is_empty() {
            return (None, RenderDiagnosticGpuStatus::Unsupported);
        }
        match self.timestamp_pairs.reserve() {
            Some(pair) => (Some(pair), RenderDiagnosticGpuStatus::Available),
            None => (None, RenderDiagnosticGpuStatus::CapacityExhausted),
        }
    }

    fn write_timestamp_index(&self, encoder: &mut impl WriteTimestamp, index: u32) {
        let bank = index / wgpu::QUERY_SET_MAX_QUERIES;
        let index = index % wgpu::QUERY_SET_MAX_QUERIES;
        encoder.write_timestamp(&self.timestamps_query_sets[bank as usize], index);
    }

    fn write_pipeline_statistics(
        &mut self,
        encoder: &mut impl WritePipelineStatistics,
    ) -> Option<u32> {
        let set = self.pipeline_statistics_query_set.as_ref()?;
        if self.num_pipeline_statistics >= self.pipeline_statistics_capacity {
            self.pipeline_statistics_capacity_exhausted += 1;
            return None;
        }
        let index = self.num_pipeline_statistics;
        encoder.begin_pipeline_statistics_query(set, index);
        self.num_pipeline_statistics += 1;
        Some(index)
    }

    fn open_span(
        &mut self,
        kind: Option<PassKind>,
        name: Cow<'static, str>,
        context: DiagnosticSpanContext,
        timestamp_pair: Option<(u32, u32)>,
        gpu_status: RenderDiagnosticGpuStatus,
        begin: Instant,
    ) -> &mut SpanRecord {
        let thread_id = thread::current().id();
        let parent = self
            .open_spans
            .iter()
            .rfind(|span| span.thread_id == thread_id);
        let parent_invocation_id = parent.map(|span| span.invocation_id);
        let context = DiagnosticSpanContext {
            view: context
                .view
                .or_else(|| parent.and_then(|span| span.context.view)),
            subview: context
                .subview
                .or_else(|| parent.and_then(|span| span.context.subview)),
            submission: context
                .submission
                .or_else(|| parent.and_then(|span| span.context.submission)),
        };
        let path_range = match parent {
            Some(parent) if parent.path_range.end == self.path_components.len() => {
                parent.path_range.start..parent.path_range.end + 1
            }
            Some(parent) => {
                self.path_components
                    .extend_from_within(parent.path_range.clone());
                self.path_components.len() - parent.path_range.len()..self.path_components.len() + 1
            }
            None => self.path_components.len()..self.path_components.len() + 1,
        };
        self.path_components.push(name);
        let invocation_id = self.next_invocation_id;
        self.next_invocation_id += 1;
        self.open_spans.push(SpanRecord {
            invocation_id,
            parent_invocation_id,
            thread_id,
            path_range,
            pass_kind: kind,
            context,
            timestamp_pair,
            gpu_status,
            begin_instant: begin,
            end_instant: None,
            pipeline_statistics_index: None,
        });
        self.open_spans.last_mut().unwrap()
    }

    fn close_span(&mut self) -> &mut SpanRecord {
        let thread_id = thread::current().id();
        let index = self
            .open_spans
            .iter()
            .rposition(|span| span.thread_id == thread_id)
            .expect("No open diagnostic span on this thread");
        let mut span = self.open_spans.swap_remove(index);
        span.end_instant = Some(Instant::now());
        self.closed_spans.push(span);
        self.closed_spans.last_mut().unwrap()
    }

    fn record_value(
        &mut self,
        encoder: &mut CommandEncoder,
        buffer: &BufferSlice,
        name: Cow<'static, str>,
        is_f32: bool,
    ) {
        if !self.gpu_enabled {
            self.value_readback_backpressure = self.value_readback_backpressure.saturating_add(1);
            return;
        }
        let dest = self.device.create_buffer(&BufferDescriptor {
            label: Some("render_diagnostic_value"),
            size: 4,
            usage: BufferUsages::COPY_DST | BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        encoder.copy_buffer_to_buffer(
            buffer.buffer(),
            buffer.offset(),
            &dest,
            0,
            Some(buffer.size().into()),
        );
        let state = Arc::new(AtomicU8::new(MAP_PENDING));
        let map_state = state.clone();
        encoder.map_buffer_on_submit(&dest, MapMode::Read, .., move |result| {
            map_state.store(
                if result.is_ok() {
                    MAP_READY
                } else {
                    MAP_FAILED
                },
                Ordering::Release,
            );
        });
        self.value_buffers.push(ValueReadback {
            buffer: dest,
            name,
            is_f32,
            state,
        });
    }

    fn begin_time_span<E: WriteTimestamp>(
        &mut self,
        encoder: &mut E,
        name: Cow<'static, str>,
        context: DiagnosticSpanContext,
    ) {
        let begin = Instant::now();
        let (pair, status) = self.reserve_timestamps(E::INSIDE_PASS);
        if let Some((index, _)) = pair {
            self.write_timestamp_index(encoder, index);
        }
        self.open_span(None, name, context, pair, status, begin);
    }

    fn end_time_span(&mut self, encoder: &mut impl WriteTimestamp) {
        let pair = self.close_span().timestamp_pair;
        if let Some((_, index)) = pair {
            self.write_timestamp_index(encoder, index);
            self.completed_timestamp_pairs[index as usize / 2] = true;
        }
    }

    fn begin_pass<P: Pass>(
        &mut self,
        pass: &mut P,
        name: Cow<'static, str>,
        context: DiagnosticSpanContext,
    ) {
        let begin = Instant::now();
        let (pair, status) = self.reserve_timestamps(true);
        if let Some((index, _)) = pair {
            self.write_timestamp_index(pass, index);
        }
        let pipeline_statistics_index = self.write_pipeline_statistics(pass);
        let span = self.open_span(Some(P::KIND), name, context, pair, status, begin);
        span.pipeline_statistics_index = pipeline_statistics_index;
    }

    fn end_pass(&mut self, pass: &mut impl Pass) {
        let span = self.close_span();
        let pair = span.timestamp_pair;
        let statistics_index = span.pipeline_statistics_index;
        if let Some((_, index)) = pair {
            self.write_timestamp_index(pass, index);
            self.completed_timestamp_pairs[index as usize / 2] = true;
        }
        if let Some(index) = statistics_index {
            pass.end_pipeline_statistics_query();
            self.completed_pipeline_statistics[index as usize] = true;
        }
    }

    fn resolve(&mut self, encoder: &mut CommandEncoder) {
        let timestamp_completions =
            &self.completed_timestamp_pairs[..self.timestamp_pairs.used as usize / 2];
        let statistics_completions =
            &self.completed_pipeline_statistics[..self.num_pipeline_statistics as usize];
        let Some(resolve) = &self.resolve_buffer else {
            return;
        };
        let Some(read) = &self.read_buffer else {
            return;
        };
        // Reuse aligned scratch offset zero for each surviving run and copy it
        // immediately into the canonical readback layout. Encoder ordering keeps
        // each copy ahead of the next overwrite, with no padding allocation and no
        // resolve of reserved-but-unwritten indices. A run never exceeds the existing
        // buffer capacity, including when timestamp or statistics queries are disabled.
        for range in completed_query_ranges(timestamp_completions, 2) {
            let bank = range.start / wgpu::QUERY_SET_MAX_QUERIES;
            let base = bank * wgpu::QUERY_SET_MAX_QUERIES;
            let size = u64::from(range.end - range.start) * TIMESTAMP_SIZE;
            encoder.resolve_query_set(
                &self.timestamps_query_sets[bank as usize],
                range.start - base..range.end - base,
                resolve,
                0,
            );
            encoder.copy_buffer_to_buffer(
                resolve,
                0,
                read,
                u64::from(range.start) * TIMESTAMP_SIZE,
                size,
            );
        }
        if let Some(set) = &self.pipeline_statistics_query_set {
            for range in completed_query_ranges(statistics_completions, 1) {
                let size = u64::from(range.end - range.start) * PIPELINE_STATISTICS_SIZE;
                encoder.resolve_query_set(set, range.clone(), resolve, 0);
                encoder.copy_buffer_to_buffer(
                    resolve,
                    0,
                    read,
                    self.pipeline_statistics_buffer_offset
                        + u64::from(range.start) * PIPELINE_STATISTICS_SIZE,
                    size,
                );
            }
        }
    }

    fn finish(&mut self, callback: impl FnOnce(RenderDiagnosticFrame) + Send + Sync + 'static) {
        self.end = Instant::now();
        self.callback = Some(Box::new(callback));
        if let Some(read) = &self.read_buffer {
            let state = self.readback_state.clone();
            read.slice(..).map_async(MapMode::Read, move |result| {
                if let Err(error) = &result {
                    bevy_log::warn!("Failed to download render statistics buffer: {error}");
                }
                state.store(
                    if result.is_ok() {
                        MAP_READY
                    } else {
                        MAP_FAILED
                    },
                    Ordering::Release,
                );
            });
        } else {
            self.readback_state.store(MAP_READY, Ordering::Release);
        }
    }

    fn run_mapped_callback(&mut self, timestamp_period_ns: f32) -> bool {
        let mut state = self.readback_state.load(Ordering::Acquire);
        if state == MAP_PENDING
            || self
                .value_buffers
                .iter()
                .any(|value| value.state.load(Ordering::Acquire) == MAP_PENDING)
        {
            return false;
        }
        let Some(callback) = self.callback.take() else {
            return true;
        };
        let mut timestamps = Vec::new();
        let mut statistics = Vec::new();
        if state == MAP_READY {
            if let Some(read) = &self.read_buffer {
                if let Ok(data) = read.slice(..).get_mapped_range() {
                timestamps.extend(
                    data[..(self.timestamp_pairs.used as usize * 8)]
                        .as_chunks::<8>()
                        .0
                        .iter()
                        .map(|&bytes| u64::from_le_bytes(bytes)),
                );
                let start = self.pipeline_statistics_buffer_offset as usize;
                let len = self.num_pipeline_statistics as usize * 40;
                statistics.extend(
                    data[start..start + len]
                        .as_chunks::<8>()
                        .0
                        .iter()
                        .map(|&bytes| u64::from_le_bytes(bytes)),
                );
                drop(data);
                } else {
                    state = MAP_FAILED;
                }
                read.unmap();
            }
        } else if let Some(read) = &self.read_buffer {
            read.unmap();
        }
        let mut frame = self.completed_frame(
            &timestamps,
            &statistics,
            timestamp_period_ns,
            state == MAP_FAILED,
        );
        for value in self.value_buffers.drain(..) {
            if value.state.load(Ordering::Acquire) == MAP_FAILED {
                frame.coverage.value_readback_failures += 1;
                value.buffer.unmap();
                continue;
            }
            let Ok(data) = value.buffer.get_mapped_range(..) else {
                frame.coverage.value_readback_failures += 1;
                value.buffer.unmap();
                continue;
            };
            let bytes: [u8; 4] = (*data).try_into().unwrap();
            frame.values.push(RenderDiagnostic {
                path: DiagnosticPath::from_components(["render", value.name.as_ref()]),
                suffix: "",
                value: if value.is_f32 {
                    f32::from_le_bytes(bytes) as f64
                } else {
                    u32::from_le_bytes(bytes) as f64
                },
            });
            drop(data);
            value.buffer.unmap();
        }
        frame.diagnostics.extend(frame.values.iter().cloned());
        frame.diagnostics = aggregate_diagnostics(frame.diagnostics);
        callback(frame);
        true
    }

    fn completed_frame(
        &self,
        timestamps: &[u64],
        statistics: &[u64],
        timestamp_period_ns: f32,
        map_failed: bool,
    ) -> RenderDiagnosticFrame {
        let mut frame = RenderDiagnosticFrame {
            frame_id: self.frame_id,
            context: self.context,
            start: self.start,
            end: self.end,
            spans: Vec::with_capacity(self.closed_spans.len() + self.open_spans.len()),
            values: Vec::new(),
            diagnostics: Vec::new(),
            coverage: RenderDiagnosticCoverage {
                timestamp_queries_used: self.timestamp_pairs.used,
                timestamp_query_capacity: self.timestamp_pairs.capacity,
                pipeline_statistics_queries_used: self.num_pipeline_statistics,
                pipeline_statistics_query_capacity: self.pipeline_statistics_capacity,
                pipeline_statistics_capacity_exhausted: self.pipeline_statistics_capacity_exhausted,
                value_readback_backpressure: self.value_readback_backpressure,
                ..Default::default()
            },
        };
        for span in self.closed_spans.iter().chain(&self.open_spans) {
            let cpu_interval = span.end_instant.map(|end| RenderDiagnosticInterval {
                start_ns: (span.begin_instant - self.start).as_secs_f64() * 1e9,
                end_ns: (end - self.start).as_secs_f64() * 1e9,
            });
            let (gpu_status, gpu_interval) = span_gpu_interval(
                span,
                &self.completed_timestamp_pairs,
                timestamps,
                timestamp_period_ns,
                map_failed,
            );
            frame.coverage.spans += 1;
            match gpu_status {
                RenderDiagnosticGpuStatus::Available => frame.coverage.gpu_spans += 1,
                RenderDiagnosticGpuStatus::Unsupported => frame.coverage.unsupported_spans += 1,
                RenderDiagnosticGpuStatus::CapacityExhausted => {
                    frame.coverage.capacity_exhausted_spans += 1
                }
                RenderDiagnosticGpuStatus::ReadbackFailed => {
                    frame.coverage.readback_failed_spans += 1
                }
                RenderDiagnosticGpuStatus::ReadbackBackpressure => {
                    frame.coverage.readback_backpressure_spans += 1
                }
                RenderDiagnosticGpuStatus::Incomplete => frame.coverage.incomplete_spans += 1,
            }
            let path = DiagnosticPath::from_components(
                core::iter::once("render").chain(
                    self.path_components[span.path_range.clone()]
                        .iter()
                        .map(|component| component.as_ref()),
                ),
            );
            if let Some(interval) = cpu_interval {
                frame.diagnostics.push(RenderDiagnostic {
                    path: path_field(&path, "elapsed_cpu"),
                    suffix: "ms",
                    value: interval.duration_ms(),
                });
            }
            if let Some(interval) = gpu_interval {
                frame.diagnostics.push(RenderDiagnostic {
                    path: path_field(&path, "elapsed_gpu"),
                    suffix: "ms",
                    value: interval.duration_ms(),
                });
                #[cfg(feature = "tracing-tracy")]
                if let Some(context) = &self.tracy_gpu_context {
                    let mut zone = context.span_alloc(path.as_str(), "", "", 0).unwrap();
                    zone.end_zone();
                    let (begin, end) = span.timestamp_pair.unwrap();
                    zone.upload_timestamp_start(timestamps[begin as usize] as i64);
                    zone.upload_timestamp_end(timestamps[end as usize] as i64);
                }
            }
            if !map_failed {
                if let Some(index) =
                    completed_statistics_index(span, &self.completed_pipeline_statistics)
                {
                    let index = index as usize * 5;
                    let fields: &[(usize, &str)] = match span.pass_kind {
                        Some(PassKind::Render) => &[
                            (0, "vertex_shader_invocations"),
                            (1, "clipper_invocations"),
                            (2, "clipper_primitives_out"),
                            (3, "fragment_shader_invocations"),
                        ],
                        Some(PassKind::Compute) => &[(4, "compute_shader_invocations")],
                        None => &[],
                    };
                    for &(offset, field) in fields {
                        if let Some(value) = statistics.get(index + offset) {
                            frame.diagnostics.push(RenderDiagnostic {
                                path: path_field(&path, field),
                                suffix: "",
                                value: *value as f64,
                            });
                        }
                    }
                }
            }
            frame.spans.push(RenderDiagnosticSpan {
                invocation_id: span.invocation_id,
                parent_invocation_id: span.parent_invocation_id,
                thread_id: span.thread_id,
                path,
                pass_kind: span.pass_kind,
                context: span.context,
                cpu_interval,
                gpu_interval,
                gpu_status,
                cpu_exclusive_ms: None,
                gpu_exclusive_ms: None,
            });
        }
        frame.spans.sort_unstable_by_key(|span| span.invocation_id);
        populate_exclusive_timings(&mut frame);
        // These are per-path exclusive measured times, not an implied full GPU total.
        for span in &frame.spans {
            if let Some(value) = frame.exclusive_cpu_ms(span) {
                frame.diagnostics.push(RenderDiagnostic {
                    path: path_field(&span.path, "elapsed_cpu_exclusive"),
                    suffix: "ms",
                    value,
                });
            }
            if let Some(value) = frame.exclusive_gpu_ms(span) {
                frame.diagnostics.push(RenderDiagnostic {
                    path: path_field(&span.path, "elapsed_gpu_exclusive"),
                    suffix: "ms",
                    value,
                });
            }
        }
        frame
    }
}

fn path_field(path: &DiagnosticPath, field: &str) -> DiagnosticPath {
    DiagnosticPath::from_components(path.components().chain(core::iter::once(field)))
}

#[derive(Debug, Clone, Resource)]
pub(crate) struct RenderDiagnosticsMutex(pub(crate) Arc<Mutex<RenderDiagnosticsFrames>>);

impl RenderDiagnosticsMutex {
    pub(crate) fn new(capacity: usize) -> Self {
        Self(Arc::new(Mutex::new(RenderDiagnosticsFrames::new(capacity))))
    }
}

pub(crate) fn sync_diagnostics(
    mutex: Res<RenderDiagnosticsMutex>,
    mut store: ResMut<DiagnosticsStore>,
    mut completed: ResMut<RenderDiagnosticsFrames>,
) {
    let Ok(mut pending) = mutex.0.lock() else {
        return;
    };
    completed.dropped_frames = completed
        .dropped_frames
        .saturating_add(core::mem::take(&mut pending.dropped_frames));
    let ready: Vec<_> = pending.drain().collect();
    drop(pending);
    for frame in ready {
        // The stable store uses arrival time so out-of-order readbacks do not rewind
        // its moving average. Raw records retain original source/frame timestamps.
        let time = Instant::now();
        for diagnostic in &frame.diagnostics {
            if store.get(&diagnostic.path).is_none() {
                store.add(Diagnostic::new(diagnostic.path.clone()).with_suffix(diagnostic.suffix));
            }
            store
                .get_mut(&diagnostic.path)
                .unwrap()
                .add_measurement(DiagnosticMeasurement {
                    time,
                    value: diagnostic.value,
                });
        }
        completed.push(frame);
    }
}

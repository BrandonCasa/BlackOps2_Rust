use wgpu::{CommandEncoder, ComputePass, QuerySet, RenderPass};

pub use super::recorder::DiagnosticsRecorder;
pub(crate) use super::recorder::{RenderDiagnosticsMutex, sync_diagnostics};
pub trait WriteTimestamp {
    /// Whether writes occur inside an active render or compute pass. Custom
    /// command-encoder implementations retain the existing default behavior.
    const INSIDE_PASS: bool = false;

    fn write_timestamp(&mut self, query_set: &QuerySet, index: u32);
}

impl WriteTimestamp for CommandEncoder {
    const INSIDE_PASS: bool = false;

    fn write_timestamp(&mut self, query_set: &QuerySet, index: u32) {
        if cfg!(target_os = "macos") {
            // When using tracy (and thus this function), rendering was flickering on macOS Tahoe.
            // See: https://github.com/bevyengine/bevy/issues/22257
            // The issue seems to be triggered when `write_timestamp` is called very close to frame
            // presentation.
            return;
        }
        CommandEncoder::write_timestamp(self, query_set, index);
    }
}

impl WriteTimestamp for RenderPass<'_> {
    const INSIDE_PASS: bool = true;

    fn write_timestamp(&mut self, query_set: &QuerySet, index: u32) {
        RenderPass::write_timestamp(self, query_set, index);
    }
}

impl WriteTimestamp for ComputePass<'_> {
    const INSIDE_PASS: bool = true;

    fn write_timestamp(&mut self, query_set: &QuerySet, index: u32) {
        ComputePass::write_timestamp(self, query_set, index);
    }
}

pub trait WritePipelineStatistics {
    fn begin_pipeline_statistics_query(&mut self, query_set: &QuerySet, index: u32);

    fn end_pipeline_statistics_query(&mut self);
}

impl WritePipelineStatistics for RenderPass<'_> {
    fn begin_pipeline_statistics_query(&mut self, query_set: &QuerySet, index: u32) {
        RenderPass::begin_pipeline_statistics_query(self, query_set, index);
    }

    fn end_pipeline_statistics_query(&mut self) {
        RenderPass::end_pipeline_statistics_query(self);
    }
}

impl WritePipelineStatistics for ComputePass<'_> {
    fn begin_pipeline_statistics_query(&mut self, query_set: &QuerySet, index: u32) {
        ComputePass::begin_pipeline_statistics_query(self, query_set, index);
    }

    fn end_pipeline_statistics_query(&mut self) {
        ComputePass::end_pipeline_statistics_query(self);
    }
}

pub trait Pass: WritePipelineStatistics + WriteTimestamp {
    const KIND: PassKind;
}

impl Pass for RenderPass<'_> {
    const KIND: PassKind = PassKind::Render;
}

impl Pass for ComputePass<'_> {
    const KIND: PassKind = PassKind::Compute;
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum PassKind {
    Render,
    Compute,
}

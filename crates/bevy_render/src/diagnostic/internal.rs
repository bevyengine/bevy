use std::{
    borrow::Cow,
    ops::{DerefMut, Range},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread::{self, ThreadId},
};

use bevy_diagnostic::{Diagnostic, DiagnosticMeasurement, DiagnosticPath, DiagnosticsStore};
use bevy_ecs::system::{Res, ResMut, Resource};
use bevy_utils::{tracing, Instant};
use std::sync::Mutex;
use wgpu::{
    Buffer, BufferDescriptor, BufferUsages, CommandEncoder, ComputePass, Features, MapMode,
    PipelineStatisticsTypes, QuerySet, QuerySetDescriptor, QueryType, Queue, RenderPass,
};

use crate::renderer::{RenderDevice, WgpuWrapper};

use super::RecordDiagnostics;

// buffer offset must be divisible by 256, so this constant must be divisible by 32 (=256/8)
const MAX_TIMESTAMP_QUERIES: u32 = 256;
const MAX_PIPELINE_STATISTICS: u32 = 128;

const TIMESTAMP_SIZE: u64 = 8;
const PIPELINE_STATISTICS_SIZE: u64 = 40;

struct DiagnosticsRecorderInternal {
    timestamp_period_ns: f32,
    features: Features,
    current_frame: Mutex<FrameData>,
    submitted_frames: Vec<FrameData>,
    finished_frames: Vec<FrameData>,
}

/// Records diagnostics into [`QuerySet`]'s keeping track of the mapping between
/// spans and indices to the corresponding entries in the [`QuerySet`].
#[derive(Resource)]
pub struct DiagnosticsRecorder(WgpuWrapper<DiagnosticsRecorderInternal>);

impl DiagnosticsRecorder {
    /// Creates the new `DiagnosticsRecorder`.
    pub fn new(device: &RenderDevice, queue: &Queue) -> DiagnosticsRecorder {
        let features = device.features();

        let timestamp_period_ns = if features.contains(Features::TIMESTAMP_QUERY) {
            queue.get_timestamp_period()
        } else {
            0.0
        };

        DiagnosticsRecorder(WgpuWrapper::new(DiagnosticsRecorderInternal {
            timestamp_period_ns,
            features,
            current_frame: Mutex::new(FrameData::new(device, features)),
            submitted_frames: Vec::new(),
            finished_frames: Vec::new(),
        }))
    }

    fn current_frame_mut(&mut self) -> &mut FrameData {
        self.0.current_frame.get_mut().expect("lock poisoned")
    }

    fn current_frame_lock(&self) -> impl DerefMut<Target = FrameData> + '_ {
        self.0.current_frame.lock().expect("lock poisoned")
    }

    /// Begins recording diagnostics for a new frame.
    pub fn begin_frame(&mut self) {
        let internal = &mut self.0;
        let mut idx = 0;
        while idx < internal.submitted_frames.len() {
            let timestamp = internal.timestamp_period_ns;
            if internal.submitted_frames[idx].run_mapped_callback(timestamp) {
                let removed = internal.submitted_frames.swap_remove(idx);
                internal.finished_frames.push(removed);
            } else {
                idx += 1;
            }
        }

        self.current_frame_mut().begin();
    }

    /// Copies data from [`QuerySet`]'s to a [`Buffer`], after which it can be downloaded to CPU.
    ///
    /// Should be called before [`DiagnosticsRecorder::finish_frame`]
    pub fn resolve(&mut self, encoder: &mut CommandEncoder) {
        self.current_frame_mut().resolve(encoder);
    }

    /// Finishes recording diagnostics for the current frame.
    ///
    /// The specified `callback` will be invoked when diagnostics become available.
    ///
    /// Should be called after [`DiagnosticsRecorder::resolve`],
    /// and **after** all commands buffers have been queued.
    pub fn finish_frame(
        &mut self,
        device: &RenderDevice,
        callback: impl FnOnce(RenderDiagnostics) + Send + Sync + 'static,
    ) {
        let internal = &mut self.0;
        internal
            .current_frame
            .get_mut()
            .expect("lock poisoned")
            .finish(callback);

        // reuse one of the finished frames, if we can
        let new_frame = match internal.finished_frames.pop() {
            Some(frame) => frame,
            None => FrameData::new(device, internal.features),
        };

        let old_frame = std::mem::replace(
            internal.current_frame.get_mut().expect("lock poisoned"),
            new_frame,
        );
        internal.submitted_frames.push(old_frame);
    }
}

impl RecordDiagnostics for DiagnosticsRecorder {
    fn begin_time_span<E: WriteTimestamp>(&self, encoder: &mut E, span_name: Cow<'static, str>) {
        self.current_frame_lock()
            .begin_time_span(encoder, span_name);
    }

    fn end_time_span<E: WriteTimestamp>(&self, encoder: &mut E) {
        self.current_frame_lock().end_time_span(encoder);
    }

    fn begin_pass_span<P: Pass>(&self, pass: &mut P, span_name: Cow<'static, str>) {
        self.current_frame_lock().begin_pass(pass, span_name);
    }

    fn end_pass_span<P: Pass>(&self, pass: &mut P) {
        self.current_frame_lock().end_pass(pass);
    }
}

struct SpanRecord {
    thread_id: ThreadId,
    path_range: Range<usize>,
    pass_kind: Option<PassKind>,
    begin_timestamp_index: Option<u32>,
    end_timestamp_index: Option<u32>,
    begin_instant: Option<Instant>,
    end_instant: Option<Instant>,
    pipeline_statistics_index: Option<u32>,
}

struct FrameData {
    timestamps_query_set: Option<QuerySet>,
    num_timestamps: u32,
    supports_timestamps_inside_passes: bool,
    supports_timestamps_inside_encoders: bool,
    pipeline_statistics_query_set: Option<QuerySet>,
    num_pipeline_statistics: u32,
    buffer_size: u64,
    pipeline_statistics_buffer_offset: u64,
    resolve_buffer: Option<Buffer>,
    read_buffer: Option<Buffer>,
    path_components: Vec<Cow<'static, str>>,
    open_spans: Vec<SpanRecord>,
    closed_spans: Vec<SpanRecord>,
    is_mapped: Arc<AtomicBool>,
    callback: Option<Box<dyn FnOnce(RenderDiagnostics) + Send + Sync + 'static>>,
}

impl FrameData {
    fn new(device: &RenderDevice, features: Features) -> FrameData {
        let wgpu_device = device.wgpu_device();
        let mut buffer_size = 0;

        let timestamps_query_set = if features.contains(Features::TIMESTAMP_QUERY) {
            buffer_size += u64::from(MAX_TIMESTAMP_QUERIES) * TIMESTAMP_SIZE;
            Some(wgpu_device.create_query_set(&QuerySetDescriptor {
                label: Some("timestamps_query_set"),
                ty: QueryType::Timestamp,
                count: MAX_TIMESTAMP_QUERIES,
            }))
        } else {
            None
        };

        let pipeline_statistics_buffer_offset = buffer_size;

        let pipeline_statistics_query_set =
            if features.contains(Features::PIPELINE_STATISTICS_QUERY) {
                buffer_size += u64::from(MAX_PIPELINE_STATISTICS) * PIPELINE_STATISTICS_SIZE;
                Some(wgpu_device.create_query_set(&QuerySetDescriptor {
                    label: Some("pipeline_statistics_query_set"),
                    ty: QueryType::PipelineStatistics(PipelineStatisticsTypes::all()),
                    count: MAX_PIPELINE_STATISTICS,
                }))
            } else {
                None
            };

        let (resolve_buffer, read_buffer) = if buffer_size > 0 {
            let resolve_buffer = wgpu_device.create_buffer(&BufferDescriptor {
                label: Some("render_statistics_resolve_buffer"),
                size: buffer_size,
                usage: BufferUsages::QUERY_RESOLVE | BufferUsages::COPY_SRC,
                mapped_at_creation: false,
            });
            let read_buffer = wgpu_device.create_buffer(&BufferDescriptor {
                label: Some("render_statistics_read_buffer"),
                size: buffer_size,
                usage: BufferUsages::COPY_DST | BufferUsages::MAP_READ,
                mapped_at_creation: false,
            });
            (Some(resolve_buffer), Some(read_buffer))
        } else {
            (None, None)
        };

        FrameData {
            timestamps_query_set,
            num_timestamps: 0,
            supports_timestamps_inside_passes: features
                .contains(Features::TIMESTAMP_QUERY_INSIDE_PASSES),
            supports_timestamps_inside_encoders: features
                .contains(Features::TIMESTAMP_QUERY_INSIDE_ENCODERS),
            pipeline_statistics_query_set,
            num_pipeline_statistics: 0,
            buffer_size,
            pipeline_statistics_buffer_offset,
            resolve_buffer,
            read_buffer,
            path_components: Vec::new(),
            open_spans: Vec::new(),
            closed_spans: Vec::new(),
            is_mapped: Arc::new(AtomicBool::new(false)),
            callback: None,
        }
    }

    fn begin(&mut self) {
        self.num_timestamps = 0;
        self.num_pipeline_statistics = 0;
        self.path_components.clear();
        self.open_spans.clear();
        self.closed_spans.clear();
    }

    fn write_timestamp(
        &mut self,
        encoder: &mut impl WriteTimestamp,
        is_inside_pass: bool,
    ) -> Option<u32> {
        // `encoder.write_timestamp` is unsupported on WebGPU.
        if !self.supports_timestamps_inside_encoders {
            return None;
        }

        if is_inside_pass && !self.supports_timestamps_inside_passes {
            return None;
        }

        if self.num_timestamps >= MAX_TIMESTAMP_QUERIES {
            return None;
        }

        let set = self.timestamps_query_set.as_ref()?;
        let index = self.num_timestamps;
        encoder.write_timestamp(set, index);
        self.num_timestamps += 1;
        Some(index)
    }

    fn write_pipeline_statistics(
        &mut self,
        encoder: &mut impl WritePipelineStatistics,
    ) -> Option<u32> {
        if self.num_pipeline_statistics >= MAX_PIPELINE_STATISTICS {
            return None;
        }

        let set = self.pipeline_statistics_query_set.as_ref()?;
        let index = self.num_pipeline_statistics;
        encoder.begin_pipeline_statistics_query(set, index);
        self.num_pipeline_statistics += 1;
        Some(index)
    }

    fn open_span(
        &mut self,
        pass_kind: Option<PassKind>,
        name: Cow<'static, str>,
    ) -> &mut SpanRecord {
        let thread_id = thread::current().id();

        let parent = self
            .open_spans
            .iter()
            .filter(|v| v.thread_id == thread_id)
            .last();

        let path_range = match &parent {
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

        self.open_spans.push(SpanRecord {
            thread_id,
            path_range,
            pass_kind,
            begin_timestamp_index: None,
            end_timestamp_index: None,
            begin_instant: None,
            end_instant: None,
            pipeline_statistics_index: None,
        });

        self.open_spans.last_mut().unwrap()
    }

    fn close_span(&mut self) -> &mut SpanRecord {
        let thread_id = thread::current().id();

        let iter = self.open_spans.iter();
        let (index, _) = iter
            .enumerate()
            .filter(|(_, v)| v.thread_id == thread_id)
            .last()
            .unwrap();

        let span = self.open_spans.swap_remove(index);
        self.closed_spans.push(span);
        self.closed_spans.last_mut().unwrap()
    }

    fn begin_time_span(&mut self, encoder: &mut impl WriteTimestamp, name: Cow<'static, str>) {
        let begin_instant = Instant::now();
        let begin_timestamp_index = self.write_timestamp(encoder, false);

        let span = self.open_span(None, name);
        span.begin_instant = Some(begin_instant);
        span.begin_timestamp_index = begin_timestamp_index;
    }

    fn end_time_span(&mut self, encoder: &mut impl WriteTimestamp) {
        let end_timestamp_index = self.write_timestamp(encoder, false);

        let span = self.close_span();
        span.end_timestamp_index = end_timestamp_index;
        span.end_instant = Some(Instant::now());
    }

    fn begin_pass<P: Pass>(&mut self, pass: &mut P, name: Cow<'static, str>) {
        let begin_instant = Instant::now();

        let begin_timestamp_index = self.write_timestamp(pass, true);
        let pipeline_statistics_index = self.write_pipeline_statistics(pass);

        let span = self.open_span(Some(P::KIND), name);
        span.begin_instant = Some(begin_instant);
        span.begin_timestamp_index = begin_timestamp_index;
        span.pipeline_statistics_index = pipeline_statistics_index;
    }

    fn end_pass(&mut self, pass: &mut impl Pass) {
        let end_timestamp_index = self.write_timestamp(pass, true);

        let span = self.close_span();
        span.end_timestamp_index = end_timestamp_index;

        if span.pipeline_statistics_index.is_some() {
            pass.end_pipeline_statistics_query();
        }

        span.end_instant = Some(Instant::now());
    }

    fn resolve(&mut self, encoder: &mut CommandEncoder) {
        let Some(resolve_buffer) = &self.resolve_buffer else {
            return;
        };

        match &self.timestamps_query_set {
            Some(set) if self.num_timestamps > 0 => {
                encoder.resolve_query_set(set, 0..self.num_timestamps, resolve_buffer, 0);
            }
            _ => {}
        }

        match &self.pipeline_statistics_query_set {
            Some(set) if self.num_pipeline_statistics > 0 => {
                encoder.resolve_query_set(
                    set,
                    0..self.num_pipeline_statistics,
                    resolve_buffer,
                    self.pipeline_statistics_buffer_offset,
                );
            }
            _ => {}
        }

        let Some(read_buffer) = &self.read_buffer else {
            return;
        };

        encoder.copy_buffer_to_buffer(resolve_buffer, 0, read_buffer, 0, self.buffer_size);
    }

    fn diagnostic_path(&self, range: &Range<usize>, field: &str) -> DiagnosticPath {
        DiagnosticPath::from_components(
            std::iter::once("render")
                .chain(self.path_components[range.clone()].iter().map(|v| &**v))
                .chain(std::iter::once(field)),
        )
    }

    fn finish(&mut self, callback: impl FnOnce(RenderDiagnostics) + Send + Sync + 'static) {
        let Some(read_buffer) = &self.read_buffer else {
            // we still have cpu timings, so let's use them

            let mut diagnostics = Vec::new();

            for span in &self.closed_spans {
                if let (Some(begin), Some(end)) = (span.begin_instant, span.end_instant) {
                    diagnostics.push(RenderDiagnostic {
                        path: self.diagnostic_path(&span.path_range, "elapsed_cpu"),
                        suffix: "ms",
                        value: (end - begin).as_secs_f64() * 1000.0,
                    });
                }
            }

            callback(RenderDiagnostics(diagnostics));
            return;
        };

        self.callback = Some(Box::new(callback));

        let is_mapped = self.is_mapped.clone();
        read_buffer.slice(..).map_async(MapMode::Read, move |res| {
            if let Err(e) = res {
                tracing::warn!("Failed to download render statistics buffer: {e}");
                return;
            }

            is_mapped.store(true, Ordering::Release);
        });
    }

    // returns true if the frame is considered finished, false otherwise
    fn run_mapped_callback(&mut self, timestamp_period_ns: f32) -> bool {
        let Some(read_buffer) = &self.read_buffer else {
            return true;
        };
        if !self.is_mapped.load(Ordering::Acquire) {
            // need to wait more
            return false;
        }
        let Some(callback) = self.callback.take() else {
            return true;
        };

        let data = read_buffer.slice(..).get_mapped_range();

        let timestamps = data[..(self.num_timestamps * 8) as usize]
            .chunks(8)
            .map(|v| u64::from_ne_bytes(v.try_into().unwrap()))
            .collect::<Vec<u64>>();

        let start = self.pipeline_statistics_buffer_offset as usize;
        let len = (self.num_pipeline_statistics as usize) * 40;
        let pipeline_statistics = data[start..start + len]
            .chunks(8)
            .map(|v| u64::from_ne_bytes(v.try_into().unwrap()))
            .collect::<Vec<u64>>();

        let mut diagnostics = Vec::new();

        for span in &self.closed_spans {
            if let (Some(begin), Some(end)) = (span.begin_instant, span.end_instant) {
                diagnostics.push(RenderDiagnostic {
                    path: self.diagnostic_path(&span.path_range, "elapsed_cpu"),
                    suffix: "ms",
                    value: (end - begin).as_secs_f64() * 1000.0,
                });
            }

            if let (Some(begin), Some(end)) = (span.begin_timestamp_index, span.end_timestamp_index)
            {
                let begin = timestamps[begin as usize] as f64;
                let end = timestamps[end as usize] as f64;
                let value = (end - begin) * (timestamp_period_ns as f64) / 1e6;

                diagnostics.push(RenderDiagnostic {
                    path: self.diagnostic_path(&span.path_range, "elapsed_gpu"),
                    suffix: "ms",
                    value,
                });
            }

            if let Some(index) = span.pipeline_statistics_index {
                let index = (index as usize) * 5;

                if span.pass_kind == Some(PassKind::Render) {
                    diagnostics.push(RenderDiagnostic {
                        path: self.diagnostic_path(&span.path_range, "vertex_shader_invocations"),
                        suffix: "",
                        value: pipeline_statistics[index] as f64,
                    });

                    diagnostics.push(RenderDiagnostic {
                        path: self.diagnostic_path(&span.path_range, "clipper_invocations"),
                        suffix: "",
                        value: pipeline_statistics[index + 1] as f64,
                    });

                    diagnostics.push(RenderDiagnostic {
                        path: self.diagnostic_path(&span.path_range, "clipper_primitives_out"),
                        suffix: "",
                        value: pipeline_statistics[index + 2] as f64,
                    });

                    diagnostics.push(RenderDiagnostic {
                        path: self.diagnostic_path(&span.path_range, "fragment_shader_invocations"),
                        suffix: "",
                        value: pipeline_statistics[index + 3] as f64,
                    });
                }

                if span.pass_kind == Some(PassKind::Compute) {
                    diagnostics.push(RenderDiagnostic {
                        path: self.diagnostic_path(&span.path_range, "compute_shader_invocations"),
                        suffix: "",
                        value: pipeline_statistics[index + 4] as f64,
                    });
                }
            }
        }

        callback(RenderDiagnostics(diagnostics));

        drop(data);
        read_buffer.unmap();
        self.is_mapped.store(false, Ordering::Release);

        true
    }
}

/// Resource which stores render diagnostics of the most recent frame.
#[derive(Debug, Default, Clone, Resource)]
pub struct RenderDiagnostics(Vec<RenderDiagnostic>);

/// A render diagnostic which has been recorded, but not yet stored in [`DiagnosticsStore`].
#[derive(Debug, Clone, Resource)]
pub struct RenderDiagnostic {
    pub path: DiagnosticPath,
    pub suffix: &'static str,
    pub value: f64,
}

/// Stores render diagnostics before they can be synced with the main app.
///
/// This mutex is locked twice per frame:
///  1. in `PreUpdate`, during [`sync_diagnostics`],
///  2. after rendering has finished and statistics have been downloaded from GPU.
#[derive(Debug, Default, Clone, Resource)]
pub struct RenderDiagnosticsMutex(pub(crate) Arc<Mutex<Option<RenderDiagnostics>>>);

/// Updates render diagnostics measurements.
pub fn sync_diagnostics(mutex: Res<RenderDiagnosticsMutex>, mut store: ResMut<DiagnosticsStore>) {
    let Some(diagnostics) = mutex.0.lock().ok().and_then(|mut v| v.take()) else {
        return;
    };

    let time = Instant::now();

    for diagnostic in &diagnostics.0 {
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
}

pub trait WriteTimestamp {
    fn write_timestamp(&mut self, query_set: &QuerySet, index: u32);
}

impl WriteTimestamp for CommandEncoder {
    fn write_timestamp(&mut self, query_set: &QuerySet, index: u32) {
        CommandEncoder::write_timestamp(self, query_set, index);
    }
}

impl WriteTimestamp for RenderPass<'_> {
    fn write_timestamp(&mut self, query_set: &QuerySet, index: u32) {
        RenderPass::write_timestamp(self, query_set, index);
    }
}

impl WriteTimestamp for ComputePass<'_> {
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

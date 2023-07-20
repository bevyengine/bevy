use std::{
    borrow::Cow,
    hash::{Hash, Hasher},
    marker::PhantomData,
    ops::Range,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread::{self, ThreadId},
};

use bevy_app::{App, Plugin, PreUpdate};

use bevy_diagnostic::DiagnosticId;
use bevy_ecs::system::{Res, ResMut, Resource};
use bevy_utils::{AHasher, Duration, Instant, Uuid};
use parking_lot::Mutex;
use smallvec::SmallVec;
use wgpu::{
    Buffer, BufferDescriptor, BufferUsages, CommandEncoder, ComputePass, Features, MapMode,
    PipelineStatisticsTypes, QuerySet, QuerySetDescriptor, QueryType, Queue, RenderPass,
};

use crate::RenderApp;

use super::{RenderDevice, RenderQueue};

// buffer offset must be divisible by 256, so this constant must be divisible by 32 (=256/8)
const MAX_TIMESTAMP_QUERIES: u32 = 256;
const MAX_PIPELINE_STATISTICS: u32 = 128;

const TIMESTAMP_SIZE: u64 = 8;
const PIPELINE_STATISTICS_SIZE: u64 = 40;

/// Enables collecting rendering diagnostics into [`RenderDiagnostics`] resource.
///
/// # Supported platforms
/// Timestamp queries and pipeline statistics are currently supported only on Vulkan and DX12.
/// On other platforms (Metal, WebGPU, WebGL2) only CPU time will be recorded.
#[allow(clippy::doc_markdown)]
#[derive(Default)]
pub struct RenderDiagnosticsPlugin;

impl Plugin for RenderDiagnosticsPlugin {
    fn build(&self, app: &mut App) {
        let render_diagnostics_mutex = RenderDiagnosticsMutex::default();
        app.insert_resource(render_diagnostics_mutex.clone())
            .init_resource::<RenderDiagnostics>()
            .add_systems(PreUpdate, sync_render_diagnostics);

        if let Ok(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app.insert_resource(render_diagnostics_mutex);
        }
    }

    fn finish(&self, app: &mut App) {
        let Ok(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        let device = render_app.world.resource::<RenderDevice>();
        let queue = render_app.world.resource::<RenderQueue>();
        render_app.insert_resource(DiagnosticsRecorder::new(device, queue));
    }
}

/// Resource which stores rendering diagnostics of the most recent frame.
#[derive(Debug, Default, Clone, Resource)]
pub struct RenderDiagnostics(pub Vec<RenderSpanDiagnostics>);

/// Diagnostics of a single render span.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct RenderSpanDiagnostics {
    /// Path of the span.
    pub path: SpanPath,
    /// Kind of the span.
    pub kind: SpanKind,
    /// CPU time spent during the duration of the span.
    pub elapsed_cpu: Option<Duration>,
    /// GPU time spent executing commands recorded inside the span.
    pub elapsed_gpu: Option<Duration>,
    /// Amount of times the vertex shader is ran.
    /// Accounts for the vertex cache when doing indexed rendering.
    pub vertex_shader_invocations: Option<u64>,
    /// Amount of times the clipper is invoked.
    /// This is also the amount of triangles output by the vertex shader.
    pub clipper_invocations: Option<u64>,
    /// Amount of primitives that are not culled by the clipper.
    /// This is the amount of triangles that are actually on screen and will be rasterized and rendered.
    pub clipper_primitives_out: Option<u64>,
    /// Amount of times the fragment shader is ran.
    /// Accounts for fragment shaders running in 2x2 blocks in order to get derivatives.
    pub fragment_shader_invocations: Option<u64>,
    /// Amount of times a compute shader is invoked.
    /// This will be equivalent to the dispatch count times the workgroup size.
    pub compute_shader_invocations: Option<u64>,
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum RenderDiagnosticKind {
    ElapsedCpu = 0,
    ElapsedGpu,
    VertexShaderInvocations,
    ClipperInvocations,
    ClipperPrimitivesOut,
    FragmentShaderInvocations,
    ComputeShaderInvocations,
}

/// Kinds of render diagnostic  spans.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum SpanKind {
    /// An explicit timestamp span.
    Timestamp,
    /// A [`RenderPass`]. Records timestamps, as well as pipeline statistics.
    RenderPass,
    /// A [`ComputePass`]. Records timestamps, as well as pipeline statistics.
    ComputePass,
}

#[derive(Debug, Default, Clone)]
pub struct SpanPath {
    components: SmallVec<[Cow<'static, str>; 2]>,
    hash: u64,
}

impl SpanPath {
    pub fn new(components: impl IntoIterator<Item = Cow<'static, str>>) -> SpanPath {
        let components: SmallVec<[Cow<'static, str>; 2]> = components.into_iter().collect();
        let mut hasher = AHasher::default();
        components.hash(&mut hasher);
        let hash = hasher.finish();
        SpanPath { components, hash }
    }

    pub fn components(&self) -> impl Iterator<Item = &str> + '_ {
        self.components.iter().map(|v| &**v)
    }

    pub fn diagnostic_id(&self, kind: RenderDiagnosticKind) -> DiagnosticId {
        DiagnosticId(Uuid::from_u64_pair(
            0x6140_553e_4b6a_4400 | u64::from(kind as u8),
            self.hash,
        ))
    }
}

impl Eq for SpanPath {}

impl PartialEq for SpanPath {
    fn eq(&self, other: &Self) -> bool {
        self.hash == other.hash && self.components == other.components
    }
}

impl Hash for SpanPath {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.hash.hash(state);
    }
}

/// Records diagnostics into [`QuerySet`]'s keeping track of the mapping between
/// spans and indices to the corresponding entries in the [`QuerySet`].
#[derive(Resource)]
pub struct DiagnosticsRecorder {
    timestamp_period: f32,
    features: Features,
    current_frame: Mutex<FrameData>,
    submitted_frames: Vec<FrameData>,
    finished_frames: Vec<FrameData>,
}

impl DiagnosticsRecorder {
    /// Creates the new `DiagnosticsRecorder`.
    pub fn new(device: &RenderDevice, queue: &Queue) -> DiagnosticsRecorder {
        let features = device.features();

        let timestamp_period = if features.contains(Features::TIMESTAMP_QUERY) {
            queue.get_timestamp_period()
        } else {
            0.0
        };

        DiagnosticsRecorder {
            timestamp_period,
            features,
            current_frame: Mutex::new(FrameData::new(device, features)),
            submitted_frames: Vec::new(),
            finished_frames: Vec::new(),
        }
    }

    /// Begins recording diagnostics for a new frame.
    pub fn begin_frame(&mut self) {
        let mut idx = 0;
        while idx < self.submitted_frames.len() {
            if self.submitted_frames[idx].run_mapped_callback(self.timestamp_period) {
                self.finished_frames
                    .push(self.submitted_frames.swap_remove(idx));
            } else {
                idx += 1;
            }
        }

        self.current_frame.get_mut().begin();
    }

    pub fn begin_time_span<E: WriteTimestamp>(
        &self,
        encoder: &mut E,
        span_name: impl Into<Cow<'static, str>>,
    ) {
        self.current_frame
            .lock()
            .begin_time_span(encoder, span_name.into());
    }

    pub fn end_time_span<E: WriteTimestamp>(&self, encoder: &mut E) {
        self.current_frame.lock().end_time_span(encoder);
    }

    pub fn begin_pass_span<P: Pass>(&self, pass: &mut P, span_name: impl Into<Cow<'static, str>>) {
        self.current_frame.lock().begin_pass(pass, span_name.into());
    }

    pub fn end_pass_span<P: Pass>(&self, pass: &mut P) {
        self.current_frame.lock().end_pass(pass);
    }

    /// Copies data from [`QuerySet`]'s to a [`Buffer`], after which it can be downloaded to CPU.
    ///
    /// Should be called before [`DiagnosticsRecorder::finish_frame`]
    pub fn resolve(&mut self, encoder: &mut CommandEncoder) {
        self.current_frame.get_mut().resolve(encoder);
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
        self.current_frame.get_mut().finish(callback);

        // reuse one of the finished frames, if we can
        let new_frame = match self.finished_frames.pop() {
            Some(frame) => frame,
            None => FrameData::new(device, self.features),
        };

        let old_frame = std::mem::replace(self.current_frame.get_mut(), new_frame);
        self.submitted_frames.push(old_frame);
    }
}

#[derive(Clone)]
pub struct OptionalDiagnosticRecorder {
    recorder: Option<Arc<DiagnosticsRecorder>>,
}

impl From<Option<DiagnosticsRecorder>> for OptionalDiagnosticRecorder {
    fn from(recorder: Option<DiagnosticsRecorder>) -> Self {
        OptionalDiagnosticRecorder {
            recorder: recorder.map(Arc::new),
        }
    }
}

impl OptionalDiagnosticRecorder {
    pub(crate) fn unwrap(self) -> Option<DiagnosticsRecorder> {
        self.recorder.map(|v| Arc::try_unwrap(v).ok().unwrap())
    }

    pub fn time_span<E: WriteTimestamp>(
        &self,
        encoder: &mut E,
        span_name: impl Into<Cow<'static, str>>,
    ) -> TimeSpanScope<E> {
        if let Some(recorder) = &self.recorder {
            recorder.begin_time_span(encoder, span_name);
            TimeSpanScope::new(recorder)
        } else {
            TimeSpanScope::new_noop()
        }
    }

    pub fn pass_span<P: Pass>(
        &self,
        pass: &mut P,
        span_name: impl Into<Cow<'static, str>>,
    ) -> PassSpanScope<P> {
        if let Some(recorder) = &self.recorder {
            recorder.begin_pass_span(pass, span_name);
            PassSpanScope::new(recorder)
        } else {
            PassSpanScope::new_noop()
        }
    }
}

pub struct TimeSpanScope<'a, E: WriteTimestamp> {
    recorder: Option<&'a DiagnosticsRecorder>,
    marker: PhantomData<E>,
}

impl<E: WriteTimestamp> TimeSpanScope<'_, E> {
    pub fn new(recorder: &DiagnosticsRecorder) -> TimeSpanScope<'_, E> {
        TimeSpanScope {
            recorder: Some(recorder),
            marker: PhantomData,
        }
    }

    pub fn new_noop() -> TimeSpanScope<'static, E> {
        TimeSpanScope {
            recorder: None,
            marker: PhantomData,
        }
    }

    pub fn end(self, encoder: &mut E) {
        if let Some(recorder) = &self.recorder {
            recorder.current_frame.lock().end_time_span(encoder);
        }
        std::mem::forget(self)
    }
}

impl<E: WriteTimestamp> Drop for TimeSpanScope<'_, E> {
    fn drop(&mut self) {
        panic!("TimeSpanScope::end was never called")
    }
}

pub struct PassSpanScope<'a, P: Pass> {
    recorder: Option<&'a DiagnosticsRecorder>,
    marker: PhantomData<P>,
}

impl<P: Pass> PassSpanScope<'_, P> {
    pub fn new(recorder: &DiagnosticsRecorder) -> PassSpanScope<'_, P> {
        PassSpanScope {
            recorder: Some(recorder),
            marker: PhantomData,
        }
    }

    pub fn new_noop() -> PassSpanScope<'static, P> {
        PassSpanScope {
            recorder: None,
            marker: PhantomData,
        }
    }

    pub fn end(self, pass: &mut P) {
        if let Some(recorder) = &self.recorder {
            recorder.current_frame.lock().end_pass(pass);
        }
        std::mem::forget(self)
    }
}

impl<P: Pass> Drop for PassSpanScope<'_, P> {
    fn drop(&mut self) {
        panic!("PassSpanScope::end was never called")
    }
}

struct SpanRecord {
    thread_id: ThreadId,
    path_range: Range<usize>,
    kind: SpanKind,
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
        if is_inside_pass && !self.supports_timestamps_inside_passes {
            return None;
        }

        match &self.timestamps_query_set {
            Some(set) if self.num_timestamps < MAX_TIMESTAMP_QUERIES => {
                let index = self.num_timestamps;
                encoder.write_timestamp(set, index);
                self.num_timestamps += 1;
                Some(index)
            }
            _ => None,
        }
    }

    fn write_pipeline_statistics(
        &mut self,
        encoder: &mut impl WritePipelineStatistics,
    ) -> Option<u32> {
        match &self.pipeline_statistics_query_set {
            Some(set) if self.num_pipeline_statistics < MAX_PIPELINE_STATISTICS => {
                let index = self.num_pipeline_statistics;
                encoder.begin_pipeline_statistics_query(set, index);
                self.num_pipeline_statistics += 1;
                Some(index)
            }
            _ => None,
        }
    }

    fn open_span(&mut self, kind: SpanKind, name: Cow<'static, str>) -> &mut SpanRecord {
        let thread_id = thread::current().id();

        let parent = self
            .open_spans
            .iter()
            .filter(|v| v.thread_id == thread_id)
            .max_by_key(|v| v.path_range.len());

        let path_range = match &parent {
            Some(parent) if parent.path_range.end == self.path_components.len() => {
                parent.path_range.start..parent.path_range.end + 1
            }
            Some(parent) => {
                self.path_components
                    .extend_from_within(parent.path_range.clone());
                self.path_components.len()..self.path_components.len() + parent.path_range.len() + 1
            }
            None => self.path_components.len()..self.path_components.len() + 1,
        };

        self.path_components.push(name);

        self.open_spans.push(SpanRecord {
            thread_id,
            path_range,
            kind,
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
            .max_by_key(|(_, v)| v.path_range.len())
            .unwrap();

        let span = self.open_spans.swap_remove(index);
        self.closed_spans.push(span);
        self.closed_spans.last_mut().unwrap()
    }

    fn begin_time_span(&mut self, encoder: &mut impl WriteTimestamp, name: Cow<'static, str>) {
        let begin_instant = Instant::now();
        let begin_timestamp_index = self.write_timestamp(encoder, false);

        let span = self.open_span(SpanKind::Timestamp, name);
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

        let kind = match P::KIND {
            PassKind::Render => SpanKind::RenderPass,
            PassKind::Compute => SpanKind::ComputePass,
        };

        let span = self.open_span(kind, name);
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
        let (Some(resolve_buffer), Some(read_buffer)) = (&self.resolve_buffer, &self.read_buffer)
        else {
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

        encoder.copy_buffer_to_buffer(resolve_buffer, 0, read_buffer, 0, self.buffer_size);
    }

    fn finish(&mut self, callback: impl FnOnce(RenderDiagnostics) + Send + Sync + 'static) {
        let Some(read_buffer) = &self.read_buffer else {
            // we still have cpu timings, so let's use them

            let diagnostics = self.closed_spans.iter().map(|span| RenderSpanDiagnostics {
                path: SpanPath::new(
                    self.path_components[span.path_range.clone()]
                        .iter()
                        .cloned(),
                ),
                kind: span.kind,
                elapsed_cpu: match (span.begin_instant, span.end_instant) {
                    (Some(begin), Some(end)) => Some(end - begin),
                    _ => None,
                },
                elapsed_gpu: None,
                vertex_shader_invocations: None,
                clipper_invocations: None,
                clipper_primitives_out: None,
                fragment_shader_invocations: None,
                compute_shader_invocations: None,
            });

            callback(RenderDiagnostics(diagnostics.collect()));
            return;
        };

        self.callback = Some(Box::new(callback));

        let is_mapped = self.is_mapped.clone();
        read_buffer.slice(..).map_async(MapMode::Read, move |res| {
            if let Err(e) = res {
                bevy_log::warn!("Failed to download render statistics buffer: {e}");
                return;
            }

            is_mapped.store(true, Ordering::Release);
        });
    }

    // returns true if the frame is considered finished, false otherwise
    fn run_mapped_callback(&mut self, timestamp_period: f32) -> bool {
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

        let diagnostics = self.closed_spans.iter().map(|span| {
            let mut diagnostics = RenderSpanDiagnostics {
                path: SpanPath::new(
                    self.path_components[span.path_range.clone()]
                        .iter()
                        .cloned(),
                ),
                kind: span.kind,
                elapsed_cpu: match (span.begin_instant, span.end_instant) {
                    (Some(begin), Some(end)) => Some(end - begin),
                    _ => None,
                },
                elapsed_gpu: match (span.begin_timestamp_index, span.end_timestamp_index) {
                    (Some(begin), Some(end)) => {
                        let begin = timestamps[begin as usize] as f64;
                        let end = timestamps[end as usize] as f64;
                        let nanos = ((end - begin) * (timestamp_period as f64)).round() as u64;
                        Some(Duration::from_nanos(nanos))
                    }
                    _ => None,
                },
                vertex_shader_invocations: None,
                clipper_invocations: None,
                clipper_primitives_out: None,
                fragment_shader_invocations: None,
                compute_shader_invocations: None,
            };

            if let Some(index) = span.pipeline_statistics_index {
                let index = (index as usize) * 5;
                if span.kind == SpanKind::RenderPass {
                    diagnostics.vertex_shader_invocations = Some(pipeline_statistics[index]);
                    diagnostics.clipper_invocations = Some(pipeline_statistics[index + 1]);
                    diagnostics.clipper_primitives_out = Some(pipeline_statistics[index + 2]);
                    diagnostics.fragment_shader_invocations = Some(pipeline_statistics[index + 3]);
                } else {
                    diagnostics.compute_shader_invocations = Some(pipeline_statistics[index + 4]);
                }
            }

            diagnostics
        });

        callback(RenderDiagnostics(diagnostics.collect()));

        drop(data);
        read_buffer.unmap();
        self.is_mapped.store(false, Ordering::Release);

        true
    }
}

/// Stores [`RenderDiagnostics`] shared between render app and main app.
///
/// This mutex is locked twice per frame: in `PreUpdate`, during [`sync_render_statistics`],
/// and after rendering has finished and statistics have been downloaded from GPU.
#[derive(Debug, Default, Clone, Resource)]
pub struct RenderDiagnosticsMutex(pub Arc<Mutex<Option<RenderDiagnostics>>>);

/// Copies fresh [`RenderDiagnostics`] from [`RenderDiagnosticsMutex`].
pub fn sync_render_diagnostics(
    mutex: Res<RenderDiagnosticsMutex>,
    mut diagnostics: ResMut<RenderDiagnostics>,
) {
    if let Some(v) = mutex.0.lock().take() {
        dbg!(&v);
        *diagnostics = v;
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

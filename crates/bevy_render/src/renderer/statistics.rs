use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use bevy_app::{App, Plugin, PreUpdate};
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::system::{Res, ResMut, Resource};
use bevy_utils::{Duration, HashMap, Instant};
use parking_lot::Mutex;
use wgpu::{
    Buffer, BufferDescriptor, BufferUsages, CommandEncoder, Features, MapMode,
    PipelineStatisticsTypes, QuerySet, QuerySetDescriptor, QueryType, Queue, RenderPass,
    RenderPassDescriptor,
};

use crate::RenderApp;

use super::{RenderDevice, RenderQueue};

// buffer offset must be divisible by 256, so this constant must be divisible by 32 (=256/8)
const MAX_TIMESTAMP_QUERIES: u32 = 256;
const MAX_PIPELINE_STATISTICS: u32 = 128;

const TIMESTAMP_SIZE: u64 = 8;
const PIPELINE_STATISTICS_SIZE: u64 = 40;

/// Enables collecting render pass statistics into [`RenderStatistics`] resource.
///
/// # Supported platforms
/// Timestamp queries and pipeline statistics are currently supported only on Vulkan and DX12.
/// On other platforms (Metal, WebGPU, WebGL2) only CPU time will be recorded.
#[allow(clippy::doc_markdown)]
#[derive(Default)]
pub struct RenderStatisticsPlugin;

impl Plugin for RenderStatisticsPlugin {
    fn build(&self, app: &mut App) {
        let render_statistics_mutex = RenderStatisticsMutex::default();
        app.insert_resource(render_statistics_mutex.clone())
            .init_resource::<RenderStatistics>()
            .add_systems(PreUpdate, sync_render_statistics);

        if let Ok(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app.insert_resource(render_statistics_mutex);
        }
    }

    fn finish(&self, app: &mut App) {
        let Ok(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        let device = render_app.world.resource::<RenderDevice>();
        let queue = render_app.world.resource::<RenderQueue>();
        render_app.insert_resource(StatisticsRecorder::new(device, queue));
    }
}

/// Resource which stores statistics for each render pass.
#[derive(Debug, Default, Clone, Resource)]
pub struct RenderStatistics(pub HashMap<String, RenderPassStatistics>);

/// Statistics for a single render pass.
#[derive(Debug, Default, Clone, Copy, Eq, PartialEq, Hash)]
pub struct RenderPassStatistics {
    /// CPU time spent recording the [`RenderPass`].
    pub elapsed_cpu: Option<Duration>,
    /// GPU time spent executing commands inside the [`RenderPass`].
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

#[derive(Default)]
struct PassRecord {
    begin_timestamp_index: Option<u32>,
    end_timestamp_index: Option<u32>,
    begin_instant: Option<Instant>,
    end_instant: Option<Instant>,
    pipeline_statistics_index: Option<u32>,
}

/// Records statistics into [`QuerySet`]'s keeping track of the mapping between
/// render passes and indices to the corresponding statistics in the [`QuerySet`].
#[derive(Resource)]
pub struct StatisticsRecorder {
    timestamp_period: f32,
    features: Features,
    current_frame: FrameData,
    submitted_frames: Vec<FrameData>,
    finished_frames: Vec<FrameData>,
}

impl StatisticsRecorder {
    /// Creates the new `StatisticRecorder`.
    pub fn new(device: &RenderDevice, queue: &Queue) -> StatisticsRecorder {
        let features = device.features();

        let timestamp_period = if features.contains(Features::TIMESTAMP_QUERY) {
            queue.get_timestamp_period()
        } else {
            0.0
        };

        StatisticsRecorder {
            timestamp_period,
            features,
            current_frame: FrameData::new(device, features),
            submitted_frames: Vec::new(),
            finished_frames: Vec::new(),
        }
    }

    /// Begins recording statistics for a new frame.
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

        self.current_frame.begin();
    }

    fn begin_render_pass(&mut self, pass: &mut RenderPass, name: &str) {
        self.current_frame.begin_render_pass(pass, name);
    }

    fn end_render_pass(&mut self, pass: &mut RenderPass, name: &str) {
        self.current_frame.end_render_pass(pass, name);
    }

    /// Copies data from [`QuerySet`]'s to a [`Buffer`], after which it can be downloaded to CPU.
    ///
    /// Should be called before [`StatisticsRecorder::finish_frame`]
    pub fn resolve(&mut self, encoder: &mut CommandEncoder) {
        self.current_frame.resolve(encoder);
    }

    /// Finishes recording statistics for the current frame.
    ///
    /// The specified `callback` will be invoked when statistics become available.
    ///
    /// Should be called after [`StatisticsRecorder::resolve`],
    /// and **after** all commands buffers have been queued.
    pub fn finish_frame(
        &mut self,
        device: &RenderDevice,
        callback: impl FnOnce(RenderStatistics) + Send + Sync + 'static,
    ) {
        self.current_frame.finish(callback);

        // reuse one of the finished frames, if we can
        let new_frame = match self.finished_frames.pop() {
            Some(frame) => frame,
            None => FrameData::new(device, self.features),
        };

        let old_frame = std::mem::replace(&mut self.current_frame, new_frame);
        self.submitted_frames.push(old_frame);
    }
}

struct FrameData {
    timestamps_query_set: Option<QuerySet>,
    num_timestamps: u32,
    pipeline_statistics_query_set: Option<QuerySet>,
    num_pipeline_statistics: u32,
    buffer_size: u64,
    pipeline_statistics_buffer_offset: u64,
    resolve_buffer: Option<Buffer>,
    read_buffer: Option<Buffer>,
    pass_records: HashMap<String, PassRecord>,
    is_mapped: Arc<AtomicBool>,
    callback: Option<Box<dyn FnOnce(RenderStatistics) + Send + Sync + 'static>>,
}

impl FrameData {
    fn new(device: &RenderDevice, features: Features) -> FrameData {
        let wgpu_device = device.wgpu_device();
        let mut buffer_size = 0;

        let timestamps_query_set = if features.contains(Features::TIMESTAMP_QUERY_INSIDE_PASSES) {
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
            pipeline_statistics_query_set,
            num_pipeline_statistics: 0,
            buffer_size,
            pipeline_statistics_buffer_offset,
            resolve_buffer,
            read_buffer,
            pass_records: HashMap::default(),
            is_mapped: Arc::new(AtomicBool::new(false)),
            callback: None,
        }
    }

    fn begin(&mut self) {
        self.num_timestamps = 0;
        self.num_pipeline_statistics = 0;
        self.pass_records.clear();
    }

    fn pass_record(&mut self, name: &str) -> &mut PassRecord {
        self.pass_records.entry(name.into()).or_default()
    }

    fn begin_render_pass(&mut self, pass: &mut RenderPass, name: &str) {
        let begin_instant = Instant::now();

        let begin_timestamp_index = match &self.timestamps_query_set {
            Some(set) if self.num_timestamps < MAX_TIMESTAMP_QUERIES => {
                let index = self.num_timestamps;
                pass.write_timestamp(set, index);
                self.num_timestamps += 1;
                Some(index)
            }
            _ => None,
        };

        let pipeline_statistics_index = match &self.pipeline_statistics_query_set {
            Some(set) if self.num_pipeline_statistics < MAX_PIPELINE_STATISTICS => {
                let index = self.num_pipeline_statistics;
                pass.begin_pipeline_statistics_query(set, index);
                self.num_pipeline_statistics += 1;
                Some(index)
            }
            _ => None,
        };

        let record = self.pass_record(name);
        record.begin_instant = Some(begin_instant);
        record.begin_timestamp_index = begin_timestamp_index;
        record.pipeline_statistics_index = pipeline_statistics_index;
    }

    fn end_render_pass(&mut self, pass: &mut RenderPass, name: &str) {
        let end_timestamp_index = match &self.timestamps_query_set {
            Some(set) if self.num_timestamps < MAX_TIMESTAMP_QUERIES => {
                let index = self.num_timestamps;
                pass.write_timestamp(set, index);
                self.num_timestamps += 1;
                Some(index)
            }
            _ => None,
        };

        let record = self.pass_record(name);
        record.end_timestamp_index = end_timestamp_index;

        if record.pipeline_statistics_index.is_some() {
            pass.end_pipeline_statistics_query();
        }

        record.end_instant = Some(Instant::now());
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

    fn finish(&mut self, callback: impl FnOnce(RenderStatistics) + Send + Sync + 'static) {
        let Some(read_buffer) = &self.read_buffer else {
            // we still have cpu timings, so let's use them

            let statistics = self.pass_records.iter().map(|(name, record)| {
                let mut statistics = RenderPassStatistics::default();

                if let (Some(begin), Some(end)) = (record.begin_instant, record.end_instant) {
                    statistics.elapsed_cpu = Some(end - begin);
                }

                (name.clone(), statistics)
            });

            callback(RenderStatistics(statistics.collect()));
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

        let statistics = self.pass_records.iter().map(|(name, record)| {
            let mut statistics = RenderPassStatistics::default();

            if let (Some(begin), Some(end)) = (record.begin_instant, record.end_instant) {
                statistics.elapsed_cpu = Some(end - begin);
            }

            if let (Some(begin), Some(end)) =
                (record.begin_timestamp_index, record.end_timestamp_index)
            {
                let begin = timestamps[begin as usize] as f64;
                let end = timestamps[end as usize] as f64;
                let nanos = ((end - begin) * (timestamp_period as f64)).round() as u64;
                statistics.elapsed_gpu = Some(Duration::from_nanos(nanos));
            }

            if let Some(index) = record.pipeline_statistics_index {
                let index = (index as usize) * 5;
                statistics.vertex_shader_invocations = Some(pipeline_statistics[index]);
                statistics.clipper_invocations = Some(pipeline_statistics[index + 1]);
                statistics.clipper_primitives_out = Some(pipeline_statistics[index + 2]);
                statistics.fragment_shader_invocations = Some(pipeline_statistics[index + 3]);
                statistics.compute_shader_invocations = Some(pipeline_statistics[index + 4]);
            }

            (name.clone(), statistics)
        });

        callback(RenderStatistics(statistics.collect()));

        drop(data);
        read_buffer.unmap();
        self.is_mapped.store(false, Ordering::Release);

        true
    }
}

/// Wrapper around [`RenderPass`] which records pipeline statistics and timings.
///
/// [`RenderPassDescriptor`] must have a label, otherwise no statistics will be recorded.
#[derive(Deref, DerefMut)]
pub struct MeasuredRenderPass<'a> {
    #[deref]
    render_pass: RenderPass<'a>,
    name: Option<String>,
    recorder: Option<&'a mut StatisticsRecorder>,
}

impl MeasuredRenderPass<'_> {
    /// Begins recording a render pass, collecting the statistics into the given [`StatisticsRecorder`].
    ///
    /// [`RenderPassDescriptor`] must have a label, otherwise no statistics will be recorded.
    pub fn new<'a>(
        encoder: &'a mut CommandEncoder,
        mut recorder: Option<&'a mut StatisticsRecorder>,
        desc: RenderPassDescriptor<'a, '_>,
    ) -> MeasuredRenderPass<'a> {
        // copy label only if recording is enabled
        let name = recorder
            .as_ref()
            .and_then(|_| desc.label.map(|v| v.to_owned()));

        let mut render_pass = encoder.begin_render_pass(&desc);

        if let (Some(recorder), Some(name)) = (&mut recorder, &name) {
            recorder.begin_render_pass(&mut render_pass, name);
        }

        MeasuredRenderPass {
            render_pass,
            name,
            recorder,
        }
    }
}

impl Drop for MeasuredRenderPass<'_> {
    fn drop(&mut self) {
        if std::thread::panicking() {
            return;
        }

        if let (Some(recorder), Some(name)) = (&mut self.recorder, &self.name) {
            recorder.end_render_pass(&mut self.render_pass, name);
        }
    }
}

impl std::fmt::Debug for MeasuredRenderPass<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MeasuredRenderPass")
            .field("render_pass", &self.render_pass)
            .finish_non_exhaustive()
    }
}

/// Stores [`RenderStatistics`] shared between render app and main app.
///
/// This mutex is locked twice per frame: in `PreUpdate`, during [`sync_render_statistics`],
/// and after rendering has finished and statistics have been downloaded from GPU.
#[derive(Debug, Default, Clone, Resource)]
pub struct RenderStatisticsMutex(pub Arc<Mutex<Option<RenderStatistics>>>);

/// Copies fresh [`RenderStatistics`] from [`RenderStatisticsMutex`].
pub fn sync_render_statistics(
    mutex: Res<RenderStatisticsMutex>,
    mut statistics: ResMut<RenderStatistics>,
) {
    if let Some(v) = mutex.0.lock().take() {
        *statistics = v;
    }
}

use std::sync::Arc;

use bevy_derive::{Deref, DerefMut};
use bevy_ecs::system::{Res, ResMut, Resource};
use bevy_utils::{Duration, HashMap, Instant};
use parking_lot::Mutex;
use wgpu::{
    util::DownloadBuffer, Buffer, BufferDescriptor, BufferUsages, CommandEncoder,
    PipelineStatisticsTypes, QuerySet, QuerySetDescriptor, QueryType, Queue, RenderPass,
    RenderPassDescriptor,
};

use super::RenderDevice;

const MAX_TIMESTAMP_QUERIES: u32 = 256;
const MAX_PIPELINE_STATISTICS: u32 = 128;

#[derive(Debug, Default, Clone, Resource)]
pub struct RenderStatistics(pub HashMap<String, RenderPassStatistics>);

#[derive(Debug, Default, Clone, Copy, Eq, PartialEq, Hash)]
pub struct RenderPassStatistics {
    pub elapsed_cpu: Option<Duration>,
    pub elapsed_gpu: Option<Duration>,
    pub vertex_shader_invocations: Option<u64>,
    pub clipper_invocations: Option<u64>,
    pub clipper_primitives_out: Option<u64>,
    pub fragment_shader_invocations: Option<u64>,
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

pub struct StatisticsRecorder {
    timestamp_period: f32,
    timestamps_query_set: QuerySet,
    num_timestamps: u32,
    pipeline_statistics_query_set: QuerySet,
    num_pipeline_statistics: u32,
    pass_records: HashMap<String, PassRecord>,
    buffer: Option<Buffer>,
}

impl StatisticsRecorder {
    pub fn new(device: &RenderDevice, queue: &Queue) -> StatisticsRecorder {
        let timestamp_period = queue.get_timestamp_period();

        let timestamps_query_set = device.wgpu_device().create_query_set(&QuerySetDescriptor {
            label: Some("timestamps_query_set"),
            ty: QueryType::Timestamp,
            count: MAX_TIMESTAMP_QUERIES,
        });

        let pipeline_statistics_query_set =
            device.wgpu_device().create_query_set(&QuerySetDescriptor {
                label: Some("pipeline_statistics_query_set"),
                ty: QueryType::PipelineStatistics(PipelineStatisticsTypes::all()),
                count: MAX_PIPELINE_STATISTICS,
            });

        StatisticsRecorder {
            timestamp_period,
            timestamps_query_set,
            num_timestamps: 0,
            pipeline_statistics_query_set,
            num_pipeline_statistics: 0,
            pass_records: HashMap::default(),
            buffer: None,
        }
    }

    fn pass_record(&mut self, name: &str) -> &mut PassRecord {
        self.pass_records.entry(name.into()).or_default()
    }

    pub fn begin_render_pass(&mut self, pass: &mut RenderPass, name: &str) {
        let begin_instant = Instant::now();

        let begin_timestamp_index = if self.num_timestamps < MAX_TIMESTAMP_QUERIES {
            let index = self.num_timestamps;
            pass.write_timestamp(&self.timestamps_query_set, index);
            self.num_timestamps += 1;
            Some(index)
        } else {
            None
        };

        let pipeline_statistics_index = if self.num_pipeline_statistics < MAX_PIPELINE_STATISTICS {
            let index = self.num_pipeline_statistics;
            pass.begin_pipeline_statistics_query(&self.pipeline_statistics_query_set, index);
            self.num_pipeline_statistics += 1;
            Some(index)
        } else {
            None
        };

        let record = self.pass_record(name);
        record.begin_instant = Some(begin_instant);
        record.begin_timestamp_index = begin_timestamp_index;
        record.pipeline_statistics_index = pipeline_statistics_index;
    }

    pub fn end_render_pass(&mut self, pass: &mut RenderPass, name: &str) {
        let end_timestamp_index = if self.num_timestamps < MAX_TIMESTAMP_QUERIES {
            let index = self.num_timestamps;
            pass.write_timestamp(&self.timestamps_query_set, index);
            self.num_timestamps += 1;
            Some(index)
        } else {
            None
        };

        let record = self.pass_record(name);
        record.end_timestamp_index = end_timestamp_index;

        if record.pipeline_statistics_index.is_some() {
            pass.end_pipeline_statistics_query();
        }

        record.end_instant = Some(Instant::now());
    }

    fn buffer_size(&self) -> (u64, u64) {
        // timestamps are stored as u64
        let mut buffer_size = u64::from(self.num_timestamps) * 8;
        if buffer_size % 256 != 0 {
            buffer_size = buffer_size + 256 - buffer_size % 256;
        }

        let pipeline_statistics_offset = buffer_size;

        // pipeline statistics are stored as [u64; 5]
        buffer_size += u64::from(self.num_pipeline_statistics) * 40;

        (buffer_size, pipeline_statistics_offset)
    }

    pub fn resolve(&mut self, encoder: &mut CommandEncoder, device: &RenderDevice) {
        let (buffer_size, pipeline_statistics_offset) = self.buffer_size();

        let buffer = device.wgpu_device().create_buffer(&BufferDescriptor {
            label: Some("download_statistics_bufer"),
            size: buffer_size,
            usage: BufferUsages::COPY_SRC | BufferUsages::QUERY_RESOLVE,
            mapped_at_creation: false,
        });

        if self.num_timestamps > 0 {
            encoder.resolve_query_set(
                &self.timestamps_query_set,
                0..self.num_timestamps,
                &buffer,
                0,
            );
        }

        if self.num_pipeline_statistics > 0 {
            encoder.resolve_query_set(
                &self.pipeline_statistics_query_set,
                0..self.num_pipeline_statistics,
                &buffer,
                pipeline_statistics_offset,
            );
        }

        self.buffer = Some(buffer);
    }

    pub fn download(
        &mut self,
        device: &RenderDevice,
        queue: &Queue,
        callback: impl FnOnce(RenderStatistics) + Send + 'static,
    ) {
        let (_, pipeline_statistics_offset) = self.buffer_size();
        let timestamp_period = self.timestamp_period;
        let num_timestamps = self.num_timestamps;
        let num_pipeline_statistics = self.num_pipeline_statistics;
        let pass_records = std::mem::take(&mut self.pass_records);

        let Some(buffer) = &self.buffer else { return };
        DownloadBuffer::read_buffer(device.wgpu_device(), queue, &buffer.slice(..), move |res| {
            let buffer = match res {
                Ok(v) => v,
                Err(e) => {
                    bevy_log::warn!("Failed to download render statistics buffer: {e}");
                    return;
                }
            };

            let timestamps = buffer[..(num_timestamps * 8) as usize]
                .chunks(8)
                .map(|v| u64::from_ne_bytes(v.try_into().unwrap()))
                .collect::<Vec<u64>>();

            let start = pipeline_statistics_offset as usize;
            let len = (num_pipeline_statistics as usize) * 40;
            let pipeline_statistics = buffer[start..start + len]
                .chunks(8)
                .map(|v| u64::from_ne_bytes(v.try_into().unwrap()))
                .collect::<Vec<u64>>();

            let statistics = pass_records.into_iter().map(|(name, record)| {
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

                (name, statistics)
            });

            callback(RenderStatistics(statistics.collect()));
        });
    }
}

#[derive(Deref, DerefMut)]
pub struct MeasuredRenderPass<'a> {
    #[deref]
    render_pass: RenderPass<'a>,
    name: Option<String>,
    recorder: &'a mut StatisticsRecorder,
}

impl MeasuredRenderPass<'_> {
    pub fn new<'a>(
        encoder: &'a mut CommandEncoder,
        recorder: &'a mut StatisticsRecorder,
        desc: RenderPassDescriptor<'a, '_>,
    ) -> MeasuredRenderPass<'a> {
        let name = desc.label.map(|v| v.to_owned());
        let mut render_pass = encoder.begin_render_pass(&desc);

        if let Some(name) = &name {
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

        if let Some(name) = &self.name {
            self.recorder.end_render_pass(&mut self.render_pass, name);
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

#[derive(Debug, Default, Clone, Resource)]
pub struct RenderStatisticsMutex(pub Arc<Mutex<Option<RenderStatistics>>>);

pub fn sync_render_statistics(
    mutex: Res<RenderStatisticsMutex>,
    mut statistics: ResMut<RenderStatistics>,
) {
    if let Some(v) = mutex.0.lock().take() {
        *statistics = v;
    }
}

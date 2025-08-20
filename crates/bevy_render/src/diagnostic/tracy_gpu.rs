use crate::renderer::{RenderAdapterInfo, RenderDevice, RenderQueue};
use tracy_client::{Client, GpuContext, GpuContextType};
use wgpu::{
    Backend, BufferDescriptor, BufferUsages, CommandEncoderDescriptor, MapMode, PollType,
    QuerySetDescriptor, QueryType, QUERY_SIZE,
};

pub fn new_tracy_gpu_context(
    adapter_info: &RenderAdapterInfo,
    device: &RenderDevice,
    queue: &RenderQueue,
) -> GpuContext {
    let tracy_gpu_backend = match adapter_info.backend {
        Backend::Vulkan => GpuContextType::Vulkan,
        Backend::Dx12 => GpuContextType::Direct3D12,
        Backend::Gl => GpuContextType::OpenGL,
        Backend::Metal | Backend::BrowserWebGpu | Backend::Noop => GpuContextType::Invalid,
    };

    let tracy_client = Client::running().unwrap();
    tracy_client
        .new_gpu_context(
            Some("RenderQueue"),
            tracy_gpu_backend,
            initial_timestamp(device, queue),
            queue.get_timestamp_period(),
        )
        .unwrap()
}

// Code copied from https://github.com/Wumpf/wgpu-profiler/blob/f9de342a62cb75f50904a98d11dd2bbeb40ceab8/src/tracy.rs
fn initial_timestamp(device: &RenderDevice, queue: &RenderQueue) -> i64 {
    let query_set = device.wgpu_device().create_query_set(&QuerySetDescriptor {
        label: None,
        ty: QueryType::Timestamp,
        count: 1,
    });

    let resolve_buffer = device.create_buffer(&BufferDescriptor {
        label: None,
        size: QUERY_SIZE as _,
        usage: BufferUsages::QUERY_RESOLVE | BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });

    let map_buffer = device.create_buffer(&BufferDescriptor {
        label: None,
        size: QUERY_SIZE as _,
        usage: BufferUsages::MAP_READ | BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    let mut timestamp_encoder = device.create_command_encoder(&CommandEncoderDescriptor::default());
    timestamp_encoder.write_timestamp(&query_set, 0);
    timestamp_encoder.resolve_query_set(&query_set, 0..1, &resolve_buffer, 0);
    // Workaround for https://github.com/gfx-rs/wgpu/issues/6406
    // TODO when that bug is fixed, merge these encoders together again
    let mut copy_encoder = device.create_command_encoder(&CommandEncoderDescriptor::default());
    copy_encoder.copy_buffer_to_buffer(&resolve_buffer, 0, &map_buffer, 0, Some(QUERY_SIZE as _));
    queue.submit([timestamp_encoder.finish(), copy_encoder.finish()]);

    map_buffer.slice(..).map_async(MapMode::Read, |_| ());
    device
        .poll(PollType::Wait)
        .expect("Failed to poll device for map async");

    let view = map_buffer.slice(..).get_mapped_range();
    i64::from_le_bytes((*view).try_into().unwrap())
}

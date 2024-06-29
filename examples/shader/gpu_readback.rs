//! A very simple compute shader that updates a gpu buffer.
//! That buffer is then copied to the cpu and sent to the main world.
//!
//! This example is not meant to teach compute shaders.
//! It is only meant to explain how to read a gpu buffer on the cpu and then use it in the main world.
//!
//! The code is based on this wgpu example:
//! <https://github.com/gfx-rs/wgpu/blob/fb305b85f692f3fbbd9509b648dfbc97072f7465/examples/src/repeated_compute/mod.rs>

use bevy::{
    prelude::*,
    render::{
        render_graph::{self, RenderGraph, RenderLabel},
        render_resource::{binding_types::storage_buffer, *},
        renderer::{RenderContext, RenderDevice, RenderQueue},
        Render, RenderApp, RenderSet,
    },
};
use crossbeam_channel::{Receiver, Sender};

/// This example uses a shader source file from the assets subdirectory
const SHADER_ASSET_PATH: &str = "shaders/gpu_readback.wgsl";

// The length of the buffer sent to the gpu
const BUFFER_LEN: usize = 16;

// To communicate between the main world and the render world we need a channel.
// Since the main world and render world run in parallel, there will always be a frame of latency
// between the data sent from the render world and the data received in the main world
//
// frame n => render world sends data through the channel at the end of the frame
// frame n + 1 => main world receives the data

/// This will receive asynchronously any data sent from the render world
#[derive(Resource, Deref)]
struct MainWorldReceiver(Receiver<Vec<u32>>);

/// This will send asynchronously any data to the main world
#[derive(Resource, Deref)]
struct RenderWorldSender(Sender<Vec<u32>>);

fn main() {
    App::new()
        .insert_resource(ClearColor(Color::BLACK))
        .add_plugins((DefaultPlugins, GpuReadbackPlugin))
        .add_systems(Update, receive)
        .run();
}

/// This system will poll the channel and try to get the data sent from the render world
fn receive(receiver: Res<MainWorldReceiver>) {
    // We don't want to block the main world on this,
    // so we use try_recv which attempts to receive without blocking
    if let Ok(data) = receiver.try_recv() {
        println!("Received data from render world: {data:?}");
    }
}

// We need a plugin to organize all the systems and render node required for this example
struct GpuReadbackPlugin;
impl Plugin for GpuReadbackPlugin {
    fn build(&self, _app: &mut App) {}

    // The render device is only accessible inside finish().
    // So we need to initialize render resources here.
    fn finish(&self, app: &mut App) {
        let (s, r) = crossbeam_channel::unbounded();
        app.insert_resource(MainWorldReceiver(r));

        let render_app = app.sub_app_mut(RenderApp);
        render_app
            .insert_resource(RenderWorldSender(s))
            .init_resource::<ComputePipeline>()
            .init_resource::<Buffers>()
            .add_systems(
                Render,
                (
                    prepare_bind_group
                        .in_set(RenderSet::PrepareBindGroups)
                        // We don't need to recreate the bind group every frame
                        .run_if(not(resource_exists::<GpuBufferBindGroup>)),
                    // We need to run it after the render graph is done
                    // because this needs to happen after submit()
                    map_and_read_buffer.after(RenderSet::Render),
                ),
            );

        // Add the compute node as a top level node to the render graph
        // This means it will only execute once per frame
        render_app
            .world_mut()
            .resource_mut::<RenderGraph>()
            .add_node(ComputeNodeLabel, ComputeNode::default());
    }
}

/// Holds the buffers that will be used to communicate between the cpu and gpu
#[derive(Resource)]
struct Buffers {
    /// The buffer that will be used by the compute shader
    ///
    /// In this example, we want to write a `Vec<u32>` to a `Buffer`. `BufferVec` is a wrapper around a `Buffer`
    /// that will make sure the data is correctly aligned for the gpu and will simplify uploading the data to the gpu.
    gpu_buffer: BufferVec<u32>,
    /// The buffer that will be read on the cpu.
    /// The `gpu_buffer` will be copied to this buffer every frame
    cpu_buffer: Buffer,
}

impl FromWorld for Buffers {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();
        let render_queue = world.resource::<RenderQueue>();

        // Create the buffer that will be accessed by the gpu
        let mut gpu_buffer = BufferVec::new(BufferUsages::STORAGE | BufferUsages::COPY_SRC);
        for _ in 0..BUFFER_LEN {
            // Init the buffer with zeroes
            gpu_buffer.push(0);
        }
        // Write the buffer so the data is accessible on the gpu
        gpu_buffer.write_buffer(render_device, render_queue);

        // For portability reasons, WebGPU draws a distinction between memory that is
        // accessible by the CPU and memory that is accessible by the GPU. Only
        // buffers accessible by the CPU can be mapped and accessed by the CPU and
        // only buffers visible to the GPU can be used in shaders. In order to get
        // data from the GPU, we need to use `CommandEncoder::copy_buffer_to_buffer` to
        // copy the buffer modified by the GPU into a mappable, CPU-accessible buffer
        let cpu_buffer = render_device.create_buffer(&BufferDescriptor {
            label: Some("readback_buffer"),
            size: (BUFFER_LEN * std::mem::size_of::<u32>()) as u64,
            usage: BufferUsages::MAP_READ | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Self {
            gpu_buffer,
            cpu_buffer,
        }
    }
}

#[derive(Resource)]
struct GpuBufferBindGroup(BindGroup);

fn prepare_bind_group(
    mut commands: Commands,
    pipeline: Res<ComputePipeline>,
    render_device: Res<RenderDevice>,
    buffers: Res<Buffers>,
) {
    let bind_group = render_device.create_bind_group(
        None,
        &pipeline.layout,
        &BindGroupEntries::single(
            buffers
                .gpu_buffer
                .binding()
                // We already did it when creating the buffer so this should never happen
                .expect("Buffer should have already been uploaded to the gpu"),
        ),
    );
    commands.insert_resource(GpuBufferBindGroup(bind_group));
}

#[derive(Resource)]
struct ComputePipeline {
    layout: BindGroupLayout,
    pipeline: CachedComputePipelineId,
}

impl FromWorld for ComputePipeline {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();
        let layout = render_device.create_bind_group_layout(
            None,
            &BindGroupLayoutEntries::single(
                ShaderStages::COMPUTE,
                storage_buffer::<Vec<u32>>(false),
            ),
        );
        let shader = world.load_asset(SHADER_ASSET_PATH);
        let pipeline_cache = world.resource::<PipelineCache>();
        let pipeline = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
            label: Some("GPU readback compute shader".into()),
            layout: vec![layout.clone()],
            push_constant_ranges: Vec::new(),
            shader: shader.clone(),
            shader_defs: Vec::new(),
            entry_point: "main".into(),
        });
        ComputePipeline { layout, pipeline }
    }
}

fn map_and_read_buffer(
    render_device: Res<RenderDevice>,
    buffers: Res<Buffers>,
    sender: Res<RenderWorldSender>,
) {
    // Finally time to get our data back from the gpu.
    // First we get a buffer slice which represents a chunk of the buffer (which we
    // can't access yet).
    // We want the whole thing so use unbounded range.
    let buffer_slice = buffers.cpu_buffer.slice(..);

    // Now things get complicated. WebGPU, for safety reasons, only allows either the GPU
    // or CPU to access a buffer's contents at a time. We need to "map" the buffer which means
    // flipping ownership of the buffer over to the CPU and making access legal. We do this
    // with `BufferSlice::map_async`.
    //
    // The problem is that map_async is not an async function so we can't await it. What
    // we need to do instead is pass in a closure that will be executed when the slice is
    // either mapped or the mapping has failed.
    //
    // The problem with this is that we don't have a reliable way to wait in the main
    // code for the buffer to be mapped and even worse, calling get_mapped_range or
    // get_mapped_range_mut prematurely will cause a panic, not return an error.
    //
    // Using channels solves this as awaiting the receiving of a message from
    // the passed closure will force the outside code to wait. It also doesn't hurt
    // if the closure finishes before the outside code catches up as the message is
    // buffered and receiving will just pick that up.
    //
    // It may also be worth noting that although on native, the usage of asynchronous
    // channels is wholly unnecessary, for the sake of portability to WASM
    // we'll use async channels that work on both native and WASM.

    let (s, r) = crossbeam_channel::unbounded::<()>();

    // Maps the buffer so it can be read on the cpu
    buffer_slice.map_async(MapMode::Read, move |r| match r {
        // This will execute once the gpu is ready, so after the call to poll()
        Ok(_) => s.send(()).expect("Failed to send map update"),
        Err(err) => panic!("Failed to map buffer {err}"),
    });

    // In order for the mapping to be completed, one of three things must happen.
    // One of those can be calling `Device::poll`. This isn't necessary on the web as devices
    // are polled automatically but natively, we need to make sure this happens manually.
    // `Maintain::Wait` will cause the thread to wait on native but not on WebGpu.

    // This blocks until the gpu is done executing everything
    render_device.poll(Maintain::wait()).panic_on_timeout();

    // This blocks until the buffer is mapped
    r.recv().expect("Failed to receive the map_async message");

    {
        let buffer_view = buffer_slice.get_mapped_range();
        let data = buffer_view
            .chunks(std::mem::size_of::<u32>())
            .map(|chunk| u32::from_ne_bytes(chunk.try_into().expect("should be a u32")))
            .collect::<Vec<u32>>();
        sender
            .send(data)
            .expect("Failed to send data to main world");
    }

    // We need to make sure all `BufferView`'s are dropped before we do what we're about
    // to do.
    // Unmap so that we can copy to the staging buffer in the next iteration.
    buffers.cpu_buffer.unmap();
}

/// Label to identify the node in the render graph
#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
struct ComputeNodeLabel;

/// The node that will execute the compute shader
#[derive(Default)]
struct ComputeNode {}
impl render_graph::Node for ComputeNode {
    fn run(
        &self,
        _graph: &mut render_graph::RenderGraphContext,
        render_context: &mut RenderContext,
        world: &World,
    ) -> Result<(), render_graph::NodeRunError> {
        let pipeline_cache = world.resource::<PipelineCache>();
        let pipeline = world.resource::<ComputePipeline>();
        let bind_group = world.resource::<GpuBufferBindGroup>();

        if let Some(init_pipeline) = pipeline_cache.get_compute_pipeline(pipeline.pipeline) {
            let mut pass =
                render_context
                    .command_encoder()
                    .begin_compute_pass(&ComputePassDescriptor {
                        label: Some("GPU readback compute pass"),
                        ..default()
                    });

            pass.set_bind_group(0, &bind_group.0, &[]);
            pass.set_pipeline(init_pipeline);
            pass.dispatch_workgroups(BUFFER_LEN as u32, 1, 1);
        }

        // Copy the gpu accessible buffer to the cpu accessible buffer
        let buffers = world.resource::<Buffers>();
        render_context.command_encoder().copy_buffer_to_buffer(
            buffers
                .gpu_buffer
                .buffer()
                .expect("Buffer should have already been uploaded to the gpu"),
            0,
            &buffers.cpu_buffer,
            0,
            (BUFFER_LEN * std::mem::size_of::<u32>()) as u64,
        );

        Ok(())
    }
}

//! A very simple compute shader that updates a gpu buffer.
//! That buffer is then copied to the cpu and sent to the main world.
//!
//! This example is not meant to teach compute shaders.
//! It is only meant to explain how to read a gpu buffer on the cpu and then use it in the main world.
//!
//! The code is based on this wgpu example:
//! <https://github.com/gfx-rs/wgpu/blob/fb305b85f692f3fbbd9509b648dfbc97072f7465/examples/src/repeated_compute/mod.rs>

use bevy::{prelude::*, render::render_resource::*};
use bevy_render::extract_component::ExtractComponent;
use bevy_render::extract_resource::{ExtractResource, ExtractResourcePlugin};
use bevy_render::gpu_readback::{Readback, ReadbackComplete};
use bevy_render::render_asset::{RenderAssetUsages, RenderAssets};
use bevy_render::render_graph::{RenderGraph, RenderLabel};
use bevy_render::render_resource::binding_types::{storage_buffer, texture_storage_2d};
use bevy_render::renderer::{RenderContext, RenderDevice};
use bevy_render::storage::{GpuShaderStorageBuffer, ShaderStorageBuffer};
use bevy_render::texture::GpuImage;
use bevy_render::{render_graph, Render, RenderApp, RenderSet};

/// This example uses a shader source file from the assets subdirectory
const SHADER_ASSET_PATH: &str = "shaders/gpu_readback.wgsl";

// The length of the buffer sent to the gpu
const BUFFER_LEN: usize = 16;

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            GpuReadbackPlugin,
            ExtractResourcePlugin::<ReadbackBuffer>::default(),
            ExtractResourcePlugin::<ReadbackImage>::default(),
        ))
        .insert_resource(ClearColor(Color::BLACK))
        .add_systems(Startup, setup)
        .run();
}

// We need a plugin to organize all the systems and render node required for this example
struct GpuReadbackPlugin;
impl Plugin for GpuReadbackPlugin {
    fn build(&self, _app: &mut App) {}

    fn finish(&self, app: &mut App) {
        let render_app = app.sub_app_mut(RenderApp);
        render_app.init_resource::<ComputePipeline>().add_systems(
            Render,
            prepare_bind_group
                .in_set(RenderSet::PrepareBindGroups)
                // We don't need to recreate the bind group every frame
                .run_if(not(resource_exists::<GpuBufferBindGroup>)),
        );

        // Add the compute node as a top level node to the render graph
        // This means it will only execute once per frame
        render_app
            .world_mut()
            .resource_mut::<RenderGraph>()
            .add_node(ComputeNodeLabel, ComputeNode::default());
    }
}

#[derive(Resource, ExtractResource, Clone)]
struct ReadbackBuffer(Handle<ShaderStorageBuffer>);

#[derive(Resource, ExtractResource, Clone)]
struct ReadbackImage(Handle<Image>);

fn setup(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut buffers: ResMut<Assets<ShaderStorageBuffer>>,
) {
    let buffer = vec![0u32; BUFFER_LEN];
    let mut buffer = ShaderStorageBuffer::from(buffer);
    // We need to enable the COPY_SRC usage so we can copy the buffer to the cpu
    buffer.buffer_description.usage |= BufferUsages::COPY_SRC;
    let buffer = buffers.add(buffer);

    let size = Extent3d {
        width: BUFFER_LEN as u32,
        height: 1,
        ..default()
    };
    let mut image = Image::new_fill(
        size,
        TextureDimension::D2,
        &[10, 0, 0, 0],
        TextureFormat::R32Uint,
        RenderAssetUsages::RENDER_WORLD,
    );
    // You need to set these texture usage flags in order to use the image as a render target
    image.texture_descriptor.usage |= TextureUsages::COPY_SRC | TextureUsages::STORAGE_BINDING;
    let image = images.add(image);

    commands
        .spawn((buffer.clone(), Readback))
        .observe(|trigger: Trigger<ReadbackComplete>| {
            info!("Buffer {:?}", trigger.event());
        });
    commands.insert_resource(ReadbackBuffer(buffer));

    commands
        .spawn((image.clone(), Readback))
        .observe(|trigger: Trigger<ReadbackComplete>| {
            info!("Image {:?}", trigger.event());
        });
    commands.insert_resource(ReadbackImage(image));
}

#[derive(Resource)]
struct GpuBufferBindGroup(BindGroup);

fn prepare_bind_group(
    mut commands: Commands,
    pipeline: Res<ComputePipeline>,
    render_device: Res<RenderDevice>,
    buffer: Res<ReadbackBuffer>,
    image: Res<ReadbackImage>,
    buffers: Res<RenderAssets<GpuShaderStorageBuffer>>,
    images: Res<RenderAssets<GpuImage>>,
) {
    let buffer = buffers.get(&buffer.0).unwrap();
    let image = images.get(&image.0).unwrap();
    let bind_group = render_device.create_bind_group(
        None,
        &pipeline.layout,
        &BindGroupEntries::sequential((
            buffer.buffer.as_entire_buffer_binding(),
            image.texture_view.into_binding(),
        )),
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
            &BindGroupLayoutEntries::sequential(
                ShaderStages::COMPUTE,
                (
                    storage_buffer::<Vec<u32>>(false),
                    texture_storage_2d(TextureFormat::R32Uint, StorageTextureAccess::WriteOnly),
                ),
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
        Ok(())
    }
}

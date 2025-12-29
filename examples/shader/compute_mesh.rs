//! Simple example demonstrating the use of the [`Readback`] component to read back data from the GPU
//! using both a storage buffer and texture.

use bevy::{
    asset::RenderAssetUsages,
    color::palettes::tailwind::RED_400,
    prelude::*,
    render::{
        extract_resource::{ExtractResource, ExtractResourcePlugin},
        gpu_readback::{Readback, ReadbackComplete},
        render_asset::RenderAssets,
        render_graph::{self, RenderGraph, RenderLabel},
        render_resource::{
            binding_types::{storage_buffer, texture_storage_2d},
            *,
        },
        renderer::{RenderContext, RenderDevice},
        storage::{GpuShaderStorageBuffer, ShaderStorageBuffer},
        texture::GpuImage,
        Render, RenderApp, RenderStartup, RenderSystems,
    },
};
use bevy_render::{
    extract_component::{ExtractComponent, ExtractComponentPlugin},
    mesh::{allocator::MeshAllocator, RenderMesh},
};

/// This example uses a shader source file from the assets subdirectory
const SHADER_ASSET_PATH: &str = "shaders/compute_mesh.wgsl";

// The length of the buffer sent to the gpu
const BUFFER_LEN: usize = 768;

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            GpuReadbackPlugin,
            ExtractResourcePlugin::<ComputedBuffers>::default(),
            ExtractComponentPlugin::<GenerateMesh>::default(),
        ))
        .insert_resource(ClearColor(Color::BLACK))
        .add_systems(Startup, setup)
        // .add_systems(Update, kick_meshes)
        .run();
}

fn kick_meshes(mut query: Query<&mut Mesh3d>) {
    for mesh in &mut query {}
}
// We need a plugin to organize all the systems and render node required for this example
struct GpuReadbackPlugin;
impl Plugin for GpuReadbackPlugin {
    fn build(&self, app: &mut App) {
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };
        render_app
            .init_resource::<Chunks>()
            .add_systems(
                RenderStartup,
                (init_compute_pipeline, add_compute_render_graph_node),
            )
            .add_systems(
                Render,
                (
                    prepare_bind_group
                        .in_set(RenderSystems::PrepareBindGroups)
                        // We don't need to recreate the bind group every frame
                        .run_if(not(resource_exists::<GpuBufferBindGroup>)),
                    prepare_chunks,
                ),
            );
    }
}

#[derive(Component, ExtractComponent, Clone)]
struct GenerateMesh(Handle<Mesh>);

#[derive(Resource, ExtractResource, Clone)]
struct ComputedBuffers {
    vertex: Handle<ShaderStorageBuffer>,
    index: Handle<ShaderStorageBuffer>,
}

fn setup(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut buffers: ResMut<Assets<ShaderStorageBuffer>>,
) {
    // a truly empty mesh will error if used in Mesh3d
    // so use a sphere for the example
    let mut empty_mesh = Cuboid::new(0.1, 0.1, 0.1).mesh().build();
    let num_indices = empty_mesh.indices().unwrap().len();
    info!(
        buffer_size=?empty_mesh.get_vertex_buffer_size(),
        vertex_size=?empty_mesh.get_vertex_size(),
        num_indices=?num_indices
    );
    empty_mesh.asset_usage = RenderAssetUsages::RENDER_WORLD;

    // Create a storage buffer with some data
    let buffer: Vec<f32> = vec![0.; BUFFER_LEN];
    let mut buffer = ShaderStorageBuffer::from(buffer);
    // We need to enable the COPY_SRC usage so we can copy the buffer to the cpu
    buffer.buffer_description.usage |= BufferUsages::COPY_SRC;
    let vertex_buffer = buffers.add(buffer);

    // Create a storage buffer with some data
    let buffer: Vec<u32> = vec![0; 36 * 32];
    let mut buffer = ShaderStorageBuffer::from(buffer);
    // We need to enable the COPY_SRC usage so we can copy the buffer to the cpu
    buffer.buffer_description.usage |= BufferUsages::COPY_SRC;
    let index_buffer = buffers.add(buffer);

    // Create a storage texture with some data
    let size = Extent3d {
        width: BUFFER_LEN as u32,
        height: 1,
        ..default()
    };

    commands.insert_resource(ComputedBuffers {
        vertex: vertex_buffer,
        index: index_buffer,
    });
    // let mut empty_mesh = Mesh::new(
    //     PrimitiveTopology::TriangleList,
    //     RenderAssetUsages::RENDER_WORLD,
    // );

    let handle = meshes.add(empty_mesh);
    commands.spawn((
        GenerateMesh(handle.clone()),
        Mesh3d(handle.clone()),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: RED_400.into(),
            ..default()
        })),
        Transform::from_xyz(0., 1., 0.),
    ));

    // commands.spawn((
    //     Mesh3d(handle),
    //     MeshMaterial3d(materials.add(StandardMaterial {
    //         base_color: RED_400.into(),
    //         ..default()
    //     })),
    //     Transform::from_xyz(2., 1., 0.),
    // ));

    // // spawn some scene
    // commands.spawn((
    //     Mesh3d(meshes.add(Circle::new(4.0))),
    //     MeshMaterial3d(materials.add(Color::WHITE)),
    //     Transform::from_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)),
    // ));
    commands.spawn((
        PointLight {
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(4.0, 8.0, 4.0),
    ));
    // camera
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(-2.5, 4.5, 9.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

fn add_compute_render_graph_node(mut render_graph: ResMut<RenderGraph>) {
    // Add the compute node as a top-level node to the render graph. This means it will only execute
    // once per frame. Normally, adding a node would use the `RenderGraphApp::add_render_graph_node`
    // method, but it does not allow adding as a top-level node.
    render_graph.add_node(ComputeNodeLabel, ComputeNode::default());
}

#[derive(Resource)]
struct GpuBufferBindGroup(BindGroup);

#[derive(Resource, Default)]
struct Chunks(Vec<AssetId<Mesh>>);

fn prepare_bind_group(
    mut commands: Commands,
    pipeline: Res<ComputePipeline>,
    render_device: Res<RenderDevice>,
    pipeline_cache: Res<PipelineCache>,
    computed_buffers: Res<ComputedBuffers>,
    buffers: Res<RenderAssets<GpuShaderStorageBuffer>>,
) {
    let vertex_buffer = buffers.get(&computed_buffers.vertex).unwrap();
    let index_buffer = buffers.get(&computed_buffers.index).unwrap();

    let bind_group = render_device.create_bind_group(
        None,
        &pipeline_cache.get_bind_group_layout(&pipeline.layout),
        &BindGroupEntries::sequential((
            vertex_buffer.buffer.as_entire_buffer_binding(),
            index_buffer.buffer.as_entire_buffer_binding(),
        )),
    );
    commands.insert_resource(GpuBufferBindGroup(bind_group));
}

fn prepare_chunks(
    meshes_to_generate: Query<&GenerateMesh>,
    mut chunks: ResMut<Chunks>,
    mesh_handles: Res<RenderAssets<RenderMesh>>,
) {
    let chunk_data: Vec<AssetId<Mesh>> = meshes_to_generate
        .iter()
        // sometimes RenderMesh doesn't exist yet!
        .map(|gmesh| gmesh.0.id())
        .collect();
    // dbg!(chunk_data);
    chunks.0 = chunk_data;
}

#[derive(Resource)]
struct ComputePipeline {
    layout: BindGroupLayoutDescriptor,
    pipeline: CachedComputePipelineId,
}

// init only happens once
fn init_compute_pipeline(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    pipeline_cache: Res<PipelineCache>,
) {
    let layout = BindGroupLayoutDescriptor::new(
        "",
        &BindGroupLayoutEntries::sequential(
            ShaderStages::COMPUTE,
            (
                // vertices
                storage_buffer::<Vec<u32>>(false),
                // indices
                storage_buffer::<Vec<u32>>(false),
            ),
        ),
    );
    let shader = asset_server.load(SHADER_ASSET_PATH);
    let pipeline = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
        label: Some("Mesh generation compute shader".into()),
        layout: vec![layout.clone()],
        shader: shader.clone(),
        ..default()
    });
    commands.insert_resource(ComputePipeline { layout, pipeline });
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
        let Some(chunks) = world.get_resource::<Chunks>() else {
            info!("no chunks");
            return Ok(());
        };
        for mesh_id in &chunks.0 {
            let pipeline_cache = world.resource::<PipelineCache>();
            let pipeline = world.resource::<ComputePipeline>();
            let bind_group = world.resource::<GpuBufferBindGroup>();

            if let Some(init_pipeline) = pipeline_cache.get_compute_pipeline(pipeline.pipeline) {
                let mut pass =
                    render_context
                        .command_encoder()
                        .begin_compute_pass(&ComputePassDescriptor {
                            label: Some("Mesh generation compute pass"),
                            ..default()
                        });

                pass.set_bind_group(0, &bind_group.0, &[]);
                pass.set_pipeline(init_pipeline);
                pass.dispatch_workgroups(1, 1, 1);
            }
            let computed_buffers = world.resource::<ComputedBuffers>();
            let buffers = world.resource::<RenderAssets<GpuShaderStorageBuffer>>();
            let mesh_allocator = world.resource::<MeshAllocator>();

            // these can be None, read the mesh allocator docs
            // to understand when.
            let (vertex, index) = mesh_allocator.mesh_slabs(&mesh_id);

            let vertex_data_from_shader = buffers.get(&computed_buffers.vertex).unwrap();
            let vertex_buffer_slice = mesh_allocator.mesh_vertex_slice(mesh_id).unwrap();
            info_once!(
                data_buffer_size=?vertex_data_from_shader.buffer.size(),
                range_start=?vertex_buffer_slice.range.start,
                range_end=?vertex_buffer_slice.range.end,
                "vertex",
            );

            render_context.command_encoder().copy_buffer_to_buffer(
                &vertex_data_from_shader.buffer,
                0,
                vertex_buffer_slice.buffer,
                0,
                // vertex_buffer_slice.range.start as u64,
                vertex_data_from_shader.buffer.size(),
            );

            let index_data_from_shader = buffers.get(&computed_buffers.index).unwrap();
            let index_buffer_slice = mesh_allocator.mesh_index_slice(mesh_id).unwrap();
            info_once!(
                data_buffer_size=?index_data_from_shader.buffer.size(),
                range_start=?index_buffer_slice.range.start,
                range_end=?index_buffer_slice.range.end,
                "index"
            );
            render_context.command_encoder().copy_buffer_to_buffer(
                &index_data_from_shader.buffer,
                0,
                index_buffer_slice.buffer,
                0,
                // index_buffer_slice.range.start as u64,
                index_data_from_shader.buffer.size(),
            );
        }

        Ok(())
    }
}

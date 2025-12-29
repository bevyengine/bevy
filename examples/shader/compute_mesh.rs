//! Simple example demonstrating the use of the [`Readback`] component to read back data from the GPU
//! using both a storage buffer and texture.

use bevy::{
    asset::RenderAssetUsages,
    color::palettes::tailwind::RED_400,
    mesh::{Indices, MeshVertexAttribute},
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
    render_resource::binding_types::uniform_buffer,
    renderer::RenderQueue,
};

/// This example uses a shader source file from the assets subdirectory
const SHADER_ASSET_PATH: &str = "shaders/compute_mesh.wgsl";

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            ComputeShaderMeshGeneratorPlugin,
            ExtractComponentPlugin::<GenerateMesh>::default(),
        ))
        .insert_resource(ClearColor(Color::BLACK))
        .add_systems(Startup, setup)
        .run();
}

// We need a plugin to organize all the systems and render node required for this example
struct ComputeShaderMeshGeneratorPlugin;
impl Plugin for ComputeShaderMeshGeneratorPlugin {
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
            .add_systems(Render, prepare_chunks);
    }
    fn finish(&self, app: &mut App) {
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };
        render_app
            .world_mut()
            .resource_mut::<MeshAllocator>()
            .extra_buffer_usages = BufferUsages::STORAGE;
    }
}

#[derive(Component, ExtractComponent, Clone)]
struct GenerateMesh(Handle<Mesh>);

fn setup(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut buffers: ResMut<Assets<ShaderStorageBuffer>>,
) {
    // a truly empty mesh will error if used in Mesh3d
    // so use a sphere for the example
    let mut empty_mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD,
    );
    // set up what we want to output from the compute shader.
    // We're using 36 indices, 24 vertices which is directly taken from
    // the Bevy Cuboid mesh
    empty_mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, vec![[0.; 3]; 24]);
    empty_mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0.; 3]; 24]);
    empty_mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, vec![[0.; 2]; 24]);
    empty_mesh.insert_indices(Indices::U32(vec![0; 36]));
    empty_mesh.asset_usage = RenderAssetUsages::RENDER_WORLD;

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

#[derive(Resource, Default)]
struct Chunks(Vec<AssetId<Mesh>>);

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
                uniform_buffer::<FirstIndex>(false),
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

#[derive(ShaderType)]
struct FirstIndex {
    first_vertex_index: u32,
    first_index_index: u32,
}

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
        let mesh_allocator = world.resource::<MeshAllocator>();

        for mesh_id in &chunks.0 {
            let pipeline_cache = world.resource::<PipelineCache>();
            let pipeline = world.resource::<ComputePipeline>();

            if let Some(init_pipeline) = pipeline_cache.get_compute_pipeline(pipeline.pipeline) {
                let vertex_buffer_slice = mesh_allocator.mesh_vertex_slice(mesh_id).unwrap();
                let index_buffer_slice = mesh_allocator.mesh_index_slice(mesh_id).unwrap();

                dbg!(&vertex_buffer_slice.range);
                dbg!(&index_buffer_slice.range);

                let first = FirstIndex {
                    first_vertex_index: vertex_buffer_slice.range.start * 4,
                    first_index_index: index_buffer_slice.range.start * 4,
                };
                let mut uniforms = UniformBuffer::from(first);
                uniforms.write_buffer(
                    render_context.render_device(),
                    world.resource::<RenderQueue>(),
                );
                let bind_group = render_context.render_device().create_bind_group(
                    None,
                    &pipeline_cache.get_bind_group_layout(&pipeline.layout),
                    &BindGroupEntries::sequential((
                        &uniforms,
                        vertex_buffer_slice.buffer.as_entire_buffer_binding(),
                        index_buffer_slice.buffer.as_entire_buffer_binding(),
                    )),
                );

                let mut pass =
                    render_context
                        .command_encoder()
                        .begin_compute_pass(&ComputePassDescriptor {
                            label: Some("Mesh generation compute pass"),
                            ..default()
                        });

                pass.set_bind_group(0, &bind_group, &[]);
                pass.set_pipeline(init_pipeline);
                pass.dispatch_workgroups(1, 1, 1);
            }
        }

        Ok(())
    }
}

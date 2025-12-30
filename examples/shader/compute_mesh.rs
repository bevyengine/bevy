//! This example shows how to initialize an empty mesh with a Handle
//! and a render-world only usage. That buffer is then filled by a
//! compute shader on the GPU without transferring data back
//! to the CPU.
//!
//! The `mesh_allocator` is used to get references to the relevant slabs
//! that contain the mesh data we're interested in.
//!
//! This example does not remove the `GenerateMesh` component after
//! generating the mesh.

use bevy::{
    asset::RenderAssetUsages,
    color::palettes::tailwind::{RED_400, SKY_400},
    mesh::Indices,
    prelude::*,
    render::{
        extract_component::{ExtractComponent, ExtractComponentPlugin},
        mesh::allocator::MeshAllocator,
        render_graph::{self, RenderGraph, RenderLabel},
        render_resource::{
            binding_types::{storage_buffer, uniform_buffer},
            *,
        },
        renderer::{RenderContext, RenderQueue},
        Render, RenderApp, RenderStartup,
    },
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
            // This allows using the mesh allocator slabs as
            // storage buffers directly in the compute shader.
            // Which means that we can write from our compute
            // shader directly to the allocated mesh slabs.
            .extra_buffer_usages = BufferUsages::STORAGE;
    }
}

/// Holds a handle to the empty mesh that should be filled
/// by the compute shader.
#[derive(Component, ExtractComponent, Clone)]
struct GenerateMesh(Handle<Mesh>);

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // a truly empty mesh will error if used in Mesh3d
    // so we set up the data to be what we want the compute shader to output
    // We're using 36 indices and 24 vertices which is directly taken from
    // the Bevy Cuboid mesh implementation.
    //
    // We allocate 50 spots for each attribute here because
    // it is *very important* that the amount of data allocated here is
    // *bigger* than (or exactly equal to) the amount of data we intend to
    // write from the compute shader. This amount of data defines how big
    // the buffer we get from the mesh_allocator will be, which in turn
    // defines how big the buffer is when we're in the compute shader.
    //
    // If it turns out you don't need all of the space when the compute shader
    // is writing data, you can write NaN to the rest of the data.
    let empty_mesh = {
        let mut mesh = Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::RENDER_WORLD,
        )
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, vec![[0.; 3]; 50])
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0.; 3]; 50])
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, vec![[0.; 2]; 50])
        .with_inserted_indices(Indices::U32(vec![0; 50]));

        mesh.asset_usage = RenderAssetUsages::RENDER_WORLD;
        mesh
    };

    let handle = meshes.add(empty_mesh);

    // we spawn two "users" of the mesh handle,
    // but only insert `GenerateMesh` on one of them
    // to show that the mesh handle works as usual
    commands.spawn((
        GenerateMesh(handle.clone()),
        Mesh3d(handle.clone()),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: RED_400.into(),
            ..default()
        })),
        Transform::from_xyz(-2.5, 1.5, 0.),
    ));

    commands.spawn((
        Mesh3d(handle),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: SKY_400.into(),
            ..default()
        })),
        Transform::from_xyz(2.5, 1.5, 0.),
    ));

    // some additional scene elements.
    // This mesh specifically is here so that we don't assume
    // mesh_allocator offsets that would only work if we had
    // one mesh in the scene.
    commands.spawn((
        Mesh3d(meshes.add(Circle::new(4.0))),
        MeshMaterial3d(materials.add(Color::WHITE)),
        Transform::from_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)),
    ));
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

/// This is called "Chunks" because this example originated
/// from a use case of generating chunks of landscape or voxels
#[derive(Resource, Default)]
struct Chunks(Vec<AssetId<Mesh>>);

fn prepare_chunks(meshes_to_generate: Query<&GenerateMesh>, mut chunks: ResMut<Chunks>) {
    // get the AssetId for each Handle<Mesh>
    // which we'll use later to get the relevant buffers
    // from the mesh_allocator
    let chunk_data: Vec<AssetId<Mesh>> = meshes_to_generate
        .iter()
        .map(|gmesh| gmesh.0.id())
        .collect();
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
                // offsets
                uniform_buffer::<DataRanges>(false),
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

// A uniform that holds the vertex and index offsets
// for the vertex/index mesh_allocator buffer slabs
#[derive(ShaderType)]
struct DataRanges {
    vertex_start: u32,
    vertex_end: u32,
    index_start: u32,
    index_end: u32,
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
                // the mesh_allocator holds slabs of meshes, so the buffers we get here
                // can contain more data than just the mesh we're asking for.
                // That's why there is a range field.
                // You should *not* touch data in these buffers that is outside of the range.
                let vertex_buffer_slice = mesh_allocator.mesh_vertex_slice(mesh_id).unwrap();
                let index_buffer_slice = mesh_allocator.mesh_index_slice(mesh_id).unwrap();

                let first = DataRanges {
                    // there are 8 vertex data values (pos, normal, uv) per vertex
                    // and the vertex_buffer_slice.range.start is in "vertex elements"
                    // which includes all of that data, so each index is worth 8 indices
                    // to our shader code.
                    vertex_start: vertex_buffer_slice.range.start * 8,
                    vertex_end: vertex_buffer_slice.range.end * 8,
                    // but each vertex index is a single value, so the index of the
                    // vertex indices is exactly what the value is
                    index_start: index_buffer_slice.range.start,
                    index_end: index_buffer_slice.range.end,
                };

                let mut uniforms = UniformBuffer::from(first);
                uniforms.write_buffer(
                    render_context.render_device(),
                    world.resource::<RenderQueue>(),
                );

                // pass in the full mesh_allocator slabs as well as the first index
                // offsets for the vertex and index buffers
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
                pass.push_debug_group("compute_mesh");

                pass.set_bind_group(0, &bind_group, &[]);
                pass.set_pipeline(init_pipeline);
                // we only dispatch 1,1,1 workgroup here, but a real compute shader
                // would take advantage of more and larger size workgroups
                pass.dispatch_workgroups(1, 1, 1);

                pass.pop_debug_group();
            }
        }

        Ok(())
    }
}

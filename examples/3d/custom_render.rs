use bevy::{
    core::AsBytes,
    prelude::*,
    render::{
        mesh::{INDEX_BUFFER_ASSET_INDEX, VERTEX_BUFFER_ASSET_INDEX},
        pipeline::{DynamicBinding, PipelineDescriptor, PipelineSpecialization, RenderPipeline},
        render_graph::{
            base, AssetRenderResourcesNode, CommandQueue, Node, RenderGraph, ResourceSlots,
            SystemNode,
        },
        renderer::{
            BufferId, BufferInfo, BufferUsage, RenderContext, RenderResourceBindings,
            RenderResourceContext, RenderResourceId, RenderResources,
        },
        shader::{ShaderStage, ShaderStages},
    },
};

/// This example illustrates how to create a custom material and mesh asset and a shader that uses that uses those assets
fn main() {
    App::build()
        .add_default_plugins()
        .add_asset::<MyMesh>()
        .add_asset::<MyMaterial>()
        .add_startup_system(setup.system())
        .run();
}

#[derive(Default)]
pub struct MyMesh;

#[derive(Default)]
pub struct MyMeshNode {
    command_queue: CommandQueue,
}

impl Node for MyMeshNode {
    fn update(
        &mut self,
        _world: &World,
        _resources: &Resources,
        render_context: &mut dyn RenderContext,
        _input: &ResourceSlots,
        _output: &mut ResourceSlots,
    ) {
        self.command_queue.execute(render_context);
    }
}

impl SystemNode for MyMeshNode {
    fn get_system(&self, commands: &mut Commands) -> Box<dyn System> {
        let system = my_mesh_node_system.system();
        commands.insert_local_resource(
            system.id(),
            MyMeshNodeState {
                command_queue: self.command_queue.clone(),
                ..Default::default()
            },
        );

        system
    }
}

#[derive(Debug, Copy, Clone)]
struct MyMeshBuffers {
    vertex: BufferId,
    index: BufferId,
    vertex_staging: BufferId,
    index_staging: BufferId,
}

impl MyMeshBuffers {
    pub fn new(
        render_resource_context: &dyn RenderResourceContext,
        vertex: BufferInfo,
        index: BufferInfo,
    ) -> Self {
        Self {
            vertex: render_resource_context.create_buffer(BufferInfo {
                buffer_usage: vertex.buffer_usage | BufferUsage::COPY_DST,
                mapped_at_creation: false,
                ..vertex
            }),
            index: render_resource_context.create_buffer(BufferInfo {
                buffer_usage: index.buffer_usage | BufferUsage::COPY_DST,
                mapped_at_creation: false,
                ..index
            }),
            vertex_staging: render_resource_context.create_buffer(BufferInfo {
                buffer_usage: BufferUsage::COPY_SRC | BufferUsage::MAP_WRITE,
                size: vertex.size,
                mapped_at_creation: false,
            }),
            index_staging: render_resource_context.create_buffer(BufferInfo {
                buffer_usage: BufferUsage::COPY_SRC | BufferUsage::MAP_WRITE,
                size: index.size,
                mapped_at_creation: false,
            }),
        }
    }
}

#[derive(Default)]
pub struct MyMeshNodeState {
    command_queue: CommandQueue,
    buffers: Option<MyMeshBuffers>,
}

#[derive(RenderResources, Default)]
pub struct MyMaterial {
    pub color: Color,
}

const VERTEX_SHADER: &str = r#"
#version 450
layout(location = 0) in vec3 Vertex_Position;
layout(set = 0, binding = 0) uniform Camera {
    mat4 ViewProj;
};
layout(set = 1, binding = 0) uniform Transform {
    mat4 Model;
};
void main() {
    gl_Position = ViewProj * Model * vec4(Vertex_Position, 1.0);
}
"#;

const FRAGMENT_SHADER: &str = r#"
#version 450
layout(location = 0) out vec4 o_Target;
layout(set = 1, binding = 1) uniform MyMaterial_color {
    vec4 color;
};
void main() {
    o_Target = color;
}
"#;

fn setup(
    mut commands: Commands,
    mut pipelines: ResMut<Assets<PipelineDescriptor>>,
    mut shaders: ResMut<Assets<Shader>>,
    mut meshes: ResMut<Assets<MyMesh>>,
    mut materials: ResMut<Assets<MyMaterial>>,
    mut render_graph: ResMut<RenderGraph>,
) {
    // Create a new shader pipeline
    let pipeline_handle = pipelines.add(PipelineDescriptor::default_config(ShaderStages {
        vertex: shaders.add(Shader::from_glsl(ShaderStage::Vertex, VERTEX_SHADER)),
        fragment: Some(shaders.add(Shader::from_glsl(ShaderStage::Fragment, FRAGMENT_SHADER))),
    }));

    // Add an AssetRenderResourcesNode to our Render Graph. This will bind MyMaterial resources to our shader
    render_graph.add_system_node(
        "my_material",
        AssetRenderResourcesNode::<MyMaterial>::new(true),
    );

    // Add a MyMeshNode to our Render Graph. This will allow us to bind custom vertices to our shader.
    render_graph.add_system_node("my_mesh", MyMeshNode::default());

    // Add a Render Graph edge connecting our new "my_material" node to the main pass node. This ensures "my_material" runs before the main pass
    render_graph
        .add_node_edge("my_material", base::node::MAIN_PASS)
        .unwrap();

    // Add a Render Graph edge connecting our new "my_mesh" node to the main pass node. This ensures "my_mesh" runs before the main pass
    render_graph
        .add_node_edge("my_mesh", base::node::MAIN_PASS)
        .unwrap();

    // Create a new material
    let material = materials.add(MyMaterial {
        color: Color::rgb(0.0, 0.8, 0.0),
    });

    let mesh = meshes.add(MyMesh {});

    // Setup our world
    commands
        // cube
        .spawn(MeshComponents {
            render_pipelines: RenderPipelines::from_pipelines(vec![RenderPipeline::specialized(
                pipeline_handle,
                // NOTE: in the future you wont need to manually declare dynamic bindings
                PipelineSpecialization {
                    dynamic_bindings: vec![
                        // Transform
                        DynamicBinding {
                            bind_group: 1,
                            binding: 0,
                        },
                        // MyMaterial_color
                        DynamicBinding {
                            bind_group: 1,
                            binding: 1,
                        },
                    ],
                    ..Default::default()
                },
            )]),
            translation: Translation::new(0.0, 0.0, 0.0),
            ..Default::default()
        })
        .with(material)
        .with(mesh)
        // camera
        .spawn(Camera3dComponents {
            transform: Transform::new_sync_disabled(Mat4::face_toward(
                Vec3::new(3.0, 5.0, -8.0),
                Vec3::new(0.0, 0.0, 0.0),
                Vec3::new(0.0, 1.0, 0.0),
            )),
            ..Default::default()
        });
}

pub fn my_mesh_node_system(
    mut state: Local<MyMeshNodeState>,
    render_resource_context: Res<Box<dyn RenderResourceContext>>,
    mut render_resource_bindings: ResMut<RenderResourceBindings>,
    handle: &Handle<MyMesh>,
) {
    let vertices = [5f32, 0., -5., 5., 0., 5., -5., 0., 5., -5., 0., -5.].as_bytes();
    let indices = [0i32, 2, 1, 0, 3, 2].as_bytes();
    let render_resource_context = &**render_resource_context;

    let buffers = if let Some(buffers) = state.buffers {
        buffers
    } else {
        let buffers = MyMeshBuffers::new(
            render_resource_context,
            BufferInfo {
                size: vertices.len(),
                buffer_usage: BufferUsage::VERTEX,
                ..Default::default()
            },
            BufferInfo {
                size: indices.len(),
                buffer_usage: BufferUsage::INDEX,
                ..Default::default()
            },
        );

        state.buffers = Some(buffers);
        buffers
    };

    render_resource_context.map_buffer(buffers.vertex_staging);
    render_resource_context.write_mapped_buffer(
        buffers.vertex_staging,
        0..vertices.len() as u64,
        &mut |data, _renderer| data[0..vertices.len()].copy_from_slice(vertices),
    );
    render_resource_context.unmap_buffer(buffers.vertex_staging);

    render_resource_context.map_buffer(buffers.index_staging);
    render_resource_context.write_mapped_buffer(
        buffers.index_staging,
        0..indices.len() as u64,
        &mut |data, _renderer| data[0..indices.len()].copy_from_slice(indices),
    );
    render_resource_context.unmap_buffer(buffers.index_staging);

    render_resource_context.set_asset_resource(
        *handle,
        RenderResourceId::Buffer(buffers.vertex),
        VERTEX_BUFFER_ASSET_INDEX,
    );
    render_resource_context.set_asset_resource(
        *handle,
        RenderResourceId::Buffer(buffers.index),
        INDEX_BUFFER_ASSET_INDEX,
    );

    state.command_queue.copy_buffer_to_buffer(
        buffers.vertex_staging,
        0,
        buffers.vertex,
        0,
        vertices.len() as u64,
    );
    state.command_queue.copy_buffer_to_buffer(
        buffers.index_staging,
        0,
        buffers.index,
        0,
        indices.len() as u64,
    );
}

use bevy::prelude::*;
use bevy_render::pipeline::{ ComputePipelineDescriptor };
use bevy_render::shader::{ComputeShaderStages, ShaderStage};
use bevy_render::renderer::{
    BufferId,
    RenderResourceContext,
    RenderResourceBindings
};

fn main() {
    App::build()
        .add_resource(Msaa { samples: 4 })
        .add_resource::<ComputeState>(ComputeState::default())
        .add_default_plugins()
        .add_startup_system(setup.system())
        .run();
}

#[derive(Default)]
struct ComputeState {
    read_buffer: Option<BufferId>,
    write_buffer: Option<BufferId>,
    pipeline_handle: Handle<ComputePipelineDescriptor>,
}

const COMPUTE_SHADER: &str = r#"
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

/// set up a simple 3D scene
fn setup(
    mut commands: Commands,
    mut compute_state: ResMut<ComputeState>,
    mut pipelines: ResMut<Assets<ComputePipelineDescriptor>>,
    mut shaders: ResMut<Assets<Shader>>,
    render_resource_context: Res<Box<dyn RenderResourceContext>>,
    mut render_resource_bindings: ResMut<RenderResourceBindings>,
) {
    let render_resource_context = &**render_resource_context;

    let pipeline = ComputePipelineDescriptor::new(
        ComputeShaderStages {
            compute: shaders.add(Shader::from_glsl(ShaderStage::Compute, COMPUTE_SHADER))
        }
    );
    compute_state.pipeline_handle = pipelines.add(pipeline);

        
}

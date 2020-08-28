use crate::{
    pipeline::{PipelineCompiler, PipelineDescriptor, RenderPipelines, VertexBufferDescriptors},
    render_graph::{Node, ResourceSlots, SystemNode},
    renderer::{RenderContext, RenderResourceContext},
    shader::Shader,
};

use bevy_asset::Assets;
use bevy_ecs::{Changed, Commands, IntoQuerySystem, Query, Res, ResMut, Resources, System, World};

pub struct CompilePipelinesNode;

impl Node for CompilePipelinesNode {
    fn update(
        &mut self,
        _world: &World,
        _resources: &Resources,
        _render_context: &mut dyn RenderContext,
        _input: &ResourceSlots,
        _output: &mut ResourceSlots,
    ) {
        // TODO: seems like we don't need to do anything here?
    }
}

impl SystemNode for CompilePipelinesNode {
    fn get_system(&self, _commands: &mut Commands) -> Box<dyn System> {
        compile_pipelines_system.system()
    }
}

fn compile_pipelines_system(
    mut pipeline_compiler: ResMut<PipelineCompiler>,
    render_resource_context: Res<Box<dyn RenderResourceContext>>,
    mut pipelines: ResMut<Assets<PipelineDescriptor>>,
    mut shaders: ResMut<Assets<Shader>>,
    vertex_buffer_descriptors: Res<VertexBufferDescriptors>,
    mut query: Query<Changed<RenderPipelines>>,
) {
    for changed_pipelines in &mut query.iter() {
        for pipeline in changed_pipelines.pipelines.iter() {
            pipeline_compiler.compile_pipeline(
                &**render_resource_context,
                &mut pipelines,
                &mut shaders,
                pipeline.pipeline,
                &vertex_buffer_descriptors,
                &pipeline.specialization,
            );
        }
    }
}

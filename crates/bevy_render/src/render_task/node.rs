use super::{compute_builder::ComputeCommandBuilder, pipeline_cache::PipelineCache, RenderTask};
use crate::{
    render_graph::{NodeRunError, RenderGraphContext, ViewNode},
    renderer::RenderContext,
    PipelineCache as PipelineCompiler,
};
use bevy_ecs::{
    entity::Entity,
    query::QueryItem,
    world::{FromWorld, World},
};
use std::marker::PhantomData;
use wgpu::{CommandEncoder, CommandEncoderDescriptor, ComputePass, ComputePassDescriptor};

#[derive(FromWorld)]
pub struct RenderTaskNode<T: RenderTask>(PhantomData<T>);

impl<T: RenderTask> ViewNode for RenderTaskNode<T> {
    type ViewQuery = (&'static T, Entity);

    fn run<'w>(
        &self,
        _graph: &mut RenderGraphContext,
        render_context: &mut RenderContext<'w>,
        (task, entity): QueryItem<'w, '_, Self::ViewQuery>,
        world: &'w World,
    ) -> Result<(), NodeRunError> {
        render_context.add_command_buffer_generation_task(move |render_device| {
            let mut command_encoder =
                render_device.create_command_encoder(&CommandEncoderDescriptor {
                    label: Some("todo"),
                });

            let task_encoder = RenderTaskEncoder {
                command_encoder: &mut command_encoder,
                compute_pass: None,
                pipeline_cache: todo!(),
                pipeline_compiler: world.resource::<PipelineCompiler>(),
            };

            task.encode_commands(task_encoder, entity, world);

            command_encoder.finish()
        });

        Ok(())
    }
}

pub struct RenderTaskEncoder<'a> {
    command_encoder: &'a mut CommandEncoder,
    compute_pass: Option<ComputePass<'static>>,
    pipeline_cache: &'a mut PipelineCache,
    pipeline_compiler: &'a PipelineCompiler,
}

impl<'a> RenderTaskEncoder<'a> {
    pub fn begin_render_pass(&mut self) {
        todo!()
    }

    pub fn compute_command<'b>(&'b mut self, pass_name: &'b str) -> ComputeCommandBuilder<'b> {
        if self.compute_pass.is_none() {
            self.compute_pass = Some(
                self.command_encoder
                    .begin_compute_pass(&ComputePassDescriptor::default())
                    .forget_lifetime(),
            );
        }

        ComputeCommandBuilder::new(
            self.compute_pass.as_mut().unwrap(),
            pass_name,
            self.pipeline_cache,
            self.pipeline_compiler,
        )
    }
}

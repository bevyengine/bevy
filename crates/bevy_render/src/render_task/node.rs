use super::{compute_builder::ComputeCommandBuilder, resource_cache::ResourceCache, RenderTask};
use crate::{
    render_graph::{NodeRunError, RenderGraphContext, ViewNode},
    renderer::{RenderContext, RenderDevice},
    PipelineCache as PipelineCompiler,
};
use bevy_ecs::{
    entity::Entity,
    query::QueryItem,
    world::{FromWorld, World},
};
use std::{
    marker::PhantomData,
    sync::{Arc, Mutex},
};
use wgpu::{CommandEncoder, CommandEncoderDescriptor, ComputePass, ComputePassDescriptor};

#[derive(FromWorld)]
pub struct RenderTaskNode<T: RenderTask> {
    resource_cache: Arc<Mutex<ResourceCache>>,
    _phantom_data: PhantomData<T>,
}

impl<T: RenderTask> ViewNode for RenderTaskNode<T> {
    type ViewQuery = (&'static T, Entity);

    fn run<'w>(
        &self,
        _graph: &mut RenderGraphContext,
        render_context: &mut RenderContext<'w>,
        (task, entity): QueryItem<'w, '_, Self::ViewQuery>,
        world: &'w World,
    ) -> Result<(), NodeRunError> {
        let resource_cache = Arc::clone(&self.resource_cache);

        render_context.add_command_buffer_generation_task(move |render_device| {
            let mut command_encoder =
                render_device.create_command_encoder(&CommandEncoderDescriptor {
                    label: Some("todo"),
                });

            let task_encoder = RenderTaskEncoder {
                command_encoder: &mut command_encoder,
                compute_pass: None,
                resource_cache: &mut resource_cache.lock().unwrap(),
                pipeline_compiler: world.resource::<PipelineCompiler>(),
                render_device: &render_device,
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
    resource_cache: &'a mut ResourceCache,
    pipeline_compiler: &'a PipelineCompiler,
    render_device: &'a RenderDevice,
}

impl<'a> RenderTaskEncoder<'a> {
    pub fn render_pass(&mut self) {
        todo!()
    }

    pub fn compute_pass<'b>(&'b mut self, pass_name: &'b str) -> ComputeCommandBuilder<'b> {
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
            self.resource_cache,
            self.pipeline_compiler,
            self.render_device,
        )
    }
}

use super::{compute_builder::ComputeCommandBuilder, RenderTask};
use crate::{
    render_graph::{NodeRunError, RenderGraphContext, ViewNode},
    renderer::RenderContext,
};
use bevy_asset::Handle;
use bevy_ecs::{
    query::QueryItem,
    world::{FromWorld, World},
};
use bevy_shader::Shader;
use std::marker::PhantomData;
use wgpu::{CommandEncoder, ComputePass, ComputePassDescriptor};

#[derive(FromWorld)]
pub struct RenderTaskNode<T: RenderTask>(PhantomData<T>);

// TODO: Can't implement ViewNode directly for T: RenderTask
impl<T: RenderTask> ViewNode for RenderTaskNode<T> {
    type ViewQuery = (&'static T,);

    fn run<'w>(
        &self,
        _graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        (task,): QueryItem<Self::ViewQuery>,
        _world: &World,
    ) -> Result<(), NodeRunError> {
        let mut encoder = RenderTaskEncoder {
            command_encoder: render_context.command_encoder(),
            compute_pass: None,
        };

        task.encode_commands(&mut encoder);

        Ok(())
    }
}

pub struct RenderTaskEncoder<'a> {
    command_encoder: &'a mut CommandEncoder,
    compute_pass: Option<ComputePass<'a>>,
}

impl<'a> RenderTaskEncoder<'a> {
    pub fn begin_render_pass(&mut self) {
        todo!()
    }

    pub fn compute_command(
        &mut self,
        pass_name: &'a str,
        shader: Handle<Shader>,
    ) -> ComputeCommandBuilder<'a> {
        if self.compute_pass.is_none() {
            self.compute_pass = Some(
                self.command_encoder
                    .begin_compute_pass(&ComputePassDescriptor::default()),
            );
        }

        ComputeCommandBuilder::new(self.compute_pass.as_mut().unwrap(), pass_name, shader)
    }
}

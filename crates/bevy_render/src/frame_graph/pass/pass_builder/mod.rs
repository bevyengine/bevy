pub mod compute_pass_builder;
pub mod encoder_pass_builder;
pub mod render_pass_builder;

pub use compute_pass_builder::*;
pub use encoder_pass_builder::*;
pub use render_pass_builder::*;

use std::{borrow::Cow, mem::take};

use crate::{
    frame_graph::{
        BindGroupBindingBuilder, EncoderCommand, EncoderCommandBuilder, PassNodeBuilder,
        RenderContext, ResourceMaterial, ResourceRead, ResourceRef, ResourceWrite,
    },
    render_resource::BindGroupLayout,
};

use super::{EncoderExecutor, PassTrait};

#[derive(Default)]
pub struct DynamicPass {
    begin_encoder_commands: Vec<EncoderCommand>,
    executors: Vec<Box<dyn EncoderExecutor>>,
    end_encoder_commands: Vec<EncoderCommand>,
}

impl PassTrait for DynamicPass {
    fn render(&self, render_context: &mut RenderContext) {
        render_context.flush_encoder();

        let mut command_encoder = render_context.create_command_encoder();

        for begin_encoder_command in self.begin_encoder_commands.iter() {
            begin_encoder_command.draw(&mut command_encoder);
        }

        for executor in self.executors.iter() {
            executor.execute(&mut command_encoder, render_context);
        }

        for end_encoder_command in self.end_encoder_commands.iter() {
            end_encoder_command.draw(&mut command_encoder);
        }

        let command_buffer = command_encoder.finish();

        render_context.add_command_buffer(command_buffer);
    }
}

pub struct PassBuilder<'a> {
    pub(crate) pass_node_builder: PassNodeBuilder<'a>,
    pass: DynamicPass,
}

impl<'a> Drop for PassBuilder<'a> {
    fn drop(&mut self) {
        let pass = take(&mut self.pass);
        self.pass_node_builder.set_pass(pass);
    }
}

impl<'a> EncoderCommandBuilder for PassBuilder<'a> {
    fn add_begin_encoder_command(&mut self, value: EncoderCommand) -> &mut Self {
        self.pass.begin_encoder_commands.push(value);

        self
    }

    fn add_end_encoder_command(&mut self, value: EncoderCommand) -> &mut Self {
        self.pass.end_encoder_commands.push(value);

        self
    }
}

impl<'a> PassBuilder<'a> {
    pub fn new(pass_node_builder: PassNodeBuilder<'a>) -> Self {
        PassBuilder {
            pass_node_builder,
            pass: DynamicPass::default(),
        }
    }

    pub fn read_material<M: ResourceMaterial>(
        &mut self,
        material: &M,
    ) -> ResourceRef<M::ResourceType, ResourceRead> {
        self.pass_node_builder.read_material(material)
    }

    pub fn write_material<M: ResourceMaterial>(
        &mut self,
        material: &M,
    ) -> ResourceRef<M::ResourceType, ResourceWrite> {
        self.pass_node_builder.write_material(material)
    }

    pub fn create_bind_group_builder<'b>(
        &'b mut self,
        label: Option<Cow<'static, str>>,
        layout: BindGroupLayout,
    ) -> BindGroupBindingBuilder<'a, 'b> {
        BindGroupBindingBuilder::new(label, layout, self)
    }

    pub fn pass_node_builder(&mut self) -> &mut PassNodeBuilder<'a> {
        &mut self.pass_node_builder
    }

    pub fn add_executor<T: EncoderExecutor>(&mut self, executor: T) {
        self.pass.executors.push(Box::new(executor));
    }

    pub fn create_render_pass_builder<'b>(&'b mut self) -> RenderPassBuilder<'a, 'b> {
        RenderPassBuilder::new(self)
    }

    pub fn create_compute_pass_builder<'b>(&'b mut self) -> ComputePassBuilder<'a, 'b> {
        ComputePassBuilder::new(self)
    }

    pub fn create_encoder_pass_builder<'b>(&'b mut self) -> EncoderPassBuilder<'a, 'b> {
        EncoderPassBuilder::new(self)
    }
}

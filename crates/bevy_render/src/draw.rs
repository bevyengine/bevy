use crate::{
    pipeline::{PipelineDescriptor, PipelineLayout},
    render_resource::{
        AssetRenderResourceBindings, BindGroup, BindGroupId, BufferId, BufferUsage, RenderResource,
        RenderResourceBinding, RenderResourceBindings, SharedBuffers,
    },
    renderer::RenderResourceContext,
};
use bevy_asset::{Assets, Handle};
use bevy_property::Properties;
use legion::{
    prelude::{ComMut, Res},
    storage::Component,
};
use std::{ops::Range, sync::Arc};
use thiserror::Error;

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum RenderCommand {
    SetPipeline {
        pipeline: Handle<PipelineDescriptor>,
    },
    SetVertexBuffer {
        slot: u32,
        buffer: BufferId,
        offset: u64,
    },
    SetIndexBuffer {
        buffer: BufferId,
        offset: u64,
    },
    SetBindGroup {
        index: u32,
        bind_group: BindGroupId,
        dynamic_uniform_indices: Option<Arc<Vec<u32>>>,
    },
    DrawIndexed {
        indices: Range<u32>,
        base_vertex: i32,
        instances: Range<u32>,
    },
}

#[derive(Properties)]
pub struct Draw {
    pub is_visible: bool,
    #[property(ignore)]
    pub render_commands: Vec<RenderCommand>,
}

impl Default for Draw {
    fn default() -> Self {
        Self {
            is_visible: true,
            render_commands: Default::default(),
        }
    }
}

impl Draw {
    pub fn get_context<'a>(
        &'a mut self,
        pipelines: &'a Assets<PipelineDescriptor>,
        render_resource_context: &'a dyn RenderResourceContext,
        render_resource_bindings: &'a RenderResourceBindings,
        asset_render_resource_bindings: &'a AssetRenderResourceBindings,
        shared_buffers: &'a SharedBuffers,
    ) -> DrawContext {
        DrawContext {
            draw: self,
            pipelines,
            render_resource_context,
            render_resource_bindings,
            asset_render_resource_bindings,
            shared_buffers,
            current_pipeline: None,
        }
    }

    pub fn clear_render_commands(&mut self) {
        self.render_commands.clear();
    }
}

#[derive(Debug, Error)]
pub enum DrawError {
    #[error("Pipeline does not exist.")]
    NonExistentPipeline,
    #[error("No pipeline set")]
    NoPipelineSet,
    #[error("Pipeline has no layout")]
    PipelineHasNoLayout,
    #[error("A BindGroup with the given index does not exist")]
    BindGroupDescriptorDoesNotExist { index: u32 },
    #[error("Failed to get a buffer for the given RenderResource.")]
    BufferAllocationFailure,
}

pub struct DrawContext<'a> {
    pub draw: &'a mut Draw,
    pub pipelines: &'a Assets<PipelineDescriptor>,
    pub render_resource_context: &'a dyn RenderResourceContext,
    pub render_resource_bindings: &'a RenderResourceBindings,
    pub asset_render_resource_bindings: &'a AssetRenderResourceBindings,
    pub shared_buffers: &'a SharedBuffers,
    pub current_pipeline: Option<&'a PipelineDescriptor>,
}

impl<'a> DrawContext<'a> {
    pub fn get_uniform_buffer<T: RenderResource>(
        &self,
        render_resource: &T,
    ) -> Result<RenderResourceBinding, DrawError> {
        self.get_buffer(render_resource, BufferUsage::UNIFORM)
    }
    pub fn get_buffer<T: RenderResource>(
        &self,
        render_resource: &T,
        buffer_usage: BufferUsage,
    ) -> Result<RenderResourceBinding, DrawError> {
        self.shared_buffers
            .get_buffer(render_resource, buffer_usage)
            .ok_or_else(|| DrawError::BufferAllocationFailure)
    }

    pub fn set_pipeline(
        &mut self,
        pipeline_handle: Handle<PipelineDescriptor>,
    ) -> Result<(), DrawError> {
        let pipeline = self
            .pipelines
            .get(&pipeline_handle)
            .ok_or_else(|| DrawError::NonExistentPipeline)?;
        self.current_pipeline = Some(pipeline);
        self.render_command(RenderCommand::SetPipeline {
            pipeline: pipeline_handle,
        });
        Ok(())
    }

    pub fn set_vertex_buffer(&mut self, slot: u32, buffer: BufferId, offset: u64) {
        self.render_command(RenderCommand::SetVertexBuffer {
            slot,
            buffer,
            offset,
        });
    }

    pub fn set_index_buffer(&mut self, buffer: BufferId, offset: u64) {
        self.render_command(RenderCommand::SetIndexBuffer { buffer, offset });
    }

    pub fn set_bind_group(&mut self, index: u32, bind_group: &BindGroup) {
        self.render_command(RenderCommand::SetBindGroup {
            index,
            bind_group: bind_group.id,
            dynamic_uniform_indices: bind_group.dynamic_uniform_indices.clone(),
        });
    }

    pub fn draw_indexed(&mut self, indices: Range<u32>, base_vertex: i32, instances: Range<u32>) {
        self.render_command(RenderCommand::DrawIndexed {
            base_vertex,
            indices,
            instances,
        });
    }

    pub fn get_pipeline_descriptor(&self) -> Result<&PipelineDescriptor, DrawError> {
        self.current_pipeline
            .ok_or_else(|| DrawError::NoPipelineSet)
    }

    pub fn get_pipeline_layout(&self) -> Result<&PipelineLayout, DrawError> {
        self.get_pipeline_descriptor().and_then(|descriptor| {
            descriptor
                .get_layout()
                .ok_or_else(|| DrawError::PipelineHasNoLayout)
        })
    }

    pub fn set_bind_groups_from_bindings(
        &mut self,
        render_resource_bindings: &RenderResourceBindings,
    ) -> Result<(), DrawError> {
        let pipeline = self
            .current_pipeline
            .ok_or_else(|| DrawError::NoPipelineSet)?;
        let layout = pipeline
            .get_layout()
            .ok_or_else(|| DrawError::PipelineHasNoLayout)?;
        for bind_group_descriptor in layout.bind_groups.iter() {
            if let Some(local_bind_group) =
                render_resource_bindings.get_descriptor_bind_group(bind_group_descriptor.id)
            {
                self.set_bind_group(bind_group_descriptor.index, local_bind_group);
            }
        }

        Ok(())
    }

    #[inline]
    pub fn render_command(&mut self, render_command: RenderCommand) {
        self.draw.render_commands.push(render_command);
    }

    pub fn draw<T: Drawable>(&mut self, drawable: &mut T) -> Result<(), DrawError> {
        drawable.draw(self)
    }
}

pub trait Drawable {
    fn draw(&mut self, draw: &mut DrawContext) -> Result<(), DrawError>;
}

pub fn draw_system<T: Drawable + Component>(
    pipelines: Res<Assets<PipelineDescriptor>>,
    render_resource_bindings: Res<RenderResourceBindings>,
    render_resource_context: Res<Box<dyn RenderResourceContext>>,
    asset_render_resource_bindings: Res<AssetRenderResourceBindings>,
    shared_buffers: Res<SharedBuffers>,
    mut draw: ComMut<Draw>,
    mut drawable: ComMut<T>,
) {
    let mut draw_context = draw.get_context(
        &pipelines,
        &**render_resource_context,
        &render_resource_bindings,
        &asset_render_resource_bindings,
        &shared_buffers,
    );
    draw_context.draw(drawable.as_mut()).unwrap();
}

pub fn clear_draw_system(mut draw: ComMut<Draw>) {
    draw.clear_render_commands();
}

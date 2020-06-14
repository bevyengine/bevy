use crate::{
    pipeline::{BindGroupDescriptor, BindGroupDescriptorId, PipelineDescriptor},
    render_resource::{
        BufferId, BufferUsage, RenderResource, RenderResourceAssignment, RenderResourceAssignments,
        RenderResourceSet, RenderResourceSetId, SharedBuffers,
    },
    renderer::{RenderResourceContext, RenderResources},
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
        bind_group_descriptor: BindGroupDescriptorId,
        render_resource_set: RenderResourceSetId,
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

#[derive(Properties)]
pub struct RenderPipelines {
    pub pipelines: Vec<Handle<PipelineDescriptor>>,
    // TODO: make these pipeline specific
    #[property(ignore)]
    pub render_resource_assignments: RenderResourceAssignments,
    #[property(ignore)]
    pub compiled_pipelines: Vec<Handle<PipelineDescriptor>>,
}

impl Default for RenderPipelines {
    fn default() -> Self {
        Self {
            render_resource_assignments: Default::default(),
            compiled_pipelines: Default::default(),
            pipelines: vec![Handle::default()],
        }
    }
}

impl Draw {
    pub fn get_context<'a>(
        &'a mut self,
        pipelines: &'a Assets<PipelineDescriptor>,
        render_resource_context: &'a dyn RenderResourceContext,
        render_resource_assignments: &'a RenderResourceAssignments,
        shared_buffers: &'a SharedBuffers,
    ) -> DrawContext {
        DrawContext {
            draw: self,
            pipelines,
            render_resource_context,
            render_resource_assignments,
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
    #[error("Failed to get a buffer for the given RenderResource.")]
    BufferAllocationFailure,
}

pub struct DrawContext<'a> {
    pub draw: &'a mut Draw,
    pub pipelines: &'a Assets<PipelineDescriptor>,
    pub render_resource_context: &'a dyn RenderResourceContext,
    pub render_resource_assignments: &'a RenderResourceAssignments,
    pub shared_buffers: &'a SharedBuffers,
    pub current_pipeline: Option<&'a PipelineDescriptor>,
}

impl<'a> DrawContext<'a> {
    pub fn get_uniform_buffer<T: RenderResource>(
        &self,
        render_resource: &T,
    ) -> Result<RenderResourceAssignment, DrawError> {
        self.get_buffer(render_resource, BufferUsage::UNIFORM)
    }
    pub fn get_buffer<T: RenderResource>(
        &self,
        render_resource: &T,
        buffer_usage: BufferUsage,
    ) -> Result<RenderResourceAssignment, DrawError> {
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

    pub fn set_bind_group(
        &mut self,
        bind_group_descriptor: &BindGroupDescriptor,
        render_resource_set: &RenderResourceSet,
    ) {
        self.render_command(RenderCommand::SetBindGroup {
            index: bind_group_descriptor.index,
            bind_group_descriptor: bind_group_descriptor.id,
            render_resource_set: render_resource_set.id,
            dynamic_uniform_indices: render_resource_set.dynamic_uniform_indices.clone(),
        });
    }

    pub fn draw_indexed(&mut self, indices: Range<u32>, base_vertex: i32, instances: Range<u32>) {
        self.render_command(RenderCommand::DrawIndexed {
            base_vertex,
            indices,
            instances,
        });
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

impl Drawable for RenderPipelines {
    fn draw(&mut self, draw: &mut DrawContext) -> Result<(), DrawError> {
        for pipeline_handle in self.compiled_pipelines.iter() {
            let pipeline = draw.pipelines.get(pipeline_handle).unwrap();
            let layout = pipeline.get_layout().unwrap();
            draw.set_pipeline(*pipeline_handle)?;
            for bind_group in layout.bind_groups.iter() {
                if let Some(local_render_resource_set) = self
                    .render_resource_assignments
                    .get_bind_group_render_resource_set(bind_group.id)
                {
                    draw.set_bind_group(bind_group, local_render_resource_set);
                } else if let Some(global_render_resource_set) = draw
                    .render_resource_assignments
                    .get_bind_group_render_resource_set(bind_group.id)
                {
                    draw.set_bind_group(bind_group, global_render_resource_set);
                }
            }
            let mut indices = 0..0;
            for (slot, vertex_buffer_descriptor) in
                layout.vertex_buffer_descriptors.iter().enumerate()
            {
                if let Some((vertex_buffer, index_buffer)) = self
                    .render_resource_assignments
                    .get_vertex_buffer(&vertex_buffer_descriptor.name)
                {
                    draw.set_vertex_buffer(slot as u32, vertex_buffer, 0);
                    if let Some(index_buffer) = index_buffer {
                        if let Some(buffer_info) =
                            draw.render_resource_context.get_buffer_info(index_buffer)
                        {
                            indices = 0..(buffer_info.size / 2) as u32;
                        } else {
                            panic!("expected buffer type");
                        }
                        draw.set_index_buffer(index_buffer, 0);
                    }
                }
            }

            draw.draw_indexed(indices, 0, 0..1);
        }

        Ok(())
    }
}

pub fn draw_system<T: Drawable + Component>(
    pipelines: Res<Assets<PipelineDescriptor>>,
    render_resource_assignments: Res<RenderResourceAssignments>,
    render_resources: Res<RenderResources>,
    shared_buffers: Res<SharedBuffers>,
    mut draw: ComMut<Draw>,
    mut drawable: ComMut<T>,
) {
    let context = &*render_resources.context;
    let mut draw_context = draw.get_context(
        &pipelines,
        context,
        &render_resource_assignments,
        &shared_buffers,
    );
    draw_context.draw(drawable.as_mut()).unwrap();
}

pub fn clear_draw_system(mut draw: ComMut<Draw>) {
    draw.clear_render_commands();
}

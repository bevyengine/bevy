use crate::{
    pipeline::{BindGroupDescriptor, BindGroupDescriptorId, PipelineDescriptor},
    render_resource::{
        RenderResourceAssignments, RenderResourceId, RenderResourceSet, RenderResourceSetId,
        ResourceInfo,
    },
    renderer::{RenderResourceContext, RenderResources},
};
use bevy_asset::{Assets, Handle};
use bevy_property::Properties;
use legion::{
    prelude::{Com, ComMut, Res},
    storage::Component,
};
use std::{ops::Range, sync::Arc};

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum RenderCommand {
    SetPipeline {
        pipeline: Handle<PipelineDescriptor>,
    },
    SetVertexBuffer {
        slot: u32,
        buffer: RenderResourceId,
        offset: u64,
    },
    SetIndexBuffer {
        buffer: RenderResourceId,
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
    ) -> DrawContext {
        DrawContext {
            draw: self,
            pipelines,
            render_resource_context,
            render_resource_assignments,
        }
    }

    pub fn clear_render_commands(&mut self) {
        self.render_commands.clear();
    }
}

pub struct DrawContext<'a> {
    pub draw: &'a mut Draw,
    pub pipelines: &'a Assets<PipelineDescriptor>,
    pub render_resource_context: &'a dyn RenderResourceContext,
    pub render_resource_assignments: &'a RenderResourceAssignments,
}

impl<'a> DrawContext<'a> {
    pub fn set_pipeline(&mut self, pipeline: Handle<PipelineDescriptor>) {
        self.render_command(RenderCommand::SetPipeline { pipeline });
    }
    pub fn set_vertex_buffer(&mut self, slot: u32, buffer: RenderResourceId, offset: u64) {
        self.render_command(RenderCommand::SetVertexBuffer {
            slot,
            buffer,
            offset,
        });
    }

    pub fn set_index_buffer(&mut self, buffer: RenderResourceId, offset: u64) {
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

    pub fn draw<T: Drawable>(&mut self, drawable: &T) {
        drawable.draw(self);
    }
}

pub trait Drawable {
    fn draw(&self, draw: &mut DrawContext);
}

impl Drawable for RenderPipelines {
    fn draw(&self, draw: &mut DrawContext) {
        for pipeline_handle in self.compiled_pipelines.iter() {
            let pipeline = draw.pipelines.get(pipeline_handle).unwrap();
            let layout = pipeline.get_layout().unwrap();
            draw.set_pipeline(*pipeline_handle);
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
                        draw.render_resource_context.get_resource_info(
                            index_buffer,
                            &mut |resource_info| match resource_info {
                                Some(ResourceInfo::Buffer(Some(buffer_info))) => {
                                    indices = 0..(buffer_info.size / 2) as u32;
                                }
                                _ => panic!("expected a buffer type"),
                            },
                        );
                        draw.set_index_buffer(index_buffer, 0);
                    }
                }
            }

            draw.draw_indexed(indices, 0, 0..1);
        }
    }
}

pub fn draw_system<T: Drawable + Component>(
    pipelines: Res<Assets<PipelineDescriptor>>,
    render_resource_assignments: Res<RenderResourceAssignments>,
    render_resources: Res<RenderResources>,
    mut draw: ComMut<Draw>,
    drawable: Com<T>,
) {
    let context = &*render_resources.context;
    let mut draw_context = draw.get_context(&pipelines, context, &render_resource_assignments);
    draw_context.draw(drawable.as_ref());
}

pub fn clear_draw_system(mut draw: ComMut<Draw>) {
    draw.clear_render_commands();
}

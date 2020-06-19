use crate::{
    pipeline::{
        PipelineCompiler, PipelineDescriptor, PipelineLayout, PipelineSpecialization,
        VertexBufferDescriptors,
    },
    render_resource::{
        AssetRenderResourceBindings, BindGroup, BindGroupId, BufferId, BufferUsage, RenderResource,
        RenderResourceBinding, RenderResourceBindings, SharedBuffers,
    },
    renderer::RenderResourceContext,
    shader::Shader,
};
use bevy_asset::{Assets, Handle};
use bevy_property::Properties;
use legion::{
    prelude::{ComMut, Res, ResourceSet},
    systems::{resource::ResourceTypeId, ResMut},
};
use std::{
    ops::{Deref, DerefMut, Range},
    sync::Arc,
};
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
    pub fn clear_render_commands(&mut self) {
        self.render_commands.clear();
    }

    pub fn set_pipeline(&mut self, pipeline: Handle<PipelineDescriptor>) {
        self.render_command(RenderCommand::SetPipeline { pipeline });
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

    #[inline]
    pub fn render_command(&mut self, render_command: RenderCommand) {
        self.render_commands.push(render_command);
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

#[derive(Clone)]
pub struct DrawContext<'a> {
    pub pipelines: ResMut<'a, Assets<PipelineDescriptor>>,
    pub shaders: ResMut<'a, Assets<Shader>>,
    pub pipeline_compiler: ResMut<'a, PipelineCompiler>,
    pub render_resource_context: Res<'a, Box<dyn RenderResourceContext>>,
    pub vertex_buffer_descriptors: Res<'a, VertexBufferDescriptors>,
    pub asset_render_resource_bindings: Res<'a, AssetRenderResourceBindings>,
    pub shared_buffers: Res<'a, SharedBuffers>,
    pub current_pipeline: Option<Handle<PipelineDescriptor>>,
}

impl<'a> ResourceSet for DrawContext<'a> {
    type PreparedResources = DrawContext<'a>;
    unsafe fn fetch_unchecked(resources: &legion::prelude::Resources) -> Self::PreparedResources {
        DrawContext {
            render_resource_context: Res::new(
                resources
                    .get::<Box<dyn RenderResourceContext>>()
                    .unwrap()
                    .deref() as *const Box<dyn RenderResourceContext>,
            ),
            vertex_buffer_descriptors: Res::new(
                resources.get::<VertexBufferDescriptors>().unwrap().deref()
                    as *const VertexBufferDescriptors,
            ),
            asset_render_resource_bindings: Res::new(
                resources
                    .get::<AssetRenderResourceBindings>()
                    .unwrap()
                    .deref() as *const AssetRenderResourceBindings,
            ),
            shared_buffers: Res::new(
                resources.get::<SharedBuffers>().unwrap().deref() as *const SharedBuffers
            ),
            pipelines: ResMut::new(
                resources
                    .get_mut::<Assets<PipelineDescriptor>>()
                    .unwrap()
                    .deref_mut() as *mut Assets<PipelineDescriptor>,
            ),
            shaders: ResMut::new(
                resources.get_mut::<Assets<Shader>>().unwrap().deref_mut() as *mut Assets<Shader>
            ),
            pipeline_compiler: ResMut::new(
                resources.get_mut::<PipelineCompiler>().unwrap().deref_mut()
                    as *mut PipelineCompiler,
            ),
            current_pipeline: None,
        }
    }
    fn read_types() -> Vec<legion::systems::resource::ResourceTypeId> {
        vec![
            ResourceTypeId::of::<Box<dyn RenderResourceContext>>(),
            ResourceTypeId::of::<VertexBufferDescriptors>(),
            ResourceTypeId::of::<AssetRenderResourceBindings>(),
            ResourceTypeId::of::<SharedBuffers>(),
        ]
    }
    fn write_types() -> Vec<legion::systems::resource::ResourceTypeId> {
        vec![
            ResourceTypeId::of::<Assets<PipelineDescriptor>>(),
            ResourceTypeId::of::<Assets<Shader>>(),
            ResourceTypeId::of::<PipelineCompiler>(),
        ]
    }
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
        draw: &mut Draw,
        pipeline_handle: Handle<PipelineDescriptor>,
        specialization: &PipelineSpecialization,
    ) -> Result<(), DrawError> {
        let specialized_pipeline = if let Some(specialized_pipeline) = self
            .pipeline_compiler
            .get_specialized_pipeline(pipeline_handle, specialization)
        {
            specialized_pipeline
        } else {
            self.pipeline_compiler.compile_pipeline(
                &**self.render_resource_context,
                &mut self.pipelines,
                &mut self.shaders,
                pipeline_handle,
                &self.vertex_buffer_descriptors,
                specialization,
            )
        };
        draw.set_pipeline(specialized_pipeline);
        self.current_pipeline = Some(specialized_pipeline);
        Ok(())
    }

    pub fn get_pipeline_descriptor(&self) -> Result<&PipelineDescriptor, DrawError> {
        self.current_pipeline
            .and_then(|handle| self.pipelines.get(&handle))
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
        &self,
        draw: &mut Draw,
        render_resource_bindings: &mut [&mut RenderResourceBindings],
    ) -> Result<(), DrawError> {
        let pipeline = self
            .current_pipeline
            .ok_or_else(|| DrawError::NoPipelineSet)?;
        let pipeline_descriptor = self
            .pipelines
            .get(&pipeline)
            .ok_or_else(|| DrawError::NonExistentPipeline)?;
        let layout = pipeline_descriptor
            .get_layout()
            .ok_or_else(|| DrawError::PipelineHasNoLayout)?;
        for bindings in render_resource_bindings.iter_mut() {
            bindings.update_bind_groups(pipeline_descriptor, &**self.render_resource_context);
        }
        for bind_group_descriptor in layout.bind_groups.iter() {
            for bindings in render_resource_bindings.iter_mut() {
                if let Some(bind_group) =
                    bindings.get_descriptor_bind_group(bind_group_descriptor.id)
                {
                    draw.set_bind_group(bind_group_descriptor.index, bind_group);
                    break;
                }
            }
        }

        Ok(())
    }

    pub fn set_vertex_buffers_from_bindings(
        &self,
        draw: &mut Draw,
        render_resource_bindings: &[&RenderResourceBindings],
    ) -> Result<Option<Range<u32>>, DrawError> {
        let mut indices = None;
        let pipeline = self
            .current_pipeline
            .ok_or_else(|| DrawError::NoPipelineSet)?;
        let pipeline_descriptor = self
            .pipelines
            .get(&pipeline)
            .ok_or_else(|| DrawError::NonExistentPipeline)?;
        let layout = pipeline_descriptor
            .get_layout()
            .ok_or_else(|| DrawError::PipelineHasNoLayout)?;
        for (slot, vertex_buffer_descriptor) in layout.vertex_buffer_descriptors.iter().enumerate()
        {
            for bindings in render_resource_bindings.iter() {
                if let Some((vertex_buffer, index_buffer)) =
                    bindings.get_vertex_buffer(&vertex_buffer_descriptor.name)
                {
                    draw.set_vertex_buffer(slot as u32, vertex_buffer, 0);
                    if let Some(index_buffer) = index_buffer {
                        if let Some(buffer_info) =
                            self.render_resource_context.get_buffer_info(index_buffer)
                        {
                            indices = Some(0..(buffer_info.size / 2) as u32);
                        } else {
                            panic!("expected buffer type");
                        }
                        draw.set_index_buffer(index_buffer, 0);
                    }

                    break;
                }
            }
        }

        Ok(indices)
    }
}

pub trait Drawable {
    fn draw(&mut self, draw: &mut Draw, context: &mut DrawContext) -> Result<(), DrawError>;
}

pub fn clear_draw_system(mut draw: ComMut<Draw>) {
    draw.clear_render_commands();
}

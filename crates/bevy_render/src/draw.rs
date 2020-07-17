use crate::{
    pipeline::{
        PipelineCompiler, PipelineDescriptor, PipelineLayout, PipelineSpecialization,
        VertexBufferDescriptors,
    },
    renderer::{
        BindGroup, BindGroupId, BufferId, BufferUsage, RenderResource, RenderResourceBinding,
        RenderResourceBindings, RenderResourceContext, SharedBuffers,
    },
    shader::Shader,
};
use bevy_asset::{Assets, Handle};
use bevy_ecs::{
    Archetype, FetchResource, Query, Res, ResMut, ResourceQuery, Resources, SystemId, TypeAccess,
    UnsafeClone,
};
use bevy_property::Properties;
use std::{any::TypeId, collections::HashMap, ops::Range, sync::Arc};
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
    pub is_transparent: bool,
    #[property(ignore)]
    pub render_commands: Vec<RenderCommand>,
}

impl Default for Draw {
    fn default() -> Self {
        Self {
            is_visible: true,
            is_transparent: false,
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

pub struct DrawContext<'a> {
    pub pipelines: ResMut<'a, Assets<PipelineDescriptor>>,
    pub shaders: ResMut<'a, Assets<Shader>>,
    pub pipeline_compiler: ResMut<'a, PipelineCompiler>,
    pub render_resource_context: Res<'a, Box<dyn RenderResourceContext>>,
    pub vertex_buffer_descriptors: Res<'a, VertexBufferDescriptors>,
    pub shared_buffers: Res<'a, SharedBuffers>,
    pub current_pipeline: Option<Handle<PipelineDescriptor>>,
}

impl<'a> UnsafeClone for DrawContext<'a> {
    unsafe fn unsafe_clone(&self) -> Self {
        Self {
            pipelines: self.pipelines.unsafe_clone(),
            shaders: self.shaders.unsafe_clone(),
            pipeline_compiler: self.pipeline_compiler.unsafe_clone(),
            render_resource_context: self.render_resource_context.unsafe_clone(),
            vertex_buffer_descriptors: self.vertex_buffer_descriptors.unsafe_clone(),
            shared_buffers: self.shared_buffers.unsafe_clone(),
            current_pipeline: self.current_pipeline.clone(),
        }
    }
}

impl<'a> ResourceQuery for DrawContext<'a> {
    type Fetch = FetchDrawContext;
}

pub struct FetchDrawContext;

// TODO: derive this impl
impl<'a> FetchResource<'a> for FetchDrawContext {
    type Item = DrawContext<'a>;
    fn borrow(resource_archetypes: &HashMap<TypeId, Archetype>) {
        resource_archetypes
            .get(&TypeId::of::<Assets<PipelineDescriptor>>())
            .unwrap()
            .borrow_mut::<Assets<PipelineDescriptor>>();
        resource_archetypes
            .get(&TypeId::of::<Assets<Shader>>())
            .unwrap()
            .borrow_mut::<Assets<Shader>>();
        resource_archetypes
            .get(&TypeId::of::<PipelineCompiler>())
            .unwrap()
            .borrow_mut::<PipelineCompiler>();
        resource_archetypes
            .get(&TypeId::of::<Box<dyn RenderResourceContext>>())
            .unwrap()
            .borrow::<Box<dyn RenderResourceContext>>();
        resource_archetypes
            .get(&TypeId::of::<VertexBufferDescriptors>())
            .unwrap()
            .borrow::<VertexBufferDescriptors>();
        resource_archetypes
            .get(&TypeId::of::<SharedBuffers>())
            .unwrap()
            .borrow::<SharedBuffers>();
    }
    fn release(resource_archetypes: &HashMap<TypeId, Archetype>) {
        resource_archetypes
            .get(&TypeId::of::<Assets<PipelineDescriptor>>())
            .unwrap()
            .release_mut::<Assets<PipelineDescriptor>>();
        resource_archetypes
            .get(&TypeId::of::<Assets<Shader>>())
            .unwrap()
            .release_mut::<Assets<Shader>>();
        resource_archetypes
            .get(&TypeId::of::<PipelineCompiler>())
            .unwrap()
            .release_mut::<PipelineCompiler>();
        resource_archetypes
            .get(&TypeId::of::<Box<dyn RenderResourceContext>>())
            .unwrap()
            .release::<Box<dyn RenderResourceContext>>();
        resource_archetypes
            .get(&TypeId::of::<VertexBufferDescriptors>())
            .unwrap()
            .release::<VertexBufferDescriptors>();
        resource_archetypes
            .get(&TypeId::of::<SharedBuffers>())
            .unwrap()
            .release::<SharedBuffers>();
    }
    unsafe fn get(resources: &'a Resources, _system_id: Option<SystemId>) -> Self::Item {
        DrawContext {
            pipelines: resources.get_res_mut::<Assets<PipelineDescriptor>>(),
            shaders: resources.get_res_mut::<Assets<Shader>>(),
            pipeline_compiler: resources.get_res_mut::<PipelineCompiler>(),
            render_resource_context: resources.get_res::<Box<dyn RenderResourceContext>>(),
            vertex_buffer_descriptors: resources.get_res::<VertexBufferDescriptors>(),
            shared_buffers: resources.get_res::<SharedBuffers>(),
            current_pipeline: None,
        }
    }

    fn access() -> TypeAccess {
        let mut access = TypeAccess::default();
        access
            .mutable
            .insert(TypeId::of::<Assets<PipelineDescriptor>>());
        access.mutable.insert(TypeId::of::<Assets<Shader>>());
        access.mutable.insert(TypeId::of::<PipelineCompiler>());
        access
            .immutable
            .insert(TypeId::of::<Box<dyn RenderResourceContext>>());
        access
            .immutable
            .insert(TypeId::of::<VertexBufferDescriptors>());
        access.immutable.insert(TypeId::of::<SharedBuffers>());
        access
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

    pub fn create_bind_group_resource(
        &self,
        index: u32,
        bind_group: &BindGroup,
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
        let bind_group_descriptor = &layout.bind_groups[index as usize];
        self.render_resource_context
            .create_bind_group(bind_group_descriptor.id, bind_group);
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

pub fn clear_draw_system(mut query: Query<&mut Draw>) {
    for draw in &mut query.iter() {
        draw.clear_render_commands();
    }
}

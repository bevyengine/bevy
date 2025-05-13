use std::{borrow::Cow, num::NonZero, ops::Deref};

use wgpu::BufferBinding;

use crate::{
    frame_graph::{FrameGraphBuffer, FrameGraphError, PassBuilder, RenderContext, ResourceDrawing},
    render_resource::BindGroupLayout,
};

use super::{
    BindGroupEntryRef, BindingResourceHelper, BindingResourceRef, IntoBindingResourceHandle,
};

pub struct BindGroupDrawingBuilder<'a, 'b> {
    label: Option<Cow<'static, str>>,
    layout: BindGroupLayout,
    entries: Vec<BindGroupEntryRef>,
    pass_builder: &'b mut PassBuilder<'a>,
}

impl<'a, 'b> BindGroupDrawingBuilder<'a, 'b> {
    pub fn new(
        label: Option<Cow<'static, str>>,
        layout: BindGroupLayout,
        pass_builder: &'b mut PassBuilder<'a>,
    ) -> Self {
        Self {
            label,
            layout,
            entries: vec![],
            pass_builder,
        }
    }

    pub fn push_bind_resource_ref(mut self, bind_resource_ref: BindingResourceRef) -> Self {
        self.entries.push(BindGroupEntryRef {
            binding: self.entries.len() as u32,
            resource: bind_resource_ref,
        });

        self
    }

    pub fn push_bind_group_handle<T: IntoBindingResourceHandle>(self, value: T) -> Self {
        let handle = T::into_binding(value);
        self.push_bind_group_entry(&handle)
    }

    pub fn push_bind_group_entry<T: BindingResourceHelper>(self, value: &T) -> Self {
        let bind_group_resource_ref =
            value.make_binding_resource_ref(self.pass_builder.pass_node_builder());
        self.push_bind_resource_ref(bind_group_resource_ref)
    }

    pub fn build(self) -> BindGroupDrawing {
        BindGroupDrawing {
            label: self.label,
            layout: self.layout,
            entries: self.entries,
        }
    }
}

#[derive(Clone)]
pub struct BindGroupDrawing {
    pub label: Option<Cow<'static, str>>,
    pub layout: BindGroupLayout,
    pub entries: Vec<BindGroupEntryRef>,
}

pub enum BindingResource<'a> {
    Buffer {
        buffer: &'a FrameGraphBuffer,
        size: Option<NonZero<u64>>,
    },
    Sampler(wgpu::Sampler),
    TextureView(wgpu::TextureView),
}

impl<'a> BindingResource<'a> {
    pub fn get_resource_binding(&self) -> wgpu::BindingResource {
        match &self {
            BindingResource::Sampler(sampler) => wgpu::BindingResource::Sampler(sampler),
            BindingResource::TextureView(texture_view) => {
                wgpu::BindingResource::TextureView(texture_view)
            }
            BindingResource::Buffer { buffer, size } => {
                wgpu::BindingResource::Buffer(BufferBinding {
                    buffer: &buffer.resource,
                    offset: 0,
                    size: *size,
                })
            }
        }
    }
}

impl ResourceDrawing for BindGroupDrawing {
    type Resource = wgpu::BindGroup;

    fn make_resource<'a>(
        &self,
        render_context: &RenderContext<'a>,
    ) -> Result<Self::Resource, FrameGraphError> {
        let mut resources = vec![];
        for entry in self.entries.iter() {
            let resource = match &entry.resource {
                BindingResourceRef::Sampler(sampler) => {
                    BindingResource::Sampler(sampler.deref().clone())
                }
                BindingResourceRef::TextureView {
                    texture,
                    texture_view_info,
                } => {
                    let texture = render_context.get_resource(texture)?;
                    BindingResource::TextureView(
                        texture
                            .resource
                            .create_view(&texture_view_info.get_texture_view_desc()),
                    )
                }
                BindingResourceRef::Buffer { buffer, size } => BindingResource::Buffer {
                    buffer: render_context.get_resource(buffer)?,
                    size: *size,
                },
            };

            resources.push((entry.binding, resource));
        }

        let bind_graoup = render_context
            .render_device
            .wgpu_device()
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: self.label.as_deref(),
                layout: self.layout.deref(),
                entries: &resources
                    .iter()
                    .map(|(binding, resource)| wgpu::BindGroupEntry {
                        binding: *binding,
                        resource: resource.get_resource_binding(),
                    })
                    .collect::<Vec<_>>(),
            });

        Ok(bind_graoup)
    }
}

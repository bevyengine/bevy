use std::{borrow::Cow, num::NonZero, ops::Deref};

use wgpu::BufferBinding;

use crate::{
    frame_graph::{FrameGraphBuffer, FrameGraphError, RenderContext, ResourceDrawing},
    render_resource::BindGroupLayout,
};

use super::{BindGroupEntryRef, BindingResourceRef};

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
                BindingResourceRef::Sampler(info) => BindingResource::Sampler(
                    render_context
                        .render_device
                        .wgpu_device()
                        .create_sampler(&info.get_sample_desc()),
                ),
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

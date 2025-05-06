use std::{borrow::Cow, ops::Deref};

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
    Buffer(&'a FrameGraphBuffer),
    Sampler(wgpu::Sampler),
    TextureView(wgpu::TextureView),
}

impl<'a> BindingResource<'a> {
    pub fn get_resource_binding(&self) -> wgpu::BindingResource {
        match &self {
            BindingResource::Buffer(buffer) => {
                wgpu::BindingResource::Buffer(buffer.resource.as_entire_buffer_binding())
            }
            BindingResource::Sampler(sampler) => wgpu::BindingResource::Sampler(sampler),
            BindingResource::TextureView(texture_view) => {
                wgpu::BindingResource::TextureView(texture_view)
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
                BindingResourceRef::Buffer(resource_ref) => {
                    BindingResource::Buffer(render_context.get_resource(resource_ref)?)
                }
                BindingResourceRef::Sampler(info) => BindingResource::Sampler(
                    render_context
                        .render_device
                        .wgpu_device()
                        .create_sampler(&info.get_sample_desc()),
                ),
                BindingResourceRef::TextureView {
                    texture_ref,
                    texture_view_info,
                } => {
                    let texture = render_context.get_resource(texture_ref)?;
                    BindingResource::TextureView(
                        texture
                            .resource
                            .create_view(&texture_view_info.get_texture_view_desc()),
                    )
                }
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

use std::{borrow::Cow, num::NonZero, ops::Deref};

use bevy_platform::collections::HashMap;
use wgpu::BufferBinding;

use crate::{
    frame_graph::{FrameGraphBuffer, PassBuilder, RenderContext, ResourceBinding},
    render_resource::BindGroupLayout,
};

use super::{
    BindGroupEntryBinding, BindGroupResourceBinding, BindGroupResourceHelper,
    IntoBindGroupResourceBinding, IntoBindGroupResourceHandle,
};

pub struct BindGroupBindingBuilder<'a, 'b> {
    label: Option<Cow<'static, str>>,
    layout: BindGroupLayout,
    entries: Vec<BindGroupEntryBinding>,
    pass_builder: &'b mut PassBuilder<'a>,
}

impl<'a, 'b> BindGroupBindingBuilder<'a, 'b> {
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

    pub fn add_binding<T: IntoBindGroupResourceBinding>(
        mut self,
        binding: u32,
        resource: T,
    ) -> Self {
        self.entries.push(BindGroupEntryBinding {
            binding,
            resource: resource.into_binding(),
        });

        self
    }

    pub fn add_handle<T: IntoBindGroupResourceHandle>(self, binding: u32, resource: T) -> Self {
        let resource = T::into_binding(resource);
        self.add_helper(binding, &resource)
    }

    pub fn add_helper<T: BindGroupResourceHelper>(self, binding: u32, resource: &T) -> Self {
        let resource =
            resource.make_binding_group_resource_binding(self.pass_builder.pass_node_builder());
        self.add_binding(binding, resource)
    }

    pub fn build(self) -> BindGroupBinding {
        BindGroupBinding {
            label: self.label,
            layout: self.layout,
            entries: self.entries,
        }
    }
}

#[derive(Clone)]
pub struct BindGroupBinding {
    pub label: Option<Cow<'static, str>>,
    pub layout: BindGroupLayout,
    pub entries: Vec<BindGroupEntryBinding>,
}

pub enum BindingResource<'a> {
    Buffer {
        buffer: &'a FrameGraphBuffer,
        size: Option<NonZero<u64>>,
    },
    Sampler(wgpu::Sampler),
    TextureView(wgpu::TextureView),
    TextureViewArray(Vec<wgpu::TextureView>),
}

pub enum BindingResourceTemp<'a> {
    Buffer {
        buffer: &'a FrameGraphBuffer,
        size: Option<NonZero<u64>>,
    },
    Sampler(wgpu::Sampler),
    TextureView(wgpu::TextureView),
    TextureViewArray(Vec<&'a wgpu::TextureView>),
}

impl<'a> BindingResourceTemp<'a> {
    pub fn get_resource_binding(&self) -> wgpu::BindingResource {
        match self {
            BindingResourceTemp::Sampler(sampler) => wgpu::BindingResource::Sampler(sampler),
            BindingResourceTemp::TextureView(texture_view) => {
                wgpu::BindingResource::TextureView(texture_view)
            }
            BindingResourceTemp::Buffer { buffer, size } => {
                wgpu::BindingResource::Buffer(BufferBinding {
                    buffer: &buffer.resource,
                    offset: 0,
                    size: *size,
                })
            }
            BindingResourceTemp::TextureViewArray(texture_views) => {
                wgpu::BindingResource::TextureViewArray(texture_views.as_slice())
            }
        }
    }
}

impl ResourceBinding for BindGroupBinding {
    type Resource = wgpu::BindGroup;

    fn make_resource<'a>(&self, render_context: &RenderContext<'a>) -> Self::Resource {
        let mut resources = HashMap::new();

        for entry in self.entries.iter() {
            match &entry.resource {
                BindGroupResourceBinding::TextureViewArray(texture_view_refs) => {
                    let mut texture_views = vec![];

                    for texture_view_ref in texture_view_refs.iter() {
                        let texture = render_context.get_resource(&texture_view_ref.texture);

                        texture_views.push(texture.resource.create_view(
                            &texture_view_ref.texture_view_info.get_texture_view_desc(),
                        ));
                    }
                    resources.insert(entry.binding, texture_views);
                }
                _ => {}
            };
        }

        let mut temp = vec![];

        for entry in self.entries.iter() {
            let resource = match &entry.resource {
                BindGroupResourceBinding::Sampler(sampler) => {
                    BindingResourceTemp::Sampler(sampler.deref().clone())
                }
                BindGroupResourceBinding::TextureView(binding) => {
                    let texture = render_context.get_resource(&binding.texture);
                    BindingResourceTemp::TextureView(
                        texture
                            .resource
                            .create_view(&binding.texture_view_info.get_texture_view_desc()),
                    )
                }
                BindGroupResourceBinding::Buffer(buffer_ref) => BindingResourceTemp::Buffer {
                    buffer: render_context.get_resource(&buffer_ref.buffer),
                    size: buffer_ref.size,
                },
                BindGroupResourceBinding::TextureViewArray(_) => {
                    let mut temp_texture_views = vec![];

                    let texture_views = resources.get(&entry.binding).unwrap();

                    for texture_view in texture_views {
                        temp_texture_views.push(texture_view);
                    }

                    BindingResourceTemp::TextureViewArray(temp_texture_views)
                }
            };

            temp.push((entry.binding, resource));
        }

        let bind_graoup = render_context
            .render_device
            .wgpu_device()
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: self.label.as_deref(),
                layout: self.layout.deref(),
                entries: &temp
                    .iter()
                    .map(|(binding, resource)| wgpu::BindGroupEntry {
                        binding: *binding,
                        resource: resource.get_resource_binding(),
                    })
                    .collect::<Vec<_>>(),
            });

        bind_graoup
    }
}

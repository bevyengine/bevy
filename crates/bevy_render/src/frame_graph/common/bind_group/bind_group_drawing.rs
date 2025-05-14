use std::{borrow::Cow, num::NonZero, ops::Deref};

use bevy_platform::collections::HashMap;
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

impl ResourceDrawing for BindGroupDrawing {
    type Resource = wgpu::BindGroup;

    fn make_resource<'a>(
        &self,
        render_context: &RenderContext<'a>,
    ) -> Result<Self::Resource, FrameGraphError> {
        let mut resources = HashMap::new();

        for entry in self.entries.iter() {
            match &entry.resource {
                BindingResourceRef::TextureViewArray(texture_view_refs) => {
                    let mut texture_views = vec![];

                    for texture_view_ref in texture_view_refs.iter() {
                        let texture = render_context.get_resource(&texture_view_ref.texture)?;

                        texture_views.push(
                            texture
                                .resource
                                .create_view(&texture_view_ref.texture_view_info.get_texture_view_desc()),
                        );
                    }
                    resources.insert(entry.binding, texture_views);
                }
                _ => {}
            };
        }

        let mut temp = vec![];

        for entry in self.entries.iter() {
            let resource = match &entry.resource {
                BindingResourceRef::Sampler(sampler) => {
                    BindingResourceTemp::Sampler(sampler.deref().clone())
                }
                BindingResourceRef::TextureView {
                    texture,
                    texture_view_info,
                } => {
                    let texture = render_context.get_resource(texture)?;
                    BindingResourceTemp::TextureView(
                        texture
                            .resource
                            .create_view(&texture_view_info.get_texture_view_desc()),
                    )
                }
                BindingResourceRef::Buffer { buffer, size } => BindingResourceTemp::Buffer {
                    buffer: render_context.get_resource(buffer)?,
                    size: *size,
                },
                BindingResourceRef::TextureViewArray(_) => {
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

        Ok(bind_graoup)
    }
}

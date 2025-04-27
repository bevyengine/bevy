use std::borrow::Cow;

use crate::{
    frame_graph::{ExtraResource, FrameGraphBuffer, FrameGraphError, ResourceRead, ResourceRef},
    render_resource::{BindGroup, BindGroupLayout},
};

pub struct BindGroupRef {
    pub label: Option<Cow<'static, str>>,
    pub layout: BindGroupLayout,
    pub entries: Vec<BindGroupEntryRef>,
}

pub struct BindGroupEntryRef {
    pub binding: u32,
    pub resource: BindingResourceRef,
}

pub enum BindingResource<'a> {
    Buffer(&'a FrameGraphBuffer),
    Sampler(wgpu::Sampler),
}

impl<'a> BindingResource<'a> {
    pub fn get_resource_binding(&self) -> wgpu::BindingResource {
        match &self {
            BindingResource::Buffer(buffer) => {
                wgpu::BindingResource::Buffer(buffer.resource.as_entire_buffer_binding())
            }
            BindingResource::Sampler(sampler) => wgpu::BindingResource::Sampler(sampler),
        }
    }
}

#[derive(Default)]
pub struct SampleInfo {
    pub label: Option<Cow<'static, str>>,
    pub address_mode_u: wgpu::AddressMode,
    pub address_mode_v: wgpu::AddressMode,
    pub address_mode_w: wgpu::AddressMode,
    pub mag_filter: wgpu::FilterMode,
    pub min_filter: wgpu::FilterMode,
    pub mipmap_filter: wgpu::FilterMode,
    pub lod_min_clamp: f32,
    pub lod_max_clamp: f32,
    pub compare: Option<wgpu::CompareFunction>,
    pub anisotropy_clamp: u16,
    pub border_color: Option<wgpu::SamplerBorderColor>,
}

impl SampleInfo {
    pub fn get_sample_desc(&self) -> wgpu::SamplerDescriptor {
        wgpu::SamplerDescriptor {
            label: self.label.as_deref(),
            address_mode_u: self.address_mode_u,
            address_mode_v: self.address_mode_v,
            address_mode_w: self.address_mode_w,
            mag_filter: self.mag_filter,
            min_filter: self.min_filter,
            mipmap_filter: self.mipmap_filter,
            lod_min_clamp: self.lod_min_clamp,
            lod_max_clamp: self.lod_max_clamp,
            compare: self.compare,
            anisotropy_clamp: self.anisotropy_clamp,
            border_color: self.border_color,
        }
    }
}

pub enum BindingResourceRef {
    Buffer(ResourceRef<FrameGraphBuffer, ResourceRead>),
    Sampler(SampleInfo),
}

impl ExtraResource for BindGroupRef {
    type Resource = BindGroup;

    fn extra_resource(
        &self,
        resource_context: &crate::frame_graph::RenderContext,
    ) -> Result<Self::Resource, FrameGraphError> {
        let mut resources = vec![];
        for entry in self.entries.iter() {
            let resource = match &entry.resource {
                BindingResourceRef::Buffer(resource_ref) => {
                    BindingResource::Buffer(resource_context.get_resource(resource_ref)?)
                }
                BindingResourceRef::Sampler(info) => BindingResource::Sampler(
                    resource_context
                        .render_device
                        .wgpu_device()
                        .create_sampler(&info.get_sample_desc()),
                ),
            };

            resources.push((entry.binding, resource));
        }

        let bind_graoup = resource_context.render_device.create_bind_group(
            self.label.as_deref(),
            &self.layout,
            &resources
                .iter()
                .map(|(binding, resource)| wgpu::BindGroupEntry {
                    binding: *binding,
                    resource: resource.get_resource_binding(),
                })
                .collect::<Vec<_>>(),
        );

        Ok(bind_graoup)
    }
}

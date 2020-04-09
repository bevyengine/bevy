use super::WgpuRenderer;
use crate::wgpu_type_converter::WgpuInto;
use bevy_asset::{AssetStorage, Handle};
use bevy_render::{
    pipeline::{BindGroupDescriptor, BindGroupDescriptorId, BindType},
    render_resource::{
        BufferInfo, RenderResource, RenderResourceAssignments, RenderResourceSetId,
        RenderResources, ResourceInfo,
    },
    renderer::Renderer,
    shader::Shader,
    texture::{SamplerDescriptor, TextureDescriptor},
};
use bevy_window::WindowId;
use std::collections::HashMap;

#[derive(Default)]
pub struct WgpuBindGroupInfo {
    pub bind_groups: HashMap<RenderResourceSetId, wgpu::BindGroup>,
}

#[derive(Default)]
pub struct WgpuResources {
    pub render_resources: RenderResources,
    pub window_surfaces: HashMap<WindowId, wgpu::Surface>,
    pub window_swap_chains: HashMap<WindowId, wgpu::SwapChain>,
    pub buffers: HashMap<RenderResource, wgpu::Buffer>,
    pub textures: HashMap<RenderResource, wgpu::TextureView>,
    pub samplers: HashMap<RenderResource, wgpu::Sampler>,
    pub resource_info: HashMap<RenderResource, ResourceInfo>,
    pub shader_modules: HashMap<Handle<Shader>, wgpu::ShaderModule>,
    pub bind_groups: HashMap<BindGroupDescriptorId, WgpuBindGroupInfo>,
    pub bind_group_layouts: HashMap<BindGroupDescriptorId, wgpu::BindGroupLayout>,
}

impl WgpuResources {
    pub fn add_resource_info(&mut self, resource: RenderResource, resource_info: ResourceInfo) {
        self.resource_info.insert(resource, resource_info);
    }

    pub fn get_bind_group(
        &self,
        bind_group_descriptor_id: BindGroupDescriptorId,
        render_resource_set_id: RenderResourceSetId,
    ) -> Option<&wgpu::BindGroup> {
        if let Some(bind_group_info) = self.bind_groups.get(&bind_group_descriptor_id) {
            bind_group_info.bind_groups.get(&render_resource_set_id)
        } else {
            None
        }
    }

    pub fn create_bind_group(
        &mut self,
        device: &wgpu::Device,
        bind_group_descriptor: &BindGroupDescriptor,
        render_resource_assignments: &RenderResourceAssignments,
    ) -> bool {
        if let Some((render_resource_set_id, _indices)) =
            render_resource_assignments.get_render_resource_set_id(bind_group_descriptor.id)
        {
            log::trace!(
                "start creating bind group for RenderResourceSet {:?}",
                render_resource_set_id
            );
            let bindings = bind_group_descriptor
                .bindings
                .iter()
                .map(|binding| {
                    if let Some(resource) = render_resource_assignments.get(&binding.name) {

                        let resource_info = self.resource_info.get(&resource).unwrap();
                        log::trace!("found binding {} ({}) resource: {:?} {:?}", binding.index, binding.name, resource, resource_info);
                        wgpu::Binding {
                            binding: binding.index,
                            resource: match &binding.bind_type {
                                BindType::SampledTexture { .. } => {
                                    if let ResourceInfo::Texture = resource_info {
                                        let texture = self.textures.get(&resource).unwrap();
                                        wgpu::BindingResource::TextureView(texture)
                                    } else {
                                        panic!("expected a Texture resource");
                                    }
                                }
                                BindType::Sampler { .. } => {
                                    if let ResourceInfo::Sampler = resource_info {
                                        let sampler = self.samplers.get(&resource).unwrap();
                                        wgpu::BindingResource::Sampler(sampler)
                                    } else {
                                        panic!("expected a Sampler resource");
                                    }
                                }
                                BindType::Uniform { .. } => {
                                    if let ResourceInfo::Buffer(buffer_info) = resource_info {
                                        let buffer = self.buffers.get(&resource).unwrap();
                                        wgpu::BindingResource::Buffer {
                                            buffer,
                                            range: 0..buffer_info.size as u64,
                                        }
                                    } else {
                                        panic!("expected a Buffer resource");
                                    }
                                }
                                _ => panic!("unsupported bind type"),
                            },
                        }
                    } else {
                        panic!(
                            "No resource assigned to uniform \"{}\" for RenderResourceAssignments {:?}",
                            binding.name,
                            render_resource_assignments.id
                        );
                    }
                })
                .collect::<Vec<wgpu::Binding>>();
            let bind_group_layout = self
                .bind_group_layouts
                .get(&bind_group_descriptor.id)
                .unwrap();
            let wgpu_bind_group_descriptor = wgpu::BindGroupDescriptor {
                label: None,
                layout: bind_group_layout,
                bindings: bindings.as_slice(),
            };

            let bind_group = device.create_bind_group(&wgpu_bind_group_descriptor);
            let bind_group_info = self
                .bind_groups
                .entry(bind_group_descriptor.id)
                .or_insert_with(|| WgpuBindGroupInfo::default());
            bind_group_info
                .bind_groups
                .insert(*render_resource_set_id, bind_group);

            log::trace!(
                "created bind group for RenderResourceSet {:?}",
                render_resource_set_id
            );
            log::trace!("{:#?}", bind_group_descriptor);
            return true;
        } else {
            return false;
        }
    }

    pub fn create_buffer(
        &mut self,
        device: &wgpu::Device,
        buffer_info: BufferInfo,
    ) -> RenderResource {
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: buffer_info.size as u64,
            usage: buffer_info.buffer_usage.wgpu_into(),
        });

        let resource = self.render_resources.get_next_resource();
        self.add_resource_info(resource, ResourceInfo::Buffer(buffer_info));

        self.buffers.insert(resource, buffer);
        resource
    }

    pub fn create_buffer_with_data(
        &mut self,
        device: &wgpu::Device,
        mut buffer_info: BufferInfo,
        data: &[u8],
    ) -> RenderResource {
        buffer_info.size = data.len();
        let buffer = device.create_buffer_with_data(data, buffer_info.buffer_usage.wgpu_into());
        self.assign_buffer(buffer, buffer_info)
    }

    pub fn get_resource_info(&self, resource: RenderResource) -> Option<&ResourceInfo> {
        self.resource_info.get(&resource)
    }

    pub fn remove_buffer(&mut self, resource: RenderResource) {
        self.buffers.remove(&resource);
        self.resource_info.remove(&resource);
    }

    pub fn assign_buffer(
        &mut self,
        buffer: wgpu::Buffer,
        buffer_info: BufferInfo,
    ) -> RenderResource {
        let resource = self.render_resources.get_next_resource();
        self.add_resource_info(resource, ResourceInfo::Buffer(buffer_info));
        self.buffers.insert(resource, buffer);
        resource
    }

    pub fn begin_create_buffer_mapped(
        buffer_info: &BufferInfo,
        renderer: &mut WgpuRenderer,
        setup_data: &mut dyn FnMut(&mut [u8], &mut dyn Renderer),
    ) -> wgpu::Buffer {
        let device = renderer.device.clone();
        let mut mapped = device.create_buffer_mapped(
            &wgpu::BufferDescriptor {
                size: buffer_info.size as u64,
                usage: buffer_info.buffer_usage.wgpu_into(),
                label: None,
            }
        );
        setup_data(&mut mapped.data, renderer);
        mapped.finish()
    }

    pub fn copy_buffer_to_buffer(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        source_buffer: RenderResource,
        source_offset: u64,
        destination_buffer: RenderResource,
        destination_offset: u64,
        size: u64,
    ) {
        let source = self.buffers.get(&source_buffer).unwrap();
        let destination = self.buffers.get(&destination_buffer).unwrap();
        encoder.copy_buffer_to_buffer(source, source_offset, destination, destination_offset, size);
    }

    pub fn create_shader_module(
        &mut self,
        device: &wgpu::Device,
        shader_handle: Handle<Shader>,
        shader_storage: &AssetStorage<Shader>,
    ) {
        self.shader_modules.entry(shader_handle).or_insert_with(|| {
            let shader = shader_storage.get(&shader_handle).unwrap();
            device.create_shader_module(&shader.get_spirv(None))
        });
    }

    pub fn create_sampler(
        &mut self,
        device: &wgpu::Device,
        sampler_descriptor: &SamplerDescriptor,
    ) -> RenderResource {
        let descriptor: wgpu::SamplerDescriptor = (*sampler_descriptor).wgpu_into();
        let sampler = device.create_sampler(&descriptor);
        let resource = self.render_resources.get_next_resource();
        self.samplers.insert(resource, sampler);
        self.add_resource_info(resource, ResourceInfo::Sampler);
        resource
    }

    pub fn create_texture(
        &mut self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        texture_descriptor: &TextureDescriptor,
        bytes: Option<&[u8]>,
    ) -> RenderResource {
        let descriptor: wgpu::TextureDescriptor = (*texture_descriptor).wgpu_into();
        let texture = device.create_texture(&descriptor);
        let texture_view = texture.create_default_view();
        if let Some(bytes) = bytes {
            let temp_buf = device.create_buffer_with_data(bytes, wgpu::BufferUsage::COPY_SRC);
            encoder.copy_buffer_to_texture(
                wgpu::BufferCopyView {
                    buffer: &temp_buf,
                    offset: 0,
                    bytes_per_row: 4 * descriptor.size.width,
                    rows_per_image: 0, // NOTE: Example sets this to 0, but should it be height?
                },
                wgpu::TextureCopyView {
                    texture: &texture,
                    mip_level: 0,
                    array_layer: 0,
                    origin: wgpu::Origin3d { x: 0, y: 0, z: 0 },
                },
                descriptor.size,
            );
        }

        let resource = self.render_resources.get_next_resource();
        self.add_resource_info(resource, ResourceInfo::Texture);
        self.textures.insert(resource, texture_view);
        resource
    }

    pub fn remove_texture(&mut self, resource: RenderResource) {
        self.textures.remove(&resource);
        self.resource_info.remove(&resource);
    }

    pub fn remove_sampler(&mut self, resource: RenderResource) {
        self.samplers.remove(&resource);
        self.resource_info.remove(&resource);
    }

    pub fn get_render_resources(&self) -> &RenderResources {
        &self.render_resources
    }

    pub fn get_render_resources_mut(&mut self) -> &mut RenderResources {
        &mut self.render_resources
    }
}

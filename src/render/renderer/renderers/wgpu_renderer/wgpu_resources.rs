use super::WgpuRenderer;
use crate::render::{
    pipeline::{BindGroup, BindType},
    render_resource::{
        BufferInfo, RenderResource, RenderResourceAssignments, RenderResourceAssignmentsId,
        RenderResources, ResourceInfo,
    },
    renderer::Renderer,
    texture::{SamplerDescriptor, TextureDescriptor},
};
use std::collections::HashMap;

pub struct WgpuResources {
    pub render_resources: RenderResources,
    pub buffers: HashMap<RenderResource, wgpu::Buffer>,
    pub textures: HashMap<RenderResource, wgpu::TextureView>,
    pub samplers: HashMap<RenderResource, wgpu::Sampler>,
    pub resource_info: HashMap<RenderResource, ResourceInfo>,
    pub bind_groups: HashMap<u64, wgpu::BindGroup>,
    pub bind_group_layouts: HashMap<u64, wgpu::BindGroupLayout>,
    pub assignment_bind_groups: HashMap<(RenderResourceAssignmentsId, u64), wgpu::BindGroup>,
}

impl WgpuResources {
    pub fn new() -> Self {
        WgpuResources {
            buffers: HashMap::new(),
            textures: HashMap::new(),
            samplers: HashMap::new(),
            resource_info: HashMap::new(),
            bind_groups: HashMap::new(),
            bind_group_layouts: HashMap::new(),
            assignment_bind_groups: HashMap::new(),
            render_resources: RenderResources::default(),
        }
    }

    pub fn add_resource_info(&mut self, resource: RenderResource, resource_info: ResourceInfo) {
        self.resource_info.insert(resource, resource_info);
    }

    // TODO: consider moving this to a resource provider
    pub fn setup_bind_group(&mut self, device: &wgpu::Device, bind_group: &BindGroup) {
        let bind_group_id = bind_group.get_hash().unwrap();

        if let None = self.bind_groups.get(&bind_group_id) {
            let mut binding_resources = Vec::new();
            // if a uniform resource buffer doesn't exist, create a new empty one
            for binding in bind_group.bindings.iter() {
                let resource = match self.render_resources.get_named_resource(&binding.name) {
                    resource @ Some(_) => resource,
                    None => return,
                };

                if let Some(resource) = resource {
                    binding_resources.push(resource);
                }
            }

            // create wgpu Bindings
            let bindings = bind_group
                .bindings
                .iter()
                .zip(binding_resources)
                .map(|(binding, resource)| {
                    let resource_info = self.resource_info.get(&resource).unwrap();
                    wgpu::Binding {
                        binding: binding.index,
                        resource: match &binding.bind_type {
                            BindType::Uniform {
                                dynamic: _,
                                properties: _,
                            } => {
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
                })
                .collect::<Vec<wgpu::Binding>>();

            let bind_group_layout = self.bind_group_layouts.get(&bind_group_id).unwrap();
            let bind_group_descriptor = wgpu::BindGroupDescriptor {
                layout: bind_group_layout,
                bindings: bindings.as_slice(),
            };

            let bind_group = device.create_bind_group(&bind_group_descriptor);
            self.bind_groups
                .insert(bind_group_id, bind_group);
        }
    }
    pub fn get_assignments_bind_group(
        &self,
        render_resource_assignment_id: RenderResourceAssignmentsId,
        bind_group_id: u64,
    ) -> Option<&wgpu::BindGroup> {
        self.assignment_bind_groups
            .get(&(render_resource_assignment_id, bind_group_id))
    }

    pub fn create_assignments_bind_group(
        &mut self,
        device: &wgpu::Device,
        bind_group: &BindGroup,
        render_resource_assignments: &RenderResourceAssignments,
    ) {
        // TODO: don't make this per-entity. bind groups should be re-used across the same resource when possible
        let bind_group_id = bind_group.get_hash().unwrap();
        let bindings = bind_group
            .bindings
            .iter()
            .map(|binding| {
                if let Some(resource) = render_resource_assignments.get(&binding.name) {
                    let resource_info = self.resource_info.get(&resource).unwrap();
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
                            BindType::Sampler => {
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
                        render_resource_assignments.get_id()
                    );
                }
            })
            .collect::<Vec<wgpu::Binding>>();
        let bind_group_layout = self.bind_group_layouts.get(&bind_group_id).unwrap();
        let bind_group_descriptor = wgpu::BindGroupDescriptor {
            layout: bind_group_layout,
            bindings: bindings.as_slice(),
        };

        let bind_group = device.create_bind_group(&bind_group_descriptor);
        // TODO: storing a large number entity bind groups might actually be really bad. make sure this is ok
        self.assignment_bind_groups.insert(
            (render_resource_assignments.get_id(), bind_group_id),
            bind_group,
        );
    }

    pub fn create_buffer(
        &mut self,
        device: &wgpu::Device,
        buffer_info: BufferInfo,
    ) -> RenderResource {
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            size: buffer_info.size as u64,
            usage: buffer_info.buffer_usage.into(),
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
        let buffer = device.create_buffer_with_data(data, buffer_info.buffer_usage.into());
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
        let device_rc = renderer.device.clone();
        let device = device_rc.borrow();

        let mut mapped =
            device.create_buffer_mapped(buffer_info.size as usize, buffer_info.buffer_usage.into());
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

    pub fn create_sampler(
        &mut self,
        device: &wgpu::Device,
        sampler_descriptor: &SamplerDescriptor,
    ) -> RenderResource {
        let descriptor: wgpu::SamplerDescriptor = (*sampler_descriptor).into();
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
        let descriptor: wgpu::TextureDescriptor = (*texture_descriptor).into();
        let texture = device.create_texture(&descriptor);
        let texture_view = texture.create_default_view();
        if let Some(bytes) = bytes {
            let temp_buf = device.create_buffer_with_data(bytes, wgpu::BufferUsage::COPY_SRC);
            encoder.copy_buffer_to_texture(
                wgpu::BufferCopyView {
                    buffer: &temp_buf,
                    offset: 0,
                    row_pitch: 4 * descriptor.size.width,
                    image_height: descriptor.size.height,
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

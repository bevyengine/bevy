use crate::{
    legion::prelude::*,
    render::render_graph_2::{
        resource_name, BindGroup, BindType, PassDescriptor, PipelineDescriptor, RenderGraph,
        RenderPass, RenderPassColorAttachmentDescriptor,
        RenderPassDepthStencilAttachmentDescriptor, Renderer, ResourceInfo, ShaderUniforms,
        TextureDimension,
    },
};
use std::{
    collections::{hash_map::DefaultHasher, HashMap},
    hash::{Hash, Hasher},
    ops::Deref,
};

pub struct WgpuRenderer {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub surface: Option<wgpu::Surface>,
    pub swap_chain_descriptor: wgpu::SwapChainDescriptor,
    pub render_pipelines: HashMap<String, wgpu::RenderPipeline>,
    pub buffers: HashMap<String, wgpu::Buffer>,
    pub textures: HashMap<String, wgpu::TextureView>,
    pub resource_info: HashMap<String, ResourceInfo>,
    pub bind_groups: HashMap<u64, BindGroupInfo>,
    pub bind_group_layouts: HashMap<u64, wgpu::BindGroupLayout>,
}

impl WgpuRenderer {
    pub fn new() -> Self {
        let adapter = wgpu::Adapter::request(
            &wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::Default,
            },
            wgpu::BackendBit::PRIMARY,
        )
        .unwrap();

        let (device, queue) = adapter.request_device(&wgpu::DeviceDescriptor {
            extensions: wgpu::Extensions {
                anisotropic_filtering: false,
            },
            limits: wgpu::Limits::default(),
        });

        let swap_chain_descriptor = wgpu::SwapChainDescriptor {
            usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT,
            format: wgpu::TextureFormat::Bgra8UnormSrgb,
            width: 0,
            height: 0,
            present_mode: wgpu::PresentMode::Vsync,
        };

        WgpuRenderer {
            device,
            queue,
            surface: None,
            swap_chain_descriptor,
            render_pipelines: HashMap::new(),
            buffers: HashMap::new(),
            textures: HashMap::new(),
            resource_info: HashMap::new(),
            bind_groups: HashMap::new(),
            bind_group_layouts: HashMap::new(),
        }
    }

    pub fn create_render_pipeline(
        pipeline_descriptor: &PipelineDescriptor,
        bind_group_layouts: &mut HashMap<u64, wgpu::BindGroupLayout>,
        device: &wgpu::Device,
    ) -> wgpu::RenderPipeline {
        let vertex_shader_module = pipeline_descriptor
            .shader_stages
            .vertex
            .create_shader_module(device);
        let fragment_shader_module = match pipeline_descriptor.shader_stages.fragment {
            Some(ref fragment_shader) => Some(fragment_shader.create_shader_module(device)),
            None => None,
        };

        // setup new bind group layouts
        for bind_group in pipeline_descriptor.pipeline_layout.bind_groups.iter() {
            let mut hasher = DefaultHasher::new();
            bind_group.hash(&mut hasher);
            let bind_group_id = hasher.finish();
            if let None = bind_group_layouts.get(&bind_group_id) {
                let bind_group_layout_binding = bind_group
                    .bindings
                    .iter()
                    .enumerate()
                    .map(|(i, binding)| wgpu::BindGroupLayoutBinding {
                        binding: i as u32,
                        visibility: wgpu::ShaderStage::VERTEX | wgpu::ShaderStage::FRAGMENT,
                        ty: (&binding.bind_type).into(),
                    })
                    .collect::<Vec<wgpu::BindGroupLayoutBinding>>();
                let bind_group_layout =
                    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                        bindings: bind_group_layout_binding.as_slice(),
                    });

                bind_group_layouts.insert(bind_group_id, bind_group_layout);
            }
        }

        // collect bind group layout references
        let bind_group_layouts = pipeline_descriptor
            .pipeline_layout
            .bind_groups
            .iter()
            .map(|bind_group| {
                let mut hasher = DefaultHasher::new();
                bind_group.hash(&mut hasher);
                let bind_group_id = hasher.finish();

                bind_group_layouts.get(&bind_group_id).unwrap()
            })
            .collect::<Vec<&wgpu::BindGroupLayout>>();

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            bind_group_layouts: bind_group_layouts.as_slice(),
        });

        let render_pipeline_descriptor = wgpu::RenderPipelineDescriptor {
            layout: &pipeline_layout,
            vertex_stage: wgpu::ProgrammableStageDescriptor {
                module: &vertex_shader_module,
                entry_point: &pipeline_descriptor.shader_stages.vertex.entry_point,
            },
            fragment_stage: match pipeline_descriptor.shader_stages.fragment {
                Some(ref fragment_shader) => Some(wgpu::ProgrammableStageDescriptor {
                    entry_point: &fragment_shader.entry_point,
                    module: fragment_shader_module.as_ref().unwrap(),
                }),
                None => None,
            },
            rasterization_state: pipeline_descriptor.rasterization_state.clone(),
            primitive_topology: pipeline_descriptor.primitive_topology,
            color_states: &pipeline_descriptor.color_states,
            depth_stencil_state: pipeline_descriptor.depth_stencil_state.clone(),
            index_format: pipeline_descriptor.index_format,
            vertex_buffers: &pipeline_descriptor
                .vertex_buffer_descriptors
                .iter()
                .map(|v| v.into())
                .collect::<Vec<wgpu::VertexBufferDescriptor>>(),
            sample_count: pipeline_descriptor.sample_count,
            sample_mask: pipeline_descriptor.sample_mask,
            alpha_to_coverage_enabled: pipeline_descriptor.alpha_to_coverage_enabled,
        };

        device.create_render_pipeline(&render_pipeline_descriptor)
    }

    pub fn create_render_pass<'a>(
        &self,
        pass_descriptor: &PassDescriptor,
        encoder: &'a mut wgpu::CommandEncoder,
        frame: &'a wgpu::SwapChainOutput,
    ) -> wgpu::RenderPass<'a> {
        encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            color_attachments: &pass_descriptor
                .color_attachments
                .iter()
                .map(|c| self.create_wgpu_color_attachment_descriptor(c, frame))
                .collect::<Vec<wgpu::RenderPassColorAttachmentDescriptor>>(),
            depth_stencil_attachment: pass_descriptor
                .depth_stencil_attachment
                .as_ref()
                .map(|d| self.create_wgpu_depth_stencil_attachment_descriptor(d, frame)),
        })
    }

    fn create_wgpu_color_attachment_descriptor<'a>(
        &'a self,
        color_attachment_descriptor: &RenderPassColorAttachmentDescriptor,
        frame: &'a wgpu::SwapChainOutput,
    ) -> wgpu::RenderPassColorAttachmentDescriptor<'a> {
        let attachment = match color_attachment_descriptor.attachment.as_str() {
            resource_name::texture::SWAP_CHAIN => &frame.view,
            _ => self
                .textures
                .get(&color_attachment_descriptor.attachment)
                .unwrap(),
        };

        let resolve_target = match color_attachment_descriptor.resolve_target {
            Some(ref target) => match target.as_str() {
                resource_name::texture::SWAP_CHAIN => Some(&frame.view),
                _ => Some(&frame.view),
            },
            None => None,
        };

        wgpu::RenderPassColorAttachmentDescriptor {
            store_op: color_attachment_descriptor.store_op,
            load_op: color_attachment_descriptor.load_op,
            clear_color: color_attachment_descriptor.clear_color,
            attachment,
            resolve_target,
        }
    }

    fn create_wgpu_depth_stencil_attachment_descriptor<'a>(
        &'a self,
        depth_stencil_attachment_descriptor: &RenderPassDepthStencilAttachmentDescriptor,
        frame: &'a wgpu::SwapChainOutput,
    ) -> wgpu::RenderPassDepthStencilAttachmentDescriptor<&'a wgpu::TextureView> {
        let attachment = match depth_stencil_attachment_descriptor.attachment.as_str() {
            resource_name::texture::SWAP_CHAIN => &frame.view,
            _ => self
                .textures
                .get(&depth_stencil_attachment_descriptor.attachment)
                .unwrap(),
        };

        wgpu::RenderPassDepthStencilAttachmentDescriptor {
            attachment,
            clear_depth: depth_stencil_attachment_descriptor.clear_depth,
            clear_stencil: depth_stencil_attachment_descriptor.clear_stencil,
            depth_load_op: depth_stencil_attachment_descriptor.depth_load_op,
            depth_store_op: depth_stencil_attachment_descriptor.depth_store_op,
            stencil_load_op: depth_stencil_attachment_descriptor.stencil_load_op,
            stencil_store_op: depth_stencil_attachment_descriptor.stencil_store_op,
        }
    }

    fn add_resource_info(&mut self, name: &str, resource_info: ResourceInfo) {
        self.resource_info.insert(name.to_string(), resource_info);
    }

    // TODO: consider moving this to a resource provider
    fn setup_bind_group(&mut self, bind_group: &BindGroup) -> u64 {
        // TODO: cache hash result in bind_group?
        let mut hasher = DefaultHasher::new();
        bind_group.hash(&mut hasher);
        let bind_group_id = hasher.finish();

        if let None = self.bind_groups.get(&bind_group_id) {
            let mut unset_uniforms = Vec::new();
            // if a uniform resource buffer doesn't exist, create a new empty one
            for binding in bind_group.bindings.iter() {
                if let None = self.resource_info.get(&binding.name) {
                    unset_uniforms.push(binding.name.to_string());
                    if let BindType::Uniform { .. } = &binding.bind_type {
                        let size = binding.bind_type.get_uniform_size().unwrap();
                        self.create_buffer(
                            &binding.name,
                            size,
                            wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST,
                        )
                    }
                }
            }

            // create wgpu Bindings
            let bindings = bind_group
                .bindings
                .iter()
                .enumerate()
                .map(|(i, b)| {
                    let resource_info = self.resource_info.get(&b.name).unwrap();
                    wgpu::Binding {
                        binding: i as u32,
                        resource: match &b.bind_type {
                            BindType::Uniform {
                                dynamic,
                                properties,
                            } => {
                                if let ResourceInfo::Buffer { size, buffer_usage } = resource_info {
                                    let buffer = self.buffers.get(&b.name).unwrap();
                                    wgpu::BindingResource::Buffer {
                                        buffer: buffer,
                                        range: 0..*size,
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

            let bind_group = self.device.create_bind_group(&bind_group_descriptor);
            self.bind_groups.insert(
                bind_group_id,
                BindGroupInfo {
                    bind_group,
                    unset_uniforms,
                },
            );
        }

        bind_group_id
    }

    fn setup_entity_shader_uniforms(
        &mut self,
        bind_group: &BindGroup,
        world: &World,
        entity: Entity,
        shader_uniforms: &ShaderUniforms,
        encoder: &mut wgpu::CommandEncoder,
    ) {
        // TODO: cache hash result in bind_group?
        let mut hasher = DefaultHasher::new();
        bind_group.hash(&mut hasher);
        let bind_group_id = hasher.finish();
        let bind_group_info = self.bind_groups.get(&bind_group_id).unwrap();
        for unset_uniform in bind_group_info.unset_uniforms.iter() {
            let mut found_uniform = false;
            for uniform_selector in shader_uniforms.uniform_selectors.iter().rev() {
                let uniforms = uniform_selector(entity, world).unwrap_or_else(|| {
                    panic!(
                        "ShaderUniform selector points to a missing component. Uniform: {}",
                        unset_uniform
                    )
                });
                if let Some(bytes) = uniforms.get_uniform_bytes(unset_uniform) {
                    // TODO: validate bind_group layout vs shader uniform
                    let temp_buffer = self
                        .device
                        .create_buffer_with_data(bytes.as_slice(), wgpu::BufferUsage::COPY_SRC);
                    let uniform_buffer = self.buffers.get(unset_uniform).unwrap();

                    encoder.copy_buffer_to_buffer(
                        &temp_buffer,
                        0,
                        uniform_buffer,
                        0,
                        bytes.len() as u64,
                    );

                    found_uniform = true;
                    break;
                }
            }

            if !found_uniform {
                panic!("ShaderUniform did not find a source for Uniform: {}. Consider adding a uniform selector to this entity's ShaderUniforms component.", unset_uniform);
            }
        }
    }

    fn setup_dynamic_entity_shader_uniforms(&mut self, world: &World, render_graph: &RenderGraph, encoder: &mut wgpu::CommandEncoder) {
        let mut dynamic_uniform_info = HashMap::new();

        // retrieve all uniforms buffers that aren't aleady set. these are "dynamic" uniforms, which are set by the user in ShaderUniforms
        // TODO: this breaks down in multiple ways:
        // (1) resource_info will be set after the first run so this won't update.
        // (2) if we create new buffers, the old bind groups will be invalid
        for pipeline in render_graph.pipeline_descriptors.values() {
            for bind_group in pipeline.pipeline_layout.bind_groups.iter() {
                for binding in bind_group.bindings.iter() {
                    // if let None = self.resource_info.get(&binding.name) {
                        if let BindType::Uniform { dynamic: true, .. } = &binding.bind_type {
                            if dynamic_uniform_info.contains_key(&binding.name) {
                                continue;
                            }

                            dynamic_uniform_info.insert(binding.name.to_string(), UniformInfo {
                                size: binding.bind_type.get_uniform_size().unwrap(),
                                count: 0,
                            });
                        }
                    // }
                }
            }
        }

        // count the number of entities providing each uniform 
        for (name, info) in dynamic_uniform_info.iter_mut() {
            for (entity, shader_uniforms) in <Read<ShaderUniforms>>::query().iter_entities(world) {
                if let Some(_) = shader_uniforms.get_uniform_info(world, entity, name) {
                    info.count += 1;
                    // TODO: assign indices to shader_uniforms here
                }
            }
        }
        
        // allocate uniform buffers
        for (name, info) in dynamic_uniform_info.iter() {
            let size = wgpu::BIND_BUFFER_ALIGNMENT * info.count;
            println!("{} {} {}", name, info.size, info.count);
            if self.buffers.contains_key(name) {
                continue;
            }

            self.create_buffer(name, size, wgpu::BufferUsage::COPY_DST | wgpu::BufferUsage::UNIFORM);
        }

        // copy entity uniform data to buffers
        for (name, info) in dynamic_uniform_info.iter_mut() {
            let size = wgpu::BIND_BUFFER_ALIGNMENT * info.count;
            let mapped = self.device.create_buffer_mapped(size as usize, wgpu::BufferUsage::COPY_SRC);
            for ((entity, shader_uniforms), slot) in <Read<ShaderUniforms>>::query().iter_entities(world).zip(mapped.data.chunks_exact_mut(wgpu::BIND_BUFFER_ALIGNMENT as usize)) {
                if let Some(bytes) = shader_uniforms.get_uniform_bytes(world, entity, name) {
                    // TODO: make this zero-copy somehow
                    let mut new_bytes = bytes.clone();
                    while new_bytes.len() < (wgpu::BIND_BUFFER_ALIGNMENT as usize) {
                        new_bytes.push(0);
                    }
                    slot.copy_from_slice(new_bytes.as_slice());
                }
            }

            let temp_buffer = mapped.finish();
            let uniform_buffer = self.buffers.get(name);
            encoder.copy_buffer_to_buffer(&temp_buffer, 0, uniform_buffer.unwrap(), 0, size);
        }
    }
}

pub struct UniformInfo {
    pub size: u64,
    pub count: u64,
}

impl Renderer for WgpuRenderer {
    fn initialize(&mut self, world: &mut World, render_graph: &mut RenderGraph) {
        let (surface, window_size) = {
            let window = world.resources.get::<winit::window::Window>().unwrap();
            let surface = wgpu::Surface::create(window.deref());
            let window_size = window.inner_size();
            (surface, window_size)
        };

        self.surface = Some(surface);
        self.resize(world, render_graph, window_size.width, window_size.height);
        for resource_provider in render_graph.resource_providers.iter_mut() {
            resource_provider.initialize(self, world);
        }
    }

    fn resize(
        &mut self,
        world: &mut World,
        render_graph: &mut RenderGraph,
        width: u32,
        height: u32,
    ) {
        self.swap_chain_descriptor.width = width;
        self.swap_chain_descriptor.height = height;
        let swap_chain = self
            .device
            .create_swap_chain(self.surface.as_ref().unwrap(), &self.swap_chain_descriptor);

        // WgpuRenderer can't own swap_chain without creating lifetime ergonomics issues, so lets just store it in World.
        world.resources.insert(swap_chain);
        for resource_provider in render_graph.resource_providers.iter_mut() {
            resource_provider.resize(self, world, width, height);
        }
    }

    fn process_render_graph(&mut self, render_graph: &mut RenderGraph, world: &mut World) {
        for resource_provider in render_graph.resource_providers.iter_mut() {
            resource_provider.update(self, world);
        }

        let mut swap_chain = world.resources.get_mut::<wgpu::SwapChain>().unwrap();
        let frame = swap_chain
            .get_next_texture()
            .expect("Timeout when acquiring next swap chain texture");

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { todo: 0 });
        
        self.setup_dynamic_entity_shader_uniforms(world, render_graph, &mut encoder);

        // setup, pipelines, bind groups, and resources
        for (pipeline_name, pipeline_descriptor) in render_graph.pipeline_descriptors.iter() {
            // create pipelines
            if let None = self.render_pipelines.get(pipeline_name) {
                let render_pipeline = WgpuRenderer::create_render_pipeline(
                    pipeline_descriptor,
                    &mut self.bind_group_layouts,
                    &self.device,
                );
                self.render_pipelines
                    .insert(pipeline_name.to_string(), render_pipeline);
            }

            // create bind groups
            for bind_group in pipeline_descriptor.pipeline_layout.bind_groups.iter() {
                self.setup_bind_group(bind_group);
            }
        }

        for (pass_name, pass_descriptor) in render_graph.pass_descriptors.iter() {
            // run passes
            let mut render_pass = self.create_render_pass(pass_descriptor, &mut encoder, &frame);
            if let Some(pass_pipelines) = render_graph.pass_pipelines.get(pass_name) {
                for pass_pipeline in pass_pipelines.iter() {
                    if let Some(pipeline_descriptor) =
                        render_graph.pipeline_descriptors.get(pass_pipeline)
                    {
                        let render_pipeline = self.render_pipelines.get(pass_pipeline).unwrap();
                        render_pass.set_pipeline(render_pipeline);

                        let mut render_pass = WgpuRenderPass {
                            render_pass: &mut render_pass,
                            renderer: self,
                            pipeline_descriptor,
                        };

                        for draw_target in pipeline_descriptor.draw_targets.iter() {
                            draw_target(world, &mut render_pass);
                        }
                    }
                }
            }
        }

        let command_buffer = encoder.finish();
        self.queue.submit(&[command_buffer]);
    }

    fn create_buffer_with_data(
        &mut self,
        name: &str,
        data: &[u8],
        buffer_usage: wgpu::BufferUsage,
    ) {
        let buffer = self.device.create_buffer_with_data(data, buffer_usage);
        self.add_resource_info(
            name,
            ResourceInfo::Buffer {
                buffer_usage,
                size: data.len() as u64,
            },
        );

        self.buffers.insert(name.to_string(), buffer);
    }

    fn create_buffer(&mut self, name: &str, size: u64, buffer_usage: wgpu::BufferUsage) {
        let buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            size: size,
            usage: buffer_usage,
        });

        self.add_resource_info(
            name,
            ResourceInfo::Buffer {
                buffer_usage,
                size: size,
            },
        );

        self.buffers.insert(name.to_string(), buffer);
    }

    fn get_resource_info(&self, name: &str) -> Option<&ResourceInfo> {
        self.resource_info.get(name)
    }

    fn remove_buffer(&mut self, name: &str) {
        self.buffers.remove(name);
    }
}

pub struct WgpuRenderPass<'a, 'b, 'c, 'd> {
    pub render_pass: &'b mut wgpu::RenderPass<'a>,
    pub pipeline_descriptor: &'c PipelineDescriptor,
    pub renderer: &'d mut WgpuRenderer,
}

impl<'a, 'b, 'c, 'd> RenderPass for WgpuRenderPass<'a, 'b, 'c, 'd> {
    fn get_renderer(&mut self) -> &mut dyn Renderer {
        self.renderer
    }

    fn get_pipeline_descriptor(&self) -> &PipelineDescriptor {
        self.pipeline_descriptor
    }

    fn set_vertex_buffer(&mut self, start_slot: u32, name: &str, offset: u64) {
        let buffer = self.renderer.buffers.get(name).unwrap();
        self.render_pass
            .set_vertex_buffers(start_slot, &[(&buffer, offset)]);
    }

    fn set_index_buffer(&mut self, name: &str, offset: u64) {
        let buffer = self.renderer.buffers.get(name).unwrap();
        self.render_pass.set_index_buffer(&buffer, offset);
    }

    fn draw_indexed(
        &mut self,
        indices: core::ops::Range<u32>,
        base_vertex: i32,
        instances: core::ops::Range<u32>,
    ) {
        self.render_pass
            .draw_indexed(indices, base_vertex, instances);
    }

    // TODO: maybe move setup to renderer.setup_bind_groups(&pipeline_desc);
    fn setup_bind_groups(&mut self, shader_uniforms: &ShaderUniforms) {
        for (i, bind_group) in self
            .pipeline_descriptor
            .pipeline_layout
            .bind_groups
            .iter()
            .enumerate()
        {
            // TODO: cache hash result in bind_group?
            let mut hasher = DefaultHasher::new();
            bind_group.hash(&mut hasher);
            let bind_group_id = hasher.finish();
            let bind_group_info = self.renderer.bind_groups.get(&bind_group_id).unwrap();

            let mut dynamic_uniform_indices = Vec::new();
            for binding in bind_group.bindings.iter() {
                if let BindType::Uniform { dynamic, ..} = binding.bind_type {
                    if !dynamic {
                        continue;
                    }

                    if let Some(index) = shader_uniforms.dynamic_uniform_indices.get(&binding.name) {
                        dynamic_uniform_indices.push(*index);
                    }
                }
            } 

            self.render_pass
                .set_bind_group(i as u32, &bind_group_info.bind_group, dynamic_uniform_indices.as_slice());
        }
    }
}

impl From<TextureDimension> for wgpu::TextureViewDimension {
    fn from(dimension: TextureDimension) -> Self {
        match dimension {
            TextureDimension::D1 => wgpu::TextureViewDimension::D1,
            TextureDimension::D2 => wgpu::TextureViewDimension::D2,
            TextureDimension::D2Array => wgpu::TextureViewDimension::D2Array,
            TextureDimension::Cube => wgpu::TextureViewDimension::Cube,
            TextureDimension::CubeArray => wgpu::TextureViewDimension::CubeArray,
            TextureDimension::D3 => wgpu::TextureViewDimension::D3,
        }
    }
}

impl From<&BindType> for wgpu::BindingType {
    fn from(bind_type: &BindType) -> Self {
        match bind_type {
            BindType::Uniform {
                dynamic,
                properties: _,
            } => wgpu::BindingType::UniformBuffer { dynamic: *dynamic },
            BindType::Buffer { dynamic, readonly } => wgpu::BindingType::StorageBuffer {
                dynamic: *dynamic,
                readonly: *readonly,
            },
            BindType::SampledTexture {
                dimension,
                multisampled,
            } => wgpu::BindingType::SampledTexture {
                dimension: (*dimension).into(),
                multisampled: *multisampled,
            },
            BindType::Sampler => wgpu::BindingType::Sampler,
            BindType::StorageTexture { dimension } => wgpu::BindingType::StorageTexture {
                dimension: (*dimension).into(),
            },
        }
    }
}

pub struct BindGroupInfo {
    pub bind_group: wgpu::BindGroup,
    pub unset_uniforms: Vec<String>,
}

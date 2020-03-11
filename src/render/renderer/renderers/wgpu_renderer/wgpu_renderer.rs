use super::{wgpu_type_converter::OwnedWgpuVertexBufferDescriptor, WgpuRenderPass, WgpuResources};
use crate::{
    asset::{AssetStorage, Handle},
    legion::prelude::*,
    render::{
        pass::{
            PassDescriptor, RenderPassColorAttachmentDescriptor,
            RenderPassDepthStencilAttachmentDescriptor,
        },
        pipeline::{BindType, PipelineDescriptor, PipelineLayout, PipelineLayoutType},
        render_graph::RenderGraph,
        render_resource::{
            resource_name, BufferUsage, RenderResource, RenderResources, ResourceInfo,
        },
        renderer::Renderer,
        shader::{DynamicUniformBufferInfo, Shader},
        texture::{SamplerDescriptor, TextureDescriptor},
        update_shader_assignments,
    },
};
use std::{collections::HashMap, ops::Deref};

pub struct WgpuRenderer {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub surface: Option<wgpu::Surface>,
    pub encoder: Option<wgpu::CommandEncoder>,
    pub swap_chain_descriptor: wgpu::SwapChainDescriptor,
    pub render_pipelines: HashMap<Handle<PipelineDescriptor>, wgpu::RenderPipeline>,
    pub wgpu_resources: WgpuResources,
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
            encoder: None,
            swap_chain_descriptor,
            wgpu_resources: WgpuResources::new(),
            render_pipelines: HashMap::new(),
        }
    }

    pub fn create_render_pipeline(
        render_resources: &RenderResources,
        dynamic_uniform_buffer_info: &HashMap<RenderResource, DynamicUniformBufferInfo>,
        pipeline_descriptor: &mut PipelineDescriptor,
        bind_group_layouts: &mut HashMap<u64, wgpu::BindGroupLayout>,
        device: &wgpu::Device,
        vertex_shader: &Shader,
        fragment_shader: Option<&Shader>,
    ) -> wgpu::RenderPipeline {
        let vertex_spirv = vertex_shader.get_spirv_shader(None);
        let fragment_spirv = fragment_shader.map(|f| f.get_spirv_shader(None));

        let vertex_shader_module = Self::create_shader_module(device, &vertex_spirv, None);
        let fragment_shader_module = match fragment_shader {
            Some(fragment_spirv) => Some(Self::create_shader_module(device, fragment_spirv, None)),
            None => None,
        };

        if let PipelineLayoutType::Reflected(None) = pipeline_descriptor.layout {
            let mut layouts = vec![vertex_spirv.reflect_layout().unwrap()];

            if let Some(ref fragment_spirv) = fragment_spirv {
                layouts.push(fragment_spirv.reflect_layout().unwrap());
            }

            let mut layout = PipelineLayout::from_shader_layouts(&mut layouts);

            // set each uniform binding to dynamic if there is a matching dynamic uniform buffer info
            for mut bind_group in layout.bind_groups.iter_mut() {
                bind_group.bindings = bind_group
                    .bindings
                    .iter()
                    .cloned()
                    .map(|mut binding| {
                        if let BindType::Uniform {
                            ref mut dynamic, ..
                        } = binding.bind_type
                        {
                            if let Some(resource) =
                                render_resources.get_named_resource(&binding.name)
                            {
                                if dynamic_uniform_buffer_info.contains_key(&resource) {
                                    *dynamic = true;
                                }
                            }
                        }

                        binding
                    })
                    .collect();
            }

            pipeline_descriptor.layout = PipelineLayoutType::Reflected(Some(layout));
        }

        let layout = pipeline_descriptor.get_layout_mut().unwrap();
        // println!("{:#?}", layout);
        // println!();

        // setup new bind group layouts
        for bind_group in layout.bind_groups.iter_mut() {
            let bind_group_id = bind_group.get_or_update_hash();
            if let None = bind_group_layouts.get(&bind_group_id) {
                let bind_group_layout_binding = bind_group
                    .bindings
                    .iter()
                    .map(|binding| wgpu::BindGroupLayoutBinding {
                        binding: binding.index,
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
        let bind_group_layouts = layout
            .bind_groups
            .iter()
            .map(|bind_group| {
                let bind_group_id = bind_group.get_hash().unwrap();
                bind_group_layouts.get(&bind_group_id).unwrap()
            })
            .collect::<Vec<&wgpu::BindGroupLayout>>();

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            bind_group_layouts: bind_group_layouts.as_slice(),
        });

        let owned_vertex_buffer_descriptors = pipeline_descriptor
            .vertex_buffer_descriptors
            .iter()
            .map(|v| v.into())
            .collect::<Vec<OwnedWgpuVertexBufferDescriptor>>();

        let color_states = pipeline_descriptor
            .color_states
            .iter()
            .map(|c| c.into())
            .collect::<Vec<wgpu::ColorStateDescriptor>>();

        let mut render_pipeline_descriptor = wgpu::RenderPipelineDescriptor {
            layout: &pipeline_layout,
            vertex_stage: wgpu::ProgrammableStageDescriptor {
                module: &vertex_shader_module,
                entry_point: "main",
            },
            fragment_stage: match fragment_shader {
                Some(_) => Some(wgpu::ProgrammableStageDescriptor {
                    entry_point: "main",
                    module: fragment_shader_module.as_ref().unwrap(),
                }),
                None => None,
            },
            rasterization_state: pipeline_descriptor
                .rasterization_state
                .as_ref()
                .map(|r| r.into()),
            primitive_topology: pipeline_descriptor.primitive_topology.into(),
            color_states: &color_states,
            depth_stencil_state: pipeline_descriptor
                .depth_stencil_state
                .as_ref()
                .map(|d| d.into()),
            index_format: pipeline_descriptor.index_format.into(),
            vertex_buffers: &owned_vertex_buffer_descriptors
                .iter()
                .map(|v| v.into())
                .collect::<Vec<wgpu::VertexBufferDescriptor>>(),
            sample_count: pipeline_descriptor.sample_count,
            sample_mask: pipeline_descriptor.sample_mask,
            alpha_to_coverage_enabled: pipeline_descriptor.alpha_to_coverage_enabled,
        };

        device.create_render_pipeline(&mut render_pipeline_descriptor)
    }

    pub fn create_render_pass<'a>(
        wgpu_resources: &'a WgpuResources,
        pass_descriptor: &PassDescriptor,
        encoder: &'a mut wgpu::CommandEncoder,
        frame: &'a wgpu::SwapChainOutput,
    ) -> wgpu::RenderPass<'a> {
        encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            color_attachments: &pass_descriptor
                .color_attachments
                .iter()
                .map(|c| Self::create_wgpu_color_attachment_descriptor(wgpu_resources, c, frame))
                .collect::<Vec<wgpu::RenderPassColorAttachmentDescriptor>>(),
            depth_stencil_attachment: pass_descriptor.depth_stencil_attachment.as_ref().map(|d| {
                Self::create_wgpu_depth_stencil_attachment_descriptor(wgpu_resources, d, frame)
            }),
        })
    }

    fn create_wgpu_color_attachment_descriptor<'a>(
        wgpu_resources: &'a WgpuResources,
        color_attachment_descriptor: &RenderPassColorAttachmentDescriptor,
        frame: &'a wgpu::SwapChainOutput,
    ) -> wgpu::RenderPassColorAttachmentDescriptor<'a> {
        let attachment = match color_attachment_descriptor.attachment.as_str() {
            resource_name::texture::SWAP_CHAIN => &frame.view,
            _ => {
                match wgpu_resources
                    .render_resources
                    .get_named_resource(&color_attachment_descriptor.attachment)
                {
                    Some(resource) => wgpu_resources.textures.get(&resource).unwrap(),
                    None => panic!(
                        "Color attachment {} does not exist",
                        &color_attachment_descriptor.attachment
                    ),
                }
            }
        };

        let resolve_target = match color_attachment_descriptor.resolve_target {
            Some(ref target) => match target.as_str() {
                resource_name::texture::SWAP_CHAIN => Some(&frame.view),
                _ => match wgpu_resources
                    .render_resources
                    .get_named_resource(target.as_str())
                {
                    Some(resource) => Some(wgpu_resources.textures.get(&resource).unwrap()),
                    None => panic!(
                        "Color attachment {} does not exist",
                        &color_attachment_descriptor.attachment
                    ),
                },
            },
            None => None,
        };

        wgpu::RenderPassColorAttachmentDescriptor {
            store_op: color_attachment_descriptor.store_op.into(),
            load_op: color_attachment_descriptor.load_op.into(),
            clear_color: color_attachment_descriptor.clear_color.into(),
            attachment,
            resolve_target,
        }
    }

    fn create_wgpu_depth_stencil_attachment_descriptor<'a>(
        wgpu_resources: &'a WgpuResources,
        depth_stencil_attachment_descriptor: &RenderPassDepthStencilAttachmentDescriptor,
        frame: &'a wgpu::SwapChainOutput,
    ) -> wgpu::RenderPassDepthStencilAttachmentDescriptor<'a> {
        let attachment = match depth_stencil_attachment_descriptor.attachment.as_str() {
            resource_name::texture::SWAP_CHAIN => &frame.view,
            _ => {
                match wgpu_resources
                    .render_resources
                    .get_named_resource(&depth_stencil_attachment_descriptor.attachment)
                {
                    Some(ref resource) => wgpu_resources.textures.get(&resource).unwrap(),
                    None => panic!(
                        "Depth stencil attachment {} does not exist",
                        &depth_stencil_attachment_descriptor.attachment
                    ),
                }
            }
        };

        wgpu::RenderPassDepthStencilAttachmentDescriptor {
            attachment,
            clear_depth: depth_stencil_attachment_descriptor.clear_depth,
            clear_stencil: depth_stencil_attachment_descriptor.clear_stencil,
            depth_load_op: depth_stencil_attachment_descriptor.depth_load_op.into(),
            depth_store_op: depth_stencil_attachment_descriptor.depth_store_op.into(),
            stencil_load_op: depth_stencil_attachment_descriptor.stencil_load_op.into(),
            stencil_store_op: depth_stencil_attachment_descriptor.stencil_store_op.into(),
        }
    }

    pub fn create_shader_module(
        device: &wgpu::Device,
        shader: &Shader,
        macros: Option<&[String]>,
    ) -> wgpu::ShaderModule {
        device.create_shader_module(&shader.get_spirv(macros))
    }

    pub fn initialize_resource_providers(
        &mut self,
        world: &mut World,
        resources: &mut Resources,
        render_graph: &mut RenderGraph,
    ) {
        self.encoder = Some(
            self.device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor { todo: 0 }),
        );
        for resource_provider in render_graph.resource_providers.iter_mut() {
            resource_provider.initialize(self, world, resources);
        }

        // consume current encoder
        let command_buffer = self.encoder.take().unwrap().finish();
        self.queue.submit(&[command_buffer]);
    }
}

impl Renderer for WgpuRenderer {
    fn initialize(
        &mut self,
        world: &mut World,
        resources: &mut Resources,
        render_graph: &mut RenderGraph,
    ) {
        let (surface, window_size) = {
            let window = resources.get::<winit::window::Window>().unwrap();
            let surface = wgpu::Surface::create(window.deref());
            let window_size = window.inner_size();
            (surface, window_size)
        };

        self.surface = Some(surface);

        self.initialize_resource_providers(world, resources, render_graph);

        self.resize(
            world,
            resources,
            render_graph,
            window_size.width,
            window_size.height,
        );
    }

    fn resize(
        &mut self,
        world: &mut World,
        resources: &mut Resources,
        render_graph: &mut RenderGraph,
        width: u32,
        height: u32,
    ) {
        self.encoder = Some(
            self.device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor { todo: 0 }),
        );
        self.swap_chain_descriptor.width = width;
        self.swap_chain_descriptor.height = height;
        let swap_chain = self
            .device
            .create_swap_chain(self.surface.as_ref().unwrap(), &self.swap_chain_descriptor);

        // WgpuRenderer can't own swap_chain without creating lifetime ergonomics issues, so lets just store it in World.
        resources.insert(swap_chain);
        for resource_provider in render_graph.resource_providers.iter_mut() {
            resource_provider.resize(self, world, resources, width, height);
        }

        // consume current encoder
        let command_buffer = self.encoder.take().unwrap().finish();
        self.queue.submit(&[command_buffer]);
    }

    fn process_render_graph(
        &mut self,
        render_graph: &mut RenderGraph,
        world: &mut World,
        resources: &mut Resources,
    ) {
        // TODO: this self.encoder handoff is a bit gross, but its here to give resource providers access to buffer copies without
        // exposing the wgpu renderer internals to ResourceProvider traits. if this can be made cleaner that would be pretty cool.
        self.encoder = Some(
            self.device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor { todo: 0 }),
        );

        for resource_provider in render_graph.resource_providers.iter_mut() {
            resource_provider.update(self, world, resources);
        }

        update_shader_assignments(world, resources, render_graph);

        for (name, texture_descriptor) in render_graph.queued_textures.drain(..) {
            let resource = self.create_texture(&texture_descriptor, None);
            self.wgpu_resources
                .render_resources
                .set_named_resource(&name, resource);
        }

        let mut encoder = self.encoder.take().unwrap();

        let mut swap_chain = resources.get_mut::<wgpu::SwapChain>().unwrap();
        let frame = swap_chain
            .get_next_texture()
            .expect("Timeout when acquiring next swap chain texture");

        // self.setup_dynamic_entity_shader_uniforms(world, render_graph, &mut encoder);

        // setup, pipelines, bind groups, and resources
        let mut pipeline_storage = resources
            .get_mut::<AssetStorage<PipelineDescriptor>>()
            .unwrap();
        let shader_storage = resources.get::<AssetStorage<Shader>>().unwrap();

        for pipeline_descriptor_handle in render_graph.pipeline_descriptors.iter() {
            let pipeline_descriptor = pipeline_storage
                .get_mut(pipeline_descriptor_handle)
                .unwrap();
            // create pipelines
            if !self
                .render_pipelines
                .contains_key(pipeline_descriptor_handle)
            {
                let vertex_shader = shader_storage
                    .get(&pipeline_descriptor.shader_stages.vertex)
                    .unwrap();
                let fragment_shader = pipeline_descriptor
                    .shader_stages
                    .fragment
                    .as_ref()
                    .map(|handle| &*shader_storage.get(&handle).unwrap());
                let render_pipeline = WgpuRenderer::create_render_pipeline(
                    &self.wgpu_resources.render_resources,
                    &self.wgpu_resources.dynamic_uniform_buffer_info,
                    pipeline_descriptor,
                    &mut self.wgpu_resources.bind_group_layouts,
                    &self.device,
                    vertex_shader,
                    fragment_shader,
                );
                self.render_pipelines
                    .insert(*pipeline_descriptor_handle, render_pipeline);
            }

            // create bind groups
            let pipeline_layout = pipeline_descriptor.get_layout().unwrap();
            for bind_group in pipeline_layout.bind_groups.iter() {
                self.wgpu_resources
                    .setup_bind_group(&self.device, bind_group);
            }
        }

        // setup draw targets
        for (pass_name, _pass_descriptor) in render_graph.pass_descriptors.iter() {
            if let Some(pass_pipelines) = render_graph.pass_pipelines.get(pass_name) {
                for pass_pipeline in pass_pipelines.iter() {
                    let pipeline_descriptor = pipeline_storage.get(pass_pipeline).unwrap();
                    for draw_target_name in pipeline_descriptor.draw_targets.iter() {
                        let draw_target =
                            render_graph.draw_targets.get_mut(draw_target_name).unwrap();
                        draw_target.setup(world, resources, self, *pass_pipeline);
                    }
                }
            }
        }

        // begin render passes
        for (pass_name, pass_descriptor) in render_graph.pass_descriptors.iter() {
            let mut render_pass = Self::create_render_pass(
                &self.wgpu_resources,
                pass_descriptor,
                &mut encoder,
                &frame,
            );
            if let Some(pass_pipelines) = render_graph.pass_pipelines.get(pass_name) {
                for pass_pipeline in pass_pipelines.iter() {
                    let pipeline_descriptor = pipeline_storage.get(pass_pipeline).unwrap();
                    let render_pipeline = self.render_pipelines.get(pass_pipeline).unwrap();
                    render_pass.set_pipeline(render_pipeline);

                    let mut wgpu_render_pass = WgpuRenderPass {
                        render_pass: &mut render_pass,
                        pipeline_descriptor,
                        wgpu_resources: &self.wgpu_resources,
                        renderer: &self,
                    };

                    for draw_target_name in pipeline_descriptor.draw_targets.iter() {
                        let draw_target = render_graph.draw_targets.get(draw_target_name).unwrap();
                        draw_target.draw(world, resources, &mut wgpu_render_pass, *pass_pipeline);
                    }
                }
            }
        }

        let command_buffer = encoder.finish();
        self.queue.submit(&[command_buffer]);
    }

    fn create_buffer_with_data(
        &mut self,
        data: &[u8],
        buffer_usage: BufferUsage,
    ) -> RenderResource {
        self.wgpu_resources
            .create_buffer_with_data(&self.device, data, buffer_usage.into())
    }

    fn create_buffer(&mut self, size: u64, buffer_usage: BufferUsage) -> RenderResource {
        self.wgpu_resources
            .create_buffer(&self.device, size, buffer_usage.into())
    }

    fn create_instance_buffer(
        &mut self,
        mesh_id: usize,
        size: usize,
        count: usize,
        buffer_usage: BufferUsage,
    ) -> RenderResource {
        self.wgpu_resources.create_instance_buffer(
            &self.device,
            mesh_id,
            size,
            count,
            buffer_usage.into(),
        )
    }

    fn create_instance_buffer_with_data(
        &mut self,
        mesh_id: usize,
        data: &[u8],
        size: usize,
        count: usize,
        buffer_usage: BufferUsage,
    ) -> RenderResource {
        self.wgpu_resources.create_instance_buffer_with_data(
            &self.device,
            mesh_id,
            data,
            size,
            count,
            buffer_usage.into(),
        )
    }

    fn get_resource_info(&self, resource: RenderResource) -> Option<&ResourceInfo> {
        self.wgpu_resources.resource_info.get(&resource)
    }

    fn remove_buffer(&mut self, resource: RenderResource) {
        self.wgpu_resources.remove_buffer(resource);
    }

    fn create_buffer_mapped(
        &mut self,
        size: usize,
        buffer_usage: BufferUsage,
        setup_data: &mut dyn FnMut(&mut [u8]),
    ) -> RenderResource {
        self.wgpu_resources.create_buffer_mapped(
            &self.device,
            size,
            buffer_usage.into(),
            setup_data,
        )
    }

    fn copy_buffer_to_buffer(
        &mut self,
        source_buffer: RenderResource,
        source_offset: u64,
        destination_buffer: RenderResource,
        destination_offset: u64,
        size: u64,
    ) {
        self.wgpu_resources.copy_buffer_to_buffer(
            self.encoder.as_mut().unwrap(),
            source_buffer,
            source_offset,
            destination_buffer,
            destination_offset,
            size,
        );
    }

    fn get_dynamic_uniform_buffer_info(
        &self,
        resource: RenderResource,
    ) -> Option<&DynamicUniformBufferInfo> {
        self.wgpu_resources
            .get_dynamic_uniform_buffer_info(resource)
    }

    fn get_dynamic_uniform_buffer_info_mut(
        &mut self,
        resource: RenderResource,
    ) -> Option<&mut DynamicUniformBufferInfo> {
        self.wgpu_resources
            .get_dynamic_uniform_buffer_info_mut(resource)
    }

    fn add_dynamic_uniform_buffer_info(
        &mut self,
        resource: RenderResource,
        info: DynamicUniformBufferInfo,
    ) {
        self.wgpu_resources
            .add_dynamic_uniform_buffer_info(resource, info);
    }

    fn create_sampler(&mut self, sampler_descriptor: &SamplerDescriptor) -> RenderResource {
        self.wgpu_resources
            .create_sampler(&self.device, sampler_descriptor)
    }

    fn create_texture(
        &mut self,
        texture_descriptor: &TextureDescriptor,
        bytes: Option<&[u8]>,
    ) -> RenderResource {
        self.wgpu_resources.create_texture(
            &self.device,
            self.encoder.as_mut().unwrap(),
            texture_descriptor,
            bytes,
        )
    }

    fn remove_texture(&mut self, resource: RenderResource) {
        self.wgpu_resources.remove_texture(resource);
    }

    fn remove_sampler(&mut self, resource: RenderResource) {
        self.wgpu_resources.remove_sampler(resource);
    }

    fn get_render_resources(&self) -> &RenderResources {
        &self.wgpu_resources.render_resources
    }

    fn get_render_resources_mut(&mut self) -> &mut RenderResources {
        &mut self.wgpu_resources.render_resources
    }

    fn set_entity_uniform_resource(
        &mut self,
        entity: Entity,
        uniform_name: &str,
        resource: RenderResource,
    ) {
        self.wgpu_resources
            .set_entity_uniform_resource(entity, uniform_name, resource)
    }
    fn get_entity_uniform_resource(
        &self,
        entity: Entity,
        uniform_name: &str,
    ) -> Option<RenderResource> {
        self.wgpu_resources
            .get_entity_uniform_resource(entity, uniform_name)
    }

    fn setup_entity_bind_groups(
        &mut self,
        entity: Entity,
        pipeline_descriptor: &PipelineDescriptor,
    ) {
        let pipeline_layout = pipeline_descriptor.get_layout().unwrap();
        for bind_group in pipeline_layout.bind_groups.iter() {
            let bind_group_id = bind_group.get_hash().unwrap();
            // only setup entity bind groups if there isn't already a "global" bind group created
            if let None = self.wgpu_resources.bind_groups.get(&bind_group_id) {
                if let None = self
                    .wgpu_resources
                    .get_entity_bind_group(entity, bind_group_id)
                {
                    self.wgpu_resources
                        .create_entity_bind_group(&self.device, bind_group, entity);
                }
            }
        }
    }
}

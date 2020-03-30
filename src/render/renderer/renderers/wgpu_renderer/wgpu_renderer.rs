use super::{wgpu_type_converter::OwnedWgpuVertexBufferDescriptor, WgpuRenderPass, WgpuResources};
use crate::{
    asset::{AssetStorage, Handle},
    core::{Event, EventHandle, Window, WindowResize},
    legion::prelude::*,
    render::{
        pass::{
            PassDescriptor, RenderPassColorAttachmentDescriptor,
            RenderPassDepthStencilAttachmentDescriptor,
        },
        pipeline::{update_shader_assignments, PipelineCompiler, PipelineDescriptor},
        render_graph::RenderGraph,
        render_resource::{
            resource_name, BufferInfo, RenderResource, RenderResourceAssignments, RenderResources,
            ResourceInfo,
        },
        renderer::Renderer,
        shader::Shader,
        texture::{SamplerDescriptor, TextureDescriptor},
    },
};
use std::{cell::RefCell, collections::HashMap, ops::Deref, rc::Rc};

pub struct WgpuRenderer {
    pub device: Rc<RefCell<wgpu::Device>>,
    pub queue: wgpu::Queue,
    pub surface: Option<wgpu::Surface>,
    pub encoder: Option<wgpu::CommandEncoder>,
    pub render_pipelines: HashMap<Handle<PipelineDescriptor>, wgpu::RenderPipeline>,
    pub wgpu_resources: WgpuResources,
    pub window_resize_handle: EventHandle<WindowResize>,
    pub intialized: bool,
}

impl WgpuRenderer {
    pub async fn new(window_resize_event: EventHandle<WindowResize>) -> Self {
        let adapter = wgpu::Adapter::request(
            &wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::Default,
            },
            wgpu::BackendBit::PRIMARY,
        )
        .await
        .unwrap();

        let (device, queue) = adapter.request_device(&wgpu::DeviceDescriptor {
            extensions: wgpu::Extensions {
                anisotropic_filtering: false,
            },
            limits: wgpu::Limits::default(),
        }).await;

        WgpuRenderer {
            device: Rc::new(RefCell::new(device)),
            queue,
            surface: None,
            encoder: None,
            window_resize_handle: window_resize_event,
            intialized: false,
            wgpu_resources: WgpuResources::default(),
            render_pipelines: HashMap::new(),
        }
    }

    fn initialize(&mut self, world: &mut World, resources: &mut Resources) {
        if self.intialized {
            return;
        }

        self.create_surface(resources);
        self.initialize_resource_providers(world, resources);
        self.resize(world, resources);

        self.intialized = true;
    }

    pub fn create_render_pipeline(
        wgpu_resources: &mut WgpuResources,
        pipeline_descriptor: &mut PipelineDescriptor,
        shader_storage: &AssetStorage<Shader>,
        device: &wgpu::Device,
    ) -> wgpu::RenderPipeline {
        let layout = pipeline_descriptor.get_layout().unwrap();
        for bind_group in layout.bind_groups.iter() {
            if let None = wgpu_resources.bind_group_layouts.get(&bind_group.id) {
                let bind_group_layout_binding = bind_group
                    .bindings
                    .iter()
                    .map(|binding| wgpu::BindGroupLayoutEntry {
                        binding: binding.index,
                        visibility: wgpu::ShaderStage::VERTEX | wgpu::ShaderStage::FRAGMENT,
                        ty: (&binding.bind_type).into(),
                    })
                    .collect::<Vec<wgpu::BindGroupLayoutEntry>>();
                let wgpu_bind_group_layout =
                    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                        bindings: bind_group_layout_binding.as_slice(),
                    });

                wgpu_resources
                    .bind_group_layouts
                    .insert(bind_group.id, wgpu_bind_group_layout);
            }
        }

        // setup and collect bind group layouts
        let bind_group_layouts = layout
            .bind_groups
            .iter()
            .map(|bind_group| {
                wgpu_resources
                    .bind_group_layouts
                    .get(&bind_group.id)
                    .unwrap()
            })
            .collect::<Vec<&wgpu::BindGroupLayout>>();

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            bind_group_layouts: bind_group_layouts.as_slice(),
        });

        let owned_vertex_buffer_descriptors = layout
            .vertex_buffer_descriptors
            .iter()
            .map(|v| v.into())
            .collect::<Vec<OwnedWgpuVertexBufferDescriptor>>();

        let color_states = pipeline_descriptor
            .color_states
            .iter()
            .map(|c| c.into())
            .collect::<Vec<wgpu::ColorStateDescriptor>>();

        if let None = wgpu_resources
            .shader_modules
            .get(&pipeline_descriptor.shader_stages.vertex)
        {
            wgpu_resources.create_shader_module(
                device,
                pipeline_descriptor.shader_stages.vertex,
                shader_storage,
            );
        }

        if let Some(fragment_handle) = pipeline_descriptor.shader_stages.fragment {
            if let None = wgpu_resources.shader_modules.get(&fragment_handle) {
                wgpu_resources.create_shader_module(device, fragment_handle, shader_storage);
            }
        };

        let vertex_shader_module = wgpu_resources
            .shader_modules
            .get(&pipeline_descriptor.shader_stages.vertex)
            .unwrap();

        let fragment_shader_module = match pipeline_descriptor.shader_stages.fragment {
            Some(fragment_handle) => {
                Some(wgpu_resources.shader_modules.get(&fragment_handle).unwrap())
            }
            None => None,
        };

        let mut render_pipeline_descriptor = wgpu::RenderPipelineDescriptor {
            layout: &pipeline_layout,
            vertex_stage: wgpu::ProgrammableStageDescriptor {
                module: &vertex_shader_module,
                entry_point: "main",
            },
            fragment_stage: match pipeline_descriptor.shader_stages.fragment {
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
            vertex_state: wgpu::VertexStateDescriptor {
                index_format: pipeline_descriptor.index_format.into(),
                vertex_buffers: &owned_vertex_buffer_descriptors
                    .iter()
                    .map(|v| v.into())
                    .collect::<Vec<wgpu::VertexBufferDescriptor>>(),
            },
            sample_count: pipeline_descriptor.sample_count,
            sample_mask: pipeline_descriptor.sample_mask,
            alpha_to_coverage_enabled: pipeline_descriptor.alpha_to_coverage_enabled,
        };

        device.create_render_pipeline(&mut render_pipeline_descriptor)
    }

    pub fn create_render_pass<'a>(
        wgpu_resources: &'a WgpuResources,
        pass_descriptor: &PassDescriptor,
        global_render_resource_assignments: &RenderResourceAssignments,
        encoder: &'a mut wgpu::CommandEncoder,
        frame: &'a wgpu::SwapChainOutput,
    ) -> wgpu::RenderPass<'a> {
        encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            color_attachments: &pass_descriptor
                .color_attachments
                .iter()
                .map(|c| {
                    Self::create_wgpu_color_attachment_descriptor(
                        wgpu_resources,
                        global_render_resource_assignments,
                        c,
                        frame,
                    )
                })
                .collect::<Vec<wgpu::RenderPassColorAttachmentDescriptor>>(),
            depth_stencil_attachment: pass_descriptor.depth_stencil_attachment.as_ref().map(|d| {
                Self::create_wgpu_depth_stencil_attachment_descriptor(
                    wgpu_resources,
                    global_render_resource_assignments,
                    d,
                    frame,
                )
            }),
        })
    }

    fn create_wgpu_color_attachment_descriptor<'a>(
        wgpu_resources: &'a WgpuResources,
        global_render_resource_assignments: &RenderResourceAssignments,
        color_attachment_descriptor: &RenderPassColorAttachmentDescriptor,
        frame: &'a wgpu::SwapChainOutput,
    ) -> wgpu::RenderPassColorAttachmentDescriptor<'a> {
        let attachment = match color_attachment_descriptor.attachment.as_str() {
            resource_name::texture::SWAP_CHAIN => &frame.view,
            _ => {
                match global_render_resource_assignments
                    .get(&color_attachment_descriptor.attachment)
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
                _ => match global_render_resource_assignments.get(target.as_str()) {
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
        global_render_resource_assignments: &RenderResourceAssignments,
        depth_stencil_attachment_descriptor: &RenderPassDepthStencilAttachmentDescriptor,
        frame: &'a wgpu::SwapChainOutput,
    ) -> wgpu::RenderPassDepthStencilAttachmentDescriptor<'a> {
        let attachment = match depth_stencil_attachment_descriptor.attachment.as_str() {
            resource_name::texture::SWAP_CHAIN => &frame.view,
            _ => {
                match global_render_resource_assignments
                    .get(&depth_stencil_attachment_descriptor.attachment)
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

    pub fn initialize_resource_providers(&mut self, world: &mut World, resources: &mut Resources) {
        let mut render_graph = resources.get_mut::<RenderGraph>().unwrap();
        self.encoder = Some(
            self.device
                .borrow()
                .create_command_encoder(&wgpu::CommandEncoderDescriptor { todo: 0 }),
        );
        for resource_provider in render_graph.resource_providers.iter_mut() {
            resource_provider.initialize(self, world, resources);
        }

        // consume current encoder
        let command_buffer = self.encoder.take().unwrap().finish();
        self.queue.submit(&[command_buffer]);
    }

    pub fn update_resource_providers(&mut self, world: &mut World, resources: &mut Resources) {
        let mut render_graph = resources.get_mut::<RenderGraph>().unwrap();
        for resource_provider in render_graph.resource_providers.iter_mut() {
            resource_provider.update(self, world, resources);
        }

        for resource_provider in render_graph.resource_providers.iter_mut() {
            resource_provider.finish_update(self, world, resources);
        }
    }

    pub fn create_queued_textures(&mut self, resources: &mut Resources) {
        let mut render_graph = resources.get_mut::<RenderGraph>().unwrap();
        let mut render_resource_assignments =
            resources.get_mut::<RenderResourceAssignments>().unwrap();
        for (name, texture_descriptor) in render_graph.queued_textures.drain(..) {
            let resource = self.create_texture(&texture_descriptor, None);
            render_resource_assignments.set(&name, resource);
        }
    }

    pub fn create_surface(&mut self, resources: &Resources) {
        #[cfg(feature = "winit")]
        {
            let window = resources.get::<winit::window::Window>().unwrap();
            let surface = wgpu::Surface::create(window.deref());
            self.surface = Some(surface);
        }
    }
}

impl Renderer for WgpuRenderer {
    fn resize(&mut self, world: &mut World, resources: &mut Resources) {
        if let Some(surface) = self.surface.as_ref() {
            self.encoder = Some(
                self.device
                    .borrow()
                    .create_command_encoder(&wgpu::CommandEncoderDescriptor { todo: 0 }),
            );
            let swap_chain_descriptor: wgpu::SwapChainDescriptor = {
                let window: &Window = &resources.get::<Window>().unwrap();
                window.into()
            };

            let swap_chain = self
                .device
                .borrow()
                .create_swap_chain(surface, &swap_chain_descriptor);

            // WgpuRenderer can't own swap_chain without creating lifetime ergonomics issues, so lets just store it in World.
            resources.insert(swap_chain);
            let mut render_graph = resources.get_mut::<RenderGraph>().unwrap();
            for resource_provider in render_graph.resource_providers.iter_mut() {
                resource_provider.resize(
                    self,
                    world,
                    resources,
                    swap_chain_descriptor.width,
                    swap_chain_descriptor.height,
                );
            }

            // consume current encoder
            let command_buffer = self.encoder.take().unwrap().finish();
            self.queue.submit(&[command_buffer]);
        } else {
            // TODO: remove this warning if this case is not a problem
            println!("warning: attempted to resize renderer before surface was ready");
        }
    }

    fn update(&mut self, world: &mut World, resources: &mut Resources) {
        self.initialize(world, resources);

        let resized =
            resources
                .get::<Event<WindowResize>>()
                .unwrap()
                .iter(&mut self.window_resize_handle)
                .last()
                .map(|_| ())
                .is_some();

        if resized {
            self.resize(world, resources);
        }
        // TODO: this self.encoder handoff is a bit gross, but its here to give resource providers access to buffer copies without
        // exposing the wgpu renderer internals to ResourceProvider traits. if this can be made cleaner that would be pretty cool.
        self.encoder = Some(
            self.device
                .borrow()
                .create_command_encoder(&wgpu::CommandEncoderDescriptor { todo: 0 }),
        );

        self.update_resource_providers(world, resources);
        update_shader_assignments(world, resources, self);
        self.create_queued_textures(resources);

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
        let render_graph = resources.get::<RenderGraph>().unwrap();
        let mut render_graph_mut = resources.get_mut::<RenderGraph>().unwrap();
        let global_render_resource_assignments =
            resources.get::<RenderResourceAssignments>().unwrap();
        let pipeline_compiler = resources.get::<PipelineCompiler>().unwrap();

        for pipeline_descriptor_handle in render_graph.pipeline_descriptors.iter() {
            if let Some(compiled_pipelines_iter) =
                pipeline_compiler.iter_compiled_pipelines(*pipeline_descriptor_handle)
            {
                for compiled_pipeline_handle in compiled_pipelines_iter {
                    // create pipelines
                    // TODO: merge this into "setup draw targets" loop
                    if !self.render_pipelines.contains_key(compiled_pipeline_handle) {
                        let compiled_pipeline_descriptor =
                            pipeline_storage.get_mut(compiled_pipeline_handle).unwrap();

                        let render_pipeline = WgpuRenderer::create_render_pipeline(
                            &mut self.wgpu_resources,
                            compiled_pipeline_descriptor,
                            &shader_storage,
                            &self.device.borrow(),
                        );
                        self.render_pipelines
                            .insert(*compiled_pipeline_handle, render_pipeline);
                    }
                }
            }
        }

        // setup draw targets
        for (pass_name, _pass_descriptor) in render_graph.pass_descriptors.iter() {
            if let Some(pass_pipelines) = render_graph.pass_pipelines.get(pass_name) {
                for pass_pipeline in pass_pipelines.iter() {
                    if let Some(compiled_pipelines_iter) =
                        pipeline_compiler.iter_compiled_pipelines(*pass_pipeline)
                    {
                        for compiled_pipeline_handle in compiled_pipelines_iter {
                            let pipeline_descriptor =
                                pipeline_storage.get(compiled_pipeline_handle).unwrap();
                            for draw_target_name in pipeline_descriptor.draw_targets.iter() {
                                let draw_target = render_graph_mut
                                    .draw_targets
                                    .get_mut(draw_target_name)
                                    .unwrap();
                                draw_target.setup(
                                    world,
                                    resources,
                                    self,
                                    *compiled_pipeline_handle,
                                );
                            }
                        }
                    }
                }
            }
        }

        // begin render passes
        for (pass_name, pass_descriptor) in render_graph.pass_descriptors.iter() {
            let mut render_pass = Self::create_render_pass(
                &self.wgpu_resources,
                pass_descriptor,
                &global_render_resource_assignments,
                &mut encoder,
                &frame,
            );
            if let Some(pass_pipelines) = render_graph.pass_pipelines.get(pass_name) {
                for pass_pipeline in pass_pipelines.iter() {
                    if let Some(compiled_pipelines_iter) =
                        pipeline_compiler.iter_compiled_pipelines(*pass_pipeline)
                    {
                        for compiled_pipeline_handle in compiled_pipelines_iter {
                            let pipeline_descriptor =
                                pipeline_storage.get(compiled_pipeline_handle).unwrap();
                            let render_pipeline =
                                self.render_pipelines.get(compiled_pipeline_handle).unwrap();
                            render_pass.set_pipeline(render_pipeline);

                            let mut wgpu_render_pass = WgpuRenderPass {
                                render_pass: &mut render_pass,
                                pipeline_descriptor,
                                wgpu_resources: &self.wgpu_resources,
                                renderer: &self,
                                bound_bind_groups: HashMap::default(),
                            };

                            for draw_target_name in pipeline_descriptor.draw_targets.iter() {
                                let draw_target =
                                    render_graph.draw_targets.get(draw_target_name).unwrap();
                                draw_target.draw(
                                    world,
                                    resources,
                                    &mut wgpu_render_pass,
                                    *compiled_pipeline_handle,
                                );
                            }
                        }
                    }
                }
            }
        }

        let command_buffer = encoder.finish();
        self.queue.submit(&[command_buffer]);
    }

    fn create_buffer_with_data(&mut self, buffer_info: BufferInfo, data: &[u8]) -> RenderResource {
        self.wgpu_resources
            .create_buffer_with_data(&self.device.borrow(), buffer_info, data)
    }

    fn create_buffer(&mut self, buffer_info: BufferInfo) -> RenderResource {
        self.wgpu_resources
            .create_buffer(&self.device.borrow(), buffer_info)
    }

    fn get_resource_info(&self, resource: RenderResource) -> Option<&ResourceInfo> {
        self.wgpu_resources.resource_info.get(&resource)
    }

    fn get_resource_info_mut(&mut self, resource: RenderResource) -> Option<&mut ResourceInfo> {
        self.wgpu_resources.resource_info.get_mut(&resource)
    }

    fn remove_buffer(&mut self, resource: RenderResource) {
        self.wgpu_resources.remove_buffer(resource);
    }

    fn create_buffer_mapped(
        &mut self,
        buffer_info: BufferInfo,
        setup_data: &mut dyn FnMut(&mut [u8], &mut dyn Renderer),
    ) -> RenderResource {
        let buffer = WgpuResources::begin_create_buffer_mapped(&buffer_info, self, setup_data);
        self.wgpu_resources.assign_buffer(buffer, buffer_info)
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

    fn create_sampler(&mut self, sampler_descriptor: &SamplerDescriptor) -> RenderResource {
        self.wgpu_resources
            .create_sampler(&self.device.borrow(), sampler_descriptor)
    }

    fn create_texture(
        &mut self,
        texture_descriptor: &TextureDescriptor,
        bytes: Option<&[u8]>,
    ) -> RenderResource {
        self.wgpu_resources.create_texture(
            &self.device.borrow(),
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

    fn setup_bind_groups(
        &mut self,
        render_resource_assignments: &mut RenderResourceAssignments,
        pipeline_descriptor: &PipelineDescriptor,
    ) {
        let pipeline_layout = pipeline_descriptor.get_layout().unwrap();
        for bind_group in pipeline_layout.bind_groups.iter() {
            if let Some(render_resource_set_id) =
                render_resource_assignments.get_or_update_render_resource_set_id(bind_group)
            {
                if let None = self
                    .wgpu_resources
                    .get_bind_group(bind_group.id, render_resource_set_id)
                {
                    self.wgpu_resources.create_bind_group(
                        &self.device.borrow(),
                        bind_group,
                        render_resource_assignments,
                    );
                } else {
                    log::trace!(
                        "reusing RenderResourceSet {:?} for bind group {}",
                        render_resource_set_id,
                        bind_group.index
                    );
                }
            }
        }
    }
}

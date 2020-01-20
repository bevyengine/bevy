use crate::{
    asset::Mesh,
    legion::prelude::*,
    render::render_graph_2::{
        resource, PassDescriptor, PipelineDescriptor, RenderGraph, RenderPass,
        RenderPassColorAttachmentDescriptor, RenderPassDepthStencilAttachmentDescriptor, Renderer,
    },
};
use std::{collections::HashMap, ops::Deref};
use zerocopy::AsBytes;

pub struct WgpuRenderer {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub surface: Option<wgpu::Surface>,
    pub swap_chain_descriptor: wgpu::SwapChainDescriptor,
    pub render_pipelines: HashMap<String, wgpu::RenderPipeline>,
    pub buffers: HashMap<String, wgpu::Buffer>,
    pub textures: HashMap<String, wgpu::TextureView>,
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
        }
    }

    pub fn create_render_pipeline(
        pipeline_descriptor: &PipelineDescriptor,
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

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            bind_group_layouts: &[],
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
        // TODO: fill this in
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
            resource::texture::SWAP_CHAIN => &frame.view,
            _ => self
                .textures
                .get(&color_attachment_descriptor.attachment)
                .unwrap(),
        };

        let resolve_target = match color_attachment_descriptor.resolve_target {
            Some(ref target) => match target.as_str() {
                resource::texture::SWAP_CHAIN => Some(&frame.view),
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
            resource::texture::SWAP_CHAIN => &frame.view,
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
}

impl Renderer for WgpuRenderer {
    fn initialize(&mut self, world: &mut World) {
        let (surface, window_size) = {
            let window = world.resources.get::<winit::window::Window>().unwrap();
            let surface = wgpu::Surface::create(window.deref());
            let window_size = window.inner_size();
            (surface, window_size)
        };

        self.surface = Some(surface);
        self.resize(world, window_size.width, window_size.height);
    }

    fn resize(&mut self, world: &mut World, width: u32, height: u32) {
        let swap_chain = self
            .device
            .create_swap_chain(self.surface.as_ref().unwrap(), &self.swap_chain_descriptor);
        self.swap_chain_descriptor.width = width;
        self.swap_chain_descriptor.height = height;

        // WgpuRenderer can't own swap_chain without creating lifetime ergonomics issues
        world.resources.insert(swap_chain);
    }

    fn process_render_graph(&mut self, render_graph: &RenderGraph, world: &mut World) {
        let mut swap_chain = world.resources.get_mut::<wgpu::SwapChain>().unwrap();
        let frame = swap_chain
            .get_next_texture()
            .expect("Timeout when acquiring next swap chain texture");

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { todo: 0 });

        for (pass_name, pass_descriptor) in render_graph.pass_descriptors.iter() {
            let mut render_pass = self.create_render_pass(pass_descriptor, &mut encoder, &frame);
            if let Some(pass_pipelines) = render_graph.pass_pipelines.get(pass_name) {
                for pass_pipeline in pass_pipelines.iter() {
                    if let Some(pipeline_descriptor) =
                        render_graph.pipeline_descriptors.get(pass_pipeline)
                    {
                        if let None = self.render_pipelines.get(pass_pipeline) {
                            let render_pipeline = WgpuRenderer::create_render_pipeline(
                                pipeline_descriptor,
                                &self.device,
                            );
                            self.render_pipelines
                                .insert(pass_pipeline.to_string(), render_pipeline);
                        }

                        let mut render_pass = WgpuRenderPass {
                            render_pass: &mut render_pass,
                            renderer: &self,
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

    fn load_mesh(&mut self, asset_id: usize, mesh: &Mesh) {
        if let None = mesh.vertex_buffer {
            self.buffers.insert(
                format!("meshv{}", asset_id),
                self.device
                    .create_buffer_with_data(mesh.vertices.as_bytes(), wgpu::BufferUsage::VERTEX),
            );
        }

        if let None = mesh.index_buffer {
            self.buffers.insert(
                format!("meshi{}", asset_id),
                self.device
                    .create_buffer_with_data(mesh.indices.as_bytes(), wgpu::BufferUsage::INDEX),
            );
        }
    }
}

pub struct WgpuRenderPass<'a, 'b, 'c> {
    pub render_pass: &'b mut wgpu::RenderPass<'a>,
    pub renderer: &'c WgpuRenderer,
}

impl<'a, 'b, 'c> RenderPass for WgpuRenderPass<'a, 'b, 'c> {
    fn set_index_buffer(&mut self, buffer: &wgpu::Buffer, offset: wgpu::BufferAddress) {
        self.render_pass.set_index_buffer(buffer, offset);
    }
}

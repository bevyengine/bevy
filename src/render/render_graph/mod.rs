mod pass;
mod pipeline;
mod render_resource_manager;

pub use pass::Pass;
pub use pipeline::Pipeline;
pub use render_resource_manager::RenderResourceManager;

use crate::render::UniformBuffer;
use legion::world::World;
use std::{collections::HashMap, ops::Deref};

pub struct RenderGraph {
    pub data: Option<RenderGraphData>,
    passes: HashMap<String, Box<dyn Pass>>,
    pipelines: HashMap<String, Box<dyn Pipeline>>,
    pass_pipelines: HashMap<String, Vec<String>>,
    render_resource_managers: Vec<Box<dyn RenderResourceManager>>,
    swap_chain: Option<wgpu::SwapChain>,
}

pub struct RenderGraphData {
    pub swap_chain_descriptor: wgpu::SwapChainDescriptor,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub surface: wgpu::Surface,
    textures: HashMap<String, wgpu::TextureView>,
    samplers: HashMap<String, wgpu::Sampler>,
    uniform_buffers: HashMap<String, UniformBuffer>,
    bind_group_layouts: HashMap<String, wgpu::BindGroupLayout>,
}
impl RenderGraphData {
    pub fn new(
        device: wgpu::Device,
        swap_chain_descriptor: wgpu::SwapChainDescriptor,
        queue: wgpu::Queue,
        surface: wgpu::Surface,
    ) -> Self {
        RenderGraphData {
            textures: HashMap::new(),
            samplers: HashMap::new(),
            uniform_buffers: HashMap::new(),
            bind_group_layouts: HashMap::new(),
            device,
            swap_chain_descriptor,
            queue,
            surface,
        }
    }

    pub fn set_uniform_buffer(&mut self, name: &str, uniform_buffer: UniformBuffer) {
        self.uniform_buffers
            .insert(name.to_string(), uniform_buffer);
    }

    pub fn get_uniform_buffer(&self, name: &str) -> Option<&UniformBuffer> {
        self.uniform_buffers.get(name)
    }

    pub fn set_bind_group_layout(&mut self, name: &str, bind_group_layout: wgpu::BindGroupLayout) {
        self.bind_group_layouts
            .insert(name.to_string(), bind_group_layout);
    }

    pub fn get_bind_group_layout(&self, name: &str) -> Option<&wgpu::BindGroupLayout> {
        self.bind_group_layouts.get(name)
    }

    pub fn set_texture(&mut self, name: &str, texture: wgpu::TextureView) {
        self.textures.insert(name.to_string(), texture);
    }

    pub fn get_texture(&self, name: &str) -> Option<&wgpu::TextureView> {
        self.textures.get(name)
    }

    pub fn set_sampler(&mut self, name: &str, sampler: wgpu::Sampler) {
        self.samplers.insert(name.to_string(), sampler);
    }

    pub fn get_sampler(&self, name: &str) -> Option<&wgpu::Sampler> {
        self.samplers.get(name)
    }
}

impl RenderGraph {
    pub fn new() -> Self {
        RenderGraph {
            passes: HashMap::new(),
            pipelines: HashMap::new(),
            pass_pipelines: HashMap::new(),
            render_resource_managers: Vec::new(),
            data: None,
            swap_chain: None,
        }
    }

    pub fn initialize(&mut self, world: &mut World) {
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

        let (surface, window_size) = {
            let window = world.resources.get::<winit::window::Window>().unwrap();
            let surface = wgpu::Surface::create(window.deref());
            let window_size = window.inner_size();
            (surface, window_size)
        };

        let swap_chain_descriptor = wgpu::SwapChainDescriptor {
            usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT,
            format: wgpu::TextureFormat::Bgra8UnormSrgb,
            width: window_size.width,
            height: window_size.height,
            present_mode: wgpu::PresentMode::Vsync,
        };

        self.swap_chain = Some(device.create_swap_chain(&surface, &swap_chain_descriptor));

        self.data = Some(RenderGraphData::new(
            device,
            swap_chain_descriptor,
            queue,
            surface,
        ));

        let data = self.data.as_mut().unwrap();
        for render_resource_manager in self.render_resource_managers.iter_mut() {
            render_resource_manager.initialize(data, world);
        }

        for pass in self.passes.values_mut() {
            pass.initialize(data);
        }

        for pipeline in self.pipelines.values_mut() {
            pipeline.initialize(data, world);
        }

        self.resize(window_size.width, window_size.height, world);
    }

    pub fn render(&mut self, world: &mut World) {
        let frame = self.swap_chain
            .as_mut()
            .unwrap()
            .get_next_texture()
            .expect("Timeout when acquiring next swap chain texture");

        let data = self.data.as_mut().unwrap();
        let mut encoder = data
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { todo: 0 });

        for render_resource_manager in self.render_resource_managers.iter_mut() {
            render_resource_manager.update(data, &mut encoder, world);
        }

        for (pass_name, pass) in self.passes.iter_mut() {
            loop {
                let render_pass = pass.begin(data, world, &mut encoder, &frame);
                if let Some(mut render_pass) = render_pass {
                    if let Some(pipeline_names) = self.pass_pipelines.get(pass_name) {
                        for pipeline_name in pipeline_names.iter() {
                            let pipeline = self.pipelines.get_mut(pipeline_name).unwrap();
                            render_pass.set_pipeline(pipeline.get_pipeline());
                            pipeline.render(data, &mut render_pass, &frame, world);
                        }
                    }

                    if !pass.should_repeat() {
                        break;
                    }
                }
            }
        }

        let command_buffer = encoder.finish();
        data.queue.submit(&[command_buffer]);
    }

    pub fn resize(&mut self, width: u32, height: u32, world: &mut World) {
        let data = self.data.as_mut().unwrap();
        data.swap_chain_descriptor.width = width;
        data.swap_chain_descriptor.height = height;
        self.swap_chain = Some(
            data.device
                .create_swap_chain(&data.surface, &data.swap_chain_descriptor),
        );
        let mut encoder = data
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { todo: 0 });

        for render_resource_manager in self.render_resource_managers.iter_mut() {
            render_resource_manager.resize(data, &mut encoder, world);
        }

        let command_buffer = encoder.finish();

        for pass in self.passes.values_mut() {
            pass.resize(data);
        }

        for pipeline in self.pipelines.values_mut() {
            pipeline.resize(data);
        }

        data.queue.submit(&[command_buffer]);
    }

    pub fn add_render_resource_manager(
        &mut self,
        render_resource_manager: Box<dyn RenderResourceManager>,
    ) {
        self.render_resource_managers.push(render_resource_manager);
    }

    pub fn set_pipeline(
        &mut self,
        pass_name: &str,
        pipeline_name: &str,
        pipeline: Box<dyn Pipeline>,
    ) {
        self.pipelines.insert(pipeline_name.to_string(), pipeline);
        if let None = self.pass_pipelines.get_mut(pass_name) {
            let pipelines = Vec::new();
            self.pass_pipelines.insert(pass_name.to_string(), pipelines);
        };

        let current_pass_pipelines = self.pass_pipelines.get_mut(pass_name).unwrap();

        current_pass_pipelines.push(pipeline_name.to_string());
    }

    pub fn set_pass(&mut self, name: &str, pass: Box<dyn Pass>) {
        self.passes.insert(name.to_string(), pass);
    }
}
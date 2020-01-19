use crate::{
    asset::{AssetStorage, Handle, Mesh},
    legion::prelude::*,
    render::{
        render_graph_2::{PipelineDescriptor, PassDescriptor, RenderGraph, ShaderMaterials},
        Instanced,
    },
};
use std::collections::HashMap;
use zerocopy::AsBytes;

// A set of draw calls. ex: get + draw meshes, get + draw instanced meshes, draw ui meshes, etc
// Mesh target
// trait DrawTarget {
//     fn draw(device: &wgpu::Device);
// }
pub type DrawTarget =
    fn(world: &World, render_pass: &mut dyn RenderPass);

pub fn mesh_draw_target(
    world: &World,
    render_pass: &mut dyn RenderPass,
) {
    let mut mesh_storage = world.resources.get_mut::<AssetStorage<Mesh>>().unwrap();
    let mut last_mesh_id = None;
    let mesh_query =
        <(Read<ShaderMaterials>, Read<Handle<Mesh>>)>::query().filter(!component::<Instanced>());
    for (material, mesh) in mesh_query.iter(world) {
        let current_mesh_id = mesh.id;

        let mut should_load_mesh = last_mesh_id == None;
        if let Some(last) = last_mesh_id {
            should_load_mesh = last != current_mesh_id;
        }

        if should_load_mesh {
            if let Some(mesh_asset) = mesh_storage.get(mesh.id) {
                // render_pass.load_mesh(mesh.id, mesh_asset);
                // render_pass.set_index_buffer(mesh_asset.index_buffer.as_ref().unwrap(), 0);
                // render_pass.set_vertex_buffers(0, &[(&mesh_asset.vertex_buffer.as_ref().unwrap(), 0)]);
            };
        }

        if let Some(ref mesh_asset) = mesh_storage.get(mesh.id) {
            // pass.set_bind_group(1, material.bind_group.as_ref().unwrap(), &[]);
            // pass.draw_indexed(0..mesh_asset.indices.len() as u32, 0, 0..1);
        };

        last_mesh_id = Some(current_mesh_id);
    }
}

pub trait Renderer {
    fn resize(&mut self, world: &mut World, width: u32, height: u32);
    fn process_render_graph(&mut self, render_graph: &RenderGraph, world: &mut World);
    fn load_mesh(&mut self, asset_id: usize, mesh: &Mesh);
}

pub struct WgpuRenderer {
    pub device: wgpu::Device,
    pub surface: wgpu::Surface,
    pub swap_chain_descriptor: wgpu::SwapChainDescriptor,
    pub render_pipelines: HashMap<String, wgpu::RenderPipeline>,
    pub buffers: HashMap<String, wgpu::Buffer>,
}

impl WgpuRenderer {
    pub fn new() -> Self {
        WgpuRenderer {

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
        pass_descriptor: &PassDescriptor,
        encoder: &'a mut wgpu::CommandEncoder,
        frame: &'a wgpu::SwapChainOutput,
    ) -> wgpu::RenderPass<'a> {
        // TODO: fill this in
        encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            color_attachments: &[],
            depth_stencil_attachment: None,
        })
    }
}

impl Renderer for WgpuRenderer {
    fn resize(&mut self, world: &mut World, width: u32, height: u32) {
        let swap_chain = self
            .device
            .create_swap_chain(&self.surface, &self.swap_chain_descriptor);
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
            let mut render_pass =
                WgpuRenderer::create_render_pass(pass_descriptor, &mut encoder, &frame);
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

pub trait RenderPass {
    fn set_index_buffer(&mut self, buffer: &wgpu::Buffer, offset: wgpu::BufferAddress);
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

// pub trait RenderResources {
//     fn get_buffer(name: &str) -> Option<Buffer>;
//     fn get_texture(name: &str) -> Option<Texture>;
//     fn get_sampler(name: &str) -> Option<Sampler>;
// }
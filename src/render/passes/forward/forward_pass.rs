use crate::render::*;
use legion::prelude::*;
use wgpu::{Device, SwapChainDescriptor};
use zerocopy::{AsBytes, FromBytes};

#[repr(C)]
#[derive(Clone, Copy, AsBytes, FromBytes)]
pub struct ForwardUniforms {
    pub proj: [[f32; 4]; 4],
    pub num_lights: [u32; 4],
}

pub struct ForwardPass {
    pub depth_format: wgpu::TextureFormat,
}

impl ForwardPass {
    pub fn new(depth_format: wgpu::TextureFormat) -> Self {
        ForwardPass { depth_format }
    }
    fn get_depth_texture(
        &self,
        device: &Device,
        swap_chain_descriptor: &SwapChainDescriptor,
    ) -> wgpu::TextureView {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            size: wgpu::Extent3d {
                width: swap_chain_descriptor.width,
                height: swap_chain_descriptor.height,
                depth: 1,
            },
            array_layer_count: 1,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: self.depth_format,
            usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT,
        });

        texture.create_default_view()
    }
}

const DEPTH_TEXTURE_NAME: &str = "forward_depth";

impl Pass for ForwardPass {
    fn initialize(&self, render_graph: &mut RenderGraphData) {
        let depth_texture =
            self.get_depth_texture(&render_graph.device, &render_graph.swap_chain_descriptor);
        render_graph.set_texture(DEPTH_TEXTURE_NAME, depth_texture);
    }
    fn begin<'a>(
        &mut self,
        render_graph: &mut RenderGraphData,
        _: &mut World,
        encoder: &'a mut wgpu::CommandEncoder,
        frame: &'a wgpu::SwapChainOutput,
    ) -> Option<wgpu::RenderPass<'a>> {
        let depth_texture = render_graph.get_texture(DEPTH_TEXTURE_NAME);
        Some(encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            color_attachments: &[wgpu::RenderPassColorAttachmentDescriptor {
                attachment: &frame.view,
                resolve_target: None,
                load_op: wgpu::LoadOp::Clear,
                store_op: wgpu::StoreOp::Store,
                clear_color: wgpu::Color {
                    r: 0.3,
                    g: 0.4,
                    b: 0.5,
                    a: 1.0,
                },
            }],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachmentDescriptor {
                attachment: depth_texture.unwrap(),
                depth_load_op: wgpu::LoadOp::Clear,
                depth_store_op: wgpu::StoreOp::Store,
                stencil_load_op: wgpu::LoadOp::Clear,
                stencil_store_op: wgpu::StoreOp::Store,
                clear_depth: 1.0,
                clear_stencil: 0,
            }),
        }))
    }

    fn resize(&self, render_graph: &mut RenderGraphData) {
        let depth_texture =
            self.get_depth_texture(&render_graph.device, &render_graph.swap_chain_descriptor);
        render_graph.set_texture(DEPTH_TEXTURE_NAME, depth_texture);
    }

    fn should_repeat(&self) -> bool {
        false
    }
}

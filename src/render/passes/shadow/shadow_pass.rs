use crate::{render::passes::shadow, render::*, LocalToWorld, Translation};
use legion::prelude::*;
use std::mem;

pub struct ShadowPass {
    pub shadow_size: wgpu::Extent3d,
    light_index: isize,
    shadow_texture: Option<wgpu::Texture>,
    shadow_format: wgpu::TextureFormat,
    pub max_lights: usize,
}

pub const SHADOW_TEXTURE_NAME: &str = "shadow_texture";

impl ShadowPass {
    pub fn new(
        shadow_size: wgpu::Extent3d,
        shadow_format: wgpu::TextureFormat,
        max_lights: usize,
    ) -> Self {
        ShadowPass {
            light_index: -1,
            shadow_texture: None,
            shadow_size,
            shadow_format,
            max_lights,
        }
    }
}

impl Pass for ShadowPass {
    fn initialize(&self, render_graph: &mut RenderGraphData) {
        let shadow_texture = render_graph
            .device
            .create_texture(&wgpu::TextureDescriptor {
                size: self.shadow_size,
                array_layer_count: self.max_lights as u32,
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: self.shadow_format,
                usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT | wgpu::TextureUsage::SAMPLED,
            });

        let shadow_view = shadow_texture.create_default_view();
        render_graph.set_texture(SHADOW_TEXTURE_NAME, shadow_view);
    }

    fn begin<'a>(
        &mut self,
        render_graph: &mut RenderGraphData,
        world: &mut World,
        encoder: &'a mut wgpu::CommandEncoder,
        _frame: &'a wgpu::SwapChainOutput,
    ) -> Option<wgpu::RenderPass<'a>> {
        if self.light_index == -1 {
            self.light_index = 0;
        }

        let light_query = <(Write<Light>, Read<LocalToWorld>, Read<Translation>)>::query();
        let light_count = light_query.iter_mut(world).count();
        for (i, (mut light, _, _)) in light_query.iter_mut(world).enumerate() {
            if i != self.light_index as usize {
                continue;
            }

            if let None = light.target_view {
                light.target_view = Some(self.shadow_texture.as_ref().unwrap().create_view(
                    &wgpu::TextureViewDescriptor {
                        format: self.shadow_format,
                        dimension: wgpu::TextureViewDimension::D2,
                        aspect: wgpu::TextureAspect::All,
                        base_mip_level: 0,
                        level_count: 1,
                        base_array_layer: i as u32,
                        array_layer_count: 1,
                    },
                ));
            }

            // The light uniform buffer already has the projection,
            // let's just copy it over to the shadow uniform buffer.
            let light_uniform_buffer = render_graph
                .get_uniform_buffer(render_resources::LIGHT_UNIFORM_BUFFER_NAME)
                .unwrap();
            let shadow_pipeline_uniform_buffer = render_graph
                .get_uniform_buffer(shadow::SHADOW_PIPELINE_UNIFORMS)
                .unwrap();
            encoder.copy_buffer_to_buffer(
                &light_uniform_buffer.buffer,
                (i * mem::size_of::<LightRaw>()) as wgpu::BufferAddress,
                &shadow_pipeline_uniform_buffer.buffer,
                0,
                64,
            );

            self.light_index += 1;
            if self.light_index as usize == light_count {
                self.light_index = -1;
            }
            return Some(encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                color_attachments: &[],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachmentDescriptor {
                    attachment: light.target_view.as_ref().unwrap(),
                    depth_load_op: wgpu::LoadOp::Clear,
                    depth_store_op: wgpu::StoreOp::Store,
                    stencil_load_op: wgpu::LoadOp::Clear,
                    stencil_store_op: wgpu::StoreOp::Store,
                    clear_depth: 1.0,
                    clear_stencil: 0,
                }),
            }));
        }

        None
    }

    fn resize(&self, _render_graph: &mut RenderGraphData) {}

    fn should_repeat(&self) -> bool {
        return self.light_index != -1;
    }
}

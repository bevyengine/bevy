use super::{
    AnyFrameGraphResource, FrameGraphBuffer, FrameGraphError, FrameGraphTexture, GraphResource, ResourceNode, ResourceRead, ResourceRef, TypeHandle
};
use crate::renderer::RenderDevice;
use bevy_platform::collections::HashMap;
use std::{borrow::Cow,  ops::Range};

pub struct TextureViewInfo {
    pub label: Option<Cow<'static, str>>,
    pub format: Option<wgpu::TextureFormat>,
    pub dimension: Option<wgpu::TextureViewDimension>,
    pub usage: Option<wgpu::TextureUsages>,
    pub aspect: wgpu::TextureAspect,
    pub base_mip_level: u32,
    pub mip_level_count: Option<u32>,
    pub base_array_layer: u32,
    pub array_layer_count: Option<u32>,
}

impl TextureViewInfo {
    pub fn get_texture_view_desc(&self) -> wgpu::TextureViewDescriptor {
        wgpu::TextureViewDescriptor {
            label: self.label.as_deref(),
            format: self.format,
            dimension: self.dimension,
            usage: self.usage,
            aspect: self.aspect,
            base_mip_level: self.base_mip_level,
            mip_level_count: self.mip_level_count,
            base_array_layer: self.base_array_layer,
            array_layer_count: self.array_layer_count,
        }
    }
}

pub struct TextureViewRef {
    pub texture_ref: ResourceRef<FrameGraphTexture, ResourceRead>,
    pub desc: TextureViewInfo,
}

impl ExtraResource for TextureViewRef {
    type Resource = wgpu::TextureView;
    fn extra_resource(
        &self,
        resource_table: &ResourceTable,
    ) -> Result<Self::Resource, FrameGraphError> {
        resource_table
            .get_resource::<FrameGraphTexture>(&self.texture_ref)
            .map(|texture| {
                texture
                    .resource
                    .create_view(&self.desc.get_texture_view_desc())
            })
            .ok_or(FrameGraphError::ResourceNotFound)
    }
}

pub struct ResourceTable {
    resources: HashMap<TypeHandle<ResourceNode>, AnyFrameGraphResource>,
}

pub trait ExtraResource {
    type Resource;
    fn extra_resource(
        &self,
        resource_table: &ResourceTable,
    ) -> Result<Self::Resource, FrameGraphError>;
}

impl ResourceTable {
    pub fn get_resource<ResourceType: GraphResource>(
        &self,
        resource_ref: &ResourceRef<ResourceType, ResourceRead>,
    ) -> Option<&ResourceType> {
        self.resources
            .get(&resource_ref.handle)
            .map(|res| GraphResource::borrow_resource(res))
    }
}

pub struct RenderContext {
    render_device: RenderDevice,
    resource_table: ResourceTable,
    command_buffer_queue: Vec<wgpu::CommandBuffer>,
}

impl RenderContext {
    pub fn begin_render_pass<'a>(
        &'a mut self,
        render_pass_info: &RenderPassInfo,
    ) -> Result<TrackedRenderPass<'a>, FrameGraphError> {
        let mut command_encoder = self
            .render_device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());

        let render_pass =
            render_pass_info.create_render_pass(&self.resource_table, &mut command_encoder)?;

        Ok(TrackedRenderPass {
            command_encoder,
            render_pass,
            render_context: self,
        })
    }

    pub fn get_resource<ResourceType: GraphResource>(
        &self,
        resource_ref: &ResourceRef<ResourceType, ResourceRead>,
    ) -> Result<&ResourceType, FrameGraphError> {
        self.resource_table
            .get_resource(resource_ref)
            .ok_or(FrameGraphError::ResourceNotFound)
    }

    pub fn add_command_buffer(&mut self, command_buffer: wgpu::CommandBuffer) {
        self.command_buffer_queue.push(command_buffer);
    }
}

pub struct TrackedRenderPass<'a> {
    command_encoder: wgpu::CommandEncoder,
    render_pass: wgpu::RenderPass<'static>,
    render_context: &'a mut RenderContext,
}

impl<'a> TrackedRenderPass<'a> {
    pub fn draw_indexed(&mut self, indices: Range<u32>, base_vertex: i32, instances: Range<u32>) {
        self.render_pass
            .draw_indexed(indices, base_vertex, instances);
    }

    pub fn draw(&mut self, vertices: Range<u32>, instances: Range<u32>) {
        self.render_pass.draw(vertices, instances);
    }

    pub fn set_vertex_buffer(
        &mut self,
        slot: u32,
        buffer_ref: &ResourceRef<FrameGraphBuffer, ResourceRead>,
    ) -> Result<(), FrameGraphError> {
        let buffer = self.render_context.get_resource(&buffer_ref)?;
        self.render_pass
            .set_vertex_buffer(slot, buffer.resource.slice(0..));

        Ok(())
    }

    pub fn set_index_buffer(
        &mut self,
        buffer_ref: &ResourceRef<FrameGraphBuffer, ResourceRead>,
        index_format: wgpu::IndexFormat,
    ) -> Result<(), FrameGraphError> {
        let buffer = self.render_context.get_resource(&buffer_ref)?;

        self.render_pass
            .set_index_buffer(buffer.resource.slice(0..), index_format);

        Ok(())
    }

    pub fn end(self) {
        drop(self.render_pass);
        let command_buffer = self.command_encoder.finish();
        self.render_context.add_command_buffer(command_buffer);
    }
}

pub struct DepthStencilAttachmentRef {
    pub view_ref: TextureViewRef,
    pub depth_ops: Option<wgpu::Operations<f32>>,
    pub stencil_ops: Option<wgpu::Operations<u32>>,
}

impl ExtraResource for DepthStencilAttachmentRef {
    type Resource = DepthStencilAttachment;

    fn extra_resource(
        &self,
        resource_table: &ResourceTable,
    ) -> Result<Self::Resource, FrameGraphError> {
        let view = self.view_ref.extra_resource(resource_table)?;

        Ok(DepthStencilAttachment {
            view,
            depth_ops: self.depth_ops.clone(),
            stencil_ops: self.stencil_ops.clone(),
        })
    }
}

pub struct DepthStencilAttachment {
    pub view: wgpu::TextureView,
    pub depth_ops: Option<wgpu::Operations<f32>>,
    pub stencil_ops: Option<wgpu::Operations<u32>>,
}

impl DepthStencilAttachment {
    pub fn get_render_pass_depth_stencil_attachment(
        &self,
    ) -> wgpu::RenderPassDepthStencilAttachment {
        wgpu::RenderPassDepthStencilAttachment {
            view: &self.view,
            depth_ops: self.depth_ops.clone(),
            stencil_ops: self.stencil_ops.clone(),
        }
    }
}

pub struct ColorAttachmentRef {
    pub view_ref: TextureViewRef,
    pub resolve_target: Option<TextureViewRef>,
    pub ops: wgpu::Operations<wgpu::Color>,
}

pub struct ColorAttachment {
    pub view: wgpu::TextureView,
    pub resolve_target: Option<wgpu::TextureView>,
    pub ops: wgpu::Operations<wgpu::Color>,
}

impl ColorAttachment {
    pub fn get_render_pass_color_attachment(&self) -> wgpu::RenderPassColorAttachment {
        wgpu::RenderPassColorAttachment {
            view: &self.view,
            resolve_target: self.resolve_target.as_ref(),
            ops: self.ops.clone(),
        }
    }
}

impl ExtraResource for ColorAttachmentRef {
    type Resource = ColorAttachment;

    fn extra_resource(
        &self,
        resource_table: &ResourceTable,
    ) -> Result<Self::Resource, FrameGraphError> {
        let view = self.view_ref.extra_resource(resource_table)?;

        if let Some(resolve_target) = &self.resolve_target {
            let resolve_target = resolve_target.extra_resource(resource_table)?;

            Ok(ColorAttachment {
                view,
                resolve_target: Some(resolve_target),
                ops: self.ops.clone(),
            })
        } else {
            Ok(ColorAttachment {
                view,
                resolve_target: None,
                ops: self.ops.clone(),
            })
        }
    }
}

pub struct RenderPassInfo {
    color_attachments: Vec<ColorAttachmentRef>,
    depth_stencil_attachment: Option<DepthStencilAttachmentRef>,
}

impl RenderPassInfo {
    pub fn create_render_pass(
        &self,
        resource_table: &ResourceTable,
        command_encoder: &mut wgpu::CommandEncoder,
    ) -> Result<wgpu::RenderPass<'static>, FrameGraphError> {
        let mut color_attachments = vec![];

        for color_attachment in self.color_attachments.iter() {
            color_attachments.push(color_attachment.extra_resource(resource_table)?);
        }

        let mut depth_stencil_attachment = None;

        if let Some(depth_stencil_attachment_ref) = &self.depth_stencil_attachment {
            depth_stencil_attachment =
                Some(depth_stencil_attachment_ref.extra_resource(resource_table)?);
        }

        let depth_stencil_attachment =
            depth_stencil_attachment
                .as_ref()
                .map(|depth_stencil_attachment| {
                    depth_stencil_attachment.get_render_pass_depth_stencil_attachment()
                });

        let render_pass = command_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: None,
            color_attachments: &color_attachments
                .iter()
                .map(|color_attachment| Some(color_attachment.get_render_pass_color_attachment()))
                .collect::<Vec<_>>(),
            depth_stencil_attachment,
            ..Default::default()
        });

        let render_pass = render_pass.forget_lifetime();

        Ok(render_pass)
    }
}

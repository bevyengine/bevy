use std::{borrow::Cow, mem::take, ops::Range};

use tracing::warn;
use wgpu::StoreOp;

use crate::{
    camera::Viewport,
    frame_graph::{
        BindGroupBluePrint, BindGroupEntryRef, BindingResourceRef, BluePrintProvider,
        ColorAttachment, ColorAttachmentBluePrint, DepthStencilAttachmentBluePrint,
        FrameGraphBuffer, FrameGraphError, FrameGraphTexture, GraphResource, PassNodeBuilder,
        RenderPassContextExecutor, ResourceBoardKey, ResourceRead, ResourceRef, SamplerInfo,
        TextureViewBluePrint, TextureViewInfo,
    },
    render_resource::{BindGroup, BindGroupLayout, Buffer, CachedRenderPipelineId, Texture},
    view::ViewDepthTexture,
};

use super::RenderPass;

pub struct RenderPassBuilder<'a> {
    render_pass: RenderPass,
    pass_node_builder: PassNodeBuilder<'a>,
}

impl<'a> Drop for RenderPassBuilder<'a> {
    fn drop(&mut self) {
        let render_pass = take(&mut self.render_pass);

        if render_pass.is_vaild() {
            self.pass_node_builder.set_pass(render_pass);
        } else {
            warn!("render pass must is vaild");
        }
    }
}

pub enum BindingResourceHandle {
    Buffer(ResourceRef<FrameGraphBuffer, ResourceRead>),
    Sampler(SamplerInfo),
    TextureView {
        texture_ref: ResourceRef<FrameGraphTexture, ResourceRead>,
        texture_view_info: TextureViewInfo,
    },
}

impl From<SamplerInfo> for BindingResourceHandle {
    fn from(value: SamplerInfo) -> Self {
        BindingResourceHandle::Sampler(value)
    }
}

impl From<ResourceRef<FrameGraphBuffer, ResourceRead>> for BindingResourceHandle {
    fn from(value: ResourceRef<FrameGraphBuffer, ResourceRead>) -> Self {
        BindingResourceHandle::Buffer(value)
    }
}

impl From<ResourceRef<FrameGraphTexture, ResourceRead>> for BindingResourceHandle {
    fn from(value: ResourceRef<FrameGraphTexture, ResourceRead>) -> Self {
        BindingResourceHandle::TextureView {
            texture_ref: value,
            texture_view_info: TextureViewInfo::default(),
        }
    }
}

impl BluePrintProvider for (&ViewDepthTexture, StoreOp) {
    type BluePrint = DepthStencilAttachmentBluePrint;

    fn make_blue_print(
        &self,
        pass_node_builder: &mut PassNodeBuilder,
    ) -> Result<Self::BluePrint, FrameGraphError> {
        let depth_texture_read =
            pass_node_builder.read_from_board(self.0.get_depth_texture_key())?;

        Ok(DepthStencilAttachmentBluePrint {
            view: TextureViewBluePrint {
                texture: depth_texture_read,
                desc: TextureViewInfo::default(),
            },
            depth_ops: self.0.get_depth_ops(StoreOp::Store),
            stencil_ops: None,
        })
    }
}

impl BluePrintProvider
    for (
        Option<Cow<'static, str>>,
        BindGroupLayout,
        Vec<BindingResourceHandle>,
    )
{
    type BluePrint = BindGroupBluePrint;

    fn make_blue_print(
        &self,
        _pass_node_builder: &mut PassNodeBuilder,
    ) -> Result<Self::BluePrint, FrameGraphError> {
        let mut entries = vec![];

        for (index, handle) in self.2.iter().enumerate() {
            match handle {
                BindingResourceHandle::Sampler(info) => {
                    entries.push(BindGroupEntryRef {
                        binding: index as u32,
                        resource: BindingResourceRef::Sampler(info.clone()),
                    });
                }

                BindingResourceHandle::Buffer(buffer_read) => {
                    entries.push(BindGroupEntryRef {
                        binding: index as u32,
                        resource: BindingResourceRef::Buffer(buffer_read.clone()),
                    });
                }

                BindingResourceHandle::TextureView {
                    texture_ref,
                    texture_view_info,
                } => {
                    entries.push(BindGroupEntryRef {
                        binding: index as u32,
                        resource: BindingResourceRef::TextureView {
                            texture_ref: texture_ref.clone(),
                            texture_view_info: texture_view_info.clone(),
                        },
                    });
                }
            }
        }

        Ok(BindGroupBluePrint {
            label: self.0.clone(),
            layout: self.1.clone(),
            entries,
        })
    }
}

impl<'a> RenderPassBuilder<'a> {
    pub fn new(pass_node_builder: PassNodeBuilder<'a>) -> Self {
        let render_pass = RenderPass::default();

        Self {
            render_pass,
            pass_node_builder,
        }
    }

    pub fn draw_indexed(
        &mut self,
        indices: Range<u32>,
        base_vertex: i32,
        instances: Range<u32>,
    ) -> &mut Self {
        self.render_pass
            .draw_indexed(indices, base_vertex, instances);

        self
    }

    pub fn draw(&mut self, vertices: Range<u32>, instances: Range<u32>) -> &mut Self {
        self.render_pass.draw(vertices, instances);
        self
    }

    pub fn set_index_buffer(
        &mut self,
        buffer_ref: &ResourceRef<FrameGraphBuffer, ResourceRead>,
        index_format: wgpu::IndexFormat,
    ) -> &mut Self {
        self.render_pass.set_index_buffer(buffer_ref, index_format);

        self
    }

    pub fn set_vertex_buffer(
        &mut self,
        slot: u32,
        buffer_ref: &ResourceRef<FrameGraphBuffer, ResourceRead>,
    ) -> &mut Self {
        self.render_pass.set_vertex_buffer(slot, buffer_ref);
        self
    }

    pub fn set_raw_bind_group(
        &mut self,
        index: u32,
        bind_group: Option<&BindGroup>,
        offsets: &[u32],
    ) -> &mut Self {
        self.render_pass
            .set_raw_bind_group(index, bind_group, offsets);

        self
    }

    pub fn read_from_board<ResourceType: GraphResource, Key: Into<ResourceBoardKey>>(
        &mut self,
        key: Key,
    ) -> Result<ResourceRef<ResourceType, ResourceRead>, FrameGraphError> {
        self.pass_node_builder.read_from_board(key)
    }

    pub fn import_and_read_buffer(
        &mut self,
        buffer: &Buffer,
    ) -> ResourceRef<FrameGraphBuffer, ResourceRead> {
        self.pass_node_builder.import_and_read_buffer(buffer)
    }

    pub fn import_and_read_texture(
        &mut self,
        texture: &Texture,
    ) -> ResourceRef<FrameGraphTexture, ResourceRead> {
        self.pass_node_builder.import_and_read_texture(texture)
    }

    pub fn set_viewport(&mut self, viewport: Option<Viewport>) -> &mut Self {
        if let Some(viewport) = viewport {
            let size = viewport.physical_size;
            let position = viewport.physical_position;
            self.render_pass
                .set_scissor_rect(position.x, position.y, size.x, size.y);
        }

        self
    }

    pub fn set_bind_group<T>(
        &mut self,
        index: u32,
        bind_group_ref: T,
        offsets: &[u32],
    ) -> Result<&mut Self, FrameGraphError>
    where
        T: BluePrintProvider<BluePrint = BindGroupBluePrint>,
    {
        let bind_group_ref = bind_group_ref.make_blue_print(&mut self.pass_node_builder)?;

        self.render_pass
            .set_bind_group(index, &bind_group_ref, offsets);
        Ok(self)
    }

    pub fn set_render_pipeline(&mut self, id: CachedRenderPipelineId) -> &mut Self {
        self.render_pass.set_render_pipeline(id);
        self
    }

    pub fn set_scissor_rect(&mut self, x: u32, y: u32, width: u32, height: u32) -> &mut Self {
        self.render_pass.set_scissor_rect(x, y, width, height);
        self
    }

    pub fn add_raw_color_attachment(&mut self, color_attachment: ColorAttachment) -> &mut Self {
        self.render_pass.add_raw_color_attachment(color_attachment);
        self
    }

    pub fn add_color_attachment<T>(&mut self, provider: &T) -> Result<&mut Self, FrameGraphError>
    where
        T: BluePrintProvider<BluePrint = ColorAttachmentBluePrint>,
    {
        let color_attachment = provider.make_blue_print(&mut self.pass_node_builder)?;

        self.render_pass.add_color_attachment(color_attachment);
        Ok(self)
    }

    pub fn set_depth_stencil_attachment<T>(
        &mut self,
        provider: &T,
    ) -> Result<&mut Self, FrameGraphError>
    where
        T: BluePrintProvider<BluePrint = DepthStencilAttachmentBluePrint>,
    {
        let color_attachment = provider.make_blue_print(&mut self.pass_node_builder)?;

        self.render_pass
            .set_depth_stencil_attachment(color_attachment);
        Ok(self)
    }
}

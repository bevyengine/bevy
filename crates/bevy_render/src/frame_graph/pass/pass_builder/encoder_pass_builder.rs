use std::mem::take;

use tracing::warn;
use wgpu::{Extent3d, ImageSubresourceRange};

use crate::{
    frame_graph::{
        EncoderPass, EncoderPassCommandBuilder, FrameGraphBuffer, FrameGraphTexture, ResourceMaterial, ResourceRead, ResourceRef, ResourceWrite, TexelCopyTextureInfo
    },
    render_resource::Buffer,
};

use super::PassBuilder;

pub struct EncoderPassBuilder<'a, 'b> {
    encoder_pass: EncoderPass,
    pass_builder: &'b mut PassBuilder<'a>,
}

impl<'a, 'b> Drop for EncoderPassBuilder<'a, 'b> {
    fn drop(&mut self) {
        self.finish();
    }
}

impl<'a, 'b> EncoderPassBuilder<'a, 'b> {
    pub fn new(pass_builder: &'b mut PassBuilder<'a>) -> Self {
        let encoder_pass = EncoderPass::default();

        Self {
            encoder_pass,
            pass_builder,
        }
    }

    pub fn read_material<M: ResourceMaterial>(
        &mut self,
        material: &M,
    ) -> ResourceRef<M::ResourceType, ResourceRead> {
        self.pass_builder.read_material(material)
    }

    pub fn write_material<M: ResourceMaterial>(
        &mut self,
        material: &M,
    ) -> ResourceRef<M::ResourceType, ResourceWrite> {
        self.pass_builder.write_material(material)
    }

    pub fn copy_texture_to_texture(
        &mut self,
        source: TexelCopyTextureInfo<ResourceRead>,
        destination: TexelCopyTextureInfo<ResourceWrite>,
        copy_size: Extent3d,
    ) -> &mut Self {
        self.encoder_pass
            .copy_texture_to_texture(source, destination, copy_size);

        self
    }

    pub fn clear_texture(
        &mut self,
        texture_ref: &ResourceRef<FrameGraphTexture, ResourceWrite>,
        subresource_range: ImageSubresourceRange,
    ) -> &mut Self {
        self.encoder_pass
            .clear_texture(texture_ref, subresource_range);

        self
    }

    pub fn clear_buffer(
        &mut self,
        buffer_ref: &ResourceRef<FrameGraphBuffer, ResourceWrite>,
        offset: u64,
        size: Option<u64>,
    ) -> &mut Self {
        self.encoder_pass.clear_buffer(buffer_ref, offset, size);

        self
    }

    fn finish(&mut self) {
        self.encoder_pass.finish();

        let encoder_pass = take(&mut self.encoder_pass);

        if encoder_pass.is_vaild() {
            self.pass_builder.add_executor(encoder_pass);
        } else {
            warn!("encoder pass must is vaild");
        }
    }
}

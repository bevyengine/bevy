use std::mem::take;

use tracing::warn;
use wgpu::{Extent3d, ImageSubresourceRange};

use crate::frame_graph::{
    EncoderPass, EncoderPassCommandBuilder, FrameGraphTexture, ResourceRead, ResourceRef,
    ResourceWrite, TexelCopyTextureInfo,
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

    fn finish(&mut self) {
        self.encoder_pass.finish();

        let encoder_pass = take(&mut self.encoder_pass);

        if encoder_pass.is_vaild() {
            self.pass_builder.add_executor(encoder_pass);
        } else {
            warn!("render pass must is vaild");
        }
    }
}

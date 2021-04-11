use super::{BufferId, SamplerId, TextureId};
use std::ops::Range;

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum RenderResourceBinding {
    Buffer {
        buffer: BufferId,
        range: Range<u64>,
        dynamic_index: Option<u32>,
    },
    Texture(TextureId),
    Sampler(SamplerId),
}

impl RenderResourceBinding {
    pub fn get_texture(&self) -> Option<TextureId> {
        if let RenderResourceBinding::Texture(texture) = self {
            Some(*texture)
        } else {
            None
        }
    }

    pub fn get_buffer(&self) -> Option<BufferId> {
        if let RenderResourceBinding::Buffer { buffer, .. } = self {
            Some(*buffer)
        } else {
            None
        }
    }

    pub fn is_dynamic_buffer(&self) -> bool {
        matches!(
            self,
            RenderResourceBinding::Buffer {
                dynamic_index: Some(_),
                ..
            }
        )
    }

    pub fn get_sampler(&self) -> Option<SamplerId> {
        if let RenderResourceBinding::Sampler(sampler) = self {
            Some(*sampler)
        } else {
            None
        }
    }
}

impl From<TextureId> for RenderResourceBinding {
    fn from(id: TextureId) -> Self {
        RenderResourceBinding::Texture(id)
    }
}

impl From<SamplerId> for RenderResourceBinding {
    fn from(id: SamplerId) -> Self {
        RenderResourceBinding::Sampler(id)
    }
}

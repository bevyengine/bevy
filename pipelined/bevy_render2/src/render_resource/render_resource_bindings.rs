use crate::render_resource::{BufferId, SamplerId, TextureViewId};
use std::ops::Range;

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum RenderResourceBinding {
    Buffer { buffer: BufferId, range: Range<u64> },
    TextureView(TextureViewId),
    Sampler(SamplerId),
}

impl RenderResourceBinding {
    pub fn get_texture_view(&self) -> Option<TextureViewId> {
        if let RenderResourceBinding::TextureView(texture) = self {
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

    pub fn get_sampler(&self) -> Option<SamplerId> {
        if let RenderResourceBinding::Sampler(sampler) = self {
            Some(*sampler)
        } else {
            None
        }
    }
}

impl From<TextureViewId> for RenderResourceBinding {
    fn from(id: TextureViewId) -> Self {
        RenderResourceBinding::TextureView(id)
    }
}

impl From<SamplerId> for RenderResourceBinding {
    fn from(id: SamplerId) -> Self {
        RenderResourceBinding::Sampler(id)
    }
}

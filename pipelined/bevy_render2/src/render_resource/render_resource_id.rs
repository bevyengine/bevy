use crate::render_resource::{BufferId, SamplerId, TextureViewId};

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub enum RenderResourceId {
    Buffer(BufferId),
    TextureView(TextureViewId),
    Sampler(SamplerId),
}

impl RenderResourceId {
    pub fn get_texture_view(&self) -> Option<TextureViewId> {
        if let RenderResourceId::TextureView(id) = self {
            Some(*id)
        } else {
            None
        }
    }

    pub fn get_buffer(&self) -> Option<BufferId> {
        if let RenderResourceId::Buffer(id) = self {
            Some(*id)
        } else {
            None
        }
    }

    pub fn get_sampler(&self) -> Option<SamplerId> {
        if let RenderResourceId::Sampler(id) = self {
            Some(*id)
        } else {
            None
        }
    }
}

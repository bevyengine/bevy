use crate::render_resource::{BufferId, SamplerId, TextureId};

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum RenderResourceType {
    Buffer,
    Texture,
    Sampler,
}

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub enum RenderResourceId {
    Buffer(BufferId),
    Texture(TextureId),
    Sampler(SamplerId),
}

impl RenderResourceId {
    pub fn get_texture(&self) -> Option<TextureId> {
        if let RenderResourceId::Texture(id) = self {
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

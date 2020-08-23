use std::{hash::{Hash, Hasher}, sync::Arc};
use uuid::Uuid;

#[derive(Clone, Debug)]
pub enum TextureId {
    // TODO: We need to separate out textures from views.
    Wgpu((Uuid, Option<Arc<wgpu::Texture>>, Arc<wgpu::TextureView>)),
    WgpuSwap(Uuid),
    None(Uuid),
}

impl TextureId {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        TextureId::None(Uuid::new_v4())
    }
}

impl PartialEq for TextureId {
    fn eq(&self, other: &Self) -> bool {
        match self {
            TextureId::Wgpu(uuid) => {
                match other {
                    TextureId::Wgpu(other_uuid) => {
                        uuid.0 == other_uuid.0
                    },
                    _ => false
                }
            },
            TextureId::WgpuSwap(uuid) => {
                match other {
                    TextureId::WgpuSwap(other_uuid) => {
                        uuid == other_uuid
                    },
                    _ => false
                }
            },
            TextureId::None(uuid) => {
                match other {
                    TextureId::None(other_uuid) => {
                        uuid == other_uuid
                    },
                    _ => false
                }
            }
        }
    }
    
}
impl Eq for TextureId {}

impl Hash for TextureId {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            TextureId::Wgpu(uuid) => {
                uuid.0.hash(state)
            }
            TextureId::WgpuSwap(uuid) => {
                uuid.hash(state)
            }
            TextureId::None(uuid) => {
                uuid.hash(state)
            }
        }
    }
}

#[derive(Clone, Debug)]
pub enum SamplerId {
    Wgpu((Uuid, Arc<wgpu::Sampler>)),
    None(Uuid)
}

impl SamplerId {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        SamplerId::None(Uuid::new_v4())
    }
}

impl PartialEq for SamplerId {
    fn eq(&self, other: &Self) -> bool {
        match self {
            SamplerId::Wgpu(uuid) => {
                match other {
                    SamplerId::Wgpu(other_uuid) => {
                        uuid.0 == other_uuid.0
                    },
                    _ => false
                }
            },
            SamplerId::None(uuid) => {
                match other {
                    SamplerId::None(other_uuid) => {
                        uuid == other_uuid
                    },
                    _ => false
                }
            }
        }
    }
    
}
impl Eq for SamplerId {}

impl Hash for SamplerId {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            SamplerId::Wgpu(uuid) => {
                uuid.0.hash(state)
            }
            SamplerId::None(uuid) => {
                uuid.hash(state)
            }
        }
    }
}

use bevy_utils::Uuid;

#[derive(Copy, Clone, Hash, Eq, PartialEq, Debug)]
pub struct TextureId(Uuid);

impl TextureId {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        TextureId(Uuid::new_v4())
    }
}

#[derive(Copy, Clone, Hash, Eq, PartialEq, Debug)]
pub struct SwapChainTextureId(Uuid);

impl SwapChainTextureId {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        SwapChainTextureId(Uuid::new_v4())
    }
}

#[derive(Copy, Clone, Hash, Eq, PartialEq, Debug)]
pub struct SamplerId(Uuid);

impl SamplerId {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        SamplerId(Uuid::new_v4())
    }
}

#[derive(Copy, Clone, Hash, Eq, PartialEq, Debug)]
pub struct TextureViewId(Uuid, TextureId);

impl TextureViewId {
    #[allow(clippy::new_without_default)]
    pub fn new(texture_id: TextureId) -> Self {
        TextureViewId(Uuid::new_v4(), texture_id)
    }

    pub fn get_texture_id(&self) -> TextureId {
        self.1
    }
}

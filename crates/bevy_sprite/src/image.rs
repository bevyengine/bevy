use crate::TextureAtlas;
use bevy_asset::Handle;
use bevy_ecs::component::Component;
use bevy_reflect::Reflect;
use bevy_render::texture::{Image, DEFAULT_IMAGE_HANDLE};

/// The sprite texture
#[derive(Component, Clone, Debug, Reflect)]
pub enum SpriteImage {
    /// Single texture
    Image(Handle<Image>),
    /// Texture atlas.
    TextureAtlas {
        /// Texture atlas handle
        handle: Handle<TextureAtlas>,
        /// Texture atlas index
        index: usize,
    },
}

impl Default for SpriteImage {
    fn default() -> Self {
        Self::Image(DEFAULT_IMAGE_HANDLE.typed())
    }
}

impl From<Handle<Image>> for SpriteImage {
    fn from(handle: Handle<Image>) -> Self {
        Self::Image(handle)
    }
}

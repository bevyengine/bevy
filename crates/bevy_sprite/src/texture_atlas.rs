use crate::TextureAtlasLayout;
use bevy_asset::{Assets, Handle};
use bevy_ecs::component::Component;
use bevy_math::Rect;
use bevy_reflect::Reflect;

/// Component used to draw a specific section of a texture.
///
/// It stores a handle to [`TextureAtlasLayout`] and the index of the current section of the atlas.
/// The texture atlas contains various *sections* of a given texture, allowing users to have a single
/// image file for either sprite animation or global mapping.
/// You can change the texture [`index`](Self::index) of the atlas to animate the sprite or dsplay only a *section* of the texture
/// for efficient rendering of related game objects.
///
/// Check the following examples for usage:
/// - [`animated sprite sheet example`](https://github.com/bevyengine/bevy/blob/latest/examples/2d/sprite_sheet.rs)
/// - [`texture atlas example`](https://github.com/bevyengine/bevy/blob/latest/examples/2d/texture_atlas.rs)
#[derive(Component, Default, Debug, Clone, Reflect)]
pub struct TextureAtlas {
    /// Texture atlas handle
    pub layout: Handle<TextureAtlasLayout>,
    /// Texture atlas section index
    pub index: usize,
}

impl From<Handle<TextureAtlasLayout>> for TextureAtlas {
    fn from(texture_atlas: Handle<TextureAtlasLayout>) -> Self {
        Self {
            layout: texture_atlas,
            index: 0,
        }
    }
}

impl TextureAtlas {
    /// Retrieves the current texture [`Rect`] of the sprite sheet according to the section `index`
    pub fn texture_rect(&self, texture_atlases: &Assets<TextureAtlasLayout>) -> Option<Rect> {
        let atlas = texture_atlases.get(&self.layout)?;
        atlas.textures.get(self.index).copied()
    }
}

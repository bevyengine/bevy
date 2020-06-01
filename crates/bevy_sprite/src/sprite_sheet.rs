use crate::Quad;
use bevy_asset::Handle;
use bevy_render::texture::Texture;

pub struct SpriteSheet {
    pub texture: Handle<Texture>,
    pub sprites: Vec<Quad>,
}

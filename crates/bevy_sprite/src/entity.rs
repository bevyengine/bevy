use crate::{
    render::SPRITE_PIPELINE_HANDLE, sprite::Sprite, ColorMaterial, Rect, QUAD_HANDLE,
};
use bevy_asset::Handle;
use bevy_derive::EntityArchetype;
use bevy_render::{mesh::Mesh, Renderable};

#[derive(EntityArchetype)]
pub struct SpriteEntity {
    pub sprite: Sprite,
    pub rect: Rect,
    pub mesh: Handle<Mesh>, // TODO: maybe abstract this out
    pub material: Handle<ColorMaterial>,
    pub renderable: Renderable,
}

impl Default for SpriteEntity {
    fn default() -> Self {
        SpriteEntity {
            sprite: Default::default(),
            rect: Default::default(),
            mesh: QUAD_HANDLE,
            material: Default::default(),
            renderable: Renderable {
                pipelines: vec![SPRITE_PIPELINE_HANDLE],
                ..Default::default()
            },
        }
    }
}

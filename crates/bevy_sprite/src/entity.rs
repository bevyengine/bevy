use crate::{
    render::SPRITE_PIPELINE_HANDLE, sprite::Sprite, ColorMaterial, Quad, QUAD_HANDLE, SpriteSheet, SPRITE_SHEET_PIPELINE_HANDLE,
};
use bevy_asset::Handle;
use bevy_derive::EntityArchetype;
use bevy_render::{mesh::Mesh, Renderable};
use bevy_transform::prelude::*;

#[derive(EntityArchetype)]
pub struct SpriteEntity {
    pub sprite: Sprite,
    pub quad: Quad,
    pub mesh: Handle<Mesh>, // TODO: maybe abstract this out
    pub material: Handle<ColorMaterial>,
    pub renderable: Renderable,
}

impl Default for SpriteEntity {
    fn default() -> Self {
        Self {
            sprite: Default::default(),
            quad: Default::default(),
            mesh: QUAD_HANDLE,
            material: Default::default(),
            renderable: Renderable {
                pipelines: vec![SPRITE_PIPELINE_HANDLE],
                ..Default::default()
            },
        }
    }
}

#[derive(EntityArchetype)]
pub struct SpriteSheetEntity {
    pub sprite: Sprite,
    pub sprite_sheet: Handle<SpriteSheet>,
    pub renderable: Renderable,
    pub local_to_world: LocalToWorld,
    pub translation: Translation,
    pub rotation: Rotation,
    pub scale: Scale,
}

impl Default for SpriteSheetEntity {
    fn default() -> Self {
        Self {
            sprite: Default::default(),
            sprite_sheet: Default::default(),
            renderable: Renderable {
                pipelines: vec![SPRITE_SHEET_PIPELINE_HANDLE],
                ..Default::default()
            },
            local_to_world: Default::default(),
            translation: Default::default(),
            rotation: Default::default(),
            scale: Default::default(),
        }
    }
}

use crate::{
    render::SPRITE_PIPELINE_HANDLE, sprite::Sprite, ColorMaterial, TextureAtlas,
    TextureAtlasSprite, QUAD_HANDLE, SPRITE_SHEET_PIPELINE_HANDLE,
};
use bevy_asset::Handle;
use bevy_ecs::Bundle;
use bevy_render::{
    mesh::Mesh,
    pipeline::{RenderPipeline, RenderPipelines},
    prelude::{Draw, Visible},
    render_graph::base::MainPass,
};
use bevy_transform::prelude::{GlobalTransform, Transform};

#[derive(Bundle, Clone)]
pub struct SpriteBundle {
    pub sprite: Sprite,
    pub mesh: Handle<Mesh>, // TODO: maybe abstract this out
    pub material: Handle<ColorMaterial>,
    pub main_pass: MainPass,
    pub draw: Draw,
    pub visible: Visible,
    pub render_pipelines: RenderPipelines,
    pub transform: Transform,
    pub global_transform: GlobalTransform,
}

impl Default for SpriteBundle {
    fn default() -> Self {
        Self {
            mesh: QUAD_HANDLE.typed(),
            render_pipelines: RenderPipelines::from_pipelines(vec![RenderPipeline::new(
                SPRITE_PIPELINE_HANDLE.typed(),
            )]),
            visible: Visible {
                is_transparent: true,
                ..Default::default()
            },
            main_pass: MainPass,
            draw: Default::default(),
            sprite: Default::default(),
            material: Default::default(),
            transform: Default::default(),
            global_transform: Default::default(),
        }
    }
}

/// A Bundle of components for drawing a single sprite from a sprite sheet (also referred
/// to as a `TextureAtlas`)
#[derive(Bundle, Clone)]
pub struct SpriteSheetBundle {
    /// The specific sprite from the texture atlas to be drawn
    pub sprite: TextureAtlasSprite,
    /// A handle to the texture atlas that holds the sprite images
    pub texture_atlas: Handle<TextureAtlas>,
    /// Data pertaining to how the sprite is drawn on the screen
    pub draw: Draw,
    pub visible: Visible,
    pub render_pipelines: RenderPipelines,
    pub main_pass: MainPass,
    pub mesh: Handle<Mesh>, // TODO: maybe abstract this out
    pub transform: Transform,
    pub global_transform: GlobalTransform,
}

impl Default for SpriteSheetBundle {
    fn default() -> Self {
        Self {
            render_pipelines: RenderPipelines::from_pipelines(vec![RenderPipeline::new(
                SPRITE_SHEET_PIPELINE_HANDLE.typed(),
            )]),
            visible: Visible {
                is_transparent: true,
                ..Default::default()
            },
            main_pass: MainPass,
            mesh: QUAD_HANDLE.typed(),
            draw: Default::default(),
            sprite: Default::default(),
            texture_atlas: Default::default(),
            transform: Default::default(),
            global_transform: Default::default(),
        }
    }
}

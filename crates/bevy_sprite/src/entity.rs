use crate::{
    render::SPRITE_PIPELINE_HANDLE, sprite::Sprite, ColorMaterial, TextureAtlas,
    TextureAtlasSprite, QUAD_HANDLE, SPRITE_SHEET_PIPELINE_HANDLE,
};
use bevy_asset::Handle;
use bevy_ecs::bundle::Bundle;
use bevy_render::{
    pipeline::{RenderPipeline, RenderPipelines},
    prelude::{MeshBundle, Visible},
};

#[derive(Bundle, Clone)]
pub struct SpriteBundle {
    pub sprite: Sprite,
    pub material: Handle<ColorMaterial>,
    #[bundle]
    pub mesh: MeshBundle,
}

impl Default for SpriteBundle {
    fn default() -> Self {
        Self {
            mesh: MeshBundle {
                mesh: QUAD_HANDLE.typed(),
                render_pipelines: RenderPipelines::from_pipelines(vec![RenderPipeline::new(
                    SPRITE_PIPELINE_HANDLE.typed(),
                )]),
                visible: Visible {
                    is_transparent: true,
                    ..Default::default()
                },
                ..Default::default()
            },
            sprite: Default::default(),
            material: Default::default(),
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
    #[bundle]
    pub mesh: MeshBundle,
}

impl Default for SpriteSheetBundle {
    fn default() -> Self {
        Self {
            mesh: MeshBundle {
                mesh: QUAD_HANDLE.typed(),
                render_pipelines: RenderPipelines::from_pipelines(vec![RenderPipeline::new(
                    SPRITE_SHEET_PIPELINE_HANDLE.typed(),
                )]),
                visible: Visible {
                    is_transparent: true,
                    ..Default::default()
                },
                ..Default::default()
            },
            sprite: Default::default(),
            texture_atlas: Default::default(),
        }
    }
}

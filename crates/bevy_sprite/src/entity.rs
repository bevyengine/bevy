use crate::{
    render::SPRITE_PIPELINE_HANDLE, sprite::Sprite, ColorMaterial, Quad, TextureAtlas,
    TextureAtlasSprite, QUAD_HANDLE, SPRITE_SHEET_PIPELINE_HANDLE,
};
use bevy_asset::Handle;
use bevy_app::EntityArchetype;
use bevy_render::{mesh::Mesh, draw::{Draw, RenderPipelines}};

#[derive(EntityArchetype)]
pub struct SpriteEntity {
    pub sprite: Sprite,
    pub quad: Quad,
    pub mesh: Handle<Mesh>, // TODO: maybe abstract this out
    pub material: Handle<ColorMaterial>,
    pub draw: Draw,
    pub render_pipelines: RenderPipelines,
}

impl Default for SpriteEntity {
    fn default() -> Self {
        Self {
            sprite: Default::default(),
            quad: Default::default(),
            mesh: QUAD_HANDLE,
            material: Default::default(),
            draw: Default::default(),
            render_pipelines: RenderPipelines {
                pipelines: vec![SPRITE_PIPELINE_HANDLE],
                ..Default::default()
            },
        }
    }
}

#[derive(EntityArchetype)]
pub struct SpriteSheetEntity {
    pub sprite: TextureAtlasSprite,
    pub texture_atlas: Handle<TextureAtlas>,
    pub draw: Draw,
    pub render_pipelines: RenderPipelines,
    pub mesh: Handle<Mesh>, // TODO: maybe abstract this out
                            // pub transform: Transform,
                            // pub translation: Translation,
                            // pub rotation: Rotation,
                            // pub scale: Scale,
}

impl Default for SpriteSheetEntity {
    fn default() -> Self {
        Self {
            sprite: Default::default(),
            texture_atlas: Default::default(),
            draw: Default::default(),
            render_pipelines: RenderPipelines {
                pipelines: vec![SPRITE_SHEET_PIPELINE_HANDLE],
                ..Default::default()
            },
            mesh: QUAD_HANDLE,
            // transform: Default::default(),
            // translation: Default::default(),
            // rotation: Default::default(),
            // scale: Default::default(),
        }
    }
}

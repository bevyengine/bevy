//! Text Gizmo functions

use std::marker::PhantomData;

use crate::config::GizmoConfigGroup;
use crate::gizmos::Gizmos;
use crate::prelude::Gizmo;
use bevy_asset::Assets;
use bevy_color::Color;
use bevy_ecs::entity::Entity;
use bevy_ecs::resource::Resource;
use bevy_ecs::schedule::CompactNodeIdAndDirection;
use bevy_ecs::system::{Res, ResMut};
use bevy_image::{Image, TextureAtlasLayout};
use bevy_math::Isometry2d;
use bevy_text::{
    ComputedTextBlock, CosmicFontSystem, Font, FontAtlasSet, LineHeight, SwashCache, TextBounds,
    TextFont, TextLayout, TextLayoutInfo, TextPipeline,
};
use bevy_utils::default;

#[derive(Resource, Default)]
pub struct GizmoTextBuffer<Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    pub text: Vec<GizmoText>,
    phantom: PhantomData<(Config, Clear)>,
}

pub struct GizmoText {
    pub isometry: Isometry2d,
    pub text: String,
    pub size: f32,
    pub color: Color,
}

pub fn gizmo_text_system<Config, Clear>(
    mut textures: ResMut<Assets<Image>>,
    fonts: Res<Assets<Font>>,
    mut text_pipeline: ResMut<TextPipeline>,
    mut texture_atlases: ResMut<Assets<TextureAtlasLayout>>,
    mut font_atlas_set: ResMut<FontAtlasSet>,
    mut font_system: ResMut<CosmicFontSystem>,
    mut swash_cache: ResMut<SwashCache>,
    mut gizmos: Gizmos<Config, Clear>,
) where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    let texts = core::mem::take(&mut gizmos.text_buffer.text);
    let mut info = TextLayoutInfo::default();
    for text in texts.iter() {
        let mut block = ComputedTextBlock::default();
        let span = (
            Entity::PLACEHOLDER,
            0,
            text.text.as_str(),
            &TextFont {
                font_size: text.size,
                ..default()
            },
            text.color,
            LineHeight::default(),
        );

        let Ok(()) = text_pipeline.queue_text(
            &mut info,
            &fonts,
            core::iter::once(span),
            1.,
            &TextLayout::new_with_no_wrap(),
            TextBounds::UNBOUNDED,
            &mut font_atlas_set,
            &mut texture_atlases,
            &mut textures,
            &mut block,
            &mut font_system,
            &mut swash_cache,
        ) else {
            continue;
        };

        println!("result: {}", info.glyphs.len());
    }
}

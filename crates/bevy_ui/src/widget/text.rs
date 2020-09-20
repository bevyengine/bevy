use crate::{CalculatedSize, Node};
use bevy_asset::{Assets, Handle};
use bevy_core::FloatOrd;
use bevy_ecs::{Changed, Local, Query, Res, ResMut};
use bevy_math::Size;
use bevy_render::{
    draw::{Draw, DrawContext, Drawable},
    prelude::Msaa,
    renderer::{AssetRenderResourceBindings, RenderResourceBindings},
    texture::Texture,
};
use bevy_sprite::TextureAtlas;
use bevy_text::{DrawableText, Font, FontAtlasSet, TextStyle};
use bevy_transform::prelude::GlobalTransform;
use bevy_utils::HashSet;

#[derive(Default)]
pub struct QueuedTextGlyphs {
    glyphs: HashSet<(Handle<Font>, FloatOrd, char)>,
}

#[derive(Default, Clone)]
pub struct Text {
    pub value: String,
    pub font: Handle<Font>,
    pub style: TextStyle,
}

pub fn text_system(
    mut queued_text_glyphs: Local<QueuedTextGlyphs>,
    mut textures: ResMut<Assets<Texture>>,
    fonts: Res<Assets<Font>>,
    mut font_atlas_sets: ResMut<Assets<FontAtlasSet>>,
    mut texture_atlases: ResMut<Assets<TextureAtlas>>,
    mut query: Query<(Changed<Text>, &mut CalculatedSize)>,
) {
    // add queued glyphs to atlases
    if !queued_text_glyphs.glyphs.is_empty() {
        let mut glyphs_to_queue = Vec::new();
        for (font_handle, FloatOrd(font_size), character) in queued_text_glyphs.glyphs.drain() {
            let font_atlases = font_atlas_sets
                .get_or_insert_with(Handle::from_id(font_handle.id), || {
                    FontAtlasSet::new(font_handle)
                });

            // try adding the glyph to an atlas. if it fails, re-queue
            if let Ok(char_str) = std::str::from_utf8(&[character as u8]) {
                if font_atlases
                    .add_glyphs_to_atlas(
                        &fonts,
                        &mut texture_atlases,
                        &mut textures,
                        font_size,
                        char_str,
                    )
                    .is_none()
                {
                    glyphs_to_queue.push((font_handle, FloatOrd(font_size), character));
                }
            }
        }

        queued_text_glyphs.glyphs.extend(glyphs_to_queue);
    }

    for (text, mut calculated_size) in &mut query.iter() {
        let font_atlases = font_atlas_sets
            .get_or_insert_with(Handle::from_id(text.font.id), || {
                FontAtlasSet::new(text.font)
            });
        // TODO: this call results in one or more TextureAtlases, whose render resources are created in the RENDER_GRAPH_SYSTEMS
        // stage. That logic runs _before_ the DRAW stage, which means we cant call add_glyphs_to_atlas in the draw stage
        // without our render resources being a frame behind. Therefore glyph atlasing either needs its own system or the TextureAtlas
        // resource generation needs to happen AFTER the render graph systems. maybe draw systems should execute within the
        // render graph so ordering like this can be taken into account? Maybe the RENDER_GRAPH_SYSTEMS stage should be removed entirely
        // in favor of node.update()? Regardless, in the immediate short term the current approach is fine.
        if let Some(width) = font_atlases.add_glyphs_to_atlas(
            &fonts,
            &mut texture_atlases,
            &mut textures,
            text.style.font_size,
            &text.value,
        ) {
            calculated_size.size = Size::new(width, text.style.font_size);
        } else {
            for character in text.value.chars() {
                queued_text_glyphs.glyphs.insert((
                    text.font,
                    FloatOrd(text.style.font_size),
                    character,
                ));
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub fn draw_text_system(
    mut draw_context: DrawContext,
    fonts: Res<Assets<Font>>,
    msaa: Res<Msaa>,
    font_atlas_sets: Res<Assets<FontAtlasSet>>,
    texture_atlases: Res<Assets<TextureAtlas>>,
    mut render_resource_bindings: ResMut<RenderResourceBindings>,
    mut asset_render_resource_bindings: ResMut<AssetRenderResourceBindings>,
    mut query: Query<(&mut Draw, &Text, &Node, &GlobalTransform)>,
) {
    for (mut draw, text, node, global_transform) in &mut query.iter() {
        if let Some(font) = fonts.get(&text.font) {
            let position = global_transform.translation() - (node.size / 2.0).extend(0.0);
            let mut drawable_text = DrawableText {
                font,
                font_atlas_set: font_atlas_sets
                    .get(&text.font.as_handle::<FontAtlasSet>())
                    .unwrap(),
                texture_atlases: &texture_atlases,
                render_resource_bindings: &mut render_resource_bindings,
                asset_render_resource_bindings: &mut asset_render_resource_bindings,
                position,
                msaa: &msaa,
                style: &text.style,
                text: &text.value,
                container_size: node.size,
            };
            drawable_text.draw(&mut draw, &mut draw_context).unwrap();
        }
    }
}

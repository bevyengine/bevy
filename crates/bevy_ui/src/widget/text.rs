use crate::{CalculatedSize, Node};
use bevy_asset::{Assets, Handle};
use bevy_ecs::{Changed, Query, Res, ResMut};
use bevy_math::{Size, Vec3};
use bevy_render::{
    draw::{Draw, DrawContext, Drawable},
    prelude::Msaa,
    renderer::{AssetRenderResourceBindings, RenderResourceBindings},
    texture::Texture,
};
use bevy_sprite::TextureAtlas;
use bevy_text::{DrawableText, Font, FontAtlasSet, TextStyle};
use bevy_transform::prelude::Transform;

#[derive(Default)]
pub struct Text {
    pub value: String,
    pub font: Handle<Font>,
    pub style: TextStyle,
}

pub fn text_system(
    mut textures: ResMut<Assets<Texture>>,
    fonts: Res<Assets<Font>>,
    mut font_atlas_sets: ResMut<Assets<FontAtlasSet>>,
    mut texture_atlases: ResMut<Assets<TextureAtlas>>,
    mut query: Query<(Changed<Text>, &mut CalculatedSize)>,
) {
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
        let width = font_atlases.add_glyphs_to_atlas(
            &fonts,
            &mut texture_atlases,
            &mut textures,
            text.style.font_size,
            &text.value,
        );

        calculated_size.size = Size::new(width, text.style.font_size);
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
    mut query: Query<(&mut Draw, &Text, &Node, &Transform)>,
) {
    for (mut draw, text, node, transform) in &mut query.iter() {
        let position =
            Vec3::from(transform.value.w_axis().truncate()) - (node.size / 2.0).extend(0.0);

        let mut drawable_text = DrawableText {
            font: fonts.get(&text.font).unwrap(),
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

use bevy_asset::{Assets, Handle};
use bevy_render::{
    draw::{Draw, DrawContext, Drawable},
    render_resource::{AssetRenderResourceBindings, RenderResourceBindings},
    texture::Texture,
    Color,
};
use bevy_sprite::{ComMut, Quad, TextureAtlas};
use bevy_text::{DrawableText, Font, FontAtlasSet, TextStyle};
use glam::Vec3;
use legion::prelude::{Com, Res, ResMut};

pub struct Label {
    pub text: String,
    pub font: Handle<Font>,
    pub style: TextStyle,
}

impl Default for Label {
    fn default() -> Self {
        Label {
            text: String::new(),
            style: TextStyle {
                color: Color::WHITE,
                font_size: 12.0,
            },
            font: Handle::default(),
        }
    }
}

impl Label {
    // PERF: this is horrendously inefficient. (1) new texture per label per frame (2) no atlas
    pub fn label_system(
        mut textures: ResMut<Assets<Texture>>,
        fonts: Res<Assets<Font>>,
        mut font_atlas_sets: ResMut<Assets<FontAtlasSet>>,
        mut texture_atlases: ResMut<Assets<TextureAtlas>>,
        label: Com<Label>,
    ) {
        let font_atlases = font_atlas_sets
            .get_or_insert_with(Handle::from_id(label.font.id), || {
                FontAtlasSet::new(label.font)
            });
        // TODO: this call results in one or more TextureAtlases, whose render resources are created in the RENDER_GRAPH_SYSTEMS
        // stage. That logic runs _before_ the DRAW stage, which means we cant call add_glyphs_to_atlas in the draw stage
        // without or render resources being a frame behind. Therefore glyph atlasing either needs its own system or the TextureAtlas
        // resource generation needs to happen AFTER the render graph systems. maybe draw systems should execute within the
        // render graph so ordering like this can be taken into account? Maybe the RENDER_GRAPH_SYSTEMS stage should be removed entirely
        // in favor of node.update()? Regardless, in the immediate short term the current approach is fine.
        font_atlases.add_glyphs_to_atlas(
            &fonts,
            &mut texture_atlases,
            &mut textures,
            label.style.font_size,
            &label.text,
        );
    }

    pub fn draw_label_system(
        mut draw_context: DrawContext,
        fonts: Res<Assets<Font>>,
        font_atlas_sets: Res<Assets<FontAtlasSet>>,
        texture_atlases: Res<Assets<TextureAtlas>>,
        mut render_resource_bindings: ResMut<RenderResourceBindings>,
        mut asset_render_resource_bindings: ResMut<AssetRenderResourceBindings>,
        mut draw: ComMut<Draw>,
        label: Com<Label>,
        quad: Com<Quad>,
    ) {
        let position = quad.position - quad.size / 2.0;
        let mut drawable_text = DrawableText::new(
            fonts.get(&label.font).unwrap(),
            font_atlas_sets
                .get(&label.font.as_handle::<FontAtlasSet>())
                .unwrap(),
            &texture_atlases,
            &mut render_resource_bindings,
            &mut asset_render_resource_bindings,
            Vec3::new(position.x(), position.y(), quad.z_index),
            &label.style,
            &label.text,
        );
        drawable_text.draw(&mut draw, &mut draw_context).unwrap();
    }
}

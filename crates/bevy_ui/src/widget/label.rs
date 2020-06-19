use bevy_asset::{Assets, Handle};
use bevy_render::{
    draw::{Draw, DrawContext, Drawable},
    render_resource::{AssetRenderResourceBindings, RenderResourceBindings},
    texture::Texture,
    Color,
};
use bevy_sprite::{ColorMaterial, ComMut, Quad, TextureAtlas};
use bevy_text::{DrawableText, Font, FontAtlasSet, TextStyle};
use legion::prelude::{Com, Res, ResMut};
use glam::Vec3;

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
        mut color_materials: ResMut<Assets<ColorMaterial>>,
        mut textures: ResMut<Assets<Texture>>,
        fonts: Res<Assets<Font>>,
        mut font_atlas_sets: ResMut<Assets<FontAtlasSet>>,
        mut texture_atlases: ResMut<Assets<TextureAtlas>>,
        label: Com<Label>,
        quad: Com<Quad>,
        color_material_handle: Com<Handle<ColorMaterial>>,
    ) {
        // ensure the texture is at least 1x1
        let width = quad.size.x().max(1.0);
        let height = quad.size.y().max(1.0);

        if let Some(font) = fonts.get(&label.font) {
            let font_atlases = font_atlas_sets
                .get_or_insert_with(Handle::from_id(label.font.id), || {
                    FontAtlasSet::new(label.font)
                });
            font_atlases.add_glyphs_to_atlas(
                &fonts,
                &mut texture_atlases,
                &mut textures,
                label.style.font_size,
                &label.text,
            );

            let material = color_materials.get_or_insert_with(*color_material_handle, || {
                ColorMaterial::from(Handle::<Texture>::new())
            });

            let texture = font.render_text(
                &label.text,
                label.style.color,
                label.style.font_size,
                width as usize,
                height as usize,
            );

            material.texture = Some(textures.add(texture));
        }
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
        let mut drawable_text = DrawableText::new(
            fonts.get(&label.font).unwrap(),
            font_atlas_sets
                .get(&label.font.as_handle::<FontAtlasSet>())
                .unwrap(),
            &texture_atlases,
            &mut render_resource_bindings,
            &mut asset_render_resource_bindings,
            Vec3::new(quad.position.x(), quad.position.y(), 0.0),
            &label.style,
            &label.text,
        );
        drawable_text.draw(&mut draw, &mut draw_context).unwrap();
    }
}

use crate::{ColorMaterial, Rect, Res, ResMut};
use bevy_asset::{Assets, Handle};
use bevy_render::{texture::Texture, Color};
use bevy_text::Font;
use legion::prelude::Com;

pub struct Label {
    pub text: String,
    pub color: Color,
    pub font_size: f32,
    pub font: Handle<Font>,
}

impl Default for Label {
    fn default() -> Self {
        Label {
            text: String::new(),
            color: Color::WHITE,
            font_size: 12.0,
            font: Handle::default(),
        }
    }
}

impl Label {
    // PERF: this is horrendously inefficient. (1) new texture every frame (2) no atlas (3) new texture for every label
    pub fn label_system(
        mut color_materials: ResMut<Assets<ColorMaterial>>,
        mut textures: ResMut<Assets<Texture>>,
        fonts: Res<Assets<Font>>,
        label: Com<Label>,
        rect: Com<Rect>,
        color_material_handle: Com<Handle<ColorMaterial>>,
    ) {
        if let Some(font) = fonts.get(&label.font) {
            let texture = font.render_text(
                &label.text,
                label.color,
                rect.size.x() as usize,
                rect.size.y() as usize,
            );

            let material = color_materials.get_or_insert_with(*color_material_handle, || ColorMaterial::from(Handle::<Texture>::new()));
            if let Some(texture) = material.texture {
                // TODO: remove texture
            }

            material.texture = Some(textures.add(texture));
        }
    }
}

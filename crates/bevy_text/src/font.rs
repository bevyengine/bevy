use ab_glyph::{FontArc, FontVec, InvalidFont, OutlinedGlyph};
use bevy_reflect::TypeUuid;
use bevy_render::{
    color::Color,
    texture::{Extent3d, Texture, TextureDimension, TextureFormat},
};

#[derive(Debug, TypeUuid)]
#[uuid = "97059ac6-c9ba-4da9-95b6-bed82c3ce198"]
pub struct Font {
    pub font: FontArc,
}

impl Font {
    pub fn try_from_bytes(font_data: Vec<u8>) -> Result<Self, InvalidFont> {
        let font = FontVec::try_from_vec(font_data)?;
        let font = FontArc::new(font);
        Ok(Font { font })
    }

    pub fn get_outlined_glyph_texture(outlined_glyph: OutlinedGlyph) -> Texture {
        let bounds = outlined_glyph.px_bounds();
        let width = bounds.width() as usize;
        let height = bounds.height() as usize;
        let mut alpha = vec![0.0; width * height];
        outlined_glyph.draw(|x, y, v| {
            alpha[y as usize * width + x as usize] = v;
        });

        // TODO: make this texture grayscale
        let color = Color::WHITE;
        let color_u8 = [
            (color.r() * 255.0) as u8,
            (color.g() * 255.0) as u8,
            (color.b() * 255.0) as u8,
        ];
        Texture::new(
            Extent3d::new(width as u32, height as u32, 1),
            TextureDimension::D2,
            alpha
                .iter()
                .map(|a| {
                    vec![
                        color_u8[0],
                        color_u8[1],
                        color_u8[2],
                        (color.a() * a * 255.0) as u8,
                    ]
                })
                .flatten()
                .collect::<Vec<u8>>(),
            TextureFormat::Rgba8UnormSrgb,
        )
    }
}

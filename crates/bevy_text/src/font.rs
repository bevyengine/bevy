use ab_glyph::{FontArc, FontVec, InvalidFont, OutlinedGlyph};
use bevy_asset::Asset;
use bevy_reflect::TypePath;
use bevy_render::{
    render_asset::RenderAssetUsages,
    render_resource::{Extent3d, TextureDimension, TextureFormat},
    texture::Image,
};

#[derive(Asset, TypePath, Debug, Clone)]
pub struct Font {
    pub font: FontArc,
}

impl Font {
    pub fn try_from_bytes(font_data: Vec<u8>) -> Result<Self, InvalidFont> {
        let font = FontVec::try_from_vec(font_data)?;
        let font = FontArc::new(font);
        Ok(Font { font })
    }

    pub fn get_outlined_glyph_texture(outlined_glyph: OutlinedGlyph) -> Image {
        let bounds = outlined_glyph.px_bounds();
        // Increase the length of the glyph texture by 2-pixels on each axis to make space
        // for a pixel wide transparent border along its edges.
        let width = bounds.width() as usize + 2;
        let height = bounds.height() as usize + 2;
        let mut alpha = vec![0.0; width * height];
        outlined_glyph.draw(|x, y, v| {
            // Displace the glyph by 1 pixel on each axis so that it is drawn in the center of the texture.
            // This leaves a pixel wide transparent border around the glyph.
            alpha[(y + 1) as usize * width + x as usize + 1] = v;
        });

        // TODO: make this texture grayscale
        Image::new(
            Extent3d {
                width: width as u32,
                height: height as u32,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            alpha
                .iter()
                .flat_map(|a| vec![255, 255, 255, (*a * 255.0) as u8])
                .collect::<Vec<u8>>(),
            TextureFormat::Rgba8UnormSrgb,
            // This glyph image never needs to reach the render world because it's placed
            // into a font texture atlas that'll be used for rendering.
            RenderAssetUsages::MAIN_WORLD,
        )
    }
}

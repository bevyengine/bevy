use ab_glyph::{FontArc, FontVec, InvalidFont, OutlinedGlyph};
use bevy_reflect::{TypePath, TypeUuid};
use bevy_render::{
    render_resource::{Extent3d, TextureDimension, TextureFormat},
    texture::Image,
};

#[derive(Debug, TypeUuid, TypePath, Clone)]
#[uuid = "97059ac6-c9ba-4da9-95b6-bed82c3ce198"]
pub struct Font {
    pub font: FontArc,
    pub data: std::sync::Arc<Vec<u8>>,
}

impl Font {
    pub fn try_from_bytes(font_data: Vec<u8>) -> Result<Self, InvalidFont> {
        // DEBUGGING:
        // eprintln!("loading font {}", font_data.len());
        let font = FontVec::try_from_vec(font_data.clone())?;
        let font = FontArc::new(font);
        // TODO: validate font
        Ok(Font {
            font,
            data: std::sync::Arc::new(font_data),
        })
    }

    // TODO: consider  moving to pipeline.rs
    pub fn get_outlined_glyph_texture_new(
        font_system: &mut cosmic_text::FontSystem,
        swash_cache: &mut cosmic_text::SwashCache,
        layout_glyph: &cosmic_text::LayoutGlyph,
    ) -> (Image, i32, i32, u32, u32) {
        // TODO: consider using cosmic_text's own caching mechanism
        let image = swash_cache
            .get_image_uncached(font_system, layout_glyph.cache_key)
            // TODO: don't unwrap
            .unwrap();

        let width = image.placement.width;
        let height = image.placement.height;

        let data = match image.content {
            cosmic_text::SwashContent::Mask => image
                .data
                .iter()
                .flat_map(|a| [255, 255, 255, *a])
                .collect(),
            cosmic_text::SwashContent::Color => image.data,
            cosmic_text::SwashContent::SubpixelMask => {
                // TODO
                todo!()
            }
        };

        // TODO: make this texture grayscale
        (
            Image::new(
                Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                TextureDimension::D2,
                data,
                TextureFormat::Rgba8UnormSrgb,
            ),
            image.placement.left,
            image.placement.top,
            width,
            height,
        )
    }

    pub fn get_outlined_glyph_texture(outlined_glyph: OutlinedGlyph) -> Image {
        let bounds = outlined_glyph.px_bounds();
        let width = bounds.width() as usize;
        let height = bounds.height() as usize;
        let mut alpha = vec![0.0; width * height];
        outlined_glyph.draw(|x, y, v| {
            alpha[y as usize * width + x as usize] = v;
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
        )
    }
}

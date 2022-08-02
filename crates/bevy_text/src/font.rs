use ab_glyph::{FontArc, FontVec, InvalidFont, OutlinedGlyph};
use bevy_asset::{AssetPath, Handle};
use bevy_reflect::{FromReflect, Reflect, TypeUuid};
use bevy_render::{
    render_resource::{Extent3d, TextureDimension, TextureFormat},
    texture::Image,
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

/// A reference to a font asset.
#[derive(Clone, Debug, Reflect, FromReflect, Default)]
pub enum FontRef {
    /// Use the default font it can be configured with the [`DefaultFont`] resource
    #[default]
    Default,
    /// A handle to a font stored in the [`Assets<Font>`](bevy_asset::Assets) resource
    Handle(Handle<Font>),
    /// An asset path leading to a font
    Path(AssetPath<'static>),
}

impl From<Handle<Font>> for FontRef {
    fn from(handle: Handle<Font>) -> Self {
        Self::Handle(handle)
    }
}

impl From<AssetPath<'static>> for FontRef {
    fn from(path: AssetPath<'static>) -> Self {
        Self::Path(path)
    }
}

impl From<&'static str> for FontRef {
    fn from(path: &'static str) -> Self {
        Self::Path(AssetPath::from(path))
    }
}

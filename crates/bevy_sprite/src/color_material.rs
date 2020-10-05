use bevy_asset::{self, Handle};
use bevy_render::{color::Color, renderer::RenderResources, shader::ShaderDefs, texture::Texture};

#[derive(Debug, RenderResources, ShaderDefs)]
pub struct ColorMaterial {
    pub color: Color,
    #[shader_def]
    pub texture: Option<Handle<Texture>>,
}

impl ColorMaterial {
    pub fn color(color: Color) -> Self {
        ColorMaterial {
            color,
            texture: None,
        }
    }

    pub fn texture(texture: Handle<Texture>) -> Self {
        ColorMaterial {
            color: Color::WHITE,
            texture: Some(texture),
        }
    }

    pub fn modulated_texture(texture: Handle<Texture>, color: Color) -> Self {
        ColorMaterial {
            color,
            texture: Some(texture),
        }
    }
}

impl Default for ColorMaterial {
    fn default() -> Self {
        ColorMaterial {
            color: Color::rgb(1.0, 1.0, 1.0),
            texture: None,
        }
    }
}

impl From<Color> for ColorMaterial {
    fn from(color: Color) -> Self {
        ColorMaterial::color(color)
    }
}

impl From<Handle<Texture>> for ColorMaterial {
    fn from(texture: Handle<Texture>) -> Self {
        ColorMaterial::texture(texture)
    }
}

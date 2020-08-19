use bevy_asset::{self, Handle};
use bevy_render::{color::Color, renderer::RenderResources, shader::ShaderDefs, texture::Texture};

#[derive(RenderResources, ShaderDefs)]
pub struct ColorMaterial {
    pub color: Color,
    pub flip_horz: f32,
    pub flip_vert: f32,
    #[shader_def]
    pub texture: Option<Handle<Texture>>,
}

impl ColorMaterial {
    pub fn color(color: Color) -> Self {
        ColorMaterial {
            color,
            texture: None,
            ..Default::default()
        }
    }

    pub fn texture(texture: Handle<Texture>) -> Self {
        ColorMaterial {
            color: Color::WHITE,
            texture: Some(texture),
            ..Default::default()
        }
    }

    pub fn modulated_texture(texture: Handle<Texture>, color: Color) -> Self {
        ColorMaterial {
            color,
            texture: Some(texture),
            ..Default::default()
        }
    }
}

impl Default for ColorMaterial {
    fn default() -> Self {
        ColorMaterial {
            color: Color::rgb(1.0, 1.0, 1.0),
            texture: None,
            flip_horz: 0.0,
            flip_vert: 0.0,
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

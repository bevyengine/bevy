use crate::{
    asset::{Handle, Texture},
    core::GetBytes,
    math::Vec4,
    render::shader::ShaderDefSuffixProvider,
};
use std::ops::Add;

#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct Color(Vec4);

impl Color {
    pub fn rgb(r: f32, g: f32, b: f32) -> Color {
        Color(Vec4::new(r, g, b, 1.0))
    }

    pub fn rgba(r: f32, g: f32, b: f32, a: f32) -> Color {
        Color(Vec4::new(r, g, b, a))
    }
}

impl Add<Color> for Color {
    type Output = Color;
    fn add(self, rhs: Color) -> Self::Output {
        Color(self.0 + rhs.0)
    }
}

impl Add<Vec4> for Color {
    type Output = Color;
    fn add(self, rhs: Vec4) -> Self::Output {
        Color(self.0 + rhs)
    }
}

impl From<Vec4> for Color {
    fn from(vec4: Vec4) -> Self {
        Color(vec4)
    }
}

impl Into<[f32; 4]> for Color {
    fn into(self) -> [f32; 4] {
        self.0.into()
    }
}

impl GetBytes for Color {
    fn get_bytes(&self) -> Vec<u8> {
        self.0.get_bytes()
    }
    fn get_bytes_ref(&self) -> Option<&[u8]> {
        self.0.get_bytes_ref()
    }
}

pub enum ColorSource {
    Color(Color),
    Texture(Handle<Texture>),
}

impl From<Vec4> for ColorSource {
    fn from(vec4: Vec4) -> Self {
        ColorSource::Color(vec4.into())
    }
}

impl From<Color> for ColorSource {
    fn from(color: Color) -> Self {
        ColorSource::Color(color)
    }
}

impl From<Handle<Texture>> for ColorSource {
    fn from(texture: Handle<Texture>) -> Self {
        ColorSource::Texture(texture)
    }
}

impl ShaderDefSuffixProvider for ColorSource {
    fn get_shader_def(&self) -> Option<&'static str> {
        match *self {
            ColorSource::Color(_) => Some("_COLOR"),
            ColorSource::Texture(_) => Some("_TEXTURE"),
        }
    }
}

impl GetBytes for ColorSource {
    fn get_bytes(&self) -> Vec<u8> {
        match *self {
            ColorSource::Color(ref color) => color.get_bytes(),
            ColorSource::Texture(_) => Vec::new(), // Texture is not a uniform
        }
    }
    fn get_bytes_ref(&self) -> Option<&[u8]> {
        None
    }
}

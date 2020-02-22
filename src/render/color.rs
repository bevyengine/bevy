use crate::{
    asset::{Handle, Texture},
    core::GetBytes,
    math::Vec4,
    render::render_graph::ShaderDefSuffixProvider,
};

pub enum ColorSource {
    Color(Vec4),
    Texture(Handle<Texture>),
}

impl From<Vec4> for ColorSource {
    fn from(vec4: Vec4) -> Self {
        ColorSource::Color(vec4)
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
            ColorSource::Color(color) => color.get_bytes(),
            ColorSource::Texture(_) => Vec::new(), // Texture is not a uniform
        }
    }
    fn get_bytes_ref(&self) -> Option<&[u8]> {
        None
    }
}

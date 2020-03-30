use crate::{
    asset::Handle,
    core::GetBytes,
    render::{
        color::ColorSource,
        pipeline::{BindType, VertexBufferDescriptor},
        texture::Texture,
    },
};

pub trait AsUniforms {
    fn get_field_infos() -> &'static [FieldInfo];
    fn get_uniform_bytes(&self, name: &str) -> Option<Vec<u8>>;
    fn get_uniform_texture(&self, name: &str) -> Option<Handle<Texture>>;
    fn get_shader_defs(&self) -> Option<Vec<String>>;
    fn get_field_bind_type(&self, name: &str) -> Option<FieldBindType>;
    fn get_uniform_bytes_ref(&self, name: &str) -> Option<&[u8]>;
    fn get_vertex_buffer_descriptor() -> Option<&'static VertexBufferDescriptor>;
}

pub trait ShaderDefSuffixProvider {
    fn get_shader_def(&self) -> Option<&'static str>;
}

impl ShaderDefSuffixProvider for bool {
    fn get_shader_def(&self) -> Option<&'static str> {
        match *self {
            true => Some(""),
            false => None,
        }
    }
}

pub enum FieldBindType {
    Uniform { size: usize },
    Texture,
}

pub struct FieldInfo {
    pub name: &'static str,
    pub uniform_name: &'static str,
    pub texture_name: &'static str,
    pub sampler_name: &'static str,
    pub is_instanceable: bool,
}

pub trait AsFieldBindType {
    fn get_field_bind_type(&self) -> Option<FieldBindType>;
}

impl AsFieldBindType for ColorSource {
    fn get_field_bind_type(&self) -> Option<FieldBindType> {
        match *self {
            ColorSource::Texture(_) => Some(FieldBindType::Texture),
            ColorSource::Color(color) => color.get_field_bind_type(),
        }
    }
}

impl AsFieldBindType for Option<Handle<Texture>> {
    fn get_field_bind_type(&self) -> Option<FieldBindType> {
        match *self {
            Some(_) => Some(FieldBindType::Texture),
            None => None,
        }
    }
}

impl AsFieldBindType for Handle<Texture> {
    fn get_field_bind_type(&self) -> Option<FieldBindType> {
        Some(FieldBindType::Texture)
    }
}

impl<T> AsFieldBindType for T
where
    T: GetBytes,
{
    // TODO: this breaks if get_bytes_ref() isn't supported for a datatype
    default fn get_field_bind_type(&self) -> Option<FieldBindType> {
        Some(FieldBindType::Uniform {
            size: self.get_bytes_ref().unwrap().len(),
        })
    }
}

pub trait GetTexture {
    fn get_texture(&self) -> Option<Handle<Texture>> {
        None
    }
}

impl<T> GetTexture for T
where
    T: GetBytes,
{
    default fn get_texture(&self) -> Option<Handle<Texture>> {
        None
    }
}

impl GetTexture for Handle<Texture> {
    fn get_texture(&self) -> Option<Handle<Texture>> {
        Some(self.clone())
    }
}

impl GetTexture for Option<Handle<Texture>> {
    fn get_texture(&self) -> Option<Handle<Texture>> {
        *self
    }
}

impl GetTexture for ColorSource {
    fn get_texture(&self) -> Option<Handle<Texture>> {
        match self {
            ColorSource::Color(_) => None,
            ColorSource::Texture(texture) => Some(texture.clone()),
        }
    }
}

pub struct UniformInfo<'a> {
    pub name: &'a str,
    pub bind_type: BindType,
}

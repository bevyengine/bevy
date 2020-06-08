use crate::{pipeline::BindType, texture::Texture};
use bevy_asset::Handle;

pub use bevy_derive::{Uniform, Uniforms};

pub trait Uniforms: Send + Sync + 'static {
    fn get_field_infos() -> &'static [FieldInfo];
    fn write_uniform_bytes(&self, name: &str, buffer: &mut [u8]);
    fn uniform_byte_len(&self, name: &str) -> usize;
    fn get_uniform_texture(&self, name: &str) -> Option<Handle<Texture>>;
    fn get_shader_defs(&self) -> Option<Vec<String>>;
    fn get_field_bind_type(&self, name: &str) -> Option<FieldBindType>;
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

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FieldBindType {
    Uniform { size: usize },
    Buffer { size: usize },
    Texture,
}

pub struct FieldInfo {
    pub name: &'static str,
    pub uniform_name: &'static str,
    pub texture_name: &'static str,
    pub sampler_name: &'static str,
}

pub struct UniformInfo<'a> {
    pub name: &'a str,
    pub bind_type: BindType,
}

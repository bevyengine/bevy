use crate::{
    asset::Handle,
    core::GetBytes,
    render::{
        color::ColorSource,
        pipeline::BindType,
        texture::{Texture, TextureViewDimension},
    },
};
use legion::prelude::Entity;
use std::collections::HashMap;

pub trait AsUniforms {
    fn get_field_infos(&self) -> &[FieldInfo];
    fn get_uniform_bytes(&self, name: &str) -> Option<Vec<u8>>;
    fn get_uniform_texture(&self, name: &str) -> Option<Handle<Texture>>;
    fn get_shader_defs(&self) -> Option<Vec<String>>;
    fn get_field_bind_type(&self, name: &str) -> Option<FieldBindType>;
    fn get_uniform_bytes_ref(&self, name: &str) -> Option<&[u8]>;
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
    Uniform,
    Texture,
}

pub struct UniformInfoIter<'a, 'b, T: AsUniforms> {
    pub field_infos: &'a [FieldInfo],
    pub uniforms: &'b T,
    pub index: usize,
    pub add_sampler: bool,
}

impl<'a, 'b, T> UniformInfoIter<'a, 'b, T>
where
    T: AsUniforms,
{
    pub fn new(field_infos: &'a [FieldInfo], uniforms: &'b T) -> Self {
        UniformInfoIter {
            field_infos,
            uniforms,
            index: 0,
            add_sampler: false,
        }
    }
}

impl<'a, 'b, T> Iterator for UniformInfoIter<'a, 'b, T>
where
    T: AsUniforms,
{
    type Item = UniformInfo<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.add_sampler {
            self.add_sampler = false;
            Some(UniformInfo {
                name: self.field_infos[self.index - 1].sampler_name,
                bind_type: BindType::Sampler,
            })
        } else {
            if self.index == self.field_infos.len() {
                None
            } else {
                let index = self.index;
                self.index += 1;
                let ref field_info = self.field_infos[index];
                let bind_type = self
                    .uniforms
                    .get_field_bind_type(field_info.name)
                    .unwrap();
                Some(match bind_type {
                    FieldBindType::Uniform => UniformInfo {
                        bind_type: BindType::Uniform {
                            dynamic: false,
                            properties: Vec::new(),
                        },
                        name: field_info.uniform_name,
                    },
                    FieldBindType::Texture => {
                        self.add_sampler = true;
                        UniformInfo {
                            bind_type: BindType::SampledTexture {
                                dimension: TextureViewDimension::D2,
                                multisampled: false,
                            },
                            name: field_info.texture_name,
                        }
                    }
                })
            }
        }
    }
}

pub struct FieldInfo {
    pub name: &'static str,
    pub uniform_name: &'static str,
    pub texture_name: &'static str,
    pub sampler_name: &'static str,
    pub is_vertex_buffer_member: bool, 
}

pub trait AsFieldBindType {
    fn get_field_bind_type(&self) -> FieldBindType;
}

impl AsFieldBindType for ColorSource {
    fn get_field_bind_type(&self) -> FieldBindType {
        match *self {
            ColorSource::Texture(_) => FieldBindType::Texture,
            ColorSource::Color(_) => FieldBindType::Uniform,
        }
    }
}

impl<T> AsFieldBindType for T
where
    T: GetBytes,
{
    default fn get_field_bind_type(&self) -> FieldBindType {
        FieldBindType::Uniform
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

pub struct DynamicUniformBufferInfo {
    pub offsets: HashMap<Entity, u32>,
    pub capacity: u64,
    pub count: u64,
}

impl DynamicUniformBufferInfo {
    pub fn new() -> Self {
        DynamicUniformBufferInfo {
            capacity: 0,
            count: 0,
            offsets: HashMap::new(),
        }
    }
}

use crate::{
    math::Vec4,
    render::render_graph::{BindType, UniformPropertyType},
};
use legion::prelude::Entity;
use std::collections::HashMap;
use zerocopy::AsBytes;

pub trait GetBytes {
    fn get_bytes(&self) -> Vec<u8>;
    fn get_bytes_ref(&self) -> Option<&[u8]>;
}

// TODO: might need to add zerocopy to this crate to impl AsBytes for external crates
// impl<T> GetBytes for T where T : AsBytes {
//     fn get_bytes(&self) -> Vec<u8> {
//         self.as_bytes().into()
//     }

//     fn get_bytes_ref(&self) -> Option<&[u8]> {
//         Some(self.as_bytes())
//     }
// }

impl GetBytes for Vec4 {
    fn get_bytes(&self) -> Vec<u8> {
        let vec4_array: [f32; 4] = (*self).into();
        vec4_array.as_bytes().into()
    }

    fn get_bytes_ref(&self) -> Option<&[u8]> {
        None
    }
}

// TODO: add ability to specify specific pipeline for uniforms
pub trait AsUniforms {
    fn get_uniform_infos(&self) -> &[UniformInfo];
    fn get_uniform_info(&self, name: &str) -> Option<&UniformInfo>;
    fn get_uniform_layouts(&self) -> &[&[UniformPropertyType]];
    fn get_uniform_bytes(&self, name: &str) -> Option<Vec<u8>>;
    fn get_shader_defs(&self) -> Option<Vec<String>>;
    // TODO: support zero-copy uniforms
    // fn get_uniform_bytes_ref(&self, name: &str) -> Option<&[u8]>;
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

// pub struct UniformInfo<'a> {
//   pub name: &'a str,
//   pub
// }

pub struct UniformInfo<'a> {
    pub name: &'a str,
    pub bind_type: BindType,
}

pub struct DynamicUniformBufferInfo {
    pub indices: HashMap<usize, Entity>,
    pub offsets: HashMap<Entity, u64>,
    pub capacity: u64,
    pub count: u64,
    pub size: u64,
}

impl DynamicUniformBufferInfo {
    pub fn new() -> Self {
        DynamicUniformBufferInfo {
            capacity: 0,
            count: 0,
            indices: HashMap::new(),
            offsets: HashMap::new(),
            size: 0,
        }
    }
}

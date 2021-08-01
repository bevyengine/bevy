use super::VertexFormat;
use bevy_reflect::{Reflect, ReflectDeserialize};
use serde::{Deserialize, Serialize};
use std::{
    borrow::Cow,
    hash::{Hash, Hasher},
};

#[derive(Clone, Debug, Eq, PartialEq, Default, Reflect, Serialize, Deserialize)]
#[reflect_value(Serialize, Deserialize, PartialEq)]
pub struct VertexBufferLayout {
    pub name: Cow<'static, str>,
    pub stride: u64,
    pub step_mode: InputStepMode,
    pub attributes: Vec<VertexAttribute>,
}

impl VertexBufferLayout {
    pub fn new_from_attribute(
        attribute: VertexAttribute,
        step_mode: InputStepMode,
    ) -> VertexBufferLayout {
        VertexBufferLayout {
            name: attribute.name.clone(),
            stride: attribute.format.get_size(),
            step_mode,
            attributes: vec![attribute],
        }
    }
}
#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub enum InputStepMode {
    Vertex = 0,
    Instance = 1,
}

impl Default for InputStepMode {
    fn default() -> Self {
        InputStepMode::Vertex
    }
}

#[derive(Clone, Debug, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub struct VertexAttribute {
    pub name: Cow<'static, str>,
    pub format: VertexFormat,
    pub offset: u64,
    pub shader_location: u32,
}

/// Internally, `bevy_render` uses hashes to identify vertex attribute names.
pub fn get_vertex_attribute_name_id(name: &str) -> u64 {
    let mut hasher = bevy_utils::AHasher::default();
    hasher.write(name.as_bytes());
    hasher.finish()
}

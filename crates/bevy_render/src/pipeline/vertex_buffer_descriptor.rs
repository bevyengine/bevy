use super::VertexFormat;
use bevy_utils::HashMap;
use std::borrow::Cow;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VertexBufferDescriptor {
    pub name: Cow<'static, str>,
    pub stride: u64,
    pub step_mode: InputStepMode,
    pub attribute: VertexAttributeDescriptor,
}

impl VertexBufferDescriptor {
    pub fn new_from_attribute(
        attribute: VertexAttributeDescriptor,
        step_mode: InputStepMode,
    ) -> VertexBufferDescriptor {
        VertexBufferDescriptor {
            name: attribute.name.clone(),
            stride: attribute.format.get_size(),
            step_mode,
            attribute: attribute.clone(),
        }
    }

    // just for tests, since a reflected layout doesn't know about the stride
    pub fn test_zero_stride(mut self) -> VertexBufferDescriptor {
        self.stride = 0;
        self
    }
}
#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq)]
pub enum InputStepMode {
    Vertex = 0,
    Instance = 1,
}

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub struct VertexAttributeDescriptor {
    pub name: Cow<'static, str>,
    pub offset: u64,
    pub format: VertexFormat,
    pub shader_location: u32,
}

#[derive(Default)]
pub struct VertexBufferDescriptors {
    pub descriptors: HashMap<String, VertexBufferDescriptor>,
}

impl VertexBufferDescriptors {
    pub fn set_many(&mut self, vertex_buffer_descriptor: VertexBufferDescriptors) {
        self.descriptors
            .extend(vertex_buffer_descriptor.descriptors);
    }

    pub fn get(&self, name: &str) -> Option<&VertexBufferDescriptor> {
        self.descriptors.get(name)
    }
}

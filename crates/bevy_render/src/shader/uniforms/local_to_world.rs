use crate::{
    pipeline::{InputStepMode, VertexAttributeDescriptor, VertexBufferDescriptor, VertexFormat},
    shader::{Uniforms, FieldBindType, FieldInfo, GetFieldBindType},
    texture::Texture,
};
use bevy_asset::Handle;
use bevy_core::bytes::Bytes;
use once_cell::sync::Lazy;

static LOCAL_TO_WORLD_FIELD_INFOS: &[FieldInfo] = &[FieldInfo {
    name: "object",
    uniform_name: "Object",
    texture_name: "",
    sampler_name: "",
    is_instanceable: true,
}];

static VERTEX_BUFFER_DESCRIPTOR: Lazy<VertexBufferDescriptor> =
    Lazy::new(|| VertexBufferDescriptor {
        attributes: vec![
            VertexAttributeDescriptor {
                name: "I_Object_Model_0".into(),
                format: VertexFormat::Float4,
                offset: 0,
                shader_location: 0,
            },
            VertexAttributeDescriptor {
                name: "I_Object_Model_1".into(),
                format: VertexFormat::Float4,
                offset: 16,
                shader_location: 1,
            },
            VertexAttributeDescriptor {
                name: "I_Object_Model_2".into(),
                format: VertexFormat::Float4,
                offset: 32,
                shader_location: 2,
            },
            VertexAttributeDescriptor {
                name: "I_Object_Model_3".into(),
                format: VertexFormat::Float4,
                offset: 48,
                shader_location: 3,
            },
        ],
        name: "Object".into(),
        step_mode: InputStepMode::Instance,
        stride: 64,
    });

impl Uniforms for bevy_transform::prelude::LocalToWorld {
    fn get_field_infos() -> &'static [FieldInfo] {
        LOCAL_TO_WORLD_FIELD_INFOS
    }

    fn get_shader_defs(&self) -> Option<Vec<String>> {
        None
    }
    fn get_field_bind_type(&self, name: &str) -> Option<FieldBindType> {
        match name {
            "object" => self.value.get_bind_type(),
            _ => None,
        }
    }
    fn get_uniform_texture(&self, _name: &str) -> Option<Handle<Texture>> {
        None
    }

    fn get_vertex_buffer_descriptor() -> Option<&'static VertexBufferDescriptor> {
        Some(&VERTEX_BUFFER_DESCRIPTOR)
    }
    fn write_uniform_bytes(&self, name: &str, buffer: &mut [u8]) {
        match name {
            "Object" => self.value.write_bytes(buffer),
            _ => {}
        }
    }
    fn uniform_byte_len(&self, name: &str) -> usize {
        match name {
            "Object" => self.value.byte_len(),
            _ => 0,
        }
    }
}

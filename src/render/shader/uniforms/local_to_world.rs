use crate::{
    asset::Handle,
    render::{
        pipeline::{
            InputStepMode, VertexAttributeDescriptor, VertexBufferDescriptor, VertexFormat,
        },
        shader::{AsUniforms, FieldBindType, FieldInfo},
        texture::Texture,
    },
};
use once_cell::sync::Lazy;
use zerocopy::AsBytes;

static LOCAL_TO_WORLD_FIELD_INFOS: &[FieldInfo] = &[FieldInfo {
    name: "object",
    uniform_name: "Object",
    texture_name: "",
    sampler_name: "",
    is_instanceable: true,
}];

static VERTEX_BUFFER_DESCRIPTOR: Lazy<VertexBufferDescriptor> = Lazy::new(|| VertexBufferDescriptor {
    attributes: vec![
        VertexAttributeDescriptor {
            name: "I_Object_Matrix_0".to_string(),
            format: VertexFormat::Float4,
            offset: 0,
            shader_location: 0,
        },
        VertexAttributeDescriptor {
            name: "I_Object_Matrix_1".to_string(),
            format: VertexFormat::Float4,
            offset: 16,
            shader_location: 0,
        },
        VertexAttributeDescriptor {
            name: "I_Object_Matrix_2".to_string(),
            format: VertexFormat::Float4,
            offset: 32,
            shader_location: 0,
        },
        VertexAttributeDescriptor {
            name: "I_Object_Matrix_3".to_string(),
            format: VertexFormat::Float4,
            offset: 48,
            shader_location: 0,
        },
    ],
    name: "Object".to_string(),
    step_mode: InputStepMode::Instance,
    stride: 64,
});

impl AsUniforms for bevy_transform::prelude::LocalToWorld {
    fn get_field_infos(&self) -> &[FieldInfo] {
        LOCAL_TO_WORLD_FIELD_INFOS
    }

    fn get_uniform_bytes(&self, name: &str) -> Option<Vec<u8>> {
        match name {
            "Object" => Some(self.0.as_ref().as_bytes().into()),
            _ => None,
        }
    }

    fn get_shader_defs(&self) -> Option<Vec<String>> {
        None
    }
    fn get_field_bind_type(&self, name: &str) -> Option<FieldBindType> {
        match name {
            "object" => Some(FieldBindType::Uniform),
            _ => None,
        }
    }
    fn get_uniform_texture(&self, _name: &str) -> Option<Handle<Texture>> {
        None
    }

    fn get_uniform_bytes_ref(&self, name: &str) -> Option<&[u8]> {
        match name {
            "Object" => Some(self.0.as_ref().as_bytes()),
            _ => None,
        }
    }

    fn get_vertex_buffer_descriptor() -> Option<&'static VertexBufferDescriptor> {
        Some(&VERTEX_BUFFER_DESCRIPTOR)
    }
}

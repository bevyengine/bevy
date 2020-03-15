use crate::{
    asset::Handle,
    render::{
        shader::{AsUniforms, FieldBindType, FieldInfo},
        texture::Texture,
    },
};

use zerocopy::AsBytes;

const LOCAL_TO_WORLD_FIELD_INFOS: &[FieldInfo] = &[FieldInfo {
    name: "object",
    uniform_name: "Object",
    texture_name: "",
    sampler_name: "",
    is_vertex_buffer_member: false,
}];

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
}

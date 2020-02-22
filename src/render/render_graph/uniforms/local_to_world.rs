use crate::render::render_graph::{uniform::AsUniforms, FieldBindType, FieldUniformName};

use zerocopy::AsBytes;

const LOCAL_TO_WORLD_FIELD_UNIFORM_NAMES: &[FieldUniformName] = &[FieldUniformName {
    field: "object",
    uniform: "Object",
    texture: "",
    sampler: "",
}];

impl AsUniforms for bevy_transform::prelude::LocalToWorld {
    fn get_field_uniform_names(&self) -> &[FieldUniformName] {
        LOCAL_TO_WORLD_FIELD_UNIFORM_NAMES
    }

    fn get_uniform_bytes(&self, name: &str) -> Option<Vec<u8>> {
        match name {
            "Object" => Some(self.0.to_cols_array_2d().as_bytes().into()),
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
}

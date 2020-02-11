use crate::{
    render::render_graph_2::{
        uniform::{AsUniforms, UniformInfo},
        BindType, UniformPropertyType,
    },
};

use zerocopy::AsBytes;

const LOCAL_TO_WORLD_UNIFORM_INFO: &[UniformInfo] = &[UniformInfo {
    name: "Object",
    bind_type: BindType::Uniform {
        dynamic: false,
        // TODO: maybe fill this in with properties (vec.push cant be const though)
        properties: Vec::new(),
    },
}];

// these are separate from BindType::Uniform{properties} because they need to be const
const LOCAL_TO_WORLD_UNIFORM_LAYOUTS: &[&[UniformPropertyType]] = &[&[]];

impl AsUniforms for bevy_transform::prelude::LocalToWorld {
    fn get_uniform_infos(&self) -> &[UniformInfo] {
        LOCAL_TO_WORLD_UNIFORM_INFO
    }

    fn get_uniform_layouts(&self) -> &[&[UniformPropertyType]] {
        LOCAL_TO_WORLD_UNIFORM_LAYOUTS
    }

    fn get_uniform_bytes(&self, name: &str) -> Option<Vec<u8>> {
        match name {
            "Object" => Some(self.0.to_cols_array_2d().as_bytes().into()),
            _ => None,
        }
    }
    fn get_uniform_info(&self, name: &str) -> Option<&UniformInfo> {
        match name {
            "Object" => Some(&LOCAL_TO_WORLD_UNIFORM_INFO[0]),
            _ => None,
        }
    }

    fn get_shader_defs(&self) -> Option<Vec<String>> {
        None
    }
}
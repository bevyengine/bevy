use crate::{
    math::Vec4,
    render::render_graph_2::{
        uniform::{AsUniforms, GetBytes, UniformInfo},
        BindType, UniformPropertyType,
    },
};

use zerocopy::AsBytes;

pub struct StandardMaterial {
    pub albedo: Vec4,
}

// create this from a derive macro
const STANDARD_MATERIAL_UNIFORM_INFO: &[UniformInfo] = &[UniformInfo {
    name: "StandardMaterial",
    bind_type: BindType::Uniform {
        dynamic: false,
        // TODO: fill this in with properties
        properties: Vec::new(),
    },
}];

// these are separate from BindType::Uniform{properties} because they need to be const
const STANDARD_MATERIAL_UNIFORM_LAYOUTS: &[&[UniformPropertyType]] = &[&[]];

// const
impl AsUniforms for StandardMaterial {
    fn get_uniform_infos(&self) -> &[UniformInfo] {
        STANDARD_MATERIAL_UNIFORM_INFO
    }

    fn get_uniform_layouts(&self) -> &[&[UniformPropertyType]] {
        STANDARD_MATERIAL_UNIFORM_LAYOUTS
    }

    fn get_uniform_bytes(&self, name: &str) -> Option<Vec<u8>> {
        match name {
            "StandardMaterial" => Some(self.albedo.get_bytes()),
            _ => None,
        }
    }
    fn get_uniform_info(&self, name: &str) -> Option<&UniformInfo> {
        match name {
            "StandardMaterial" => Some(&STANDARD_MATERIAL_UNIFORM_INFO[0]),
            _ => None,
        }
    }

    // fn iter_properties(&self) -> std::slice::Iter<&'static str>  {
    //   STANDARD_MATERIAL_PROPERTIES.iter()
    // }
    // fn get_property(&self, name: &str) -> Option<ShaderValue> {
    //   match name {
    //     "albedo" => Some(match self.albedo {
    //       Albedo::Color(color) => ShaderValue::Vec4(color),
    //       Albedo::Texture(ref texture) => ShaderValue::Texture(texture)
    //     }),
    //     _ => None,
    //   }
    // }
}

// create this from a derive macro
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

// const ST
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
    // fn iter_properties(&self) -> std::slice::Iter<&'static str>  {
    //   STANDARD_MATERIAL_PROPERTIES.iter()
    // }
    // fn get_property(&self, name: &str) -> Option<ShaderValue> {
    //   match name {
    //     "albedo" => Some(match self.albedo {
    //       Albedo::Color(color) => ShaderValue::Vec4(color),
    //       Albedo::Texture(ref texture) => ShaderValue::Texture(texture)
    //     }),
    //     _ => None,
    //   }
    // }
}

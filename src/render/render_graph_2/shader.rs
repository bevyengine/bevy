use crate::{
    legion::{
        borrow::RefMap,
        prelude::{Entity, World},
    },
    math::Vec4,
    render::render_graph_2::{BindType, UniformPropertyType},
};
use std::collections::HashMap;
use legion::storage::Component;
use zerocopy::AsBytes;

pub type ShaderUniformSelector = fn(Entity, &World) -> Option<RefMap<&dyn AsUniforms>>;
pub struct ShaderUniforms {
    // used for distinguishing
    pub uniform_selectors: Vec<ShaderUniformSelector>,
    pub dynamic_uniform_indices: HashMap<String, u64>,
}

impl ShaderUniforms {
    pub fn new() -> Self {
        ShaderUniforms {
            uniform_selectors: Vec::new(),
            dynamic_uniform_indices: HashMap::new(),
        }
    }

    pub fn add(&mut self, selector: ShaderUniformSelector) {
        self.uniform_selectors.push(selector);
    }

    pub fn get_uniform_info<'a>(
        &'a self,
        world: &'a World,
        entity: Entity,
        uniform_name: &str,
    ) -> Option<&'a UniformInfo> {
        for uniform_selector in self.uniform_selectors.iter().rev() {
            let uniforms = uniform_selector(entity, world).unwrap_or_else(|| {
                panic!(
                    "ShaderUniform selector points to a missing component. Uniform: {}",
                    uniform_name
                )
            });

            let info = uniforms.get_uniform_info(uniform_name);
            if let Some(_) = info {
              return info;
            }
        }

        None
    }

    pub fn get_uniform_bytes<'a>(
        &'a self,
        world: &'a World,
        entity: Entity,
        uniform_name: &str,
    ) -> Option<Vec<u8>> {
        for uniform_selector in self.uniform_selectors.iter().rev() {
            let uniforms = uniform_selector(entity, world).unwrap_or_else(|| {
                panic!(
                    "ShaderUniform selector points to a missing component. Uniform: {}",
                    uniform_name
                )
            });

            let bytes = uniforms.get_uniform_bytes(uniform_name);
            if let Some(_) = bytes {
              return bytes;
            }
        }

        None
    }
}

pub struct StandardMaterial {
    pub albedo: Vec4,
}

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

pub trait AsUniforms {
    fn get_uniform_infos(&self) -> &[UniformInfo];
    fn get_uniform_info(&self, name: &str) -> Option<&UniformInfo>;
    fn get_uniform_layouts(&self) -> &[&[UniformPropertyType]];
    fn get_uniform_bytes(&self, name: &str) -> Option<Vec<u8>>;
    // TODO: support zero-copy uniforms
    // fn get_uniform_bytes_ref(&self, name: &str) -> Option<&[u8]>;
}

// pub struct UniformInfo<'a> {
//   pub name: &'a str,
//   pub
// }

pub struct UniformInfo<'a> {
    pub name: &'a str,
    pub bind_type: BindType,
}

pub fn uniform_selector<T>(entity: Entity, world: &World) -> Option<RefMap<&dyn AsUniforms>>
where
    T: AsUniforms + Component,
{
    world
        .get_component::<T>(entity)
        .map(|c| c.map_into(|s| s as &dyn AsUniforms))
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

// const ST
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
        // TODO: fill this in with properties
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

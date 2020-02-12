use crate::{
    legion::{
        borrow::RefMap,
        prelude::{Entity, World},
    },
    math::Vec4,
    render::render_graph_2::{BindType, UniformPropertyType},
};
use legion::storage::Component;
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

pub fn uniform_selector<T>(entity: Entity, world: &World) -> Option<RefMap<&dyn AsUniforms>>
where
    T: AsUniforms + Component,
{
    world
        .get_component::<T>(entity)
        .map(|c| c.map_into(|s| s as &dyn AsUniforms))
}

// TODO: Remove these

pub type ShaderUniformSelector = fn(Entity, &World) -> Option<RefMap<&dyn AsUniforms>>;

#[derive(Default)]
pub struct ShaderUniforms {
    // used for distinguishing
    pub uniform_selectors: Vec<ShaderUniformSelector>,
}

impl ShaderUniforms {
    pub fn new() -> Self {
        ShaderUniforms {
            uniform_selectors: Vec::new(),
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

use crate::{
    legion::{
        borrow::RefMap,
        prelude::{Entity, World},
    },
    math::Vec4,
    render::render_graph_2::{
        wgpu_renderer::DynamicUniformBufferInfo, BindType, ResourceProvider, UniformPropertyType,
    },
};
use legion::{prelude::*, storage::Component};
use zerocopy::AsBytes;
use std::marker::PhantomData;

pub type ShaderUniformSelector = fn(Entity, &World) -> Option<RefMap<&dyn AsUniforms>>;
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

pub struct UniformResourceProvider<T>
where
    T: AsUniforms + Send + Sync,
{
    _marker: PhantomData<T>,
    uniform_buffer_info_names: Vec<String>,
    // dynamic_uniform_buffer_infos: HashMap<String, DynamicUniformBufferInfo>,
}

impl<T> UniformResourceProvider<T>
where
    T: AsUniforms + Send + Sync,
{
    pub fn new() -> Self {
        UniformResourceProvider {
            // dynamic_uniform_buffer_infos: HashMap::new(),
            uniform_buffer_info_names: Vec::new(),
            _marker: PhantomData,
        }
    }
}

impl<T> ResourceProvider for UniformResourceProvider<T>
where
    T: AsUniforms + Send + Sync + 'static,
{
    fn initialize(&mut self, renderer: &mut dyn super::Renderer, world: &mut World) {}

    fn update(&mut self, renderer: &mut dyn super::Renderer, world: &mut World) {
        let query = <Read<T>>::query();
        // retrieve all uniforms buffers that aren't aleady set. these are "dynamic" uniforms, which are set by the user in ShaderUniforms
        // TODO: this breaks down in multiple ways:
        // (SOLVED 1) resource_info will be set after the first run so this won't update.
        // (2) if we create new buffers, the old bind groups will be invalid

        // reset all uniform buffer info counts
        for name in self.uniform_buffer_info_names.iter() {
            renderer
                .get_dynamic_uniform_buffer_info_mut(name)
                .unwrap()
                .count = 0;
        }

        let mut sizes = Vec::new();
        let mut counts = Vec::new();
        for uniforms in query.iter(world) {
            let uniform_layouts = uniforms.get_uniform_layouts();
            for (i, uniform_info) in uniforms.get_uniform_infos().iter().enumerate() {
                // only add the first time a uniform info is processed
                if self.uniform_buffer_info_names.len() <= i {
                    let uniform_layout = uniform_layouts[i];
                    let size = uniform_layout
                        .iter()
                        .map(|u| u.get_size())
                        .fold(0, |total, current| total + current);
                    sizes.push(size);

                    self.uniform_buffer_info_names
                        .push(uniform_info.name.to_string());
                }

                if counts.len() <= i {
                    counts.push(0);
                }

                counts[i] += 1;
            }
        }

        // create and update uniform buffer info. this is separate from the last block to avoid
        // the expense of hashing for large numbers of entities
        for (i, name) in self.uniform_buffer_info_names.iter().enumerate() {
            if let None = renderer.get_dynamic_uniform_buffer_info(name) {
                let mut info = DynamicUniformBufferInfo::new();
                info.size = sizes[i];
                renderer.add_dynamic_uniform_buffer_info(name, info);
            }

            let info = renderer.get_dynamic_uniform_buffer_info_mut(name).unwrap();
            info.count = counts[i];
        }

        // allocate uniform buffers
        for name in self.uniform_buffer_info_names.iter() {
            if let Some(_) = renderer.get_resource_info(name) {
                continue;
            }

            let info = renderer.get_dynamic_uniform_buffer_info_mut(name).unwrap();

            // allocate enough space for twice as many entities as there are currently;
            info.capacity = info.count * 2;
            let size = wgpu::BIND_BUFFER_ALIGNMENT * info.capacity;
            renderer.create_buffer(
                name,
                size,
                wgpu::BufferUsage::COPY_DST | wgpu::BufferUsage::UNIFORM,
            );
        }

        // copy entity uniform data to buffers
        for name in self.uniform_buffer_info_names.iter() {
            let size = {
                let info = renderer.get_dynamic_uniform_buffer_info(name).unwrap();
                wgpu::BIND_BUFFER_ALIGNMENT * info.count
            };

            let alignment = wgpu::BIND_BUFFER_ALIGNMENT as usize;
            let mut offset = 0usize;
            let info = renderer.get_dynamic_uniform_buffer_info_mut(name).unwrap();
            for (i, (entity, _uniforms)) in query.iter_entities(world).enumerate() {
                // TODO: check if index has changed. if it has, then entity should be updated
                // TODO: only mem-map entities if their data has changed
                // PERF: These hashmap inserts are pretty expensive (10 fps for 10000 entities)
                info.offsets.insert(entity, offset as u64);
                info.indices.insert(i, entity);
                // TODO: try getting ref first
                offset += alignment;
            }

            // let mut data = vec![Default::default(); size as usize];
            renderer.create_buffer_mapped(
                "tmp_uniform_mapped",
                size as usize,
                wgpu::BufferUsage::COPY_SRC,
                &mut |mapped| {
                    let alignment = wgpu::BIND_BUFFER_ALIGNMENT as usize;
                    let mut offset = 0usize;
                    for uniforms in query.iter(world) {
                        // TODO: check if index has changed. if it has, then entity should be updated
                        // TODO: only mem-map entities if their data has changed
                        // TODO: try getting ref first
                        if let Some(uniform_bytes) = uniforms.get_uniform_bytes(name) {
                            mapped[offset..(offset + uniform_bytes.len())]
                                .copy_from_slice(uniform_bytes.as_slice());
                            offset += alignment;
                        }
                    }
                },
            );

            renderer.copy_buffer_to_buffer("tmp_uniform_mapped", 0, name, 0, size);
        }
    }

    fn resize(
        &mut self,
        renderer: &mut dyn super::Renderer,
        _world: &mut World,
        _width: u32,
        _height: u32,
    ) {
    }
}

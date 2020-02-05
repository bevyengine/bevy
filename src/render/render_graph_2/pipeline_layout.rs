use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
};
pub struct PipelineLayout {
    pub bind_groups: Vec<BindGroup>,
}

impl PipelineLayout {
    pub fn new() -> Self {
        PipelineLayout {
            bind_groups: Vec::new(),
        }
    }
}

#[derive(Hash)]
pub struct BindGroup {
    pub bindings: Vec<Binding>,
    hash: Option<u64>,
}

impl BindGroup {
    pub fn new(bindings: Vec<Binding>) -> Self {
        BindGroup {
            bindings,
            hash: None,
        }
    }

    pub fn get_hash(&self) -> u64 {
        self.hash.unwrap()
    }

    pub fn update_hash(&mut self) {
        let mut hasher = DefaultHasher::new();
        self.hash(&mut hasher);
        self.hash = Some(hasher.finish());
    }
}

#[derive(Hash)]
pub struct Binding {
    pub name: String,
    pub bind_type: BindType,
    // TODO: ADD SHADER STAGE VISIBILITY
}

#[derive(Hash)]
pub enum BindType {
    Uniform {
        dynamic: bool,
        properties: Vec<UniformProperty>,
    },
    Buffer {
        dynamic: bool,
        readonly: bool,
    },
    Sampler,
    SampledTexture {
        multisampled: bool,
        dimension: TextureDimension,
    },
    StorageTexture {
        dimension: TextureDimension,
    },
}

impl BindType {
    pub fn get_uniform_size(&self) -> Option<u64> {
        match self {
            BindType::Uniform { properties, .. } => {
                Some(properties.iter().fold(0, |total, property| {
                    total + property.property_type.get_size()
                }))
            }
            _ => None,
        }
    }
}

#[derive(Hash)]
pub struct UniformProperty {
    pub name: String,
    pub property_type: UniformPropertyType,
}

#[derive(Hash)]
pub enum UniformPropertyType {
    // TODO: Add all types here
    Int,
    Float,
    UVec4,
    Vec3,
    Vec4,
    Mat4,
    Struct(Vec<UniformPropertyType>),
    Array(Box<UniformPropertyType>, usize),
}

impl UniformPropertyType {
    pub fn get_size(&self) -> u64 {
        match self {
            UniformPropertyType::Int => 4,
            UniformPropertyType::Float => 4,
            UniformPropertyType::UVec4 => 4 * 4,
            UniformPropertyType::Vec3 => 4 * 3,
            UniformPropertyType::Vec4 => 4 * 4,
            UniformPropertyType::Mat4 => 4 * 4 * 4,
            UniformPropertyType::Struct(properties) => properties
                .iter()
                .map(|p| p.get_size())
                .fold(0, |total, size| total + size),
            UniformPropertyType::Array(property, length) => property.get_size() * *length as u64,
        }
    }
}

#[derive(Copy, Clone, Hash)]
pub enum TextureDimension {
    D1,
    D2,
    D2Array,
    Cube,
    CubeArray,
    D3,
}

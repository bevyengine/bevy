use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
};

#[derive(Clone, Debug)]
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

#[derive(Hash, Clone, Debug)]
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

#[derive(Hash, Clone, Debug)]
pub struct Binding {
    pub name: String,
    pub bind_type: BindType,
    // TODO: ADD SHADER STAGE VISIBILITY
}

#[derive(Hash, Clone, Debug)]
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
        dimension: TextureViewDimension,
    },
    StorageTexture {
        dimension: TextureViewDimension,
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

#[derive(Hash, Clone, Debug)]
pub struct UniformProperty {
    pub name: String,
    pub property_type: UniformPropertyType,
}

#[derive(Hash, Clone, Debug)]
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

#[derive(Copy, Clone, Debug, Hash)]
pub enum TextureViewDimension {
    D1,
    D2,
    D2Array,
    Cube,
    CubeArray,
    D3,
}

impl From<TextureViewDimension> for wgpu::TextureViewDimension {
    fn from(dimension: TextureViewDimension) -> Self {
        match dimension {
            TextureViewDimension::D1 => wgpu::TextureViewDimension::D1,
            TextureViewDimension::D2 => wgpu::TextureViewDimension::D2,
            TextureViewDimension::D2Array => wgpu::TextureViewDimension::D2Array,
            TextureViewDimension::Cube => wgpu::TextureViewDimension::Cube,
            TextureViewDimension::CubeArray => wgpu::TextureViewDimension::CubeArray,
            TextureViewDimension::D3 => wgpu::TextureViewDimension::D3,
        }
    }
}

#[derive(Copy, Clone, Debug, Hash)]
pub enum TextureDimension {
    D1,
    D2,
    D3,
}

impl From<TextureDimension> for wgpu::TextureDimension {
    fn from(dimension: TextureDimension) -> Self {
        match dimension {
            TextureDimension::D1 => wgpu::TextureDimension::D1,
            TextureDimension::D2 => wgpu::TextureDimension::D2,
            TextureDimension::D3 => wgpu::TextureDimension::D3,
        }
    }
}

#[derive(Copy, Clone)]
pub struct TextureDescriptor {
    pub size: wgpu::Extent3d,
    pub array_layer_count: u32,
    pub mip_level_count: u32,
    pub sample_count: u32,
    pub dimension: TextureDimension,
    pub format: wgpu::TextureFormat,
    pub usage: wgpu::TextureUsage,
}

impl From<TextureDescriptor> for wgpu::TextureDescriptor {
    fn from(texture_descriptor: TextureDescriptor) -> Self {
        wgpu::TextureDescriptor {
            size: texture_descriptor.size,
            array_layer_count: texture_descriptor.array_layer_count,
            mip_level_count: texture_descriptor.mip_level_count,
            sample_count: texture_descriptor.sample_count,
            dimension: texture_descriptor.dimension.into(),
            format: texture_descriptor.format,
            usage: texture_descriptor.usage,
        }
    }
}

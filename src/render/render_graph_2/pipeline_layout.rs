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

pub struct BindGroup {
    pub bindings: Vec<Binding>
}

pub struct Binding {
    pub name: String,
    pub bind_type: BindType,
    // TODO: ADD SHADER STAGE VISIBILITY
}

pub enum BindType {
    Uniform {
        dynamic: bool,
        properties: Vec<UniformProperty>
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

pub struct UniformProperty {
    pub name: String,
    pub property_type: UniformPropertyType,
}

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

#[derive(Copy, Clone)]
pub enum TextureDimension {
    D1,
    D2,
    D2Array,
    Cube,
    CubeArray,
    D3,
}

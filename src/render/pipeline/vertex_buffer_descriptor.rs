use super::VertexFormat;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VertexBufferDescriptor {
    pub name: String,
    pub stride: u64,
    pub step_mode: InputStepMode,
    pub attributes: Vec<VertexAttributeDescriptor>,
}

#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq)]
pub enum InputStepMode {
    Vertex = 0,
    Instance = 1,
}

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub struct VertexAttributeDescriptor {
    pub name: String,
    pub offset: u64,
    pub format: VertexFormat,
    pub shader_location: u32,
}
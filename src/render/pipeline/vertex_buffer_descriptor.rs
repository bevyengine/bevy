#[derive(Clone, Debug)]
pub struct VertexBufferDescriptor {
    pub stride: u64,
    pub step_mode: InputStepMode,
    pub attributes: Vec<VertexAttributeDescriptor>,
}

#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq)]
pub enum VertexFormat {
    Uchar2 = 1,
    Uchar4 = 3,
    Char2 = 5,
    Char4 = 7,
    Uchar2Norm = 9,
    Uchar4Norm = 11,
    Char2Norm = 14,
    Char4Norm = 16,
    Ushort2 = 18,
    Ushort4 = 20,
    Short2 = 22,
    Short4 = 24,
    Ushort2Norm = 26,
    Ushort4Norm = 28,
    Short2Norm = 30,
    Short4Norm = 32,
    Half2 = 34,
    Half4 = 36,
    Float = 37,
    Float2 = 38,
    Float3 = 39,
    Float4 = 40,
    Uint = 41,
    Uint2 = 42,
    Uint3 = 43,
    Uint4 = 44,
    Int = 45,
    Int2 = 46,
    Int3 = 47,
    Int4 = 48,
}

#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq)]
pub enum InputStepMode {
    Vertex = 0,
    Instance = 1,
}

#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq)]
pub struct VertexAttributeDescriptor {
    pub offset: u64,
    pub format: VertexFormat,
    pub shader_location: u32,
}

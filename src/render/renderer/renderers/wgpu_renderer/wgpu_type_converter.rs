use crate::{
    prelude::Color,
    render::{
        pass::{LoadOp, StoreOp},
        pipeline::{
            InputStepMode, VertexAttributeDescriptor, VertexBufferDescriptor, VertexFormat, BindType
        },
        render_resource::BufferUsage,
    },
};

impl From<VertexFormat> for wgpu::VertexFormat {
    fn from(val: VertexFormat) -> Self {
        match val {
            VertexFormat::Uchar2 => wgpu::VertexFormat::Uchar2,
            VertexFormat::Uchar4 => wgpu::VertexFormat::Uchar4,
            VertexFormat::Char2 => wgpu::VertexFormat::Char2,
            VertexFormat::Char4 => wgpu::VertexFormat::Char4,
            VertexFormat::Uchar2Norm => wgpu::VertexFormat::Uchar2Norm,
            VertexFormat::Uchar4Norm => wgpu::VertexFormat::Uchar4Norm,
            VertexFormat::Char2Norm => wgpu::VertexFormat::Char2Norm,
            VertexFormat::Char4Norm => wgpu::VertexFormat::Char4Norm,
            VertexFormat::Ushort2 => wgpu::VertexFormat::Ushort2,
            VertexFormat::Ushort4 => wgpu::VertexFormat::Ushort4,
            VertexFormat::Short2 => wgpu::VertexFormat::Short2,
            VertexFormat::Short4 => wgpu::VertexFormat::Short4,
            VertexFormat::Ushort2Norm => wgpu::VertexFormat::Ushort2Norm,
            VertexFormat::Ushort4Norm => wgpu::VertexFormat::Ushort4Norm,
            VertexFormat::Short2Norm => wgpu::VertexFormat::Short2Norm,
            VertexFormat::Short4Norm => wgpu::VertexFormat::Short4Norm,
            VertexFormat::Half2 => wgpu::VertexFormat::Half2,
            VertexFormat::Half4 => wgpu::VertexFormat::Half4,
            VertexFormat::Float => wgpu::VertexFormat::Float,
            VertexFormat::Float2 => wgpu::VertexFormat::Float2,
            VertexFormat::Float3 => wgpu::VertexFormat::Float3,
            VertexFormat::Float4 => wgpu::VertexFormat::Float4,
            VertexFormat::Uint => wgpu::VertexFormat::Uint,
            VertexFormat::Uint2 => wgpu::VertexFormat::Uint2,
            VertexFormat::Uint3 => wgpu::VertexFormat::Uint3,
            VertexFormat::Uint4 => wgpu::VertexFormat::Uint4,
            VertexFormat::Int => wgpu::VertexFormat::Int,
            VertexFormat::Int2 => wgpu::VertexFormat::Int2,
            VertexFormat::Int3 => wgpu::VertexFormat::Int3,
            VertexFormat::Int4 => wgpu::VertexFormat::Int4,
        }
    }
}

impl From<&VertexAttributeDescriptor> for wgpu::VertexAttributeDescriptor {
    fn from(val: &VertexAttributeDescriptor) -> Self {
        wgpu::VertexAttributeDescriptor {
            format: val.format.into(),
            offset: val.offset,
            shader_location: val.shader_location,
        }
    }
}

impl From<InputStepMode> for wgpu::InputStepMode {
    fn from(val: InputStepMode) -> Self {
        match val {
            InputStepMode::Vertex => wgpu::InputStepMode::Vertex,
            InputStepMode::Instance => wgpu::InputStepMode::Instance,
        }
    }
}

#[derive(Clone, Debug)]
pub struct OwnedWgpuVertexBufferDescriptor {
    pub stride: wgpu::BufferAddress,
    pub step_mode: wgpu::InputStepMode,
    pub attributes: Vec<wgpu::VertexAttributeDescriptor>,
}

impl From<&VertexBufferDescriptor> for OwnedWgpuVertexBufferDescriptor {
    fn from(val: &VertexBufferDescriptor) -> OwnedWgpuVertexBufferDescriptor {
        let attributes = val
            .attributes
            .iter()
            .map(|a| a.into())
            .collect::<Vec<wgpu::VertexAttributeDescriptor>>();
        OwnedWgpuVertexBufferDescriptor {
            step_mode: val.step_mode.into(),
            stride: val.stride,
            attributes,
        }
    }
}

impl<'a> From<&'a OwnedWgpuVertexBufferDescriptor> for wgpu::VertexBufferDescriptor<'a> {
    fn from(val: &'a OwnedWgpuVertexBufferDescriptor) -> Self {
        wgpu::VertexBufferDescriptor {
            attributes: &val.attributes,
            step_mode: val.step_mode,
            stride: val.stride,
        }
    }
}

impl From<Color> for wgpu::Color {
    fn from(color: Color) -> Self {
        wgpu::Color {
            r: color.r as f64,
            g: color.g as f64,
            b: color.b as f64,
            a: color.a as f64,
        }
    }
}

impl From<BufferUsage> for wgpu::BufferUsage {
    fn from(val: BufferUsage) -> Self {
        wgpu::BufferUsage::from_bits(val.bits()).unwrap()
    }
}

impl From<LoadOp> for wgpu::LoadOp {
    fn from(val: LoadOp) -> Self {
        match val {
            LoadOp::Clear => wgpu::LoadOp::Clear,
            LoadOp::Load => wgpu::LoadOp::Load,
        }
    }
}

impl From<StoreOp> for wgpu::StoreOp {
    fn from(val: StoreOp) -> Self {
        match val {
            StoreOp::Clear => wgpu::StoreOp::Clear,
            StoreOp::Store => wgpu::StoreOp::Store,
        }
    }
}

impl From<&BindType> for wgpu::BindingType {
    fn from(bind_type: &BindType) -> Self {
        match bind_type {
            BindType::Uniform {
                dynamic,
                properties: _,
            } => wgpu::BindingType::UniformBuffer { dynamic: *dynamic },
            BindType::Buffer { dynamic, readonly } => wgpu::BindingType::StorageBuffer {
                dynamic: *dynamic,
                readonly: *readonly,
            },
            BindType::SampledTexture {
                dimension,
                multisampled,
            } => wgpu::BindingType::SampledTexture {
                dimension: (*dimension).into(),
                multisampled: *multisampled,
            },
            BindType::Sampler => wgpu::BindingType::Sampler,
            BindType::StorageTexture { dimension } => wgpu::BindingType::StorageTexture {
                dimension: (*dimension).into(),
            },
        }
    }
}
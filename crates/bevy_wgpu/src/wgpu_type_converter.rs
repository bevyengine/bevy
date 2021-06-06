use crate::{WgpuFeature, WgpuFeatures, WgpuLimits};
use bevy_render::{
    color::Color,
    pass::{LoadOp, Operations},
    pipeline::{
        BindType, BlendFactor, BlendOperation, BlendState, ColorTargetState, ColorWrite,
        CompareFunction, DepthBiasState, DepthStencilState, Face, FrontFace, IndexFormat,
        InputStepMode, MultisampleState, PolygonMode, PrimitiveState, PrimitiveTopology,
        StencilFaceState, StencilOperation, StencilState, VertexAttribute, VertexBufferLayout,
        VertexFormat,
    },
    renderer::BufferUsage,
    texture::{
        AddressMode, Extent3d, FilterMode, SamplerBorderColor, SamplerDescriptor,
        StorageTextureAccess, TextureDescriptor, TextureDimension, TextureFormat,
        TextureSampleType, TextureUsage, TextureViewDimension,
    },
};
use bevy_window::Window;
use wgpu::BufferBindingType;

pub trait WgpuFrom<T> {
    fn from(val: T) -> Self;
}

pub trait WgpuInto<U> {
    fn wgpu_into(self) -> U;
}

impl<T, U> WgpuInto<U> for T
where
    U: WgpuFrom<T>,
{
    fn wgpu_into(self) -> U {
        U::from(self)
    }
}

impl WgpuFrom<VertexFormat> for wgpu::VertexFormat {
    fn from(val: VertexFormat) -> Self {
        match val {
            VertexFormat::Uint8x2 => wgpu::VertexFormat::Uint8x2,
            VertexFormat::Uint8x4 => wgpu::VertexFormat::Uint8x4,
            VertexFormat::Sint8x2 => wgpu::VertexFormat::Sint8x2,
            VertexFormat::Sint8x4 => wgpu::VertexFormat::Sint8x4,
            VertexFormat::Unorm8x2 => wgpu::VertexFormat::Unorm8x2,
            VertexFormat::Unorm8x4 => wgpu::VertexFormat::Unorm8x4,
            VertexFormat::Snorm8x2 => wgpu::VertexFormat::Snorm8x2,
            VertexFormat::Snorm8x4 => wgpu::VertexFormat::Snorm8x4,
            VertexFormat::Uint16x2 => wgpu::VertexFormat::Uint16x2,
            VertexFormat::Uint16x4 => wgpu::VertexFormat::Uint16x4,
            VertexFormat::Sint16x2 => wgpu::VertexFormat::Sint16x2,
            VertexFormat::Sint16x4 => wgpu::VertexFormat::Sint16x4,
            VertexFormat::Unorm16x2 => wgpu::VertexFormat::Unorm16x2,
            VertexFormat::Unorm16x4 => wgpu::VertexFormat::Unorm16x4,
            VertexFormat::Snorm16x2 => wgpu::VertexFormat::Snorm16x2,
            VertexFormat::Snorm16x4 => wgpu::VertexFormat::Snorm16x4,
            VertexFormat::Float16x2 => wgpu::VertexFormat::Float16x2,
            VertexFormat::Float16x4 => wgpu::VertexFormat::Float16x4,
            VertexFormat::Float32 => wgpu::VertexFormat::Float32,
            VertexFormat::Float32x2 => wgpu::VertexFormat::Float32x2,
            VertexFormat::Float32x3 => wgpu::VertexFormat::Float32x3,
            VertexFormat::Float32x4 => wgpu::VertexFormat::Float32x4,
            VertexFormat::Uint32 => wgpu::VertexFormat::Uint32,
            VertexFormat::Uint32x2 => wgpu::VertexFormat::Uint32x2,
            VertexFormat::Uint32x3 => wgpu::VertexFormat::Uint32x3,
            VertexFormat::Uint32x4 => wgpu::VertexFormat::Uint32x4,
            VertexFormat::Sint32 => wgpu::VertexFormat::Sint32,
            VertexFormat::Sint32x2 => wgpu::VertexFormat::Sint32x2,
            VertexFormat::Sint32x3 => wgpu::VertexFormat::Sint32x3,
            VertexFormat::Sint32x4 => wgpu::VertexFormat::Sint32x4,
        }
    }
}

impl WgpuFrom<&VertexAttribute> for wgpu::VertexAttribute {
    fn from(val: &VertexAttribute) -> Self {
        wgpu::VertexAttribute {
            format: val.format.wgpu_into(),
            offset: val.offset,
            shader_location: val.shader_location,
        }
    }
}

impl WgpuFrom<InputStepMode> for wgpu::InputStepMode {
    fn from(val: InputStepMode) -> Self {
        match val {
            InputStepMode::Vertex => wgpu::InputStepMode::Vertex,
            InputStepMode::Instance => wgpu::InputStepMode::Instance,
        }
    }
}

#[derive(Clone, Debug)]
pub struct OwnedWgpuVertexBufferLayout {
    pub array_stride: wgpu::BufferAddress,
    pub step_mode: wgpu::InputStepMode,
    pub attributes: Vec<wgpu::VertexAttribute>,
}

impl WgpuFrom<&VertexBufferLayout> for OwnedWgpuVertexBufferLayout {
    fn from(val: &VertexBufferLayout) -> OwnedWgpuVertexBufferLayout {
        let attributes = val
            .attributes
            .iter()
            .map(|a| a.wgpu_into())
            .collect::<Vec<wgpu::VertexAttribute>>();

        OwnedWgpuVertexBufferLayout {
            step_mode: val.step_mode.wgpu_into(),
            array_stride: val.stride,
            attributes,
        }
    }
}

impl<'a> From<&'a OwnedWgpuVertexBufferLayout> for wgpu::VertexBufferLayout<'a> {
    fn from(val: &'a OwnedWgpuVertexBufferLayout) -> Self {
        wgpu::VertexBufferLayout {
            attributes: &val.attributes,
            step_mode: val.step_mode,
            array_stride: val.array_stride,
        }
    }
}

impl WgpuFrom<Color> for wgpu::Color {
    fn from(color: Color) -> Self {
        let linear = color.as_linear_rgba_f32();
        wgpu::Color {
            r: linear[0] as f64,
            g: linear[1] as f64,
            b: linear[2] as f64,
            a: linear[3] as f64,
        }
    }
}

impl WgpuFrom<BufferUsage> for wgpu::BufferUsage {
    fn from(val: BufferUsage) -> Self {
        wgpu::BufferUsage::from_bits(val.bits()).unwrap()
    }
}

impl WgpuFrom<&LoadOp<Color>> for wgpu::LoadOp<wgpu::Color> {
    fn from(val: &LoadOp<Color>) -> Self {
        match val {
            LoadOp::Clear(value) => wgpu::LoadOp::Clear((*value).wgpu_into()),
            LoadOp::Load => wgpu::LoadOp::Load,
        }
    }
}

impl WgpuFrom<&LoadOp<f32>> for wgpu::LoadOp<f32> {
    fn from(val: &LoadOp<f32>) -> Self {
        match val {
            LoadOp::Clear(value) => wgpu::LoadOp::Clear(*value),
            LoadOp::Load => wgpu::LoadOp::Load,
        }
    }
}

impl WgpuFrom<&LoadOp<u32>> for wgpu::LoadOp<u32> {
    fn from(val: &LoadOp<u32>) -> Self {
        match val {
            LoadOp::Clear(value) => wgpu::LoadOp::Clear(*value),
            LoadOp::Load => wgpu::LoadOp::Load,
        }
    }
}

impl<'a, T, U> WgpuFrom<&'a Operations<T>> for wgpu::Operations<U>
where
    wgpu::LoadOp<U>: WgpuFrom<&'a LoadOp<T>>,
{
    fn from(val: &'a Operations<T>) -> Self {
        Self {
            load: (&val.load).wgpu_into(),
            store: val.store,
        }
    }
}

impl WgpuFrom<&BindType> for wgpu::BindingType {
    fn from(bind_type: &BindType) -> Self {
        match bind_type {
            BindType::Uniform {
                has_dynamic_offset, ..
            } => wgpu::BindingType::Buffer {
                ty: BufferBindingType::Uniform,
                has_dynamic_offset: *has_dynamic_offset,
                min_binding_size: bind_type.get_uniform_size().and_then(wgpu::BufferSize::new),
            },
            BindType::StorageBuffer {
                has_dynamic_offset,
                readonly,
            } => wgpu::BindingType::Buffer {
                ty: BufferBindingType::Storage {
                    read_only: *readonly,
                },
                has_dynamic_offset: *has_dynamic_offset,
                min_binding_size: bind_type.get_uniform_size().and_then(wgpu::BufferSize::new),
            },
            BindType::Texture {
                view_dimension,
                multisampled,
                sample_type,
            } => wgpu::BindingType::Texture {
                view_dimension: (*view_dimension).wgpu_into(),
                multisampled: *multisampled,
                sample_type: (*sample_type).wgpu_into(),
            },
            BindType::Sampler {
                comparison,
                filtering,
            } => wgpu::BindingType::Sampler {
                filtering: *filtering,
                comparison: *comparison,
            },
            BindType::StorageTexture {
                view_dimension,
                format,
                access,
            } => wgpu::BindingType::StorageTexture {
                access: (*access).wgpu_into(),
                view_dimension: (*view_dimension).wgpu_into(),
                format: (*format).wgpu_into(),
            },
        }
    }
}

impl WgpuFrom<TextureSampleType> for wgpu::TextureSampleType {
    fn from(texture_component_type: TextureSampleType) -> Self {
        match texture_component_type {
            TextureSampleType::Float { filterable } => {
                wgpu::TextureSampleType::Float { filterable }
            }
            TextureSampleType::Sint => wgpu::TextureSampleType::Sint,
            TextureSampleType::Uint => wgpu::TextureSampleType::Uint,
            TextureSampleType::Depth => wgpu::TextureSampleType::Depth,
        }
    }
}

impl WgpuFrom<StorageTextureAccess> for wgpu::StorageTextureAccess {
    fn from(storage_texture_access: StorageTextureAccess) -> Self {
        match storage_texture_access {
            StorageTextureAccess::ReadOnly => wgpu::StorageTextureAccess::ReadOnly,
            StorageTextureAccess::WriteOnly => wgpu::StorageTextureAccess::WriteOnly,
            StorageTextureAccess::ReadWrite => wgpu::StorageTextureAccess::ReadWrite,
        }
    }
}

impl WgpuFrom<Extent3d> for wgpu::Extent3d {
    fn from(val: Extent3d) -> Self {
        wgpu::Extent3d {
            height: val.height,
            width: val.width,
            depth_or_array_layers: val.depth_or_array_layers,
        }
    }
}

impl WgpuFrom<&TextureDescriptor> for wgpu::TextureDescriptor<'_> {
    fn from(texture_descriptor: &TextureDescriptor) -> Self {
        wgpu::TextureDescriptor {
            label: None,
            size: texture_descriptor.size.wgpu_into(),
            mip_level_count: texture_descriptor.mip_level_count,
            sample_count: texture_descriptor.sample_count,
            dimension: texture_descriptor.dimension.wgpu_into(),
            format: texture_descriptor.format.wgpu_into(),
            usage: texture_descriptor.usage.wgpu_into(),
        }
    }
}

impl WgpuFrom<TextureViewDimension> for wgpu::TextureViewDimension {
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

impl WgpuFrom<TextureDimension> for wgpu::TextureDimension {
    fn from(dimension: TextureDimension) -> Self {
        match dimension {
            TextureDimension::D1 => wgpu::TextureDimension::D1,
            TextureDimension::D2 => wgpu::TextureDimension::D2,
            TextureDimension::D3 => wgpu::TextureDimension::D3,
        }
    }
}

impl WgpuFrom<TextureFormat> for wgpu::TextureFormat {
    fn from(val: TextureFormat) -> Self {
        match val {
            TextureFormat::R8Unorm => wgpu::TextureFormat::R8Unorm,
            TextureFormat::R8Snorm => wgpu::TextureFormat::R8Snorm,
            TextureFormat::R8Uint => wgpu::TextureFormat::R8Uint,
            TextureFormat::R8Sint => wgpu::TextureFormat::R8Sint,
            TextureFormat::R16Uint => wgpu::TextureFormat::R16Uint,
            TextureFormat::R16Sint => wgpu::TextureFormat::R16Sint,
            TextureFormat::R16Float => wgpu::TextureFormat::R16Float,
            TextureFormat::Rg8Unorm => wgpu::TextureFormat::Rg8Unorm,
            TextureFormat::Rg8Snorm => wgpu::TextureFormat::Rg8Snorm,
            TextureFormat::Rg8Uint => wgpu::TextureFormat::Rg8Uint,
            TextureFormat::Rg8Sint => wgpu::TextureFormat::Rg8Sint,
            TextureFormat::R32Uint => wgpu::TextureFormat::R32Uint,
            TextureFormat::R32Sint => wgpu::TextureFormat::R32Sint,
            TextureFormat::R32Float => wgpu::TextureFormat::R32Float,
            TextureFormat::Rg16Uint => wgpu::TextureFormat::Rg16Uint,
            TextureFormat::Rg16Sint => wgpu::TextureFormat::Rg16Sint,
            TextureFormat::Rg16Float => wgpu::TextureFormat::Rg16Float,
            TextureFormat::Rgba8Unorm => wgpu::TextureFormat::Rgba8Unorm,
            TextureFormat::Rgba8UnormSrgb => wgpu::TextureFormat::Rgba8UnormSrgb,
            TextureFormat::Rgba8Snorm => wgpu::TextureFormat::Rgba8Snorm,
            TextureFormat::Rgba8Uint => wgpu::TextureFormat::Rgba8Uint,
            TextureFormat::Rgba8Sint => wgpu::TextureFormat::Rgba8Sint,
            TextureFormat::Bgra8Unorm => wgpu::TextureFormat::Bgra8Unorm,
            TextureFormat::Bgra8UnormSrgb => wgpu::TextureFormat::Bgra8UnormSrgb,
            TextureFormat::Rgb10a2Unorm => wgpu::TextureFormat::Rgb10a2Unorm,
            TextureFormat::Rg11b10Float => wgpu::TextureFormat::Rg11b10Float,
            TextureFormat::Rg32Uint => wgpu::TextureFormat::Rg32Uint,
            TextureFormat::Rg32Sint => wgpu::TextureFormat::Rg32Sint,
            TextureFormat::Rg32Float => wgpu::TextureFormat::Rg32Float,
            TextureFormat::Rgba16Uint => wgpu::TextureFormat::Rgba16Uint,
            TextureFormat::Rgba16Sint => wgpu::TextureFormat::Rgba16Sint,
            TextureFormat::Rgba16Float => wgpu::TextureFormat::Rgba16Float,
            TextureFormat::Rgba32Uint => wgpu::TextureFormat::Rgba32Uint,
            TextureFormat::Rgba32Sint => wgpu::TextureFormat::Rgba32Sint,
            TextureFormat::Rgba32Float => wgpu::TextureFormat::Rgba32Float,
            TextureFormat::Depth32Float => wgpu::TextureFormat::Depth32Float,
            TextureFormat::Depth24Plus => wgpu::TextureFormat::Depth24Plus,
            TextureFormat::Depth24PlusStencil8 => wgpu::TextureFormat::Depth24PlusStencil8,
        }
    }
}

impl WgpuFrom<TextureUsage> for wgpu::TextureUsage {
    fn from(val: TextureUsage) -> Self {
        wgpu::TextureUsage::from_bits(val.bits()).unwrap()
    }
}

impl WgpuFrom<&StencilState> for wgpu::StencilState {
    fn from(val: &StencilState) -> Self {
        wgpu::StencilState {
            back: (&val.back).wgpu_into(),
            front: (&val.front).wgpu_into(),
            read_mask: val.read_mask,
            write_mask: val.write_mask,
        }
    }
}

impl WgpuFrom<DepthStencilState> for wgpu::DepthStencilState {
    fn from(val: DepthStencilState) -> Self {
        wgpu::DepthStencilState {
            depth_compare: val.depth_compare.wgpu_into(),
            depth_write_enabled: val.depth_write_enabled,
            format: val.format.wgpu_into(),
            stencil: (&val.stencil).wgpu_into(),
            bias: val.bias.wgpu_into(),
        }
    }
}

impl WgpuFrom<MultisampleState> for wgpu::MultisampleState {
    fn from(val: MultisampleState) -> Self {
        wgpu::MultisampleState {
            count: val.count,
            mask: val.mask,
            alpha_to_coverage_enabled: val.alpha_to_coverage_enabled,
        }
    }
}

impl WgpuFrom<&StencilFaceState> for wgpu::StencilFaceState {
    fn from(val: &StencilFaceState) -> Self {
        wgpu::StencilFaceState {
            compare: val.compare.wgpu_into(),
            depth_fail_op: val.depth_fail_op.wgpu_into(),
            fail_op: val.fail_op.wgpu_into(),
            pass_op: val.pass_op.wgpu_into(),
        }
    }
}

impl WgpuFrom<CompareFunction> for wgpu::CompareFunction {
    fn from(val: CompareFunction) -> Self {
        match val {
            CompareFunction::Never => wgpu::CompareFunction::Never,
            CompareFunction::Less => wgpu::CompareFunction::Less,
            CompareFunction::Equal => wgpu::CompareFunction::Equal,
            CompareFunction::LessEqual => wgpu::CompareFunction::LessEqual,
            CompareFunction::Greater => wgpu::CompareFunction::Greater,
            CompareFunction::NotEqual => wgpu::CompareFunction::NotEqual,
            CompareFunction::GreaterEqual => wgpu::CompareFunction::GreaterEqual,
            CompareFunction::Always => wgpu::CompareFunction::Always,
        }
    }
}

static COMPARE_FUNCTION_NEVER: &wgpu::CompareFunction = &wgpu::CompareFunction::Never;
static COMPARE_FUNCTION_LESS: &wgpu::CompareFunction = &wgpu::CompareFunction::Less;
static COMPARE_FUNCTION_EQUAL: &wgpu::CompareFunction = &wgpu::CompareFunction::Equal;
static COMPARE_FUNCTION_LESSEQUAL: &wgpu::CompareFunction = &wgpu::CompareFunction::LessEqual;
static COMPARE_FUNCTION_GREATER: &wgpu::CompareFunction = &wgpu::CompareFunction::Greater;
static COMPARE_FUNCTION_NOTEQUAL: &wgpu::CompareFunction = &wgpu::CompareFunction::NotEqual;
static COMPARE_FUNCTION_GREATEREQUAL: &wgpu::CompareFunction = &wgpu::CompareFunction::GreaterEqual;
static COMPARE_FUNCTION_ALWAYS: &wgpu::CompareFunction = &wgpu::CompareFunction::Always;

impl WgpuFrom<CompareFunction> for &'static wgpu::CompareFunction {
    fn from(val: CompareFunction) -> Self {
        match val {
            CompareFunction::Never => COMPARE_FUNCTION_NEVER,
            CompareFunction::Less => COMPARE_FUNCTION_LESS,
            CompareFunction::Equal => COMPARE_FUNCTION_EQUAL,
            CompareFunction::LessEqual => COMPARE_FUNCTION_LESSEQUAL,
            CompareFunction::Greater => COMPARE_FUNCTION_GREATER,
            CompareFunction::NotEqual => COMPARE_FUNCTION_NOTEQUAL,
            CompareFunction::GreaterEqual => COMPARE_FUNCTION_GREATEREQUAL,
            CompareFunction::Always => COMPARE_FUNCTION_ALWAYS,
        }
    }
}

impl WgpuFrom<StencilOperation> for wgpu::StencilOperation {
    fn from(val: StencilOperation) -> Self {
        match val {
            StencilOperation::Keep => wgpu::StencilOperation::Keep,
            StencilOperation::Zero => wgpu::StencilOperation::Zero,
            StencilOperation::Replace => wgpu::StencilOperation::Replace,
            StencilOperation::Invert => wgpu::StencilOperation::Invert,
            StencilOperation::IncrementClamp => wgpu::StencilOperation::IncrementClamp,
            StencilOperation::DecrementClamp => wgpu::StencilOperation::DecrementClamp,
            StencilOperation::IncrementWrap => wgpu::StencilOperation::IncrementWrap,
            StencilOperation::DecrementWrap => wgpu::StencilOperation::DecrementWrap,
        }
    }
}

impl WgpuFrom<PrimitiveTopology> for wgpu::PrimitiveTopology {
    fn from(val: PrimitiveTopology) -> Self {
        match val {
            PrimitiveTopology::PointList => wgpu::PrimitiveTopology::PointList,
            PrimitiveTopology::LineList => wgpu::PrimitiveTopology::LineList,
            PrimitiveTopology::LineStrip => wgpu::PrimitiveTopology::LineStrip,
            PrimitiveTopology::TriangleList => wgpu::PrimitiveTopology::TriangleList,
            PrimitiveTopology::TriangleStrip => wgpu::PrimitiveTopology::TriangleStrip,
        }
    }
}

impl WgpuFrom<FrontFace> for wgpu::FrontFace {
    fn from(val: FrontFace) -> Self {
        match val {
            FrontFace::Ccw => wgpu::FrontFace::Ccw,
            FrontFace::Cw => wgpu::FrontFace::Cw,
        }
    }
}

impl WgpuFrom<Face> for wgpu::Face {
    fn from(val: Face) -> Self {
        match val {
            Face::Front => wgpu::Face::Front,
            Face::Back => wgpu::Face::Back,
        }
    }
}

impl WgpuFrom<PolygonMode> for wgpu::PolygonMode {
    fn from(val: PolygonMode) -> wgpu::PolygonMode {
        match val {
            PolygonMode::Fill => wgpu::PolygonMode::Fill,
            PolygonMode::Line => wgpu::PolygonMode::Line,
            PolygonMode::Point => wgpu::PolygonMode::Point,
        }
    }
}

impl WgpuFrom<DepthBiasState> for wgpu::DepthBiasState {
    fn from(val: DepthBiasState) -> Self {
        wgpu::DepthBiasState {
            constant: val.constant,
            slope_scale: val.slope_scale,
            clamp: val.clamp,
        }
    }
}

impl WgpuFrom<&ColorTargetState> for wgpu::ColorTargetState {
    fn from(val: &ColorTargetState) -> Self {
        wgpu::ColorTargetState {
            format: val.format.wgpu_into(),
            write_mask: val.write_mask.wgpu_into(),
            blend: val.blend.map(|blend| blend.wgpu_into()),
        }
    }
}

impl WgpuFrom<PrimitiveState> for wgpu::PrimitiveState {
    fn from(val: PrimitiveState) -> Self {
        wgpu::PrimitiveState {
            topology: val.topology.wgpu_into(),
            strip_index_format: val
                .strip_index_format
                .map(|index_format| index_format.wgpu_into()),
            front_face: val.front_face.wgpu_into(),
            cull_mode: val.cull_mode.map(|face| face.wgpu_into()),
            polygon_mode: val.polygon_mode.wgpu_into(),
            clamp_depth: val.clamp_depth,
            conservative: val.conservative,
        }
    }
}

impl WgpuFrom<ColorWrite> for wgpu::ColorWrite {
    fn from(val: ColorWrite) -> Self {
        wgpu::ColorWrite::from_bits(val.bits()).unwrap()
    }
}

impl WgpuFrom<BlendState> for wgpu::BlendState {
    fn from(val: BlendState) -> Self {
        wgpu::BlendState {
            color: wgpu::BlendComponent {
                src_factor: val.color.src_factor.wgpu_into(),
                dst_factor: val.color.dst_factor.wgpu_into(),
                operation: val.color.operation.wgpu_into(),
            },
            alpha: wgpu::BlendComponent {
                src_factor: val.alpha.src_factor.wgpu_into(),
                dst_factor: val.alpha.dst_factor.wgpu_into(),
                operation: val.alpha.operation.wgpu_into(),
            },
        }
    }
}

impl WgpuFrom<BlendFactor> for wgpu::BlendFactor {
    fn from(val: BlendFactor) -> Self {
        match val {
            BlendFactor::Zero => wgpu::BlendFactor::Zero,
            BlendFactor::One => wgpu::BlendFactor::One,
            BlendFactor::Src => wgpu::BlendFactor::Src,
            BlendFactor::OneMinusSrc => wgpu::BlendFactor::OneMinusSrc,
            BlendFactor::SrcAlpha => wgpu::BlendFactor::SrcAlpha,
            BlendFactor::OneMinusSrcAlpha => wgpu::BlendFactor::OneMinusSrcAlpha,
            BlendFactor::Dst => wgpu::BlendFactor::Dst,
            BlendFactor::OneMinusDst => wgpu::BlendFactor::OneMinusDst,
            BlendFactor::DstAlpha => wgpu::BlendFactor::DstAlpha,
            BlendFactor::OneMinusDstAlpha => wgpu::BlendFactor::OneMinusDstAlpha,
            BlendFactor::SrcAlphaSaturated => wgpu::BlendFactor::SrcAlphaSaturated,
            BlendFactor::Constant => wgpu::BlendFactor::Constant,
            BlendFactor::OneMinusConstant => wgpu::BlendFactor::OneMinusConstant,
        }
    }
}

impl WgpuFrom<BlendOperation> for wgpu::BlendOperation {
    fn from(val: BlendOperation) -> Self {
        match val {
            BlendOperation::Add => wgpu::BlendOperation::Add,
            BlendOperation::Subtract => wgpu::BlendOperation::Subtract,
            BlendOperation::ReverseSubtract => wgpu::BlendOperation::ReverseSubtract,
            BlendOperation::Min => wgpu::BlendOperation::Min,
            BlendOperation::Max => wgpu::BlendOperation::Max,
        }
    }
}

impl WgpuFrom<IndexFormat> for wgpu::IndexFormat {
    fn from(val: IndexFormat) -> Self {
        match val {
            IndexFormat::Uint16 => wgpu::IndexFormat::Uint16,
            IndexFormat::Uint32 => wgpu::IndexFormat::Uint32,
        }
    }
}

impl WgpuFrom<SamplerDescriptor> for wgpu::SamplerDescriptor<'_> {
    fn from(sampler_descriptor: SamplerDescriptor) -> Self {
        wgpu::SamplerDescriptor {
            label: None,
            address_mode_u: sampler_descriptor.address_mode_u.wgpu_into(),
            address_mode_v: sampler_descriptor.address_mode_v.wgpu_into(),
            address_mode_w: sampler_descriptor.address_mode_w.wgpu_into(),
            mag_filter: sampler_descriptor.mag_filter.wgpu_into(),
            min_filter: sampler_descriptor.min_filter.wgpu_into(),
            mipmap_filter: sampler_descriptor.mipmap_filter.wgpu_into(),
            lod_min_clamp: sampler_descriptor.lod_min_clamp,
            lod_max_clamp: sampler_descriptor.lod_max_clamp,
            compare: sampler_descriptor.compare_function.map(|c| c.wgpu_into()),
            anisotropy_clamp: sampler_descriptor.anisotropy_clamp,
            border_color: sampler_descriptor
                .border_color
                .map(|border_color| border_color.wgpu_into()),
        }
    }
}

impl WgpuFrom<AddressMode> for wgpu::AddressMode {
    fn from(val: AddressMode) -> Self {
        match val {
            AddressMode::ClampToEdge => wgpu::AddressMode::ClampToEdge,
            AddressMode::Repeat => wgpu::AddressMode::Repeat,
            AddressMode::MirrorRepeat => wgpu::AddressMode::MirrorRepeat,
        }
    }
}

impl WgpuFrom<FilterMode> for wgpu::FilterMode {
    fn from(val: FilterMode) -> Self {
        match val {
            FilterMode::Nearest => wgpu::FilterMode::Nearest,
            FilterMode::Linear => wgpu::FilterMode::Linear,
        }
    }
}

impl WgpuFrom<SamplerBorderColor> for wgpu::SamplerBorderColor {
    fn from(val: SamplerBorderColor) -> Self {
        match val {
            SamplerBorderColor::TransparentBlack => wgpu::SamplerBorderColor::TransparentBlack,
            SamplerBorderColor::OpaqueBlack => wgpu::SamplerBorderColor::OpaqueBlack,
            SamplerBorderColor::OpaqueWhite => wgpu::SamplerBorderColor::OpaqueWhite,
        }
    }
}

impl WgpuFrom<&Window> for wgpu::SwapChainDescriptor {
    fn from(window: &Window) -> Self {
        wgpu::SwapChainDescriptor {
            usage: wgpu::TextureUsage::RENDER_ATTACHMENT,
            format: TextureFormat::default().wgpu_into(),
            width: window.physical_width().max(1),
            height: window.physical_height().max(1),
            present_mode: if window.vsync() {
                wgpu::PresentMode::Fifo
            } else {
                wgpu::PresentMode::Immediate
            },
        }
    }
}

impl WgpuFrom<WgpuFeature> for wgpu::Features {
    fn from(value: WgpuFeature) -> Self {
        match value {
            WgpuFeature::DepthClamping => wgpu::Features::DEPTH_CLAMPING,
            WgpuFeature::TextureCompressionBc => wgpu::Features::TEXTURE_COMPRESSION_BC,
            WgpuFeature::TimestampQuery => wgpu::Features::TIMESTAMP_QUERY,
            WgpuFeature::PipelineStatisticsQuery => wgpu::Features::PIPELINE_STATISTICS_QUERY,
            WgpuFeature::MappablePrimaryBuffers => wgpu::Features::MAPPABLE_PRIMARY_BUFFERS,
            WgpuFeature::SampledTextureBindingArray => {
                wgpu::Features::SAMPLED_TEXTURE_BINDING_ARRAY
            }
            WgpuFeature::SampledTextureArrayDynamicIndexing => {
                wgpu::Features::SAMPLED_TEXTURE_ARRAY_DYNAMIC_INDEXING
            }
            WgpuFeature::SampledTextureArrayNonUniformIndexing => {
                wgpu::Features::SAMPLED_TEXTURE_ARRAY_NON_UNIFORM_INDEXING
            }
            WgpuFeature::UnsizedBindingArray => wgpu::Features::UNSIZED_BINDING_ARRAY,
            WgpuFeature::MultiDrawIndirect => wgpu::Features::MULTI_DRAW_INDIRECT,
            WgpuFeature::MultiDrawIndirectCount => wgpu::Features::MULTI_DRAW_INDIRECT_COUNT,
            WgpuFeature::PushConstants => wgpu::Features::PUSH_CONSTANTS,
            WgpuFeature::AddressModeClampToBorder => wgpu::Features::ADDRESS_MODE_CLAMP_TO_BORDER,
            WgpuFeature::NonFillPolygonMode => wgpu::Features::NON_FILL_POLYGON_MODE,
            WgpuFeature::TextureCompressionEtc2 => wgpu::Features::TEXTURE_COMPRESSION_ETC2,
            WgpuFeature::TextureCompressionAstcLdr => wgpu::Features::TEXTURE_COMPRESSION_ASTC_LDR,
            WgpuFeature::TextureAdapterSpecificFormatFeatures => {
                wgpu::Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES
            }
            WgpuFeature::ShaderFloat64 => wgpu::Features::SHADER_FLOAT64,
            WgpuFeature::VertexAttribute64Bit => wgpu::Features::VERTEX_ATTRIBUTE_64BIT,
        }
    }
}

impl WgpuFrom<WgpuFeatures> for wgpu::Features {
    fn from(features: WgpuFeatures) -> Self {
        features
            .features
            .iter()
            .fold(wgpu::Features::empty(), |wgpu_features, feature| {
                wgpu_features | (*feature).wgpu_into()
            })
    }
}

impl WgpuFrom<WgpuLimits> for wgpu::Limits {
    fn from(val: WgpuLimits) -> Self {
        wgpu::Limits {
            max_bind_groups: val.max_bind_groups,
            max_dynamic_uniform_buffers_per_pipeline_layout: val
                .max_dynamic_uniform_buffers_per_pipeline_layout,
            max_dynamic_storage_buffers_per_pipeline_layout: val
                .max_dynamic_storage_buffers_per_pipeline_layout,
            max_sampled_textures_per_shader_stage: val.max_sampled_textures_per_shader_stage,
            max_samplers_per_shader_stage: val.max_samplers_per_shader_stage,
            max_storage_buffers_per_shader_stage: val.max_storage_buffers_per_shader_stage,
            max_storage_textures_per_shader_stage: val.max_storage_textures_per_shader_stage,
            max_uniform_buffers_per_shader_stage: val.max_uniform_buffers_per_shader_stage,
            max_uniform_buffer_binding_size: val.max_uniform_buffer_binding_size,
            max_push_constant_size: val.max_push_constant_size,
            max_texture_dimension_1d: val.max_texture_dimension_1d,
            max_texture_dimension_2d: val.max_texture_dimension_2d,
            max_texture_dimension_3d: val.max_texture_dimension_3d,
            max_texture_array_layers: val.max_texture_array_layers,
            max_storage_buffer_binding_size: val.max_storage_buffer_binding_size,
            max_vertex_buffers: val.max_vertex_buffers,
            max_vertex_attributes: val.max_vertex_attributes,
            max_vertex_buffer_array_stride: val.max_vertex_buffer_array_stride,
        }
    }
}

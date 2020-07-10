use bevy_render::{
    pass::{LoadOp, Operations},
    pipeline::{
        state_descriptors::{
            BlendDescriptor, BlendFactor, BlendOperation, ColorStateDescriptor, ColorWrite,
            CompareFunction, CullMode, DepthStencilStateDescriptor, FrontFace, IndexFormat,
            PrimitiveTopology, RasterizationStateDescriptor, StencilOperation,
            StencilStateFaceDescriptor,
        },
        BindType, InputStepMode, VertexAttributeDescriptor, VertexBufferDescriptor, VertexFormat,
    },
    render_resource::BufferUsage,
    texture::{
        AddressMode, Extent3d, FilterMode, SamplerDescriptor, TextureComponentType,
        TextureDescriptor, TextureDimension, TextureFormat, TextureUsage, TextureViewDimension,
    },
    Color,
};
use bevy_window::Window;

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

impl WgpuFrom<&VertexAttributeDescriptor> for wgpu::VertexAttributeDescriptor {
    fn from(val: &VertexAttributeDescriptor) -> Self {
        wgpu::VertexAttributeDescriptor {
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
pub struct OwnedWgpuVertexBufferDescriptor {
    pub stride: wgpu::BufferAddress,
    pub step_mode: wgpu::InputStepMode,
    pub attributes: Vec<wgpu::VertexAttributeDescriptor>,
}

impl WgpuFrom<&VertexBufferDescriptor> for OwnedWgpuVertexBufferDescriptor {
    fn from(val: &VertexBufferDescriptor) -> OwnedWgpuVertexBufferDescriptor {
        let attributes = val
            .attributes
            .iter()
            .map(|a| a.wgpu_into())
            .collect::<Vec<wgpu::VertexAttributeDescriptor>>();
        OwnedWgpuVertexBufferDescriptor {
            step_mode: val.step_mode.wgpu_into(),
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

impl WgpuFrom<Color> for wgpu::Color {
    fn from(color: Color) -> Self {
        wgpu::Color {
            r: color.r as f64,
            g: color.g as f64,
            b: color.b as f64,
            a: color.a as f64,
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
            LoadOp::Clear(value) => wgpu::LoadOp::Clear(value.clone().wgpu_into()),
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
                dynamic,
                properties: _properties,
            } => wgpu::BindingType::UniformBuffer {
                dynamic: *dynamic,
                min_binding_size: bind_type
                    .get_uniform_size()
                    .and_then(|size| wgpu::BufferSize::new(size)),
            },
            BindType::StorageBuffer { dynamic, readonly } => wgpu::BindingType::StorageBuffer {
                dynamic: *dynamic,
                readonly: *readonly,
                min_binding_size: bind_type
                    .get_uniform_size()
                    .and_then(|size| wgpu::BufferSize::new(size)),
            },
            BindType::SampledTexture {
                dimension,
                multisampled,
                component_type,
            } => wgpu::BindingType::SampledTexture {
                dimension: (*dimension).wgpu_into(),
                multisampled: *multisampled,
                component_type: (*component_type).wgpu_into(),
            },
            BindType::Sampler { comparison } => wgpu::BindingType::Sampler {
                comparison: *comparison,
            },
            BindType::StorageTexture {
                dimension,
                format,
                readonly,
            } => wgpu::BindingType::StorageTexture {
                dimension: (*dimension).wgpu_into(),
                format: (*format).wgpu_into(),
                readonly: *readonly,
            },
        }
    }
}

impl WgpuFrom<TextureComponentType> for wgpu::TextureComponentType {
    fn from(texture_component_type: TextureComponentType) -> Self {
        match texture_component_type {
            TextureComponentType::Float => wgpu::TextureComponentType::Float,
            TextureComponentType::Sint => wgpu::TextureComponentType::Sint,
            TextureComponentType::Uint => wgpu::TextureComponentType::Uint,
        }
    }
}

impl WgpuFrom<Extent3d> for wgpu::Extent3d {
    fn from(val: Extent3d) -> Self {
        wgpu::Extent3d {
            depth: val.depth,
            height: val.height,
            width: val.width,
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

impl WgpuFrom<&DepthStencilStateDescriptor> for wgpu::DepthStencilStateDescriptor {
    fn from(val: &DepthStencilStateDescriptor) -> Self {
        wgpu::DepthStencilStateDescriptor {
            depth_compare: val.depth_compare.wgpu_into(),
            depth_write_enabled: val.depth_write_enabled,
            format: val.format.wgpu_into(),
            stencil_back: (&val.stencil_back).wgpu_into(),
            stencil_front: (&val.stencil_front).wgpu_into(),
            stencil_read_mask: val.stencil_read_mask,
            stencil_write_mask: val.stencil_write_mask,
        }
    }
}

impl WgpuFrom<&StencilStateFaceDescriptor> for wgpu::StencilStateFaceDescriptor {
    fn from(val: &StencilStateFaceDescriptor) -> Self {
        wgpu::StencilStateFaceDescriptor {
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

impl WgpuFrom<CullMode> for wgpu::CullMode {
    fn from(val: CullMode) -> Self {
        match val {
            CullMode::None => wgpu::CullMode::None,
            CullMode::Front => wgpu::CullMode::Front,
            CullMode::Back => wgpu::CullMode::Back,
        }
    }
}

impl WgpuFrom<&RasterizationStateDescriptor> for wgpu::RasterizationStateDescriptor {
    fn from(val: &RasterizationStateDescriptor) -> Self {
        wgpu::RasterizationStateDescriptor {
            front_face: val.front_face.wgpu_into(),
            cull_mode: val.cull_mode.wgpu_into(),
            depth_bias: val.depth_bias,
            depth_bias_slope_scale: val.depth_bias_slope_scale,
            depth_bias_clamp: val.depth_bias_clamp,
        }
    }
}

impl WgpuFrom<&ColorStateDescriptor> for wgpu::ColorStateDescriptor {
    fn from(val: &ColorStateDescriptor) -> Self {
        wgpu::ColorStateDescriptor {
            format: val.format.wgpu_into(),
            alpha_blend: (&val.alpha_blend).wgpu_into(),
            color_blend: (&val.color_blend).wgpu_into(),
            write_mask: val.write_mask.wgpu_into(),
        }
    }
}

impl WgpuFrom<ColorWrite> for wgpu::ColorWrite {
    fn from(val: ColorWrite) -> Self {
        wgpu::ColorWrite::from_bits(val.bits()).unwrap()
    }
}

impl WgpuFrom<&BlendDescriptor> for wgpu::BlendDescriptor {
    fn from(val: &BlendDescriptor) -> Self {
        wgpu::BlendDescriptor {
            src_factor: val.src_factor.wgpu_into(),
            dst_factor: val.dst_factor.wgpu_into(),
            operation: val.operation.wgpu_into(),
        }
    }
}

impl WgpuFrom<BlendFactor> for wgpu::BlendFactor {
    fn from(val: BlendFactor) -> Self {
        match val {
            BlendFactor::Zero => wgpu::BlendFactor::Zero,
            BlendFactor::One => wgpu::BlendFactor::One,
            BlendFactor::SrcColor => wgpu::BlendFactor::SrcColor,
            BlendFactor::OneMinusSrcColor => wgpu::BlendFactor::OneMinusSrcColor,
            BlendFactor::SrcAlpha => wgpu::BlendFactor::SrcAlpha,
            BlendFactor::OneMinusSrcAlpha => wgpu::BlendFactor::OneMinusSrcAlpha,
            BlendFactor::DstColor => wgpu::BlendFactor::DstColor,
            BlendFactor::OneMinusDstColor => wgpu::BlendFactor::OneMinusDstColor,
            BlendFactor::DstAlpha => wgpu::BlendFactor::DstAlpha,
            BlendFactor::OneMinusDstAlpha => wgpu::BlendFactor::OneMinusDstAlpha,
            BlendFactor::SrcAlphaSaturated => wgpu::BlendFactor::SrcAlphaSaturated,
            BlendFactor::BlendColor => wgpu::BlendFactor::BlendColor,
            BlendFactor::OneMinusBlendColor => wgpu::BlendFactor::OneMinusBlendColor,
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
            anisotropy_clamp: sampler_descriptor.anisotropy_clamp.clone(),
            ..Default::default()
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

impl WgpuFrom<&Window> for wgpu::SwapChainDescriptor {
    fn from(window: &Window) -> Self {
        wgpu::SwapChainDescriptor {
            usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT,
            format: wgpu::TextureFormat::Bgra8UnormSrgb,
            width: window.width,
            height: window.height,
            present_mode: if window.vsync {
                wgpu::PresentMode::Fifo
            } else {
                wgpu::PresentMode::Immediate
            },
        }
    }
}

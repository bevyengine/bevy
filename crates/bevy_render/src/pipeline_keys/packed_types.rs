// KeyType implementations for primitive and third-party types
// todo: add u2-7 newtypes

use crate::pipeline_keys::*;

impl AnyKeyType for () {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl KeyTypeConcrete for () {
    fn unpack(_: KeyPrimitive, _: &KeyMetaStore) -> Self {}

    fn positions(_: &KeyMetaStore) -> HashMap<TypeId, SizeOffset> {
        HashMap::default()
    }

    fn pack(_: &Self, _: &KeyMetaStore) -> PackedPipelineKey<Self>
    where
        Self: Sized,
    {
        PackedPipelineKey {
            packed: 0,
            size: 0,
            _p: PhantomData,
        }
    }

    fn size(_: &KeyMetaStore) -> u8 {
        0
    }
}

impl FixedSizeKey for () {
    fn fixed_size() -> u8 {
        0
    }
}

impl<T: AnyKeyType> AnyKeyType for Option<T> {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl<T: KeyTypeConcrete> KeyTypeConcrete for Option<T> {
    fn unpack(value: KeyPrimitive, store: &KeyMetaStore) -> Self {
        if value & 1 == 0 {
            None
        } else {
            Some(T::unpack(value >> 1, store))
        }
    }

    fn positions(store: &KeyMetaStore) -> HashMap<TypeId, SizeOffset> {
        HashMap::from_iter([(TypeId::of::<Self>(), SizeOffset(T::size(store) + 1, 0))])
    }

    fn pack(value: &Self, store: &KeyMetaStore) -> PackedPipelineKey<Self>
    where
        Self: Sized,
    {
        if let Some(inner) = value {
            let packed = T::pack(inner, store);
            PackedPipelineKey {
                packed: packed.packed << 1 | 1,
                size: packed.size + 1,
                _p: PhantomData,
            }
        } else {
            PackedPipelineKey {
                packed: 0,
                size: T::size(store) + 1,
                _p: PhantomData,
            }
        }
    }
}

impl<T: FixedSizeKey> FixedSizeKey for Option<T> {
    fn fixed_size() -> u8 {
        T::fixed_size() + 1
    }
}

impl AnyKeyType for bool {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl KeyTypeConcrete for bool {
    fn positions(_: &KeyMetaStore) -> HashMap<TypeId, SizeOffset> {
        HashMap::from_iter([(TypeId::of::<Self>(), SizeOffset(1, 0))])
    }

    fn size(_: &KeyMetaStore) -> u8 {
        Self::fixed_size()
    }

    fn pack(value: &Self, _: &KeyMetaStore) -> PackedPipelineKey<Self> {
        let raw = if *value { 1 } else { 0 };

        PackedPipelineKey::new(raw, 1)
    }

    fn unpack(value: KeyPrimitive, _: &KeyMetaStore) -> Self {
        value != 0
    }
}

impl FixedSizeKey for bool {
    fn fixed_size() -> u8 {
        1
    }
}

impl AnyKeyType for u8 {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl KeyTypeConcrete for u8 {
    fn positions(_: &KeyMetaStore) -> HashMap<TypeId, SizeOffset> {
        HashMap::from_iter([(TypeId::of::<Self>(), SizeOffset(8, 0))])
    }

    fn size(_: &KeyMetaStore) -> u8 {
        Self::fixed_size()
    }

    fn pack(value: &Self, _: &KeyMetaStore) -> PackedPipelineKey<Self> {
        PackedPipelineKey::new(*value as KeyPrimitive, 8)
    }

    fn unpack(value: KeyPrimitive, _: &KeyMetaStore) -> Self {
        value as u8
    }
}

impl FixedSizeKey for u8 {
    fn fixed_size() -> u8 {
        8
    }
}

impl AnyKeyType for u32 {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl KeyTypeConcrete for u32 {
    fn positions(_: &KeyMetaStore) -> HashMap<TypeId, SizeOffset> {
        HashMap::from_iter([(TypeId::of::<Self>(), SizeOffset(Self::fixed_size(), 0))])
    }

    fn size(_: &KeyMetaStore) -> u8 {
        Self::fixed_size()
    }

    fn pack(value: &Self, _: &KeyMetaStore) -> PackedPipelineKey<Self> {
        PackedPipelineKey::new(*value as KeyPrimitive, Self::fixed_size())
    }

    fn unpack(value: KeyPrimitive, _: &KeyMetaStore) -> Self {
        value as Self
    }
}

impl FixedSizeKey for u32 {
    fn fixed_size() -> u8 {
        32
    }
}

impl AnyKeyType for i32 {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl KeyTypeConcrete for i32 {
    fn positions(_: &KeyMetaStore) -> HashMap<TypeId, SizeOffset> {
        HashMap::from_iter([(TypeId::of::<Self>(), SizeOffset(Self::fixed_size(), 0))])
    }

    fn size(_: &KeyMetaStore) -> u8 {
        Self::fixed_size()
    }

    fn pack(value: &Self, _: &KeyMetaStore) -> PackedPipelineKey<Self> {
        PackedPipelineKey::new(*value as KeyPrimitive, Self::fixed_size())
    }

    fn unpack(value: KeyPrimitive, _: &KeyMetaStore) -> Self {
        value as Self
    }
}

impl FixedSizeKey for i32 {
    fn fixed_size() -> u8 {
        32
    }
}

// blend factor
fn _check_blendfactor_variant_count(value: &wgpu::BlendFactor) {
    // this is to ensure we cover all variants. if the number of variants changes in future, the key size may need to be updated.
    // this will be possible robustly once https://github.com/rust-lang/rust/issues/73662 lands.
    // 13 variants => 4 bits
    match value {
        wgpu::BlendFactor::Zero
        | wgpu::BlendFactor::One
        | wgpu::BlendFactor::Src
        | wgpu::BlendFactor::OneMinusSrc
        | wgpu::BlendFactor::SrcAlpha
        | wgpu::BlendFactor::OneMinusSrcAlpha
        | wgpu::BlendFactor::Dst
        | wgpu::BlendFactor::OneMinusDst
        | wgpu::BlendFactor::DstAlpha
        | wgpu::BlendFactor::OneMinusDstAlpha
        | wgpu::BlendFactor::SrcAlphaSaturated
        | wgpu::BlendFactor::Constant
        | wgpu::BlendFactor::OneMinusConstant => (),
    }
}

impl AnyKeyType for wgpu::BlendFactor {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl KeyTypeConcrete for wgpu::BlendFactor {
    fn positions(_: &KeyMetaStore) -> HashMap<TypeId, SizeOffset> {
        HashMap::from_iter([(TypeId::of::<Self>(), SizeOffset(Self::fixed_size(), 0))])
    }

    fn size(_: &KeyMetaStore) -> u8 {
        Self::fixed_size()
    }

    fn pack(value: &Self, _: &KeyMetaStore) -> PackedPipelineKey<Self> {
        let raw = match value {
            wgpu::BlendFactor::Zero => 0,
            wgpu::BlendFactor::One => 1,
            wgpu::BlendFactor::Src => 2,
            wgpu::BlendFactor::OneMinusSrc => 3,
            wgpu::BlendFactor::SrcAlpha => 4,
            wgpu::BlendFactor::OneMinusSrcAlpha => 5,
            wgpu::BlendFactor::Dst => 6,
            wgpu::BlendFactor::OneMinusDst => 7,
            wgpu::BlendFactor::DstAlpha => 8,
            wgpu::BlendFactor::OneMinusDstAlpha => 9,
            wgpu::BlendFactor::SrcAlphaSaturated => 10,
            wgpu::BlendFactor::Constant => 11,
            wgpu::BlendFactor::OneMinusConstant => 12,
        };

        PackedPipelineKey::new(raw, Self::fixed_size())
    }

    fn unpack(value: KeyPrimitive, _: &KeyMetaStore) -> Self {
        match value {
            0 => wgpu::BlendFactor::Zero,
            1 => wgpu::BlendFactor::One,
            2 => wgpu::BlendFactor::Src,
            3 => wgpu::BlendFactor::OneMinusSrc,
            4 => wgpu::BlendFactor::SrcAlpha,
            5 => wgpu::BlendFactor::OneMinusSrcAlpha,
            6 => wgpu::BlendFactor::Dst,
            7 => wgpu::BlendFactor::OneMinusDst,
            8 => wgpu::BlendFactor::DstAlpha,
            9 => wgpu::BlendFactor::OneMinusDstAlpha,
            10 => wgpu::BlendFactor::SrcAlphaSaturated,
            11 => wgpu::BlendFactor::Constant,
            12 => wgpu::BlendFactor::OneMinusConstant,
            _ => unreachable!(),
        }
    }
}

impl FixedSizeKey for wgpu::BlendFactor {
    fn fixed_size() -> u8 {
        4
    }
}

// blend operation
fn _check_blendoperation_variant_count(value: &wgpu::BlendOperation) {
    // this is to ensure we cover all variants. if the number of variants changes in future, the key size may need to be updated.
    // this will be possible robustly once https://github.com/rust-lang/rust/issues/73662 lands.
    // 5 variants => 3 bits
    match value {
        wgpu::BlendOperation::Add
        | wgpu::BlendOperation::Subtract
        | wgpu::BlendOperation::ReverseSubtract
        | wgpu::BlendOperation::Min
        | wgpu::BlendOperation::Max => (),
    }
}

impl AnyKeyType for wgpu::BlendOperation {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl KeyTypeConcrete for wgpu::BlendOperation {
    fn positions(_: &KeyMetaStore) -> HashMap<TypeId, SizeOffset> {
        HashMap::from_iter([(TypeId::of::<Self>(), SizeOffset(Self::fixed_size(), 0))])
    }

    fn size(_: &KeyMetaStore) -> u8 {
        Self::fixed_size()
    }

    fn pack(value: &Self, _: &KeyMetaStore) -> PackedPipelineKey<Self> {
        let raw = match value {
            wgpu::BlendOperation::Add => 0,
            wgpu::BlendOperation::Subtract => 1,
            wgpu::BlendOperation::ReverseSubtract => 2,
            wgpu::BlendOperation::Min => 3,
            wgpu::BlendOperation::Max => 4,
        };

        PackedPipelineKey::new(raw, Self::fixed_size())
    }

    fn unpack(value: KeyPrimitive, _: &KeyMetaStore) -> Self {
        match value {
            0 => wgpu::BlendOperation::Add,
            1 => wgpu::BlendOperation::Subtract,
            2 => wgpu::BlendOperation::ReverseSubtract,
            3 => wgpu::BlendOperation::Min,
            4 => wgpu::BlendOperation::Max,
            _ => unreachable!(),
        }
    }
}

impl FixedSizeKey for wgpu::BlendOperation {
    fn fixed_size() -> u8 {
        3
    }
}

impl AnyKeyType for wgpu::BlendComponent {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl KeyTypeConcrete for wgpu::BlendComponent {
    fn positions(store: &KeyMetaStore) -> HashMap<TypeId, SizeOffset> {
        HashMap::from_iter([(TypeId::of::<Self>(), SizeOffset(Self::size(store), 0u8))])
    }

    fn size(_: &KeyMetaStore) -> u8 {
        Self::fixed_size()
    }

    fn pack(value: &Self, store: &KeyMetaStore) -> PackedPipelineKey<Self> {
        let tuple = (value.src_factor, value.dst_factor, value.operation);
        let PackedPipelineKey { packed, size, .. } = KeyTypeConcrete::pack(&tuple, store);
        PackedPipelineKey::new(packed, size)
    }

    fn unpack(value: KeyPrimitive, store: &KeyMetaStore) -> Self {
        let (src_factor, dst_factor, operation) = KeyTypeConcrete::unpack(value, store);
        Self {
            src_factor,
            dst_factor,
            operation,
        }
    }
}

impl FixedSizeKey for wgpu::BlendComponent {
    fn fixed_size() -> u8 {
        wgpu::BlendFactor::fixed_size() * 2 + wgpu::BlendOperation::fixed_size()
    }
}

impl AnyKeyType for wgpu::BlendState {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl KeyTypeConcrete for wgpu::BlendState {
    fn positions(store: &KeyMetaStore) -> HashMap<TypeId, SizeOffset> {
        HashMap::from_iter([(TypeId::of::<Self>(), SizeOffset(Self::size(store), 0u8))])
    }

    fn size(_: &KeyMetaStore) -> u8 {
        Self::fixed_size()
    }

    fn pack(value: &Self, store: &KeyMetaStore) -> PackedPipelineKey<Self> {
        let tuple = (value.color, value.alpha);
        let PackedPipelineKey { packed, size, .. } = KeyTypeConcrete::pack(&tuple, store);
        PackedPipelineKey::new(packed, size)
    }

    fn unpack(value: KeyPrimitive, store: &KeyMetaStore) -> Self {
        let (color, alpha) = KeyTypeConcrete::unpack(value, store);
        Self { color, alpha }
    }
}

impl FixedSizeKey for wgpu::BlendState {
    fn fixed_size() -> u8 {
        wgpu::BlendComponent::fixed_size() * 2
    }
}

// AstcBlock
fn _check_astcblock_variant_count(value: &wgpu::AstcBlock) {
    // this is to ensure we cover all variants. if the number of variants changes in future, the key size may need to be updated.
    // this will be possible robustly once https://github.com/rust-lang/rust/issues/73662 lands.
    // 14 variants => 4 bits
    match value {
        wgpu::AstcBlock::B4x4
        | wgpu::AstcBlock::B5x4
        | wgpu::AstcBlock::B5x5
        | wgpu::AstcBlock::B6x5
        | wgpu::AstcBlock::B6x6
        | wgpu::AstcBlock::B8x5
        | wgpu::AstcBlock::B8x6
        | wgpu::AstcBlock::B8x8
        | wgpu::AstcBlock::B10x5
        | wgpu::AstcBlock::B10x6
        | wgpu::AstcBlock::B10x8
        | wgpu::AstcBlock::B10x10
        | wgpu::AstcBlock::B12x10
        | wgpu::AstcBlock::B12x12 => (),
    }
}

impl AnyKeyType for wgpu::AstcBlock {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl KeyTypeConcrete for wgpu::AstcBlock {
    fn positions(_: &KeyMetaStore) -> HashMap<TypeId, SizeOffset> {
        HashMap::from_iter([(TypeId::of::<Self>(), SizeOffset(Self::fixed_size(), 0))])
    }

    fn size(_: &KeyMetaStore) -> u8 {
        Self::fixed_size()
    }

    fn pack(value: &Self, _: &KeyMetaStore) -> PackedPipelineKey<Self> {
        let raw = match value {
            wgpu::AstcBlock::B4x4 => 0,
            wgpu::AstcBlock::B5x4 => 1,
            wgpu::AstcBlock::B5x5 => 2,
            wgpu::AstcBlock::B6x5 => 3,
            wgpu::AstcBlock::B6x6 => 4,
            wgpu::AstcBlock::B8x5 => 5,
            wgpu::AstcBlock::B8x6 => 6,
            wgpu::AstcBlock::B8x8 => 7,
            wgpu::AstcBlock::B10x5 => 8,
            wgpu::AstcBlock::B10x6 => 9,
            wgpu::AstcBlock::B10x8 => 10,
            wgpu::AstcBlock::B10x10 => 11,
            wgpu::AstcBlock::B12x10 => 12,
            wgpu::AstcBlock::B12x12 => 13,
        };

        PackedPipelineKey::new(raw, Self::fixed_size())
    }

    fn unpack(value: KeyPrimitive, _: &KeyMetaStore) -> Self {
        match value {
            0 => wgpu::AstcBlock::B4x4,
            1 => wgpu::AstcBlock::B5x4,
            2 => wgpu::AstcBlock::B5x5,
            3 => wgpu::AstcBlock::B6x5,
            4 => wgpu::AstcBlock::B6x6,
            5 => wgpu::AstcBlock::B8x5,
            6 => wgpu::AstcBlock::B8x6,
            7 => wgpu::AstcBlock::B8x8,
            8 => wgpu::AstcBlock::B10x5,
            9 => wgpu::AstcBlock::B10x6,
            10 => wgpu::AstcBlock::B10x8,
            11 => wgpu::AstcBlock::B10x10,
            12 => wgpu::AstcBlock::B12x10,
            13 => wgpu::AstcBlock::B12x12,
            _ => unreachable!(),
        }
    }
}

impl FixedSizeKey for wgpu::AstcBlock {
    fn fixed_size() -> u8 {
        4
    }
}

// AstcChannel
fn _check_astcchannel_variant_count(value: &wgpu::AstcChannel) {
    // this is to ensure we cover all variants. if the number of variants changes in future, the key size may need to be updated.
    // this will be possible robustly once https://github.com/rust-lang/rust/issues/73662 lands.
    // 3 variants => 2 bits
    match value {
        wgpu::AstcChannel::Unorm | wgpu::AstcChannel::UnormSrgb | wgpu::AstcChannel::Hdr => (),
    }
}

impl AnyKeyType for wgpu::AstcChannel {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl KeyTypeConcrete for wgpu::AstcChannel {
    fn positions(_: &KeyMetaStore) -> HashMap<TypeId, SizeOffset> {
        HashMap::from_iter([(TypeId::of::<Self>(), SizeOffset(Self::fixed_size(), 0))])
    }

    fn size(_: &KeyMetaStore) -> u8 {
        Self::fixed_size()
    }

    fn pack(value: &Self, _: &KeyMetaStore) -> PackedPipelineKey<Self> {
        let raw = match value {
            wgpu::AstcChannel::Unorm => 0,
            wgpu::AstcChannel::UnormSrgb => 1,
            wgpu::AstcChannel::Hdr => 2,
        };

        PackedPipelineKey::new(raw, Self::fixed_size())
    }

    fn unpack(value: KeyPrimitive, _: &KeyMetaStore) -> Self {
        match value {
            0 => wgpu::AstcChannel::Unorm,
            1 => wgpu::AstcChannel::UnormSrgb,
            2 => wgpu::AstcChannel::Hdr,
            _ => unreachable!(),
        }
    }
}

impl FixedSizeKey for wgpu::AstcChannel {
    fn fixed_size() -> u8 {
        2
    }
}

impl AnyKeyType for wgpu::TextureFormat {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

const WGPU_VARIANT_SIZE: u8 = 8;
impl KeyTypeConcrete for wgpu::TextureFormat {
    fn unpack(value: KeyPrimitive, store: &KeyMetaStore) -> Self {
        let variant = value & ((1 << WGPU_VARIANT_SIZE) - 1);
        let value = value >> WGPU_VARIANT_SIZE;
        let (block, channel) = KeyTypeConcrete::unpack(value, store);

        match variant {
            0 => wgpu::TextureFormat::R8Unorm,
            1 => wgpu::TextureFormat::R8Snorm,
            2 => wgpu::TextureFormat::R8Uint,
            3 => wgpu::TextureFormat::R8Sint,
            4 => wgpu::TextureFormat::R16Uint,
            5 => wgpu::TextureFormat::R16Sint,
            6 => wgpu::TextureFormat::R16Unorm,
            7 => wgpu::TextureFormat::R16Snorm,
            8 => wgpu::TextureFormat::R16Float,
            9 => wgpu::TextureFormat::Rg8Unorm,
            10 => wgpu::TextureFormat::Rg8Snorm,
            11 => wgpu::TextureFormat::Rg8Uint,
            12 => wgpu::TextureFormat::Rg8Sint,
            13 => wgpu::TextureFormat::R32Uint,
            14 => wgpu::TextureFormat::R32Sint,
            15 => wgpu::TextureFormat::R32Float,
            16 => wgpu::TextureFormat::Rg16Uint,
            17 => wgpu::TextureFormat::Rg16Sint,
            18 => wgpu::TextureFormat::Rg16Unorm,
            19 => wgpu::TextureFormat::Rg16Snorm,
            20 => wgpu::TextureFormat::Rg16Float,
            21 => wgpu::TextureFormat::Rgba8Unorm,
            22 => wgpu::TextureFormat::Rgba8UnormSrgb,
            23 => wgpu::TextureFormat::Rgba8Snorm,
            24 => wgpu::TextureFormat::Rgba8Uint,
            25 => wgpu::TextureFormat::Rgba8Sint,
            26 => wgpu::TextureFormat::Bgra8Unorm,
            27 => wgpu::TextureFormat::Bgra8UnormSrgb,
            28 => wgpu::TextureFormat::Rgb9e5Ufloat,
            29 => wgpu::TextureFormat::Rgb10a2Unorm,
            30 => wgpu::TextureFormat::Rg11b10Float,
            31 => wgpu::TextureFormat::Rg32Uint,
            32 => wgpu::TextureFormat::Rg32Sint,
            33 => wgpu::TextureFormat::Rg32Float,
            34 => wgpu::TextureFormat::Rgba16Uint,
            35 => wgpu::TextureFormat::Rgba16Sint,
            36 => wgpu::TextureFormat::Rgba16Unorm,
            37 => wgpu::TextureFormat::Rgba16Snorm,
            38 => wgpu::TextureFormat::Rgba16Float,
            39 => wgpu::TextureFormat::Rgba32Uint,
            40 => wgpu::TextureFormat::Rgba32Sint,
            41 => wgpu::TextureFormat::Rgba32Float,
            42 => wgpu::TextureFormat::Stencil8,
            43 => wgpu::TextureFormat::Depth16Unorm,
            44 => wgpu::TextureFormat::Depth24Plus,
            45 => wgpu::TextureFormat::Depth24PlusStencil8,
            46 => wgpu::TextureFormat::Depth32Float,
            47 => wgpu::TextureFormat::Depth32FloatStencil8,
            48 => wgpu::TextureFormat::Bc1RgbaUnorm,
            49 => wgpu::TextureFormat::Bc1RgbaUnormSrgb,
            50 => wgpu::TextureFormat::Bc2RgbaUnorm,
            51 => wgpu::TextureFormat::Bc2RgbaUnormSrgb,
            52 => wgpu::TextureFormat::Bc3RgbaUnorm,
            53 => wgpu::TextureFormat::Bc3RgbaUnormSrgb,
            54 => wgpu::TextureFormat::Bc4RUnorm,
            55 => wgpu::TextureFormat::Bc4RSnorm,
            56 => wgpu::TextureFormat::Bc5RgUnorm,
            57 => wgpu::TextureFormat::Bc5RgSnorm,
            58 => wgpu::TextureFormat::Bc6hRgbUfloat,
            59 => wgpu::TextureFormat::Bc6hRgbFloat,
            60 => wgpu::TextureFormat::Bc7RgbaUnorm,
            61 => wgpu::TextureFormat::Bc7RgbaUnormSrgb,
            62 => wgpu::TextureFormat::Etc2Rgb8Unorm,
            63 => wgpu::TextureFormat::Etc2Rgb8UnormSrgb,
            64 => wgpu::TextureFormat::Etc2Rgb8A1Unorm,
            65 => wgpu::TextureFormat::Etc2Rgb8A1UnormSrgb,
            66 => wgpu::TextureFormat::Etc2Rgba8Unorm,
            67 => wgpu::TextureFormat::Etc2Rgba8UnormSrgb,
            68 => wgpu::TextureFormat::EacR11Unorm,
            69 => wgpu::TextureFormat::EacR11Snorm,
            70 => wgpu::TextureFormat::EacRg11Unorm,
            71 => wgpu::TextureFormat::EacRg11Snorm,
            72 => wgpu::TextureFormat::Astc { block, channel },
            _ => unreachable!(),
        }
    }

    fn positions(_: &KeyMetaStore) -> HashMap<TypeId, SizeOffset> {
        HashMap::from_iter([(TypeId::of::<Self>(), SizeOffset(Self::fixed_size(), 0u8))])
    }

    fn size(_: &KeyMetaStore) -> u8 {
        Self::fixed_size()
    }

    fn pack(value: &Self, store: &KeyMetaStore) -> PackedPipelineKey<Self>
    where
        Self: Sized,
    {
        let variant = match value {
            wgpu::TextureFormat::R8Unorm => 0,
            wgpu::TextureFormat::R8Snorm => 1,
            wgpu::TextureFormat::R8Uint => 2,
            wgpu::TextureFormat::R8Sint => 3,
            wgpu::TextureFormat::R16Uint => 4,
            wgpu::TextureFormat::R16Sint => 5,
            wgpu::TextureFormat::R16Unorm => 6,
            wgpu::TextureFormat::R16Snorm => 7,
            wgpu::TextureFormat::R16Float => 8,
            wgpu::TextureFormat::Rg8Unorm => 9,
            wgpu::TextureFormat::Rg8Snorm => 10,
            wgpu::TextureFormat::Rg8Uint => 11,
            wgpu::TextureFormat::Rg8Sint => 12,
            wgpu::TextureFormat::R32Uint => 13,
            wgpu::TextureFormat::R32Sint => 14,
            wgpu::TextureFormat::R32Float => 15,
            wgpu::TextureFormat::Rg16Uint => 16,
            wgpu::TextureFormat::Rg16Sint => 17,
            wgpu::TextureFormat::Rg16Unorm => 18,
            wgpu::TextureFormat::Rg16Snorm => 19,
            wgpu::TextureFormat::Rg16Float => 20,
            wgpu::TextureFormat::Rgba8Unorm => 21,
            wgpu::TextureFormat::Rgba8UnormSrgb => 22,
            wgpu::TextureFormat::Rgba8Snorm => 23,
            wgpu::TextureFormat::Rgba8Uint => 24,
            wgpu::TextureFormat::Rgba8Sint => 25,
            wgpu::TextureFormat::Bgra8Unorm => 26,
            wgpu::TextureFormat::Bgra8UnormSrgb => 27,
            wgpu::TextureFormat::Rgb9e5Ufloat => 28,
            wgpu::TextureFormat::Rgb10a2Unorm => 29,
            wgpu::TextureFormat::Rg11b10Float => 30,
            wgpu::TextureFormat::Rg32Uint => 31,
            wgpu::TextureFormat::Rg32Sint => 32,
            wgpu::TextureFormat::Rg32Float => 33,
            wgpu::TextureFormat::Rgba16Uint => 34,
            wgpu::TextureFormat::Rgba16Sint => 35,
            wgpu::TextureFormat::Rgba16Unorm => 36,
            wgpu::TextureFormat::Rgba16Snorm => 37,
            wgpu::TextureFormat::Rgba16Float => 38,
            wgpu::TextureFormat::Rgba32Uint => 39,
            wgpu::TextureFormat::Rgba32Sint => 40,
            wgpu::TextureFormat::Rgba32Float => 41,
            wgpu::TextureFormat::Stencil8 => 42,
            wgpu::TextureFormat::Depth16Unorm => 43,
            wgpu::TextureFormat::Depth24Plus => 44,
            wgpu::TextureFormat::Depth24PlusStencil8 => 45,
            wgpu::TextureFormat::Depth32Float => 46,
            wgpu::TextureFormat::Depth32FloatStencil8 => 47,
            wgpu::TextureFormat::Bc1RgbaUnorm => 48,
            wgpu::TextureFormat::Bc1RgbaUnormSrgb => 49,
            wgpu::TextureFormat::Bc2RgbaUnorm => 50,
            wgpu::TextureFormat::Bc2RgbaUnormSrgb => 51,
            wgpu::TextureFormat::Bc3RgbaUnorm => 52,
            wgpu::TextureFormat::Bc3RgbaUnormSrgb => 53,
            wgpu::TextureFormat::Bc4RUnorm => 54,
            wgpu::TextureFormat::Bc4RSnorm => 55,
            wgpu::TextureFormat::Bc5RgUnorm => 56,
            wgpu::TextureFormat::Bc5RgSnorm => 57,
            wgpu::TextureFormat::Bc6hRgbUfloat => 58,
            wgpu::TextureFormat::Bc6hRgbFloat => 59,
            wgpu::TextureFormat::Bc7RgbaUnorm => 60,
            wgpu::TextureFormat::Bc7RgbaUnormSrgb => 61,
            wgpu::TextureFormat::Etc2Rgb8Unorm => 62,
            wgpu::TextureFormat::Etc2Rgb8UnormSrgb => 63,
            wgpu::TextureFormat::Etc2Rgb8A1Unorm => 64,
            wgpu::TextureFormat::Etc2Rgb8A1UnormSrgb => 65,
            wgpu::TextureFormat::Etc2Rgba8Unorm => 66,
            wgpu::TextureFormat::Etc2Rgba8UnormSrgb => 67,
            wgpu::TextureFormat::EacR11Unorm => 68,
            wgpu::TextureFormat::EacR11Snorm => 69,
            wgpu::TextureFormat::EacRg11Unorm => 70,
            wgpu::TextureFormat::EacRg11Snorm => 71,
            wgpu::TextureFormat::Astc { .. } => 72,
        };

        let (block, channel) = match value {
            wgpu::TextureFormat::Astc { block, channel } => (*block, *channel),
            _ => (wgpu::AstcBlock::B4x4, wgpu::AstcChannel::Unorm), // doesn't matter, won't be read
        };

        let packed_block_channel = KeyTypeConcrete::pack(&(block, channel), store);
        let packed = (packed_block_channel.packed << WGPU_VARIANT_SIZE) | variant;
        PackedPipelineKey {
            packed,
            size: WGPU_VARIANT_SIZE + packed_block_channel.size,
            _p: PhantomData,
        }
    }
}

impl FixedSizeKey for wgpu::TextureFormat {
    fn fixed_size() -> u8 {
        14 + wgpu::AstcBlock::fixed_size() + wgpu::AstcChannel::fixed_size()
    }
}

// PrimitiveTopology
fn _check_primitivetopology_variant_count(value: &wgpu::PrimitiveTopology) {
    // this is to ensure we cover all variants. if the number of variants changes in future, the key size may need to be updated.
    // this will be possible robustly once https://github.com/rust-lang/rust/issues/73662 lands.
    // 5 variants => 3 bits
    match value {
        wgpu::PrimitiveTopology::PointList
        | wgpu::PrimitiveTopology::LineList
        | wgpu::PrimitiveTopology::LineStrip
        | wgpu::PrimitiveTopology::TriangleList
        | wgpu::PrimitiveTopology::TriangleStrip => (),
    }
}

impl AnyKeyType for wgpu::PrimitiveTopology {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl KeyTypeConcrete for wgpu::PrimitiveTopology {
    fn positions(_: &KeyMetaStore) -> HashMap<TypeId, SizeOffset> {
        HashMap::from_iter([(TypeId::of::<Self>(), SizeOffset(Self::fixed_size(), 0))])
    }

    fn size(_: &KeyMetaStore) -> u8 {
        Self::fixed_size()
    }

    fn pack(value: &Self, _: &KeyMetaStore) -> PackedPipelineKey<Self> {
        let raw = match value {
            wgpu::PrimitiveTopology::PointList => 0,
            wgpu::PrimitiveTopology::LineList => 1,
            wgpu::PrimitiveTopology::LineStrip => 2,
            wgpu::PrimitiveTopology::TriangleList => 3,
            wgpu::PrimitiveTopology::TriangleStrip => 4,
        };

        PackedPipelineKey::new(raw, Self::fixed_size())
    }

    fn unpack(value: KeyPrimitive, _: &KeyMetaStore) -> Self {
        match value {
            0 => wgpu::PrimitiveTopology::PointList,
            1 => wgpu::PrimitiveTopology::LineList,
            2 => wgpu::PrimitiveTopology::LineStrip,
            3 => wgpu::PrimitiveTopology::TriangleList,
            4 => wgpu::PrimitiveTopology::TriangleStrip,
            _ => unreachable!(),
        }
    }
}

impl FixedSizeKey for wgpu::PrimitiveTopology {
    fn fixed_size() -> u8 {
        3
    }
}

// Face
fn _check_face_variant_count(value: &wgpu::Face) {
    // this is to ensure we cover all variants. if the number of variants changes in future, the key size may need to be updated.
    // this will be possible robustly once https://github.com/rust-lang/rust/issues/73662 lands.
    // 2 variants => 1 bits
    match value {
        wgpu::Face::Front | wgpu::Face::Back => (),
    }
}

impl AnyKeyType for wgpu::Face {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl KeyTypeConcrete for wgpu::Face {
    fn positions(_: &KeyMetaStore) -> HashMap<TypeId, SizeOffset> {
        HashMap::from_iter([(TypeId::of::<Self>(), SizeOffset(Self::fixed_size(), 0))])
    }

    fn size(_: &KeyMetaStore) -> u8 {
        Self::fixed_size()
    }

    fn pack(value: &Self, _: &KeyMetaStore) -> PackedPipelineKey<Self> {
        let raw = match value {
            wgpu::Face::Front => 0,
            wgpu::Face::Back => 1,
        };

        PackedPipelineKey::new(raw, Self::fixed_size())
    }

    fn unpack(value: KeyPrimitive, _: &KeyMetaStore) -> Self {
        match value {
            0 => wgpu::Face::Front,
            1 => wgpu::Face::Back,
            _ => unreachable!(),
        }
    }
}

impl FixedSizeKey for wgpu::Face {
    fn fixed_size() -> u8 {
        1
    }
}

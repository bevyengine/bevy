// NOTE: These are currently just copies of the wgpu types, but they might change in the future

#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq, Ord, PartialOrd)]
pub enum TextureViewDimension {
    D1,
    D2,
    D2Array,
    Cube,
    CubeArray,
    D3,
}

#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq)]
pub enum TextureDimension {
    D1,
    D2,
    D3,
}

// TODO: use math type here
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Extent3d {
    pub width: u32,
    pub height: u32,
    pub depth: u32,
}

#[derive(Copy, Hash, Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum TextureComponentType {
    Float,
    Sint,
    Uint,
}

#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq, Ord, PartialOrd)]
pub enum TextureFormat {
    // Normal 8 bit formats
    R8Unorm = 0,
    R8Snorm = 1,
    R8Uint = 2,
    R8Sint = 3,

    // Normal 16 bit formats
    R16Uint = 4,
    R16Sint = 5,
    R16Float = 6,
    Rg8Unorm = 7,
    Rg8Snorm = 8,
    Rg8Uint = 9,
    Rg8Sint = 10,

    // Normal 32 bit formats
    R32Uint = 11,
    R32Sint = 12,
    R32Float = 13,
    Rg16Uint = 14,
    Rg16Sint = 15,
    Rg16Float = 16,
    Rgba8Unorm = 17,
    Rgba8UnormSrgb = 18,
    Rgba8Snorm = 19,
    Rgba8Uint = 20,
    Rgba8Sint = 21,
    Bgra8Unorm = 22,
    Bgra8UnormSrgb = 23,

    // Packed 32 bit formats
    Rgb10a2Unorm = 24,
    Rg11b10Float = 25,

    // Normal 64 bit formats
    Rg32Uint = 26,
    Rg32Sint = 27,
    Rg32Float = 28,
    Rgba16Uint = 29,
    Rgba16Sint = 30,
    Rgba16Float = 31,

    // Normal 128 bit formats
    Rgba32Uint = 32,
    Rgba32Sint = 33,
    Rgba32Float = 34,

    // Depth and stencil formats
    Depth32Float = 35,
    Depth24Plus = 36,
    Depth24PlusStencil8 = 37,
}

bitflags::bitflags! {
    #[repr(transparent)]
    pub struct TextureUsage: u32 {
        const COPY_SRC = 1;
        const COPY_DST = 2;
        const SAMPLED = 4;
        const STORAGE = 8;
        const OUTPUT_ATTACHMENT = 16;
        const NONE = 0;
        /// The combination of all read-only usages.
        const READ_ALL = Self::COPY_SRC.bits | Self::SAMPLED.bits;
        /// The combination of all write-only and read-write usages.
        const WRITE_ALL = Self::COPY_DST.bits | Self::STORAGE.bits | Self::OUTPUT_ATTACHMENT.bits;
        /// The combination of all usages that the are guaranteed to be be ordered by the hardware.
        /// If a usage is not ordered, then even if it doesn't change between draw calls, there
        /// still need to be pipeline barriers inserted for synchronization.
        const ORDERED = Self::READ_ALL.bits | Self::OUTPUT_ATTACHMENT.bits;
        const UNINITIALIZED = 0xFFFF;
    }
}

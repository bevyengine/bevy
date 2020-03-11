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


#[derive(Copy, Clone, Debug, Hash)]
pub enum TextureDimension {
    D1,
    D2,
    D3,
}

#[derive(Clone, Copy, Debug)]
pub struct Extent3d {
    pub width: u32,
    pub height: u32,
    pub depth: u32,
}

#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq)]
pub enum TextureFormat {
    // Normal 8 bit formats
    R8Unorm = 0,
    R8Snorm = 1,
    R8Uint = 2,
    R8Sint = 3,

    // Normal 16 bit formats
    R16Unorm = 4,
    R16Snorm = 5,
    R16Uint = 6,
    R16Sint = 7,
    R16Float = 8,

    Rg8Unorm = 9,
    Rg8Snorm = 10,
    Rg8Uint = 11,
    Rg8Sint = 12,

    // Normal 32 bit formats
    R32Uint = 13,
    R32Sint = 14,
    R32Float = 15,
    Rg16Unorm = 16,
    Rg16Snorm = 17,
    Rg16Uint = 18,
    Rg16Sint = 19,
    Rg16Float = 20,
    Rgba8Unorm = 21,
    Rgba8UnormSrgb = 22,
    Rgba8Snorm = 23,
    Rgba8Uint = 24,
    Rgba8Sint = 25,
    Bgra8Unorm = 26,
    Bgra8UnormSrgb = 27,

    // Packed 32 bit formats
    Rgb10a2Unorm = 28,
    Rg11b10Float = 29,

    // Normal 64 bit formats
    Rg32Uint = 30,
    Rg32Sint = 31,
    Rg32Float = 32,
    Rgba16Unorm = 33,
    Rgba16Snorm = 34,
    Rgba16Uint = 35,
    Rgba16Sint = 36,
    Rgba16Float = 37,

    // Normal 128 bit formats
    Rgba32Uint = 38,
    Rgba32Sint = 39,
    Rgba32Float = 40,

    // Depth and stencil formats
    Depth32Float = 41,
    Depth24Plus = 42,
    Depth24PlusStencil8 = 43,
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
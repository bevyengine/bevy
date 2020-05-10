#[cfg(feature = "shader_alignment_32_bits")]
pub type AlignedU8 = u32;
#[cfg(not(feature = "shader_alignment_32_bits"))]
pub type AlignedU8 = u8;

#[cfg(feature = "shader_alignment_32_bits")]
pub type AlignedU16 = u32;
#[cfg(not(feature = "shader_alignment_32_bits"))]
pub type AlignedU16 = u16;

#[cfg(feature = "shader_alignment_32_bits")]
pub type AlignedI8 = i32;
#[cfg(not(feature = "shader_alignment_32_bits"))]
pub type AlignedI8 = i8;

#[cfg(feature = "shader_alignment_32_bits")]
pub type AlignedI16 = i32;
#[cfg(not(feature = "shader_alignment_32_bits"))]
pub type AlignedI16 = i16;
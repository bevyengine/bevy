//! [`GpuElement`]: records that can be copied to a buffer as they are.

use alloc::vec::Vec;

use super::{
    encase::{
        internal::{AlignmentValue, WriteInto},
        StorageBuffer, UniformBuffer,
    },
    ShaderSize,
};
use bytemuck::Pod;

/// A record type whose Rust layout is byte-for-byte its shader layout, so a
/// `#[repr(C)]` value can be copied into a buffer as is.
///
/// Implemented for every [`Pod`] type that derives [`ShaderType`](super::ShaderType).
/// Nothing checks that on its own: call [`assert_gpu_layout!`](crate::assert_gpu_layout) for each record type.
pub trait GpuElement: Pod + ShaderSize + WriteInto {}

impl<T: Pod + ShaderSize + WriteInto> GpuElement for T {}

/// Panics in debug builds if `T`'s bytes differ from its std430 (storage buffer) layout.
pub fn debug_assert_std430<T: GpuElement>() {
    if cfg!(debug_assertions) {
        let (pattern, encased) = encase_sample::<T>(|value| {
            let mut buffer = StorageBuffer::new(Vec::<u8>::new());
            buffer.write(value).unwrap();
            buffer.into_inner()
        });
        assert_matches::<T>(&pattern, &encased, "std430");
    }
}

/// Panics in debug builds if `T`'s bytes differ from its std140 (uniform buffer) layout,
/// or if `T` isn't valid in the uniform address space.
pub fn debug_assert_std140<T: GpuElement>() {
    if cfg!(debug_assertions) {
        let (pattern, encased) = encase_sample::<T>(|value| {
            let mut buffer = UniformBuffer::new(Vec::<u8>::new());
            buffer.write(value).unwrap();
            buffer.into_inner()
        });
        assert_matches::<T>(&pattern, &encased, "std140");
    }
}

/// A `T` made of distinct, nonzero bytes, so any moved field or padding changes the output
fn encase_sample<T: GpuElement>(encase: impl FnOnce(&T) -> Vec<u8>) -> (Vec<u8>, Vec<u8>) {
    let pattern: Vec<u8> = (0..size_of::<T>()).map(|i| (i % 251) as u8 + 1).collect();
    let encased = encase(&bytemuck::pod_read_unaligned(&pattern));
    (pattern, encased)
}

fn assert_matches<T>(pattern: &[u8], encased: &[u8], layout: &str) {
    assert!(
        encased == pattern,
        "{} doesn't match its {layout} layout: its fields are padded or placed differently in the shader",
        core::any::type_name::<T>(),
    );
}

/// Fails to compile if `T`'s size isn't its std430 size.
pub const fn assert_std430_size<T: GpuElement>() {
    assert!(
        size_of::<T>() as u64 == T::SHADER_SIZE.get(),
        "size differs from the std430 layout: add or remove padding"
    );
}

/// Fails to compile if `T`'s size isn't its std140 size, which rounds up to 16 bytes for structs.
pub const fn assert_std140_size<T: GpuElement>() {
    let natural = T::METADATA.alignment().get();
    let align = match T::METADATA.uniform_min_alignment() {
        Some(min) if min.get() > natural => min.get(),
        _ => natural,
    };
    assert!(
        size_of::<T>() as u64 == AlignmentValue::new(align).round_up(T::SHADER_SIZE.get()),
        "size differs from the std140 layout: add or remove padding"
    );
}

/// Checks that types have the same layout in Rust and in a shader.
///
/// Fails to compile if a size differs, and adds a test called `{name}` that checks
/// every field against encase's layout ([`debug_assert_std430`] or [`debug_assert_std140`]).
///
/// ```ignore
/// #[derive(Clone, Copy, Pod, Zeroable, ShaderType)]
/// #[repr(C)]
/// struct Light { position: Vec3, intensity: f32 }
///
/// assert_gpu_layout!(light_layout, std430, Light);
/// ```
#[macro_export]
macro_rules! assert_gpu_layout {
    ($name:ident, std430, $($ty:ty),+ $(,)?) => {
        $crate::assert_gpu_layout!(@ $name, assert_std430_size, debug_assert_std430, $($ty),+);
    };
    ($name:ident, std140, $($ty:ty),+ $(,)?) => {
        $crate::assert_gpu_layout!(@ $name, assert_std140_size, debug_assert_std140, $($ty),+);
    };
    (@ $name:ident, $size:ident, $exact:ident, $($ty:ty),+) => {
        const _: () = { $($crate::render_resource::$size::<$ty>();)+ };

        #[cfg(test)]
        #[test]
        fn $name() {
            $($crate::render_resource::$exact::<$ty>();)+
        }
    };
}

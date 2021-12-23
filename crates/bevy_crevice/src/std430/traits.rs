use core::mem::{size_of, MaybeUninit};
#[cfg(feature = "std")]
use std::io::{self, Write};

use bytemuck::{bytes_of, Pod, Zeroable};

#[cfg(feature = "std")]
use crate::std430::Writer;

/// Trait implemented for all `std430` primitives. Generally should not be
/// implemented outside this crate.
pub unsafe trait Std430: Copy + Zeroable + Pod {
    /// The required alignment of the type. Must be a power of two.
    ///
    /// This is distinct from the value returned by `std::mem::align_of` because
    /// `AsStd430` structs do not use Rust's alignment. This enables them to
    /// control and zero their padding bytes, making converting them to and from
    /// slices safe.
    const ALIGNMENT: usize;

    /// Whether this type requires a padding at the end (ie, is a struct or an array
    /// of primitives).
    /// See <https://www.khronos.org/registry/OpenGL/specs/gl/glspec45.core.pdf#page=159>
    /// (rule 4 and 9)
    const PAD_AT_END: bool = false;
    /// Padded type (Std430Padded specialization)
    /// The usual implementation is
    /// type Padded = Std430Padded<Self, {align_offset(size_of::<Self>(), ALIGNMENT)}>;
    type Padded: Std430Convertible<Self>;

    /// Casts the type to a byte array. Implementors should not override this
    /// method.
    ///
    /// # Safety
    /// This is always safe due to the requirements of [`bytemuck::Pod`] being a
    /// prerequisite for this trait.
    fn as_bytes(&self) -> &[u8] {
        bytes_of(self)
    }
}

/// Trait specifically for Std430::Padded, implements conversions between padded type and base type.
pub trait Std430Convertible<T: Std430>: Copy {
    /// Convert from self to Std430
    fn into_std430(self) -> T;
    /// Convert from Std430 to self
    fn from_std430(_: T) -> Self;
}

impl<T: Std430> Std430Convertible<T> for T {
    fn into_std430(self) -> T {
        self
    }
    fn from_std430(also_self: T) -> Self {
        also_self
    }
}

/// Unfortunately, we cannot easily derive padded representation for generic Std140 types.
/// For now, we'll just use this empty enum with no values.
#[derive(Copy, Clone)]
pub enum InvalidPadded {}
impl<T: Std430> Std430Convertible<T> for InvalidPadded {
    fn into_std430(self) -> T {
        unimplemented!()
    }
    fn from_std430(_: T) -> Self {
        unimplemented!()
    }
}
/**
Trait implemented for all types that can be turned into `std430` values.

This trait can often be `#[derive]`'d instead of manually implementing it. Any
struct which contains only fields that also implement `AsStd430` can derive
`AsStd430`.

Types from the mint crate implement `AsStd430`, making them convenient for use
in uniform types. Most Rust geometry crates, like cgmath, nalgebra, and
ultraviolet support mint.

## Example

```glsl
uniform CAMERA {
    mat4 view;
    mat4 projection;
} camera;
```

```no_run
use bevy_crevice::std430::{AsStd430, Std430};

#[derive(AsStd430)]
struct CameraUniform {
    view: mint::ColumnMatrix4<f32>,
    projection: mint::ColumnMatrix4<f32>,
}

let view: mint::ColumnMatrix4<f32> = todo!("your math code here");
let projection: mint::ColumnMatrix4<f32> = todo!("your math code here");

let camera = CameraUniform {
    view,
    projection,
};

# fn write_to_gpu_buffer(bytes: &[u8]) {}
let camera_std430 = camera.as_std430();
write_to_gpu_buffer(camera_std430.as_bytes());
```
*/
pub trait AsStd430 {
    /// The `std430` version of this value.
    type Output: Std430;

    /// Convert this value into the `std430` version of itself.
    fn as_std430(&self) -> Self::Output;

    /// Returns the size of the `std430` version of this type. Useful for
    /// pre-sizing buffers.
    fn std430_size_static() -> usize {
        size_of::<Self::Output>()
    }

    /// Converts from `std430` version of self to self.
    fn from_std430(value: Self::Output) -> Self;
}

impl<T> AsStd430 for T
where
    T: Std430,
{
    type Output = Self;

    fn as_std430(&self) -> Self {
        *self
    }

    fn from_std430(value: Self) -> Self {
        value
    }
}

#[doc(hidden)]
#[derive(Copy, Clone, Debug)]
pub struct Std430Padded<T: Std430, const PAD: usize> {
    inner: T,
    _padding: [u8; PAD],
}

unsafe impl<T: Std430, const PAD: usize> Zeroable for Std430Padded<T, PAD> {}
unsafe impl<T: Std430, const PAD: usize> Pod for Std430Padded<T, PAD> {}

impl<T: Std430, const PAD: usize> Std430Convertible<T> for Std430Padded<T, PAD> {
    fn into_std430(self) -> T {
        self.inner
    }

    fn from_std430(inner: T) -> Self {
        Self {
            inner,
            _padding: [0u8; PAD],
        }
    }
}

#[doc(hidden)]
#[derive(Copy, Clone, Debug)]
#[repr(transparent)]
pub struct Std430Array<T: Std430, const N: usize>([T::Padded; N]);

unsafe impl<T: Std430, const N: usize> Zeroable for Std430Array<T, N> where T::Padded: Zeroable {}
unsafe impl<T: Std430, const N: usize> Pod for Std430Array<T, N> where T::Padded: Pod {}
unsafe impl<T: Std430, const N: usize> Std430 for Std430Array<T, N>
where
    T::Padded: Pod,
{
    const ALIGNMENT: usize = T::ALIGNMENT;
    type Padded = Self;
}

impl<T: Std430, const N: usize> Std430Array<T, N> {
    fn uninit_array() -> [MaybeUninit<T::Padded>; N] {
        unsafe { MaybeUninit::uninit().assume_init() }
    }

    fn from_uninit_array(a: [MaybeUninit<T::Padded>; N]) -> Self {
        unsafe { core::mem::transmute_copy(&a) }
    }
}

impl<T: AsStd430, const N: usize> AsStd430 for [T; N]
where
    <T::Output as Std430>::Padded: Pod,
{
    type Output = Std430Array<T::Output, N>;
    fn as_std430(&self) -> Self::Output {
        let mut res = Self::Output::uninit_array();

        for i in 0..N {
            res[i] = MaybeUninit::new(Std430Convertible::from_std430(self[i].as_std430()));
        }

        Self::Output::from_uninit_array(res)
    }

    fn from_std430(val: Self::Output) -> Self {
        let mut res: [MaybeUninit<T>; N] = unsafe { MaybeUninit::uninit().assume_init() };
        for i in 0..N {
            res[i] = MaybeUninit::new(T::from_std430(val.0[i].into_std430()));
        }
        unsafe { core::mem::transmute_copy(&res) }
    }
}

/// Trait implemented for all types that can be written into a buffer as
/// `std430` bytes. This type is more general than [`AsStd430`]: all `AsStd430`
/// types implement `WriteStd430`, but not the other way around.
///
/// While `AsStd430` requires implementers to return a type that implements the
/// `Std430` trait, `WriteStd430` directly writes bytes using a [`Writer`]. This
/// makes `WriteStd430` usable for writing slices or other DSTs that could not
/// implement `AsStd430` without allocating new memory on the heap.
#[cfg(feature = "std")]
pub trait WriteStd430 {
    /// Writes this value into the given [`Writer`] using `std430` layout rules.
    ///
    /// Should return the offset of the first byte of this type, as returned by
    /// the first call to [`Writer::write`].
    fn write_std430<W: Write>(&self, writer: &mut Writer<W>) -> io::Result<usize>;

    /// The space required to write this value using `std430` layout rules. This
    /// does not include alignment padding that may be needed before or after
    /// this type when written as part of a larger buffer.
    fn std430_size(&self) -> usize {
        let mut writer = Writer::new(io::sink());
        self.write_std430(&mut writer).unwrap();
        writer.len()
    }
}

#[cfg(feature = "std")]
impl<T> WriteStd430 for T
where
    T: AsStd430,
{
    fn write_std430<W: Write>(&self, writer: &mut Writer<W>) -> io::Result<usize> {
        writer.write_std430(&self.as_std430())
    }

    fn std430_size(&self) -> usize {
        size_of::<<Self as AsStd430>::Output>()
    }
}

#[cfg(feature = "std")]
impl<T> WriteStd430 for [T]
where
    T: WriteStd430,
{
    fn write_std430<W: Write>(&self, writer: &mut Writer<W>) -> io::Result<usize> {
        let mut offset = writer.len();

        let mut iter = self.iter();

        if let Some(item) = iter.next() {
            offset = item.write_std430(writer)?;
        }

        for item in self.iter() {
            item.write_std430(writer)?;
        }

        Ok(offset)
    }

    fn std430_size(&self) -> usize {
        let mut writer = Writer::new(io::sink());
        self.write_std430(&mut writer).unwrap();
        writer.len()
    }
}

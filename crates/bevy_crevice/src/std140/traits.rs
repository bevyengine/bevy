use core::mem::{size_of, MaybeUninit};
#[cfg(feature = "std")]
use std::io::{self, Write};

use bytemuck::{bytes_of, Pod, Zeroable};

#[cfg(feature = "std")]
use crate::std140::Writer;

/// Trait implemented for all `std140` primitives. Generally should not be
/// implemented outside this crate.
pub unsafe trait Std140: Copy + Zeroable + Pod {
    /// The required alignment of the type. Must be a power of two.
    ///
    /// This is distinct from the value returned by `std::mem::align_of` because
    /// `AsStd140` structs do not use Rust's alignment. This enables them to
    /// control and zero their padding bytes, making converting them to and from
    /// slices safe.
    const ALIGNMENT: usize;

    /// Whether this type requires a padding at the end (ie, is a struct or an array
    /// of primitives).
    /// See <https://www.khronos.org/registry/OpenGL/specs/gl/glspec45.core.pdf#page=159>
    /// (rule 4 and 9)
    const PAD_AT_END: bool = false;
    /// Padded type (Std140Padded specialization)
    /// The usual implementation is
    /// type Padded = Std140Padded<Self, {align_offset(size_of::<Self>(), max(16, ALIGNMENT))}>;
    type Padded: Std140Convertible<Self>;

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

/// Trait specifically for Std140::Padded, implements conversions between padded type and base type.
pub trait Std140Convertible<T: Std140>: Copy {
    /// Convert from self to Std140
    fn into_std140(self) -> T;
    /// Convert from Std140 to self
    fn from_std140(_: T) -> Self;
}

impl<T: Std140> Std140Convertible<T> for T {
    fn into_std140(self) -> T {
        self
    }
    fn from_std140(also_self: T) -> Self {
        also_self
    }
}

/// Unfortunately, we cannot easily derive padded representation for generic Std140 types.
/// For now, we'll just use this empty enum with no values.
#[derive(Copy, Clone)]
pub enum InvalidPadded {}
impl<T: Std140> Std140Convertible<T> for InvalidPadded {
    fn into_std140(self) -> T {
        unimplemented!()
    }
    fn from_std140(_: T) -> Self {
        unimplemented!()
    }
}
/**
Trait implemented for all types that can be turned into `std140` values.
*
This trait can often be `#[derive]`'d instead of manually implementing it. Any
struct which contains only fields that also implement `AsStd140` can derive
`AsStd140`.

Types from the mint crate implement `AsStd140`, making them convenient for use
in uniform types. Most Rust math crates, like cgmath, nalgebra, and
ultraviolet support mint.

## Example

```glsl
uniform CAMERA {
    mat4 view;
    mat4 projection;
} camera;
```

```no_run
use bevy_crevice::std140::{AsStd140, Std140};

#[derive(AsStd140)]
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
let camera_std140 = camera.as_std140();
write_to_gpu_buffer(camera_std140.as_bytes());
```
*/
pub trait AsStd140 {
    /// The `std140` version of this value.
    type Output: Std140;

    /// Convert this value into the `std140` version of itself.
    fn as_std140(&self) -> Self::Output;

    /// Returns the size of the `std140` version of this type. Useful for
    /// pre-sizing buffers.
    fn std140_size_static() -> usize {
        size_of::<Self::Output>()
    }

    /// Converts from `std140` version of self to self.
    fn from_std140(val: Self::Output) -> Self;
}

impl<T> AsStd140 for T
where
    T: Std140,
{
    type Output = Self;

    fn as_std140(&self) -> Self {
        *self
    }

    fn from_std140(x: Self) -> Self {
        x
    }
}

#[doc(hidden)]
#[derive(Copy, Clone, Debug)]
pub struct Std140Padded<T: Std140, const PAD: usize> {
    inner: T,
    _padding: [u8; PAD],
}

unsafe impl<T: Std140, const PAD: usize> Zeroable for Std140Padded<T, PAD> {}
unsafe impl<T: Std140, const PAD: usize> Pod for Std140Padded<T, PAD> {}

impl<T: Std140, const PAD: usize> Std140Convertible<T> for Std140Padded<T, PAD> {
    fn into_std140(self) -> T {
        self.inner
    }

    fn from_std140(inner: T) -> Self {
        Self {
            inner,
            _padding: [0u8; PAD],
        }
    }
}

#[doc(hidden)]
#[derive(Copy, Clone, Debug)]
#[repr(transparent)]
pub struct Std140Array<T: Std140, const N: usize>([T::Padded; N]);

unsafe impl<T: Std140, const N: usize> Zeroable for Std140Array<T, N> where T::Padded: Zeroable {}
unsafe impl<T: Std140, const N: usize> Pod for Std140Array<T, N> where T::Padded: Pod {}
unsafe impl<T: Std140, const N: usize> Std140 for Std140Array<T, N>
where
    T::Padded: Pod,
{
    const ALIGNMENT: usize = crate::internal::max(T::ALIGNMENT, 16);
    type Padded = Self;
}

impl<T: Std140, const N: usize> Std140Array<T, N> {
    fn uninit_array() -> [MaybeUninit<T::Padded>; N] {
        unsafe { MaybeUninit::uninit().assume_init() }
    }

    fn from_uninit_array(a: [MaybeUninit<T::Padded>; N]) -> Self {
        unsafe { core::mem::transmute_copy(&a) }
    }
}

impl<T: AsStd140, const N: usize> AsStd140 for [T; N]
where
    <T::Output as Std140>::Padded: Pod,
{
    type Output = Std140Array<T::Output, N>;
    fn as_std140(&self) -> Self::Output {
        let mut res = Self::Output::uninit_array();

        for i in 0..N {
            res[i] = MaybeUninit::new(Std140Convertible::from_std140(self[i].as_std140()));
        }

        Self::Output::from_uninit_array(res)
    }

    fn from_std140(val: Self::Output) -> Self {
        let mut res: [MaybeUninit<T>; N] = unsafe { MaybeUninit::uninit().assume_init() };
        for i in 0..N {
            res[i] = MaybeUninit::new(T::from_std140(Std140Convertible::into_std140(val.0[i])));
        }
        unsafe { core::mem::transmute_copy(&res) }
    }
}

/// Trait implemented for all types that can be written into a buffer as
/// `std140` bytes. This type is more general than [`AsStd140`]: all `AsStd140`
/// types implement `WriteStd140`, but not the other way around.
///
/// While `AsStd140` requires implementers to return a type that implements the
/// `Std140` trait, `WriteStd140` directly writes bytes using a [`Writer`]. This
/// makes `WriteStd140` usable for writing slices or other DSTs that could not
/// implement `AsStd140` without allocating new memory on the heap.
#[cfg(feature = "std")]
pub trait WriteStd140 {
    /// Writes this value into the given [`Writer`] using `std140` layout rules.
    ///
    /// Should return the offset of the first byte of this type, as returned by
    /// the first call to [`Writer::write`].
    fn write_std140<W: Write>(&self, writer: &mut Writer<W>) -> io::Result<usize>;

    /// The space required to write this value using `std140` layout rules. This
    /// does not include alignment padding that may be needed before or after
    /// this type when written as part of a larger buffer.
    fn std140_size(&self) -> usize {
        let mut writer = Writer::new(io::sink());
        self.write_std140(&mut writer).unwrap();
        writer.len()
    }
}

#[cfg(feature = "std")]
impl<T> WriteStd140 for T
where
    T: AsStd140,
{
    fn write_std140<W: Write>(&self, writer: &mut Writer<W>) -> io::Result<usize> {
        writer.write_std140(&self.as_std140())
    }

    fn std140_size(&self) -> usize {
        size_of::<<Self as AsStd140>::Output>()
    }
}

#[cfg(feature = "std")]
impl<T> WriteStd140 for [T]
where
    T: WriteStd140,
{
    fn write_std140<W: Write>(&self, writer: &mut Writer<W>) -> io::Result<usize> {
        // if no items are written, offset is current position of the writer
        let mut offset = writer.len();

        let mut iter = self.iter();

        if let Some(item) = iter.next() {
            offset = item.write_std140(writer)?;
        }

        for item in iter {
            item.write_std140(writer)?;
        }

        Ok(offset)
    }

    fn std140_size(&self) -> usize {
        let mut writer = Writer::new(io::sink());
        self.write_std140(&mut writer).unwrap();
        writer.len()
    }
}

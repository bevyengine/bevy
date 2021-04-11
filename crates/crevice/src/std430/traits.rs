use std::io::{self, Write};
use std::mem::size_of;

use bytemuck::{bytes_of, Pod, Zeroable};

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

```skip
use cgmath::prelude::*;
use cgmath::{Matrix4, Deg, perspective};
use crevice::std430::{AsStd430, Std430};

#[derive(AsStd430)]
struct CameraUniform {
    view: mint::ColumnMatrix4<f32>,
    projection: mint::ColumnMatrix4<f32>,
}

let camera = CameraUniform {
    view: Matrix4::identity().into(),
    projection: perspective(Deg(60.0), 16.0/9.0, 0.01, 100.0).into(),
};

# fn write_to_gpu_buffer(bytes: &[u8]) {}
let camera_std430 = camera.as_std430();
write_to_gpu_buffer(camera_std430.as_bytes());
```
*/
pub trait AsStd430 {
    /// The `std430` version of this value.
    type Std430Type: Std430;

    /// Convert this value into the `std430` version of itself.
    fn as_std430(&self) -> Self::Std430Type;

    /// Returns the size of the `std430` version of this type. Useful for
    /// pre-sizing buffers.
    fn std430_size_static() -> usize {
        size_of::<Self::Std430Type>()
    }
}

impl<T> AsStd430 for T
where
    T: Std430,
{
    type Std430Type = Self;

    fn as_std430(&self) -> Self {
        *self
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

impl<T> WriteStd430 for T
where
    T: AsStd430,
{
    fn write_std430<W: Write>(&self, writer: &mut Writer<W>) -> io::Result<usize> {
        writer.write_std430(&self.as_std430())
    }

    fn std430_size(&self) -> usize {
        size_of::<<Self as AsStd430>::Std430Type>()
    }
}

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

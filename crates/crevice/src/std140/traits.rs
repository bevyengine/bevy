use std::io::{self, Write};
use std::mem::size_of;

use bytemuck::{bytes_of, Pod, Zeroable};

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
Trait implemented for all types that can be turned into `std140` values.

This trait can often be `#[derive]`'d instead of manually implementing it. Any
struct which contains only fields that also implement `AsStd140` can derive
`AsStd140`.

Types from the mint crate implement `AsStd140`, making them convenient for use
in uniform types. Most Rust geometry crates, like cgmath, nalgebra, and
ultraviolet support mint.

## Example

```glsl
uniform CAMERA {
    mat4 view;
    mat4 projection;
} camera;
```

```
use cgmath::prelude::*;
use cgmath::{Matrix4, Deg, perspective};
use crevice::std140::{AsStd140, Std140};

#[derive(AsStd140)]
struct CameraUniform {
    view: mint::ColumnMatrix4<f32>,
    projection: mint::ColumnMatrix4<f32>,
}

let camera = CameraUniform {
    view: Matrix4::identity().into(),
    projection: perspective(Deg(60.0), 16.0/9.0, 0.01, 100.0).into(),
};

# fn write_to_gpu_buffer(bytes: &[u8]) {}
let camera_std140 = camera.as_std140();
write_to_gpu_buffer(camera_std140.as_bytes());
```
*/
pub trait AsStd140 {
    /// The `std140` version of this value.
    type Std140Type: Std140;

    /// Convert this value into the `std140` version of itself.
    fn as_std140(&self) -> Self::Std140Type;

    /// Returns the size of the `std140` version of this type. Useful for
    /// pre-sizing buffers.
    fn std140_size_static() -> usize {
        size_of::<Self::Std140Type>()
    }
}

impl<T> AsStd140 for T
where
    T: Std140,
{
    type Std140Type = Self;

    fn as_std140(&self) -> Self {
        *self
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

impl<T> WriteStd140 for T
where
    T: AsStd140,
{
    fn write_std140<W: Write>(&self, writer: &mut Writer<W>) -> io::Result<usize> {
        writer.write_std140(&self.as_std140())
    }

    fn std140_size(&self) -> usize {
        size_of::<<Self as AsStd140>::Std140Type>()
    }
}

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

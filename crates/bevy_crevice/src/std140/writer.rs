use std::io::{self, Write};
use std::mem::size_of;

use bytemuck::bytes_of;

use crate::internal::align_offset;
use crate::std140::{AsStd140, Std140, WriteStd140};

/**
Type that enables writing correctly aligned `std140` values to a buffer.

`Writer` is useful when many values need to be laid out in a row that cannot be
represented by a struct alone, like dynamically sized arrays or dynamically
laid-out values.

## Example
In this example, we'll write a length-prefixed list of lights to a buffer.
`std140::Writer` helps align correctly, even across multiple structs, which can
be tricky and error-prone otherwise.

```glsl
struct PointLight {
    vec3 position;
    vec3 color;
    float brightness;
};

buffer POINT_LIGHTS {
    uint len;
    PointLight[] lights;
} point_lights;
```

```
use crevice::std140::{self, AsStd140};

#[derive(AsStd140)]
struct PointLight {
    position: mint::Vector3<f32>,
    color: mint::Vector3<f32>,
    brightness: f32,
}

let lights = vec![
    PointLight {
        position: [0.0, 1.0, 0.0].into(),
        color: [1.0, 0.0, 0.0].into(),
        brightness: 0.6,
    },
    PointLight {
        position: [0.0, 4.0, 3.0].into(),
        color: [1.0, 1.0, 1.0].into(),
        brightness: 1.0,
    },
];

# fn map_gpu_buffer_for_write() -> &'static mut [u8] {
#     Box::leak(vec![0; 1024].into_boxed_slice())
# }
let target_buffer = map_gpu_buffer_for_write();
let mut writer = std140::Writer::new(target_buffer);

let light_count = lights.len() as u32;
writer.write(&light_count)?;

// Crevice will automatically insert the required padding to align the
// PointLight structure correctly. In this case, there will be 12 bytes of
// padding between the length field and the light list.

writer.write(lights.as_slice())?;

# fn unmap_gpu_buffer() {}
unmap_gpu_buffer();

# Ok::<(), std::io::Error>(())
```
*/
pub struct Writer<W> {
    writer: W,
    offset: usize,
}

impl<W: Write> Writer<W> {
    /// Create a new `Writer`, wrapping a buffer, file, or other type that
    /// implements [`std::io::Write`].
    pub fn new(writer: W) -> Self {
        Self { writer, offset: 0 }
    }

    /// Write a new value to the underlying buffer, writing zeroed padding where
    /// necessary.
    ///
    /// Returns the offset into the buffer that the value was written to.
    pub fn write<T>(&mut self, value: &T) -> io::Result<usize>
    where
        T: WriteStd140 + ?Sized,
    {
        value.write_std140(self)
    }

    /// Write an iterator of values to the underlying buffer.
    ///
    /// Returns the offset into the buffer that the first value was written to.
    /// If no values were written, returns the `len()`.
    pub fn write_iter<I, T>(&mut self, iter: I) -> io::Result<usize>
    where
        I: IntoIterator<Item = T>,
        T: WriteStd140,
    {
        let mut offset = self.offset;

        let mut iter = iter.into_iter();

        if let Some(item) = iter.next() {
            offset = item.write_std140(self)?;
        }

        for item in iter {
            item.write_std140(self)?;
        }

        Ok(offset)
    }

    /// Write an `Std140` type to the underlying buffer.
    pub fn write_std140<T>(&mut self, value: &T) -> io::Result<usize>
    where
        T: Std140,
    {
        let padding = align_offset(self.offset, T::ALIGNMENT);

        for _ in 0..padding {
            self.writer.write_all(&[0])?;
        }
        self.offset += padding;

        let value = value.as_std140();
        self.writer.write_all(bytes_of(&value))?;

        let write_here = self.offset;
        self.offset += size_of::<T>();

        Ok(write_here)
    }

    /// Write a slice of values to the underlying buffer.
    #[deprecated(
        since = "0.6.0",
        note = "Use `write` instead -- it now works on slices."
    )]
    pub fn write_slice<T>(&mut self, slice: &[T]) -> io::Result<usize>
    where
        T: AsStd140,
    {
        self.write(slice)
    }

    /// Returns the amount of data written by this `Writer`.
    pub fn len(&self) -> usize {
        self.offset
    }
}

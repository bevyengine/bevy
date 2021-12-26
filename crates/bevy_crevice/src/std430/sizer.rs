use core::mem::size_of;

use crate::internal::align_offset;
use crate::std430::{AsStd430, Std430};

/**
Type that computes the buffer size needed by a series of `std430` types laid
out.

This type works well well when paired with `Writer`, precomputing a buffer's
size to alleviate the need to dynamically re-allocate buffers.

## Example

```glsl
struct Frob {
    vec3 size;
    float frobiness;
}

buffer FROBS {
    uint len;
    Frob[] frobs;
} frobs;
```

```
use bevy_crevice::std430::{self, AsStd430};

#[derive(AsStd430)]
struct Frob {
    size: mint::Vector3<f32>,
    frobiness: f32,
}

// Many APIs require that buffers contain at least enough space for all
// fixed-size bindiongs to a buffer as well as one element of any arrays, if
// there are any.
let mut sizer = std430::Sizer::new();
sizer.add::<u32>();
sizer.add::<Frob>();

# fn create_buffer_with_size(size: usize) {}
let buffer = create_buffer_with_size(sizer.len());
# assert_eq!(sizer.len(), 32);
```
*/
pub struct Sizer {
    offset: usize,
}

impl Sizer {
    /// Create a new `Sizer`.
    pub fn new() -> Self {
        Self { offset: 0 }
    }

    /// Add a type's necessary padding and size to the `Sizer`. Returns the
    /// offset into the buffer where that type would be written.
    pub fn add<T>(&mut self) -> usize
    where
        T: AsStd430,
    {
        let size = size_of::<<T as AsStd430>::Output>();
        let alignment = <T as AsStd430>::Output::ALIGNMENT;
        let padding = align_offset(self.offset, alignment);

        self.offset += padding;
        let write_here = self.offset;

        self.offset += size;

        write_here
    }

    /// Returns the number of bytes required to contain all the types added to
    /// the `Sizer`.
    pub fn len(&self) -> usize {
        self.offset
    }
}

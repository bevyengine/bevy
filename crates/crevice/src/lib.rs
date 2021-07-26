#![allow(clippy::all)]

/*!
[![GitHub CI Status](https://github.com/LPGhatguy/crevice/workflows/CI/badge.svg)](https://github.com/LPGhatguy/crevice/actions)
[![crevice on crates.io](https://img.shields.io/crates/v/crevice.svg)](https://crates.io/crates/crevice)
[![crevice docs](https://img.shields.io/badge/docs-docs.rs-orange.svg)](https://docs.rs/crevice)

Crevice creates GLSL-compatible versions of types through the power of derive
macros. Generated structures provide an [`as_bytes`][std140::Std140::as_bytes]
method to allow safely packing data into buffers for uploading.

Generated structs also implement [`bytemuck::Zeroable`] and
[`bytemuck::Pod`] for use with other libraries.

Crevice is similar to [`glsl-layout`][glsl-layout], but supports `mint` types
and explicitly initializes padding to remove one source of undefined behavior.

Examples in this crate use cgmath, but any math crate that works with the mint
crate will also work. Some other crates include nalgebra, ultraviolet, glam, and
vek.

## Examples

### Single Value

Uploading many types can be done by deriving `AsStd140` and using
[`as_std140`][std140::AsStd140::as_std140] and
[`as_bytes`][std140::Std140::as_bytes] to turn the result into bytes.

```glsl
uniform MAIN {
    mat3 orientation;
    vec3 position;
    float scale;
} main;
```

```skip
use crevice::std140::{AsStd140, Std140};
use cgmath::prelude::*;
use cgmath::{Matrix3, Vector3};

#[derive(AsStd140)]
struct MainUniform {
    orientation: mint::ColumnMatrix3<f32>,
    position: mint::Vector3<f32>,
    scale: f32,
}

let value = MainUniform {
    orientation: Matrix3::identity().into(),
    position: Vector3::new(1.0, 2.0, 3.0).into(),
    scale: 4.0,
};

let value_std140 = value.as_std140();

# fn upload_data_to_gpu(_value: &[u8]) {}
upload_data_to_gpu(value_std140.as_bytes());
```

### Sequential Types

More complicated data can be uploaded using the std140 `Writer` type.

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

```skip
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

## Minimum Supported Rust Version (MSRV)

Crevice supports Rust 1.46.0 and newer due to use of new `const fn` features.

[glsl-layout]: https://github.com/rustgd/glsl-layout
[Zeroable]: https://docs.rs/bytemuck/latest/bytemuck/trait.Zeroable.html
[Pod]: https://docs.rs/bytemuck/latest/bytemuck/trait.Pod.html
[TypeLayout]: https://docs.rs/type-layout/latest/type_layout/trait.TypeLayout.html
*/

#![deny(missing_docs)]
#![cfg_attr(not(feature = "std"), no_std)]

pub mod std140;
pub mod std430;

#[doc(hidden)]
pub mod internal;

mod mint;

mod glam;

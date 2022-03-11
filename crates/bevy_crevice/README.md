# Bevy Crevice

This is a fork of [Crevice](https://crates.io/crates/crevice) for
[Bevy](https://bevyengine.org).

For use outside of Bevy, you should consider
using [Crevice](https://crates.io/crates/crevice) directly.

It was forked to allow better integration in Bevy:

* Easier derive macro usage, without needing to depend on `Crevice` directly.
* Use of unmerged features (as of the fork), like
[Array Support](https://github.com/LPGhatguy/crevice/pull/27/).
* Renaming of traits and macros to better match Bevy API.

## Crevice

Crevice creates GLSL-compatible versions of types through the power of derive
macros. Generated structures provide an [`as_bytes`][std140::Std140::as_bytes]
method to allow safely packing data into buffers for uploading.

Generated structs also implement [`bytemuck::Zeroable`] and
[`bytemuck::Pod`] for use with other libraries.

Crevice is similar to [`glsl-layout`][glsl-layout], but supports types from many
math crates, can generate GLSL source from structs, and explicitly initializes
padding to remove one source of undefined behavior.

Crevice has support for many Rust math libraries via feature flags, and most
other math libraries by use of the mint crate. Crevice currently supports:

* mint 0.5, enabled by default
* cgmath 0.18, using the `cgmath` feature
* nalgebra 0.29, using the `nalgebra` feature
* glam 0.20, using the `glam` feature

PRs are welcome to add or update math libraries to Crevice.

If your math library is not supported, it's possible to define structs using the
types from mint and convert your math library's types into mint types. This is
supported by most Rust math libraries.

Your math library may require you to turn on a feature flag to get mint support.
For example, cgmath requires the "mint" feature to be enabled to allow
conversions to and from mint types.

## Examples

### Single Value

Uploading many types can be done by deriving [`AsStd140`][std140::AsStd140] and
using [`as_std140`][std140::AsStd140::as_std140] and
[`as_bytes`][std140::Std140::as_bytes] to turn the result into bytes.

```glsl
uniform MAIN {
    mat3 orientation;
    vec3 position;
    float scale;
} main;
```

```rust
use bevy_crevice::std140::{AsStd140, Std140};

#[derive(AsStd140)]
struct MainUniform {
    orientation: mint::ColumnMatrix3<f32>,
    position: mint::Vector3<f32>,
    scale: f32,
}

let value = MainUniform {
    orientation: [
        [1.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [0.0, 0.0, 1.0],
    ].into(),
    position: [1.0, 2.0, 3.0].into(),
    scale: 4.0,
};

let value_std140 = value.as_std140();

upload_data_to_gpu(value_std140.as_bytes());
```

### Sequential Types

More complicated data can be uploaded using the std140
[`Writer`][std140::Writer] type.

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

```rust
use bevy_crevice::std140::{self, AsStd140};

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

let target_buffer = map_gpu_buffer_for_write();
let mut writer = std140::Writer::new(target_buffer);

let light_count = lights.len() as u32;
writer.write(&light_count)?;

// Crevice will automatically insert the required padding to align the
// PointLight structure correctly. In this case, there will be 12 bytes of
// padding between the length field and the light list.

writer.write(lights.as_slice())?;

unmap_gpu_buffer();

```

## Features

* `std` (default): Enables [`std::io::Write`]-based structs.
* `cgmath`: Enables support for types from cgmath.
* `nalgebra`: Enables support for types from nalgebra.
* `glam`: Enables support for types from glam.

## Minimum Supported Rust Version (MSRV)

Crevice supports Rust 1.52.1 and newer due to use of new `const fn` features.

[glsl-layout]: https://github.com/rustgd/glsl-layout

[std140::AsStd140]: https://docs.rs/crevice/latest/crevice/std140/trait.AsStd140.html
[std140::AsStd140::as_std140]: https://docs.rs/crevice/latest/crevice/std140/trait.AsStd140.html#method.as_std140
[std140::Std140::as_bytes]: https://docs.rs/crevice/latest/crevice/std140/trait.Std140.html#method.as_bytes
[std140::Writer]: https://docs.rs/crevice/latest/crevice/std140/struct.Writer.html

[`std::io::Write`]: https://doc.rust-lang.org/stable/std/io/trait.Write.html

[`bytemuck::Pod`]: https://docs.rs/bytemuck/latest/bytemuck/trait.Pod.html
[`bytemuck::Zeroable`]: https://docs.rs/bytemuck/latest/bytemuck/trait.Zeroable.html

## License

Licensed under either of

* Apache License, Version 2.0, ([LICENSE-APACHE](http://www.apache.org/licenses/LICENSE-2.0))
* MIT license ([LICENSE-MIT](http://opensource.org/licenses/MIT))

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall
be dual licensed as above, without any additional terms or conditions.

# glam

[![Build Status]][travis-ci] [![Coverage Status]][coveralls.io]
[![Latest Version]][crates.io] [![docs]][docs.rs]

A simple and fast 3D math library for games and graphics.

## Development status

`glam` is in alpha stage. Minimal base functionality has been implemented
and the look and feel of the API has solidified.

## Features

* Only single precision floating point (`f32`) arithmetic is supported
* vectors: `Vec2`, `Vec3`, `Vec4`
* square matrices: `Mat2`, `Mat3`, `Mat4`
* a quaternion type: `Quat`

### SIMD

The `Vec3`, `Vec4` and `Quat` types use SSE2 on x86/x86_64 architectures.
`Mat2`, `Mat3` and `Mat4` also use SSE2 for some functionality. Not everything
has a SIMD implementation yet.

Note that this does result in some wasted space in the case of `Vec3` and `Mat3`
as the SIMD vector type is 16 bytes large and 16 byte aligned.

It is possible to opt out of using SIMD types for Vec3 and Mat3 storage with the
`packed-vec3` feature.

`glam` outperforms similar Rust libraries such as [`cgmath`][cgmath],
[`nalgebra-glm`][nalgebra-glm] and others for common operations as tested by the
[`mathbench`][mathbench] project.

If you are more concerned with size than speed you can build glam with the
feature `scalar-math` enabled to disable SIMD usage.

Due to the use of SIMD, vector elements may only be get and set via accessor
methods, e.g. `Vec3::x()` and `Vec3::x_mut()` or `Vec3::set_x()`. If getting or
setting more than one element it is more efficient to convert from tuples or
arrays:

```
let (x, y, z) = v.into();
let [x, y, z]: [f32; 3] = v.into();
```

### Optional features

* `mint` - for interoperating with other 3D math libraries
* `rand` - implementations of `Distribution` trait for all `glam` types. This
  is primarily used for unit testing
* `serde` - implementations of `Serialize` and `Deserialize` for all `glam`
  types. Note that serialization should work between builds of `glam` with and
  without SIMD enabled

### Feature gates

* `packed-vec3` - disable using SIMD types for `Vec3` and `Mat3` storage.  This
  avoids wasting space due to 16 byte alignment at the cost of some performance.
* `scalar-math` - compiles with SIMD support disabled
* `glam-assert` - adds assertions which check the validity of parameters passed to
  `glam` to help catch runtime errors

## Conventions

### Column vectors

`glam` interprets vectors as column matrices (also known as "column vectors")
meaning when transforming a vector with a matrix the matrix goes on the left,
e.g. `v' = Mv`.  DirectX uses row vectors, OpenGL uses column vectors. There
are pros and cons to both.

### Column-major order

Matrices are stored in column major format. Each column vector is stored in
contiguous memory.

### Co-ordinate system

`glam` is co-ordinate system agnostic and intends to support both right handed
and left handed conventions.

Rotations follow the left-hand rule.

## Design Philosophy

The design of this library is guided by a desire for simplicity and good
performance.

* No traits or generics for simplicity of implementation and usage
* Only single precision floating point (`f32`) arithmetic is supported
* All dependencies are optional (e.g. `mint`, `rand` and `serde`)
* Follows the [Rust API Guidelines] where possible
* Aiming for 100% test [coverage][coveralls.io]
* Common functionality is benchmarked using [Criterion.rs]

## Future work

* Experiment with a using a 4x3 matrix as a 3D transform type that can be more
  efficient than `Mat4` for certain operations like inverse and multiplies
* `no-std` support
* `wasm` support

## Inspirations

There were many inspirations for the interface and internals of glam from the
Rust and C++ worlds. In particular:

* [How to write a maths library in 2016] inspired the initial `Vec3`
  implementation
* [Realtime Math] - header only C++11 with SSE and NEON SIMD intrinsic support
* [DirectXMath] - header only SIMD C++ linear algebra library for use in games
  and graphics apps
* `glam` is a play on the name of the popular C++ library `glm`

## License

Licensed under either of

* Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE)
  or http://www.apache.org/licenses/LICENSE-2.0)
* MIT license ([LICENSE-MIT](LICENSE-MIT)
  or http://opensource.org/licenses/MIT)

at your option.

## Contribution

Contributions in any form (issues, pull requests, etc.) to this project must
adhere to Rust's [Code of Conduct].

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.

Thank you to all of the `glam` [contributors]!

## Support
If you are interested in contributing or have a request or suggestion
[create an issue] on github.


[Build Status]: https://travis-ci.org/bitshifter/glam-rs.svg?branch=master
[travis-ci]: https://travis-ci.org/bitshifter/glam-rs
[Coverage Status]: https://coveralls.io/repos/github/bitshifter/glam-rs/badge.svg?branch=master
[coveralls.io]: https://coveralls.io/github/bitshifter/glam-rs?branch=master
[Code of Conduct]: https://www.rust-lang.org/en-US/conduct.html
[Latest Version]: https://img.shields.io/crates/v/glam.svg
[crates.io]: https://crates.io/crates/glam/
[docs]: https://docs.rs/glam/badge.svg
[docs.rs]: https://docs.rs/glam/
[Rust API Guidelines]: https://rust-lang-nursery.github.io/api-guidelines/
[Criterion.rs]: https://bheisler.github.io/criterion.rs/book/index.html
[cgmath]: https://github.com/rustgd/cgmath
[nalgebra-glm]: https://github.com/rustsim/nalgebra
[mathbench]: https://github.com/bitshifter/mathbench-rs
[create an issue]: https://github.com/bitshifter/glam-rs/issues
[contributors]: https://github.com/bitshifter/glam-rs/graphs/contributors
[How to write a maths library in 2016]: http://www.codersnotes.com/notes/maths-lib-2016/
[Realtime Math]: https://github.com/nfrechette/rtm
[DirectXMath]: https://docs.microsoft.com/en-us/windows/desktop/dxmath/directxmath-portal

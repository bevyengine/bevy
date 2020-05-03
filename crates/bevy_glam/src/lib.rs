/*!
# glam

`glam` is a simple and fast linear algebra library for games and graphics.

## Features

`glam` is built with SIMD in mind. Currently only SSE2 on x86/x86_64 is
supported as this is what stable Rust supports.

* Single precision float (`f32`) support only
* SSE2 implementation for most types, including `Mat2`, `Mat3`, `Mat4`, `Quat`,
  `Vec3` and `Vec4`
* Scalar fallback implementations exist when SSE2 is not available
* Most functionality includes unit tests and benchmarks

## Linear algebra conventions

`glam` interprets vectors as column matrices (also known as "column vectors")
meaning when transforming a vector with a matrix the matrix goes on the left.

```
use glam::{Mat3, Vec3};
let m = Mat3::identity();
let x = Vec3::unit_x();
let v = m * x;
assert_eq!(v, x);
```

Matrices are stored in memory in column-major order.

Rotations follow left-hand rule. The direction of the axis gives the direction
of rotation: with the left thumb pointing in the positive direction of the axis
the left fingers curl around the axis in the direction of the rotation.

```
use glam::{Mat3, Vec3};
// rotate +x 90 degrees clockwise around y giving -z
let m = Mat3::from_rotation_y(90.0_f32.to_radians());
let v = m * Vec3::unit_x();
assert!(v.abs_diff_eq(-Vec3::unit_z(), core::f32::EPSILON));
```

## Size and alignment of types

Most `glam` types use SIMD for storage meaning most types are 16 byte aligned.
The only exception is Vec2`. When SSE2 is not available on the target
architecture the types will still be 16 byte aligned, so object sizes and
layouts will not change between architectures.

16 byte alignment means that some types will have a stride larger than their
size resulting in some wasted space.

| Type | f32 bytes | SIMD bytes | Wasted bytes |
|:-----|----------:|-----------:|-------------:|
|`Vec3`|         12|          16|             4|
|`Mat3`|         36|          48|            12|

Despite this wasted space the SIMD version tends to outperform the `f32`
implementation in [**mathbench**](https://github.com/bitshifter/mathbench-rs)
benchmarks.

SIMD support can be disabled entirely using the `scalar-math` feature. This
feature will also disable SIMD alignment meaning most types will use native
`f32` alignment of 4 bytes.

All the main `glam` types are tagged with #[repr(C)], so they are possible
to expose as struct members to C interfaces if desired. Be mindful of Vec3's
extra float though.

## Accessing internal data

The SIMD types that `glam` builds on are opaque and their contents are not
directly accessible. Because of this `glam` types uses getter and setter
methods instead of providing direct access.

```
use glam::Vec3;
let mut v = Vec3::new(1.0, 2.0, 3.0);
assert_eq!(v.y(), 2.0);
v.set_z(1.0);
assert_eq!(v.z(), 1.0);
```

If you need to access multiple elements it is easier to convert the type to a
tuple or array:

```
use glam::Vec3;
let v = Vec3::new(1.0, 2.0, 3.0);
let (x, y, z) = v.into();
assert_eq!((x, y, z), (1.0, 2.0, 3.0));
```

## SIMD and scalar consistency

`glam` types implement `serde` `Serialize` and `Deserialize` traits to ensure
that they will serialize and deserialize exactly the same whether or not
SIMD support is being used.

The SIMD versions implement `core::fmt::Display` traits so they print the same as
the scalar version.

```
use glam::Vec3;
let a = Vec3::new(1.0, 2.0, 3.0);
assert_eq!(format!("{}", a), "[1, 2, 3]");
```

## Feature gates

All `glam` dependencies are optional, however some are required for tests
and benchmarks.

* `std` - the default feature, has no dependencies.
* `rand` - used to generate random values. Used in benchmarks.
* `serde` - used for serialization and deserialization of types.
* `mint` - used for interoperating with other linear algebra libraries.
* `scalar-math` - disables SIMD support and uses native alignment for all
  types.
* `debug-glam-assert` - adds assertions in debug builds which check the validity
  of parameters passed to `glam` to help catch runtime errors.
* `glam-assert` - adds assertions to all builds which check the validity of
  parameters passed to `glam` to help catch runtime errors.

*/
#![doc(html_root_url = "https://docs.rs/glam/0.8.7")]

#[macro_use]
mod macros;

pub mod f32;

pub use self::f32::{
    mat2, mat3, mat4, quat, vec2, vec3, vec4, Mat2, Mat3, Mat4, Quat, Vec2, Vec2Mask, Vec3,
    Vec3Mask, Vec4, Vec4Mask,
};

#[cfg(feature = "transform-types")]
pub use self::f32::{TransformRT, TransformSRT};

#[repr(align(16))]
pub(crate) struct Align16<T>(T);

impl<T> Align16<T> {
    #[allow(dead_code)]
    pub fn as_ptr(&self) -> *const T {
        &self.0
    }

    #[allow(dead_code)]
    pub fn as_mut_ptr(&mut self) -> *mut T {
        &mut self.0
    }
}

#[test]
fn test_align16() {
    use core::{mem, ptr};
    let mut a = Align16::<f32>(1.0);
    assert_eq!(mem::align_of_val(&a), 16);
    unsafe {
        assert_eq!(ptr::read(a.as_ptr()), 1.0);
        ptr::write(a.as_mut_ptr(), -1.0);
    }
    assert_eq!(a.0, -1.0);
}

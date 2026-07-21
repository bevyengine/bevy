#![forbid(unsafe_code)]
#![cfg_attr(
    any(docsrs, docsrs_dep),
    expect(
        internal_features,
        reason = "rustdoc_internals is needed for fake_variadic"
    )
)]
#![cfg_attr(any(docsrs, docsrs_dep), feature(rustdoc_internals))]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![doc(
    html_logo_url = "https://bevy.org/assets/icon.png",
    html_favicon_url = "https://bevy.org/assets/icon.png"
)]
#![no_std]

//! Geometric algorithms and utilities for primitive shapes.
//!
//! This crate extends the primitives in [`bevy_shape`] with common geometric operations, including:
//!
//! - [Computing measurements such as area, perimeter, surface area, and volume.](measured)
//! - [Computing axis-aligned and spherical bounding volumes.](bounding)
//! - [Ray casting and intersection queries.](ray)
//! - [Constructing inset and hollow ("ring") variants of 2D primitives.](ring)
//! - [Random sampling from geometric primitives and triangle meshes](sampling) (via the `rand` feature).
//!
//! The functionality is provided through extension traits, allowing geometric
//! algorithms to be used directly with the corresponding primitive types.

#[cfg(feature = "std")]
extern crate std;

#[cfg(feature = "alloc")]
extern crate alloc;

pub mod bounding;
pub mod inset;
pub mod measured;
pub mod ray;
pub mod ring;

#[cfg(feature = "rand")]
pub mod sampling;

/// The geometry prelude.
///
/// This includes all geometry traits defined in this crate, re-exported for your convenience.
pub mod prelude {
    #[doc(hidden)]
    pub use crate::bounding::{bounded2d::*, bounded3d::*, BoundingVolume, IntersectsVolume};

    #[doc(hidden)]
    pub use crate::inset::*;

    #[doc(hidden)]
    pub use crate::measured::*;

    #[doc(hidden)]
    pub use crate::ray::{raycast2d::*, raycast3d::*, Ray2dIntersectionExt, Ray3dIntersectionExt};

    #[doc(hidden)]
    pub use crate::ring::*;

    #[doc(hidden)]
    #[cfg(feature = "rand")]
    pub use crate::sampling::*;
}

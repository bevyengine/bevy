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

//! This module defines functionality for primitive shapes.

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
    pub use crate::ray::{raycast2d::*, raycast3d::*, Ray2dIntersection, Ray3dIntersection};

    #[doc(hidden)]
    pub use crate::ring::*;

    #[doc(hidden)]
    #[cfg(feature = "rand")]
    pub use crate::sampling::*;
}

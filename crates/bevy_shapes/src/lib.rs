//! # Bevy Shapes
//!
//! Geometric primitives and shape-related traits for Bevy.
//!
//! This crate provides the complete collection of 2D and 3D geometric primitives
//! used throughout the Bevy ecosystem, along with implementations of common traits
//! for operations such as meshing, bounding volumes, sampling, and debug rendering.
//!
//! ## Feature flags
//!
//! Additional functionality is available through optional features:
//!
//! - `meshing` — Generate meshes from supported primitives.
//! - `bounding` — Compute bounding volumes.
//! - `sampling` — Sample points on or within primitives.
//! - `gizmos` — Draw primitives using Bevy Gizmos.
//!
//! ## Examples
//!
//! Constructing a primitive:
//!
//! ```
//! use bevy_shapes::primitives::Circle;
//!
//! let circle = Circle::new(1.0);
//! ```
//!
//! Many traits are implemented behind feature flags:
//!
//! ```ignore
//! use bevy_shapes::primitives::Sphere;
//! use bevy_shapes::meshing::Meshable;
//!
//! let mesh = sphere.mesh();
//! ```
//!
//! Most users will interact with the types in the [`primitives`] module, while
//! optional modules provide additional functionality for those primitives.
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

#[cfg(feature = "std")]
extern crate std;

#[cfg(feature = "alloc")]
extern crate alloc;

pub mod primitives;

#[cfg(feature = "meshing")]
pub mod meshing;

#[cfg(feature = "bounding")]
pub mod bounding;

#[cfg(feature = "sampling")]
pub mod sampling;

#[cfg(feature = "gizmos")]
pub mod gizmos;

/// The shapes prelude.
///
/// This includes the most common types in this crate, re-exported for your convenience.
pub mod prelude {
    #[doc(hidden)]
    pub use crate::primitives::*;

    #[doc(hidden)]
    #[cfg(feature = "meshing")]
    pub use crate::meshing::*;

    #[doc(hidden)]
    #[cfg(feature = "bounding")]
    pub use crate::bounding::*;

    #[doc(hidden)]
    #[cfg(feature = "sampling")]
    pub use crate::sampling::*;

    #[doc(hidden)]
    #[cfg(feature = "gizmos")]
    pub use crate::gizmos::*;
}

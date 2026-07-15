//! This module contains the implementations for bounding traits from the [`bevy_math`] crate for
//! working with primitive shapes
//!
//! There are four traits used:
//! - [`BoundingVolume`] is a generic abstraction for any bounding volume
//! - [`IntersectsVolume`] abstracts intersection tests against a [`BoundingVolume`]
//! - [`Bounded2d`]/[`Bounded3d`] are abstractions for shapes to generate [`BoundingVolume`]s

pub mod dim2;
pub mod dim3;
pub mod extrusion;

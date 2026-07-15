//! A module for rendering each of the 2D and 3D [`bevy_math::primitives`] with [`Gizmos`](`crate::prelude::Gizmos`).

mod dim2;
pub use dim2::*;

mod dim3;
pub use dim3::*;

pub(crate) mod helpers;

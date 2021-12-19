//! Defines traits and types for working with data adhering to GLSL's `std140`
//! layout specification.

mod primitives;
mod sizer;
mod traits;
#[cfg(feature = "std")]
mod writer;

pub use self::primitives::*;
pub use self::sizer::*;
pub use self::traits::*;
#[cfg(feature = "std")]
pub use self::writer::*;

pub use bevy_crevice_derive::AsStd430;

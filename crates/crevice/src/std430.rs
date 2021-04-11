//! Defines traits and types for working with data adhering to GLSL's `std140`
//! layout specification.

mod primitives;
mod sizer;
mod traits;
mod writer;

pub use self::primitives::*;
pub use self::sizer::*;
pub use self::traits::*;
pub use self::writer::*;

pub use crevice_derive::AsStd430;

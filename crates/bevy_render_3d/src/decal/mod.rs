//! Decal rendering.
//!
//! Decals are a material that render on top of the surface that they're placed above.
//! They can be used to render signs, paint, snow, impact craters, and other effects on top of surfaces.

// TODO: Once other decal types are added, write a paragraph comparing the different types in the module docs.

pub mod clustered;
mod forward;

pub use forward::*;

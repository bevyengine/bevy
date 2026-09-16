//! Provides reflected type information.
//!
//! Most reflected types will implement [`Typed`],
//! enabling their compile-time [`TypeInfo`] to be accessed at runtime.

mod error;
mod opaque;
mod type_info;
mod typed;

pub use error::*;
pub use opaque::*;
pub use type_info::*;
pub use typed::*;

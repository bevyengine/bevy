//! Argument types and utilities for working with [`DynamicClosure`] and [`DynamicClosureMut`].
//!
//! [`DynamicClosure`]: crate::func::DynamicClosure
//! [`DynamicClosureMut`]: crate::func::DynamicClosureMut

pub use arg::*;
pub use error::*;
pub use from_arg::*;
pub use info::*;
pub use list::*;
pub use ownership::*;

mod arg;
mod error;
mod from_arg;
mod info;
mod list;
mod ownership;

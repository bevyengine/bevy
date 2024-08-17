//! Argument types and utilities for working with [`DynamicCallable`] and [`DynamicCallableMut`].
//!
//! [`DynamicCallable`]: crate::func::DynamicCallable
//! [`DynamicCallableMut`]: crate::func::DynamicCallableMut

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

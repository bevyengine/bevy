//! Argument types and utilities for working with [`DynamicFunction`] and [`DynamicFunctionMut`].
//!
//! [`DynamicFunction`]: crate::func::DynamicFunction
//! [`DynamicFunctionMut`]: crate::func::DynamicFunctionMut

pub use arg::*;
pub use count::*;
pub use error::*;
pub use from_arg::*;
pub use info::*;
pub use list::*;
pub use ownership::*;

mod arg;
mod count;
mod error;
mod from_arg;
mod info;
mod list;
mod ownership;

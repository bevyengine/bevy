//! Argument types and utilities for working with dynamic [`Functions`].
//!
//! [`Functions`]: crate::func::Function

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

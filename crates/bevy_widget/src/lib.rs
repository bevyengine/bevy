#![forbid(unsafe_code)]
#![warn(missing_docs)]
//! This crate contains UI widgets like buttons

mod widget;
mod widget_bundles;

pub use widget::*;
pub use widget_bundles::*;

#[doc(hidden)]
pub mod prelude {
	#[doc(hidden)]
    pub use super::widget::Button;
    pub use super::widget_bundles::*;
}

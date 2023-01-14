#![forbid(unsafe_code)]
#![warn(missing_docs)]
//! The official collection of user interface widgets for `bevy_ui`.

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

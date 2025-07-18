//! Meta-module containing all feathers containers (passive widgets that hold other widgets).
mod flex_spacer;
mod pane;
mod subpane;

pub use flex_spacer::flex_spacer;
pub use pane::{pane, pane_body, pane_header, pane_header_divider};
pub use subpane::{subpane, subpane_body, subpane_header};

//! This module provides text editing functionality, by wrapping the functionality of the
//! [`cosmic_text`] crate.
//!
//! The primary entry point into the text editing functionality is the [`TextInputBuffer`] component,
//! which includes a [`cosmic_text::Editor`], and adds the associated "required components" needed
//! to construct functioning text input fields.
//!
//! ## How this works
//!
//! Text editing functionality is included as part of [`TextPlugin`](crate::TextPlugin),
//! and the systems which perform the work are grouped under the [`TextInputSystems`] system set.
//!  
//! The [`TextInputBuffer`] comes with the following required components that act as machinery to convert user inputs into text:
//!
//! * [`TextInputValue`] - Contains the current text in the text input buffer.
//!    * Automatically synchronized with the buffer by [`apply_text_edits`] after any edits are applied.
//! * [`TextEdits`] - Text input commands queue, used to queue up text edit and navigation actions.\
//!    * Contains a queue of [`TextEdit`] actions that are applied to the buffer.
//!    * These are applied by the [`apply_text_edits` system.
//! * [`TextInputTarget`] - Details of the render target the text input will be rendered to, such as size and scale factor.
//!
//!
//! Layouting is done in:
//!
//! * [`update_text_input_layouts`] - Updates the `TextLayoutInfo` for each text input for rendering.
//! * [`update_placeholder_layouts`] - Updates the `TextLayoutInfo` for each [`Placeholder`] for rendering.
//!
//! ## Configuring text input
//!
//! Several components are used to configure the text input, and belong on the [`TextInputBuffer`] entity:
//!
//! * [`TextInputAttributes`] - Common text input properties set by the user, such as font, font size, line height, justification, maximum characters etc.
//! * [`Placeholder`] - Optional component that can be added to display placeholder text when the input buffer is empty.
//! * [`CursorBlink`] - Optional component that controls cursor blinking.

use bevy_ecs::schedule::SystemSet;
pub use cosmic_text::Motion;

/// Systems handling text input update and layout
#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
pub struct TextInputSystems;

mod buffer;
mod cursor_blink;
mod layout;
mod placeholder;
mod target;
mod text_edit;

pub use buffer::*;
pub use cursor_blink::*;
pub use layout::*;
pub use placeholder::*;
pub use target::*;
pub use text_edit::*;

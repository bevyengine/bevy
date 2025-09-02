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
//! * [`TextInputFilter`] - Optional component that can be added to restrict the text input to certain formats, such as integers, decimals, hexadecimal etc.
//! * [`PasswordMask`] - Optional component that can be added to hide the text input buffer contents by replacing the characters with a mask character.
//! * [`Placeholder`] - Optional component that can be added to display placeholder text when the input buffer is empty.
//! * [`CursorBlink`] - Optional component that controls cursor blinking.
//!
//! ## Copy-paste and clipboard support
//!
//! The clipboard support provided by this module is very basic, and only works within the `bevy` app,
//! storing a simple [`String`].
//!
//! It can be accessed via the [`Clipboard`] resource.

use bevy_ecs::component::Component;
use bevy_math::Vec2;

/// Details of the target the text input will be rendered to
#[derive(Component, PartialEq, Debug, Default)]
pub struct TextInputTarget {
    /// Size of the target in physical pixels
    pub size: Vec2,
    /// Scale factor of the target
    pub scale_factor: f32,
}

impl TextInputTarget {
    /// Returns true if the target has zero or negative size.
    pub fn is_empty(&self) -> bool {
        (self.scale_factor * self.size).cmple(Vec2::ZERO).all()
    }
}

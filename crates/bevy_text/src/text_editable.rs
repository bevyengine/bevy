//! A simple text input widget for Bevy UI.
//!
//! The [`EditableText`] widget is an undecorated rectangular text input field,
//! which allows users to input and edit text within a Bevy UI application.
//! Every [`EditableText`] component is also a [`Node`](https://docs.rs/bevy/latest/bevy/prelude/struct.Node.html) in the Bevy UI hierarchy,
//! allowing you to position and size it using standard Bevy UI layout techniques.
//! You can think of it as the editable equivalent of [`Text`](https://docs.rs/bevy/latest/bevy/prelude/struct.Text.html),
//! and components such as [`TextFont`] and [`TextColor`] can be used to style it.
//!
//! [`EditableText`] supports the following functionality:
//!
//! - Text entry
//! - Basic keyboard-driven cursor movement (arrow keys, home/end keys)
//! - Backspace and delete operations
//!
//! You might use this widget as the basis for text input fields in forms, chat boxes, for naming characters,
//! or any other scenario where you want to extract an unformatted text string from the user.
//!
//! Reusable widgets that build on top of this basic text input field (as might be found in Bevy's Feathers UI framework),
//! will typically combine this widget with additional UI elements such as borders, backgrounds, and labels,
//! creating a multi-entity widget that matches the semantics and visual appearance required by the application.
//!
//! ## Handling user input
//!
//! User input is handled via a plugin in `bevy_ui_widgets`:
//! [`bevy_text`](crate) is not aware of input events directly.
//!
//! With the correct plugin enabled, when an [`EditableText`] entity is focused,
//! keyboard input events are captured and processed into [`TextEdit`] actions.
//!
//! ## Limitations
//!
//! The formatting of the text is uniform throughout the entire input field.
//! As a result, rich text-editing is out-of-scope:
//! this widget is not intended to form the basis for a full-featured text editor.
//!
//! Similarly, this widget is "headless": it has no built-in styling, and is intended to be used
//! with a themed UI framework of your choice (e.g. Feathers). This means that no text boxes, borders, or other
//! visual elements are provided by default, and must be added separately using Bevy UI entities / components,
//! and any reactive styling (e.g., focus/hover states) must also be implemented separately.
//!
//! However, the following features are planned but currently not implemented:
//!
//! - Home / End key support for moving the cursor to the start / end of the text
//! - Placeholder text (displayed when the input is empty)
//! - Click to place cursor
//! - Cursor blinking
//! - Clipboard operations (copy, cut, paste)
//! - Undo/redo functionality
//! - Newline support for multi-line input
//! - Input Method Editor (IME) support for complex scripts
//! - Text validation (e.g., email format, numeric input, max length)
//! - Password-style character masking
//! - Soft-wrapping of long lines
//! - Vertical scrolling for multi-line input
//! - Horizontal scrolling for long lines
//! - Mobile pop-up keyboard support
//! - Overwrite mode (typically toggled by the `Insert` key)
//! - Bidirectional text support (e.g., mixing left-to-right and right-to-left scripts)
//! - AccessKit integration for screen readers and other assistive technologies
//! - World-space text input
//! - Text input labels (used for accessibility, tooltips or form descriptions)
//! - Input consumption (preventing other systems from receiving keyboard input events when the text input is focused)
//! - Text form submission handling
//!
//! If you require any of these features, please consider contributing it to the crate,
//! one feature at a time!
// Note: this logic is in `bevy_text`, rather than higher up in `bevy_ui` or `bevy_ui_widgets`,
// because doing so allows us to process `EditableText` in the various systems provided by `bevy_text`
// and `bevy_ui`, such as text layout and font management.

use crate::{
    text_edit::TextEdit, FontCx, FontHinting, LayoutCx, LineHeight, TextBrush, TextColor, TextFont,
    TextLayout,
};
use bevy_ecs::prelude::*;
use core::time::Duration;
use parley::{FontContext, LayoutContext, PlainEditor, SplitString};

/// Resource containing the current contents of the clipboard.
///
/// Placeholder for a proper clipboard implementation with support for the OS clipboard and non-text content.
#[derive(Resource, Default)]
pub struct Clipboard(pub String);

/// A plain-text text input field.
///
/// Please see this module docs for more details on usage and functionality.
///
/// Note that text editing operations are trickier than they might first appear,
/// due to the complexities of Unicode text handling.
///
/// As a result, we store an internal [`PlainEditor`] instance,
/// which manages both the text content and the cursor position,
/// and provides methods for applying text edits and cursor movements correctly
/// according to Unicode rules.
#[derive(Component)]
#[require(TextLayout, TextFont, TextColor, LineHeight, FontHinting)]
pub struct EditableText {
    /// A [`parley::PlainEditor`], tracking both the text content and cursor position.
    ///
    /// This serves as an analogue to [`ComputedTextBlock`](crate::ComputedTextBlock) for editable text.
    ///
    /// In most cases, you should queue text edits via the [`EditableText::queue_edit`] method instead of directly manipulating the editor,
    /// and then allow the [`apply_text_edits`] system to apply the edits at the appropriate time in the update cycle.
    ///
    /// Note that many more complex editing operations require working with [`PlainEditor::driver`].
    /// These operations should generally be batched together to avoid redundant layout work.
    // The B: Brush generic here must match the brush used by `ComputedTextBlock` to ensure that the font system is compatible.
    pub editor: PlainEditor<TextBrush>,
    /// Text edit actions that have been requested but not yet applied.
    ///
    /// These edits are processed in first-in, first-out order.
    pub pending_edits: Vec<TextEdit>,
    /// Cursor width, relative to font size
    pub cursor_width: f32,
    /// Cursor blink period in seconds.
    pub cursor_blink_period: Duration,
    /// True if a `TextEdit` was applied this frame
    pub text_edited: bool,
    /// Maximum number of characters the text input can contain.
    ///
    /// Edits which would cause the length to exceed the maximum are ignored.
    /// Does not stop setting a string longer than the maximum using `set_text`.
    pub max_characters: Option<usize>,
    /// Sets the input’s height in number of visible lines.
    pub visible_lines: Option<f32>,
    /// Allow new lines
    pub allow_newlines: bool,
}

impl Default for EditableText {
    fn default() -> Self {
        Self {
            // Defaults selected to match `Text::default()`
            editor: PlainEditor::new(100.),
            pending_edits: Vec::new(),
            cursor_width: 0.2,
            cursor_blink_period: Duration::from_secs(1),
            text_edited: false,
            max_characters: None,
            visible_lines: Some(1.),
            allow_newlines: false,
        }
    }
}

impl EditableText {
    /// Access the internal [`PlainEditor`].
    pub fn editor(&self) -> &PlainEditor<TextBrush> {
        &self.editor
    }

    /// Mutably access the internal [`PlainEditor`].
    ///
    pub fn editor_mut(&mut self) -> &mut PlainEditor<TextBrush> {
        &mut self.editor
    }

    /// Get the current text input as a [`SplitString`].
    ///
    /// A [`SplitString`] can be converted into a [`String`] using `to_string` if needed.
    pub fn value(&self) -> SplitString<'_> {
        self.editor.text()
    }

    /// Queue a [`TextEdit`] action to be applied later by the [`apply_text_edits`] system.
    pub fn queue_edit(&mut self, edit: TextEdit) {
        self.pending_edits.push(edit);
    }

    /// Applies all [`TextEdit`]s in `pending_edits` immediately, updating the [`PlainEditor`] text / cursor state accordingly.
    ///
    /// [`FontContext`] should be gathered from the [`FontCx`] resource, and [`LayoutContext`] should be gathered from the [`LayoutCx`] resource.
    pub fn apply_pending_edits(
        &mut self,
        font_context: &mut FontContext,
        layout_context: &mut LayoutContext<TextBrush>,
        clipboard_text: &mut String,
    ) {
        let Self {
            editor,
            pending_edits,
            max_characters,
            ..
        } = self;

        let mut driver = editor.driver(font_context, layout_context);

        for edit in pending_edits.drain(..) {
            edit.apply(&mut driver, clipboard_text, *max_characters);
        }
    }

    /// Clears the current input and resets the cursor position.
    pub fn clear(
        &mut self,
        font_context: &mut FontContext,
        layout_context: &mut LayoutContext<TextBrush>,
    ) {
        self.editor.set_text("");
        let mut driver = self.editor_mut().driver(font_context, layout_context);
        driver.move_to_byte(0);
        self.pending_edits.clear();
    }
}

/// Applies pending text edit actions to all [`EditableText`] widgets.
pub fn apply_text_edits(
    mut query: Query<&mut EditableText>,
    mut font_context: ResMut<FontCx>,
    mut layout_context: ResMut<LayoutCx>,
    mut clipboard_text: ResMut<Clipboard>,
) {
    for mut editable_text in query.iter_mut() {
        editable_text.text_edited = !editable_text.pending_edits.is_empty();
        editable_text.apply_pending_edits(
            &mut font_context.0,
            &mut layout_context.0,
            &mut clipboard_text.0,
        );
    }
}

use bevy_reflect::Reflect;
use parley::PlainEditorDriver;
use smol_str::SmolStr;

use crate::TextBrush;

/// crate::{FontSmoothing, TextBrush}edit and navigation actions applied by the `apply_text_edits` system.
#[derive(Debug, Clone, PartialEq, Eq, Reflect)]
pub enum TextEdit {
    /// Insert a character at the cursor. If there is a selection, replaces the selection with the character instead.
    ///
    /// Typically generated in response to keyboard text input events.
    ///
    /// This is intended to insert a single Unicode grapheme cluster, such as a letter, digit, punctuation mark, or emoji.
    Insert(SmolStr),
    /// Delete the character behind the cursor.
    /// If there is a selection, deletes the selection instead.
    ///
    /// Typically generated in response to the backspace key.
    ///
    /// This operation removes an entire Unicode grapheme cluster, which may consist of multiple bytes,
    /// shifting the cursor position accordingly.
    Backspace,
    /// Delete the character at the cursor.
    /// If there is a selection, deletes the selection instead.
    ///
    /// Typically generated in response to the delete key.
    ///
    /// This operation removes an entire Unicode grapheme cluster, which may consist of multiple bytes,
    /// shifting the cursor position accordingly.
    Delete,
    /// Moves the cursor by one position to the right.
    ///
    /// Typically generated in response to the right key.
    MoveCursorRight,
    /// Moves the cursor by one position to the left.
    ///
    /// Typically generated in response to the left key.
    MoveCursorLeft,
    /// Move selection on to the right.
    ///
    /// Typically generated in response to shift and the right key.
    SelectRight,
    /// Move selection on to the left.
    ///
    /// Typically generated in response to shift and the left key.
    SelectLeft,
}

/// Takes a `TextEdit` and applies to `PlainEditorDriver`
pub fn apply_edit<'a>(
    edit: TextEdit,
    mut driver: PlainEditorDriver<'a, TextBrush>,
) -> PlainEditorDriver<'a, TextBrush> {
    match edit {
        TextEdit::Insert(str) => driver.insert_or_replace_selection(&str),
        TextEdit::Backspace => driver.backdelete(),
        TextEdit::Delete => driver.delete(),
        TextEdit::MoveCursorRight => driver.move_right(),
        TextEdit::MoveCursorLeft => driver.move_left(),
        TextEdit::SelectRight => driver.select_right(),
        TextEdit::SelectLeft => driver.select_left(),
    }
    driver
}

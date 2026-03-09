use parley::PlainEditorDriver;
use smol_str::SmolStr;

use crate::FontSmoothing;

/// Deferred text input edit and navigation actions applied by the `apply_text_edits` system.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TextEdit {
    /// Insert a character at the cursor. If there is a selection, replaces the selection with the character instead.
    ///
    /// Typically generated in response to keyboard text input events.
    ///
    /// This is intended to insert a single Unicode grapheme cluster, such as a letter, digit, punctuation mark, or emoji.
    /// Ordinarily, this is derived from [`KeyboardInput::logical_key`](bevy_input::keyboard::KeyboardInput::logical_key),
    /// which stores a [`SmolStr`] inside of the [`Key::Character`] variant, which may represent multiple bytes.
    Insert(SmolStr),
    /// Delete the character behind the cursor.
    /// If there is a selection, deletes the selection instead.
    ///
    /// Typically generated in response to the [`Backspace`](Key::Backspace) key.
    ///
    /// This operation removes an entire Unicode grapheme cluster, which may consist of multiple bytes,
    /// shifting the cursor position accordingly.
    Backspace,
    /// Delete the character at the cursor.
    /// If there is a selection, deletes the selection instead.
    ///
    /// Typically generated in response to the [`Delete`](Key::Delete) key.
    ///
    /// This operation removes an entire Unicode grapheme cluster, which may consist of multiple bytes,
    /// shifting the cursor position accordingly.
    Delete,
    /// Moves the cursor by one position to the right.
    ///
    /// Typically generated in response to the [`Right`](Key::Right) key.
    MoveCursorRight,
    /// Moves the cursor by one position to the left.
    ///
    /// Typically generated in response to the [`Left`](Key::Left) key.
    MoveCursorLeft,
}

/// Takes a `TextEdit` and applies to `PlainEditorDriver`
pub fn apply_edit<'a>(
    edit: TextEdit,
    mut driver: PlainEditorDriver<'a, (u32, FontSmoothing)>,
) -> PlainEditorDriver<'a, (u32, FontSmoothing)> {
    match edit {
        TextEdit::Insert(str) => driver.insert_or_replace_selection(&str),
        TextEdit::Backspace => driver.backdelete(),
        TextEdit::Delete => driver.delete(),
        TextEdit::MoveCursorRight => driver.move_right(),
        TextEdit::MoveCursorLeft => driver.move_left(),
    }
    return driver;
}

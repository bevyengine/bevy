use bevy_reflect::Reflect;
use parley::PlainEditorDriver;
use smol_str::SmolStr;

use crate::TextBrush;

/// Deferred text input edit and navigation actions applied by the `apply_text_edits` system.
#[derive(Debug, Clone, PartialEq)]
pub enum TextEdit {
    /// Copy the current selection into the clipboard.
    ///
    /// Typically generated in response to copy commands such as Ctrl + C or Cmd + C.
    Copy,
    /// Copy the current selection into the clipboard, and the delete the selected text.
    ///
    /// Typically generated in response to cut commands such as Ctrl + X or Cmd + X.
    Cut,
    /// Paste the current clipboard contents at the cursor. If there is a selection, replaces the selection with the clipboard contents instead.
    ///
    /// Typically generated in response to paste commands such as Ctrl + V or Cmd + V.
    Paste,
    /// Insert a character or string at the cursor. If there is a selection, replaces the selection with the character instead.
    ///
    /// Typically generated in response to keyboard text input events.
    ///
    /// This is intended to insert a single Unicode grapheme cluster, such as a letter, digit, punctuation mark, or emoji.
    /// Ordinarily, this is derived from `bevy_input::keyboard::KeyboardInput::logical_key`,
    /// which stores a [`SmolStr`] inside of the `Key::Character` variant, which may represent multiple bytes.
    Insert(SmolStr),
    /// Delete the character behind the cursor.
    /// If there is a selection, deletes the selection instead.
    ///
    /// Typically generated in response to the Backspace key.
    ///
    /// This operation removes an entire Unicode grapheme cluster, which may consist of multiple bytes,
    /// shifting the cursor position accordingly.
    Backspace,
    /// Delete the word behind the cursor.
    /// If there is a selection, deletes the selection instead.
    ///
    /// Typically generated in response to Ctrl + Backspace or Option + Backspace.
    BackspaceWord,
    /// Delete the character at the cursor.
    /// If there is a selection, deletes the selection instead.
    ///
    /// Typically generated in response to the Delete key.
    ///
    /// This operation removes an entire Unicode grapheme cluster, which may consist of multiple bytes,
    /// shifting the cursor position accordingly.
    Delete,
    /// Delete the word at the cursor.
    /// If there is a selection, deletes the selection instead.
    ///
    /// Typically generated in response to Ctrl + Delete or Option + Delete.
    DeleteWord,
    /// Moves the cursor by one position to the left.
    /// `true` moves and extends selection.
    ///
    /// Typically generated in response to the Left key.
    Left(bool),
    /// Moves the cursor by one position to the right.
    /// `true` moves and extends selection.
    ///
    /// Typically generated in response to the Right key.
    Right(bool),
    /// Moves the cursor a word to the left.
    /// `true` moves and extends selection.
    ///
    /// Typically generated in response to Ctrl + Left or Option + Left.
    WordLeft(bool),
    /// Moves the cursor a word to the right.
    /// `true` moves and extends selection.
    ///
    /// Typically generated in response to Ctrl + Right or Option + Right.
    WordRight(bool),
    /// Moves the cursor up by one visual line.
    /// `true` moves and extends selection.
    ///
    /// Typically generated in response to the Up key.
    Up(bool),
    /// Moves the cursor down by one visual line.
    /// `true` moves and extends selection.
    ///
    /// Typically generated in response to the Down key.
    Down(bool),
    /// Moves the cursor to the start of the text.
    /// `true` moves and extends selection.
    ///
    /// Typically generated in response to Ctrl + Home or Command + Up.
    TextStart(bool),
    /// Moves the cursor to the end of the text.
    /// `true` moves and extends selection.
    ///
    /// Typically generated in response to Ctrl + End or Command + Down.
    TextEnd(bool),
    /// Moves the cursor to the start of the current hard line.
    /// A hardline is a line seperated by a newline character.
    /// `true` moves and extends selection.
    ///
    /// Typically generated in response to Command + Left.
    HardLineStart(bool),
    /// Moves the cursor to the end of the current hard line.
    /// `true` moves and extends selection.
    ///
    /// Typically generated in response to Command + Right.
    HardLineEnd(bool),
    /// Moves the cursor to the start of the current visual line.
    /// `true` moves and extends selection.
    ///
    /// Typically generated in response to the Home key.
    LineStart(bool),
    /// Moves the cursor to the end of the current visual line.
    /// `true` moves and extends selection.
    ///
    /// Typically generated in response to the End key.
    LineEnd(bool),
    /// Collapses the current selection to a caret.
    ///
    /// Typically generated in response to the Escape key.
    CollapseSelection,
    /// Selects all text.
    ///
    /// Typically generated in response to select-all commands such as Ctrl + A or Cmd + A.
    SelectAll,
    /// Moves the cursor to the given point.
    ///
    /// Typically generated in response to a pointer press within the text area.
    MoveToPoint(Vec2),
    /// Extends the current selection to the given point.
    ///
    /// Typically generated in response to dragging a pointer within the text area.
    ExtendSelectionToPoint(Vec2),
    /// Extends the current selection from the existing anchor to the given point.
    ///
    /// Typically generated in response to shift-clicking within the text area.
    ShiftClickExtension(Vec2),
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

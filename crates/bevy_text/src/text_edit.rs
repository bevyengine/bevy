use bevy_math::Vec2;
use bevy_reflect::Reflect;
use parley::PlainEditorDriver;
use smol_str::SmolStr;

use crate::TextBrush;

/// A selection within IME preedit text, expressed as byte offsets from the start of the preedit.
///
/// The anchor and focus map directly onto parley's `Selection::new(anchor, focus)` when
/// the preedit is applied. If `anchor == focus`, the selection is a caret.
///
/// This corresponds to [`ImePredit::Commit.cursor`](https://docs.rs/bevy/latest/bevy/prelude/enum.Ime.html#variant.Preedit.field.cursor)
/// from `bevy_window`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Reflect)]
pub struct PreeditCursor {
    /// Anchor byte offset within the preedit text.
    pub anchor: usize,
    /// Focus (caret) byte offset within the preedit text.
    pub focus: usize,
}

/// Deferred text input edit and navigation actions applied by the `apply_text_edits` system.
#[derive(Debug, Clone, PartialEq, Reflect)]
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
    /// A hardline is a line separated by a newline character.
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
    /// Set the IME preedit/composing text at the cursor, or clear it if `value` is empty.
    ///
    /// The preedit text is excluded from [`EditableText::value`](crate::EditableText::value).
    /// `cursor` describes the selection within the preedit text, or `None` to hide the cursor.
    ///
    /// Passing an empty `value` clears any in-progress composition; `cursor` is ignored in
    /// that case. Use [`TextEdit::clear_ime_compose`] as a convenience constructor.
    ///
    /// Typically generated in response to [`bevy_window::Ime::Preedit`] events.
    ///
    /// [`bevy_window::Ime::Preedit`]: https://docs.rs/bevy/latest/bevy/prelude/enum.Ime.html#variant.Preedit
    ImeSetCompose {
        /// The current preedit string. An empty string clears the composition.
        value: SmolStr,
        /// Selection within the preedit text, or `None` to hide the cursor.
        cursor: Option<PreeditCursor>,
    },
    /// Accept IME composition and insert `value` at the cursor.
    ///
    /// Clears any in-progress preedit first, then inserts the committed string,
    /// respecting [`EditableText::max_characters`](crate::EditableText::max_characters)
    /// and the `char_filter`.
    ///
    /// Typically generated in response to [`bevy_window::Ime::Commit`] events.
    ///
    /// [`bevy_window::Ime::Commit`]: https://docs.rs/bevy/latest/bevy/prelude/enum.Ime.html#variant.Commit
    ImeCommit {
        /// The committed text to insert at the cursor.
        value: SmolStr,
    },
}

impl TextEdit {
    /// Convenience constructor for a [`TextEdit::ImeSetCompose`] that clears the preedit.
    pub fn clear_ime_compose() -> Self {
        Self::ImeSetCompose {
            value: SmolStr::new_inline(""),
            cursor: None,
        }
    }

    /// Apply the `TextEdit` to the text editor driver
    pub fn apply<'a>(
        self,
        driver: &'a mut PlainEditorDriver<TextBrush>,
        clipboard_text: &mut String,
        max_characters: Option<usize>,
        char_filter: impl Fn(char) -> bool,
    ) {
        match self {
            TextEdit::Copy => {
                if let Some(text) = driver.editor.selected_text() {
                    clipboard_text.clear();
                    clipboard_text.push_str(text);
                }
            }
            TextEdit::Cut => {
                if let Some(text) = driver.editor.selected_text() {
                    clipboard_text.clear();
                    clipboard_text.push_str(text);
                    driver.delete();
                }
            }
            TextEdit::Paste => {
                if !clipboard_text.chars().all(char_filter) {
                    return;
                }
                if let Some(max) = max_characters {
                    let select_len = driver
                        .editor
                        .selected_text()
                        .map(str::chars)
                        .map(Iterator::count)
                        .unwrap_or(0);
                    if max
                        < driver.editor.text().chars().count() - select_len
                            + clipboard_text.chars().count()
                    {
                        return;
                    }
                }
                driver.insert_or_replace_selection(clipboard_text.as_str());
            }
            TextEdit::Insert(text) => {
                if !text.chars().all(char_filter) {
                    return;
                }
                if let Some(max) = max_characters {
                    let select_len = driver
                        .editor
                        .selected_text()
                        .map(str::chars)
                        .map(Iterator::count)
                        .unwrap_or(0);
                    if max
                        < driver.editor.text().chars().count() - select_len + text.chars().count()
                    {
                        return;
                    }
                }
                driver.insert_or_replace_selection(text.as_str());
            }
            TextEdit::Backspace => driver.backdelete(),
            TextEdit::BackspaceWord => driver.backdelete_word(),
            TextEdit::Delete => driver.delete(),
            TextEdit::DeleteWord => driver.delete_word(),
            TextEdit::Left(false) => driver.move_left(),
            TextEdit::Right(false) => driver.move_right(),
            TextEdit::WordLeft(false) => driver.move_word_left(),
            TextEdit::WordRight(false) => driver.move_word_right(),
            TextEdit::Up(false) => driver.move_up(),
            TextEdit::Down(false) => driver.move_down(),
            TextEdit::TextStart(false) => driver.move_to_text_start(),
            TextEdit::TextEnd(false) => driver.move_to_text_end(),
            TextEdit::HardLineStart(false) => driver.move_to_hard_line_start(),
            TextEdit::HardLineEnd(false) => driver.move_to_hard_line_end(),
            TextEdit::LineStart(false) => driver.move_to_line_start(),
            TextEdit::LineEnd(false) => driver.move_to_line_end(),
            TextEdit::Left(true) => driver.select_left(),
            TextEdit::Right(true) => driver.select_right(),
            TextEdit::WordLeft(true) => driver.select_word_left(),
            TextEdit::WordRight(true) => driver.select_word_right(),
            TextEdit::Up(true) => driver.select_up(),
            TextEdit::Down(true) => driver.select_down(),
            TextEdit::TextStart(true) => driver.select_to_text_start(),
            TextEdit::TextEnd(true) => driver.select_to_text_end(),
            TextEdit::HardLineStart(true) => driver.select_to_hard_line_start(),
            TextEdit::HardLineEnd(true) => driver.select_to_hard_line_end(),
            TextEdit::LineStart(true) => driver.select_to_line_start(),
            TextEdit::LineEnd(true) => driver.select_to_line_end(),
            TextEdit::CollapseSelection => driver.collapse_selection(),
            TextEdit::SelectAll => driver.select_all(),
            TextEdit::MoveToPoint(point) => driver.move_to_point(point.x, point.y),
            TextEdit::ExtendSelectionToPoint(point) => {
                driver.extend_selection_to_point(point.x, point.y);
            }
            TextEdit::ShiftClickExtension(point) => driver.shift_click_extension(point.x, point.y),
            TextEdit::ImeSetCompose { value, cursor } => {
                if value.is_empty() {
                    driver.clear_compose();
                } else {
                    let cursor = cursor.map(|c| (c.anchor, c.focus));
                    driver.set_compose(&value, cursor);
                }
            }
            TextEdit::ImeCommit { value: text } => {
                driver.clear_compose();
                if text.chars().all(&char_filter)
                    && max_characters.is_none_or(|max| {
                        driver.editor.text().chars().count() + text.chars().count() <= max
                    })
                {
                    driver.insert_or_replace_selection(text.as_str());
                }
            }
        }
    }
}

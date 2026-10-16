//! Password-style character masking for [`EditableText`](crate::EditableText),
//! see [`EditableText::mask`](crate::EditableText::mask).
//!
//! While masked, the editor's buffer holds one glyph per entered character and
//! the entered text lives in [`MaskState`]. The editor lays out the glyphs
//! itself, so cursor, selection and click positions are exact, and since every
//! glyph has the same byte length, editor offsets map to char indices by
//! division.

use crate::{scroll::TextViewport, text_edit::reveal_cursor, MaskState, TextBrush, TextEdit};
use alloc::string::String;
use bevy_math::Vec2;
use core::ops::Range;
use parley::{PlainEditor, PlainEditorDriver};

/// The entered text of a masked field, and the glyph the editor shows for it.
#[derive(Clone)]
pub(crate) struct Masked {
    pub(crate) glyph: char,
    pub(crate) value: String,
}

impl Masked {
    /// One glyph per char of the entered text.
    fn glyphs(&self) -> String {
        glyphs(self.glyph, self.value.chars().count())
    }
}

fn glyphs(glyph: char, n: usize) -> String {
    core::iter::repeat_n(glyph, n).collect()
}

/// Whether `editor` shows exactly the glyphs for `masked`.
pub(crate) fn shows(editor: &PlainEditor<TextBrush>, masked: &Masked) -> bool {
    let text = editor.raw_text();
    !editor.is_composing()
        && text.len() == masked.glyph.len_utf8() * masked.value.chars().count()
        && text.chars().all(|c| c == masked.glyph)
}

/// Brings the editor in line with `mask`: conceals, reveals or swaps glyphs,
/// keeping the selection by char index.
///
/// While masked, any other editor content was written around the edit queue
/// and is discarded: the entered text is never read back from the editor.
pub(crate) fn sync(
    mask: Option<char>,
    state: &mut MaskState,
    driver: &mut PlainEditorDriver<TextBrush>,
) {
    let Some(glyph) = mask else {
        // Reveal.
        if let Some(masked) = state.0.take() {
            let selection = char_selection(driver.editor);
            show(driver, &masked.value, selection);
        }
        return;
    };
    match state.0 {
        // Conceal. A preedit would end up in the value, drop it first.
        None => {
            driver.clear_compose();
            let masked = Masked {
                glyph,
                value: driver.editor.raw_text().into(),
            };
            let selection = char_selection(driver.editor);
            show(driver, &masked.glyphs(), selection);
            state.0 = Some(masked);
        }
        Some(ref mut masked) => {
            let intact = shows(driver.editor, masked);
            if intact && glyph == masked.glyph {
                return;
            }
            if !intact {
                bevy_log::warn_once!(
                    "A masked `EditableText`'s editor was changed outside its edit queue; the change was discarded."
                );
                driver.clear_compose();
            }
            let selection = char_selection(driver.editor);
            masked.glyph = glyph;
            show(driver, &masked.glyphs(), selection);
        }
    }
}

/// The selection's anchor and focus as char indices. Indices past the end or
/// off a char boundary (stale after an outside `set_text`) clamp to the end.
fn char_selection(editor: &PlainEditor<TextBrush>) -> (usize, usize) {
    let text = editor.raw_text();
    let chars_before = |byte: usize| {
        text.get(..byte)
            .map_or_else(|| text.chars().count(), |s| s.chars().count())
    };
    let selection = editor.raw_selection();
    (
        chars_before(selection.anchor().index()),
        chars_before(selection.focus().index()),
    )
}

/// Replaces the editor's text and restores a char-index selection.
fn show(driver: &mut PlainEditorDriver<TextBrush>, text: &str, (anchor, focus): (usize, usize)) {
    driver.editor.set_text(text);
    driver.select_byte_range(byte_at(text, anchor), byte_at(text, focus));
}

fn byte_at(text: &str, char_idx: usize) -> usize {
    text.char_indices()
        .nth(char_idx)
        .map_or(text.len(), |(byte, _)| byte)
}

/// Replaces the chars in `range` (char indices) of the entered text.
fn splice_chars(value: &mut String, range: Range<usize>, replacement: &str) {
    let (start, end) = (byte_at(value, range.start), byte_at(value, range.end));
    value.replace_range(start..end, replacement);
}

/// The current selection as a char range into the all-glyph editor text.
fn selection_char_range(driver: &PlainEditorDriver<TextBrush>, glyph: char) -> Range<usize> {
    let range = driver.editor.raw_selection().text_range();
    range.start / glyph.len_utf8()..range.end / glyph.len_utf8()
}

fn editor_char_len(driver: &PlainEditorDriver<TextBrush>, glyph: char) -> usize {
    driver.editor.raw_text().len() / glyph.len_utf8()
}

/// Inserts `text` into a masked field: the entered chars replace the
/// selection in the value, the editor gets the same number of glyphs.
/// `char_filter` and `max_characters` check the entered chars; the editor
/// insert bypasses the filter, which the glyph itself may fail.
pub(crate) fn masked_insert(
    text: &str,
    masked: &mut Masked,
    driver: &mut PlainEditorDriver<TextBrush>,
    max_characters: Option<usize>,
    char_filter: &impl Fn(char) -> bool,
) {
    if !text.chars().all(char_filter) {
        bevy_log::debug!("Masked insert rejected by char filter.");
        return;
    }
    let selection = selection_char_range(driver, masked.glyph);
    let inserted = text.chars().count();
    if let Some(max) = max_characters
        && max < editor_char_len(driver, masked.glyph) - selection.len() + inserted
    {
        return;
    }
    splice_chars(&mut masked.value, selection, text);
    driver.insert_or_replace_selection(&glyphs(masked.glyph, inserted));
}

/// Applies a deletion-style op and mirrors the removed char range on the
/// entered text.
fn mirrored_delete<'e>(
    masked: &mut Masked,
    driver: &mut PlainEditorDriver<'e, TextBrush>,
    op: impl FnOnce(&mut PlainEditorDriver<'e, TextBrush>),
) {
    let len_before = editor_char_len(driver, masked.glyph);
    op(driver);
    let removed = len_before.saturating_sub(editor_char_len(driver, masked.glyph));
    if removed > 0 {
        let caret = selection_char_range(driver, masked.glyph).start;
        splice_chars(&mut masked.value, caret..caret + removed, "");
    }
}

/// The masked counterpart of [`TextEdit::apply`]: edits that change the text
/// keep the editor all-glyphs and the entered text in `masked`; cursor,
/// selection and scroll edits pass through.
#[expect(
    clippy::too_many_arguments,
    reason = "mirrors TextEdit::apply's parameters plus the masked state"
)]
pub(crate) fn apply_masked_edit(
    edit: TextEdit,
    masked: &mut Masked,
    driver: &mut PlainEditorDriver<TextBrush>,
    viewport: &mut TextViewport,
    cursor_margin: Vec2,
    clipboard: &mut bevy_clipboard::Clipboard,
    max_characters: Option<usize>,
    char_filter: &impl Fn(char) -> bool,
) {
    match edit {
        // Never put the entered text (or useless glyphs) on the clipboard.
        TextEdit::Copy => {}
        // Cut only deletes; kept apart from `Delete`, which also deletes
        // without a selection.
        TextEdit::Cut => {
            if driver.editor.selected_text().is_some() {
                mirrored_delete(masked, driver, PlainEditorDriver::delete);
                reveal_cursor(driver, viewport, cursor_margin);
            }
        }
        TextEdit::Insert(text) => {
            masked_insert(text.as_str(), masked, driver, max_characters, char_filter);
            reveal_cursor(driver, viewport, cursor_margin);
        }
        // `apply_pending_edits` resolves pastes before they get here.
        TextEdit::Paste => {
            bevy_log::debug!("TextEdit::Paste ignored in masked apply path.");
        }
        TextEdit::Backspace => {
            mirrored_delete(masked, driver, PlainEditorDriver::backdelete);
            reveal_cursor(driver, viewport, cursor_margin);
        }
        // Word boundaries would leak the text: clear to the start. A selection
        // is deleted alone, as unmasked.
        TextEdit::BackspaceWord => {
            if driver.editor.selected_text().is_some() {
                mirrored_delete(masked, driver, PlainEditorDriver::delete);
            } else {
                let end = selection_char_range(driver, masked.glyph).end;
                if end > 0 {
                    splice_chars(&mut masked.value, 0..end, "");
                    driver.editor.set_text(&masked.glyphs());
                    driver.move_to_text_start();
                }
            }
            reveal_cursor(driver, viewport, cursor_margin);
        }
        TextEdit::Delete => {
            mirrored_delete(masked, driver, PlainEditorDriver::delete);
            reveal_cursor(driver, viewport, cursor_margin);
        }
        // Clear to the end, as `BackspaceWord` clears to the start.
        TextEdit::DeleteWord => {
            if driver.editor.selected_text().is_some() {
                mirrored_delete(masked, driver, PlainEditorDriver::delete);
            } else {
                let start = selection_char_range(driver, masked.glyph).start;
                if start < editor_char_len(driver, masked.glyph) {
                    splice_chars(&mut masked.value, start..usize::MAX, "");
                    driver.editor.set_text(&masked.glyphs());
                    driver.move_to_text_end();
                }
            }
            reveal_cursor(driver, viewport, cursor_margin);
        }
        // IME commits are entered text: route them through the masked insert.
        TextEdit::ImeCommit { value: commit } => {
            driver.clear_compose();
            masked_insert(commit.as_str(), masked, driver, max_characters, char_filter);
            reveal_cursor(driver, viewport, cursor_margin);
        }
        // A preedit renders in the buffer, so it would show the raw text.
        // Suppressing it makes IME entry commit-per-key, like platform
        // secure-entry keyboards.
        TextEdit::ImeSetCompose { .. } => driver.clear_compose(),
        // Listed rather than a catch-all, so a new text-changing variant
        // can't reach the plain path unnoticed.
        other @ (TextEdit::Left(_)
        | TextEdit::Right(_)
        | TextEdit::WordLeft(_)
        | TextEdit::WordRight(_)
        | TextEdit::Up(_)
        | TextEdit::Down(_)
        | TextEdit::TextStart(_)
        | TextEdit::TextEnd(_)
        | TextEdit::HardLineStart(_)
        | TextEdit::HardLineEnd(_)
        | TextEdit::LineStart(_)
        | TextEdit::LineEnd(_)
        | TextEdit::CollapseSelection
        | TextEdit::SelectAll
        | TextEdit::SelectAllIfCollapsed
        | TextEdit::MoveToPoint(_)
        | TextEdit::SelectWordAtPoint(_)
        | TextEdit::SelectLineAtPoint(_)
        | TextEdit::SelectedHardLineAtPoint(_)
        | TextEdit::ExtendSelectionToPoint(_)
        | TextEdit::ShiftClickExtension(_)
        | TextEdit::ScrollByLines(_)
        | TextEdit::ScrollTo(_)
        | TextEdit::ScrollBy(_)) => other.apply(
            driver,
            viewport,
            cursor_margin,
            clipboard,
            max_characters,
            char_filter,
        ),
    }
}

#[cfg(test)]
mod tests {
    use crate::{EditableText, FontCx, LayoutCx, TextEdit};
    use alloc::string::{String, ToString};
    use bevy_app::App;
    use bevy_ecs::prelude::*;

    fn app_with_field(mut text: EditableText) -> (App, Entity) {
        let mut app = App::new();
        app.init_resource::<FontCx>();
        app.init_resource::<LayoutCx>();
        app.init_resource::<bevy_clipboard::Clipboard>();
        // an empty font collection makes every layout-dependent edit
        // (cursor motion, SelectAll, cluster deletes) degenerate to a
        // no-op -- register the same subset font text_edit's tests use
        let font = crate::Font::from_bytes(include_bytes!("FiraMono-subset.ttf").to_vec());
        app.world_mut()
            .resource_mut::<FontCx>()
            .collection
            .register_fonts(font.data, None);
        app.add_systems(bevy_app::Update, crate::apply_text_edits);

        text.editor.edit_styles().insert(
            parley::FontFamilyName::Named(alloc::borrow::Cow::Borrowed("Fira Mono")).into(),
        );
        let entity = app.world_mut().spawn(text).id();
        app.update();
        (app, entity)
    }

    fn masked_field(initial: &str) -> (App, Entity) {
        app_with_field(EditableText {
            mask: Some('*'),
            ..EditableText::new(initial)
        })
    }

    fn text_mut(app: &mut App, entity: Entity) -> Mut<'_, EditableText> {
        app.world_mut().get_mut::<EditableText>(entity).unwrap()
    }

    fn queue(app: &mut App, entity: Entity, edit: TextEdit) {
        text_mut(app, entity).queue_edit(edit);
        app.update();
    }

    fn set_mask(app: &mut App, entity: Entity, mask: Option<char>) {
        text_mut(app, entity).mask = mask;
        app.update();
    }

    fn select_chars(app: &mut App, entity: Entity, range: core::ops::Range<usize>) {
        queue(app, entity, TextEdit::TextStart(false));
        for _ in 0..range.start {
            queue(app, entity, TextEdit::Right(false));
        }
        for _ in range {
            queue(app, entity, TextEdit::Right(true));
        }
    }

    fn real(app: &App, entity: Entity) -> String {
        app.world()
            .get::<EditableText>(entity)
            .unwrap()
            .value()
            .to_string()
    }

    fn shown(app: &App, entity: Entity) -> String {
        app.world()
            .get::<EditableText>(entity)
            .unwrap()
            .editor_text()
            .to_string()
    }

    #[test]
    fn conceals_prefilled_text() {
        let (app, entity) = masked_field("secret");
        assert_eq!(real(&app, entity), "secret");
        assert_eq!(shown(&app, entity), "******");
    }

    #[test]
    fn masking_later_conceals_without_edits() {
        let (mut app, entity) = app_with_field(EditableText::new("abc"));
        assert_eq!(shown(&app, entity), "abc");
        set_mask(&mut app, entity, Some('*'));
        assert_eq!(real(&app, entity), "abc");
        assert_eq!(shown(&app, entity), "***");
    }

    #[test]
    fn typing_masks_and_accumulates() {
        let (mut app, entity) = masked_field("");
        queue(&mut app, entity, TextEdit::Insert("a".into()));
        queue(&mut app, entity, TextEdit::Insert("b".into()));
        assert_eq!(real(&app, entity), "ab");
        assert_eq!(shown(&app, entity), "**");
    }

    #[test]
    fn backspace_mirrors_on_real_value() {
        let (mut app, entity) = masked_field("abc");
        queue(&mut app, entity, TextEdit::TextEnd(false));
        queue(&mut app, entity, TextEdit::Backspace);
        assert_eq!(real(&app, entity), "ab");
        assert_eq!(shown(&app, entity), "**");
    }

    #[test]
    fn delete_at_start_mirrors_position() {
        let (mut app, entity) = masked_field("abc");
        queue(&mut app, entity, TextEdit::TextStart(false));
        queue(&mut app, entity, TextEdit::Delete);
        assert_eq!(real(&app, entity), "bc");
    }

    #[test]
    fn delete_word_clears_to_end() {
        let (mut app, entity) = masked_field("abcdef");
        queue(&mut app, entity, TextEdit::TextStart(false));
        queue(&mut app, entity, TextEdit::Right(false));
        queue(&mut app, entity, TextEdit::Right(false));
        queue(&mut app, entity, TextEdit::DeleteWord);
        assert_eq!(real(&app, entity), "ab");
        assert_eq!(shown(&app, entity), "**");
    }

    #[test]
    fn backspace_word_clears_to_start() {
        let (mut app, entity) = masked_field("abcdef");
        queue(&mut app, entity, TextEdit::TextEnd(false));
        queue(&mut app, entity, TextEdit::BackspaceWord);
        assert_eq!(real(&app, entity), "");
        assert_eq!(shown(&app, entity), "");
    }

    #[test]
    fn backspace_word_deletes_only_the_selection() {
        let (mut app, entity) = masked_field("abcdef");
        select_chars(&mut app, entity, 2..4);
        queue(&mut app, entity, TextEdit::BackspaceWord);
        assert_eq!(real(&app, entity), "abef");
        assert_eq!(shown(&app, entity), "****");
        // the caret is left where the selection started
        queue(&mut app, entity, TextEdit::Insert("X".into()));
        assert_eq!(real(&app, entity), "abXef");
    }

    #[test]
    fn delete_word_deletes_only_the_selection() {
        let (mut app, entity) = masked_field("abcdef");
        select_chars(&mut app, entity, 2..4);
        queue(&mut app, entity, TextEdit::DeleteWord);
        assert_eq!(real(&app, entity), "abef");
        assert_eq!(shown(&app, entity), "****");
        queue(&mut app, entity, TextEdit::Insert("X".into()));
        assert_eq!(real(&app, entity), "abXef");
    }

    #[test]
    fn typing_over_selection_replaces_range() {
        let (mut app, entity) = masked_field("abcd");
        queue(&mut app, entity, TextEdit::SelectAll);
        queue(&mut app, entity, TextEdit::Insert("z".into()));
        assert_eq!(real(&app, entity), "z");
        assert_eq!(shown(&app, entity), "*");
    }

    #[test]
    fn cut_without_selection_is_a_noop() {
        let (mut app, entity) = masked_field("abc");
        queue(&mut app, entity, TextEdit::TextStart(false));
        queue(&mut app, entity, TextEdit::Cut);
        // collapsed cursor: Cut must not forward-delete like Delete would
        assert_eq!(real(&app, entity), "abc");
    }

    #[test]
    fn copy_and_cut_never_reach_the_clipboard() {
        let (mut app, entity) = masked_field("secret");
        app.world_mut()
            .resource_mut::<bevy_clipboard::Clipboard>()
            .set_text("sentinel")
            .unwrap();
        queue(&mut app, entity, TextEdit::SelectAll);
        queue(&mut app, entity, TextEdit::Copy);
        queue(&mut app, entity, TextEdit::Cut);
        // cut still deletes...
        assert_eq!(real(&app, entity), "");
        // ...but the clipboard is untouched
        let mut read = app
            .world_mut()
            .resource_mut::<bevy_clipboard::Clipboard>()
            .fetch_text();
        assert_eq!(read.poll_result().unwrap().unwrap(), "sentinel");
    }

    #[test]
    fn filter_applies_to_real_chars_not_glyphs() {
        // a digit filter must accept '5' even though the glyph fails it
        let (mut app, entity) = masked_field("");
        app.world_mut()
            .entity_mut(entity)
            .insert(crate::EditableTextFilter::new(|c| c.is_ascii_digit()));
        queue(&mut app, entity, TextEdit::Insert("5".into()));
        queue(&mut app, entity, TextEdit::Insert("x".into()));
        assert_eq!(real(&app, entity), "5");
        assert_eq!(shown(&app, entity), "*");
    }

    #[test]
    fn unmasking_reveals() {
        let (mut app, entity) = masked_field("secret");
        set_mask(&mut app, entity, None);
        assert_eq!(shown(&app, entity), "secret");
        assert_eq!(real(&app, entity), "secret");
    }

    #[test]
    fn changing_the_glyph_keeps_the_value() {
        // '•' is 3 bytes in UTF-8, so editor offsets change scale with the glyph
        let (mut app, entity) = masked_field("secret");
        set_mask(&mut app, entity, Some('•'));
        assert_eq!(real(&app, entity), "secret");
        assert_eq!(shown(&app, entity), "••••••");
        queue(&mut app, entity, TextEdit::TextEnd(false));
        queue(&mut app, entity, TextEdit::Backspace);
        queue(&mut app, entity, TextEdit::Insert("!".into()));
        assert_eq!(real(&app, entity), "secre!");
        assert_eq!(shown(&app, entity), "••••••");
    }

    #[test]
    fn toggling_keeps_the_caret() {
        let (mut app, entity) = masked_field("secret");
        queue(&mut app, entity, TextEdit::TextStart(false));
        for _ in 0..3 {
            queue(&mut app, entity, TextEdit::Right(false));
        }
        set_mask(&mut app, entity, None);
        queue(&mut app, entity, TextEdit::Insert("X".into()));
        assert_eq!(shown(&app, entity), "secXret");
        set_mask(&mut app, entity, Some('*'));
        queue(&mut app, entity, TextEdit::Insert("Y".into()));
        assert_eq!(real(&app, entity), "secXYret");
    }

    #[test]
    fn direct_editor_writes_are_discarded() {
        let (mut app, entity) = masked_field("old");
        text_mut(&mut app, entity).editor.set_text("new!");
        app.update();
        assert_eq!(real(&app, entity), "old");
        assert_eq!(shown(&app, entity), "***");
    }

    #[test]
    fn direct_text_edit_apply_is_discarded() {
        let (mut app, entity) = masked_field("old");
        queue(&mut app, entity, TextEdit::TextEnd(false));
        let world = app.world_mut();
        world.resource_scope(|world, mut font_cx: Mut<FontCx>| {
            world.resource_scope(|world, mut layout_cx: Mut<LayoutCx>| {
                world.resource_scope(|world, mut clipboard: Mut<bevy_clipboard::Clipboard>| {
                    let mut text = world.get_mut::<EditableText>(entity).unwrap();
                    let EditableText {
                        editor,
                        viewport,
                        cursor_margin,
                        ..
                    } = &mut *text;
                    let mut driver = editor.driver(&mut font_cx, &mut layout_cx.0);
                    TextEdit::Insert("x".into()).apply(
                        &mut driver,
                        viewport,
                        *cursor_margin,
                        &mut clipboard,
                        None,
                        |_| true,
                    );
                });
            });
        });
        assert_eq!(shown(&app, entity), "***x");
        app.update();
        assert_eq!(real(&app, entity), "old");
        assert_eq!(shown(&app, entity), "***");
    }

    #[test]
    fn ime_commit_routes_through_the_mask() {
        // the soft-keyboard (mobile) input path
        let (mut app, entity) = masked_field("");
        queue(
            &mut app,
            entity,
            TextEdit::ImeCommit {
                value: "abc".into(),
            },
        );
        assert_eq!(real(&app, entity), "abc");
        assert_eq!(shown(&app, entity), "***");
    }

    #[test]
    fn value_is_honest_while_masked() {
        let (mut app, entity) = masked_field("secret");
        queue(&mut app, entity, TextEdit::TextEnd(false));
        queue(&mut app, entity, TextEdit::Insert("!".into()));
        assert_eq!(real(&app, entity), "secret!");
        assert_eq!(shown(&app, entity), "*******");
    }

    #[test]
    fn mask_state_present_iff_masked() {
        let (mut app, entity) = masked_field("abc");
        assert!(text_mut(&mut app, entity).mask_state.0.is_some());
        set_mask(&mut app, entity, None);
        assert!(text_mut(&mut app, entity).mask_state.0.is_none());
    }

    #[test]
    fn multibyte_value_survives_conceal_and_reveal() {
        // one glyph per char, not per byte; the reveal restores the exact string
        let (mut app, entity) = masked_field("pé🔑");
        assert_eq!(real(&app, entity), "pé🔑");
        assert_eq!(shown(&app, entity), "***");
        set_mask(&mut app, entity, None);
        assert_eq!(shown(&app, entity), "pé🔑");
    }

    #[test]
    fn clear_clears_the_value() {
        let (mut app, entity) = masked_field("secret");
        text_mut(&mut app, entity).clear();
        assert_eq!(real(&app, entity), "");
        app.update();
        assert_eq!(real(&app, entity), "");
        assert_eq!(shown(&app, entity), "");
    }
}

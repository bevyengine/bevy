//! Password-style character masking for [`EditableText`].
//!
//! See [`CharacterMask`].

use crate::{scroll::TextViewport, text_edit::reveal_cursor, EditableText, TextBrush, TextEdit};
use bevy_ecs::lifecycle::HookContext;
use bevy_ecs::prelude::*;
use bevy_ecs::world::DeferredWorld;
use bevy_math::Vec2;
use bevy_reflect::Reflect;
use parley::PlainEditorDriver;

/// Masks an [`EditableText`]'s displayed content: the editor's buffer holds
/// one mask glyph per character while the real text accumulates here.
///
/// The editor lays out and renders the mask string itself, so cursor
/// positioning, selection geometry, and click-to-position are exact by
/// construction -- one glyph per character preserves index parity in both
/// char and byte space (every char of an all-mask string is exactly
/// `glyph.len_utf8()` bytes).
///
/// Because the editor's buffer contains only mask glyphs,
/// [`EditableText::value`] returns the mask string. **Read the real content
/// via [`CharacterMask::value`].**
///
/// Adding the component conceals: the editor's current content is captured
/// as the real value and replaced with mask glyphs. Removing it reveals:
/// the real value is written back into the editor. A show/hide-password
/// toggle is therefore just inserting/removing the component.
///
/// Standard password-field behavior is enforced while masked:
/// [`TextEdit::Copy`] is a no-op and [`TextEdit::Cut`] degrades to a
/// selection-gated deletion (the real text never reaches the clipboard);
/// paste routes the clipboard text into the real value. IME input stays
/// enabled -- on mobile the soft keyboard delivers text through IME
/// commits, which route through the masked insert -- but composition
/// (preedit) is suppressed, since preedit renders in-buffer and would
/// display the raw text; IME entry is commit-per-key, matching platform
/// secure-entry behavior.
///
/// [`EditableText::max_characters`] and `EditableTextFilter` apply to the
/// **real** characters (a digit-only filter accepts `5` into a masked
/// field even though the mask glyph itself would fail the filter).
///
/// Masking is enforced by
/// [`EditableText::apply_pending_edits`] / [`apply_text_edits`]
/// (the documented entry points). Applying a [`TextEdit`] directly via
/// [`TextEdit::apply`] bypasses the mask.
///
/// Known limitation: one glyph per `char`, not per grapheme cluster --
/// combining sequences render as multiple glyphs.
///
/// [`apply_text_edits`]: crate::apply_text_edits
#[derive(Component, Debug, Clone, Reflect)]
#[reflect(Component)]
#[component(on_add = on_mask_added, on_remove = on_mask_removed)]
pub struct CharacterMask {
    /// The real (unmasked) text. Prefer [`CharacterMask::value`] for reads.
    pub value: String,
    /// The glyph rendered per character.
    ///
    /// Defaults to `*`: Bevy's embedded default font is a minimal ASCII
    /// subset that does not contain the typographic bullet, so `*` is the
    /// only default that renders everywhere. If the field's font covers
    /// U+2022, prefer `glyph: '\u{2022}'` (`•`) for the conventional look.
    pub glyph: char,
}

impl Default for CharacterMask {
    fn default() -> Self {
        Self {
            value: String::new(),
            glyph: '*',
        }
    }
}

impl CharacterMask {
    /// The real (unmasked) text.
    pub fn value(&self) -> &str {
        &self.value
    }

    /// The mask string for `n` characters.
    fn mask_string(&self, n: usize) -> String {
        self.glyph.to_string().repeat(n)
    }

    /// The mask string matching the current real value.
    fn expected_editor_text(&self) -> String {
        self.mask_string(self.value.chars().count())
    }

    /// Char index from a byte offset into the all-mask editor string.
    fn char_index(&self, byte_offset: usize) -> usize {
        byte_offset / self.glyph.len_utf8()
    }

    /// Replaces the chars in `range` (char indices) of the real value.
    fn splice_chars(&mut self, range: core::ops::Range<usize>, replacement: &str) {
        let byte_at = |char_idx: usize| {
            self.value
                .char_indices()
                .nth(char_idx)
                .map_or(self.value.len(), |(byte, _)| byte)
        };
        let (start, end) = (byte_at(range.start), byte_at(range.end));
        self.value.replace_range(start..end, replacement);
    }
}

/// Adding [`CharacterMask`] conceals the editor's current content.
///
/// If [`EditableText`] isn't on the entity yet (bundle insertion order is
/// unspecified), this is a no-op -- [`reconcile`] heals it on the next run
/// of [`apply_text_edits`](crate::apply_text_edits).
fn on_mask_added(mut world: DeferredWorld, context: HookContext) {
    let entity = context.entity;
    let Some(mask) = world.get::<CharacterMask>(entity) else {
        return;
    };
    let expected = mask.expected_editor_text();
    let Some(mut editable_text) = world.get_mut::<EditableText>(entity) else {
        return;
    };
    if editable_text.value() != expected.as_str() {
        // Adopt whatever the editor holds as the real value, then conceal.
        let current = editable_text.value().to_string();
        editable_text.editor.set_text("");
        let Some(mut mask) = world.get_mut::<CharacterMask>(entity) else {
            return;
        };
        if !current.is_empty() {
            mask.value = current;
        }
        let masked = mask.expected_editor_text();
        if let Some(mut editable_text) = world.get_mut::<EditableText>(entity) {
            editable_text.editor.set_text(&masked);
        }
    }
}

/// Removing [`CharacterMask`] reveals: the real value is written back into
/// the editor.
fn on_mask_removed(mut world: DeferredWorld, context: HookContext) {
    let entity = context.entity;
    let Some(mask) = world.get::<CharacterMask>(entity) else {
        return;
    };
    let value = mask.value.clone();
    if let Some(mut editable_text) = world.get_mut::<EditableText>(entity) {
        editable_text.editor.set_text(&value);
    }
}

/// Enforces the masking invariant: the editor must contain exactly the mask
/// string for the current real value. Any deviation means the editor was
/// set from outside the masked edit path (bundle-ordering races at spawn,
/// [`EditableText::clear`], direct `set_text`, tests) -- the editor's
/// content is adopted as the new real value and re-concealed.
///
/// Returns `true` if the editor was modified (the caller's generation
/// check then emits `TextEditChange` / re-layout as usual).
pub(crate) fn reconcile(mask: &mut CharacterMask, editable_text: &mut EditableText) -> bool {
    let expected = mask.expected_editor_text();
    if editable_text.value() == expected.as_str() {
        return false;
    }
    let current = editable_text.value().to_string();
    mask.value = current;
    let masked = mask.expected_editor_text();
    editable_text.editor.set_text(&masked);
    true
}

/// The current selection as a char range into the (all-mask) editor text.
/// Collapsed cursors yield an empty range at the caret.
fn selection_char_range(
    driver: &PlainEditorDriver<TextBrush>,
    mask: &CharacterMask,
) -> core::ops::Range<usize> {
    let range = driver.editor.raw_selection().text_range();
    mask.char_index(range.start)..mask.char_index(range.end)
}

/// Editor text length in chars (cheap: all chars are the mask glyph).
fn editor_char_len(driver: &PlainEditorDriver<TextBrush>, mask: &CharacterMask) -> usize {
    mask.char_index(driver.editor.raw_text().len())
}

/// Inserts `text` into a masked field: the real chars go into the mask's
/// value (replacing the selection), the editor receives the same number of
/// mask glyphs. `char_filter` and `max_characters` are checked against the
/// REAL characters -- the editor insertion deliberately bypasses the
/// filter, since the mask glyph itself would fail e.g. a digit filter.
pub(crate) fn masked_insert(
    text: &str,
    mask: &mut CharacterMask,
    driver: &mut PlainEditorDriver<TextBrush>,
    max_characters: Option<usize>,
    char_filter: &impl Fn(char) -> bool,
) {
    if !text.chars().all(char_filter) {
        bevy_log::debug!("Masked insert rejected by char filter.");
        return;
    }
    let selection = selection_char_range(driver, mask);
    if let Some(max) = max_characters {
        let len = editor_char_len(driver, mask);
        if max < len - selection.len() + text.chars().count() {
            return;
        }
    }
    mask.splice_chars(selection, text);
    let glyphs = mask.mask_string(text.chars().count());
    driver.insert_or_replace_selection(&glyphs);
}

/// Applies a deletion-style driver op and mirrors the removed char range on
/// the real value. Parley collapses the caret to the removal start, so the
/// removed range in pre-edit space is `caret_after..caret_after + removed`.
fn mirrored_delete<'e>(
    mask: &mut CharacterMask,
    driver: &mut PlainEditorDriver<'e, TextBrush>,
    op: impl FnOnce(&mut PlainEditorDriver<'e, TextBrush>),
) {
    let len_before = editor_char_len(driver, mask);
    op(driver);
    let len_after = editor_char_len(driver, mask);
    let removed = len_before.saturating_sub(len_after);
    if removed > 0 {
        let caret = selection_char_range(driver, mask).start;
        mask.splice_chars(caret..caret + removed, "");
    }
}

/// The masked counterpart of [`TextEdit::apply`]: content-mutating edits
/// are transformed to keep the editor all-glyphs and the real value in
/// [`CharacterMask`]; cursor, selection, and scroll edits pass through
/// untouched (geometry over the mask string is the point of the design).
#[expect(
    clippy::too_many_arguments,
    reason = "mirrors TextEdit::apply's parameter list plus the mask"
)]
pub(crate) fn apply_masked_edit(
    edit: TextEdit,
    mask: &mut CharacterMask,
    driver: &mut PlainEditorDriver<TextBrush>,
    viewport: &mut TextViewport,
    cursor_margin: Vec2,
    clipboard: &mut bevy_clipboard::Clipboard,
    max_characters: Option<usize>,
    char_filter: &impl Fn(char) -> bool,
) {
    match edit {
        // Never write real text (or useless mask glyphs) to the clipboard.
        TextEdit::Copy => {}
        // Cut keeps its delete but its clipboard write is deliberately
        // suppressed -- this must NOT be merged with the Delete arm even
        // when bodies look similar: Cut is selection-gated (no-op on a
        // collapsed cursor, matching unmasked behavior), Delete is not.
        TextEdit::Cut => {
            if driver.editor.selected_text().is_some() {
                mirrored_delete(mask, driver, PlainEditorDriver::delete);
                reveal_cursor(driver, viewport, cursor_margin);
            }
        }
        TextEdit::Insert(text) => {
            masked_insert(text.as_str(), mask, driver, max_characters, char_filter);
            reveal_cursor(driver, viewport, cursor_margin);
        }
        // Paste is intercepted at the `apply_pending_edits` level (it owns
        // the async clipboard barrier); reaching here means a direct
        // `TextEdit::apply`-style call path, which bypasses masking by
        // documented contract -- treat as a no-op rather than leak.
        TextEdit::Paste => {
            bevy_log::debug!("TextEdit::Paste ignored in masked apply path.");
        }
        TextEdit::Backspace => {
            mirrored_delete(mask, driver, PlainEditorDriver::backdelete);
            reveal_cursor(driver, viewport, cursor_margin);
        }
        TextEdit::BackspaceWord => {
            // Masked word-ops treat the whole value as ONE word, explicitly:
            // mask glyphs are punctuation under UAX #29, so parley's own
            // word segmentation may split them per-glyph (this is why the
            // driver's backdelete_word is NOT used here) -- and word ops
            // must never reflect the real text's word structure anyway.
            // Clears everything up to the selection end; caret lands at 0.
            let end = selection_char_range(driver, mask).end;
            if end > 0 {
                mask.splice_chars(0..end, "");
                let remaining = mask.expected_editor_text();
                driver.editor.set_text(&remaining);
                driver.move_to_text_start();
            }
            reveal_cursor(driver, viewport, cursor_margin);
        }
        TextEdit::Delete => {
            mirrored_delete(mask, driver, PlainEditorDriver::delete);
            reveal_cursor(driver, viewport, cursor_margin);
        }
        TextEdit::DeleteWord => {
            // Mirror of BackspaceWord: clears from the selection start to
            // the end of the value. The caret's char index after set_text +
            // move_to_text_end equals the remaining length, which is
            // exactly the deletion start -- so the caret stays put.
            let start = selection_char_range(driver, mask).start;
            let len = editor_char_len(driver, mask);
            if start < len {
                mask.splice_chars(start..len, "");
                let remaining = mask.expected_editor_text();
                driver.editor.set_text(&remaining);
                driver.move_to_text_end();
            }
            reveal_cursor(driver, viewport, cursor_margin);
        }
        // On mobile, the soft keyboard delivers text through IME commits --
        // this is the primary touch input path, not an edge case. Routed
        // through the masked insert so keyboard text can never bypass the
        // mask.
        TextEdit::ImeCommit { value } => {
            let clear = TextEdit::clear_ime_compose();
            clear.apply(
                driver,
                viewport,
                cursor_margin,
                clipboard,
                max_characters,
                char_filter,
            );
            masked_insert(value.as_str(), mask, driver, max_characters, char_filter);
            reveal_cursor(driver, viewport, cursor_margin);
        }
        // Preedit renders in-buffer -- a masked field must never display
        // the raw composing text. Suppressing composition degrades IME
        // entry to commit-per-key, matching secure-keyboard behavior.
        TextEdit::ImeSetCompose { .. } => {
            let clear = TextEdit::clear_ime_compose();
            clear.apply(
                driver,
                viewport,
                cursor_margin,
                clipboard,
                max_characters,
                char_filter,
            );
        }
        // Cursor movement, selection, point ops, scrolling: geometry over
        // the mask string is exact by construction -- pass through.
        other => other.apply(
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
    use super::*;
    use crate::{FontCx, LayoutCx};
    use bevy_app::App;

    fn masked_field(initial: &str) -> (App, Entity) {
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

        let mut editable_text = EditableText::new(initial);
        editable_text.editor.edit_styles().insert(
            parley::FontFamilyName::Named(alloc::borrow::Cow::Borrowed("Fira Mono")).into(),
        );
        let entity = app
            .world_mut()
            .spawn((editable_text, CharacterMask::default()))
            .id();
        app.update();
        (app, entity)
    }

    fn queue(app: &mut App, entity: Entity, edit: TextEdit) {
        app.world_mut()
            .get_mut::<EditableText>(entity)
            .unwrap()
            .queue_edit(edit);
        app.update();
    }

    fn real(app: &App, entity: Entity) -> String {
        app.world()
            .get::<CharacterMask>(entity)
            .unwrap()
            .value()
            .to_string()
    }

    fn shown(app: &mut App, entity: Entity) -> String {
        app.world_mut()
            .get_mut::<EditableText>(entity)
            .unwrap()
            .value()
            .to_string()
    }

    #[test]
    fn conceals_prefilled_text_on_add() {
        let (mut app, entity) = masked_field("secret");
        assert_eq!(real(&app, entity), "secret");
        assert_eq!(shown(&mut app, entity), "******");
    }

    #[test]
    fn typing_masks_and_accumulates() {
        let (mut app, entity) = masked_field("");
        queue(&mut app, entity, TextEdit::Insert("a".into()));
        queue(&mut app, entity, TextEdit::Insert("b".into()));
        assert_eq!(real(&app, entity), "ab");
        assert_eq!(shown(&mut app, entity), "**");
    }

    #[test]
    fn backspace_mirrors_on_real_value() {
        let (mut app, entity) = masked_field("abc");
        queue(&mut app, entity, TextEdit::TextEnd(false));
        queue(&mut app, entity, TextEdit::Backspace);
        assert_eq!(real(&app, entity), "ab");
        assert_eq!(shown(&mut app, entity), "**");
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
        assert_eq!(shown(&mut app, entity), "**");
    }

    #[test]
    fn backspace_word_clears_to_start() {
        // an all-glyph string is one word: conventional password behavior
        let (mut app, entity) = masked_field("abcdef");
        queue(&mut app, entity, TextEdit::TextEnd(false));
        queue(&mut app, entity, TextEdit::BackspaceWord);
        assert_eq!(real(&app, entity), "");
        assert_eq!(shown(&mut app, entity), "");
    }

    #[test]
    fn typing_over_selection_replaces_range() {
        let (mut app, entity) = masked_field("abcd");
        queue(&mut app, entity, TextEdit::SelectAll);
        queue(&mut app, entity, TextEdit::Insert("z".into()));
        assert_eq!(real(&app, entity), "z");
        assert_eq!(shown(&mut app, entity), "*");
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
        // regression guard for the filter bypass: a digit filter must
        // accept '5' into a masked field even though '•' fails it
        let (mut app, entity) = masked_field("");
        app.world_mut()
            .entity_mut(entity)
            .insert(crate::EditableTextFilter::new(|c| c.is_ascii_digit()));
        queue(&mut app, entity, TextEdit::Insert("5".into()));
        queue(&mut app, entity, TextEdit::Insert("x".into()));
        assert_eq!(real(&app, entity), "5");
        assert_eq!(shown(&mut app, entity), "*");
    }

    #[test]
    fn removing_mask_reveals() {
        let (mut app, entity) = masked_field("secret");
        app.world_mut().entity_mut(entity).remove::<CharacterMask>();
        assert_eq!(shown(&mut app, entity), "secret");
    }

    #[test]
    fn external_set_text_is_reconciled() {
        let (mut app, entity) = masked_field("old");
        app.world_mut()
            .get_mut::<EditableText>(entity)
            .unwrap()
            .editor
            .set_text("new!");
        app.update();
        assert_eq!(real(&app, entity), "new!");
        assert_eq!(shown(&mut app, entity), "****");
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
        assert_eq!(shown(&mut app, entity), "***");
    }
}

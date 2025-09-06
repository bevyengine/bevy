use bevy_ecs::{
    change_detection::{DetectChanges, DetectChangesMut},
    component::Component,
    system::{Query, ResMut},
};
pub use cosmic_text::Motion;
use cosmic_text::{Action, Buffer, Edit, Editor, Metrics, Selection};

use crate::{CosmicFontSystem, TextInputBuffer, DEFAULT_FONT_SIZE, DEFAULT_LINE_HEIGHT};

/// Add this component to hide the text input buffer contents
/// by replacing the characters with `mask_char`.
///
/// It is strongly recommended to only use a [`PasswordMask` with fixed-width fonts.
/// With variable width fonts mouse picking and horizontal scrolling
/// may not work correctly.
///
/// This is updated in [`update_password_masks`].
#[derive(Component)]
pub struct PasswordMask {
    /// If true the password will not be hidden
    pub show_password: bool,
    /// Char that will replace the masked input characters, by default `*`
    pub mask_char: char,
    /// Buffer mirroring the actual text input buffer but only containing `mask_char`s
    pub editor: Editor<'static>,
}

impl Default for PasswordMask {
    fn default() -> Self {
        Self {
            show_password: false,
            mask_char: '*',
            editor: Editor::new(Buffer::new_empty(Metrics::new(
                DEFAULT_FONT_SIZE,
                DEFAULT_LINE_HEIGHT,
            ))),
        }
    }
}

/// Update each [`PasswordMask`] to mirror its underlying [`TextInputBuffer`].
///
/// The recommended practice is to use fixed-width fonts for password inputs.
/// With variable sized fonts the glyph geometry of the password mask editor buffer may not match the
/// underlying editor buffer, possibly resulting in incorrect scrolling and mouse interactions.
pub fn update_password_masks(
    mut text_input_query: Query<(&mut TextInputBuffer, &mut PasswordMask)>,
    mut cosmic_font_system: ResMut<CosmicFontSystem>,
) {
    let font_system = &mut cosmic_font_system.0;
    for (mut buffer, mut mask) in text_input_query.iter_mut() {
        if buffer.editor.redraw() || mask.is_changed() {
            buffer.editor.shape_as_needed(font_system, false);
            let mask_text: String = buffer.get_text().chars().map(|_| mask.mask_char).collect();
            let mask_editor = &mut mask.bypass_change_detection().editor;
            *mask_editor = buffer.editor.clone();
            let mut editor = mask_editor.borrow_with(font_system);
            let selection = editor.selection();
            let cursor = editor.cursor();
            editor.action(Action::Motion(Motion::BufferStart));
            let start = editor.cursor();
            editor.set_selection(Selection::Normal(start));
            editor.action(Action::Motion(Motion::BufferEnd));
            editor.action(Action::Delete);
            editor.insert_string(&mask_text, None);
            editor.set_selection(selection);
            editor.set_cursor(cursor);
            editor.set_redraw(true);
        }
    }
}

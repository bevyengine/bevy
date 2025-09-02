use bevy_ecs::resource::Resource;

/// Basic clipboard implementation that only works within the bevy app.
///
/// This is written to in the [`crate::apply_text_edits`] system when
/// [`crate::TextEdit::Copy`], [`crate::TextEdit::Cut`] or [`crate::TextEdit::Paste`] edits are applied.
#[derive(Resource, Default)]
pub struct Clipboard(pub String);

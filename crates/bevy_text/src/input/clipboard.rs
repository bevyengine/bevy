use bevy_ecs::resource::Resource;

/// Basic clipboard implementation that only works within the bevy app.
///
/// This is written to in the [`apply_text_edits`] system when
/// [`TextEdit::Copy`], [`TextEdit::Cut`] or [`TextEdit::Paste`] edits are applied.
#[derive(Resource, Default)]
pub struct Clipboard(pub String);

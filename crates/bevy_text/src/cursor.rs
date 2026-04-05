use bevy_color::{
    palettes::css::{GREEN, RED},
    Color,
};
use bevy_ecs::component::Component;

/// Controls text cursor appearance.
///
/// When this component on the same entity as an [`EditableText`](`crate::EditableText`) ,
/// and the [`UiRenderPlugin`](https://docs.rs/bevy/latest/bevy/ui_render/struct.UiRenderPlugin.html)
/// is active, a simple rectangle will be drawn for the cursor.
/// This is an optional component, to allow for stylistic cursors.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct TextCursorStyle {
    /// Color of the cursor
    pub color: Color,
    /// Background color of selected text
    pub selection_color: Color,
    /// If some, overrides the color of selected text
    pub selected_text_color: Option<Color>,
}

impl Default for TextCursorStyle {
    fn default() -> Self {
        Self {
            color: RED.into(),
            selection_color: Color::from(GREEN),
            selected_text_color: None,
        }
    }
}

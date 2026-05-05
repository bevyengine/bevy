use bevy_color::{
    palettes::tailwind::{SKY_300, SKY_400, SLATE_700},
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
    /// Background color of unfocused selected text
    ///
    /// In many applications, this is completely transparent.
    /// This reduces visual clutter of de-emphasized text inputs.
    pub unfocused_selection_color: Color,
    /// If some, overrides the color of selected text
    pub selected_text_color: Option<Color>,
}

impl Default for TextCursorStyle {
    fn default() -> Self {
        Self {
            color: Color::from(SLATE_700),
            selection_color: Color::from(SKY_300),
            unfocused_selection_color: Color::from(SKY_400),
            selected_text_color: None,
        }
    }
}

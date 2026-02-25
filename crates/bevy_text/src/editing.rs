use bevy_color::Color;
use bevy_ecs::component::Component;

/// Text Cursor style
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct TextCursorStyle {
    /// Color of the cursor
    pub color: Color,
    /// Background color of selected text
    pub selection_color: Color,
    /// Color of text under selection
    pub selected_text_color: Option<Color>,
}

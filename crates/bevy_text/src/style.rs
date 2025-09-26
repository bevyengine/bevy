use crate::*;
use bevy_color::Color;
use bevy_ecs::component::Component;
use bevy_ecs::prelude::*;

/// Fallback text style used if a text entity and all its ancestors lack text styling components.
#[derive(Resource)]
pub struct DefaultTextStyle {
    /// The font used by a text entity when neither it nor any ancestor has a [`TextFont`] component.
    pub font: TextFont,
    /// The color used by a text entity when neither it nor any ancestor has a [`TextColor`] component.
    pub color: Color,
}

impl Default for DefaultTextStyle {
    fn default() -> Self {
        Self {
            font: Default::default(),
            color: Color::WHITE,
        }
    }
}

/// The resolved text style for a text entity.
#[derive(Component, PartialEq, Default)]
pub struct ComputedTextStyle {
    /// The resolved font, taken from the nearest ancestor (including self) with a [`TextFont`],
    /// or from [`DefaultTextStyle`] if none is found.
    pub font: TextFont,
    /// The resolved text color, taken from the nearest ancestor (including self) with a [`TextColor`],
    /// or from [`DefaultTextStyle`] if none is found.
    pub color: Color,
    /// Scale factor of the text entity's render target.
    pub scale_factor: f32,
}

impl ComputedTextStyle {
    /// The resolved font, taken from the nearest ancestor (including self) with a [`TextFont`],
    /// or from [`DefaultTextStyle`] if none is found.
    pub const fn font(&self) -> &TextFont {
        &self.font
    }

    /// The resolved text color, taken from the nearest ancestor (including self) with a [`TextColor`],
    /// or from [`DefaultTextStyle`] if none is found.
    pub const fn color(&self) -> Color {
        self.color
    }
}

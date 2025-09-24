use crate::*;
use bevy_color::Color;
use bevy_ecs::component::Component;
use bevy_ecs::prelude::*;

/// Fallback text style used if a text entity and all its ancestors lack text styling components.
#[derive(Resource)]
pub struct DefaultTextStyle {
    /// The font used by a text entity when neither it nor any ancestor has a [`TextFont`] component.
    font: TextFont,
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
///
/// Updated by [`update_text_styles`]
#[derive(Component, PartialEq, Default)]
pub struct ComputedTextStyle {
    /// The resolved font, taken from the nearest ancestor (including self) with a [`TextFont`],
    /// or from [`DefaultTextStyle`] if none is found.
    pub(crate) font: TextFont,
    /// The resolved text color, taken from the nearest ancestor (including self) with a [`TextColor`],
    /// or from [`DefaultTextStyle`] if none is found.
    pub(crate) color: Color,
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

/// Update the `ComputedTextStyle` for each text node from the
/// `TextFont`s and `TextColor`s of its nearest ancestors, or from [`DefaultTextStyle`] if none are found.
pub fn update_text_styles(
    default_text_style: Res<DefaultTextStyle>,
    mut computed_text_query: Query<(Entity, &mut ComputedTextStyle)>,
    parent_query: Query<&ChildOf>,
    font_query: Query<(Option<&TextFont>, Option<&TextColor>)>,
) {
    for (start, mut style) in computed_text_query.iter_mut() {
        let (mut font, mut color) = font_query.get(start).unwrap();
        let mut ancestors = parent_query.iter_ancestors(start);

        while (font.is_none() || color.is_none())
            && let Some(ancestor) = ancestors.next()
        {
            let (next_font, next_color) = font_query.get(ancestor).unwrap();
            font = font.or(next_font);
            color = color.or(next_color);
        }

        let new_style = ComputedTextStyle {
            font: font.unwrap_or(&default_text_style.font).clone(),
            color: color.map(|t| t.0).unwrap_or(default_text_style.color),
        };

        if new_style.font != style.font {
            *style = new_style;
        } else {
            // bypass change detection, we don't need to do any updates if only the text color has changed
            style.bypass_change_detection().color = new_style.color;
        }
    }
}

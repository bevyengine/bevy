use crate::*;
use bevy_color::Color;
use bevy_ecs::component::Component;
use bevy_ecs::prelude::*;

/// Default text style
#[derive(Resource)]
pub struct DefaultTextStyle {
    /// default font
    pub font: TextFont,
    /// default color
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

/// Computed text style
#[derive(Component, PartialEq, Default)]
pub struct ComputedTextStyle {
    /// From nearest ancestor with a `TextFont`
    pub(crate) font: TextFont,
    /// From nearest ancestor with a `TextColor`
    pub(crate) color: Color,
}

impl ComputedTextStyle {
    /// Computed text font
    pub const fn font(&self) -> &TextFont {
        &self.font
    }

    /// Computed text color
    pub const fn color(&self) -> Color {
        self.color
    }
}

/// update text styles
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
            style.color = new_style.color;
        }
    }
}

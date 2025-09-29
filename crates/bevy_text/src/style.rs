use crate::*;
use bevy_asset::Handle;
use bevy_color::Color;
use bevy_derive::Deref;
use bevy_derive::DerefMut;
use bevy_ecs::component::Component;
use bevy_ecs::entity::EntityHashSet;
use bevy_ecs::prelude::*;
use bevy_reflect::Reflect;

/// Text style
#[derive(Clone, PartialEq)]
pub struct TextStyle {
    /// The font used by a text entity when neither it nor any ancestor has a [`TextFont`] component.
    pub font: Handle<Font>,
    /// Default value
    pub font_size: f32,
    /// The color used by a text entity when neither it nor any ancestor has a [`TextColor`] component.
    pub color: Color,
    /// Default value
    pub font_smoothing: FontSmoothing,
    /// Default value
    pub line_height: LineHeight,
}

impl TextStyle {
    /// Returns the text style as a bundle of components
    pub fn bundle(&self) -> impl Bundle {
        (
            TextFont(self.font.clone()),
            FontSize(self.font_size),
            TextColor(self.color),
            self.font_smoothing,
            self.line_height,
        )
    }
}

impl Default for TextStyle {
    fn default() -> Self {
        Self {
            font: Default::default(),
            color: Color::WHITE,
            font_smoothing: FontSmoothing::default(),
            line_height: LineHeight::default(),
            font_size: FontSize::default().0,
        }
    }
}

/// Fallback text style used if a text entity and all its ancestors lack text styling components.
#[derive(Resource, Default, Clone, Deref, DerefMut)]
pub struct DefaultTextStyle(pub TextStyle);

/// Internal struct for managing propagation
#[derive(Component, Clone, PartialEq, Reflect)]
#[reflect(Component, Clone, PartialEq)]
pub struct InheritedTextStyle {
    /// The resolved font, taken from the nearest ancestor (including self) with a [`TextFont`],
    /// or from [`DefaultTextStyle`] if none is found.
    pub(crate) font: Option<Handle<Font>>,
    /// The vertical height of rasterized glyphs in the font atlas in pixels.
    pub(crate) font_size: Option<f32>,
    /// The antialiasing method to use when rendering text.
    pub(crate) font_smoothing: Option<FontSmoothing>,
    /// The vertical height of a line of text, from the top of one line to the top of the
    /// next.
    pub(crate) line_height: Option<LineHeight>,
    /// The resolved text color, taken from the nearest ancestor (including self) with a [`TextColor`],
    /// or from [`DefaultTextStyle`] if none is found.
    pub(crate) color: Option<Color>,
}

/// The resolved text style for a text entity.
///
/// Updated by [`update_text_styles`]
#[derive(Component, PartialEq, Default)]
pub struct ComputedTextStyle {
    /// The resolved font, taken from the nearest ancestor (including self) with a [`TextFont`],
    /// or from [`DefaultTextStyle`] if none is found.
    pub(crate) font: Handle<Font>,
    /// The vertical height of rasterized glyphs in the font atlas in pixels.
    pub(crate) font_size: f32,
    /// The antialiasing method to use when rendering text.
    pub(crate) font_smoothing: FontSmoothing,
    /// The vertical height of a line of text, from the top of one line to the top of the
    /// next.
    pub(crate) line_height: LineHeight,
    /// The resolved text color, taken from the nearest ancestor (including self) with a [`TextColor`],
    /// or from [`DefaultTextStyle`] if none is found.
    pub(crate) color: Color,
}

impl ComputedTextStyle {
    /// The resolved font, taken from the nearest ancestor (including self) with a [`TextFont`],
    /// or from [`DefaultTextStyle`] if none is found.
    pub const fn font(&self) -> &Handle<Font> {
        &self.font
    }

    /// The resolved text color, taken from the nearest ancestor (including self) with a [`TextColor`],
    /// or from [`DefaultTextStyle`] if none is found.
    pub const fn color(&self) -> Color {
        self.color
    }

    /// The vertical height of a line of text, from the top of one line to the top of the
    /// next.
    pub const fn line_height(&self) -> LineHeight {
        self.line_height
    }

    /// The vertical height of rasterized glyphs in the font atlas in pixels.
    pub const fn font_size(&self) -> f32 {
        self.font_size
    }

    /// The antialiasing method to use when rendering text.
    pub const fn font_smoothing(&self) -> FontSmoothing {
        self.font_smoothing
    }
}

/// Update the `ComputedTextStyle` for each text node from the
/// `TextFont`s and `TextColor`s of its nearest ancestors, or from [`DefaultTextStyle`] if none are found.
pub fn update_text_styles(
    default_text_style: Res<DefaultTextStyle>,
    mut computed_text_query: Query<(Entity, &mut ComputedTextStyle)>,
    parent_query: Query<&ChildOf>,
    font_query: Query<(
        Option<&TextFont>,
        Option<&TextColor>,
        Option<&FontSize>,
        Option<&LineHeight>,
        Option<&FontSmoothing>,
    )>,
) {
    for (start, mut style) in computed_text_query.iter_mut() {
        let (mut font, mut color, mut size, mut line_height, mut smoothing) =
            font_query.get(start).unwrap();
        let mut ancestors = parent_query.iter_ancestors(start);

        while (font.is_none()
            || color.is_none()
            || size.is_none()
            || line_height.is_none()
            || smoothing.is_none())
            && let Some(ancestor) = ancestors.next()
        {
            let (next_font, next_color, next_size, next_line_height, next_smoothing) =
                font_query.get(ancestor).unwrap();
            font = font.or(next_font);
            color = color.or(next_color);
            size = size.or(next_size);
            line_height = line_height.or(next_line_height);
            smoothing = smoothing.or(next_smoothing);
        }

        let new_style = ComputedTextStyle {
            font: font
                .map_or(&default_text_style.font, |font| &font.0)
                .clone(),
            color: color.map(|t| t.0).unwrap_or(default_text_style.color),
            font_size: size.map_or(default_text_style.font_size, |size| size.0),
            font_smoothing: smoothing
                .copied()
                .unwrap_or(default_text_style.font_smoothing),
            line_height: line_height
                .copied()
                .unwrap_or(default_text_style.line_height),
        };

        if new_style.font != style.font {
            *style = new_style;
        } else {
            // bypass change detection, we don't need to do any updates if only the text color has changed
            style.bypass_change_detection().color = new_style.color;
        }
    }
}

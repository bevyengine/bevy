use bevy_color::Alpha;
use bevy_ecs::{component::Component, reflect::ReflectComponent};
use bevy_reflect::{prelude::ReflectDefault, Reflect};
use bevy_scene2::{bsn, Scene};
use bevy_ui::{BackgroundColor, BorderRadius, Node, PositionType, Val};

use crate::{alpha_pattern::AlphaPattern, constants::size, palette};

/// Marker identifying a color swatch.
#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Clone, Default)]
pub struct ColorSwatch;

/// Marker identifying the color swatch foreground, the piece that actually displays the color
/// in front of the alpha pattern. This exists so that users can reach in and change the color
/// dynamically.
#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Clone, Default)]
pub struct ColorSwatchFg;

/// Template function to spawn a color swatch.
///
/// # Arguments
/// * `overrides` - a bundle of components that are merged in with the normal swatch components.
pub fn color_swatch() -> impl Scene {
    bsn! {
        Node {
            height: size::ROW_HEIGHT,
            min_width: size::ROW_HEIGHT,
        }
        ColorSwatch
        AlphaPattern
        BorderRadius::all(Val::Px(5.0))
        [
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.),
                top: Val::Px(0.),
                bottom: Val::Px(0.),
                right: Val::Px(0.),
            }
            ColorSwatchFg
            BackgroundColor({palette::ACCENT.with_alpha(0.5)})
            BorderRadius::all(Val::Px(5.0))
        ]
    }
}

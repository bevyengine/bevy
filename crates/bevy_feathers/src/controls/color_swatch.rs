use bevy_asset::Handle;
use bevy_color::Alpha;
use bevy_ecs::{
    bundle::Bundle, children, component::Component, reflect::ReflectComponent, spawn::SpawnRelated,
};
use bevy_reflect::{prelude::ReflectDefault, Reflect};
use bevy_ui::{BackgroundColor, BorderRadius, Node, PositionType, Val};
use bevy_ui_render::ui_material::MaterialNode;

use crate::{
    alpha_pattern::{AlphaPattern, AlphaPatternMaterial},
    constants::size,
    palette,
};

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
pub fn color_swatch<B: Bundle>(overrides: B) -> impl Bundle {
    (
        Node {
            height: size::ROW_HEIGHT,
            min_width: size::ROW_HEIGHT,
            ..Default::default()
        },
        ColorSwatch,
        AlphaPattern,
        MaterialNode::<AlphaPatternMaterial>(Handle::default()),
        BorderRadius::all(Val::Px(5.0)),
        overrides,
        children![(
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.),
                top: Val::Px(0.),
                bottom: Val::Px(0.),
                right: Val::Px(0.),
                ..Default::default()
            },
            ColorSwatchFg,
            BackgroundColor(palette::ACCENT.with_alpha(0.5)),
            BorderRadius::all(Val::Px(5.0))
        ),],
    )
}

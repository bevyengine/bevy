use bevy_asset::Handle;
use bevy_color::Alpha;
use bevy_ecs::{bundle::Bundle, children, spawn::SpawnRelated};
use bevy_ui::{BackgroundColor, BorderRadius, Node, PositionType, Val};
use bevy_ui_render::ui_material::MaterialNode;

use crate::{
    alpha_pattern::{AlphaPattern, AlphaPatternMaterial},
    constants::size,
    palette,
};

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
            BackgroundColor(palette::ACCENT.with_alpha(0.5)),
            BorderRadius::all(Val::Px(5.0))
        ),],
    )
}

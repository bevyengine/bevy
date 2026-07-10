//! The standard `bevy_feathers` dark theme.
use crate::theme::{
    build_theme, default_axis_colors, default_token_slots, EditablePalette, OklchaArray, ThemeProps,
};
use bevy_color::Oklcha;

/// Default feathers dark palette
pub fn create_dark_theme() -> ThemeProps {
    build_theme(&default_dark_palette().resolve(), default_token_slots())
}

/// Default feathers dark palette editable inputs
pub fn default_dark_palette() -> EditablePalette {
    EditablePalette {
        neutrals: OklchaArray {
            hue: 280.0,
            chroma: 0.008,
            l: [0.2414, 0.287, 0.3373, 0.42, 0.47, 0.52, 0.57],
        },
        accent: OklchaArray {
            hue: 255.4,
            chroma: 0.1594,
            l: [0.542, 0.592, 0.642, 0.742],
        },
        text: OklchaArray {
            hue: 286.37,
            chroma: 0.0014,
            l: [1.0, 0.7607],
        },
        contrast: Oklcha::new(1.0, 0.0, 0.0, 1.0),
        axes: default_axis_colors(),
        dim_text_alpha_modifier: 0.2,
    }
}

//! The standard `bevy_feathers` light theme. NOT APPROVED YET
use crate::theme::{
    build_theme, default_axis_colors, default_token_slots, EditablePalette, OklchaArray, ThemeProps,
};
use bevy_color::Oklcha;

/// Default feathers light palette
pub fn create_light_theme() -> ThemeProps {
    build_theme(&default_light_palette().resolve(), default_token_slots())
}

/// Default feathers light palette editable inputs
pub fn default_light_palette() -> EditablePalette {
    EditablePalette {
        neutrals: OklchaArray {
            hue: 266.0,
            chroma: 0.02,
            l: [0.99, 0.95, 0.90, 0.83, 0.76, 0.70, 0.66],
        },
        accent: OklchaArray {
            hue: 255.4,
            chroma: 0.1594,
            l: [0.61, 0.55, 0.50, 0.45],
        },
        text: OklchaArray {
            hue: 266.0,
            chroma: 0.0014,
            l: [0.07, 0.14],
        },
        contrast: Oklcha::new(1.0, 0.0, 0.0, 1.0),
        axes: default_axis_colors(),
        dim_text_alpha_modifier: 0.4,
    }
}

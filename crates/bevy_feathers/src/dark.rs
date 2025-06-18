use crate::{colors, theme::tokens};
use bevy_color::{Alpha, Luminance};
use bevy_platform::collections::HashMap;

use crate::theme::ThemeProps;

/// Create a [`ThemeProps`] object and populate it with the colors for the default dark theme.
pub fn create_dark_theme() -> ThemeProps {
    ThemeProps {
        color: HashMap::from([
            (tokens::WINDOW_BG.into(), colors::GRAY_0),
            (tokens::BUTTON_BG.into(), colors::GRAY_3),
            (tokens::BUTTON_BG_HOVER.into(), colors::GRAY_3.lighter(0.05)),
            (
                tokens::BUTTON_BG_PRESSED.into(),
                colors::GRAY_3.lighter(0.1),
            ),
            (tokens::BUTTON_BG_DISABLED.into(), colors::GRAY_2),
            (tokens::BUTTON_PRIMARY_BG.into(), colors::ACCENT),
            (
                tokens::BUTTON_PRIMARY_BG_HOVER.into(),
                colors::ACCENT.lighter(0.05),
            ),
            (
                tokens::BUTTON_PRIMARY_BG_PRESSED.into(),
                colors::ACCENT.lighter(0.1),
            ),
            (tokens::BUTTON_PRIMARY_BG_DISABLED.into(), colors::GRAY_2),
            (tokens::BUTTON_TEXT.into(), colors::WHITE),
            (
                tokens::BUTTON_TEXT_DISABLED.into(),
                colors::WHITE.with_alpha(0.5),
            ),
            (tokens::SLIDER_BG.into(), colors::GRAY_1),
            (tokens::SLIDER_BAR.into(), colors::ACCENT),
            (tokens::SLIDER_BAR_DISABLED.into(), colors::GRAY_2),
            (tokens::SLIDER_TEXT.into(), colors::WHITE),
            (
                tokens::SLIDER_TEXT_DISABLED.into(),
                colors::WHITE.with_alpha(0.5),
            ),
        ]),
    }
}

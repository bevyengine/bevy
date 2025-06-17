use crate::{
    colors,
    theme::tokens::{BUTTON_BG, BUTTON_TXT},
};
use bevy_platform::collections::HashMap;

use crate::theme::{tokens::WINDOW_BG, ThemeProps};

/// Create a [`ThemeProps`] object and populate it with the colors for the default dark theme.
pub fn create_dark_theme() -> ThemeProps {
    ThemeProps {
        color: HashMap::from([
            (WINDOW_BG.into(), colors::GRAY_0),
            (BUTTON_BG.into(), colors::GRAY_3),
            (BUTTON_TXT.into(), colors::WHITE),
        ]),
    }
}

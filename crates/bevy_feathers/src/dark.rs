use crate::colors;
use bevy_platform::collections::HashMap;

use crate::theme::{tokens::WINDOW_BG, ThemeProps};

/// Create a [`ThemeProps`] object and populate it with the colors for the default dark theme.
pub fn create_dark_theme() -> ThemeProps {
    let mut props = ThemeProps {
        color: HashMap::with_capacity(50),
    };
    props.color.insert(WINDOW_BG.into(), colors::GRAY_0);
    props
}

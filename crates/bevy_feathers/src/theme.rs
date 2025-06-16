use bevy_color::{palettes, Color};
use bevy_ecs::resource::Resource;
use bevy_log::warn_once;
use bevy_platform::collections::HashMap;

/// A collection of properties that make up a theme.
#[derive(Default, Clone)]
pub struct ThemeProps {
    /// Map of design tokens to colors.
    pub color: HashMap<String, Color>,
    // Other style property types to be added later.
}

/// The currently selected user interface theme. Overwriting this resource changes the theme.
#[derive(Resource, Default)]
pub struct UiTheme(pub ThemeProps);

impl UiTheme {
    /// Lookup a color by design token. If the theme does not have an entry for that token,
    /// logs a warning and returns an error color.
    pub fn color<'a>(&self, token: &'a str) -> Color {
        let color = self.0.color.get(token);
        match color {
            Some(c) => *c,
            None => {
                warn_once!("Theme color {} not found.", token);
                // Return a bright obnoxious color to make the error obvious.
                palettes::basic::FUCHSIA.into()
            }
        }
    }

    /// Associate a design token with a given color.
    pub fn set_color(&mut self, token: impl Into<String>, color: Color) {
        self.0.color.insert(token.into(), color);
    }
}

/// UX design tokens
pub mod tokens {
    /// Window background
    pub const WINDOW_BG: &str = "window.bg";

    /// Focus ring
    pub const FOCUS_RING: &str = "focus";

    /// Regular text
    pub const TEXT_MAIN: &str = "text.main";
    /// Dim text
    pub const TEXT_DIM: &str = "text.dim";

    // Normal buttons

    /// Regular button background
    pub const BUTTON_BG: &str = "button.bg";
    /// Regular button background (hovered)
    pub const BUTTON_BG_HOVER: &str = "button.bg.hover";
    /// Regular button background (disabled)
    pub const BUTTON_BG_DISABLED: &str = "button.bg.disabled";
    /// Regular button background (pressed)
    pub const BUTTON_BG_PRESSED: &str = "button.bg.pressed";
    /// Regular button text
    pub const BUTTON_TXT: &str = "button.txt";
    /// Regular button text (disabled)
    pub const BUTTON_TXT_DISABLED: &str = "button.txt.disabled";

    // Primary ("default") buttons

    /// Primary button background
    pub const BUTTON_PRIMARY_BG: &str = "button.primary.bg";
    /// Primary button background (hovered)
    pub const BUTTON_PRIMARY_BG_HOVER: &str = "button.primary.bg.hover";
    /// Primary button background (disabled)
    pub const BUTTON_PRIMARY_BG_DISABLED: &str = "button.primary.bg.disabled";
    /// Primary button background (pressed)
    pub const BUTTON_PRIMARY_BG_PRESSED: &str = "button.primary.bg.pressed";
    /// Primary button text
    pub const BUTTON_PRIMARY_TXT: &str = "button.primary.txt";
    /// Primary button text (disabled)
    pub const BUTTON_PRIMARY_TXT_DISABLED: &str = "button.primary.txt.disabled";

    // Slider

    /// Background for slider
    pub const SLIDER_BG: &str = "slider.bg";
    /// Background for slider moving bar
    pub const SLIDER_BAR: &str = "slider.bar";
    /// Background for slider moving bar (disabled)
    pub const SLIDER_BAR_DISABLED: &str = "slider.bar.disabled";
    /// Background for slider text
    pub const SLIDER_TEXT: &str = "slider.text";
    /// Background for slider text (disabled)
    pub const SLIDER_TEXT_DISABLED: &str = "slider.text.disabled";

    // Checkbox

    /// Checkbox border around the checkmark
    pub const CHECKBOX_BORDER: &str = "checkbox.border";
    /// Checkbox border around the checkmark (hovered)
    pub const CHECKBOX_BORDER_HOVER: &str = "checkbox.border.hover";
    /// Checkbox border around the checkmark (disabled)
    pub const CHECKBOX_BORDER_DISABLED: &str = "checkbox.border.disabled";
    /// Checkbox check mark
    pub const CHECKBOX_MARK: &str = "checkbox.mark";
    /// Checkbox check mark (disabled)
    pub const CHECKBOX_MARK_DISABLED: &str = "checkbox.mark.disabled";
    /// Checkbox label text
    pub const CHECKBOX_TEXT: &str = "checkbox.text";
    /// Checkbox label text (disabled)
    pub const CHECKBOX_TEXT_DISABLED: &str = "checkbox.text.disabled";
}

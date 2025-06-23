//! Design tokens used by Feathers themes.
//!
//! The term "design token" is commonly used in UX design to mean the smallest unit of a theme,
//! similar in concept to a CSS variable. Each token represents an assignment of a color or
//! value to a specific visual aspect of a widget, such as background or border.

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
pub const BUTTON_TEXT: &str = "button.txt";
/// Regular button text (disabled)
pub const BUTTON_TEXT_DISABLED: &str = "button.txt.disabled";

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
pub const BUTTON_PRIMARY_TEXT: &str = "button.primary.txt";
/// Primary button text (disabled)
pub const BUTTON_PRIMARY_TEXT_DISABLED: &str = "button.primary.txt.disabled";

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

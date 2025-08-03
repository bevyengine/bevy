//! Design tokens used by Feathers themes.
//!
//! The term "design token" is commonly used in UX design to mean the smallest unit of a theme,
//! similar in concept to a CSS variable. Each token represents an assignment of a color or
//! value to a specific visual aspect of a widget, such as background or border.

/// Window background
pub const WINDOW_BG: &str = "feathers.window.bg";

/// Focus ring
pub const FOCUS_RING: &str = "feathers.focus";

/// Regular text
pub const TEXT_MAIN: &str = "feathers.text.main";
/// Dim text
pub const TEXT_DIM: &str = "feathers.text.dim";

// Normal buttons

/// Regular button background
pub const BUTTON_BG: &str = "feathers.button.bg";
/// Regular button background (hovered)
pub const BUTTON_BG_HOVER: &str = "feathers.button.bg.hover";
/// Regular button background (disabled)
pub const BUTTON_BG_DISABLED: &str = "feathers.button.bg.disabled";
/// Regular button background (pressed)
pub const BUTTON_BG_PRESSED: &str = "feathers.button.bg.pressed";
/// Regular button text
pub const BUTTON_TEXT: &str = "feathers.button.txt";
/// Regular button text (disabled)
pub const BUTTON_TEXT_DISABLED: &str = "feathers.button.txt.disabled";

// Primary ("default") buttons

/// Primary button background
pub const BUTTON_PRIMARY_BG: &str = "feathers.button.primary.bg";
/// Primary button background (hovered)
pub const BUTTON_PRIMARY_BG_HOVER: &str = "feathers.button.primary.bg.hover";
/// Primary button background (disabled)
pub const BUTTON_PRIMARY_BG_DISABLED: &str = "feathers.button.primary.bg.disabled";
/// Primary button background (pressed)
pub const BUTTON_PRIMARY_BG_PRESSED: &str = "feathers.button.primary.bg.pressed";
/// Primary button text
pub const BUTTON_PRIMARY_TEXT: &str = "feathers.button.primary.txt";
/// Primary button text (disabled)
pub const BUTTON_PRIMARY_TEXT_DISABLED: &str = "feathers.button.primary.txt.disabled";

// Selected ("toggled") buttons

/// Selected button background
pub const BUTTON_SELECTED_BG: &str = "feathers.button.selected.bg";
/// Selected button background (hovered)
pub const BUTTON_SELECTED_BG_HOVER: &str = "feathers.button.selected.bg.hover";
/// Selected button background (disabled)
pub const BUTTON_SELECTED_BG_DISABLED: &str = "feathers.button.selected.bg.disabled";
/// Selected button background (pressed)
pub const BUTTON_SELECTED_BG_PRESSED: &str = "feathers.button.selected.bg.pressed";

// Plain buttons (transparent background)

/// Plain button background
pub const BUTTON_PLAIN_BG: &str = "feathers.button.plain.bg";
/// Plain button background (hovered)
pub const BUTTON_PLAIN_BG_HOVER: &str = "feathers.button.plain.bg.hover";
/// Plain button background (disabled)
pub const BUTTON_PLAIN_BG_DISABLED: &str = "feathers.button.plain.bg.disabled";
/// Plain button background (pressed)
pub const BUTTON_PLAIN_BG_PRESSED: &str = "feathers.button.plain.bg.pressed";

// Slider

/// Background for slider
pub const SLIDER_BG: &str = "feathers.slider.bg";
/// Background for slider moving bar
pub const SLIDER_BAR: &str = "feathers.slider.bar";
/// Background for slider moving bar (disabled)
pub const SLIDER_BAR_DISABLED: &str = "feathers.slider.bar.disabled";
/// Background for slider text
pub const SLIDER_TEXT: &str = "feathers.slider.text";
/// Background for slider text (disabled)
pub const SLIDER_TEXT_DISABLED: &str = "feathers.slider.text.disabled";

// Checkbox

/// Checkbox background around the checkmark
pub const CHECKBOX_BG: &str = "feathers.checkbox.bg";
/// Checkbox border around the checkmark (disabled)
pub const CHECKBOX_BG_DISABLED: &str = "feathers.checkbox.bg.disabled";
/// Checkbox background around the checkmark
pub const CHECKBOX_BG_CHECKED: &str = "feathers.checkbox.bg.checked";
/// Checkbox border around the checkmark (disabled)
pub const CHECKBOX_BG_CHECKED_DISABLED: &str = "feathers.checkbox.bg.checked.disabled";
/// Checkbox border around the checkmark
pub const CHECKBOX_BORDER: &str = "feathers.checkbox.border";
/// Checkbox border around the checkmark (hovered)
pub const CHECKBOX_BORDER_HOVER: &str = "feathers.checkbox.border.hover";
/// Checkbox border around the checkmark (disabled)
pub const CHECKBOX_BORDER_DISABLED: &str = "feathers.checkbox.border.disabled";
/// Checkbox check mark
pub const CHECKBOX_MARK: &str = "feathers.checkbox.mark";
/// Checkbox check mark (disabled)
pub const CHECKBOX_MARK_DISABLED: &str = "feathers.checkbox.mark.disabled";
/// Checkbox label text
pub const CHECKBOX_TEXT: &str = "feathers.checkbox.text";
/// Checkbox label text (disabled)
pub const CHECKBOX_TEXT_DISABLED: &str = "feathers.checkbox.text.disabled";

// Radio button

/// Radio border around the checkmark
pub const RADIO_BORDER: &str = "feathers.radio.border";
/// Radio border around the checkmark (hovered)
pub const RADIO_BORDER_HOVER: &str = "feathers.radio.border.hover";
/// Radio border around the checkmark (disabled)
pub const RADIO_BORDER_DISABLED: &str = "feathers.radio.border.disabled";
/// Radio check mark
pub const RADIO_MARK: &str = "feathers.radio.mark";
/// Radio check mark (disabled)
pub const RADIO_MARK_DISABLED: &str = "feathers.radio.mark.disabled";
/// Radio label text
pub const RADIO_TEXT: &str = "feathers.radio.text";
/// Radio label text (disabled)
pub const RADIO_TEXT_DISABLED: &str = "feathers.radio.text.disabled";

// Toggle Switch

/// Switch background around the checkmark
pub const SWITCH_BG: &str = "feathers.switch.bg";
/// Switch border around the checkmark (disabled)
pub const SWITCH_BG_DISABLED: &str = "feathers.switch.bg.disabled";
/// Switch background around the checkmark
pub const SWITCH_BG_CHECKED: &str = "feathers.switch.bg.checked";
/// Switch border around the checkmark (disabled)
pub const SWITCH_BG_CHECKED_DISABLED: &str = "feathers.switch.bg.checked.disabled";
/// Switch border around the checkmark
pub const SWITCH_BORDER: &str = "feathers.switch.border";
/// Switch border around the checkmark (hovered)
pub const SWITCH_BORDER_HOVER: &str = "feathers.switch.border.hover";
/// Switch border around the checkmark (disabled)
pub const SWITCH_BORDER_DISABLED: &str = "feathers.switch.border.disabled";
/// Switch slide
pub const SWITCH_SLIDE: &str = "feathers.switch.slide";
/// Switch slide (disabled)
pub const SWITCH_SLIDE_DISABLED: &str = "feathers.switch.slide.disabled";

// Pane

/// Pane header background
pub const PANE_HEADER_BG: &str = "feathers.pane.header.bg";
/// Pane header border
pub const PANE_HEADER_BORDER: &str = "feathers.pane.header.border";
/// Pane header text color
pub const PANE_HEADER_TEXT: &str = "feathers.pane.header.text";
/// Pane header divider color
pub const PANE_HEADER_DIVIDER: &str = "feathers.pane.header.divider";

// Subpane

/// Subpane background
pub const SUBPANE_HEADER_BG: &str = "feathers.subpane.header.bg";
/// Subpane header border
pub const SUBPANE_HEADER_BORDER: &str = "feathers.subpane.header.border";
/// Subpane header text color
pub const SUBPANE_HEADER_TEXT: &str = "feathers.subpane.header.text";
/// Subpane body background
pub const SUBPANE_BODY_BG: &str = "feathers.subpane.body.bg";
/// Subpane body border
pub const SUBPANE_BODY_BORDER: &str = "feathers.subpane.body.border";

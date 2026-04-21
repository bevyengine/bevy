//! Design tokens used by Feathers themes.
//!
//! The term "design token" is commonly used in UX design to mean the smallest unit of a theme,
//! similar in concept to a CSS variable. Each token represents an assignment of a color or
//! value to a specific visual aspect of a widget, such as background or border.

use crate::theme::ThemeToken;

/// Window background
pub const WINDOW_BG: ThemeToken = ThemeToken::new_static("feathers.window.bg");

/// Focus ring
pub const FOCUS_RING: ThemeToken = ThemeToken::new_static("feathers.focus");

/// Regular text
pub const TEXT_MAIN: ThemeToken = ThemeToken::new_static("feathers.text.main");
/// Dim text
pub const TEXT_DIM: ThemeToken = ThemeToken::new_static("feathers.text.dim");

// Normal buttons

/// Regular button background
pub const BUTTON_BG: ThemeToken = ThemeToken::new_static("feathers.button.bg");
/// Regular button background (hovered)
pub const BUTTON_BG_HOVER: ThemeToken = ThemeToken::new_static("feathers.button.bg.hover");
/// Regular button background (disabled)
pub const BUTTON_BG_DISABLED: ThemeToken = ThemeToken::new_static("feathers.button.bg.disabled");
/// Regular button background (pressed)
pub const BUTTON_BG_PRESSED: ThemeToken = ThemeToken::new_static("feathers.button.bg.pressed");
/// Regular button text
pub const BUTTON_TEXT: ThemeToken = ThemeToken::new_static("feathers.button.txt");
/// Regular button text (disabled)
pub const BUTTON_TEXT_DISABLED: ThemeToken = ThemeToken::new_static("feathers.button.txt.disabled");

// Primary ("default") buttons

/// Primary button background
pub const BUTTON_PRIMARY_BG: ThemeToken = ThemeToken::new_static("feathers.button.primary.bg");
/// Primary button background (hovered)
pub const BUTTON_PRIMARY_BG_HOVER: ThemeToken =
    ThemeToken::new_static("feathers.button.primary.bg.hover");
/// Primary button background (disabled)
pub const BUTTON_PRIMARY_BG_DISABLED: ThemeToken =
    ThemeToken::new_static("feathers.button.primary.bg.disabled");
/// Primary button background (pressed)
pub const BUTTON_PRIMARY_BG_PRESSED: ThemeToken =
    ThemeToken::new_static("feathers.button.primary.bg.pressed");
/// Primary button text
pub const BUTTON_PRIMARY_TEXT: ThemeToken = ThemeToken::new_static("feathers.button.primary.txt");
/// Primary button text (disabled)
pub const BUTTON_PRIMARY_TEXT_DISABLED: ThemeToken =
    ThemeToken::new_static("feathers.button.primary.txt.disabled");

// Plain buttons (transparent background)

/// Plain button background
pub const BUTTON_PLAIN_BG: ThemeToken = ThemeToken::new_static("feathers.button.plain.bg");
/// Plain button background (hovered)
pub const BUTTON_PLAIN_BG_HOVER: ThemeToken =
    ThemeToken::new_static("feathers.button.plain.bg.hover");
/// Plain button background (disabled)
pub const BUTTON_PLAIN_BG_DISABLED: ThemeToken =
    ThemeToken::new_static("feathers.button.plain.bg.disabled");
/// Plain button background (pressed)
pub const BUTTON_PLAIN_BG_PRESSED: ThemeToken =
    ThemeToken::new_static("feathers.button.plain.bg.pressed");

// Slider

/// Background for slider
pub const SLIDER_BG: ThemeToken = ThemeToken::new_static("feathers.slider.bg");
/// Background for slider (hovered)
pub const SLIDER_BG_HOVER: ThemeToken = ThemeToken::new_static("feathers.slider.bg.hover");
/// Background for slider (pressed)
pub const SLIDER_BG_PRESSED: ThemeToken = ThemeToken::new_static("feathers.slider.bg.pressed");
/// Background for slider (disabled)
pub const SLIDER_BG_DISABLED: ThemeToken = ThemeToken::new_static("feathers.slider.bg.disabled");
/// Fill color for slider
pub const SLIDER_BAR: ThemeToken = ThemeToken::new_static("feathers.slider.bar");
/// Fill color for slider (hovered)
pub const SLIDER_BAR_HOVER: ThemeToken = ThemeToken::new_static("feathers.slider.bar.hover");
/// Fill color for slider (pressed)
pub const SLIDER_BAR_PRESSED: ThemeToken = ThemeToken::new_static("feathers.slider.bar.pressed");
/// Fill color for slider (disabled)
pub const SLIDER_BAR_DISABLED: ThemeToken = ThemeToken::new_static("feathers.slider.bar.disabled");
/// Background for slider text
pub const SLIDER_TEXT: ThemeToken = ThemeToken::new_static("feathers.slider.text");
/// Background for slider text (disabled)
pub const SLIDER_TEXT_DISABLED: ThemeToken =
    ThemeToken::new_static("feathers.slider.text.disabled");

// Scrollbar

/// Background for scrollbar
pub const SCROLLBAR_BG: ThemeToken = ThemeToken::new_static("feathers.scrollbar.bg");
/// Background for scrollbar moving bar
pub const SCROLLBAR_THUMB: ThemeToken = ThemeToken::new_static("feathers.scrollbar.thumb");
/// Background for scrollbar moving bar (hovered)
pub const SCROLLBAR_THUMB_HOVER: ThemeToken =
    ThemeToken::new_static("feathers.scrollbar.thumb.hover");

// Checkbox

/// Checkbox background around the checkmark
pub const CHECKBOX_BG: ThemeToken = ThemeToken::new_static("feathers.checkbox.bg");
/// Checkbox background around the checkmark (hovered)
pub const CHECKBOX_BG_HOVER: ThemeToken = ThemeToken::new_static("feathers.checkbox.bg.hover");
/// Checkbox background around the checkmark (pressed)
pub const CHECKBOX_BG_PRESSED: ThemeToken = ThemeToken::new_static("feathers.checkbox.bg.pressed");
/// Checkbox border around the checkmark (disabled)
pub const CHECKBOX_BG_DISABLED: ThemeToken =
    ThemeToken::new_static("feathers.checkbox.bg.disabled");
/// Checkbox background around the checkmark (checked)
pub const CHECKBOX_BG_CHECKED: ThemeToken = ThemeToken::new_static("feathers.checkbox.bg.checked");
/// Checkbox background around the checkmark (checked+hover)
pub const CHECKBOX_BG_CHECKED_HOVER: ThemeToken =
    ThemeToken::new_static("feathers.checkbox.bg.checked.hover");
/// Checkbox background around the checkmark (checked+pressed)
pub const CHECKBOX_BG_CHECKED_PRESSED: ThemeToken =
    ThemeToken::new_static("feathers.checkbox.bg.checked.pressed");
/// Checkbox border around the checkmark (checked+disabled)
pub const CHECKBOX_BG_CHECKED_DISABLED: ThemeToken =
    ThemeToken::new_static("feathers.checkbox.bg.checked.disabled");
/// Checkbox border around the checkmark
pub const CHECKBOX_BORDER: ThemeToken = ThemeToken::new_static("feathers.checkbox.border");
/// Checkbox border around the checkmark (hovered)
pub const CHECKBOX_BORDER_HOVER: ThemeToken =
    ThemeToken::new_static("feathers.checkbox.border.hover");
/// Checkbox border around the checkmark (pressed)
pub const CHECKBOX_BORDER_PRESSED: ThemeToken =
    ThemeToken::new_static("feathers.checkbox.border.pressed");
/// Checkbox border around the checkmark (disabled)
pub const CHECKBOX_BORDER_DISABLED: ThemeToken =
    ThemeToken::new_static("feathers.checkbox.border.disabled");
/// Checkbox border around the checkmark (checked)
pub const CHECKBOX_BORDER_CHECKED: ThemeToken =
    ThemeToken::new_static("feathers.checkbox.border.checked");
/// Checkbox border around the checkmark (checked+hovered)
pub const CHECKBOX_BORDER_CHECKED_HOVER: ThemeToken =
    ThemeToken::new_static("feathers.checkbox.border.checked.hover");
/// Checkbox border around the checkmark (checked+pressed)
pub const CHECKBOX_BORDER_CHECKED_PRESSED: ThemeToken =
    ThemeToken::new_static("feathers.checkbox.border.checked.pressed");
/// Checkbox border around the checkmark (checked+disabled)
pub const CHECKBOX_BORDER_CHECKED_DISABLED: ThemeToken =
    ThemeToken::new_static("feathers.checkbox.border.checked.disabled");
/// Checkbox check mark
pub const CHECKBOX_MARK: ThemeToken = ThemeToken::new_static("feathers.checkbox.mark");
/// Checkbox check mark (disabled)
pub const CHECKBOX_MARK_DISABLED: ThemeToken =
    ThemeToken::new_static("feathers.checkbox.mark.disabled");
/// Checkbox label text
pub const CHECKBOX_TEXT: ThemeToken = ThemeToken::new_static("feathers.checkbox.text");
/// Checkbox label text (disabled)
pub const CHECKBOX_TEXT_DISABLED: ThemeToken =
    ThemeToken::new_static("feathers.checkbox.text.disabled");

// Radio button

/// Border around the radio button
pub const RADIO_BORDER: ThemeToken = ThemeToken::new_static("feathers.radio.border");
/// Border around the radio button (hovered)
pub const RADIO_BORDER_HOVER: ThemeToken = ThemeToken::new_static("feathers.radio.border.hover");
/// Border around the radio button (pressed)
pub const RADIO_BORDER_PRESSED: ThemeToken =
    ThemeToken::new_static("feathers.radio.border.pressed");
/// Border around the radio button (disabled)
pub const RADIO_BORDER_DISABLED: ThemeToken =
    ThemeToken::new_static("feathers.radio.border.disabled");
/// Border around the radio button (checked)
pub const RADIO_BORDER_CHECKED: ThemeToken =
    ThemeToken::new_static("feathers.radio.border.checked");
/// Border around the radio button (checked+hovered)
pub const RADIO_BORDER_CHECKED_HOVER: ThemeToken =
    ThemeToken::new_static("feathers.radio.border.checked.hover");
/// Border around the radio button (checked+pressed)
pub const RADIO_BORDER_CHECKED_PRESSED: ThemeToken =
    ThemeToken::new_static("feathers.radio.border.checked.pressed");
/// Border around the radio button (checked+disabled)
pub const RADIO_BORDER_CHECKED_DISABLED: ThemeToken =
    ThemeToken::new_static("feathers.radio.border.checked.disabled");
/// Radio check mark
pub const RADIO_MARK: ThemeToken = ThemeToken::new_static("feathers.radio.mark");
/// Radio check mark (hovered)
pub const RADIO_MARK_HOVER: ThemeToken = ThemeToken::new_static("feathers.radio.mark.hover");
/// Radio check mark (pressed)
pub const RADIO_MARK_PRESSED: ThemeToken = ThemeToken::new_static("feathers.radio.mark.pressed");
/// Radio check mark (disabled)
pub const RADIO_MARK_DISABLED: ThemeToken = ThemeToken::new_static("feathers.radio.mark.disabled");
/// Radio label text
pub const RADIO_TEXT: ThemeToken = ThemeToken::new_static("feathers.radio.text");
/// Radio label text (disabled)
pub const RADIO_TEXT_DISABLED: ThemeToken = ThemeToken::new_static("feathers.radio.text.disabled");

// Toggle Switch

/// Switch background around the switch
pub const SWITCH_BG: ThemeToken = ThemeToken::new_static("feathers.switch.bg");
/// Switch background around the switch (hovered)
pub const SWITCH_BG_HOVER: ThemeToken = ThemeToken::new_static("feathers.switch.bg.hover");
/// Switch background around the switch (pressed)
pub const SWITCH_BG_PRESSED: ThemeToken = ThemeToken::new_static("feathers.switch.bg.pressed");
/// Switch background around the switch (disabled)
pub const SWITCH_BG_DISABLED: ThemeToken = ThemeToken::new_static("feathers.switch.bg.disabled");
/// Switch background around the switch (checked)
pub const SWITCH_BG_CHECKED: ThemeToken = ThemeToken::new_static("feathers.switch.bg.checked");
/// Switch background around the switch (checked+hover)
pub const SWITCH_BG_CHECKED_HOVER: ThemeToken =
    ThemeToken::new_static("feathers.switch.bg.checked.hover");
/// Switch background around the switch (checked+pressed)
pub const SWITCH_BG_CHECKED_PRESSED: ThemeToken =
    ThemeToken::new_static("feathers.switch.bg.checked.pressed");
/// Switch background around the switch (checked+disabled)
pub const SWITCH_BG_CHECKED_DISABLED: ThemeToken =
    ThemeToken::new_static("feathers.switch.bg.checked.disabled");
/// Switch border around the switch
pub const SWITCH_BORDER: ThemeToken = ThemeToken::new_static("feathers.switch.border");
/// Switch border around the switch (hovered)
pub const SWITCH_BORDER_HOVER: ThemeToken = ThemeToken::new_static("feathers.switch.border.hover");
/// Switch border around the switch (pressed)
pub const SWITCH_BORDER_PRESSED: ThemeToken =
    ThemeToken::new_static("feathers.switch.border.hover.pressed");
/// Switch border around the switch (disabled)
pub const SWITCH_BORDER_DISABLED: ThemeToken =
    ThemeToken::new_static("feathers.switch.border.disabled");
/// Switch border around the switch (checked)
pub const SWITCH_BORDER_CHECKED: ThemeToken =
    ThemeToken::new_static("feathers.switch.border.checked");
/// Switch border around the switch (checked+hovered)
pub const SWITCH_BORDER_CHECKED_HOVER: ThemeToken =
    ThemeToken::new_static("feathers.switch.border.checked.hover");
/// Switch border around the switch (checked+pressed)
pub const SWITCH_BORDER_CHECKED_PRESSED: ThemeToken =
    ThemeToken::new_static("feathers.switch.border.checked.pressed");
/// Switch border around the switch (checked+disabled)
pub const SWITCH_BORDER_CHECKED_DISABLED: ThemeToken =
    ThemeToken::new_static("feathers.switch.border.checked.disabled");
/// Switch slide background
pub const SWITCH_SLIDE_BG: ThemeToken = ThemeToken::new_static("feathers.switch.slide.bg");
/// Switch slide background (hovered)
pub const SWITCH_SLIDE_BG_HOVER: ThemeToken =
    ThemeToken::new_static("feathers.switch.slide.bg.hover");
/// Switch slide background (pressed)
pub const SWITCH_SLIDE_BG_PRESSED: ThemeToken =
    ThemeToken::new_static("feathers.switch.slide.bg.pressed");
/// Switch slide background (disabled)
pub const SWITCH_SLIDE_BG_DISABLED: ThemeToken =
    ThemeToken::new_static("feathers.switch.slide.bg.disabled");
/// Switch slide background (checked)
pub const SWITCH_SLIDE_BG_CHECKED: ThemeToken =
    ThemeToken::new_static("feathers.switch.slide.bg.checked");
/// Switch slide background (checked+hovered)
pub const SWITCH_SLIDE_BG_CHECKED_HOVER: ThemeToken =
    ThemeToken::new_static("feathers.switch.slide.bg.checked.hover");
/// Switch slide background (checked+pressed)
pub const SWITCH_SLIDE_BG_CHECKED_PRESSED: ThemeToken =
    ThemeToken::new_static("feathers.switch.slide.bg.checked.pressed");
/// Switch slide background (checked+disabled)
pub const SWITCH_SLIDE_BG_CHECKED_DISABLED: ThemeToken =
    ThemeToken::new_static("feathers.switch.slide.bg.checked.disabled");

/// Switch slide border
pub const SWITCH_SLIDE_BORDER: ThemeToken = ThemeToken::new_static("feathers.switch.slide.border");
/// Switch slide border (hovered)
pub const SWITCH_SLIDE_BORDER_HOVER: ThemeToken =
    ThemeToken::new_static("feathers.switch.slide.border.hover");
/// Switch slide border (pressed)
pub const SWITCH_SLIDE_BORDER_PRESSED: ThemeToken =
    ThemeToken::new_static("feathers.switch.slide.border.pressed");
/// Switch slide border (disabled)
pub const SWITCH_SLIDE_BORDER_DISABLED: ThemeToken =
    ThemeToken::new_static("feathers.switch.slide.border.disabled");
/// Switch slide border (checked)
pub const SWITCH_SLIDE_BORDER_CHECKED: ThemeToken =
    ThemeToken::new_static("feathers.switch.slide.border.checked");
/// Switch slide border (checked+hovered)
pub const SWITCH_SLIDE_BORDER_CHECKED_HOVER: ThemeToken =
    ThemeToken::new_static("feathers.switch.slide.border.checked.hover");
/// Switch slide border (checked+pressed)
pub const SWITCH_SLIDE_BORDER_CHECKED_PRESSED: ThemeToken =
    ThemeToken::new_static("feathers.switch.slide.border.checked.pressed");
/// Switch slide border (checked+disabled)
pub const SWITCH_SLIDE_BORDER_CHECKED_DISABLED: ThemeToken =
    ThemeToken::new_static("feathers.switch.slide.border.checked.disabled");

// Color Plane

/// Color plane frame background
pub const COLOR_PLANE_BG: ThemeToken = ThemeToken::new_static("feathers.colorplane.bg");

// Menus

/// Menu background
pub const MENU_BG: ThemeToken = ThemeToken::new_static("feathers.menu.bg");
/// Menu border
pub const MENU_BORDER: ThemeToken = ThemeToken::new_static("feathers.menu.border");
/// Menu item hovered
pub const MENUITEM_BG_HOVER: ThemeToken = ThemeToken::new_static("feathers.menuitem.bg.hover");
/// Menu item pressed
pub const MENUITEM_BG_PRESSED: ThemeToken = ThemeToken::new_static("feathers.menuitem.bg.pressed");
/// Menu item focused
pub const MENUITEM_BG_FOCUSED: ThemeToken = ThemeToken::new_static("feathers.menuitem.bg.focused");
/// Menu item text
pub const MENUITEM_TEXT: ThemeToken = ThemeToken::new_static("feathers.menuitem.text");
/// Menu item text (disabled)
pub const MENUITEM_TEXT_DISABLED: ThemeToken =
    ThemeToken::new_static("feathers.menuitem.text.disabled");

// Text Input

/// Background for text input
pub const TEXT_INPUT_BG: ThemeToken = ThemeToken::new_static("feathers.textinput.bg");
/// Text color for text input
pub const TEXT_INPUT_TEXT: ThemeToken = ThemeToken::new_static("feathers.textinput.text");
/// Text color for text input (disabled)
pub const TEXT_INPUT_TEXT_DISABLED: ThemeToken =
    ThemeToken::new_static("feathers.textinput.text.disabled");
/// Cursor color for text input
pub const TEXT_INPUT_CURSOR: ThemeToken = ThemeToken::new_static("feathers.textinput.cursor");
/// Selection color for text input
pub const TEXT_INPUT_SELECTION: ThemeToken = ThemeToken::new_static("feathers.textinput.selection");
/// Background color for label text
pub const TEXT_INPUT_LABEL_BG: ThemeToken = ThemeToken::new_static("feathers.textinput.label.bg");
/// Sigil color for X
pub const TEXT_INPUT_X_AXIS: ThemeToken = ThemeToken::new_static("feathers.textinput.axis.x");
/// Sigil color for Y
pub const TEXT_INPUT_Y_AXIS: ThemeToken = ThemeToken::new_static("feathers.textinput.axis.y");
/// Sigil color for Z
pub const TEXT_INPUT_Z_AXIS: ThemeToken = ThemeToken::new_static("feathers.textinput.axis.z");

// Pane

/// Pane header background
pub const PANE_HEADER_BG: ThemeToken = ThemeToken::new_static("feathers.pane.header.bg");
/// Pane header border
pub const PANE_HEADER_BORDER: ThemeToken = ThemeToken::new_static("feathers.pane.header.border");
/// Pane header text color
pub const PANE_HEADER_TEXT: ThemeToken = ThemeToken::new_static("feathers.pane.header.text");
/// Pane header divider color
pub const PANE_HEADER_DIVIDER: ThemeToken = ThemeToken::new_static("feathers.pane.header.divider");
/// Pane body background
pub const PANE_BODY_BG: ThemeToken = ThemeToken::new_static("feathers.pane.body.bg");

// Subpane

/// Subpane background
pub const SUBPANE_HEADER_BG: ThemeToken = ThemeToken::new_static("feathers.subpane.header.bg");
/// Subpane header border
pub const SUBPANE_HEADER_BORDER: ThemeToken =
    ThemeToken::new_static("feathers.subpane.header.border");
/// Subpane header text color
pub const SUBPANE_HEADER_TEXT: ThemeToken = ThemeToken::new_static("feathers.subpane.header.text");
/// Subpane body background
pub const SUBPANE_BODY_BG: ThemeToken = ThemeToken::new_static("feathers.subpane.body.bg");
/// Subpane body border
pub const SUBPANE_BODY_BORDER: ThemeToken = ThemeToken::new_static("feathers.subpane.body.border");

// Group

/// Group background
pub const GROUP_HEADER_BG: ThemeToken = ThemeToken::new_static("feathers.group.header.bg");
/// Group header border
pub const GROUP_HEADER_BORDER: ThemeToken = ThemeToken::new_static("feathers.group.header.border");
/// Group header text color
pub const GROUP_HEADER_TEXT: ThemeToken = ThemeToken::new_static("feathers.group.header.text");
/// Group body background
pub const GROUP_BODY_BG: ThemeToken = ThemeToken::new_static("feathers.group.body.bg");
/// Group body border
pub const GROUP_BODY_BORDER: ThemeToken = ThemeToken::new_static("feathers.group.body.border");

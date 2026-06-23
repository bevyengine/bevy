//! The standard `bevy_feathers` dark theme.
use crate::tokens;
use bevy_color::{Alpha, Color, Luminance};
use bevy_platform::collections::HashMap;

use crate::theme::ThemeProps;

/// Create a [`ThemeProps`] object and populate it with the colors for the default dark theme.
pub fn create_dark_theme() -> ThemeProps {
    ThemeProps {
        color: HashMap::from([
            (tokens::WINDOW_BG, dark_palette::GRAY_0),
            (tokens::FOCUS_RING, dark_palette::ACCENT.with_alpha(0.5)),
            (tokens::TEXT_MAIN, dark_palette::TEXT_GRAY_1),
            (tokens::TEXT_DIM, dark_palette::TEXT_GRAY_2),
            // Button (normal)
            (tokens::BUTTON_BG, dark_palette::GRAY_3),
            (tokens::BUTTON_BG_HOVER, dark_palette::GRAY_3.lighter(0.05)),
            (tokens::BUTTON_BG_PRESSED, dark_palette::GRAY_3.lighter(0.1)),
            (tokens::BUTTON_BG_DISABLED, dark_palette::GRAY_2),
            // Button (primary)
            (tokens::BUTTON_PRIMARY_BG, dark_palette::ACCENT),
            (
                tokens::BUTTON_PRIMARY_BG_HOVER,
                dark_palette::ACCENT.lighter(0.05),
            ),
            (
                tokens::BUTTON_PRIMARY_BG_PRESSED,
                dark_palette::ACCENT.lighter(0.1),
            ),
            (tokens::BUTTON_PRIMARY_BG_DISABLED, dark_palette::GRAY_2),
            // Button (plain)
            (tokens::BUTTON_PLAIN_BG, Color::NONE),
            (tokens::BUTTON_PLAIN_BG_HOVER, dark_palette::GRAY_2),
            (tokens::BUTTON_PLAIN_BG_PRESSED, dark_palette::GRAY_3),
            (tokens::BUTTON_PLAIN_BG_DISABLED, Color::NONE),
            // Button text
            (tokens::BUTTON_TEXT, dark_palette::WHITE),
            (
                tokens::BUTTON_TEXT_DISABLED,
                dark_palette::WHITE.with_alpha(0.5),
            ),
            (tokens::BUTTON_PRIMARY_TEXT, dark_palette::WHITE),
            (
                tokens::BUTTON_PRIMARY_TEXT_DISABLED,
                dark_palette::WHITE.with_alpha(0.5),
            ),
            // Slider
            (
                tokens::SLIDER_BG,
                dark_palette::LIGHT_GRAY_MIX.with_alpha(0.028),
            ),
            (
                tokens::SLIDER_BG_HOVER,
                dark_palette::LIGHT_GRAY_MIX.with_alpha(0.045),
            ),
            (
                tokens::SLIDER_BG_PRESSED,
                dark_palette::LIGHT_GRAY_MIX.with_alpha(0.045),
            ),
            (tokens::SLIDER_BG_DISABLED, dark_palette::GRAY_1),
            (tokens::SLIDER_BAR, dark_palette::ACCENT),
            (tokens::SLIDER_BAR_HOVER, dark_palette::ACCENT.lighter(0.05)),
            (
                tokens::SLIDER_BAR_PRESSED,
                dark_palette::ACCENT.lighter(0.05),
            ),
            (tokens::SLIDER_BAR_DISABLED, dark_palette::GRAY_2),
            (tokens::SLIDER_TEXT, dark_palette::WHITE),
            (
                tokens::SLIDER_TEXT_DISABLED,
                dark_palette::WHITE.with_alpha(0.5),
            ),
            // Scrollbar
            (tokens::SCROLLBAR_BG, dark_palette::GRAY_2),
            (tokens::SCROLLBAR_THUMB, dark_palette::ACCENT),
            (
                tokens::SCROLLBAR_THUMB_HOVER,
                dark_palette::ACCENT.lighter(0.1),
            ),
            // Checkbox
            (tokens::CHECKBOX_BG, dark_palette::GRAY_3),
            (tokens::CHECKBOX_BG_HOVER, dark_palette::GRAY_3),
            (tokens::CHECKBOX_BG_PRESSED, dark_palette::GRAY_3),
            (
                tokens::CHECKBOX_BG_DISABLED,
                dark_palette::GRAY_1.with_alpha(0.5),
            ),
            (tokens::CHECKBOX_BG_CHECKED, dark_palette::ACCENT),
            (
                tokens::CHECKBOX_BG_CHECKED_HOVER,
                dark_palette::ACCENT.lighter(0.05),
            ),
            (
                tokens::CHECKBOX_BG_CHECKED_PRESSED,
                dark_palette::ACCENT.lighter(0.1),
            ),
            (
                tokens::CHECKBOX_BG_CHECKED_DISABLED,
                dark_palette::GRAY_1.with_alpha(0.5),
            ),
            (tokens::CHECKBOX_BORDER, dark_palette::GRAY_3),
            (
                tokens::CHECKBOX_BORDER_HOVER,
                dark_palette::GRAY_3.lighter(0.05),
            ),
            (
                tokens::CHECKBOX_BORDER_PRESSED,
                dark_palette::GRAY_3.lighter(0.1),
            ),
            (
                tokens::CHECKBOX_BORDER_DISABLED,
                dark_palette::GRAY_3.with_alpha(0.5),
            ),
            (tokens::CHECKBOX_BORDER_CHECKED, dark_palette::ACCENT),
            (
                tokens::CHECKBOX_BORDER_CHECKED_HOVER,
                dark_palette::ACCENT.lighter(0.05),
            ),
            (
                tokens::CHECKBOX_BORDER_CHECKED_PRESSED,
                dark_palette::ACCENT.lighter(0.1),
            ),
            (
                tokens::CHECKBOX_BORDER_CHECKED_DISABLED,
                dark_palette::GRAY_3.with_alpha(0.5),
            ),
            (tokens::CHECKBOX_MARK, dark_palette::WHITE),
            (tokens::CHECKBOX_MARK_DISABLED, dark_palette::TEXT_GRAY_2),
            (tokens::CHECKBOX_TEXT, dark_palette::TEXT_GRAY_1),
            (
                tokens::CHECKBOX_TEXT_DISABLED,
                dark_palette::TEXT_GRAY_1.with_alpha(0.5),
            ),
            // Radio (default look is no background)
            (tokens::RADIO_BG, Color::NONE),
            (tokens::RADIO_BG_HOVER, Color::NONE),
            (tokens::RADIO_BG_PRESSED, Color::NONE),
            (tokens::RADIO_BG_DISABLED, Color::NONE),
            (tokens::RADIO_BG_CHECKED, Color::NONE),
            (tokens::RADIO_BG_CHECKED_HOVER, Color::NONE),
            (tokens::RADIO_BG_CHECKED_PRESSED, Color::NONE),
            (tokens::RADIO_BG_CHECKED_DISABLED, Color::NONE),
            (tokens::RADIO_BORDER, dark_palette::GRAY_3),
            (
                tokens::RADIO_BORDER_HOVER,
                dark_palette::GRAY_3.lighter(0.05),
            ),
            (
                tokens::RADIO_BORDER_PRESSED,
                dark_palette::GRAY_3.lighter(0.1),
            ),
            (
                tokens::RADIO_BORDER_DISABLED,
                dark_palette::GRAY_3.with_alpha(0.5),
            ),
            (tokens::RADIO_BORDER_CHECKED, dark_palette::ACCENT),
            (
                tokens::RADIO_BORDER_CHECKED_HOVER,
                dark_palette::ACCENT.lighter(0.05),
            ),
            (
                tokens::RADIO_BORDER_CHECKED_PRESSED,
                dark_palette::ACCENT.lighter(0.1),
            ),
            (
                tokens::RADIO_BORDER_CHECKED_DISABLED,
                dark_palette::GRAY_3.with_alpha(0.5),
            ),
            (tokens::RADIO_MARK, dark_palette::ACCENT),
            (tokens::RADIO_MARK_HOVER, dark_palette::ACCENT.lighter(0.05)),
            (
                tokens::RADIO_MARK_PRESSED,
                dark_palette::ACCENT.lighter(0.1),
            ),
            (
                tokens::RADIO_MARK_DISABLED,
                dark_palette::GRAY_3.with_alpha(0.5),
            ),
            (tokens::RADIO_TEXT, dark_palette::TEXT_GRAY_1),
            (
                tokens::RADIO_TEXT_DISABLED,
                dark_palette::TEXT_GRAY_1.with_alpha(0.5),
            ),
            // Toggle Switch
            (tokens::SWITCH_BG, dark_palette::GRAY_3),
            (tokens::SWITCH_BG_HOVER, dark_palette::GRAY_3.lighter(0.05)),
            (tokens::SWITCH_BG_PRESSED, dark_palette::GRAY_3.lighter(0.1)),
            (
                tokens::SWITCH_BG_DISABLED,
                dark_palette::GRAY_1.with_alpha(0.5),
            ),
            (tokens::SWITCH_BG_CHECKED, dark_palette::ACCENT),
            (
                tokens::SWITCH_BG_CHECKED_HOVER,
                dark_palette::ACCENT.lighter(0.05),
            ),
            (
                tokens::SWITCH_BG_CHECKED_PRESSED,
                dark_palette::ACCENT.lighter(0.1),
            ),
            (
                tokens::SWITCH_BG_CHECKED_DISABLED,
                dark_palette::GRAY_1.with_alpha(0.5),
            ),
            (tokens::SWITCH_BORDER, dark_palette::GRAY_3),
            (
                tokens::SWITCH_BORDER_HOVER,
                dark_palette::GRAY_3.lighter(0.05),
            ),
            (
                tokens::SWITCH_BORDER_PRESSED,
                dark_palette::GRAY_3.lighter(0.1),
            ),
            (
                tokens::SWITCH_BORDER_DISABLED,
                dark_palette::GRAY_3.with_alpha(0.5),
            ),
            (tokens::SWITCH_BORDER_CHECKED, dark_palette::ACCENT),
            (
                tokens::SWITCH_BORDER_CHECKED_HOVER,
                dark_palette::ACCENT.lighter(0.05),
            ),
            (
                tokens::SWITCH_BORDER_CHECKED_PRESSED,
                dark_palette::ACCENT.lighter(0.1),
            ),
            (
                tokens::SWITCH_BORDER_CHECKED_DISABLED,
                dark_palette::GRAY_3.with_alpha(0.5),
            ),
            (
                tokens::SWITCH_SLIDE_BG,
                dark_palette::TEXT_GRAY_1.lighter(0.1),
            ),
            (
                tokens::SWITCH_SLIDE_BG_HOVER,
                dark_palette::TEXT_GRAY_1.lighter(0.1),
            ),
            (
                tokens::SWITCH_SLIDE_BG_PRESSED,
                dark_palette::TEXT_GRAY_1.lighter(0.1),
            ),
            (
                tokens::SWITCH_SLIDE_BG_DISABLED,
                dark_palette::GRAY_1.with_alpha(0.5),
            ),
            (
                tokens::SWITCH_SLIDE_BG_CHECKED,
                dark_palette::TEXT_GRAY_1.lighter(0.1),
            ),
            (
                tokens::SWITCH_SLIDE_BG_CHECKED_HOVER,
                dark_palette::TEXT_GRAY_1.lighter(0.1),
            ),
            (
                tokens::SWITCH_SLIDE_BG_CHECKED_PRESSED,
                dark_palette::TEXT_GRAY_1.lighter(0.1),
            ),
            (
                tokens::SWITCH_SLIDE_BG_CHECKED_DISABLED,
                dark_palette::TEXT_GRAY_2.with_alpha(0.3),
            ),
            (
                tokens::SWITCH_SLIDE_BORDER,
                dark_palette::TEXT_GRAY_1.lighter(0.1),
            ),
            (
                tokens::SWITCH_SLIDE_BORDER_HOVER,
                dark_palette::TEXT_GRAY_1.lighter(0.1),
            ),
            (
                tokens::SWITCH_SLIDE_BORDER_PRESSED,
                dark_palette::TEXT_GRAY_1.lighter(0.1),
            ),
            (
                tokens::SWITCH_SLIDE_BORDER_DISABLED,
                dark_palette::GRAY_2.with_alpha(0.5),
            ),
            (
                tokens::SWITCH_SLIDE_BORDER_CHECKED,
                dark_palette::TEXT_GRAY_1.lighter(0.1),
            ),
            (
                tokens::SWITCH_SLIDE_BORDER_CHECKED_HOVER,
                dark_palette::TEXT_GRAY_1.lighter(0.1),
            ),
            (
                tokens::SWITCH_SLIDE_BORDER_CHECKED_PRESSED,
                dark_palette::TEXT_GRAY_1.lighter(0.1),
            ),
            (
                tokens::SWITCH_SLIDE_BORDER_CHECKED_DISABLED,
                dark_palette::TEXT_GRAY_2.with_alpha(0.3),
            ),
            (tokens::COLOR_PLANE_BG, dark_palette::GRAY_1),
            // Menus
            (tokens::MENU_BG, dark_palette::GRAY_1),
            (tokens::MENU_BORDER, dark_palette::BORDER_GRAY),
            (
                tokens::MENUITEM_BG_HOVER,
                dark_palette::GRAY_1.lighter(0.05),
            ),
            (
                tokens::MENUITEM_BG_PRESSED,
                dark_palette::GRAY_1.lighter(0.1),
            ),
            (
                tokens::MENUITEM_BG_FOCUSED,
                dark_palette::GRAY_1.lighter(0.1),
            ),
            (tokens::MENUITEM_TEXT, dark_palette::WHITE),
            (
                tokens::MENUITEM_TEXT_DISABLED,
                dark_palette::WHITE.with_alpha(0.5),
            ),
            // Text Input
            (
                tokens::TEXT_INPUT_BG,
                dark_palette::LIGHT_GRAY_MIX.with_alpha(0.028),
            ),
            (
                tokens::TEXT_INPUT_LABEL_BG,
                dark_palette::LIGHT_GRAY_MIX.with_alpha(0.09),
            ),
            (tokens::TEXT_INPUT_TEXT, dark_palette::WHITE),
            (
                tokens::TEXT_INPUT_TEXT_DISABLED,
                dark_palette::WHITE.with_alpha(0.5),
            ),
            (tokens::TEXT_INPUT_CURSOR, dark_palette::ACCENT.lighter(0.2)),
            (tokens::TEXT_INPUT_SELECTION, dark_palette::ACCENT),
            (
                tokens::TEXT_INPUT_SELECTION_UNFOCUSED,
                dark_palette::TRANSPARENT,
            ),
            (tokens::TEXT_INPUT_X_AXIS, dark_palette::X_AXIS),
            (tokens::TEXT_INPUT_Y_AXIS, dark_palette::Y_AXIS),
            (tokens::TEXT_INPUT_Z_AXIS, dark_palette::Z_AXIS),
            // Pane
            (tokens::PANE_HEADER_BG, dark_palette::GRAY_0),
            (tokens::PANE_HEADER_BORDER, dark_palette::BORDER_GRAY),
            (tokens::PANE_HEADER_TEXT, dark_palette::TEXT_GRAY_1),
            (tokens::PANE_HEADER_DIVIDER, dark_palette::BORDER_GRAY),
            (tokens::PANE_BODY_BG, dark_palette::GRAY_1),
            // Subpane
            (tokens::SUBPANE_HEADER_BG, dark_palette::GRAY_2),
            (tokens::SUBPANE_HEADER_BORDER, dark_palette::GRAY_3),
            (tokens::SUBPANE_HEADER_TEXT, dark_palette::TEXT_GRAY_1),
            (tokens::SUBPANE_BODY_BG, dark_palette::GRAY_1),
            (tokens::SUBPANE_BODY_BORDER, dark_palette::GRAY_2),
            // Group
            (tokens::GROUP_HEADER_BG, dark_palette::GRAY_2),
            (tokens::GROUP_HEADER_BORDER, dark_palette::GRAY_3),
            (tokens::GROUP_HEADER_TEXT, dark_palette::TEXT_GRAY_1),
            (tokens::GROUP_BODY_BG, dark_palette::GRAY_2),
            (tokens::GROUP_BODY_BORDER, dark_palette::GRAY_3),
            // Listview
            (tokens::LISTROW_BG, Color::NONE),
            (
                tokens::LISTROW_BG_HOVER,
                dark_palette::GRAY_3.with_alpha(0.5),
            ),
            (tokens::LISTROW_BG_SELECTED, dark_palette::GRAY_3),
            (tokens::LISTROW_TEXT, dark_palette::WHITE),
            (
                tokens::LISTROW_TEXT_DISABLED,
                dark_palette::WHITE.with_alpha(0.5),
            ),
            (tokens::DIALOG_BG, dark_palette::GRAY_1),
            (tokens::DIALOG_BORDER, dark_palette::BORDER_GRAY),
            (tokens::DIALOG_HEADER_BG, dark_palette::GRAY_0),
            (tokens::DIALOG_TEXT, dark_palette::TEXT_GRAY_1),
        ]),
    }
}

// Feathers dark theme standard palette
mod dark_palette {
    use bevy_color::Color;

    /// <div style="background-color: transparent; width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const TRANSPARENT: Color = Color::oklcha(0.0, 0.0, 0.0, 0.0);
    /// <div style="background-color: #1F1F24; width: 10px; padding: 10px; border: 1px solid;"></div> - window background
    pub const GRAY_0: Color = Color::oklcha(0.2414, 0.0095, 285.67, 1.0);
    /// <div style="background-color: #2A2A2E; width: 10px; padding: 10px; border: 1px solid;"></div> - pane background
    pub const GRAY_1: Color = Color::oklcha(0.2866, 0.0072, 285.93, 1.0);
    /// <div style="background-color: #36373B; width: 10px; padding: 10px; border: 1px solid;"></div> - item background
    pub const GRAY_2: Color = Color::oklcha(0.3373, 0.0071, 274.77, 1.0);
    /// <div style="background-color: #46474D; width: 10px; padding: 10px; border: 1px solid;"></div> - item background (active)
    pub const GRAY_3: Color = Color::oklcha(0.3992, 0.0101, 278.38, 1.0);
    /// <div style="background-color: #414142; width: 10px; padding: 10px; border: 1px solid;"></div> - border
    pub const BORDER_GRAY: Color = Color::oklcha(0.3757, 0.0017, 286.32, 1.0);
    /// <div style="background-color: #B1B1B2; width: 10px; padding: 10px; border: 1px solid;"></div> - bright label text
    pub const TEXT_GRAY_1: Color = Color::oklcha(0.7607, 0.0014, 286.37, 1.0);
    /// <div style="background-color: #838385; width: 10px; padding: 10px; border: 1px solid;"></div> - dim label text
    pub const TEXT_GRAY_2: Color = Color::oklcha(0.6106, 0.003, 286.31, 1.0);
    /// <div style="background-color: #c0c3cf; width: 10px; padding: 10px; border: 1px solid;"></div> - mix with background to produce text fill
    pub const LIGHT_GRAY_MIX: Color = Color::oklcha(0.8185, 0.0171, 274.77, 1.0);
    /// <div style="background-color: #FFFFFF; width: 10px; padding: 10px; border: 1px solid;"></div> - button label text
    pub const WHITE: Color = Color::oklcha(1.0, 0.000000059604645, 90.0, 1.0);
    /// <div style="background-color: #206EC9; width: 10px; padding: 10px; border: 1px solid;"></div> - call-to-action and selection color
    pub const ACCENT: Color = Color::oklcha(0.542, 0.1594, 255.4, 1.0);
    /// <div style="background-color: #AB4051; width: 10px; padding: 10px; border: 1px solid;"></div> - for X-axis inputs and drag handles
    pub const X_AXIS: Color = Color::oklcha(0.5232, 0.1404, 13.84, 1.0);
    /// <div style="background-color: #5D8D0A; width: 10px; padding: 10px; border: 1px solid;"></div> - for Y-axis inputs and drag handles
    pub const Y_AXIS: Color = Color::oklcha(0.5866, 0.1543, 129.84, 1.0);
    /// <div style="background-color: #2160A3; width: 10px; padding: 10px; border: 1px solid;"></div> - for Z-axis inputs and drag handles
    pub const Z_AXIS: Color = Color::oklcha(0.4847, 0.1249, 253.08, 1.0);
}

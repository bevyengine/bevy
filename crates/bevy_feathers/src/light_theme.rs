//! The standard `bevy_feathers` light theme. NOT APPROVED YET
use crate::tokens;
use bevy_color::{Alpha, Color, Luminance};
use bevy_platform::collections::HashMap;

use crate::theme::ThemeProps;

/// Create a [`ThemeProps`] object and populate it with the colors for the alternative light theme.
pub fn create_light_theme() -> ThemeProps {
    ThemeProps {
        color: HashMap::from([
            (tokens::WINDOW_BG, light_palette::GRAY_0),
            (tokens::FOCUS_RING, light_palette::ACCENT.with_alpha(0.5)),
            (tokens::TEXT_MAIN, light_palette::TEXT_GRAY_1),
            (tokens::TEXT_DIM, light_palette::TEXT_GRAY_2),
            // Button (normal)
            (tokens::BUTTON_BG, light_palette::GRAY_3),
            (tokens::BUTTON_BG_HOVER, light_palette::GRAY_3.lighter(0.05)),
            (
                tokens::BUTTON_BG_PRESSED,
                light_palette::GRAY_3.lighter(0.1),
            ),
            (tokens::BUTTON_BG_DISABLED, light_palette::GRAY_2),
            // Button (primary)
            (tokens::BUTTON_PRIMARY_BG, light_palette::ACCENT),
            (
                tokens::BUTTON_PRIMARY_BG_HOVER,
                light_palette::ACCENT.lighter(0.05),
            ),
            (
                tokens::BUTTON_PRIMARY_BG_PRESSED,
                light_palette::ACCENT.lighter(0.1),
            ),
            (tokens::BUTTON_PRIMARY_BG_DISABLED, light_palette::GRAY_2),
            // Button (plain)
            (tokens::BUTTON_PLAIN_BG, Color::NONE),
            (tokens::BUTTON_PLAIN_BG_HOVER, light_palette::GRAY_2),
            (tokens::BUTTON_PLAIN_BG_PRESSED, light_palette::GRAY_3),
            (tokens::BUTTON_PLAIN_BG_DISABLED, Color::NONE),
            // Button text
            (tokens::BUTTON_TEXT, light_palette::TEXT_GRAY_1),
            (
                tokens::BUTTON_TEXT_DISABLED,
                light_palette::TEXT_GRAY_1.with_alpha(0.65),
            ),
            // Primary button sits on ACCENT, so its text stays white.
            (tokens::BUTTON_PRIMARY_TEXT, light_palette::WHITE),
            (
                tokens::BUTTON_PRIMARY_TEXT_DISABLED,
                light_palette::TEXT_GRAY_1.with_alpha(0.65),
            ),
            // Slider
            (
                tokens::SLIDER_BG,
                light_palette::DARK_GRAY_MIX.with_alpha(0.16),
            ),
            (
                tokens::SLIDER_BG_HOVER,
                light_palette::DARK_GRAY_MIX.with_alpha(0.22),
            ),
            (
                tokens::SLIDER_BG_PRESSED,
                light_palette::DARK_GRAY_MIX.with_alpha(0.22),
            ),
            (tokens::SLIDER_BG_DISABLED, light_palette::GRAY_1),
            (tokens::SLIDER_BAR, light_palette::ACCENT),
            (
                tokens::SLIDER_BAR_HOVER,
                light_palette::ACCENT.lighter(0.05),
            ),
            (
                tokens::SLIDER_BAR_PRESSED,
                light_palette::ACCENT.lighter(0.05),
            ),
            (tokens::SLIDER_BAR_DISABLED, light_palette::GRAY_2),
            // Slider value text spans both the light track and the accent fill,
            // so it must be dark to stay legible over the unfilled portion.
            (tokens::SLIDER_TEXT, light_palette::TEXT_GRAY_1),
            (
                tokens::SLIDER_TEXT_DISABLED,
                light_palette::TEXT_GRAY_1.with_alpha(0.65),
            ),
            // Scrollbar
            (tokens::SCROLLBAR_BG, light_palette::GRAY_2),
            (tokens::SCROLLBAR_THUMB, light_palette::ACCENT),
            (
                tokens::SCROLLBAR_THUMB_HOVER,
                light_palette::ACCENT.lighter(0.1),
            ),
            // Checkbox
            (tokens::CHECKBOX_BG, light_palette::GRAY_3),
            (tokens::CHECKBOX_BG_HOVER, light_palette::GRAY_3),
            (tokens::CHECKBOX_BG_PRESSED, light_palette::GRAY_3),
            (
                tokens::CHECKBOX_BG_DISABLED,
                light_palette::GRAY_1.with_alpha(0.5),
            ),
            (tokens::CHECKBOX_BG_CHECKED, light_palette::ACCENT),
            (
                tokens::CHECKBOX_BG_CHECKED_HOVER,
                light_palette::ACCENT.lighter(0.05),
            ),
            (
                tokens::CHECKBOX_BG_CHECKED_PRESSED,
                light_palette::ACCENT.lighter(0.1),
            ),
            (
                tokens::CHECKBOX_BG_CHECKED_DISABLED,
                light_palette::GRAY_1.with_alpha(0.5),
            ),
            (tokens::CHECKBOX_BORDER, light_palette::GRAY_3),
            (
                tokens::CHECKBOX_BORDER_HOVER,
                light_palette::GRAY_3.lighter(0.05),
            ),
            (
                tokens::CHECKBOX_BORDER_PRESSED,
                light_palette::GRAY_3.lighter(0.1),
            ),
            (
                tokens::CHECKBOX_BORDER_DISABLED,
                light_palette::GRAY_3.with_alpha(0.5),
            ),
            (tokens::CHECKBOX_BORDER_CHECKED, light_palette::ACCENT),
            (
                tokens::CHECKBOX_BORDER_CHECKED_HOVER,
                light_palette::ACCENT.lighter(0.05),
            ),
            (
                tokens::CHECKBOX_BORDER_CHECKED_PRESSED,
                light_palette::ACCENT.lighter(0.1),
            ),
            (
                tokens::CHECKBOX_BORDER_CHECKED_DISABLED,
                light_palette::GRAY_3.with_alpha(0.5),
            ),
            (tokens::CHECKBOX_MARK, light_palette::WHITE),
            (tokens::CHECKBOX_MARK_DISABLED, light_palette::TEXT_GRAY_2),
            (tokens::CHECKBOX_TEXT, light_palette::TEXT_GRAY_1),
            (
                tokens::CHECKBOX_TEXT_DISABLED,
                light_palette::TEXT_GRAY_1.with_alpha(0.65),
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
            (tokens::RADIO_BORDER, light_palette::GRAY_3),
            (
                tokens::RADIO_BORDER_HOVER,
                light_palette::GRAY_3.lighter(0.05),
            ),
            (
                tokens::RADIO_BORDER_PRESSED,
                light_palette::GRAY_3.lighter(0.1),
            ),
            (
                tokens::RADIO_BORDER_DISABLED,
                light_palette::GRAY_3.with_alpha(0.5),
            ),
            (tokens::RADIO_BORDER_CHECKED, light_palette::ACCENT),
            (
                tokens::RADIO_BORDER_CHECKED_HOVER,
                light_palette::ACCENT.lighter(0.05),
            ),
            (
                tokens::RADIO_BORDER_CHECKED_PRESSED,
                light_palette::ACCENT.lighter(0.1),
            ),
            (
                tokens::RADIO_BORDER_CHECKED_DISABLED,
                light_palette::GRAY_3.with_alpha(0.5),
            ),
            (tokens::RADIO_MARK, light_palette::ACCENT),
            (
                tokens::RADIO_MARK_HOVER,
                light_palette::ACCENT.lighter(0.05),
            ),
            (
                tokens::RADIO_MARK_PRESSED,
                light_palette::ACCENT.lighter(0.1),
            ),
            (
                tokens::RADIO_MARK_DISABLED,
                light_palette::GRAY_3.with_alpha(0.5),
            ),
            (tokens::RADIO_TEXT, light_palette::TEXT_GRAY_1),
            (
                tokens::RADIO_TEXT_DISABLED,
                light_palette::TEXT_GRAY_1.with_alpha(0.65),
            ),
            // Toggle Switch
            (tokens::SWITCH_BG, light_palette::GRAY_3),
            (tokens::SWITCH_BG_HOVER, light_palette::GRAY_3.lighter(0.05)),
            (
                tokens::SWITCH_BG_PRESSED,
                light_palette::GRAY_3.lighter(0.1),
            ),
            (
                tokens::SWITCH_BG_DISABLED,
                light_palette::GRAY_1.with_alpha(0.5),
            ),
            (tokens::SWITCH_BG_CHECKED, light_palette::ACCENT),
            (
                tokens::SWITCH_BG_CHECKED_HOVER,
                light_palette::ACCENT.lighter(0.05),
            ),
            (
                tokens::SWITCH_BG_CHECKED_PRESSED,
                light_palette::ACCENT.lighter(0.1),
            ),
            (
                tokens::SWITCH_BG_CHECKED_DISABLED,
                light_palette::GRAY_1.with_alpha(0.5),
            ),
            (tokens::SWITCH_BORDER, light_palette::GRAY_3),
            (
                tokens::SWITCH_BORDER_HOVER,
                light_palette::GRAY_3.lighter(0.05),
            ),
            (
                tokens::SWITCH_BORDER_PRESSED,
                light_palette::GRAY_3.lighter(0.1),
            ),
            (
                tokens::SWITCH_BORDER_DISABLED,
                light_palette::GRAY_3.with_alpha(0.5),
            ),
            (tokens::SWITCH_BORDER_CHECKED, light_palette::ACCENT),
            (
                tokens::SWITCH_BORDER_CHECKED_HOVER,
                light_palette::ACCENT.lighter(0.05),
            ),
            (
                tokens::SWITCH_BORDER_CHECKED_PRESSED,
                light_palette::ACCENT.lighter(0.1),
            ),
            (
                tokens::SWITCH_BORDER_CHECKED_DISABLED,
                light_palette::GRAY_3.with_alpha(0.5),
            ),
            (
                tokens::SWITCH_SLIDE_BG,
                light_palette::TEXT_GRAY_1.lighter(0.1),
            ),
            (
                tokens::SWITCH_SLIDE_BG_HOVER,
                light_palette::TEXT_GRAY_1.lighter(0.1),
            ),
            (
                tokens::SWITCH_SLIDE_BG_PRESSED,
                light_palette::TEXT_GRAY_1.lighter(0.1),
            ),
            (
                tokens::SWITCH_SLIDE_BG_DISABLED,
                light_palette::GRAY_1.with_alpha(0.5),
            ),
            (
                tokens::SWITCH_SLIDE_BG_CHECKED,
                light_palette::TEXT_GRAY_1.lighter(0.1),
            ),
            (
                tokens::SWITCH_SLIDE_BG_CHECKED_HOVER,
                light_palette::TEXT_GRAY_1.lighter(0.1),
            ),
            (
                tokens::SWITCH_SLIDE_BG_CHECKED_PRESSED,
                light_palette::TEXT_GRAY_1.lighter(0.1),
            ),
            (
                tokens::SWITCH_SLIDE_BG_CHECKED_DISABLED,
                light_palette::TEXT_GRAY_2.with_alpha(0.3),
            ),
            (
                tokens::SWITCH_SLIDE_BORDER,
                light_palette::TEXT_GRAY_1.lighter(0.1),
            ),
            (
                tokens::SWITCH_SLIDE_BORDER_HOVER,
                light_palette::TEXT_GRAY_1.lighter(0.1),
            ),
            (
                tokens::SWITCH_SLIDE_BORDER_PRESSED,
                light_palette::TEXT_GRAY_1.lighter(0.1),
            ),
            (
                tokens::SWITCH_SLIDE_BORDER_DISABLED,
                light_palette::GRAY_2.with_alpha(0.5),
            ),
            (
                tokens::SWITCH_SLIDE_BORDER_CHECKED,
                light_palette::TEXT_GRAY_1.lighter(0.1),
            ),
            (
                tokens::SWITCH_SLIDE_BORDER_CHECKED_HOVER,
                light_palette::TEXT_GRAY_1.lighter(0.1),
            ),
            (
                tokens::SWITCH_SLIDE_BORDER_CHECKED_PRESSED,
                light_palette::TEXT_GRAY_1.lighter(0.1),
            ),
            (
                tokens::SWITCH_SLIDE_BORDER_CHECKED_DISABLED,
                light_palette::TEXT_GRAY_2.with_alpha(0.3),
            ),
            (tokens::COLOR_PLANE_BG, light_palette::GRAY_1),
            // Menus
            (tokens::MENU_BG, light_palette::GRAY_1),
            (tokens::MENU_BORDER, light_palette::BORDER_GRAY),
            (
                tokens::MENUITEM_BG_HOVER,
                light_palette::GRAY_1.lighter(0.05),
            ),
            (
                tokens::MENUITEM_BG_PRESSED,
                light_palette::GRAY_1.lighter(0.1),
            ),
            (
                tokens::MENUITEM_BG_FOCUSED,
                light_palette::GRAY_1.lighter(0.1),
            ),
            (tokens::MENUITEM_TEXT, light_palette::TEXT_GRAY_1),
            (
                tokens::MENUITEM_TEXT_DISABLED,
                light_palette::TEXT_GRAY_1.with_alpha(0.65),
            ),
            // Text Input
            (
                tokens::TEXT_INPUT_BG,
                light_palette::DARK_GRAY_MIX.with_alpha(0.16),
            ),
            (
                tokens::TEXT_INPUT_LABEL_BG,
                light_palette::DARK_GRAY_MIX.with_alpha(0.28),
            ),
            (tokens::TEXT_INPUT_TEXT, light_palette::TEXT_GRAY_1),
            (
                tokens::TEXT_INPUT_TEXT_DISABLED,
                light_palette::TEXT_GRAY_1.with_alpha(0.65),
            ),
            (
                tokens::TEXT_INPUT_CURSOR,
                light_palette::ACCENT.lighter(0.2),
            ),
            (tokens::TEXT_INPUT_SELECTION, light_palette::ACCENT),
            (
                tokens::TEXT_INPUT_SELECTION_UNFOCUSED,
                light_palette::TRANSPARENT,
            ),
            (tokens::TEXT_INPUT_X_AXIS, light_palette::X_AXIS),
            (tokens::TEXT_INPUT_Y_AXIS, light_palette::Y_AXIS),
            (tokens::TEXT_INPUT_Z_AXIS, light_palette::Z_AXIS),
            // Pane
            (tokens::PANE_HEADER_BG, light_palette::GRAY_0),
            (tokens::PANE_HEADER_BORDER, light_palette::BORDER_GRAY),
            (tokens::PANE_HEADER_TEXT, light_palette::TEXT_GRAY_1),
            (tokens::PANE_HEADER_DIVIDER, light_palette::BORDER_GRAY),
            (tokens::PANE_BODY_BG, light_palette::GRAY_1),
            // Subpane
            (tokens::SUBPANE_HEADER_BG, light_palette::GRAY_2),
            (tokens::SUBPANE_HEADER_BORDER, light_palette::GRAY_3),
            (tokens::SUBPANE_HEADER_TEXT, light_palette::TEXT_GRAY_1),
            (tokens::SUBPANE_BODY_BG, light_palette::GRAY_1),
            (tokens::SUBPANE_BODY_BORDER, light_palette::GRAY_2),
            // Group
            (tokens::GROUP_HEADER_BG, light_palette::GRAY_2),
            (tokens::GROUP_HEADER_BORDER, light_palette::GRAY_3),
            (tokens::GROUP_HEADER_TEXT, light_palette::TEXT_GRAY_1),
            (tokens::GROUP_BODY_BG, light_palette::GRAY_2),
            (tokens::GROUP_BODY_BORDER, light_palette::GRAY_3),
            // Listview
            (tokens::LISTROW_BG, Color::NONE),
            (
                tokens::LISTROW_BG_HOVER,
                light_palette::GRAY_3.with_alpha(0.5),
            ),
            (tokens::LISTROW_BG_SELECTED, light_palette::GRAY_3),
            (tokens::LISTROW_TEXT, light_palette::TEXT_GRAY_1),
            (
                tokens::LISTROW_TEXT_DISABLED,
                light_palette::TEXT_GRAY_1.with_alpha(0.65),
            ),
            (tokens::DIALOG_BG, light_palette::GRAY_1),
            (tokens::DIALOG_BORDER, light_palette::BORDER_GRAY),
            (tokens::DIALOG_HEADER_BG, light_palette::GRAY_0),
            (tokens::DIALOG_TEXT, light_palette::TEXT_GRAY_1),
        ]),
    }
}

mod light_palette {

    use super::*;

    /// <div style="background-color: transparent; width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const TRANSPARENT: Color = Color::oklcha(0.0, 0.0, 0.0, 0.0);
    /// <div style="background-color: #ECEFF4; width: 10px; padding: 10px; border: 1px solid;"></div> - window background
    pub const GRAY_0: Color = Color::oklcha(0.9514, 0.008, 266.0, 1.0);
    /// <div style="background-color: #E2E7F3; width: 10px; padding: 10px; border: 1px solid;"></div> - pane background
    pub const GRAY_1: Color = Color::oklcha(0.9284, 0.0161, 266.0, 1.0);
    /// <div style="background-color: #BBBFCA; width: 10px; padding: 10px; border: 1px solid;"></div> - item background
    pub const GRAY_2: Color = Color::oklcha(0.8055, 0.0158, 266.0, 1.0);
    /// <div style="background-color: #A2A6B0; width: 10px; padding: 10px; border: 1px solid;"></div> - item background (active)
    pub const GRAY_3: Color = Color::oklcha(0.7254, 0.0157, 266.0, 1.0);
    /// <div style="background-color: #C0C0C1; width: 10px; padding: 10px; border: 1px solid;"></div> - border
    pub const BORDER_GRAY: Color = Color::oklcha(0.8083, 0.0017, 266.0, 1.0);
    /// <div style="background-color: #222223; width: 10px; padding: 10px; border: 1px solid;"></div> - bright label text
    pub const TEXT_GRAY_1: Color = Color::oklcha(0.252, 0.0014, 266.0, 1.0);
    /// <div style="background-color: #666768; width: 10px; padding: 10px; border: 1px solid;"></div> - dim label text
    pub const TEXT_GRAY_2: Color = Color::oklcha(0.512, 0.003, 266.0, 1.0);
    /// <div style="background-color: #434750; width: 10px; padding: 10px; border: 1px solid;"></div> - mix with background to produce text fill
    pub const DARK_GRAY_MIX: Color = Color::oklcha(0.3972, 0.0171, 266.0, 1.0);
    /// <div style="background-color: #FFFFFF; width: 10px; padding: 10px; border: 1px solid;"></div> - button label text
    pub const WHITE: Color = Color::oklcha(1.0, 0.000000059604645, 90.0, 1.0);
    /// <div style="background-color: #2774CF; width: 10px; padding: 10px; border: 1px solid;"></div> - call-to-action and selection color
    pub const ACCENT: Color = Color::oklcha(0.56, 0.1594, 255.4, 1.0);
    /// <div style="background-color: #AB4051; width: 10px; padding: 10px; border: 1px solid;"></div> - for X-axis inputs and drag handles
    pub const X_AXIS: Color = Color::oklcha(0.5232, 0.1404, 13.84, 1.0);
    /// <div style="background-color: #5D8D0A; width: 10px; padding: 10px; border: 1px solid;"></div> - for Y-axis inputs and drag handles
    pub const Y_AXIS: Color = Color::oklcha(0.5866, 0.1543, 129.84, 1.0);
    /// <div style="background-color: #2160A3; width: 10px; padding: 10px; border: 1px solid;"></div> - for Z-axis inputs and drag handles
    pub const Z_AXIS: Color = Color::oklcha(0.4847, 0.1249, 253.08, 1.0);
}

#[cfg(test)]
mod tests {
    use bevy_platform::collections::HashSet;

    use crate::dark_theme::create_dark_theme;
    use crate::light_theme::create_light_theme;

    #[test]
    fn test_same_keys_dark_and_light() {
        let dark_theme = create_dark_theme();
        let light_theme = create_light_theme();
        assert!(
            dark_theme.color.keys().collect::<HashSet<_>>()
                == light_theme.color.keys().collect::<HashSet<_>>(),
            "Keys do not match between light and dark themes"
        );
    }
}

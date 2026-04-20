//! The standard `bevy_feathers` dark theme.
use crate::{palette, tokens};
use bevy_color::{Alpha, Color, Luminance};
use bevy_platform::collections::HashMap;

use crate::theme::ThemeProps;

/// Create a [`ThemeProps`] object and populate it with the colors for the default dark theme.
pub fn create_dark_theme() -> ThemeProps {
    ThemeProps {
        color: HashMap::from([
            (tokens::WINDOW_BG, palette::GRAY_0),
            (tokens::FOCUS_RING, palette::ACCENT.with_alpha(0.5)),
            (tokens::TEXT_MAIN, palette::LIGHT_GRAY_1),
            (tokens::TEXT_DIM, palette::LIGHT_GRAY_2),
            // Button (normal)
            (tokens::BUTTON_BG, palette::GRAY_3),
            (tokens::BUTTON_BG_HOVER, palette::GRAY_3.lighter(0.05)),
            (tokens::BUTTON_BG_PRESSED, palette::GRAY_3.lighter(0.1)),
            (tokens::BUTTON_BG_DISABLED, palette::GRAY_2),
            // Button (primary)
            (tokens::BUTTON_PRIMARY_BG, palette::ACCENT),
            (
                tokens::BUTTON_PRIMARY_BG_HOVER,
                palette::ACCENT.lighter(0.05),
            ),
            (
                tokens::BUTTON_PRIMARY_BG_PRESSED,
                palette::ACCENT.lighter(0.1),
            ),
            (tokens::BUTTON_PRIMARY_BG_DISABLED, palette::GRAY_2),
            // Button (plain)
            (tokens::BUTTON_PLAIN_BG, Color::NONE),
            (tokens::BUTTON_PLAIN_BG_HOVER, palette::GRAY_2),
            (tokens::BUTTON_PLAIN_BG_PRESSED, palette::GRAY_3),
            (tokens::BUTTON_PLAIN_BG_DISABLED, Color::NONE),
            // Button text
            (tokens::BUTTON_TEXT, palette::WHITE),
            (tokens::BUTTON_TEXT_DISABLED, palette::WHITE.with_alpha(0.5)),
            (tokens::BUTTON_PRIMARY_TEXT, palette::WHITE),
            (
                tokens::BUTTON_PRIMARY_TEXT_DISABLED,
                palette::WHITE.with_alpha(0.5),
            ),
            // Slider
            (tokens::SLIDER_BG, palette::GRAY_1),
            (tokens::SLIDER_BG_HOVER, palette::GRAY_1.lighter(0.05)),
            (tokens::SLIDER_BG_PRESSED, palette::GRAY_1.lighter(0.1)),
            (tokens::SLIDER_BG_DISABLED, palette::GRAY_1),
            (tokens::SLIDER_BAR, palette::ACCENT),
            (tokens::SLIDER_BAR_HOVER, palette::ACCENT.lighter(0.05)),
            (tokens::SLIDER_BAR_PRESSED, palette::ACCENT.lighter(0.1)),
            (tokens::SLIDER_BAR_DISABLED, palette::GRAY_2),
            (tokens::SLIDER_TEXT, palette::WHITE),
            (tokens::SLIDER_TEXT_DISABLED, palette::WHITE.with_alpha(0.5)),
            // Scrollbar
            (tokens::SCROLLBAR_BG, palette::GRAY_2),
            (tokens::SCROLLBAR_THUMB, palette::ACCENT),
            (tokens::SCROLLBAR_THUMB_HOVER, palette::ACCENT.lighter(0.1)),
            // Checkbox
            (tokens::CHECKBOX_BG, palette::GRAY_3),
            (tokens::CHECKBOX_BG_HOVER, palette::GRAY_3),
            (tokens::CHECKBOX_BG_PRESSED, palette::GRAY_3),
            (
                tokens::CHECKBOX_BG_DISABLED,
                palette::GRAY_1.with_alpha(0.5),
            ),
            (tokens::CHECKBOX_BG_CHECKED, palette::ACCENT),
            (
                tokens::CHECKBOX_BG_CHECKED_HOVER,
                palette::ACCENT.lighter(0.05),
            ),
            (
                tokens::CHECKBOX_BG_CHECKED_PRESSED,
                palette::ACCENT.lighter(0.1),
            ),
            (
                tokens::CHECKBOX_BG_CHECKED_DISABLED,
                palette::GRAY_1.with_alpha(0.5),
            ),
            (tokens::CHECKBOX_BORDER, palette::GRAY_3),
            (tokens::CHECKBOX_BORDER_HOVER, palette::GRAY_3.lighter(0.05)),
            (
                tokens::CHECKBOX_BORDER_PRESSED,
                palette::GRAY_3.lighter(0.1),
            ),
            (
                tokens::CHECKBOX_BORDER_DISABLED,
                palette::GRAY_3.with_alpha(0.5),
            ),
            (tokens::CHECKBOX_BORDER_CHECKED, palette::ACCENT),
            (
                tokens::CHECKBOX_BORDER_CHECKED_HOVER,
                palette::ACCENT.lighter(0.05),
            ),
            (
                tokens::CHECKBOX_BORDER_CHECKED_PRESSED,
                palette::ACCENT.lighter(0.1),
            ),
            (
                tokens::CHECKBOX_BORDER_CHECKED_DISABLED,
                palette::GRAY_3.with_alpha(0.5),
            ),
            (tokens::CHECKBOX_MARK, palette::WHITE),
            (tokens::CHECKBOX_MARK_DISABLED, palette::LIGHT_GRAY_2),
            (tokens::CHECKBOX_TEXT, palette::LIGHT_GRAY_1),
            (
                tokens::CHECKBOX_TEXT_DISABLED,
                palette::LIGHT_GRAY_1.with_alpha(0.5),
            ),
            // Radio
            (tokens::RADIO_BORDER, palette::GRAY_3),
            (tokens::RADIO_BORDER_HOVER, palette::GRAY_3.lighter(0.05)),
            (tokens::RADIO_BORDER_PRESSED, palette::GRAY_3.lighter(0.1)),
            (
                tokens::RADIO_BORDER_DISABLED,
                palette::GRAY_3.with_alpha(0.5),
            ),
            (tokens::RADIO_BORDER_CHECKED, palette::ACCENT),
            (
                tokens::RADIO_BORDER_CHECKED_HOVER,
                palette::ACCENT.lighter(0.05),
            ),
            (
                tokens::RADIO_BORDER_CHECKED_PRESSED,
                palette::ACCENT.lighter(0.1),
            ),
            (
                tokens::RADIO_BORDER_CHECKED_DISABLED,
                palette::GRAY_3.with_alpha(0.5),
            ),
            (tokens::RADIO_MARK, palette::ACCENT),
            (tokens::RADIO_MARK_HOVER, palette::ACCENT.lighter(0.05)),
            (tokens::RADIO_MARK_PRESSED, palette::ACCENT.lighter(0.1)),
            (tokens::RADIO_MARK_DISABLED, palette::GRAY_3.with_alpha(0.5)),
            (tokens::RADIO_TEXT, palette::LIGHT_GRAY_1),
            (
                tokens::RADIO_TEXT_DISABLED,
                palette::LIGHT_GRAY_1.with_alpha(0.5),
            ),
            // Toggle Switch
            (tokens::SWITCH_BG, palette::GRAY_3),
            (tokens::SWITCH_BG_HOVER, palette::GRAY_3.lighter(0.05)),
            (tokens::SWITCH_BG_PRESSED, palette::GRAY_3.lighter(0.1)),
            (tokens::SWITCH_BG_DISABLED, palette::GRAY_1.with_alpha(0.5)),
            (tokens::SWITCH_BG_CHECKED, palette::ACCENT),
            (
                tokens::SWITCH_BG_CHECKED_HOVER,
                palette::ACCENT.lighter(0.05),
            ),
            (
                tokens::SWITCH_BG_CHECKED_PRESSED,
                palette::ACCENT.lighter(0.1),
            ),
            (
                tokens::SWITCH_BG_CHECKED_DISABLED,
                palette::GRAY_1.with_alpha(0.5),
            ),
            (tokens::SWITCH_BORDER, palette::GRAY_3),
            (tokens::SWITCH_BORDER_HOVER, palette::GRAY_3.lighter(0.05)),
            (tokens::SWITCH_BORDER_PRESSED, palette::GRAY_3.lighter(0.1)),
            (
                tokens::SWITCH_BORDER_DISABLED,
                palette::GRAY_3.with_alpha(0.5),
            ),
            (tokens::SWITCH_BORDER_CHECKED, palette::ACCENT),
            (
                tokens::SWITCH_BORDER_CHECKED_HOVER,
                palette::ACCENT.lighter(0.05),
            ),
            (
                tokens::SWITCH_BORDER_CHECKED_PRESSED,
                palette::ACCENT.lighter(0.1),
            ),
            (
                tokens::SWITCH_BORDER_CHECKED_DISABLED,
                palette::GRAY_3.with_alpha(0.5),
            ),
            (tokens::SWITCH_SLIDE_BG, palette::LIGHT_GRAY_1.lighter(0.1)),
            (
                tokens::SWITCH_SLIDE_BG_HOVER,
                palette::LIGHT_GRAY_1.lighter(0.1),
            ),
            (
                tokens::SWITCH_SLIDE_BG_PRESSED,
                palette::LIGHT_GRAY_1.lighter(0.1),
            ),
            (
                tokens::SWITCH_SLIDE_BG_DISABLED,
                palette::GRAY_1.with_alpha(0.5),
            ),
            (
                tokens::SWITCH_SLIDE_BG_CHECKED,
                palette::LIGHT_GRAY_1.lighter(0.1),
            ),
            (
                tokens::SWITCH_SLIDE_BG_CHECKED_HOVER,
                palette::LIGHT_GRAY_1.lighter(0.1),
            ),
            (
                tokens::SWITCH_SLIDE_BG_CHECKED_PRESSED,
                palette::LIGHT_GRAY_1.lighter(0.1),
            ),
            (
                tokens::SWITCH_SLIDE_BG_CHECKED_DISABLED,
                palette::LIGHT_GRAY_2.with_alpha(0.3),
            ),
            (
                tokens::SWITCH_SLIDE_BORDER,
                palette::LIGHT_GRAY_1.lighter(0.1),
            ),
            (
                tokens::SWITCH_SLIDE_BORDER_HOVER,
                palette::LIGHT_GRAY_1.lighter(0.1),
            ),
            (
                tokens::SWITCH_SLIDE_BORDER_PRESSED,
                palette::LIGHT_GRAY_1.lighter(0.1),
            ),
            (
                tokens::SWITCH_SLIDE_BORDER_DISABLED,
                palette::GRAY_2.with_alpha(0.5),
            ),
            (
                tokens::SWITCH_SLIDE_BORDER_CHECKED,
                palette::LIGHT_GRAY_1.lighter(0.1),
            ),
            (
                tokens::SWITCH_SLIDE_BORDER_CHECKED_HOVER,
                palette::LIGHT_GRAY_1.lighter(0.1),
            ),
            (
                tokens::SWITCH_SLIDE_BORDER_CHECKED_PRESSED,
                palette::LIGHT_GRAY_1.lighter(0.1),
            ),
            (
                tokens::SWITCH_SLIDE_BORDER_CHECKED_DISABLED,
                palette::LIGHT_GRAY_2.with_alpha(0.3),
            ),
            (tokens::COLOR_PLANE_BG, palette::GRAY_1),
            // Menus
            (tokens::MENU_BG, palette::GRAY_1),
            (tokens::MENU_BORDER, palette::WARM_GRAY_1),
            (tokens::MENUITEM_BG_HOVER, palette::GRAY_1.lighter(0.05)),
            (tokens::MENUITEM_BG_PRESSED, palette::GRAY_1.lighter(0.1)),
            (tokens::MENUITEM_BG_FOCUSED, palette::GRAY_1.lighter(0.1)),
            (tokens::MENUITEM_TEXT, palette::WHITE),
            (
                tokens::MENUITEM_TEXT_DISABLED,
                palette::WHITE.with_alpha(0.5),
            ),
            // Text Input
            (tokens::TEXT_INPUT_BG, palette::GRAY_1),
            (tokens::TEXT_INPUT_TEXT, palette::WHITE),
            (
                tokens::TEXT_INPUT_TEXT_DISABLED,
                palette::WHITE.with_alpha(0.5),
            ),
            (tokens::TEXT_INPUT_CURSOR, palette::ACCENT.lighter(0.2)),
            (tokens::TEXT_INPUT_SELECTION, palette::ACCENT),
            // Pane
            (tokens::PANE_HEADER_BG, palette::GRAY_0),
            (tokens::PANE_HEADER_BORDER, palette::WARM_GRAY_1),
            (tokens::PANE_HEADER_TEXT, palette::LIGHT_GRAY_1),
            (tokens::PANE_HEADER_DIVIDER, palette::WARM_GRAY_1),
            (tokens::PANE_BODY_BG, palette::GRAY_1),
            // Subpane
            (tokens::SUBPANE_HEADER_BG, palette::GRAY_2),
            (tokens::SUBPANE_HEADER_BORDER, palette::GRAY_3),
            (tokens::SUBPANE_HEADER_TEXT, palette::LIGHT_GRAY_1),
            (tokens::SUBPANE_BODY_BG, palette::GRAY_1),
            (tokens::SUBPANE_BODY_BORDER, palette::GRAY_2),
            // Group
            (tokens::GROUP_HEADER_BG, palette::GRAY_2),
            (tokens::GROUP_HEADER_BORDER, palette::GRAY_3),
            (tokens::GROUP_HEADER_TEXT, palette::LIGHT_GRAY_1),
            (tokens::GROUP_BODY_BG, palette::GRAY_2),
            (tokens::GROUP_BODY_BORDER, palette::GRAY_3),
        ]),
    }
}

//! The standard `bevy_feathers` dark theme.
use crate::{palette, tokens};
use bevy_color::{Alpha, Color, Luminance};
use bevy_platform::collections::HashMap;

use crate::theme::ThemeProps;

/// Create a [`ThemeProps`] object and populate it with the colors for the default dark theme.
pub fn create_dark_theme() -> ThemeProps {
    ThemeProps {
        color: HashMap::from([
            (tokens::WINDOW_BG.into(), palette::GRAY_0),
            // Button (normal)
            (tokens::BUTTON_BG.into(), palette::GRAY_3),
            (
                tokens::BUTTON_BG_HOVER.into(),
                palette::GRAY_3.lighter(0.05),
            ),
            (
                tokens::BUTTON_BG_PRESSED.into(),
                palette::GRAY_3.lighter(0.1),
            ),
            (tokens::BUTTON_BG_DISABLED.into(), palette::GRAY_2),
            // Button (primary)
            (tokens::BUTTON_PRIMARY_BG.into(), palette::ACCENT),
            (
                tokens::BUTTON_PRIMARY_BG_HOVER.into(),
                palette::ACCENT.lighter(0.05),
            ),
            (
                tokens::BUTTON_PRIMARY_BG_PRESSED.into(),
                palette::ACCENT.lighter(0.1),
            ),
            (tokens::BUTTON_PRIMARY_BG_DISABLED.into(), palette::GRAY_2),
            // Button (selected)
            (tokens::BUTTON_SELECTED_BG.into(), palette::GRAY_3),
            (
                tokens::BUTTON_SELECTED_BG_HOVER.into(),
                palette::GRAY_3.lighter(0.05),
            ),
            (
                tokens::BUTTON_SELECTED_BG_PRESSED.into(),
                palette::GRAY_3.lighter(0.1),
            ),
            (tokens::BUTTON_SELECTED_BG_DISABLED.into(), palette::GRAY_2),
            // Button (plain)
            (tokens::BUTTON_PLAIN_BG.into(), Color::NONE),
            (tokens::BUTTON_PLAIN_BG_HOVER.into(), palette::GRAY_2),
            (tokens::BUTTON_PLAIN_BG_PRESSED.into(), palette::GRAY_3),
            (tokens::BUTTON_PLAIN_BG_DISABLED.into(), Color::NONE),
            // Button text
            (tokens::BUTTON_TEXT.into(), palette::WHITE),
            (
                tokens::BUTTON_TEXT_DISABLED.into(),
                palette::WHITE.with_alpha(0.5),
            ),
            (tokens::BUTTON_PRIMARY_TEXT.into(), palette::WHITE),
            (
                tokens::BUTTON_PRIMARY_TEXT_DISABLED.into(),
                palette::WHITE.with_alpha(0.5),
            ),
            // Slider
            (tokens::SLIDER_BG.into(), palette::GRAY_1),
            (tokens::SLIDER_BAR.into(), palette::ACCENT),
            (tokens::SLIDER_BAR_DISABLED.into(), palette::GRAY_2),
            (tokens::SLIDER_TEXT.into(), palette::WHITE),
            (
                tokens::SLIDER_TEXT_DISABLED.into(),
                palette::WHITE.with_alpha(0.5),
            ),
            // Checkbox
            (tokens::CHECKBOX_BG.into(), palette::GRAY_3),
            (tokens::CHECKBOX_BG_CHECKED.into(), palette::ACCENT),
            (
                tokens::CHECKBOX_BG_DISABLED.into(),
                palette::GRAY_1.with_alpha(0.5),
            ),
            (
                tokens::CHECKBOX_BG_CHECKED_DISABLED.into(),
                palette::GRAY_3.with_alpha(0.5),
            ),
            (tokens::CHECKBOX_BORDER.into(), palette::GRAY_3),
            (
                tokens::CHECKBOX_BORDER_HOVER.into(),
                palette::GRAY_3.lighter(0.1),
            ),
            (
                tokens::CHECKBOX_BORDER_DISABLED.into(),
                palette::GRAY_3.with_alpha(0.5),
            ),
            (tokens::CHECKBOX_MARK.into(), palette::WHITE),
            (tokens::CHECKBOX_MARK_DISABLED.into(), palette::LIGHT_GRAY_2),
            (tokens::CHECKBOX_TEXT.into(), palette::LIGHT_GRAY_1),
            (
                tokens::CHECKBOX_TEXT_DISABLED.into(),
                palette::LIGHT_GRAY_1.with_alpha(0.5),
            ),
            // Radio
            (tokens::RADIO_BORDER.into(), palette::GRAY_3),
            (
                tokens::RADIO_BORDER_HOVER.into(),
                palette::GRAY_3.lighter(0.1),
            ),
            (
                tokens::RADIO_BORDER_DISABLED.into(),
                palette::GRAY_3.with_alpha(0.5),
            ),
            (tokens::RADIO_MARK.into(), palette::ACCENT),
            (
                tokens::RADIO_MARK_DISABLED.into(),
                palette::ACCENT.with_alpha(0.5),
            ),
            (tokens::RADIO_TEXT.into(), palette::LIGHT_GRAY_1),
            (
                tokens::RADIO_TEXT_DISABLED.into(),
                palette::LIGHT_GRAY_1.with_alpha(0.5),
            ),
            // Toggle Switch
            (tokens::SWITCH_BG.into(), palette::GRAY_3),
            (tokens::SWITCH_BG_CHECKED.into(), palette::ACCENT),
            (
                tokens::SWITCH_BG_DISABLED.into(),
                palette::GRAY_1.with_alpha(0.5),
            ),
            (
                tokens::SWITCH_BG_CHECKED_DISABLED.into(),
                palette::GRAY_3.with_alpha(0.5),
            ),
            (tokens::SWITCH_BORDER.into(), palette::GRAY_3),
            (
                tokens::SWITCH_BORDER_HOVER.into(),
                palette::GRAY_3.lighter(0.1),
            ),
            (
                tokens::SWITCH_BORDER_DISABLED.into(),
                palette::GRAY_3.with_alpha(0.5),
            ),
            (tokens::SWITCH_SLIDE.into(), palette::LIGHT_GRAY_2),
            (
                tokens::SWITCH_SLIDE_DISABLED.into(),
                palette::LIGHT_GRAY_2.with_alpha(0.3),
            ),
            // Pane
            (tokens::PANE_HEADER_BG.into(), palette::GRAY_0),
            (tokens::PANE_HEADER_BORDER.into(), palette::WARM_GRAY_1),
            (tokens::PANE_HEADER_TEXT.into(), palette::LIGHT_GRAY_1),
            (tokens::PANE_HEADER_DIVIDER.into(), palette::WARM_GRAY_1),
            // Subpane
            (tokens::SUBPANE_HEADER_BG.into(), palette::GRAY_2),
            (tokens::SUBPANE_HEADER_BORDER.into(), palette::GRAY_3),
            (tokens::SUBPANE_HEADER_TEXT.into(), palette::LIGHT_GRAY_1),
            (tokens::SUBPANE_BODY_BG.into(), palette::GRAY_1),
            (tokens::SUBPANE_BODY_BORDER.into(), palette::GRAY_2),
        ]),
    }
}

//! The standard `bevy_feathers` dark theme.
use crate::{palette, tokens};
use bevy_color::{Alpha, Luminance};
use bevy_platform::collections::HashMap;

use crate::theme::ThemeProps;

/// Create a [`ThemeProps`] object and populate it with the colors for the default dark theme.
pub fn create_dark_theme() -> ThemeProps {
    ThemeProps {
        color: HashMap::from([
            (tokens::WINDOW_BG, palette::GRAY_0),
            // Button
            (tokens::BUTTON_BG, palette::GRAY_3),
            (tokens::BUTTON_BG_HOVER, palette::GRAY_3.lighter(0.05)),
            (tokens::BUTTON_BG_PRESSED, palette::GRAY_3.lighter(0.1)),
            (tokens::BUTTON_BG_DISABLED, palette::GRAY_2),
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
            (tokens::BUTTON_TEXT, palette::WHITE),
            (tokens::BUTTON_TEXT_DISABLED, palette::WHITE.with_alpha(0.5)),
            (tokens::BUTTON_PRIMARY_TEXT, palette::WHITE),
            (
                tokens::BUTTON_PRIMARY_TEXT_DISABLED,
                palette::WHITE.with_alpha(0.5),
            ),
            // Slider
            (tokens::SLIDER_BG, palette::GRAY_1),
            (tokens::SLIDER_BAR, palette::ACCENT),
            (tokens::SLIDER_BAR_DISABLED, palette::GRAY_2),
            (tokens::SLIDER_TEXT, palette::WHITE),
            (tokens::SLIDER_TEXT_DISABLED, palette::WHITE.with_alpha(0.5)),
            // Checkbox
            (tokens::CHECKBOX_BG, palette::GRAY_3),
            (tokens::CHECKBOX_BG_CHECKED, palette::ACCENT),
            (
                tokens::CHECKBOX_BG_DISABLED,
                palette::GRAY_1.with_alpha(0.5),
            ),
            (
                tokens::CHECKBOX_BG_CHECKED_DISABLED,
                palette::GRAY_3.with_alpha(0.5),
            ),
            (tokens::CHECKBOX_BORDER, palette::GRAY_3),
            (tokens::CHECKBOX_BORDER_HOVER, palette::GRAY_3.lighter(0.1)),
            (
                tokens::CHECKBOX_BORDER_DISABLED,
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
            (tokens::RADIO_BORDER_HOVER, palette::GRAY_3.lighter(0.1)),
            (
                tokens::RADIO_BORDER_DISABLED,
                palette::GRAY_3.with_alpha(0.5),
            ),
            (tokens::RADIO_MARK, palette::ACCENT),
            (tokens::RADIO_MARK_DISABLED, palette::ACCENT.with_alpha(0.5)),
            (tokens::RADIO_TEXT, palette::LIGHT_GRAY_1),
            (
                tokens::RADIO_TEXT_DISABLED,
                palette::LIGHT_GRAY_1.with_alpha(0.5),
            ),
            // Toggle Switch
            (tokens::SWITCH_BG, palette::GRAY_3),
            (tokens::SWITCH_BG_CHECKED, palette::ACCENT),
            (tokens::SWITCH_BG_DISABLED, palette::GRAY_1.with_alpha(0.5)),
            (
                tokens::SWITCH_BG_CHECKED_DISABLED,
                palette::GRAY_3.with_alpha(0.5),
            ),
            (tokens::SWITCH_BORDER, palette::GRAY_3),
            (tokens::SWITCH_BORDER_HOVER, palette::GRAY_3.lighter(0.1)),
            (
                tokens::SWITCH_BORDER_DISABLED,
                palette::GRAY_3.with_alpha(0.5),
            ),
            (tokens::SWITCH_SLIDE, palette::LIGHT_GRAY_2),
            (
                tokens::SWITCH_SLIDE_DISABLED,
                palette::LIGHT_GRAY_2.with_alpha(0.3),
            ),
        ]),
    }
}

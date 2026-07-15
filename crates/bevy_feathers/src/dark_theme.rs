//! The standard `bevy_feathers` dark theme.
use crate::{
    palette,
    theme::SurfaceLevel,
    tokens::{self, semantic},
};
use bevy_color::{Alpha, Color, Luminance};
use bevy_platform::collections::HashMap;

use crate::theme::ThemeProps;

/// Create a [`ThemeProps`] object and populate it with the colors for the default dark theme.
pub fn create_dark_theme() -> ThemeProps {
    ThemeProps {
        semantic_base: HashMap::from([
            (semantic::SURFACE_WINDOW, palette::GRAY_0),
            (semantic::SURFACE_PANE_BODY, palette::GRAY_1),
            (semantic::SURFACE_PANE_HEADER, palette::GRAY_0),
            (semantic::SURFACE_SUBPANE_BODY, palette::GRAY_1),
            (semantic::SURFACE_SUBPANE_HEADER, palette::GRAY_2),
            (semantic::SURFACE_GROUP, palette::GRAY_2),
            (semantic::SURFACE_DIALOG, palette::GRAY_1),
            (semantic::SURFACE_MENU, palette::GRAY_1),
            (semantic::BORDER_DEFAULT, palette::WARM_GRAY_1),
            (semantic::TEXT_DEFAULT, palette::LIGHT_GRAY_1),
            (semantic::TEXT_DIM, palette::LIGHT_GRAY_2),
            (
                semantic::TEXT_DISABLED,
                palette::LIGHT_GRAY_1.with_alpha(0.4),
            ),
            (semantic::TEXT_ON_ACCENT, palette::WHITE),
            (semantic::FILL_ACCENT_DEFAULT, palette::ACCENT),
            (semantic::FILL_ACCENT_HOVER, palette::ACCENT.lighter(0.05)),
            (semantic::FILL_ACCENT_PRESSED, palette::ACCENT.lighter(0.1)),
            (
                semantic::FILL_ACCENT_DISABLED,
                palette::GRAY_3.with_alpha(0.7),
            ),
            (semantic::FILL_ACCENT_DISABLED, Color::NONE),
            (semantic::FILL_SOLID_DEFAULT, palette::GRAY_3),
            (semantic::FILL_SOLID_HOVER, palette::GRAY_3.lighter(0.05)),
            (semantic::FILL_SOLID_PRESSED, palette::GRAY_3.lighter(0.1)),
            (
                semantic::FILL_SOLID_DISABLED,
                palette::GRAY_2.with_alpha(0.7),
            ),
            (semantic::FILL_ITEM_DEFAULT, Color::NONE),
            (semantic::FILL_ITEM_HOVER, palette::GRAY_1.lighter(0.05)),
            (semantic::FILL_ITEM_PRESSED, palette::GRAY_1.lighter(0.1)),
            (semantic::FILL_FIELD_DEFAULT, palette::GRAY_2),
            (semantic::FILL_FIELD_HOVER, palette::GRAY_2.lighter(0.05)),
            (semantic::FILL_FIELD_PRESSED, palette::GRAY_2.lighter(0.1)),
            (semantic::FILL_NONE, Color::NONE),
            (semantic::AXIS_X, palette::X_AXIS),
            (semantic::AXIS_Y, palette::Y_AXIS),
            (semantic::AXIS_Z, palette::Z_AXIS),
            (semantic::FOCUS_RING, palette::ACCENT.with_alpha(0.5)),
        ]),
        semantic_overrides: HashMap::from([
            (
                // Only group context is light enough to need overrides. Most controls are tuned to look
                // good against SURFACE_PANE_BODY; although SURFACE_WINDOW is considerably darker,
                // the additional contrast is insufficient to need correction.
                SurfaceLevel::Highest,
                HashMap::from([
                    (semantic::FILL_SOLID_DEFAULT, palette::GRAY_3.lighter(0.05)), // We don't have a GRAY_4
                    (semantic::FILL_SOLID_HOVER, palette::GRAY_3.lighter(0.1)),
                    (semantic::FILL_SOLID_PRESSED, palette::GRAY_3.lighter(0.15)),
                    (
                        semantic::FILL_SOLID_DISABLED,
                        palette::GRAY_3.lighter(0.05).with_alpha(0.7),
                    ),
                    (semantic::FILL_FIELD_DEFAULT, palette::GRAY_3),
                    (semantic::FILL_FIELD_HOVER, palette::GRAY_3.lighter(0.02)), // Field hover is subtle
                    (semantic::FILL_FIELD_PRESSED, palette::GRAY_3.lighter(0.05)),
                    (semantic::FILL_ITEM_HOVER, palette::GRAY_3.lighter(0.05)),
                    (semantic::FILL_ITEM_PRESSED, palette::GRAY_3.lighter(0.1)),
                ]),
            ),
            (
                SurfaceLevel::Higher,
                HashMap::from([
                    (semantic::FILL_FIELD_DEFAULT, palette::GRAY_3),
                    (semantic::FILL_ITEM_HOVER, palette::GRAY_2.lighter(0.02)), // Field hover is subtle
                    (semantic::FILL_ITEM_PRESSED, palette::GRAY_2.lighter(0.05)),
                ]),
            ),
            (
                SurfaceLevel::Floating,
                HashMap::from([
                    (semantic::FILL_SOLID_DEFAULT, palette::GRAY_3),
                    (semantic::FILL_FIELD_DEFAULT, palette::GRAY_3),
                    (semantic::FILL_ITEM_HOVER, palette::GRAY_2.lighter(0.02)), // Field hover is subtle
                    (semantic::FILL_ITEM_PRESSED, palette::GRAY_2.lighter(0.05)),
                ]),
            ),
        ]),
        token_assignments: HashMap::from([
            (tokens::WINDOW_BG, semantic::SURFACE_WINDOW),
            (tokens::FOCUS_RING, semantic::FOCUS_RING),
            (tokens::TEXT_MAIN, semantic::TEXT_DEFAULT),
            (tokens::TEXT_DIM, semantic::TEXT_DIM),
            // Button (normal)
            (tokens::BUTTON_BG, semantic::FILL_SOLID_DEFAULT),
            (tokens::BUTTON_BG_HOVER, semantic::FILL_SOLID_HOVER),
            (tokens::BUTTON_BG_PRESSED, semantic::FILL_SOLID_PRESSED),
            (tokens::BUTTON_BG_DISABLED, semantic::FILL_SOLID_DISABLED),
            // Button (primary)
            (tokens::BUTTON_PRIMARY_BG, semantic::FILL_ACCENT_DEFAULT),
            (tokens::BUTTON_PRIMARY_BG_HOVER, semantic::FILL_ACCENT_HOVER),
            (
                tokens::BUTTON_PRIMARY_BG_PRESSED,
                semantic::FILL_ACCENT_PRESSED,
            ),
            (
                tokens::BUTTON_PRIMARY_BG_DISABLED,
                semantic::FILL_ACCENT_DISABLED,
            ),
            // Button (plain)
            (tokens::BUTTON_PLAIN_BG, semantic::FILL_NONE),
            (tokens::BUTTON_PLAIN_BG_HOVER, semantic::FILL_ITEM_HOVER),
            (tokens::BUTTON_PLAIN_BG_PRESSED, semantic::FILL_ITEM_PRESSED),
            (tokens::BUTTON_PLAIN_BG_DISABLED, semantic::FILL_NONE),
            // Button text
            (tokens::BUTTON_TEXT, semantic::TEXT_ON_ACCENT),
            (tokens::BUTTON_TEXT_DISABLED, semantic::TEXT_DISABLED),
            (tokens::BUTTON_PRIMARY_TEXT, semantic::TEXT_ON_ACCENT),
            (
                tokens::BUTTON_PRIMARY_TEXT_DISABLED,
                semantic::TEXT_DISABLED,
            ),
            // Slider
            (tokens::SLIDER_BG, semantic::FILL_FIELD_DEFAULT),
            (tokens::SLIDER_BG_HOVER, semantic::FILL_FIELD_HOVER),
            (tokens::SLIDER_BG_PRESSED, semantic::FILL_FIELD_PRESSED),
            (tokens::SLIDER_BG_DISABLED, semantic::FILL_FIELD_DEFAULT),
            (tokens::SLIDER_BAR, semantic::FILL_ACCENT_DEFAULT),
            (tokens::SLIDER_BAR_HOVER, semantic::FILL_ACCENT_HOVER),
            (tokens::SLIDER_BAR_PRESSED, semantic::FILL_ACCENT_HOVER), // Pressed is too bright
            (tokens::SLIDER_BAR_DISABLED, semantic::FILL_ACCENT_DISABLED),
            (tokens::SLIDER_TEXT, semantic::TEXT_ON_ACCENT),
            (tokens::SLIDER_TEXT_DISABLED, semantic::TEXT_DISABLED),
            // Scrollbar
            (tokens::SCROLLBAR_BG, semantic::FILL_SOLID_DISABLED),
            (tokens::SCROLLBAR_THUMB, semantic::FILL_ACCENT_DEFAULT),
            (tokens::SCROLLBAR_THUMB_HOVER, semantic::FILL_ACCENT_HOVER), // Needed a dim gray here
            // Checkbox
            (tokens::CHECKBOX_BG, semantic::FILL_SOLID_DEFAULT),
            (tokens::CHECKBOX_BG_HOVER, semantic::FILL_SOLID_DEFAULT),
            (tokens::CHECKBOX_BG_PRESSED, semantic::FILL_SOLID_DEFAULT),
            (tokens::CHECKBOX_BG_DISABLED, semantic::FILL_NONE),
            (tokens::CHECKBOX_BG_CHECKED, semantic::FILL_ACCENT_DEFAULT),
            (tokens::CHECKBOX_BG_CHECKED_DISABLED, semantic::FILL_NONE),
            (
                tokens::CHECKBOX_BG_CHECKED_HOVER,
                semantic::FILL_ACCENT_HOVER,
            ),
            (
                tokens::CHECKBOX_BG_CHECKED_PRESSED,
                semantic::FILL_ACCENT_PRESSED,
            ),
            (tokens::CHECKBOX_BORDER, semantic::FILL_SOLID_DEFAULT),
            (tokens::CHECKBOX_BORDER_HOVER, semantic::FILL_SOLID_HOVER),
            (
                tokens::CHECKBOX_BORDER_PRESSED,
                semantic::FILL_SOLID_PRESSED,
            ),
            (
                tokens::CHECKBOX_BORDER_DISABLED,
                semantic::FILL_SOLID_DISABLED,
            ),
            (
                tokens::CHECKBOX_BORDER_CHECKED,
                semantic::FILL_ACCENT_DEFAULT,
            ),
            (
                tokens::CHECKBOX_BORDER_CHECKED_HOVER,
                semantic::FILL_ACCENT_HOVER,
            ),
            (
                tokens::CHECKBOX_BORDER_CHECKED_PRESSED,
                semantic::FILL_ACCENT_PRESSED,
            ),
            (
                tokens::CHECKBOX_BORDER_CHECKED_DISABLED,
                semantic::FILL_ACCENT_DISABLED,
            ),
            (tokens::CHECKBOX_MARK, semantic::TEXT_ON_ACCENT),
            (tokens::CHECKBOX_MARK_DISABLED, semantic::TEXT_DISABLED),
            (tokens::CHECKBOX_TEXT, semantic::TEXT_DEFAULT),
            (tokens::CHECKBOX_TEXT_DISABLED, semantic::TEXT_DISABLED),
            // Radio (default look is no background)
            (tokens::RADIO_BG, semantic::FILL_NONE),
            (tokens::RADIO_BG_HOVER, semantic::FILL_NONE),
            (tokens::RADIO_BG_PRESSED, semantic::FILL_NONE),
            (tokens::RADIO_BG_DISABLED, semantic::FILL_NONE),
            (tokens::RADIO_BG_CHECKED, semantic::FILL_NONE),
            (tokens::RADIO_BG_CHECKED_HOVER, semantic::FILL_NONE),
            (tokens::RADIO_BG_CHECKED_PRESSED, semantic::FILL_NONE),
            (tokens::RADIO_BG_CHECKED_DISABLED, semantic::FILL_NONE),
            (tokens::RADIO_BORDER, semantic::FILL_SOLID_DEFAULT),
            (tokens::RADIO_BORDER_HOVER, semantic::FILL_SOLID_HOVER),
            (tokens::RADIO_BORDER_PRESSED, semantic::FILL_SOLID_PRESSED),
            (tokens::RADIO_BORDER_DISABLED, semantic::FILL_SOLID_DISABLED),
            (tokens::RADIO_BORDER_CHECKED, semantic::FILL_ACCENT_DEFAULT),
            (
                tokens::RADIO_BORDER_CHECKED_HOVER,
                semantic::FILL_ACCENT_HOVER,
            ),
            (
                tokens::RADIO_BORDER_CHECKED_PRESSED,
                semantic::FILL_ACCENT_PRESSED,
            ),
            (
                tokens::RADIO_BORDER_CHECKED_DISABLED,
                semantic::FILL_ACCENT_DISABLED,
            ),
            (tokens::RADIO_MARK, semantic::FILL_ACCENT_DEFAULT),
            (tokens::RADIO_MARK_HOVER, semantic::FILL_ACCENT_HOVER),
            (tokens::RADIO_MARK_PRESSED, semantic::FILL_ACCENT_PRESSED),
            (tokens::RADIO_MARK_DISABLED, semantic::FILL_ACCENT_DISABLED),
            (tokens::RADIO_TEXT, semantic::TEXT_DEFAULT),
            (tokens::RADIO_TEXT_DISABLED, semantic::TEXT_DISABLED),
            // Toggle Switch
            (tokens::SWITCH_BG, semantic::FILL_SOLID_DEFAULT),
            (tokens::SWITCH_BG_HOVER, semantic::FILL_SOLID_HOVER),
            (tokens::SWITCH_BG_PRESSED, semantic::FILL_SOLID_PRESSED),
            (tokens::SWITCH_BG_DISABLED, semantic::FILL_SOLID_DISABLED),
            (tokens::SWITCH_BG_CHECKED, semantic::FILL_ACCENT_DEFAULT),
            (tokens::SWITCH_BG_CHECKED_HOVER, semantic::FILL_ACCENT_HOVER),
            (
                tokens::SWITCH_BG_CHECKED_PRESSED,
                semantic::FILL_SOLID_PRESSED,
            ),
            (
                tokens::SWITCH_BG_CHECKED_DISABLED,
                semantic::FILL_ACCENT_DISABLED,
            ),
            (tokens::SWITCH_BORDER, semantic::FILL_SOLID_DEFAULT),
            (tokens::SWITCH_BORDER_HOVER, semantic::FILL_SOLID_HOVER),
            (tokens::SWITCH_BORDER_PRESSED, semantic::FILL_SOLID_PRESSED),
            (tokens::SWITCH_BORDER_DISABLED, semantic::FILL_SOLID_DEFAULT),
            (tokens::SWITCH_BORDER_CHECKED, semantic::FILL_ACCENT_DEFAULT),
            (
                tokens::SWITCH_BORDER_CHECKED_HOVER,
                semantic::FILL_ACCENT_HOVER,
            ),
            (
                tokens::SWITCH_BORDER_CHECKED_PRESSED,
                semantic::FILL_ACCENT_PRESSED,
            ),
            (
                tokens::SWITCH_BORDER_CHECKED_DISABLED,
                semantic::FILL_SOLID_DEFAULT,
            ),
            (tokens::SWITCH_SLIDE_BG, semantic::TEXT_DEFAULT),
            (tokens::SWITCH_SLIDE_BG_HOVER, semantic::TEXT_DEFAULT),
            (tokens::SWITCH_SLIDE_BG_PRESSED, semantic::TEXT_DEFAULT),
            (tokens::SWITCH_SLIDE_BG_DISABLED, semantic::TEXT_DISABLED),
            (tokens::SWITCH_SLIDE_BG_CHECKED, semantic::TEXT_DEFAULT),
            (
                tokens::SWITCH_SLIDE_BG_CHECKED_HOVER,
                semantic::TEXT_DEFAULT,
            ),
            (
                tokens::SWITCH_SLIDE_BG_CHECKED_PRESSED,
                semantic::TEXT_DEFAULT,
            ),
            (
                tokens::SWITCH_SLIDE_BG_CHECKED_DISABLED,
                semantic::TEXT_DISABLED,
            ),
            (tokens::SWITCH_SLIDE_BORDER, semantic::TEXT_DEFAULT),
            (tokens::SWITCH_SLIDE_BORDER_HOVER, semantic::TEXT_DEFAULT),
            (tokens::SWITCH_SLIDE_BORDER_PRESSED, semantic::TEXT_DEFAULT),
            (
                tokens::SWITCH_SLIDE_BORDER_DISABLED,
                semantic::TEXT_DISABLED,
            ),
            (tokens::SWITCH_SLIDE_BORDER_CHECKED, semantic::TEXT_DEFAULT),
            (
                tokens::SWITCH_SLIDE_BORDER_CHECKED_HOVER,
                semantic::TEXT_DEFAULT,
            ),
            (
                tokens::SWITCH_SLIDE_BORDER_CHECKED_PRESSED,
                semantic::TEXT_DEFAULT,
            ),
            (
                tokens::SWITCH_SLIDE_BORDER_CHECKED_DISABLED,
                semantic::TEXT_DISABLED,
            ),
            (tokens::COLOR_PLANE_BG, semantic::FILL_FIELD_DEFAULT),
            // Menus
            (tokens::MENU_BG, semantic::SURFACE_MENU),
            (tokens::MENU_BORDER, semantic::BORDER_DEFAULT),
            (tokens::MENUITEM_BG, semantic::FILL_ITEM_DEFAULT),
            (tokens::MENUITEM_BG_HOVER, semantic::FILL_ITEM_HOVER),
            (tokens::MENUITEM_BG_PRESSED, semantic::FILL_ITEM_PRESSED),
            (tokens::MENUITEM_BG_FOCUSED, semantic::FILL_ITEM_PRESSED),
            (tokens::MENUITEM_TEXT, semantic::TEXT_ON_ACCENT),
            (tokens::MENUITEM_TEXT_DISABLED, semantic::TEXT_DISABLED),
            // Text Input
            (tokens::TEXT_INPUT_BG, semantic::FILL_FIELD_DEFAULT),
            (tokens::TEXT_INPUT_LABEL_BG, semantic::FILL_SOLID_DEFAULT),
            (tokens::TEXT_INPUT_TEXT, semantic::TEXT_ON_ACCENT),
            (tokens::TEXT_INPUT_TEXT_DISABLED, semantic::TEXT_DISABLED),
            (tokens::TEXT_INPUT_CURSOR, semantic::FILL_ACCENT_PRESSED),
            (tokens::TEXT_INPUT_SELECTION, semantic::FILL_ACCENT_DEFAULT),
            (tokens::TEXT_INPUT_SELECTION_UNFOCUSED, semantic::FILL_NONE),
            (tokens::TEXT_INPUT_X_AXIS, semantic::AXIS_X),
            (tokens::TEXT_INPUT_Y_AXIS, semantic::AXIS_Y),
            (tokens::TEXT_INPUT_Z_AXIS, semantic::AXIS_Z),
            // Pane
            (tokens::PANE_HEADER_BG, semantic::SURFACE_PANE_HEADER),
            (tokens::PANE_HEADER_BORDER, semantic::SURFACE_PANE_BODY),
            (tokens::PANE_HEADER_TEXT, semantic::TEXT_DEFAULT),
            (tokens::PANE_HEADER_DIVIDER, semantic::SURFACE_PANE_BODY),
            (tokens::PANE_BODY_BG, semantic::SURFACE_PANE_BODY),
            // Subpane
            (tokens::SUBPANE_HEADER_BG, semantic::SURFACE_SUBPANE_HEADER),
            (
                tokens::SUBPANE_HEADER_BORDER,
                semantic::SURFACE_SUBPANE_HEADER,
            ),
            (tokens::SUBPANE_HEADER_TEXT, semantic::TEXT_DEFAULT),
            (tokens::SUBPANE_BODY_BG, semantic::SURFACE_SUBPANE_BODY),
            (
                tokens::SUBPANE_BODY_BORDER,
                semantic::SURFACE_SUBPANE_HEADER,
            ),
            // Group
            (tokens::GROUP_BG, semantic::SURFACE_GROUP),
            (tokens::GROUP_BORDER, semantic::SURFACE_GROUP),
            (tokens::GROUP_HEADER_TEXT, semantic::TEXT_DEFAULT),
            // Listview
            (tokens::LISTROW_BG, semantic::FILL_ITEM_DEFAULT),
            (tokens::LISTROW_BG_HOVER, semantic::FILL_ITEM_HOVER),
            (tokens::LISTROW_BG_SELECTED, semantic::FILL_ITEM_PRESSED),
            (tokens::LISTROW_TEXT, semantic::TEXT_ON_ACCENT),
            (tokens::LISTROW_TEXT_DISABLED, semantic::TEXT_DISABLED),
            (tokens::DIALOG_BG, semantic::SURFACE_DIALOG),
            (tokens::DIALOG_BORDER, semantic::BORDER_DEFAULT),
            (tokens::DIALOG_HEADER_BG, semantic::SURFACE_WINDOW),
            (tokens::DIALOG_TEXT, semantic::TEXT_DEFAULT),
            (tokens::DIALOG_HEADER_TEXT, semantic::TEXT_DEFAULT),
        ]),
    }
}

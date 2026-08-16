use bevy::{
    color::palettes, feathers::theme::ThemeProps, platform::collections::HashMap, prelude::*,
};

/// Creates a basic example theme props for the radio buttons and number inputs.
pub fn basic_example_theme(text_color: Color) -> ThemeProps {
    let mut color = HashMap::new();

    // Radio Button tokens
    color.insert(bevy::feathers::tokens::RADIO_TEXT, text_color);
    color.insert(bevy::feathers::tokens::RADIO_MARK, Color::BLACK);
    color.insert(bevy::feathers::tokens::RADIO_MARK_HOVER, Color::BLACK);
    color.insert(bevy::feathers::tokens::RADIO_MARK_PRESSED, Color::BLACK);

    color.insert(bevy::feathers::tokens::RADIO_BG, Color::WHITE);
    color.insert(
        bevy::feathers::tokens::RADIO_BG_HOVER,
        palettes::basic::GRAY.into(),
    );
    color.insert(
        bevy::feathers::tokens::RADIO_BG_PRESSED,
        palettes::basic::GRAY.into(),
    );
    color.insert(bevy::feathers::tokens::RADIO_BG_CHECKED, Color::WHITE);
    color.insert(bevy::feathers::tokens::RADIO_BG_CHECKED_HOVER, Color::BLACK);
    color.insert(
        bevy::feathers::tokens::RADIO_BG_CHECKED_PRESSED,
        Color::BLACK,
    );

    color.insert(bevy::feathers::tokens::RADIO_BORDER, Color::BLACK);
    color.insert(
        bevy::feathers::tokens::RADIO_BORDER_HOVER,
        palettes::basic::GRAY.into(),
    );
    color.insert(
        bevy::feathers::tokens::RADIO_BORDER_PRESSED,
        palettes::basic::BLACK.into(),
    );
    color.insert(bevy::feathers::tokens::RADIO_BORDER_CHECKED, Color::BLACK);
    color.insert(
        bevy::feathers::tokens::RADIO_BORDER_CHECKED_HOVER,
        Color::BLACK,
    );
    color.insert(
        bevy::feathers::tokens::RADIO_BORDER_CHECKED_PRESSED,
        Color::BLACK,
    );

    // Number Input tokens
    color.insert(bevy::feathers::tokens::TEXT_INPUT_TEXT, text_color);
    color.insert(bevy::feathers::tokens::TEXT_INPUT_BG, Color::BLACK);
    color.insert(bevy::feathers::tokens::TEXT_INPUT_LABEL_BG, Color::BLACK);

    color.insert(bevy::feathers::tokens::SLIDER_BAR, Color::WHITE);
    color.insert(bevy::feathers::tokens::SLIDER_BAR_HOVER, Color::WHITE);
    color.insert(bevy::feathers::tokens::SLIDER_BAR_PRESSED, Color::WHITE);
    color.insert(bevy::feathers::tokens::SLIDER_BG, Color::BLACK);
    color.insert(bevy::feathers::tokens::SLIDER_BG_HOVER, Color::BLACK);
    color.insert(bevy::feathers::tokens::SLIDER_BG_PRESSED, Color::BLACK);

    color.insert(
        bevy::feathers::tokens::TEXT_INPUT_CURSOR,
        bevy::feathers::palette::ACCENT.lighter(0.2),
    );
    color.insert(
        bevy::feathers::tokens::TEXT_INPUT_SELECTION,
        bevy::feathers::palette::ACCENT,
    );
    color.insert(
        bevy::feathers::tokens::TEXT_INPUT_SELECTION_UNFOCUSED,
        bevy::feathers::palette::TRANSPARENT,
    );

    // Checkbox tokens
    color.insert(bevy::feathers::tokens::CHECKBOX_TEXT, text_color);
    color.insert(bevy::feathers::tokens::CHECKBOX_BG, Color::BLACK);
    color.insert(bevy::feathers::tokens::CHECKBOX_MARK, Color::WHITE);
    color.insert(bevy::feathers::tokens::CHECKBOX_BORDER, Color::BLACK);
    color.insert(
        bevy::feathers::tokens::CHECKBOX_BORDER_HOVER,
        bevy::feathers::palette::GRAY_2,
    );
    color.insert(
        bevy::feathers::tokens::CHECKBOX_BORDER_CHECKED_HOVER,
        bevy::feathers::palette::GRAY_0,
    );
    color.insert(
        bevy::feathers::tokens::CHECKBOX_BORDER_PRESSED,
        bevy::feathers::palette::GRAY_1,
    );
    color.insert(
        bevy::feathers::tokens::CHECKBOX_BORDER_CHECKED_PRESSED,
        bevy::feathers::palette::GRAY_1,
    );
    color.insert(
        bevy::feathers::tokens::CHECKBOX_BORDER_CHECKED,
        Color::BLACK,
    );
    color.insert(
        bevy::feathers::tokens::CHECKBOX_BG_HOVER,
        bevy::feathers::palette::GRAY_0,
    );
    color.insert(bevy::feathers::tokens::CHECKBOX_BG_CHECKED, Color::BLACK);
    color.insert(
        bevy::feathers::tokens::CHECKBOX_BG_CHECKED_HOVER,
        Color::BLACK,
    );
    color.insert(
        bevy::feathers::tokens::CHECKBOX_BG_CHECKED_PRESSED,
        bevy::feathers::palette::GRAY_1,
    );
    color.insert(bevy::feathers::tokens::CHECKBOX_BG_PRESSED, Color::BLACK);
    // Feathers Button / Select
    color.insert(
        bevy::feathers::tokens::BUTTON_BG,
        bevy::feathers::palette::TRANSPARENT,
    );
    color.insert(
        bevy::feathers::tokens::BUTTON_BG_HOVER,
        bevy::feathers::palette::TRANSPARENT,
    );
    color.insert(
        bevy::feathers::tokens::BUTTON_BG_PRESSED,
        bevy::feathers::palette::TRANSPARENT,
    );
    color.insert(
        bevy::feathers::tokens::MENU_BG,
        bevy::feathers::palette::GRAY_0,
    );
    color.insert(
        bevy::feathers::tokens::MENU_BORDER,
        bevy::feathers::palette::GRAY_0,
    );
    color.insert(
        bevy::feathers::tokens::MENUITEM_BG_HOVER,
        bevy::feathers::palette::TRANSPARENT,
    );
    color.insert(
        bevy::feathers::tokens::MENUITEM_BG_PRESSED,
        bevy::feathers::palette::TRANSPARENT,
    );
    color.insert(
        bevy::feathers::tokens::MENUITEM_BG_FOCUSED,
        bevy::feathers::palette::TRANSPARENT,
    );
    color.insert(
        bevy::feathers::tokens::LISTROW_BG,
        bevy::feathers::palette::TRANSPARENT,
    );
    color.insert(
        bevy::feathers::tokens::LISTROW_BG_HOVER,
        bevy::feathers::palette::BLACK,
    );
    color.insert(
        bevy::feathers::tokens::LISTROW_BG_SELECTED,
        bevy::feathers::palette::BLACK,
    );
    color.insert(bevy::feathers::tokens::BUTTON_TEXT, text_color);
    color.insert(bevy::feathers::tokens::LISTROW_TEXT, text_color);

    // Slider tokens
    color.insert(
        bevy::feathers::tokens::SLIDER_BG,
        bevy::feathers::palette::BLACK,
    );
    color.insert(
        bevy::feathers::tokens::SLIDER_BG_HOVER,
        bevy::feathers::palette::GRAY_0,
    );
    color.insert(
        bevy::feathers::tokens::SLIDER_BG_PRESSED,
        bevy::feathers::palette::GRAY_1,
    );
    color.insert(
        bevy::feathers::tokens::SLIDER_BAR,
        bevy::feathers::palette::ACCENT,
    );
    color.insert(
        bevy::feathers::tokens::SLIDER_BAR_HOVER,
        bevy::feathers::palette::ACCENT.lighter(0.05),
    );
    color.insert(
        bevy::feathers::tokens::SLIDER_BAR_PRESSED,
        bevy::feathers::palette::ACCENT.lighter(0.1),
    );
    color.insert(bevy::feathers::tokens::SLIDER_TEXT, text_color);

    // Button tokens
    color.insert(
        bevy::feathers::tokens::BUTTON_PLAIN_BG,
        bevy::feathers::palette::BLACK,
    );
    color.insert(
        bevy::feathers::tokens::BUTTON_PLAIN_BG_HOVER,
        bevy::feathers::palette::GRAY_0,
    );
    color.insert(
        bevy::feathers::tokens::BUTTON_PLAIN_BG_PRESSED,
        bevy::feathers::palette::GRAY_1,
    );
    color.insert(bevy::feathers::tokens::BUTTON_TEXT, text_color);

    // Pane tokens
    color.insert(
        bevy::feathers::tokens::PANE_HEADER_BG,
        bevy::feathers::palette::BLACK,
    );
    color.insert(
        bevy::feathers::tokens::PANE_HEADER_BORDER,
        bevy::feathers::palette::GRAY_0,
    );
    color.insert(bevy::feathers::tokens::PANE_HEADER_TEXT, text_color);
    color.insert(
        bevy::feathers::tokens::PANE_BODY_BG,
        bevy::feathers::palette::GRAY_0,
    );

    // SubPane tokens
    color.insert(
        bevy::feathers::tokens::SUBPANE_BODY_BG,
        bevy::feathers::palette::GRAY_1,
    );
    color.insert(
        bevy::feathers::tokens::SUBPANE_BODY_BORDER,
        bevy::feathers::palette::GRAY_2,
    );
    color.insert(
        bevy::feathers::tokens::SUBPANE_HEADER_BG,
        bevy::feathers::palette::GRAY_2,
    );
    color.insert(
        bevy::feathers::tokens::SUBPANE_HEADER_BORDER,
        bevy::feathers::palette::GRAY_2,
    );
    color.insert(bevy::feathers::tokens::SUBPANE_HEADER_TEXT, text_color);

    // Listbox tokens
    color.insert(
        bevy::feathers::tokens::LISTROW_BG,
        bevy::feathers::palette::GRAY_1,
    );
    color.insert(
        bevy::feathers::tokens::LISTROW_BG_HOVER,
        bevy::feathers::palette::GRAY_2,
    );
    color.insert(
        bevy::feathers::tokens::LISTROW_BG_SELECTED,
        bevy::feathers::palette::GRAY_3,
    );
    color.insert(bevy::feathers::tokens::LISTROW_TEXT, text_color);
    color.insert(
        bevy::feathers::tokens::FOCUS_RING,
        bevy::feathers::palette::ACCENT.with_alpha(0.5),
    );

    // Scrollbar tokens
    color.insert(
        bevy::feathers::tokens::SCROLLBAR_BG,
        bevy::feathers::palette::WARM_GRAY_1,
    );
    color.insert(
        bevy::feathers::tokens::SCROLLBAR_THUMB,
        bevy::feathers::palette::ACCENT,
    );
    color.insert(
        bevy::feathers::tokens::SCROLLBAR_THUMB_HOVER,
        bevy::feathers::palette::ACCENT.lighter(0.05),
    );

    // Main text color
    color.insert(bevy::feathers::tokens::TEXT_MAIN, text_color);
    ThemeProps::new_non_contextual(color)
}

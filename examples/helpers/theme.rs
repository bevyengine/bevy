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

    // Main text color
    color.insert(bevy::feathers::tokens::TEXT_MAIN, text_color);
    ThemeProps { color }
}

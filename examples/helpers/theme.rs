use bevy::{
    color::palettes, feathers::theme::ThemeProps, platform::collections::HashMap, prelude::*,
};

/// Creates a basic feathers theme props for the radio buttons.
pub fn basic_radio_button_theme(text_color: Color) -> ThemeProps {
    let mut color = HashMap::new();
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

    color.insert(bevy::feathers::tokens::TEXT_MAIN, text_color);
    ThemeProps { color }
}

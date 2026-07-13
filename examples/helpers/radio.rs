/// Helpers to create a basic option menu using Feathers Radio Buttons.
/// Using these helpers requires the `bevy_feathers` feature to be enabled.
use bevy::{
    color::palettes,
    feathers::{controls::FeathersRadio, display::caption, theme::ThemeProps},
    platform::collections::HashMap,
    prelude::*,
    ui::Checked,
    ui_widgets::RadioGroup,
};

/// A component that wraps a radio button's option value
#[derive(Clone, Copy, Component, Default)]
pub struct RadioButtonOptionValue<T>(pub T)
where
    T: Clone + Default + Send + Sync + Unpin + 'static;

/// Returns a [`Node`] appropriate for the outer main UI node as a `Scene`.
///
/// This UI is in the bottom left corner and has flex column support
pub fn main_ui_node_scene() -> impl Scene {
    bsn! {
        Node {
            flex_direction: FlexDirection::Column,
            position_type: PositionType::Absolute,
            row_gap: px(6),
            left: px(10),
            bottom: px(10),
        }
    }
}

/// Creates a basic feathers theme props for the radio buttons.
pub fn basic_radio_button_theme() -> ThemeProps {
    let mut color = HashMap::new();
    color.insert(bevy::feathers::tokens::RADIO_TEXT, Color::BLACK);
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
    ThemeProps { color }
}

/// Spawns the radio buttons that allow configuration of a setting.
///
/// To react to changes in value, create an observer that listens to
/// `ValueChange<Entity>>`. Query for the value entity's `RadioButtonOptionValue`
/// and unwrap the new option value.
///
/// Ensure the radio button self updates its own state by adding the
/// `ui_widgets::radio_self_update` observer to the app.
pub fn feathers_option_buttons<T>(title: &'static str, options: &[(T, &str)]) -> impl Scene
where
    T: Clone + Default + Send + Sync + Unpin + 'static,
{
    let buttons = options
        .iter()
        .cloned()
        .enumerate()
        .map(|(option_index, (option_value, option_name))| {
            feathers_option_button(option_value, option_name, option_index == 0)
        })
        .collect::<Vec<_>>();
    // Add the parent node for the row.
    bsn! {
        Node {
            align_items: AlignItems::Center,
            column_gap: px(5),
        }
        RadioGroup
        Children [
            Text::new(title)
            TextFont {
                font_size: FontSize::Px(18.0),
            }
            TextColor(Color::BLACK),
            {buttons}
        ]
    }
}

/// Spawns a single feathers radio button that allows configuration of a setting.
fn feathers_option_button<T>(
    option_value: T,
    option_name: &str,
    is_selected: bool,
) -> Box<dyn Scene>
where
    T: Clone + Default + Send + Sync + Unpin + 'static,
{
    if is_selected {
        Box::new(bsn! {
            @FeathersRadio {
                @caption: bsn! { caption(option_name) }
            }
            Checked
            RadioButtonOptionValue<T>(option_value)
        })
    } else {
        Box::new(bsn! {
            @FeathersRadio {
                @caption: bsn! { caption(option_name) }
            }
            RadioButtonOptionValue<T>(option_value)
        })
    }
}

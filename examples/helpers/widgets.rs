//! Simple widgets for example UI.
//!
//! Unlike other examples, which demonstrate an application, this demonstrates a plugin library.

use bevy::prelude::*;

/// An event that's sent whenever the user changes one of the settings by
/// clicking a radio button.
#[derive(Clone, Message, Deref, DerefMut)]
pub struct WidgetClickEvent<T>(T);

/// A marker component that we place on all widgets that send
/// [`WidgetClickEvent`]s of the given type.
#[derive(Clone, Component, Deref, DerefMut)]
pub struct WidgetClickSender<T>(T)
where
    T: Clone + Send + Sync + 'static;

/// A marker component that we place on all radio `Button`s.
#[derive(Clone, Copy, Component)]
pub struct RadioButton;

/// A marker component that we place on all `Text` inside radio buttons.
#[derive(Clone, Copy, Component)]
pub struct RadioButtonText;

/// The size of the border that surrounds buttons.
pub const BUTTON_BORDER: UiRect = UiRect::all(Val::Px(1.0));

/// The color of the border that surrounds buttons.
pub const BUTTON_BORDER_COLOR: BorderColor = BorderColor {
    left: Color::WHITE,
    right: Color::WHITE,
    top: Color::WHITE,
    bottom: Color::WHITE,
};

/// The amount of rounding to apply to button corners.
pub const BUTTON_BORDER_RADIUS_SIZE: Val = Val::Px(6.0);

/// The amount of space between the edge of the button and its label.
pub const BUTTON_PADDING: UiRect = UiRect::axes(Val::Px(12.0), Val::Px(6.0));

/// Returns a [`Node`] appropriate for the outer main UI node.
///
/// This UI is in the bottom left corner and has flex column support
pub fn main_ui_node() -> Node {
    Node {
        flex_direction: FlexDirection::Column,
        position_type: PositionType::Absolute,
        row_gap: px(6),
        left: px(10),
        bottom: px(10),
        ..default()
    }
}

/// Spawns a single radio button that allows configuration of a setting.
///
/// The type parameter specifies the value that will be packaged up and sent in
/// a [`WidgetClickEvent`] when the radio button is clicked.
pub fn option_button<T>(
    option_value: T,
    option_name: &str,
    is_selected: bool,
    is_first: bool,
    is_last: bool,
) -> impl Bundle
where
    T: Clone + Send + Sync + 'static,
{
    let (bg_color, fg_color) = if is_selected {
        (Color::WHITE, Color::BLACK)
    } else {
        (Color::BLACK, Color::WHITE)
    };

    // Add the button node.
    (
        Button,
        Node {
            border: BUTTON_BORDER.with_left(if is_first { px(1) } else { px(0) }),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            padding: BUTTON_PADDING,
            ..default()
        },
        BUTTON_BORDER_COLOR,
        BorderRadius::ZERO
            .with_left(if is_first {
                BUTTON_BORDER_RADIUS_SIZE
            } else {
                px(0)
            })
            .with_right(if is_last {
                BUTTON_BORDER_RADIUS_SIZE
            } else {
                px(0)
            }),
        BackgroundColor(bg_color),
        RadioButton,
        WidgetClickSender(option_value.clone()),
        children![(
            ui_text(option_name, fg_color),
            RadioButtonText,
            WidgetClickSender(option_value),
        )],
    )
}

/// Spawns the buttons that allow configuration of a setting.
///
/// The user may change the setting to any one of the labeled `options`. The
/// value of the given type parameter will be packaged up and sent as a
/// [`WidgetClickEvent`] when one of the radio buttons is clicked.
pub fn option_buttons<T>(title: &str, options: &[(T, &str)]) -> impl Bundle
where
    T: Clone + Send + Sync + 'static,
{
    let buttons = options
        .iter()
        .cloned()
        .enumerate()
        .map(|(option_index, (option_value, option_name))| {
            option_button(
                option_value,
                option_name,
                option_index == 0,
                option_index == 0,
                option_index == options.len() - 1,
            )
        })
        .collect::<Vec<_>>();
    // Add the parent node for the row.
    (
        Node {
            align_items: AlignItems::Center,
            ..default()
        },
        Children::spawn((
            Spawn((
                ui_text(title, Color::BLACK),
                Node {
                    width: px(125),
                    ..default()
                },
            )),
            SpawnIter(buttons.into_iter()),
        )),
    )
}

/// Creates a text bundle for the UI.
pub fn ui_text(label: &str, color: Color) -> impl Bundle + use<> {
    (
        Text::new(label),
        TextFont {
            font_size: 18.0,
            ..default()
        },
        TextColor(color),
    )
}

/// Checks for clicks on the radio buttons and sends `RadioButtonChangeEvent`s
/// as necessary.
pub fn handle_ui_interactions<T>(
    mut interactions: Query<
        (&Interaction, &WidgetClickSender<T>),
        (With<Button>, With<RadioButton>),
    >,
    mut widget_click_events: MessageWriter<WidgetClickEvent<T>>,
) where
    T: Clone + Send + Sync + 'static,
{
    for (interaction, click_event) in interactions.iter_mut() {
        if *interaction == Interaction::Pressed {
            widget_click_events.write(WidgetClickEvent((**click_event).clone()));
        }
    }
}

/// Updates the style of the button part of a radio button to reflect its
/// selected status.
pub fn update_ui_radio_button(background_color: &mut BackgroundColor, selected: bool) {
    background_color.0 = if selected { Color::WHITE } else { Color::BLACK };
}

/// Updates the color of the label of a radio button to reflect its selected
/// status.
pub fn update_ui_radio_button_text(entity: Entity, writer: &mut TextUiWriter, selected: bool) {
    let text_color = if selected { Color::BLACK } else { Color::WHITE };

    writer.for_each_color(entity, |mut color| {
        color.0 = text_color;
    });
}

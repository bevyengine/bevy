//! Simple widgets for example UI.

use bevy::{ecs::system::EntityCommands, prelude::*};

/// An event that's sent whenever the user changes one of the settings by
/// clicking a radio button.
#[derive(Clone, Event, Deref, DerefMut)]
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

/// Returns a [`Style`] appropriate for the outer main UI node.
///
/// This UI is in the bottom left corner and has flex column support
pub fn main_ui_style() -> Style {
    Style {
        flex_direction: FlexDirection::Column,
        position_type: PositionType::Absolute,
        row_gap: Val::Px(6.0),
        left: Val::Px(10.0),
        bottom: Val::Px(10.0),
        ..default()
    }
}

/// Spawns a single radio button that allows configuration of a setting.
///
/// The type parameter specifies the value that will be packaged up and sent in
/// a [`WidgetClickEvent`] when the radio button is clicked.
pub fn spawn_option_button<T>(
    parent: &mut ChildBuilder,
    option_value: T,
    option_name: &str,
    is_selected: bool,
    is_first: bool,
    is_last: bool,
) where
    T: Clone + Send + Sync + 'static,
{
    let (bg_color, fg_color) = if is_selected {
        (Color::WHITE, Color::BLACK)
    } else {
        (Color::BLACK, Color::WHITE)
    };

    // Add the button node.
    parent
        .spawn((
            Button,
            Style {
                border: UiRect::all(Val::Px(1.0)).with_left(if is_first {
                    Val::Px(1.0)
                } else {
                    Val::Px(0.0)
                }),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                padding: UiRect::axes(Val::Px(12.0), Val::Px(6.0)),
                ..default()
            },
            BorderColor(Color::WHITE),
            BorderRadius::ZERO
                .with_left(if is_first { Val::Px(6.0) } else { Val::Px(0.0) })
                .with_right(if is_last { Val::Px(6.0) } else { Val::Px(0.0) }),
            BackgroundColor(bg_color),
        ))
        .insert(RadioButton)
        .insert(WidgetClickSender(option_value.clone()))
        .with_children(|parent| {
            spawn_ui_text(parent, option_name, fg_color)
                .insert(RadioButtonText)
                .insert(WidgetClickSender(option_value));
        });
}

/// Spawns the buttons that allow configuration of a setting.
///
/// The user may change the setting to any one of the labeled `options`. The
/// value of the given type parameter will be packaged up and sent as a
/// [`WidgetClickEvent`] when one of the radio buttons is clicked.
pub fn spawn_option_buttons<T>(parent: &mut ChildBuilder, title: &str, options: &[(T, &str)])
where
    T: Clone + Send + Sync + 'static,
{
    // Add the parent node for the row.
    parent
        .spawn((
            Node::default(),
            Style {
                align_items: AlignItems::Center,
                ..default()
            },
        ))
        .with_children(|parent| {
            spawn_ui_text(parent, title, Color::BLACK).insert(Style {
                width: Val::Px(125.0),
                ..default()
            });

            for (option_index, (option_value, option_name)) in options.iter().cloned().enumerate() {
                spawn_option_button(
                    parent,
                    option_value,
                    option_name,
                    option_index == 0,
                    option_index == 0,
                    option_index == options.len() - 1,
                );
            }
        });
}

/// Spawns text for the UI.
///
/// Returns the `EntityCommands`, which allow further customization of the text
/// style.
pub fn spawn_ui_text<'a>(
    parent: &'a mut ChildBuilder,
    label: &str,
    color: Color,
) -> EntityCommands<'a> {
    parent.spawn((
        Text::new(label),
        TextFont {
            font_size: 18.0,
            ..default()
        },
        TextColor(color),
    ))
}

/// Checks for clicks on the radio buttons and sends `RadioButtonChangeEvent`s
/// as necessary.
pub fn handle_ui_interactions<T>(
    mut interactions: Query<
        (&Interaction, &WidgetClickSender<T>),
        (With<Button>, With<RadioButton>),
    >,
    mut widget_click_events: EventWriter<WidgetClickEvent<T>>,
) where
    T: Clone + Send + Sync + 'static,
{
    for (interaction, click_event) in interactions.iter_mut() {
        if *interaction == Interaction::Pressed {
            widget_click_events.send(WidgetClickEvent((**click_event).clone()));
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

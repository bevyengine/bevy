//! This example illustrates how does the `InteractionPolicy` component work and its purpose.
//! It's a scene with two buttons, one having the `Hold` interaction policy
//! and the other having the `Release` interaction policy.

use bevy::{
    prelude::*,
    ui::{
        InteractionState, InteractionStateChangedFilter, InteractionStateHandler, PressPolicy,
        RelativeCursorPosition,
    },
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(Update, button_system)
        .run();
}

const NORMAL_BUTTON: Color = Color::rgb(0.15, 0.15, 0.15);
const HOVERED_BUTTON: Color = Color::rgb(0.25, 0.25, 0.25);
const PRESSED_BUTTON: Color = Color::rgb(0.35, 0.75, 0.35);

fn button_system(
    mut interaction_query: Query<
        (
            &Pressed,
            &RelativeCursorPosition,
            &mut BackgroundColor,
            &Children,
        ),
        (InteractionStateChangedFilter, With<Button>),
    >,
    mut text_query: Query<&mut Text>,
) {
    for (pressed, relative_cursor_position, mut color, children) in &mut interaction_query {
        let mut text = text_query.get_mut(children[0]).unwrap();
        match (pressed, relative_cursor_position).interaction_state() {
            InteractionState::Pressed => {
                text.sections[0].value = "Press".to_string();
                *color = PRESSED_BUTTON.into();
            }
            InteractionState::Hovered => {
                text.sections[0].value = "Hover".to_string();
                *color = HOVERED_BUTTON.into();
            }
            InteractionState::None => {
                text.sections[0].value = "Button".to_string();
                *color = NORMAL_BUTTON.into();
            }
        }
    }
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    // UI camera
    commands.spawn(Camera2dBundle::default());

    commands
        .spawn(NodeBundle {
            style: Style {
                size: Size::new(Val::Percent(100.0), Val::Percent(100.0)),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                flex_direction: FlexDirection::Column,
                ..default()
            },
            ..default()
        })
        .with_children(|parent| {
            parent.spawn(TextBundle::from_section(
                "Try clicking a button and then stop hovering over it\nwhile keeping the mouse button pressed",
                TextStyle {
                    font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                    font_size: 40.0,
                    color: Color::WHITE,
                }
            ).with_text_alignment(TextAlignment::Center).with_style(Style {
                margin: UiRect::bottom(Val::Px(35.)),
                ..default()
            }));

            parent.spawn(NodeBundle::default()).with_children(|parent| {
                parent
                    .spawn(ButtonBundle {
                        style: Style {
                            size: Size::new(Val::Px(150.0), Val::Px(65.0)),
                            // horizontally center child text
                            justify_content: JustifyContent::Center,
                            // vertically center child text
                            align_items: AlignItems::Center,
                            margin: UiRect::right(Val::Px(15.)),
                            ..default()
                        },
                        background_color: NORMAL_BUTTON.into(),
                        pressed: Pressed::new(PressPolicy::Hold),    // The button on the left has the Hold interaction policy
                        ..default()
                    })
                    .with_children(|parent| {
                        parent.spawn(TextBundle::from_section(
                            "Button",
                            TextStyle {
                                font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                                font_size: 40.0,
                                color: Color::rgb(0.9, 0.9, 0.9),
                            },
                        ));
                    });

                parent
                    .spawn(ButtonBundle {
                        style: Style {
                            size: Size::new(Val::Px(150.0), Val::Px(65.0)),
                            // horizontally center child text
                            justify_content: JustifyContent::Center,
                            // vertically center child text
                            align_items: AlignItems::Center,
                            margin: UiRect::left(Val::Px(15.)),
                            ..default()
                        },
                        background_color: NORMAL_BUTTON.into(),
                        pressed: Pressed::new(PressPolicy::Release),    // The button on the right has the Release interaction policy
                        ..default()
                    })
                    .with_children(|parent| {
                        parent.spawn(TextBundle::from_section(
                            "Button",
                            TextStyle {
                                font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                                font_size: 40.0,
                                color: Color::rgb(0.9, 0.9, 0.9),
                            },
                        ));
                    });
            });
        });
}

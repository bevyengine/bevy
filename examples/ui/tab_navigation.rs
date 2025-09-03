//! This example illustrates the use of tab navigation.

use bevy::{
    color::palettes::basic::*,
    input_focus::{
        tab_navigation::{TabGroup, TabIndex, TabNavigationPlugin},
        InputDispatchPlugin, InputFocus,
    },
    prelude::*,
    winit::WinitSettings,
};

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, InputDispatchPlugin, TabNavigationPlugin))
        // Only run the app when there is user input. This will significantly reduce CPU/GPU use.
        .insert_resource(WinitSettings::desktop_app())
        .add_systems(Startup, setup)
        .add_systems(Update, (button_system, focus_system))
        .run();
}

const NORMAL_BUTTON: Color = Color::srgb(0.15, 0.15, 0.15);
const HOVERED_BUTTON: Color = Color::srgb(0.25, 0.25, 0.25);
const PRESSED_BUTTON: Color = Color::srgb(0.35, 0.75, 0.35);

fn button_system(
    mut interaction_query: Query<
        (&Interaction, &mut BackgroundColor, &mut BorderColor),
        (Changed<Interaction>, With<Button>),
    >,
) {
    for (interaction, mut color, mut border_color) in &mut interaction_query {
        match *interaction {
            Interaction::Pressed => {
                *color = PRESSED_BUTTON.into();
                *border_color = BorderColor::all(RED);
            }
            Interaction::Hovered => {
                *color = HOVERED_BUTTON.into();
                *border_color = BorderColor::all(Color::WHITE);
            }
            Interaction::None => {
                *color = NORMAL_BUTTON.into();
                *border_color = BorderColor::all(Color::BLACK);
            }
        }
    }
}

fn focus_system(
    mut commands: Commands,
    focus: Res<InputFocus>,
    mut query: Query<Entity, With<Button>>,
) {
    if focus.is_changed() {
        for button in query.iter_mut() {
            if focus.0 == Some(button) {
                commands.entity(button).insert(Outline {
                    color: Color::WHITE,
                    width: px(2),
                    offset: px(2),
                });
            } else {
                commands.entity(button).remove::<Outline>();
            }
        }
    }
}

fn setup(mut commands: Commands) {
    // ui camera
    commands.spawn(Camera2d);
    commands
        .spawn(Node {
            width: percent(100),
            height: percent(100),
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            row_gap: px(6),
            ..default()
        })
        .observe(
            |mut event: On<Pointer<Click>>, mut focus: ResMut<InputFocus>| {
                focus.0 = None;
                event.propagate(false);
            },
        )
        .with_children(|parent| {
            for (label, tab_group, indices) in [
                // In this group all the buttons have the same `TabIndex` so they will be visited according to their order as children.
                ("TabGroup 0", TabGroup::new(0), [0, 0, 0, 0]),
                // In this group the `TabIndex`s are reversed so the buttons will be visited in right-to-left order.
                ("TabGroup 2", TabGroup::new(2), [3, 2, 1, 0]),
                // In this group the orders of the indices and buttons match so the buttons will be visited in left-to-right order.
                ("TabGroup 1", TabGroup::new(1), [0, 1, 2, 3]),
                // Visit the modal group's buttons in an arbitrary order.
                ("Modal TabGroup", TabGroup::modal(), [0, 3, 1, 2]),
            ] {
                parent.spawn(Text::new(label));
                parent
                    .spawn((
                        Node {
                            display: Display::Flex,
                            flex_direction: FlexDirection::Row,
                            column_gap: px(6),
                            margin: UiRect {
                                bottom: px(10),
                                ..default()
                            },
                            ..default()
                        },
                        tab_group,
                    ))
                    .with_children(|parent| {
                        for i in indices {
                            parent
                                .spawn((
                                    Button,
                                    Node {
                                        width: px(200),
                                        height: px(65),
                                        border: UiRect::all(px(5)),
                                        justify_content: JustifyContent::Center,
                                        align_items: AlignItems::Center,
                                        ..default()
                                    },
                                    BorderColor::all(Color::BLACK),
                                    BackgroundColor(NORMAL_BUTTON),
                                    TabIndex(i),
                                    children![(
                                        Text::new(format!("TabIndex {i}")),
                                        TextFont {
                                            font_size: 20.0,
                                            ..default()
                                        },
                                        TextColor(Color::srgb(0.9, 0.9, 0.9)),
                                    )],
                                ))
                                .observe(
                                    |mut event: On<Pointer<Click>>,
                                    mut focus: ResMut<InputFocus>| {
                                        focus.0 = Some(event.entity());
                                        event.propagate(false);
                                    },
                                );
                        }
                    });
            }
        });
}

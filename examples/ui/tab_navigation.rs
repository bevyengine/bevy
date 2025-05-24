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
        (
            &Interaction,
            &mut BackgroundColor,
            &mut BorderColor,
            &Children,
        ),
        (Changed<Interaction>, With<Button>),
    >,
    mut text_query: Query<&mut Text>,
) {
    for (interaction, mut color, mut border_color, children) in &mut interaction_query {
        let mut text = text_query.get_mut(children[0]).unwrap();
        match *interaction {
            Interaction::Pressed => {
                **text = "Press".to_string();
                *color = PRESSED_BUTTON.into();
                border_color.0 = RED.into();
            }
            Interaction::Hovered => {
                **text = "Hover".to_string();
                *color = HOVERED_BUTTON.into();
                border_color.0 = Color::WHITE;
            }
            Interaction::None => {
                **text = "Button".to_string();
                *color = NORMAL_BUTTON.into();
                border_color.0 = Color::BLACK;
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
                    width: Val::Px(2.0),
                    offset: Val::Px(2.0),
                });
            } else {
                commands.entity(button).remove::<Outline>();
            }
        }
    }
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    // ui camera
    commands.spawn(Camera2d);
    commands
        .spawn(Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            ..default()
        })
        .observe(
            |mut trigger: Trigger<Pointer<Click>>, mut focus: ResMut<InputFocus>| {
                focus.0 = None;
                trigger.propagate(false);
            },
        )
        .with_children(|parent| {
            parent.spawn(Text::new("Tab Group 0"));
            parent
                .spawn((
                    Node {
                        display: Display::Flex,
                        flex_direction: FlexDirection::Row,
                        column_gap: Val::Px(6.0),
                        margin: UiRect {
                            bottom: Val::Px(10.0),
                            ..default()
                        },
                        ..default()
                    },
                    TabGroup::new(0),
                ))
                .with_children(|parent| {
                    create_button(parent, &asset_server);
                    create_button(parent, &asset_server);
                    create_button(parent, &asset_server);
                    create_button(parent, &asset_server);
                });

            parent.spawn(Text::new("Tab Group 2"));
            parent
                .spawn((
                    Node {
                        display: Display::Flex,
                        flex_direction: FlexDirection::Row,
                        column_gap: Val::Px(6.0),
                        margin: UiRect {
                            bottom: Val::Px(10.0),
                            ..default()
                        },
                        ..default()
                    },
                    TabGroup::new(2),
                ))
                .with_children(|parent| {
                    create_button(parent, &asset_server);
                    create_button(parent, &asset_server);
                    create_button(parent, &asset_server);
                    create_button(parent, &asset_server);
                });

            parent.spawn(Text::new("Tab Group 1"));
            parent
                .spawn((
                    Node {
                        display: Display::Flex,
                        flex_direction: FlexDirection::Row,
                        column_gap: Val::Px(6.0),
                        margin: UiRect {
                            bottom: Val::Px(10.0),
                            ..default()
                        },
                        ..default()
                    },
                    TabGroup::new(1),
                ))
                .with_children(|parent| {
                    create_button(parent, &asset_server);
                    create_button(parent, &asset_server);
                    create_button(parent, &asset_server);
                    create_button(parent, &asset_server);
                });

            parent.spawn(Text::new("Modal Tab Group"));
            parent
                .spawn((
                    Node {
                        display: Display::Flex,
                        flex_direction: FlexDirection::Row,
                        column_gap: Val::Px(6.0),
                        ..default()
                    },
                    TabGroup::modal(),
                ))
                .with_children(|parent| {
                    create_button(parent, &asset_server);
                    create_button(parent, &asset_server);
                    create_button(parent, &asset_server);
                    create_button(parent, &asset_server);
                });
        });
}

fn create_button(parent: &mut ChildSpawnerCommands<'_>, asset_server: &AssetServer) {
    parent
        .spawn((
            Button,
            Node {
                width: Val::Px(150.0),
                height: Val::Px(65.0),
                border: UiRect::all(Val::Px(5.0)),
                // horizontally center child text
                justify_content: JustifyContent::Center,
                // vertically center child text
                align_items: AlignItems::Center,
                ..default()
            },
            BorderColor(Color::BLACK),
            BorderRadius::MAX,
            BackgroundColor(NORMAL_BUTTON),
            TabIndex(0),
        ))
        .observe(
            |mut trigger: Trigger<Pointer<Click>>, mut focus: ResMut<InputFocus>| {
                focus.0 = Some(trigger.target());
                trigger.propagate(false);
            },
        )
        .with_child((
            Text::new("Button"),
            TextFont {
                font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                font_size: 23.0,
                ..default()
            },
            TextColor(Color::srgb(0.9, 0.9, 0.9)),
        ));
}

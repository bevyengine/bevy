//! multiple text inputs example

use bevy::color::palettes::css::{NAVY, YELLOW};
use bevy::core_widgets::{Activate, Callback, CoreButton, CoreWidgetsPlugins};
use bevy::ecs::relationship::RelatedSpawner;
use bevy::input_focus::{
    tab_navigation::{TabIndex, TabNavigationPlugin},
    InputDispatchPlugin,
};
use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::text::{PasswordMask, Placeholder, TextInputValue};
use bevy::ui::widget::{TextField, TextInputPlugin};

const MAX_PASSWORD_LENGTH: usize = 10;

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            InputDispatchPlugin,
            TabNavigationPlugin,
            CoreWidgetsPlugins,
            TextInputPlugin,
        ))
        .add_systems(Startup, setup)
        .add_systems(Update, update_char_count)
        .run();
}

#[derive(Component)]
struct CharCountNode;

fn setup(mut commands: Commands) {
    // UI camera
    commands.spawn(Camera2d);

    let on_click =
        commands.register_system(|_: In<Activate>, mut query: Query<&mut PasswordMask>| {
            for mut password in query.iter_mut() {
                password.show_password = !password.show_password;
            }
        });

    commands.spawn((
        Node {
            width: Val::Percent(100.),
            height: Val::Percent(100.),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(20.),
            ..Default::default()
        },
        children![(
            Node {
                flex_direction: FlexDirection::Column,
                border: UiRect::all(Val::Px(5.)),
                padding: UiRect::all(Val::Px(10.)),
                row_gap: Val::Px(10.),
                ..default()
            },
            BorderColor::all(YELLOW),
            BackgroundColor(NAVY.into()),
            Children::spawn((
                Spawn(Text::new("Password Input Field Demo"),),
                Spawn((
                    Node::default(),
                    Children::spawn(SpawnWith(move |parent: &mut RelatedSpawner<ChildOf>| {
                        parent.spawn((
                            Node {
                                flex_direction: FlexDirection::Column,
                                width: Val::Px(300.),
                                border: UiRect::all(Val::Px(2.0)),
                                padding: UiRect::all(Val::Px(2.0)),
                                ..default()
                            },
                            BorderColor::all(Color::WHITE),
                            BackgroundColor(Color::BLACK),
                            children![(
                                TextField {
                                    max_chars: Some(MAX_PASSWORD_LENGTH),
                                    justify: Justify::Left,
                                    clear_on_submit: true,
                                },
                                Placeholder::new("enter a password"),
                                TextColor(Color::WHITE),
                                TabIndex(0),
                                TextInputValue::default(),
                                PasswordMask::default(),
                            )],
                        ));

                        parent.spawn((
                            Node {
                                border: UiRect::all(Val::Px(2.0)),
                                padding: UiRect::all(Val::Px(2.0)),
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
                                ..default()
                            },
                            CoreButton {
                                on_activate: Callback::System(on_click),
                            },
                            Hovered::default(),
                            TabIndex(0),
                            BorderColor::all(Color::WHITE),
                            BackgroundColor(Color::BLACK),
                            children![(Text::new("Show/Hide"),)],
                        ));
                    }))
                )),
                Spawn((
                    Node::default(),
                    children![(
                        Text::new(format!("{MAX_PASSWORD_LENGTH} characters left.")),
                        CharCountNode,
                    )]
                ))
            )),
        )],
    ));
}

fn update_char_count(
    value_query: Query<&TextInputValue, Changed<TextInputValue>>,
    mut text_query: Query<&mut Text, With<CharCountNode>>,
) {
    if let Ok(value) = value_query.single()
        && let Ok(mut text) = text_query.single_mut()
    {
        text.0 = format!(
            "{} characters left.",
            MAX_PASSWORD_LENGTH - value.get().chars().count()
        );
    }
}

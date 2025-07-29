//! multiple text inputs example

use bevy::color::palettes::css::NAVY;
use bevy::color::palettes::css::YELLOW;
use bevy::core_widgets::Activate;
use bevy::core_widgets::Callback;
use bevy::core_widgets::CoreButton;
use bevy::core_widgets::CoreWidgetsPlugins;
use bevy::input_focus::tab_navigation::TabIndex;
use bevy::input_focus::tab_navigation::TabNavigationPlugin;
use bevy::input_focus::InputDispatchPlugin;
use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::text::Prompt;
use bevy::text::TextInputPasswordMask;
use bevy::text::TextInputValue;
use bevy_ecs::relationship::RelatedSpawner;

const FONT_PATH: &'static str = "fonts/FiraSans-Bold.ttf";
const MAX_PASSWORD_LENGTH: usize = 20;

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            InputDispatchPlugin,
            TabNavigationPlugin,
            CoreWidgetsPlugins,
        ))
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    let font = asset_server.load(FONT_PATH);

    // UI camera
    commands.spawn(Camera2d);

    let on_click = commands.register_system(
        |_: In<Activate>, mut query: Query<&mut TextInputPasswordMask>| {
            for mut password in query.iter_mut() {
                password.show_password = !password.show_password;
                println!("show = {}", password.show_password);
            }
        },
    );

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
            BorderColor::all(YELLOW.into()),
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
                                InputField {
                                    justify: Justify::Left,
                                },
                                TextFont { font, ..default() },
                                Prompt::new("enter a password"),
                                TextColor(Color::WHITE),
                                TabIndex(0),
                                TextInputValue::default(),
                                TextInputPasswordMask::default(),
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
                    children![(Text::new(format!("{MAX_PASSWORD_LENGTH} characters left.")),)]
                ))
            )),
        )],
    ));
}

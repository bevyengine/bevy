//! Demonstrates how to use `FixedNode` to lay out a UI node as a root node

use bevy::color::palettes::css::BLUE;
use bevy::color::palettes::css::RED;
use bevy::color::palettes::css::YELLOW;
use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);

    commands
        .spawn((
            Node {
                width: percent(100),
                height: percent(100),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(BLUE.into()),
            Pickable::IGNORE,
            children![(
                FixedNode,
                Node {
                    width: px(100),
                    height: px(100),
                    ..default()
                },
                BackgroundColor(YELLOW.into()),
            )],
        ))
        .observe(
            |over: On<Pointer<Over>>, mut colors: Query<&mut BackgroundColor>| {
                colors.get_mut(over.entity).unwrap().0 = RED.into();
            },
        )
        .observe(
            |over: On<Pointer<Leave>>, mut colors: Query<&mut BackgroundColor>| {
                colors.get_mut(over.entity).unwrap().0 = BLUE.into();
            },
        );
}

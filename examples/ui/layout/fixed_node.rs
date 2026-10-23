//! Demonstrates how to use `FixedNode` to lay out a UI node as a root node

use bevy::color::palettes::css::*;
use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);

    commands.spawn_scene(
        bsn! {
            Node {
                display: Display::Flex,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                width: percent(100),
                height: percent(100),
            }
            Pickable::IGNORE
            Children [
                Node {
                    width: percent(50),
                    height: percent(50),
                }
                BackgroundColor(LIGHT_GRAY)
                Children [
                    Node {
                        position_type: PositionType::Absolute,
                        top: px(-50),
                        left: px(-50),
                        width: px(100),
                        height: px(100),
                    }
                    BackgroundColor(RED)
                    --
                    Node {
                      position_type: PositionType::Absolute,
                       top: px(-50),
                       right: px(-50),
                       width: px(100),
                       height: px(100),
                    }
                    BackgroundColor(BLUE)
                    --
                    FixedNode
                    Node {
                        top: px(0),
                        left: px(0)
                        width: px(100),
                        height: px(100),
                    }
                    BackgroundColor(GREEN)
                    on(|over: On<PointerOver>, mut colors: Query<&mut BackgroundColor>| {
                        colors.get_mut(over.entity).unwrap().0 = RED.into();
                    })
                    on(|leave: On<PointerLeave>, mut colors: Query<&mut BackgroundColor>| {
                        colors.get_mut(leave.entity).unwrap().0 = BLUE.into();
                    })
                    --
                    FixedNode
                    Node {
                        position_type: PositionType::Absolute,
                        top: px(0),
                        right: px(0),
                        width: px(100),
                        height: px(100),
                    }
                    BackgroundColor(YELLOW)
                ]
            ]
        }
    );
}

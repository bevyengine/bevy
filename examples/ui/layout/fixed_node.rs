//! Demonstrates how to use `FixedNode` to lay out a UI node as a root node

use bevy::color::palettes::css::*;
use bevy::prelude::*;
use bevy::ui::debug::print_ui_layout_tree;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(Update, debug)
        .run();
}

fn debug(keyboard: Res<ButtonInput<KeyCode>>, world: &World) {
    if keyboard.just_pressed(KeyCode::Space) {
        print_ui_layout_tree(world);
    }
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);

    commands.spawn_scene(bsn! {
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
                #TopLeftAbsolute
                Node {
                    position_type: PositionType::Absolute,
                    top: px(-50),
                    left: px(-50),
                    width: px(100),
                    height: px(100),
                }
                BackgroundColor(RED)
                --
                #TopRightAbsolute
                Node {
                  position_type: PositionType::Absolute,
                   top: px(-50),
                   right: px(-50),
                   width: px(100),
                   height: px(100),
                }
                BackgroundColor(BLUE)
                --
                #TopLeftFixed
                FixedNode
                Node {
                    position_type: PositionType::Absolute,
                    top: px(0),
                    left: px(0),
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
                #TopRightFixed
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
    });
}

//! Ui scaling with overflow clipping.

use bevy::{input::mouse::MouseWheel, prelude::*};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, scene.spawn())
        .add_systems(Update, zoom)
        .run();
}

fn scene() -> impl SceneList {
    bsn_list![Camera2d, root()]
}

#[derive(Component, Clone, Default)]
struct Root;

fn root() -> impl Scene {
    bsn! {
        Root
        Node {
            width: percent(100),
            height: percent(100),
            display: Display::Flex,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            flex_direction: FlexDirection::Column
        }
        Children [
            Node {
                overflow: Overflow::clip(),
                width: px(150.),
            }
            Children [
                Node {
                    width: px(150.)
                }
                Text::new("Scroll to zoom.")
                BackgroundColor(Color::BLACK),

                Node
                Text::new("This should be clipped")
                BackgroundColor(Color::BLACK)
            ]
        ]
    }
}

fn zoom(
    mut root_transform: Single<&mut UiTransform, With<Root>>,
    mut scroll: MessageReader<MouseWheel>,
) {
    for event in scroll.read() {
        root_transform.scale *= 1.0 + event.y * 0.05;
    }
}

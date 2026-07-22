//! This example demonstrates the use of Ghost Nodes.
//!
//! UI layout will ignore ghost nodes, and treat their children as if they were direct descendants of the first non-ghost ancestor.
//!
//! # Warning
//!
//! This is an experimental feature, and should be used with caution,
//! especially in concert with 3rd party plugins or systems that may not be aware of ghost nodes.
//!
//! In order to use [`GhostNode`]s you must enable the `ghost_nodes` feature flag.

use bevy::{
    prelude::*,
    text::FontSourceTemplate,
    ui::experimental::GhostNode,
    ui_widgets::{Activate, Button},
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, scene.spawn())
        .run();
}

#[derive(Component, Clone, Default)]
struct Counter(i32);

fn scene() -> impl SceneList {
    bsn_list![Camera2d, ghost_root(), normal_root()]
}

/// Ghost UI root
fn ghost_root() -> impl Scene {
    bsn! {
        GhostNode
        Children [(
            Node
            Children [ label("This text node is rendered under a ghost root") ]
        )]
    }
}

/// Normal UI root
fn normal_root() -> impl Scene {
    bsn! {
        Node {
            width: percent(100),
            height: percent(100),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
        }
        Children [(
            Node Counter(0)
            Children [
                // Ghost children using a separate counter state.
                // These buttons are being treated as children of the layout parent
                // in the context of UI, but they share the ghost node's counter.
                (
                    GhostNode Counter(0)
                    Children [ button(), button() ]
                ),
                // A normal child using the layout parent counter
                button(),
            ]
        )]
    }
}

fn button() -> impl Scene {
    bsn! {
        Button
        // Bump the counter belonging to this button's parent, then refresh every
        // label under that parent. Buttons sharing a parent share a counter, so
        // pressing either ghost child updates both of their labels.
        on(|activate: On<Activate>,
            child_of_query: Query<&ChildOf>,
            children_query: Query<&Children>,
            mut counter_query: Query<&mut Counter>,
            mut text_query: Query<&mut Text>| {
            let Ok(parent) = child_of_query.get(activate.entity).map(ChildOf::parent) else {
                return;
            };
            let Ok(mut counter) = counter_query.get_mut(parent) else {
                return;
            };
            counter.0 += 1;
            let value = counter.0.to_string();

            let Ok(siblings) = children_query.get(parent) else {
                return;
            };
            for &sibling in siblings {
                let Ok(labels) = children_query.get(sibling) else {
                    continue;
                };
                for &label in labels {
                    if let Ok(mut text) = text_query.get_mut(label) {
                        **text = value.clone();
                    }
                }
            }
        })
        Node {
            width: px(150),
            height: px(65),
            border: px(5),
            // horizontally center child text
            justify_content: JustifyContent::Center,
            // vertically center child text
            align_items: AlignItems::Center,
            border_radius: BorderRadius::MAX,
        }
        BorderColor::from(Color::BLACK)
        BackgroundColor(Color::srgb(0.15, 0.15, 0.15))
        Children [ label("0") ]
    }
}

fn label(text: &str) -> impl Scene {
    bsn! {
        Text(text)
        TextFont {
            font: FontSourceTemplate::Handle("fonts/FiraSans-Bold.ttf"),
            font_size: px(33.0),
        }
        TextColor(Color::srgb(0.9, 0.9, 0.9))
    }
}

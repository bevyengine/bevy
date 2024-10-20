//! This example demonstrates the use of Ghost Nodes.
//!
//! UI layout will ignore ghost nodes, and treat their children as if they were direct descendants of the first non-ghost ancestor.
//!
//! # Warning
//!
//! This is an experimental feature, and should be used with caution,
//! especially in concert with 3rd party plugins or systems that may not be aware of ghost nodes.
//!
//! To add [`GhostNode`] components to entities, you must enable the `ghost_nodes` feature flag,
//! as they are otherwise unconstructable even though the type is defined.

use bevy::{prelude::*, ui::experimental::GhostNode, winit::WinitSettings};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .insert_resource(WinitSettings::desktop_app())
        .add_systems(Startup, setup)
        .add_systems(Update, button_system)
        .run();
}

#[derive(Component)]
struct Counter(i32);

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    let font_handle = asset_server.load("fonts/FiraSans-Bold.ttf");

    commands.spawn(Camera2d);

    // Ghost UI root
    commands
        .spawn(GhostNode::new())
        .with_children(|ghost_root| {
            ghost_root.spawn(Node::default()).with_child(create_label(
                "This text node is rendered under a ghost root",
                font_handle.clone(),
            ));
        });

    // Normal UI root
    commands
        .spawn(Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            ..default()
        })
        .with_children(|parent| {
            parent
                .spawn((Node::default(), Counter(0)))
                .with_children(|layout_parent| {
                    layout_parent
                        .spawn((GhostNode::new(), Counter(0)))
                        .with_children(|ghost_parent| {
                            // Ghost children using a separate counter state
                            // These buttons are being treated as children of layout_parent in the context of UI
                            ghost_parent
                                .spawn(create_button())
                                .with_child(create_label("0", font_handle.clone()));
                            ghost_parent
                                .spawn(create_button())
                                .with_child(create_label("0", font_handle.clone()));
                        });

                    // A normal child using the layout parent counter
                    layout_parent
                        .spawn(create_button())
                        .with_child(create_label("0", font_handle.clone()));
                });
        });
}

fn create_button() -> impl Bundle {
    (
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
        BackgroundColor(Color::srgb(0.15, 0.15, 0.15)),
    )
}

fn create_label(text: &str, font: Handle<Font>) -> (Text, TextFont, TextColor) {
    (
        Text::new(text),
        TextFont {
            font,
            font_size: 33.0,
            ..default()
        },
        TextColor(Color::srgb(0.9, 0.9, 0.9)),
    )
}

fn button_system(
    mut interaction_query: Query<(&Interaction, &Parent), (Changed<Interaction>, With<Button>)>,
    labels_query: Query<(&Children, &Parent), With<Button>>,
    mut text_query: Query<&mut Text>,
    mut counter_query: Query<&mut Counter>,
) {
    // Update parent counter on click
    for (interaction, parent) in &mut interaction_query {
        if matches!(interaction, Interaction::Pressed) {
            let mut counter = counter_query.get_mut(parent.get()).unwrap();
            counter.0 += 1;
        }
    }

    // Update button labels to match their parent counter
    for (children, parent) in &labels_query {
        let counter = counter_query.get(parent.get()).unwrap();
        let mut text = text_query.get_mut(children[0]).unwrap();

        **text = counter.0.to_string();
    }
}

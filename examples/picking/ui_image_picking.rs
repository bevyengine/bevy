//! Demonstrates alpha-based picking for UI image nodes.
//!
//! By default the UI picking backend only reports an [`ImageNode`] as hit when
//! the pointer is over a pixel that is more opaque than the threshold in
//! [`UiPickingSettings`]. Hovering the transparent regions of an image (such as
//! the area around the Bevy bird) therefore does nothing.
//!
//! This can be changed globally through [`UiPickingSettings::picking_mode`], or
//! per-node by adding the [`UiPickingMode`] component. The two birds below
//! share the same texture but pick differently:
//!
//! - The left one uses the default alpha threshold.
//! - The right one falls back to bounding-box picking.

use core::fmt::Debug;

use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(Camera2d);

    let bird = asset_server.load("branding/bevy_bird_dark.png");

    commands
        .spawn(Node {
            width: percent(100),
            height: percent(100),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            row_gap: px(40),
            ..default()
        })
        .with_children(|parent| {
            parent.spawn(Text::new(
                "Hover the birds. The dark panel marks each image's rectangle.\n\
                 Left: alpha threshold picking mode: the transparent area can not be picked.\n\
                 Right: bounding box picking mode: the whole rectangle can be picked.",
            ));

            parent
                .spawn(Node {
                    column_gap: px(80),
                    ..default()
                })
                .with_children(|parent| {
                    spawn_bird(parent, bird.clone(), "Alpha threshold", None);
                    spawn_bird(
                        parent,
                        bird.clone(),
                        "Bounding box",
                        Some(UiPickingMode::BoundingBox),
                    );
                });
        });
}

fn spawn_bird(
    parent: &mut ChildSpawnerCommands,
    image: Handle<Image>,
    label: &str,
    picking_mode: Option<UiPickingMode>,
) {
    parent
        .spawn(Node {
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            row_gap: px(10),
            ..default()
        })
        .with_children(|parent| {
            parent.spawn(Text::new(label));

            let mut bird = parent.spawn((
                ImageNode::new(image),
                // A backing panel that fills the node's rectangle, so the
                // transparent parts of the image are visible against it.
                BackgroundColor(Color::srgb(0.15, 0.15, 0.15)),
            ));

            if let Some(picking_mode) = picking_mode {
                bird.insert(picking_mode);
            }

            bird.observe(recolor_on::<Pointer<Over>>(Color::srgb(0.0, 1.0, 1.0)))
                .observe(recolor_on::<Pointer<Out>>(Color::WHITE));
        });
}

/// An observer that tints the target image node.
fn recolor_on<E: EntityEvent + Debug + Clone + Reflect>(
    color: Color,
) -> impl Fn(On<E>, Query<&mut ImageNode>) {
    move |ev, mut images| {
        let Ok(mut image) = images.get_mut(ev.event_target()) else {
            return;
        };
        image.color = color;
    }
}

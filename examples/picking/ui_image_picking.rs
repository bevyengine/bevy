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
        .add_systems(Startup, scene.spawn())
        .run();
}

fn scene() -> impl SceneList {
    bsn_list![Camera2d, ui()]
}

fn ui() -> impl Scene {
    bsn! {
        Node {
            width: percent(100),
            height: percent(100),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            row_gap: px(40),
        }
        Children [
            Text(
                "Hover the birds. The dark panel marks each image's rectangle.\n\
                 Left: alpha threshold picking mode: the transparent area can not be picked.\n\
                 Right: bounding box picking mode: the whole rectangle can be picked."
            ),
            (
                Node {
                    column_gap: px(80),
                }
                Children [
                    bird("Alpha threshold", None),
                    bird("Bounding box", Some(UiPickingMode::BoundingBox)),
                ]
            )
        ]
    }
}

fn bird(label: &str, picking_mode: Option<UiPickingMode>) -> impl Scene {
    // Optionally override the picking mode for this node. When `None`, the node
    // falls back to the global default in `UiPickingSettings`.
    let picking_mode: Box<dyn Scene> = match picking_mode {
        Some(picking_mode) => Box::new(bsn! { template(move |_| Ok(picking_mode)) }),
        None => Box::new(()),
    };

    bsn! {
        Node {
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            row_gap: px(10),
        }
        Children [
            Text(label),
            (
                // A backing panel that fills the node's rectangle, so the
                // transparent parts of the image are visible against it.
                ImageNode { image: "branding/bevy_bird_dark.png" }
                BackgroundColor(Color::srgb(0.15, 0.15, 0.15))
                {picking_mode}
                on(recolor::<Pointer<Over>>(Color::srgb(0.0, 1.0, 1.0)))
                on(recolor::<Pointer<Out>>(Color::WHITE))
            )
        ]
    }
}

/// An observer that tints the target image node.
fn recolor<E: EntityEvent + Debug + Clone + Reflect>(
    color: Color,
) -> impl Fn(On<E>, Query<&mut ImageNode>) + Clone {
    move |ev, mut images| {
        let Ok(mut image) = images.get_mut(ev.event_target()) else {
            return;
        };
        image.color = color;
    }
}

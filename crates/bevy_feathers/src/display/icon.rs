//! BSN Scene for loading images and displaying them as [`ImageNode`]s.
use bevy_scene::{bsn, Scene};
use bevy_ui::{px, widget::ImageNode, Node};

/// Template which displays an icon.
pub fn icon(image: &'static str) -> impl Scene {
    bsn! {
        Node {
            height: px(14),
        }
        ImageNode {
            image: image
        }
    }
}

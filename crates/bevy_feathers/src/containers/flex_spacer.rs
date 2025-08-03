use bevy_scene2::{bsn, Scene};
use bevy_ui::Node;

/// An invisible UI node that takes up space, and which has a positive `flex_grow` setting.
pub fn flex_spacer() -> impl Scene {
    bsn! {
        Node {
            flex_grow: 1.0,
        }
    }
}

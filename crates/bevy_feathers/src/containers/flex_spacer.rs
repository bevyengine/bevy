use bevy_scene::{bsn, Scene};
use bevy_ui::Node;

/// An invisible UI node that takes up space, and which has a positive `flex_grow` setting.
/// This is normally used within containers to provide a gap.
pub fn flex_spacer() -> impl Scene {
    bsn! {
        Node {
            flex_grow: 1.0,
        }
    }
}

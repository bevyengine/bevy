/// Helpers to create a [`Node`] appropriate for the outer main UI node as a `Scene`.
use bevy::prelude::*;

/// Returns a [`Node`] appropriate for the outer main UI node as a `Scene`.
///
/// This UI is in the bottom left corner and has flex column support
pub fn bottom_left_scene() -> impl Scene {
    bsn! {
        Node {
            flex_direction: FlexDirection::Column,
            position_type: PositionType::Absolute,
            row_gap: px(6),
            left: px(10),
            bottom: px(10),
        }
    }
}

/// Returns a [`Node`] appropriate for the outer main UI node as a `Scene`.
///
/// This UI is in the top left corner and has flex column support
pub fn top_left_scene() -> impl Scene {
    bsn! {
        Node {
            flex_direction: FlexDirection::Column,
            position_type: PositionType::Absolute,
            row_gap: px(6),
            left: px(10),
            top: px(10),
        }
    }
}

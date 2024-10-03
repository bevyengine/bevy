use crate::ResolvedBorderRadius;
use bevy_ecs::prelude::*;
use bevy_math::{Rect, Vec2};
use bevy_reflect::{std_traits::ReflectDefault, Reflect};

#[cfg(feature = "serialize")]
use bevy_reflect::{ReflectDeserialize, ReflectSerialize};

/// A component storing the position of the mouse relative to the node, (0., 0.) being the top-left corner and (1., 1.) being the bottom-right
/// If the mouse is not over the node, the value will go beyond the range of (0., 0.) to (1., 1.)
///
/// The component is updated when it is in the same entity with [`Node`].
#[derive(Component, Copy, Clone, Default, PartialEq, Debug, Reflect)]
#[reflect(Component, Default, PartialEq, Debug)]
#[cfg_attr(
    feature = "serialize",
    derive(serde::Serialize, serde::Deserialize),
    reflect(Serialize, Deserialize)
)]
pub struct RelativeCursorPosition {
    /// Visible area of the Node relative to the size of the entire Node.
    pub normalized_visible_node_rect: Rect,
    /// Cursor position relative to the size and position of the Node.
    /// A None value indicates that the cursor position is unknown.
    pub normalized: Option<Vec2>,
}

impl RelativeCursorPosition {
    /// A helper function to check if the mouse is over the node
    pub fn mouse_over(&self) -> bool {
        self.normalized
            .map(|position| self.normalized_visible_node_rect.contains(position))
            .unwrap_or(false)
    }
}

/// Describes whether the node should block interactions with lower nodes
#[derive(Component, Copy, Clone, Eq, PartialEq, Debug, Reflect)]
#[reflect(Component, Default, PartialEq, Debug)]
#[cfg_attr(
    feature = "serialize",
    derive(serde::Serialize, serde::Deserialize),
    reflect(Serialize, Deserialize)
)]
pub enum FocusPolicy {
    /// Blocks interaction
    Block,
    /// Lets interaction pass through
    Pass,
}

impl FocusPolicy {
    const DEFAULT: Self = Self::Pass;
}

impl Default for FocusPolicy {
    fn default() -> Self {
        Self::DEFAULT
    }
}

// Returns true if `point` (relative to the rectangle's center) is within the bounds of a rounded rectangle with
// the given size and border radius.
//
// Matches the sdf function in `ui.wgsl` that is used by the UI renderer to draw rounded rectangles.
pub(crate) fn pick_rounded_rect(
    point: Vec2,
    size: Vec2,
    border_radius: ResolvedBorderRadius,
) -> bool {
    let [top, bottom] = if point.x < 0. {
        [border_radius.top_left, border_radius.bottom_left]
    } else {
        [border_radius.top_right, border_radius.bottom_right]
    };
    let r = if point.y < 0. { top } else { bottom };

    let corner_to_point = point.abs() - 0.5 * size;
    let q = corner_to_point + r;
    let l = q.max(Vec2::ZERO).length();
    let m = q.max_element().min(0.);
    l + m - r < 0.
}

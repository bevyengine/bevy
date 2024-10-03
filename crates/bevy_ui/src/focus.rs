use crate::ResolvedBorderRadius;
use bevy_math::Vec2;

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

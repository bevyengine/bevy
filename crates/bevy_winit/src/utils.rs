use bevy_log::warn;
use bevy_window::WindowResizeConstraints;

pub(crate) fn check_resize_constraints(
    WindowResizeConstraints {
        mut min_width,
        mut min_height,
        mut max_width,
        mut max_height,
    }: WindowResizeConstraints,
) -> WindowResizeConstraints {
    min_width = min_width.max(1.);
    min_height = min_height.max(1.);
    if max_width < min_width {
        warn!(
            "The given maximum width {} is smaller than the minimum width {}",
            max_width, min_width
        );
        max_width = min_width;
    }
    if max_height < min_height {
        warn!(
            "The given maximum height {} is smaller than the minimum height {}",
            max_height, min_height
        );
        max_height = min_height;
    }
    WindowResizeConstraints {
        min_width,
        min_height,
        max_width,
        max_height,
    }
}

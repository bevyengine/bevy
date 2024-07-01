use crate::layout::ui_surface::UiSurface;

/// Prints a debug representation of the computed layout of the UI layout tree for each camera.
#[deprecated(
    since = "0.13.3",
    note = "use `ui_surface.ui_layout_tree_debug_string()` instead"
)]
pub fn print_ui_layout_tree(ui_surface: &UiSurface) {
    let debug_layout_tree = match ui_surface.ui_layout_tree_debug_string() {
        Ok(output) => output,
        Err(err) => {
            bevy_utils::tracing::error!("Failed to print ui layout tree: {err}");
            return;
        }
    };
    bevy_utils::tracing::info!("{debug_layout_tree}");
}

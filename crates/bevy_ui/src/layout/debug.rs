use core::fmt::Write;

use crate::layout::ui_surface::UiSurface;

/// Prints the latest computed UI layouts stored in [`UiSurface`].
pub fn print_ui_layout_tree(ui_surface: &UiSurface) {
    let mut out = String::new();

    for (entity, layout) in &ui_surface.layouts {
        let layout = layout.rounded;
        writeln!(
            out,
            "[x: {x:<4} y: {y:<4} width: {width:<4} height: {height:<4}] ({entity})",
            x = layout.location.x,
            y = layout.location.y,
            width = layout.size.width,
            height = layout.size.height,
        )
        .ok();
    }

    tracing::info!("Computed UI layouts\n{out}");
}

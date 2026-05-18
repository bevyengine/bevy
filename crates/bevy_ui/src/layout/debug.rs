use core::fmt::Write;

use bevy_ecs::{entity::Entity, system::Query};

use crate::layout::ui_surface::ComputedLayout;

/// Prints the latest computed UI layouts.
pub fn print_ui_layout_tree(layout_query: Query<(Entity, &ComputedLayout)>) {
    let mut out = String::new();

    for (entity, computed_layout) in &layout_query {
        if let Some((layout, _)) = computed_layout.get(true) {
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
    }

    tracing::info!("Computed UI layouts\n{out}");
}

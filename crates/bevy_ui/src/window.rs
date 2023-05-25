use crate::LayoutContext;
use crate::UiScale;
use bevy_ecs::prelude::Entity;
use bevy_ecs::query::With;
use bevy_ecs::system::Commands;
use bevy_ecs::system::Query;
use bevy_ecs::system::Res;
use bevy_math::Vec2;
use bevy_window::PrimaryWindow;
use bevy_window::Window;

/// Checks for window and scale factor changes and updates the [`LayoutContext`].
pub fn ui_windows_system(
    mut commands: Commands,
    ui_scale: Res<UiScale>,
    mut primary_window: Query<(Entity, &Window, Option<&mut LayoutContext>), With<PrimaryWindow>>,
) {
    let (primary_window_entity, logical_to_physical_factor, physical_size, maybe_layout_context) =
        if let Ok((entity, primary_window, maybe_layout_context)) = primary_window.get_single_mut()
        {
            (
                entity,
                primary_window.resolution.scale_factor(),
                Vec2::new(
                    primary_window.resolution.physical_width() as f32,
                    primary_window.resolution.physical_height() as f32,
                ),
                maybe_layout_context,
            )
        } else {
            return;
        };

    let scale_factor = logical_to_physical_factor * ui_scale.scale;
    let physical_to_logical_factor = logical_to_physical_factor.recip();
    let new_layout_context = LayoutContext {
        root_node_size: physical_size,
        combined_scale_factor: scale_factor,
        layout_to_logical_factor: physical_to_logical_factor,
    };

    if let Some(mut layout_context) = maybe_layout_context {
        if approx::relative_ne!(
            layout_context.root_node_size.x,
            new_layout_context.root_node_size.x
        ) || approx::relative_ne!(
            layout_context.root_node_size.y,
            new_layout_context.root_node_size.y
        ) || approx::relative_ne!(
            layout_context.combined_scale_factor,
            new_layout_context.combined_scale_factor
        ) {
            *layout_context = new_layout_context;
        }
    } else {
        commands
            .entity(primary_window_entity)
            .insert(new_layout_context);
    }
}

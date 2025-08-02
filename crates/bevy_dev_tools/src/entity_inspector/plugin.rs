//! Plugin implementation for the Entity Inspector.

use bevy_app::{App, Plugin, Update, MainScheduleOrder, Last};
use bevy_ecs::schedule::IntoScheduleConfigs;
use bevy_state::prelude::*;

use super::{systems, InspectorConfig, InspectorData, InspectorState, InspectorLast};

/// Plugin that provides entity/component inspection capabilities.
///
/// Add this plugin to your app to enable the entity inspector:
/// ```rust
/// app.add_plugins(EntityInspectorPlugin);
/// ```
///
/// The inspector can be toggled with F12 by default.
#[derive(Default, Debug)]
pub struct EntityInspectorPlugin;

impl Plugin for EntityInspectorPlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<InspectorState>()
            .init_resource::<InspectorConfig>()
            .init_resource::<InspectorData>()
            .add_systems(
                Update,
                (
                    systems::handle_toggle_input,
                    systems::manage_inspector_window,
                    systems::populate_entity_list.after(systems::manage_inspector_window),
                    systems::handle_entity_selection,
                    systems::handle_collapsible_sections,
                    systems::debug_entity_count,
                )
            );
        
        // Create our own schedule to avoid World borrow conflicts, like bevy_remote does
        app.init_schedule(InspectorLast)
            .world_mut()
            .resource_mut::<MainScheduleOrder>()
            .insert_after(Last, InspectorLast);
        
        app.add_systems(
            InspectorLast,
            systems::process_inspector_updates,
        );
    }
}
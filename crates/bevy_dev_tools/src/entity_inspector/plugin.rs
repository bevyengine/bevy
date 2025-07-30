//! Plugin implementation for the Entity Inspector.

use bevy_app::{App, Plugin, Update};
use bevy_ecs::schedule::IntoScheduleConfigs;
use bevy_state::prelude::*;

use super::{systems, InspectorConfig, InspectorData, InspectorState};

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
                    systems::display_entity_components,
                    systems::debug_entity_count,
                )
            )
            .add_systems(
                Update,
                systems::update_component_values_live, 
            );
    }
}
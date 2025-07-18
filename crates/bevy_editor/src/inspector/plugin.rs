use bevy::prelude::*;

use super::{
    events::InspectorEvent,
    remote::{RemoteEntities, poll_remote_entities, fetch_entity_components},
    selection::{reset_selected_entity_if_entity_despawned, SelectedEntity},
    tree::{TreeNodeInteraction, TreeState},
    ui::{setup_inspector, update_entity_tree, handle_tree_interactions, update_component_details},
};

/// A plugin that provides an inspector UI.
pub struct InspectorPlugin;

impl Plugin for InspectorPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SelectedEntity>()
            .register_type::<SelectedEntity>()
            .init_resource::<RemoteEntities>()
            .init_resource::<TreeState>()
            .add_event::<InspectorEvent>()
            .add_event::<TreeNodeInteraction>()
            .add_systems(Startup, setup_inspector)
            .add_systems(
                Update,
                (
                    poll_remote_entities,
                    fetch_entity_components,
                    update_entity_tree,
                    handle_tree_interactions,
                    update_component_details,
                    reset_selected_entity_if_entity_despawned,
                ),
            );
    }
}

//! Connection management and update systems

use bevy_ecs::prelude::*;
use bevy_time::Time;
use crate::inspector::remote::types::{EditorState, RemoteConnection, ConnectionStatus, EntitiesFetched};
use crate::inspector::remote::client;
use tracing::{info, warn};

/// Update remote connection and fetch data periodically
pub fn update_remote_connection(
    time: Res<Time>,
    mut remote_conn: ResMut<RemoteConnection>,
    mut editor_state: ResMut<EditorState>,
    mut commands: Commands,
) {
    let current_time = time.elapsed_secs_f64();
    
    if current_time - remote_conn.last_fetch >= remote_conn.fetch_interval {
        remote_conn.last_fetch = current_time;
        
        // Update status to show we're attempting to connect
        if editor_state.connection_status == ConnectionStatus::Disconnected {
            editor_state.connection_status = ConnectionStatus::Connecting;
        }
        
        // Try to fetch entities using the remote client framework
        match client::try_fetch_entities(&remote_conn.base_url) {
            Ok(entities) => {
                info!("Successfully fetched {} entities from remote server", entities.len());
                commands.trigger(EntitiesFetched { entities });
                editor_state.connection_status = ConnectionStatus::Connected;
            }
            Err(err) => {
                warn!("Failed to fetch entities: {}", err);
                // Only set error status if we're not already showing disconnected
                if editor_state.connection_status != ConnectionStatus::Disconnected {
                    editor_state.connection_status = ConnectionStatus::Error(err);
                }
                // Clear entities when connection fails
                if !editor_state.entities.is_empty() {
                    editor_state.entities.clear();
                    editor_state.selected_entity_id = None;
                    editor_state.show_components = false;
                }
            }
        }
    }
}

/// Handle entities fetched event
pub fn handle_entities_fetched(
    trigger: On<EntitiesFetched>,
    mut editor_state: ResMut<EditorState>,
) {
    editor_state.entities = trigger.event().entities.clone();
    editor_state.connection_status = ConnectionStatus::Connected;
}

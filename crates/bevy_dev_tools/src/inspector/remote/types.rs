//! Remote connection types and events

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

/// Remote entity representation from bevy_remote
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteEntity {
    pub id: u32,
    pub components: Vec<String>, // Display names (cleaned)
    pub full_component_names: Vec<String>, // Full type names for API calls
}

/// Connection status for remote bevy_remote server
#[derive(Debug, Clone, PartialEq)]
pub enum ConnectionStatus {
    Disconnected,
    Connecting,
    Connected,
    Error(String),
}

impl Default for ConnectionStatus {
    fn default() -> Self {
        ConnectionStatus::Disconnected
    }
}

/// Remote connection configuration
#[derive(Resource)]
pub struct RemoteConnection {
    pub base_url: String,
    pub last_fetch: f64,
    pub fetch_interval: f64,
}

impl Default for RemoteConnection {
    fn default() -> Self {
        Self {
            base_url: "http://127.0.0.1:15702".to_string(),
            last_fetch: 0.0,
            fetch_interval: 1.0, // Fetch every second
        }
    }
}

/// Event fired when entities are fetched from remote server
#[derive(Event, Clone)]
pub struct EntitiesFetched {
    pub entities: Vec<RemoteEntity>,
}

/// Event fired when component data is fetched for a specific entity
#[derive(Event, Clone)] 
pub struct ComponentDataFetched {
    pub entity_id: u32,
    pub component_data: String,
}

/// Global editor state
#[derive(Resource, Default)]
pub struct EditorState {
    pub selected_entity_id: Option<u32>,
    pub entities: Vec<RemoteEntity>,
    pub show_components: bool,
    pub connection_status: ConnectionStatus,
}

/// Component display state for tracking expanded/collapsed items
#[derive(Resource, Default)]
pub struct ComponentDisplayState {
    pub expanded_paths: std::collections::HashSet<String>,
}

/// Parsed component field for structured display
#[derive(Debug, Clone)]
pub struct ComponentField {
    pub name: String,
    pub field_type: String,
    pub value: serde_json::Value,
    pub is_expandable: bool,
}

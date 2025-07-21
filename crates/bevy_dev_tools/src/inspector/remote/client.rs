//! HTTP client for bevy_remote protocol

use crate::inspector::remote::types::RemoteEntity;

// Temporarily commented out until bevy::remote is available in bevy_dev_tools
// use bevy::remote::{
//     builtin_methods::{
//         BrpQuery, BrpQueryFilter, BrpQueryParams, ComponentSelector, BRP_QUERY_METHOD,
//     },
//     BrpRequest,
// };

/// Attempts to connect to a bevy_remote server and fetch entity data
pub fn try_fetch_entities(_base_url: &str) -> Result<Vec<RemoteEntity>, String> {
    // Temporarily disabled until bevy::remote is available
    Err("Remote functionality temporarily disabled".to_string())
}

/// Fetch component data for a specific entity
pub fn try_fetch_component_data(_base_url: &str, _entity_id: u32) -> Result<String, String> {
    // Temporarily disabled until bevy::remote is available
    Err("Remote functionality temporarily disabled".to_string())
}

/// Parse entity data from bevy_remote response
pub fn parse_brp_entities(_result: serde_json::Value) -> Result<Vec<RemoteEntity>, String> {
    // Temporarily disabled until bevy::remote is available
    Err("Remote functionality temporarily disabled".to_string())
}

/// Try to parse a simple component name response format
pub fn parse_simple_component_data(_response: &str) -> Result<String, String> {
    // Temporarily disabled until bevy::remote is available
    Err("Remote functionality temporarily disabled".to_string())
}

/// Test connection to remote server  
pub fn test_connection(_base_url: &str) -> Result<String, String> {
    // Temporarily disabled until bevy::remote is available
    Err("Remote functionality temporarily disabled".to_string())
}

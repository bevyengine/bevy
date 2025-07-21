//! HTTP client for bevy_remote protocol

use crate::remote::types::RemoteEntity;
use bevy::remote::{
    builtin_methods::{
        BrpQuery, BrpQueryFilter, BrpQueryParams, ComponentSelector, BRP_QUERY_METHOD,
    },
    BrpRequest,
};

/// Attempts to connect to a bevy_remote server and fetch entity data
pub fn try_fetch_entities(base_url: &str) -> Result<Vec<RemoteEntity>, String> {
    // Create a query to get all entities with their components
    let query_request = BrpRequest {
        jsonrpc: "2.0".to_string(),
        method: BRP_QUERY_METHOD.to_string(),
        id: Some(serde_json::to_value(1).map_err(|e| format!("JSON error: {}", e))?),
        params: Some(
            serde_json::to_value(BrpQueryParams {
                data: BrpQuery {
                    components: Vec::default(), // Get all components
                    option: ComponentSelector::All,
                    has: Vec::default(),
                },
                strict: false,
                filter: BrpQueryFilter::default(),
            })
            .map_err(|e| format!("Failed to serialize query params: {}", e))?,
        ),
    };
    
    // Make the HTTP request
    let response = ureq::post(base_url)
        .timeout(std::time::Duration::from_secs(2))
        .send_json(&query_request)
        .map_err(|e| format!("HTTP request failed: {}", e))?;
    
    // Parse the response as JSON first
    let json_response: serde_json::Value = response
        .into_json()
        .map_err(|e| format!("Failed to parse response: {}", e))?;
    
    // Check if we have an error or result
    if let Some(error) = json_response.get("error") {
        return Err(format!("Server error: {}", error));
    }
    
    if let Some(result) = json_response.get("result") {
        parse_brp_entities(result.clone())
    } else {
        Err("No result or error in response".to_string())
    }
}

/// Fetch component data for a specific entity
pub fn try_fetch_component_data(base_url: &str, entity_id: u32) -> Result<String, String> {
    // Create a get request for specific entity
    let get_request = BrpRequest {
        jsonrpc: "2.0".to_string(),
        method: "bevy/get".to_string(),
        id: Some(serde_json::to_value(2).map_err(|e| format!("JSON error: {}", e))?),
        params: Some(
            serde_json::json!({
                "entity": entity_id as u64,
                "components": [] // Get all components for this entity
            })
        ),
    };
    
    let response = ureq::post(base_url)
        .timeout(std::time::Duration::from_secs(2))
        .send_json(&get_request)
        .map_err(|e| format!("HTTP request failed: {}", e))?;
    
    let json_response: serde_json::Value = response
        .into_json()
        .map_err(|e| format!("Failed to parse response: {}", e))?;
    
    // Check if we have an error or result
    if let Some(error) = json_response.get("error") {
        return Err(format!("Server error: {}", error));
    }
    
    if let Some(result) = json_response.get("result") {
        Ok(serde_json::to_string_pretty(result)
            .unwrap_or_else(|_| "Failed to format component data".to_string()))
    } else {
        Err("No result or error in response".to_string())
    }
}

/// Fetch component data for a specific entity with explicit component names
pub fn try_fetch_component_data_with_names(
    base_url: &str, 
    entity_id: u32, 
    component_names: Vec<String>
) -> Result<String, String> {
    // Create a get request for specific entity with component names
    let get_request = BrpRequest {
        jsonrpc: "2.0".to_string(),
        method: "bevy/get".to_string(),
        id: Some(serde_json::to_value(2).map_err(|e| format!("JSON error: {}", e))?),
        params: Some(
            serde_json::json!({
                "entity": entity_id as u64,
                "components": component_names
            })
        ),
    };
    
    let response = ureq::post(base_url)
        .timeout(std::time::Duration::from_secs(2))
        .send_json(&get_request)
        .map_err(|e| format!("HTTP request failed: {}", e))?;
    
    let json_response: serde_json::Value = response
        .into_json()
        .map_err(|e| format!("Failed to parse response: {}", e))?;
    
    // Check if we have an error or result
    if let Some(error) = json_response.get("error") {
        return Err(format!("Server error: {}", error));
    }
    
    if let Some(result) = json_response.get("result") {
        Ok(serde_json::to_string_pretty(result)
            .unwrap_or_else(|_| "Failed to format component data".to_string()))
    } else {
        Err("No result or error in response".to_string())
    }
}

/// Parse BRP query response into our RemoteEntity format
fn parse_brp_entities(result: serde_json::Value) -> Result<Vec<RemoteEntity>, String> {
    let mut entities = Vec::new();
    
    if let Some(entity_array) = result.as_array() {
        for entity_obj in entity_array {
            if let Some(entity_data) = entity_obj.as_object() {
                // Get entity ID
                let entity_id = entity_data
                    .get("entity")
                    .and_then(|v| v.as_u64())
                    .ok_or("Missing or invalid entity ID")?;
                
                // Extract component names
                let mut components = Vec::new();
                let mut full_component_names = Vec::new();
                if let Some(components_obj) = entity_data.get("components").and_then(|v| v.as_object()) {
                    for component_name in components_obj.keys() {
                        // Store the full name for API calls
                        full_component_names.push(component_name.clone());
                        // Clean up component names (remove module paths for readability)
                        let clean_name = component_name
                            .split("::")
                            .last()
                            .unwrap_or(component_name)
                            .to_string();
                        components.push(clean_name);
                    }
                }
                
                entities.push(RemoteEntity {
                    id: entity_id as u32,
                    components,
                    full_component_names,
                });
            }
        }
    } else {
        return Err("Expected array of entities in response".to_string());
    }
    
    Ok(entities)
}

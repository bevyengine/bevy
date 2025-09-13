//! Integration tests for the bevy_remote_inspector

use bevy_ecs::prelude::*;
use bevy_app::App;
use bevy_remote_inspector::http_client::{HttpRemoteClient, HttpRemoteConfig, RemoteEntity};
use serde_json::{json, Value};
use std::collections::HashMap;

/// Create a mock HTTP client with test data for integration testing
fn create_mock_client_with_test_data() -> HttpRemoteClient {
    let config = HttpRemoteConfig::default();
    let mut client = HttpRemoteClient::new(&config);
    
    // Add mock entities for testing
    let mut entities = HashMap::new();
    
    // Entity 1: Player with Transform and Player components
    let mut player_components = HashMap::new();
    player_components.insert(
        "bevy_transform::components::transform::Transform".to_string(),
        json!({
            "translation": { "x": 0.0, "y": 0.0, "z": 0.0 },
            "rotation": { "x": 0.0, "y": 0.0, "z": 0.0, "w": 1.0 },
            "scale": { "x": 1.0, "y": 1.0, "z": 1.0 }
        })
    );
    player_components.insert(
        "Player".to_string(),
        json!({
            "speed": 5.0,
            "health": 100
        })
    );
    
    entities.insert(1, RemoteEntity {
        id: 1,
        name: Some("Player".to_string()),
        components: player_components,
    });
    
    // Entity 2: Enemy with Transform and Enemy components
    let mut enemy_components = HashMap::new();
    enemy_components.insert(
        "bevy_transform::components::transform::Transform".to_string(),
        json!({
            "translation": { "x": 10.0, "y": 0.0, "z": 0.0 },
            "rotation": { "x": 0.0, "y": 0.0, "z": 0.0, "w": 1.0 },
            "scale": { "x": 1.0, "y": 1.0, "z": 1.0 }
        })
    );
    enemy_components.insert(
        "Enemy".to_string(),
        json!({
            "damage": 25,
            "health": 75
        })
    );
    
    entities.insert(2, RemoteEntity {
        id: 2,
        name: Some("Enemy".to_string()),
        components: enemy_components,
    });
    
    // Entity 3: Camera with Transform and Camera components
    let mut camera_components = HashMap::new();
    camera_components.insert(
        "bevy_transform::components::transform::Transform".to_string(),
        json!({
            "translation": { "x": 0.0, "y": 0.0, "z": 5.0 },
            "rotation": { "x": 0.0, "y": 0.0, "z": 0.0, "w": 1.0 },
            "scale": { "x": 1.0, "y": 1.0, "z": 1.0 }
        })
    );
    camera_components.insert(
        "bevy_render::camera::camera::Camera".to_string(),
        json!({
            "projection": "perspective",
            "viewport": null
        })
    );
    
    entities.insert(3, RemoteEntity {
        id: 3,
        name: Some("MainCamera".to_string()),
        components: camera_components,
    });
    
    // Entity 4: Unnamed entity with just Transform
    let mut unnamed_components = HashMap::new();
    unnamed_components.insert(
        "bevy_transform::components::transform::Transform".to_string(),
        json!({
            "translation": { "x": -5.0, "y": 2.0, "z": 0.0 },
            "rotation": { "x": 0.0, "y": 0.0, "z": 0.0, "w": 1.0 },
            "scale": { "x": 1.0, "y": 1.0, "z": 1.0 }
        })
    );
    
    entities.insert(4, RemoteEntity {
        id: 4,
        name: None,
        components: unnamed_components,
    });
    
    // Set the entities in the client
    client.entities = entities;
    client.is_connected = true; // Mark as connected for testing
    
    client
}

#[test]
fn test_mock_client_creation() {
    let client = create_mock_client_with_test_data();
    
    assert!(client.is_connected);
    assert_eq!(client.entities.len(), 4);
    assert!(client.entities.contains_key(&1));
    assert!(client.entities.contains_key(&2));
    assert!(client.entities.contains_key(&3));
    assert!(client.entities.contains_key(&4));
}

#[test]
fn test_entity_data_structure() {
    let client = create_mock_client_with_test_data();
    
    // Test Player entity
    let player = client.get_entity(1).expect("Player entity should exist");
    assert_eq!(player.id, 1);
    assert_eq!(player.name, Some("Player".to_string()));
    assert!(player.components.contains_key("bevy_transform::components::transform::Transform"));
    assert!(player.components.contains_key("Player"));
    
    // Test Enemy entity
    let enemy = client.get_entity(2).expect("Enemy entity should exist");
    assert_eq!(enemy.id, 2);
    assert_eq!(enemy.name, Some("Enemy".to_string()));
    assert!(enemy.components.contains_key("Enemy"));
    
    // Test Camera entity
    let camera = client.get_entity(3).expect("Camera entity should exist");
    assert_eq!(camera.id, 3);
    assert_eq!(camera.name, Some("MainCamera".to_string()));
    assert!(camera.components.contains_key("bevy_render::camera::camera::Camera"));
    
    // Test unnamed entity
    let unnamed = client.get_entity(4).expect("Unnamed entity should exist");
    assert_eq!(unnamed.id, 4);
    assert_eq!(unnamed.name, None);
    assert_eq!(unnamed.components.len(), 1); // Only Transform
}

#[test]
fn test_component_data_parsing() {
    let client = create_mock_client_with_test_data();
    
    let player = client.get_entity(1).expect("Player entity should exist");
    
    // Test Transform component parsing
    if let Some(transform) = player.components.get("bevy_transform::components::transform::Transform") {
        let translation = &transform["translation"];
        assert_eq!(translation["x"], 0.0);
        assert_eq!(translation["y"], 0.0);
        assert_eq!(translation["z"], 0.0);
    } else {
        panic!("Transform component should exist");
    }
    
    // Test Player component parsing
    if let Some(player_comp) = player.components.get("Player") {
        assert_eq!(player_comp["speed"], 5.0);
        assert_eq!(player_comp["health"], 100);
    } else {
        panic!("Player component should exist");
    }
}

#[test]
fn test_entity_ids_retrieval() {
    let client = create_mock_client_with_test_data();
    
    let entity_ids = client.get_entity_ids();
    assert_eq!(entity_ids.len(), 4);
    
    // Should contain all our test entity IDs
    assert!(entity_ids.contains(&1));
    assert!(entity_ids.contains(&2));
    assert!(entity_ids.contains(&3));
    assert!(entity_ids.contains(&4));
}

#[test]
fn test_nonexistent_entity() {
    let client = create_mock_client_with_test_data();
    
    let result = client.get_entity(999);
    assert!(result.is_none(), "Should return None for non-existent entity");
}

/// Helper function to create evolving test data (simulates live updates)
pub fn create_evolving_test_data(time_offset: f32) -> HashMap<u32, RemoteEntity> {
    let mut entities = HashMap::new();
    
    // Simulate moving player
    let mut player_components = HashMap::new();
    player_components.insert(
        "bevy_transform::components::transform::Transform".to_string(),
        json!({
            "translation": { 
                "x": time_offset.sin() * 2.0, 
                "y": 0.0, 
                "z": 0.0 
            },
            "rotation": { "x": 0.0, "y": 0.0, "z": 0.0, "w": 1.0 },
            "scale": { "x": 1.0, "y": 1.0, "z": 1.0 }
        })
    );
    
    // Simulate changing health
    let health = 50 + (time_offset * 0.1).sin() as i32 * 25;
    player_components.insert(
        "Player".to_string(),
        json!({
            "speed": 5.0,
            "health": health.max(1).min(100)
        })
    );
    
    entities.insert(1, RemoteEntity {
        id: 1,
        name: Some("Player".to_string()),
        components: player_components,
    });
    
    entities
}

#[test]
fn test_evolving_data() {
    let data1 = create_evolving_test_data(0.0);
    let data2 = create_evolving_test_data(1.0);
    
    // Data should be different at different time offsets
    let player1 = &data1[&1];
    let player2 = &data2[&1];
    
    let transform1 = &player1.components["bevy_transform::components::transform::Transform"];
    let transform2 = &player2.components["bevy_transform::components::transform::Transform"];
    
    // X position should be different due to sin function
    assert_ne!(transform1["translation"]["x"], transform2["translation"]["x"]);
}
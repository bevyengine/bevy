use bevy::prelude::*;
use std::collections::HashMap;

use crate::inspector::events::{ComponentData, EntityData, InspectorEvent};

/// Resource for tracking remote entities and their polling state
#[derive(Resource, Default)]
pub struct RemoteEntities {
    pub entities: HashMap<Entity, EntityData>,
    pub has_polled: bool,
}

/// System that polls the remote application for entities
pub fn poll_remote_entities(
    mut remote_entities: ResMut<RemoteEntities>,
    mut inspector_events: EventWriter<InspectorEvent>,
) {
    // Only poll once for this demo
    if !remote_entities.has_polled {
        remote_entities.has_polled = true;
        
        // Mock some entities for demonstration
        let mock_entities = vec![
            EntityData {
                entity: Entity::from_bits(0),
                components: vec![
                    ComponentData {
                        type_name: "Transform".to_string(),
                        data: serde_json::json!({
                            "translation": [0.0, 0.0, 0.0],
                            "rotation": [0.0, 0.0, 0.0, 1.0],
                            "scale": [1.0, 1.0, 1.0]
                        }),
                    },
                    ComponentData {
                        type_name: "Mesh".to_string(),
                        data: serde_json::json!({ "handle": "cube" }),
                    },
                ],
            },
            EntityData {
                entity: Entity::from_bits(1),
                components: vec![
                    ComponentData {
                        type_name: "Camera".to_string(),
                        data: serde_json::json!({ "fov": 45.0 }),
                    },
                    ComponentData {
                        type_name: "Transform".to_string(),
                        data: serde_json::json!({
                            "translation": [0.0, 0.0, 5.0],
                            "rotation": [0.0, 0.0, 0.0, 1.0],
                            "scale": [1.0, 1.0, 1.0]
                        }),
                    },
                ],
            },
        ];
        
        // Add to local cache
        for entity_data in &mock_entities {
            remote_entities.entities.insert(entity_data.entity, entity_data.clone());
        }
        
        // Send event with new entities
        inspector_events.trigger(InspectorEvent::EntitiesAdded(mock_entities));
    }
}

/// System that fetches component data for selected entities
pub fn fetch_entity_components(
    selected_entity: Res<crate::inspector::selection::SelectedEntity>,
    mut inspector_events: EventWriter<InspectorEvent>,
) {
    if let Some(entity) = selected_entity.0 {
        // Mock component data for demonstration
        let components = vec![
            ComponentData {
                type_name: "Transform".to_string(),
                data: serde_json::json!({
                    "translation": [0.0, 0.0, 0.0],
                    "rotation": [0.0, 0.0, 0.0, 1.0],
                    "scale": [1.0, 1.0, 1.0]
                }),
            },
        ];
        
        inspector_events.trigger(InspectorEvent::ComponentsChanged {
            entity,
            new_components: components,
        });
    }
}

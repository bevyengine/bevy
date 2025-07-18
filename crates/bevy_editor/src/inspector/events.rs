use bevy::prelude::*;
use serde::Deserialize;

/// Represents a component's data received from the remote server.
#[derive(Clone, Debug, Deserialize)]
pub struct ComponentData {
    pub type_name: String,
    // Using serde_json::Value to hold arbitrary component data
    pub data: serde_json::Value,
}

/// Represents an entity and its components from the remote server.
#[derive(Clone, Debug, Deserialize)]
pub struct EntityData {
    pub entity: Entity,
    pub components: Vec<ComponentData>,
}

/// Events that are sent to the inspector UI to notify it of changes.
#[derive(Event)]
pub enum InspectorEvent {
    /// Sent when new entities are detected.
    EntitiesAdded(Vec<EntityData>),
    /// Sent when entities are removed.
    EntitiesRemoved(Vec<Entity>),
    /// Sent when components of an entity are changed.
    ComponentsChanged {
        entity: Entity,
        new_components: Vec<ComponentData>,
    },
}

impl bevy::ecs::event::BufferedEvent for InspectorEvent {}

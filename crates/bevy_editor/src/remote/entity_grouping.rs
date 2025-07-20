//! Entity grouping system for hierarchical display
//! 
//! This module provides functionality to group entities by their primary components
//! for display in a hierarchical tree view.

use super::types::RemoteEntity;
use super::entity_naming::{analyze_component_for_naming, ComponentPrecedence};
use std::collections::HashMap;

/// Represents a group of entities with the same primary component type
#[derive(Debug, Clone)]
pub struct EntityGroup {
    pub group_name: String,
    pub group_type: GroupType,
    pub entities: Vec<RemoteEntity>,
    pub is_expanded: bool,
}

/// Types of entity groups for categorization
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum GroupType {
    /// Entities with Name components
    Named,
    /// Camera entities
    Cameras,
    /// Light entities (DirectionalLight, PointLight, SpotLight)
    Lights,
    /// Window entities
    Windows,
    /// Pointer/interaction entities
    Pointers,
    /// Audio entities (AudioSource, AudioListener)
    Audio,
    /// UI entities (Button, Text, Node, etc.)
    UI,
    /// Mesh/rendering entities
    Rendering,
    /// User/custom component entities
    UserComponents(String), // Contains the component name
    /// Empty entities (no components)
    Empty,
    /// Generic entities (only basic Bevy components)
    Generic,
}

impl GroupType {
    /// Get the display name for this group type
    pub fn display_name(&self) -> String {
        match self {
            GroupType::Named => "Named Entities".to_string(),
            GroupType::Cameras => "Cameras".to_string(),
            GroupType::Lights => "Lights".to_string(),
            GroupType::Windows => "Windows".to_string(),
            GroupType::Pointers => "Pointers".to_string(),
            GroupType::Audio => "Audio".to_string(),
            GroupType::UI => "UI Elements".to_string(),
            GroupType::Rendering => "Rendering".to_string(),
            GroupType::UserComponents(name) => format!("{} Components", name),
            GroupType::Empty => "Empty Entities".to_string(),
            GroupType::Generic => "Generic Entities".to_string(),
        }
    }

    /// Get the precedence for sorting groups
    pub fn precedence(&self) -> u8 {
        match self {
            GroupType::Named => 0,
            GroupType::Cameras => 1,
            GroupType::Lights => 2,
            GroupType::Windows => 3,
            GroupType::Audio => 4,
            GroupType::UI => 5,
            GroupType::Rendering => 6,
            GroupType::Pointers => 7,
            GroupType::UserComponents(_) => 8,
            GroupType::Generic => 9,
            GroupType::Empty => 10,
        }
    }
}

/// Group entities by their primary component types
pub fn group_entities_by_component(entities: &[RemoteEntity]) -> Vec<EntityGroup> {
    let mut groups: HashMap<GroupType, Vec<RemoteEntity>> = HashMap::new();
    
    for entity in entities {
        let group_type = determine_entity_group_type(entity);
        groups.entry(group_type).or_default().push(entity.clone());
    }
    
    // Convert to EntityGroup structs and sort
    let mut entity_groups: Vec<EntityGroup> = groups
        .into_iter()
        .map(|(group_type, entities)| {
            let mut sorted_entities = entities;
            // Sort entities within each group by ID
            sorted_entities.sort_by_key(|e| e.id);
            
            EntityGroup {
                group_name: group_type.display_name(),
                group_type,
                entities: sorted_entities,
                is_expanded: false, // Default to collapsed for cleaner initial view
            }
        })
        .collect();
    
    // Sort groups by precedence
    entity_groups.sort_by_key(|group| group.group_type.precedence());
    
    entity_groups
}

/// Determine which group an entity belongs to based on its components
fn determine_entity_group_type(entity: &RemoteEntity) -> GroupType {
    // Check if empty
    if entity.components.is_empty() && entity.full_component_names.is_empty() {
        return GroupType::Empty;
    }
    
    // Find the highest precedence component to determine grouping
    let mut best_precedence = ComponentPrecedence::EntityId;
    let mut group_type = GroupType::Generic;
    
    // Check short component names first
    for component_name in &entity.components {
        if let Some(naming) = analyze_component_for_naming(component_name) {
            if naming.precedence < best_precedence {
                best_precedence = naming.precedence;
                group_type = component_to_group_type(component_name, &naming.precedence);
            }
        }
    }
    
    // Check full component names
    for full_component_name in &entity.full_component_names {
        let component_name = full_component_name.split("::").last().unwrap_or(full_component_name);
        if let Some(naming) = analyze_component_for_naming(component_name) {
            if naming.precedence < best_precedence {
                best_precedence = naming.precedence;
                group_type = component_to_group_type(component_name, &naming.precedence);
            }
        }
    }
    
    group_type
}

/// Convert a component name and precedence to a group type
fn component_to_group_type(component_name: &str, precedence: &ComponentPrecedence) -> GroupType {
    let clean_name = component_name.split("::").last().unwrap_or(component_name);
    
    match precedence {
        ComponentPrecedence::Name => GroupType::Named,
        ComponentPrecedence::UserComponent => {
            GroupType::UserComponents(clean_name.to_string())
        },
        ComponentPrecedence::PrimaryBevy | ComponentPrecedence::SecondaryBevy => {
            match clean_name {
                "Camera" | "Camera2d" | "Camera3d" => GroupType::Cameras,
                "DirectionalLight" | "PointLight" | "SpotLight" => GroupType::Lights,
                "Window" => GroupType::Windows,
                "PointerId" | "PointerInteraction" | "PointerLocation" | "PointerPress" => GroupType::Pointers,
                "AudioListener" | "AudioSource" => GroupType::Audio,
                "Button" | "Text" | "Text2d" | "Node" | "ImageNode" => GroupType::UI,
                "Mesh3d" | "Mesh2d" | "MeshMaterial3d" | "MeshMaterial2d" | "Sprite" => GroupType::Rendering,
                _ => GroupType::Generic,
            }
        },
        ComponentPrecedence::EntityId => GroupType::Generic,
    }
}

/// Create a flat list from grouped entities for backwards compatibility
pub fn flatten_grouped_entities(groups: &[EntityGroup]) -> Vec<RemoteEntity> {
    let mut flattened = Vec::new();
    
    for group in groups {
        if group.is_expanded {
            flattened.extend(group.entities.iter().cloned());
        }
    }
    
    flattened
}

/// Get the total count of entities across all groups
pub fn get_total_entity_count(groups: &[EntityGroup]) -> usize {
    groups.iter().map(|g| g.entities.len()).sum()
}

/// Get the count of visible (expanded) entities
pub fn get_visible_entity_count(groups: &[EntityGroup]) -> usize {
    groups.iter()
        .filter(|g| g.is_expanded)
        .map(|g| g.entities.len())
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_entity_grouping() {
        let entities = vec![
            RemoteEntity {
                id: 1,
                components: vec!["Camera".to_string()],
                full_component_names: vec!["bevy_render::camera::Camera".to_string()],
            },
            RemoteEntity {
                id: 2,
                components: vec!["Camera".to_string()],
                full_component_names: vec!["bevy_render::camera::Camera".to_string()],
            },
            RemoteEntity {
                id: 3,
                components: vec!["DirectionalLight".to_string()],
                full_component_names: vec!["bevy_pbr::light::DirectionalLight".to_string()],
            },
            RemoteEntity {
                id: 4,
                components: vec!["Player".to_string()],
                full_component_names: vec!["my_game::Player".to_string()],
            },
            RemoteEntity {
                id: 5,
                components: vec![],
                full_component_names: vec![],
            },
        ];
        
        let groups = group_entities_by_component(&entities);
        
        // Should have multiple groups
        assert!(groups.len() > 1);
        
        // Find specific groups
        let camera_group = groups.iter().find(|g| matches!(g.group_type, GroupType::Cameras));
        assert!(camera_group.is_some());
        assert_eq!(camera_group.unwrap().entities.len(), 2);
        
        let light_group = groups.iter().find(|g| matches!(g.group_type, GroupType::Lights));
        assert!(light_group.is_some());
        assert_eq!(light_group.unwrap().entities.len(), 1);
        
        let user_group = groups.iter().find(|g| matches!(g.group_type, GroupType::UserComponents(_)));
        assert!(user_group.is_some());
        assert_eq!(user_group.unwrap().entities.len(), 1);
        
        let empty_group = groups.iter().find(|g| matches!(g.group_type, GroupType::Empty));
        assert!(empty_group.is_some());
        assert_eq!(empty_group.unwrap().entities.len(), 1);
    }
    
    #[test]
    fn test_group_precedence() {
        let groups = vec![
            GroupType::Generic,
            GroupType::Named,
            GroupType::Cameras,
            GroupType::Empty,
            GroupType::UserComponents("Player".to_string()),
        ];
        
        let mut sorted_groups = groups;
        sorted_groups.sort_by_key(|g| g.precedence());
        
        // Named should be first, Empty should be last
        assert!(matches!(sorted_groups[0], GroupType::Named));
        assert!(matches!(sorted_groups[sorted_groups.len() - 1], GroupType::Empty));
    }
}

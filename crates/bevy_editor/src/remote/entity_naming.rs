//! Smart entity naming system for the editor
//! 
//! This module provides intelligent naming for entities based on their components.
//! It follows a precedence system:
//! 1. Name component (highest priority)
//! 2. Common Bevy components (Camera, Window, etc.)
//! 3. User components
//! 4. Entity ID fallback

use super::types::RemoteEntity;
use serde_json::Value;

/// Component precedence levels for naming
#[derive(Debug, PartialEq, PartialOrd, Clone)]
pub enum ComponentPrecedence {
    /// Built-in Name component - highest priority
    Name = 0,
    /// User/custom components - very high priority for meaningful names
    UserComponent = 1,
    /// Primary Bevy components (Camera, Window, etc.)
    PrimaryBevy = 2,
    /// Secondary Bevy components (Transform, Visibility, etc.)
    SecondaryBevy = 3,
    /// Entity ID fallback - lowest priority
    EntityId = 4,
}

/// A component that can provide a name for an entity
#[derive(Debug, Clone)]
pub struct NamingComponent {
    pub name: String,
    pub precedence: ComponentPrecedence,
    pub display_name: String,
}

/// Generate a smart display name for an entity based on its components
pub fn generate_entity_display_name(entity: &RemoteEntity, component_data: Option<&Value>) -> String {
    let mut best_naming: Option<NamingComponent> = None;
    
    // First check if we have component data with actual Name values
    if let Some(data) = component_data {
        if let Some(name_value) = extract_name_from_component_data(data) {
            return format!("#{} ({})", entity.id, name_value);
        }
    }
    
    // Analyze available components to find the best naming option
    for component_name in &entity.components {
        if let Some(naming) = analyze_component_for_naming(component_name) {
            if best_naming.is_none() || naming.precedence < best_naming.as_ref().unwrap().precedence {
                best_naming = Some(naming);
            }
        }
    }
    
    // Also check full component names for more accurate detection
    for full_component_name in &entity.full_component_names {
        if let Some(naming) = analyze_component_for_naming(full_component_name) {
            if best_naming.is_none() || naming.precedence < best_naming.as_ref().unwrap().precedence {
                best_naming = Some(naming);
            }
        }
    }
    
    match best_naming {
        Some(naming) if naming.precedence != ComponentPrecedence::EntityId => {
            format!("#{} ({})", entity.id, naming.display_name)
        }
        _ => {
            // Fallback to entity ID
            format!("Entity {}", entity.id)
        }
    }
}

/// Extract the actual name value from component data JSON
fn extract_name_from_component_data(component_data: &Value) -> Option<String> {
    if let Some(components) = component_data.get("components").and_then(|v| v.as_object()) {
        // Look for Name component in various forms
        for (component_type, component_value) in components {
            if component_type.contains("Name") || component_type.ends_with("::Name") {
                // Try to extract the actual name value
                if let Some(name_str) = component_value.as_str() {
                    return Some(name_str.to_string());
                }
                // Handle nested name values
                if let Some(obj) = component_value.as_object() {
                    if let Some(name_val) = obj.get("name").or_else(|| obj.get("value")).or_else(|| obj.get("0")) {
                        if let Some(name_str) = name_val.as_str() {
                            return Some(name_str.to_string());
                        }
                    }
                }
            }
        }
    }
    None
}

/// Analyze a component name to determine if it can provide entity naming
fn analyze_component_for_naming(component_name: &str) -> Option<NamingComponent> {
    // Remove module paths for analysis
    let clean_name = component_name
        .split("::")
        .last()
        .unwrap_or(component_name);
    
    // Check for Name component (highest priority)
    if clean_name == "Name" || component_name.ends_with("::Name") {
        return Some(NamingComponent {
            name: component_name.to_string(),
            precedence: ComponentPrecedence::Name,
            display_name: "Named Entity".to_string(),
        });
    }
    
    // Primary Bevy components that are very distinctive
    match clean_name {
        "Camera" | "Camera2d" | "Camera3d" => Some(NamingComponent {
            name: component_name.to_string(),
            precedence: ComponentPrecedence::PrimaryBevy,
            display_name: "Camera".to_string(),
        }),
        "Window" => Some(NamingComponent {
            name: component_name.to_string(),
            precedence: ComponentPrecedence::PrimaryBevy,
            display_name: "Window".to_string(),
        }),
        "DirectionalLight" => Some(NamingComponent {
            name: component_name.to_string(),
            precedence: ComponentPrecedence::PrimaryBevy,
            display_name: "Directional Light".to_string(),
        }),
        "PointLight" => Some(NamingComponent {
            name: component_name.to_string(),
            precedence: ComponentPrecedence::PrimaryBevy,
            display_name: "Point Light".to_string(),
        }),
        "SpotLight" => Some(NamingComponent {
            name: component_name.to_string(),
            precedence: ComponentPrecedence::PrimaryBevy,
            display_name: "Spot Light".to_string(),
        }),
        "AudioListener" => Some(NamingComponent {
            name: component_name.to_string(),
            precedence: ComponentPrecedence::PrimaryBevy,
            display_name: "Audio Listener".to_string(),
        }),
        "AudioSource" => Some(NamingComponent {
            name: component_name.to_string(),
            precedence: ComponentPrecedence::PrimaryBevy,
            display_name: "Audio Source".to_string(),
        }),
        
        // Secondary Bevy components (lower priority)
        "Mesh3d" | "Mesh2d" => Some(NamingComponent {
            name: component_name.to_string(),
            precedence: ComponentPrecedence::SecondaryBevy,
            display_name: "Mesh".to_string(),
        }),
        "MeshMaterial3d" | "MeshMaterial2d" => Some(NamingComponent {
            name: component_name.to_string(),
            precedence: ComponentPrecedence::SecondaryBevy,
            display_name: "Material".to_string(),
        }),
        "Text" | "Text2d" => Some(NamingComponent {
            name: component_name.to_string(),
            precedence: ComponentPrecedence::SecondaryBevy,
            display_name: "Text".to_string(),
        }),
        "Sprite" => Some(NamingComponent {
            name: component_name.to_string(),
            precedence: ComponentPrecedence::SecondaryBevy,
            display_name: "Sprite".to_string(),
        }),
        "ImageNode" => Some(NamingComponent {
            name: component_name.to_string(),
            precedence: ComponentPrecedence::SecondaryBevy,
            display_name: "Image".to_string(),
        }),
        "Button" => Some(NamingComponent {
            name: component_name.to_string(),
            precedence: ComponentPrecedence::SecondaryBevy,
            display_name: "Button".to_string(),
        }),
        "Node" => Some(NamingComponent {
            name: component_name.to_string(),
            precedence: ComponentPrecedence::SecondaryBevy,
            display_name: "UI Node".to_string(),
        }),
        
        _ => {
            // Check if it's likely a user component (not starting with common Bevy prefixes)
            if !is_bevy_builtin_component(component_name) {
                Some(NamingComponent {
                    name: component_name.to_string(),
                    precedence: ComponentPrecedence::UserComponent,
                    display_name: clean_name.to_string(),
                })
            } else {
                None
            }
        }
    }
}

/// Check if a component is likely a Bevy built-in component
fn is_bevy_builtin_component(component_name: &str) -> bool {
    let bevy_prefixes = [
        "bevy_",
        "std::",
        "core::",
        "alloc::",
        "winit::",
    ];
    
    let has_bevy_prefix = bevy_prefixes.iter().any(|prefix| component_name.starts_with(prefix));
    let is_common_bevy = matches!(component_name.split("::").last().unwrap_or(""), 
        "Transform" | "GlobalTransform" | "Visibility" | "InheritedVisibility" | 
        "ViewVisibility" | "ComputedVisibility" | "Parent" | "Children" | "Aabb" |
        "TransformTreeChanged" | "Frustum" | "Projection" | "VisibleEntities" |
        "DebandDither" | "Tonemapping" | "ClusterConfig" | "RenderEntity" | 
        "SyncToRenderWorld" | "Msaa" | "CubemapFrusta" | "CubemapVisibleEntities"
    );
    
    has_bevy_prefix || is_common_bevy
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_entity_naming_precedence() {
        let mut entity = RemoteEntity {
            id: 123,
            components: vec!["Camera".to_string(), "Transform".to_string()],
            full_component_names: vec!["bevy_render::camera::Camera".to_string(), "bevy_transform::components::Transform".to_string()],
        };
        
        let display_name = generate_entity_display_name(&entity, None);
        assert_eq!(display_name, "#123 (Camera)");
        
        // Test with user component (should take precedence over Camera)
        entity.components.push("Player".to_string());
        entity.full_component_names.push("my_game::Player".to_string());
        
        let display_name = generate_entity_display_name(&entity, None);
        assert_eq!(display_name, "#123 (Player)");
        
        // Test with Name component (should take precedence over everything)
        entity.components.push("Name".to_string());
        entity.full_component_names.push("bevy_core::name::Name".to_string());
        
        let display_name = generate_entity_display_name(&entity, None);
        assert_eq!(display_name, "#123 (Named Entity)");
    }
    
    #[test]
    fn test_user_component_precedence_over_bevy() {
        let entity = RemoteEntity {
            id: 456,
            components: vec!["Cube".to_string(), "Mesh3d".to_string(), "Transform".to_string()],
            full_component_names: vec![
                "my_game::Cube".to_string(), 
                "bevy_mesh::mesh::Mesh3d".to_string(),
                "bevy_transform::components::Transform".to_string()
            ],
        };
        
        let display_name = generate_entity_display_name(&entity, None);
        // User component "Cube" should take precedence over "Mesh3d"
        assert_eq!(display_name, "#456 (Cube)");
    }
    
    #[test]
    fn test_fallback_to_entity_id() {
        let entity = RemoteEntity {
            id: 789,
            components: vec!["Transform".to_string(), "Visibility".to_string()],
            full_component_names: vec!["bevy_transform::components::Transform".to_string(), "bevy_render::view::visibility::Visibility".to_string()],
        };
        
        let display_name = generate_entity_display_name(&entity, None);
        assert_eq!(display_name, "Entity 789");
    }
    
    #[test]
    fn test_cube_example_from_server() {
        // Simulate the cube entity from server.rs example
        let entity = RemoteEntity {
            id: 42,
            components: vec![
                "Aabb".to_string(), 
                "Cube".to_string(), 
                "Mesh3d".to_string(), 
                "MeshMaterial3d".to_string(),
                "Transform".to_string(),
            ],
            full_component_names: vec![
                "bevy_camera::primitives::Aabb".to_string(),
                "server::Cube".to_string(),
                "bevy_mesh::mesh::Mesh3d".to_string(),
                "bevy_pbr::material::MeshMaterial3d".to_string(),
                "bevy_transform::components::Transform".to_string(),
            ],
        };
        
        let display_name = generate_entity_display_name(&entity, None);
        // User component "Cube" should take precedence over all Bevy components including Aabb
        assert_eq!(display_name, "#42 (Cube)");
        println!("Cube entity correctly named: {}", display_name);
    }
}

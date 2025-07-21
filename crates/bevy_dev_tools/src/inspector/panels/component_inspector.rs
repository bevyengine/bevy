//! Component inspector panel for viewing/editing component data

use bevy::prelude::*;
use bevy::ui::{AlignItems, JustifyContent};
use crate::themes::DarkTheme;
use crate::remote::types::{ComponentDisplayState, ComponentDataFetched, ComponentField};
use crate::formatting::{format_value_inline, format_simple_value};
use serde_json::Value;

/// Component for marking UI elements
#[derive(Component)]
pub struct ComponentInspector;

/// Component for the content area of the component inspector
#[derive(Component)]
pub struct ComponentInspectorContent;

/// Component for text elements in the component inspector
#[derive(Component)]
pub struct ComponentInspectorText;

/// Component for scrollable areas in the component inspector
#[derive(Component)]
pub struct ComponentInspectorScrollArea;

/// Handle component data fetched event  
pub fn handle_component_data_fetched(
    trigger: On<ComponentDataFetched>,
    mut commands: Commands,
    component_content_query: Query<Entity, With<ComponentInspectorContent>>,
    display_state: Res<ComponentDisplayState>,
    mut last_data_cache: Local<String>,
) {
    let event = trigger.event();
    
    // Simple cache key: entity_id + data content
    let cache_key = format!("{}:{}", event.entity_id, event.component_data);
    
    // Check if this is the same data to prevent flickering
    if *last_data_cache == cache_key {
        return; // Same data, don't rebuild
    }
    
    // Update our cache
    *last_data_cache = cache_key;
    
    for content_entity in &component_content_query {
        // Clear existing content
        commands.entity(content_entity).despawn_children();
        
        // Build new widget-based content
        commands.entity(content_entity).with_children(|parent| {
            if event.component_data.trim().is_empty() {
                parent.spawn((
                    Text::new(format!("Entity {} - No Component Data\n\nNo component data received from server.", event.entity_id)),
                    TextFont {
                        ..default()
                    },
                    TextColor(DarkTheme::TEXT_MUTED),
                ));
            } else {
                // Build interactive component widgets
                build_component_widgets(parent, event.entity_id, &event.component_data, &display_state);
            }
        });
    }
}

/// Build component display as interactive widgets instead of just text
pub fn build_component_widgets(
    parent: &mut ChildSpawnerCommands,
    entity_id: u32,
    components_data: &str,
    display_state: &ComponentDisplayState,
) {
    // First check if this looks like the component names format (simple list)
    if components_data.contains("Component names for Entity") || 
       (components_data.contains("- ") && !components_data.starts_with("{")) || 
       (components_data.contains("* ") && !components_data.starts_with("{")) {
        println!("   → Taking COMPONENT NAMES path");
        build_component_names_display(parent, entity_id, components_data, display_state);
        return;
    }
    
    // Try to parse the JSON response 
    if let Ok(json_value) = serde_json::from_str::<Value>(components_data) {
        println!("   → JSON parsing succeeded");
        
        // Check for wrapped format with "components" key FIRST
        if let Some(components_obj) = json_value.get("components").and_then(|v| v.as_object()) {
            build_json_components(parent, entity_id, components_obj, display_state);
            return;
        }
        // Then check if it's a direct object (component data directly)
        else if let Some(components_obj) = json_value.as_object() {
            build_json_components(parent, entity_id, components_obj, display_state);
            return;
        }
    }
    parent.spawn((
        Text::new(format!("Entity {} - Component Data\n\n{}", entity_id, components_data)),
        TextFont {
            font_size: 13.0,
            ..default()
        },
        TextColor(DarkTheme::TEXT_SECONDARY),
    ));
}

/// Build component display from JSON data
fn build_json_components(
    parent: &mut ChildSpawnerCommands,
    entity_id: u32,
    components_obj: &serde_json::Map<String, Value>,
    display_state: &ComponentDisplayState,
) {
    // Header
    parent.spawn((
        Text::new(format!("Entity {} - Components", entity_id)),
        TextFont {
            font_size: 15.0,
            ..default()
        },
        TextColor(DarkTheme::TEXT_PRIMARY),
        Node {
            margin: UiRect::bottom(Val::Px(12.0)),
            ..default()
        },
    ));
    
    for (component_name, component_data) in components_obj {
        // Clean component name (remove module path)
        let clean_name = component_name.split("::").last().unwrap_or(component_name);
        build_component_widget(parent, clean_name, component_data, component_name, display_state);
    }
}

/// Build component display from component names (fallback when JSON fetch fails)
fn build_component_names_display(
    parent: &mut ChildSpawnerCommands,
    entity_id: u32,
    components_data: &str,
    display_state: &ComponentDisplayState,
) {
    // Header
    parent.spawn((
        Text::new(format!("Entity {} - Components", entity_id)),
        TextFont {
            font_size: 15.0,
            ..default()
        },
        TextColor(DarkTheme::TEXT_PRIMARY),
        Node {
            margin: UiRect::bottom(Val::Px(12.0)),
            ..default()
        },
    ));
    
    // Extract component names from the fallback text
    let lines: Vec<&str> = components_data.lines().collect();
    let component_names: Vec<&str> = lines.iter()
        .skip(2) // Skip the header lines
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            let trimmed = line.trim();
            // Remove bullet points and other prefixes - only ASCII characters
            if trimmed.starts_with("- ") {
                &trimmed[2..]
            } else if trimmed.starts_with("* ") {
                &trimmed[2..]
            } else {
                trimmed
            }
        })
        .filter(|name| !name.is_empty())
        .collect();
    
    // Display each component in the original beautiful format
    for component_name in component_names {
        build_component_name_widget(parent, component_name, display_state);
    }
}

/// Build a component widget from just the name (when full data isn't available)
fn build_component_name_widget(
    parent: &mut ChildSpawnerCommands,
    full_component_name: &str,
    _display_state: &ComponentDisplayState,
) {
    // Clean component name (remove module path)
    let clean_name = full_component_name.split("::").last().unwrap_or(full_component_name);
    let package_name = extract_package_name(full_component_name);
    
    // Component container - simple style matching original
    parent.spawn((
        Node {
            width: Val::Percent(100.0),
            flex_direction: FlexDirection::Column,
            margin: UiRect::bottom(Val::Px(4.0)),
            padding: UiRect::all(Val::Px(4.0)),
            ..default()
        },
    )).with_children(|parent| {
        // Component title in original format: [package] ComponentName
        let display_text = if package_name.is_empty() {
            clean_name.to_string()
        } else {
            format!("{} {}", package_name, clean_name)
        };
        parent.spawn((
            Text::new(display_text),
            TextFont {
                font_size: 13.0,
                ..default()
            },
            TextColor(DarkTheme::TEXT_SECONDARY),
        ));
    });
}

/// Extract package name from a full component type string
pub fn extract_package_name(full_component_name: &str) -> String {
    // Handle different patterns:
    // bevy_transform::components::Transform -> [bevy_transform]
    // bevy_ui::ui_node::Node -> [bevy_ui]  
    // cube::server::SomeComponent -> [cube]
    // std::collections::HashMap -> [std]
    // MyComponent -> [MyComponent] (no package)
    
    if let Some(first_separator) = full_component_name.find("::") {
        let package_part = &full_component_name[..first_separator];
        format!("[{}]", package_part)
    } else {
        // No package separator, just use the component name itself without brackets
        String::new()
    }
}

/// Build a single component widget with expansion capabilities
pub fn build_component_widget(
    parent: &mut ChildSpawnerCommands,
    clean_name: &str,
    component_data: &Value,
    full_component_name: &str,
    display_state: &ComponentDisplayState,
) {
    // Component header container
    parent.spawn((
        Node {
            width: Val::Percent(100.0),
            flex_direction: FlexDirection::Column,
            margin: UiRect::bottom(Val::Px(12.0)),
            padding: UiRect::all(Val::Px(8.0)),
            border: UiRect::all(Val::Px(1.0)),
            ..default()
        },
        BorderColor::all(DarkTheme::BORDER_PRIMARY),
        BackgroundColor(DarkTheme::BACKGROUND_SECONDARY),
    )).with_children(|parent| {
        // Component title with package name
        let package_name = extract_package_name(full_component_name);
        parent.spawn((
            Text::new(format!("{} {}", package_name, clean_name)),
            TextFont {
                font_size: 14.0,
                ..default()
            },
            TextColor(DarkTheme::TEXT_PRIMARY),
            Node {
                margin: UiRect::bottom(Val::Px(6.0)),
                ..default()
            },
        ));
        
        // Build component fields
        let fields = parse_component_fields(full_component_name, component_data);
        for field in fields {
            build_field_widget(parent, &field, 1, &format!("{}.{}", clean_name, field.name), display_state);
        }
    });
}

/// Parse component JSON into structured fields
pub fn parse_component_fields(component_name: &str, component_data: &Value) -> Vec<ComponentField> {
    let mut fields = Vec::new();
    
    // Special handling for GlobalTransform matrix data
    if component_name.contains("GlobalTransform") {
        if let Some(arr) = component_data.as_array() {
            if arr.len() == 12 {
                // GlobalTransform is a 3x4 affine transformation matrix
                // [m00, m01, m02, m10, m11, m12, m20, m21, m22, m30, m31, m32]
                // Where the matrix is:
                // | m00 m10 m20 m30 |
                // | m01 m11 m21 m31 |
                // | m02 m12 m22 m32 |
                // |  0   0   0   1  |
                
                fields.push(ComponentField {
                    name: "matrix".to_string(),
                    field_type: "matrix3x4".to_string(),
                    value: component_data.clone(),
                    is_expandable: true,
                });
                return fields;
            }
        }
    }
    
    if let Some(obj) = component_data.as_object() {
        for (field_name, field_value) in obj {
            let field_type = match field_value {
                Value::Number(_) => "number",
                Value::String(_) => "string", 
                Value::Bool(_) => "boolean",
                Value::Array(_) => "array",
                Value::Object(_) => "object",
                Value::Null => "null",
            }.to_string();
            
            let is_expandable = matches!(field_value, Value::Array(_) | Value::Object(_));
            
            fields.push(ComponentField {
                name: field_name.clone(),
                field_type,
                value: field_value.clone(),
                is_expandable,
            });
        }
    } else if let Some(_arr) = component_data.as_array() {
        // Handle components that are directly arrays
        fields.push(ComponentField {
            name: "data".to_string(),
            field_type: "array".to_string(),
            value: component_data.clone(),
            is_expandable: true,
        });
    } else {
        // Handle primitive values (numbers, strings, booleans)
        let field_type = match component_data {
            Value::Number(_) => "number",
            Value::String(_) => "string", 
            Value::Bool(_) => "boolean",
            Value::Null => "null",
            _ => "unknown",
        }.to_string();
        
        fields.push(ComponentField {
            name: "value".to_string(),
            field_type,
            value: component_data.clone(),
            is_expandable: false,
        });
    }
    
    fields
}

/// Build a field widget with expansion button if needed
pub fn build_field_widget(
    parent: &mut ChildSpawnerCommands,
    field: &ComponentField,
    indent_level: usize,
    path: &str,
    display_state: &ComponentDisplayState,
) {
    let indent_px = (indent_level as f32) * 16.0;
    
    parent.spawn((
        Node {
            width: Val::Percent(100.0),
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            margin: UiRect::left(Val::Px(indent_px)),
            padding: UiRect::vertical(Val::Px(2.0)),
            ..default()
        },
    )).with_children(|parent| {
        if field.is_expandable {
            let is_expanded = display_state.expanded_paths.contains(path);
            
            // Expansion button
            parent.spawn((
                Button,
                crate::widgets::ExpansionButton {
                    path: path.to_string(),
                    is_expanded,
                },
                Node {
                    width: Val::Px(20.0),
                    height: Val::Px(16.0),
                    margin: UiRect::right(Val::Px(6.0)),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    ..default()
                },
                BackgroundColor(DarkTheme::EXPANSION_BUTTON_DEFAULT),
                BorderColor::all(DarkTheme::BORDER_PRIMARY),
            )).with_children(|parent| {
                parent.spawn((
                    Text::new(if is_expanded { "-" } else { "+" }),
                    TextFont {
                        font_size: 14.0, // Slightly larger for better visibility
                        ..default()
                    },
                    TextColor(DarkTheme::TEXT_PRIMARY),
                ));
            });
            
            // Field name and summary
            let value_summary = format_value_inline(&field.value);
            parent.spawn((
                Text::new(format!("{}: {}", field.name, value_summary)),
                TextFont {
                    font_size: 13.0,
                    ..default()
                },
                TextColor(DarkTheme::TEXT_SECONDARY),
            ));
        } else {
            // Indentation space for non-expandable fields
            parent.spawn((
                Node {
                    width: Val::Px(26.0), // Space for button + margin
                    height: Val::Px(16.0),
                    ..default()
                },
            ));
            
            // For simple values, display inline
            let formatted_value = format_simple_value(&field.value);
            parent.spawn((
                Text::new(format!("{}: {}", field.name, formatted_value)),
                TextFont {
                    font_size: 13.0,
                    ..default()
                },
                TextColor(DarkTheme::TEXT_SECONDARY),
            ));
        }
    });
    
    // Show expanded children if the field is expanded
    if field.is_expandable && display_state.expanded_paths.contains(path) {
        if matches!(field.value, Value::Object(_)) {
            build_expanded_object_widgets(parent, &field.value, indent_level + 1, path, display_state);
        } else if matches!(field.value, Value::Array(_)) {
            build_expanded_array_widgets(parent, &field.value, indent_level + 1, path, display_state);
        }
    }
}

/// Build widgets for expanded object fields
pub fn build_expanded_object_widgets(
    parent: &mut ChildSpawnerCommands,
    value: &Value,
    indent_level: usize,
    path: &str,
    display_state: &ComponentDisplayState,
) {
    let indent_px = (indent_level as f32) * 16.0;
    
    if let Some(obj) = value.as_object() {
        // Check for common Bevy types first
        if let (Some(x), Some(y), Some(z)) = (obj.get("x"), obj.get("y"), obj.get("z")) {
            if x.is_number() && y.is_number() && z.is_number() {
                parent.spawn((
                    Node {
                        flex_direction: FlexDirection::Column,
                        margin: UiRect::left(Val::Px(indent_px)),
                        ..default()
                    },
                )).with_children(|parent| {
                    parent.spawn((
                        Text::new(format!("x: {:.3}", x.as_f64().unwrap_or(0.0))),
                        TextFont { font_size: 12.0, ..default() },
                        TextColor(DarkTheme::TEXT_SECONDARY),
                    ));
                    parent.spawn((
                        Text::new(format!("y: {:.3}", y.as_f64().unwrap_or(0.0))),
                        TextFont { font_size: 12.0, ..default() },
                        TextColor(DarkTheme::TEXT_SECONDARY),
                    ));
                    parent.spawn((
                        Text::new(format!("z: {:.3}", z.as_f64().unwrap_or(0.0))),
                        TextFont { font_size: 12.0, ..default() },
                        TextColor(DarkTheme::TEXT_SECONDARY),
                    ));
                    if let Some(w) = obj.get("w") {
                        if w.is_number() {
                            parent.spawn((
                                Text::new(format!("w: {:.3}", w.as_f64().unwrap_or(0.0))),
                                TextFont { font_size: 12.0, ..default() },
                                TextColor(DarkTheme::TEXT_SECONDARY),
                            ));
                        }
                    }
                });
                return;
            }
        }
        
        // Generic object handling
        for (key, val) in obj {
            let child_path = format!("{}.{}", path, key);
            let child_field = ComponentField {
                name: key.clone(),
                field_type: match val {
                    Value::Number(_) => "number",
                    Value::String(_) => "string", 
                    Value::Bool(_) => "boolean",
                    Value::Array(_) => "array",
                    Value::Object(_) => "object",
                    Value::Null => "null",
                }.to_string(),
                value: val.clone(),
                is_expandable: matches!(val, Value::Array(_) | Value::Object(_)),
            };
            build_field_widget(parent, &child_field, indent_level, &child_path, display_state);
        }
    }
}

/// Build widgets for expanded array fields
pub fn build_expanded_array_widgets(
    parent: &mut ChildSpawnerCommands,
    value: &Value,
    indent_level: usize,
    path: &str,
    _display_state: &ComponentDisplayState,
) {
    let indent_px = (indent_level as f32) * 16.0;
    
    if let Some(arr) = value.as_array() {
        parent.spawn((
            Node {
                flex_direction: FlexDirection::Column,
                margin: UiRect::left(Val::Px(indent_px)),
                ..default()
            },
        )).with_children(|parent| {
            // Special handling for GlobalTransform matrix (12 elements)
            if arr.len() == 12 && path.contains("matrix") && arr.iter().all(|v| v.is_number()) {
                // GlobalTransform is a 3x4 affine transformation matrix
                // Display as meaningful transformation components
                if let (Some(m30), Some(m31), Some(m32)) = (
                    arr.get(9).and_then(|v| v.as_f64()),
                    arr.get(10).and_then(|v| v.as_f64()),
                    arr.get(11).and_then(|v| v.as_f64())
                ) {
                    parent.spawn((
                        Text::new(format!("translation: ({:.3}, {:.3}, {:.3})", m30, m31, m32)),
                        TextFont { font_size: 12.0, ..default() },
                        TextColor(DarkTheme::TEXT_SECONDARY),
                    ));
                }
                
                // Show scale (diagonal elements)
                if let (Some(m00), Some(m11), Some(m22)) = (
                    arr.get(0).and_then(|v| v.as_f64()),
                    arr.get(4).and_then(|v| v.as_f64()),
                    arr.get(8).and_then(|v| v.as_f64())
                ) {
                    parent.spawn((
                        Text::new(format!("scale: ({:.3}, {:.3}, {:.3})", m00, m11, m22)),
                        TextFont { font_size: 12.0, ..default() },
                        TextColor(DarkTheme::TEXT_SECONDARY),
                    ));
                }
                
                // Show raw matrix values for debugging
                parent.spawn((
                    Text::new("raw matrix:".to_string()),
                    TextFont { font_size: 12.0, ..default() },
                    TextColor(DarkTheme::TEXT_MUTED),
                ));
                
                for (i, item) in arr.iter().enumerate() {
                    let row = i / 3;
                    let col = i % 3;
                    parent.spawn((
                        Text::new(format!("  [{}][{}]: {:.3}", row, col, item.as_f64().unwrap_or(0.0))),
                        TextFont { font_size: 11.0, ..default() },
                        TextColor(DarkTheme::TEXT_MUTED),
                    ));
                }
            } else if arr.len() <= 4 && arr.iter().all(|v| v.is_number()) {
                // Small numeric arrays (Vec2, Vec3, Vec4, Quat components)
                for (i, item) in arr.iter().enumerate() {
                    let comp_name = match i {
                        0 => "x", 1 => "y", 2 => "z", 3 => "w",
                        _ => &format!("[{}]", i),
                    };
                    parent.spawn((
                        Text::new(format!("{}: {:.3}", comp_name, item.as_f64().unwrap_or(0.0))),
                        TextFont { font_size: 12.0, ..default() },
                        TextColor(DarkTheme::TEXT_SECONDARY),
                    ));
                }
            } else if arr.len() <= 10 {
                // Small arrays - show all items with proper names
                for (i, item) in arr.iter().enumerate() {
                    let formatted = crate::formatting::display::format_simple_value(item);
                    // Don't show array indices for single values, show the content directly
                    if arr.len() == 1 {
                        parent.spawn((
                            Text::new(formatted),
                            TextFont { font_size: 12.0, ..default() },
                            TextColor(DarkTheme::TEXT_SECONDARY),
                        ));
                    } else {
                        parent.spawn((
                            Text::new(format!("[{}]: {}", i, formatted)),
                            TextFont { font_size: 12.0, ..default() },
                            TextColor(DarkTheme::TEXT_SECONDARY),
                        ));
                    }
                }
            } else {
                // Large arrays - show first few items
                for (i, item) in arr.iter().take(3).enumerate() {
                    let formatted = crate::formatting::display::format_simple_value(item);
                    parent.spawn((
                        Text::new(format!("[{}]: {}", i, formatted)),
                        TextFont { font_size: 12.0, ..default() },
                        TextColor(DarkTheme::TEXT_SECONDARY),
                    ));
                }
                parent.spawn((
                    Text::new(format!("... ({} more items)", arr.len() - 3)),
                    TextFont { font_size: 12.0, ..default() },
                    TextColor(DarkTheme::TEXT_MUTED),
                ));
            }
        });
    }
}

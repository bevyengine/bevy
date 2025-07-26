//! Data Source Abstractions for Inspector

use bevy_ecs::entity::Entity;
use std::collections::HashMap;
use std::any::TypeId;

/// Trait for providing entity and component data to the inspector
pub trait InspectorDataSource: Send + Sync + 'static {
    /// Fetch all entities and their component data using ECS queries
    fn fetch_entities_from_queries(&mut self) -> Vec<EntityData>;
    
    /// Get reflected component data for a specific entity and component type
    /// This is a placeholder for future reflection support
    fn get_component_reflection_data(
        &self,
        entity: Entity,
        component_type: TypeId,
    ) -> Option<String>;
    
    /// Get component type information placeholder
    fn get_component_type_info_name(&self, type_id: TypeId) -> Option<String>;
    
    /// Check if this data source supports real-time updates
    fn supports_live_updates(&self) -> bool {
        true
    }
    
    /// Get a display name for this data source
    fn source_name(&self) -> &str;
    
    /// Allow downcasting to concrete types for cloning
    fn as_any(&self) -> &dyn std::any::Any;
}

/// Entity data container
#[derive(Clone, Debug)]
pub struct EntityData {
    pub id: Entity,
    pub name: Option<String>,
    pub components: Vec<ComponentData>,
    pub archetype_id: u32,
}

/// Component data container
#[derive(Clone, Debug)]
pub struct ComponentData {
    pub type_name: String,
    pub type_id: TypeId,
    pub size_bytes: usize,
    pub is_reflected: bool,
}

/// Local world data source - inspects the current world
pub struct LocalWorldDataSource;

impl InspectorDataSource for LocalWorldDataSource {
    fn fetch_entities_from_queries(&mut self) -> Vec<EntityData> {
        // For now, return empty data - this will be populated by the system
        // that has access to the actual queries
        Vec::new()
    }
    
    fn get_component_reflection_data(
        &self,
        _entity: Entity,
        _component_type: TypeId,
    ) -> Option<String> {
        // TODO: Implement component reflection data retrieval
        None
    }
    
    fn get_component_type_info_name(&self, _type_id: TypeId) -> Option<String> {
        // TODO: Implement type info name lookup
        None
    }
    
    fn source_name(&self) -> &str {
        "Local World"
    }
    
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

/// Remote data source for inspecting other processes/servers
pub struct RemoteDataSource {
    pub connection_url: String,
    pub cached_entities: Vec<EntityData>,
    pub last_update: std::time::Instant,
}

impl RemoteDataSource {
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            connection_url: url.into(),
            cached_entities: Vec::new(),
            last_update: std::time::Instant::now(),
        }
    }
}

impl InspectorDataSource for RemoteDataSource {
    fn fetch_entities_from_queries(&mut self) -> Vec<EntityData> {
        // TODO: Implement remote data fetching via bevy_remote or similar
        // For now, return cached data
        self.cached_entities.clone()
    }
    
    fn get_component_reflection_data(
        &self,
        _entity: Entity,
        _component_type: TypeId,
    ) -> Option<String> {
        // TODO: Implement remote component reflection
        None
    }
    
    fn get_component_type_info_name(&self, _type_id: TypeId) -> Option<String> {
        // TODO: Implement remote type info lookup
        None
    }
    
    fn supports_live_updates(&self) -> bool {
        false // Remote sources typically use polling
    }
    
    fn source_name(&self) -> &str {
        "Remote"
    }
    
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

/// Asset file data source for inspecting scene files
pub struct AssetFileDataSource {
    pub file_path: std::path::PathBuf,
    pub cached_entities: Vec<EntityData>,
}

impl AssetFileDataSource {
    pub fn new(path: impl Into<std::path::PathBuf>) -> Self {
        Self {
            file_path: path.into(),
            cached_entities: Vec::new(),
        }
    }
}

impl InspectorDataSource for AssetFileDataSource {
    fn fetch_entities_from_queries(&mut self) -> Vec<EntityData> {
        // TODO: Implement scene file parsing
        self.cached_entities.clone()
    }
    
    fn get_component_reflection_data(
        &self,
        _entity: Entity,
        _component_type: TypeId,
    ) -> Option<String> {
        None
    }
    
    fn get_component_type_info_name(&self, _type_id: TypeId) -> Option<String> {
        None
    }
    
    fn supports_live_updates(&self) -> bool {
        false
    }
    
    fn source_name(&self) -> &str {
        "Asset File"
    }
    
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

/// Entity grouping utilities
pub struct EntityGrouping;

impl EntityGrouping {
    /// Group entities by their primary component types
    pub fn group_by_components(
        entities: &[EntityData],
        rules: &crate::inspector::EntityGroupingRules,
    ) -> HashMap<String, Vec<Entity>> {
        let mut groups: HashMap<String, Vec<Entity>> = HashMap::new();
        
        for entity in entities {
            let group_name = Self::determine_group_name(entity, rules);
            groups.entry(group_name).or_default().push(entity.id);
        }
        
        groups
    }
    
    fn determine_group_name(entity: &EntityData, rules: &crate::inspector::EntityGroupingRules) -> String {
        // Check for custom group name matches first
        let component_names: Vec<String> = entity.components.iter()
            .map(|c| c.type_name.clone())
            .collect();
        
        for (pattern, custom_name) in &rules.custom_group_names {
            if pattern.iter().all(|p| component_names.contains(p)) {
                return custom_name.clone();
            }
        }
        
        // Use priority-based grouping
        for priority_component in &rules.component_priority {
            if component_names.contains(priority_component) {
                return format!("{}s", priority_component);
            }
        }
        
        // Fallback grouping
        if let Some(first_component) = entity.components.first() {
            if !rules.ignored_components.contains(&first_component.type_name) {
                return format!("{}s", first_component.type_name);
            }
        }
        
        "Ungrouped".to_string()
    }
}

/// Resource wrapper for the current data source
#[derive(bevy_ecs::resource::Resource)]
pub struct InspectorDataSourceResource {
    pub source: Box<dyn InspectorDataSource>,
}

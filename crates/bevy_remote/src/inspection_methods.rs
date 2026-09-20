//! `world.inspect*` verbs for the Bevy Remote Protocol.

use bevy_dev_tools::inspection::{
    component_inspection::{
        ComponentInspectionError, ComponentInspectionSettings, ComponentMetadataMap,
    },
    entity_inspection::{EntityInspectionError, EntityInspectionSettings},
    extension_methods::WorldInspectionExtensionTrait,
    resource_inspection::{ResourceInspectionError, ResourceInspectionSettings},
    world_summary::{SummarySettings, WorldSummaryExt},
};
use bevy_ecs::{entity::Entity, system::In, world::World};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{
    builtin_methods::{parse, parse_some},
    error_codes, BrpError, BrpResult,
};

/// The method path for a `world.inspect` request.
pub const BRP_INSPECT_METHOD: &str = "world.inspect";

/// The method path for a `world.inspect_component` request.
pub const BRP_INSPECT_COMPONENT_METHOD: &str = "world.inspect_component";

/// The method path for a `world.inspect_component_type` request.
pub const BRP_INSPECT_COMPONENT_TYPE_METHOD: &str = "world.inspect_component_type";

/// The method path for a `world.inspect_resource` request.
pub const BRP_INSPECT_RESOURCE_METHOD: &str = "world.inspect_resource";

/// The method path for a `world.inspect_all_resources` request.
pub const BRP_INSPECT_ALL_RESOURCES_METHOD: &str = "world.inspect_all_resources";

/// The method path for a `world.summarize` request.
pub const BRP_SUMMARIZE_METHOD: &str = "world.summarize";

/// The method path for a `registry.component_metadata` request.
pub const BRP_COMPONENT_METADATA_METHOD: &str = "registry.component_metadata";

/// The parameters of a `world.inspect` request.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BrpInspectParams {
    /// The entity to inspect.
    pub entity: Entity,

    /// The settings used for the inspection, defaulted when omitted.
    #[serde(default)]
    pub settings: Option<EntityInspectionSettings>,
}

/// The parameters of a `world.inspect_component` request.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BrpInspectComponentParams {
    /// The entity that owns the component.
    pub entity: Entity,

    /// The fully-qualified type name of the component to inspect.
    pub component: String,

    /// The settings used for the inspection, defaulted when omitted.
    #[serde(default)]
    pub settings: Option<ComponentInspectionSettings>,
}

/// The parameters of a `world.inspect_component_type` request.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BrpInspectComponentTypeParams {
    /// The fully-qualified type name of the component type to inspect.
    pub component: String,
}

/// The parameters of a `world.inspect_resource` request.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BrpInspectResourceParams {
    /// The fully-qualified type name of the resource to inspect.
    pub resource: String,

    /// The settings used for the inspection, defaulted when omitted.
    #[serde(default)]
    pub settings: Option<ResourceInspectionSettings>,
}

/// The parameters of a `world.inspect_all_resources` request.
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct BrpInspectAllResourcesParams {
    /// The settings used for the inspections, defaulted when omitted.
    #[serde(default)]
    pub settings: Option<ResourceInspectionSettings>,
}

/// The parameters of a `world.summarize` request.
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct BrpSummarizeParams {
    /// The settings used for the summary, defaulted when omitted.
    #[serde(default)]
    pub settings: Option<SummarySettings>,
}

/// Handles a `world.inspect` request coming from a client.
pub fn process_remote_inspect_request(In(params): In<Option<Value>>, world: &World) -> BrpResult {
    let BrpInspectParams { entity, settings } = parse_some(params)?;

    let mut settings = settings.unwrap_or_default();
    settings.component_settings.include_serialized_value = true;

    match world.inspect(entity, settings) {
        Ok(inspection) => serde_json::to_value(inspection).map_err(BrpError::internal),
        Err(error) => Err(entity_inspection_error(entity, error)),
    }
}

/// Handles a `world.inspect_component` request coming from a client.
pub fn process_remote_inspect_component_request(
    In(params): In<Option<Value>>,
    world: &World,
) -> BrpResult {
    let BrpInspectComponentParams {
        entity,
        component,
        settings,
    } = parse_some(params)?;

    let mut settings = settings.unwrap_or_default();
    settings.include_serialized_value = true;

    let metadata_map = ComponentMetadataMap::generate(world);
    let Some((component_id, metadata)) = metadata_map.get_component_metadata_by_name(&component)
    else {
        return Err(component_name_not_in_metadata(&component));
    };

    match world.inspect_component_by_id(component_id, entity, metadata, settings) {
        Ok(inspection) => serde_json::to_value(inspection).map_err(BrpError::internal),
        Err(error) => Err(component_inspection_error(&component, entity, error)),
    }
}

/// Handles a `world.inspect_component_type` request coming from a client.
pub fn process_remote_inspect_component_type_request(
    In(params): In<Option<Value>>,
    world: &World,
) -> BrpResult {
    let BrpInspectComponentTypeParams { component } = parse_some(params)?;

    let metadata_map = ComponentMetadataMap::generate(world);
    let Some((component_id, _)) = metadata_map.get_component_metadata_by_name(&component) else {
        return Err(component_name_not_in_metadata(&component));
    };

    match world.inspect_component_type_by_id(component_id) {
        Ok(inspection) => serde_json::to_value(inspection).map_err(BrpError::internal),
        Err(error) => Err(component_type_inspection_error(error)),
    }
}

/// Handles a `world.inspect_resource` request coming from a client.
pub fn process_remote_inspect_resource_request(
    In(params): In<Option<Value>>,
    world: &World,
) -> BrpResult {
    let BrpInspectResourceParams { resource, settings } = parse_some(params)?;

    let mut settings = settings.unwrap_or_default();
    settings.include_serialized_value = true;

    let metadata_map = ComponentMetadataMap::generate(world);
    let Some((component_id, _)) = metadata_map.get_component_metadata_by_name(&resource) else {
        return Err(component_name_not_in_metadata(&resource));
    };

    match world.inspect_resource_by_id(component_id, settings) {
        Ok(inspection) => serde_json::to_value(inspection).map_err(BrpError::internal),
        Err(error) => Err(resource_inspection_error(error)),
    }
}

/// Handles a `world.inspect_all_resources` request coming from a client.
pub fn process_remote_inspect_all_resources_request(
    In(params): In<Option<Value>>,
    world: &World,
) -> BrpResult {
    let BrpInspectAllResourcesParams { settings } = parse_optional(params)?;

    let mut settings = settings.unwrap_or_default();
    settings.include_serialized_value = true;

    serde_json::to_value(world.inspect_all_resources(settings)).map_err(BrpError::internal)
}

/// Handles a `world.summarize` request coming from a client.
pub fn process_remote_summarize_request(In(params): In<Option<Value>>, world: &World) -> BrpResult {
    let BrpSummarizeParams { settings } = parse_optional(params)?;

    serde_json::to_value(world.summarize(settings.unwrap_or_default())).map_err(BrpError::internal)
}

/// Handles a `registry.component_metadata` request coming from a client.
pub fn process_remote_component_metadata_request(
    In(_params): In<Option<Value>>,
    world: &World,
) -> BrpResult {
    serde_json::to_value(ComponentMetadataMap::generate(world)).map_err(BrpError::internal)
}

/// Parses request parameters that may be omitted entirely.
fn parse_optional<T: Default + for<'de> Deserialize<'de>>(
    params: Option<Value>,
) -> Result<T, BrpError> {
    match params {
        None | Some(Value::Null) => Ok(T::default()),
        Some(value) => parse(value),
    }
}

/// Builds the error returned when a component or resource name is absent from the metadata map.
fn component_name_not_in_metadata(name: &str) -> BrpError {
    BrpError {
        code: error_codes::COMPONENT_NAME_NOT_IN_METADATA,
        message: format!("Component not found in metadata: `{name}`"),
        data: serde_json::to_value(name).ok(),
    }
}

/// Converts an [`EntityInspectionError`] into a [`BrpError`].
fn entity_inspection_error(entity: Entity, error: EntityInspectionError) -> BrpError {
    match error {
        EntityInspectionError::EntityNotFound(_) => BrpError::entity_not_found(entity),
    }
}

/// Converts a [`ComponentInspectionError`] raised while inspecting a component on an entity.
fn component_inspection_error(
    component: &str,
    entity: Entity,
    error: ComponentInspectionError,
) -> BrpError {
    match error {
        ComponentInspectionError::ComponentNotFound(_) => {
            BrpError::component_not_present(component, entity)
        }
        other => component_type_inspection_error(other),
    }
}

/// Converts a [`ComponentInspectionError`] raised while inspecting a component type.
fn component_type_inspection_error(error: ComponentInspectionError) -> BrpError {
    BrpError::component_error(error)
}

/// Converts a [`ResourceInspectionError`] into a [`BrpError`].
fn resource_inspection_error(error: ResourceInspectionError) -> BrpError {
    match error {
        ResourceInspectionError::ResourceNotRegistered(name) => {
            BrpError::resource_error(format!("Resource type `{name}` is not registered"))
        }
        ResourceInspectionError::ResourceIdNotRegistered(component_id) => {
            BrpError::resource_not_present(&format!("{component_id:?}"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy_ecs::{
        component::Component,
        name::Name,
        reflect::{AppTypeRegistry, ReflectComponent, ReflectResource},
        resource::Resource,
    };
    use bevy_reflect::Reflect;
    use serde_json::json;

    #[derive(Component, Reflect, Debug)]
    #[reflect(Component)]
    struct Health(u32);

    #[derive(Resource, Reflect, Debug)]
    #[reflect(Resource)]
    struct Score(u32);

    fn test_world() -> World {
        let mut world = World::new();
        world.init_resource::<AppTypeRegistry>();
        {
            let registry = world.resource_mut::<AppTypeRegistry>();
            let mut registry = registry.write();
            registry.register::<Health>();
            registry.register::<Score>();
        }
        world.insert_resource(Score(11));
        world
    }

    fn health_type_path() -> String {
        core::any::type_name::<Health>().to_string()
    }

    #[test]
    fn inspect_returns_label_and_serialized_component() {
        let mut world = test_world();
        let entity = world.spawn((Name::new("Player"), Health(7))).id();

        let result = process_remote_inspect_request(In(Some(json!({ "entity": entity }))), &world)
            .expect("the entity should be inspectable");

        assert_eq!(result["label"]["label"], json!("Player"));

        let components = result["components"].as_array().unwrap();
        let health = components
            .iter()
            .find(|component| component["name"] == json!(health_type_path()))
            .expect("the `Health` component should be inspected");
        assert_eq!(health["serialized_value"], json!(7));
    }

    #[test]
    fn inspect_missing_entity_errors() {
        let mut world = test_world();
        let entity = world.spawn(Health(7)).id();
        world.despawn(entity);

        let error = process_remote_inspect_request(In(Some(json!({ "entity": entity }))), &world)
            .expect_err("a despawned entity should not be inspectable");

        assert_eq!(error.code, error_codes::ENTITY_NOT_FOUND);
    }

    #[test]
    fn inspect_component_by_type_path() {
        let mut world = test_world();
        let entity = world.spawn(Health(7)).id();

        let result = process_remote_inspect_component_request(
            In(Some(
                json!({ "entity": entity, "component": health_type_path() }),
            )),
            &world,
        )
        .expect("the component should be inspectable");

        assert_eq!(result["serialized_value"], json!(7));
    }

    #[test]
    fn inspect_component_type_counts_entities() {
        let mut world = test_world();
        world.spawn(Health(7));
        world.spawn(Health(9));

        let result = process_remote_inspect_component_type_request(
            In(Some(json!({ "component": health_type_path() }))),
            &world,
        )
        .expect("the component type should be inspectable");

        assert_eq!(result["entity_count"], json!(2));
    }

    #[test]
    fn inspect_unknown_component_name_errors() {
        let world = test_world();

        let error = process_remote_inspect_component_type_request(
            In(Some(json!({ "component": "not::a::Component" }))),
            &world,
        )
        .expect_err("an unknown component name should be rejected");

        assert_eq!(error.code, error_codes::COMPONENT_NAME_NOT_IN_METADATA);
    }

    #[test]
    fn inspect_resource_returns_serialized_value() {
        let world = test_world();

        let result = process_remote_inspect_resource_request(
            In(Some(json!({ "resource": core::any::type_name::<Score>() }))),
            &world,
        )
        .expect("the resource should be inspectable");

        assert_eq!(result["serialized_value"], json!(11));
    }

    #[test]
    fn inspect_all_resources_includes_inserted_resource() {
        let world = test_world();

        let result = process_remote_inspect_all_resources_request(In(None), &world)
            .expect("resources should be inspectable");

        assert!(result
            .as_array()
            .unwrap()
            .iter()
            .any(|inspection| inspection["name"] == json!(core::any::type_name::<Score>())));
    }

    #[test]
    fn summarize_reports_counts() {
        let mut world = test_world();
        world.spawn(Health(7));

        let result = process_remote_summarize_request(In(None), &world)
            .expect("the world should be summarizable");

        assert!(result["total_entities"].as_u64().unwrap() >= 1);
        assert!(result["total_archetypes"].as_u64().unwrap() >= 1);
        assert!(!result["archetype_summaries"].as_array().unwrap().is_empty());
    }

    #[test]
    fn component_metadata_contains_registered_component() {
        let mut world = test_world();
        world.spawn(Health(7));

        let result = process_remote_component_metadata_request(In(None), &world)
            .expect("the metadata map should be serializable");

        let map = result["map"].as_object().unwrap();
        assert!(map
            .values()
            .any(|metadata| metadata["name"] == json!(health_type_path())));
    }
}

//! Built-in verbs for the Bevy Remote Protocol.

use core::any::TypeId;

use anyhow::{anyhow, Result as AnyhowResult};
use bevy_ecs::{
    component::ComponentId,
    entity::Entity,
    event::EventCursor,
    hierarchy::ChildOf,
    lifecycle::RemovedComponentEntity,
    query::QueryBuilder,
    reflect::{AppTypeRegistry, ReflectComponent, ReflectResource},
    system::{In, Local},
    world::{EntityRef, EntityWorldMut, FilteredEntityRef, World},
};
use bevy_log::warn_once;
use bevy_platform::collections::HashMap;
use bevy_reflect::{
    serde::{ReflectSerializer, TypedReflectDeserializer},
    GetPath, PartialReflect, TypeRegistration, TypeRegistry,
};
use serde::{de::DeserializeSeed as _, Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::{
    error_codes,
    schemas::{
        json_schema::{export_type, JsonSchemaBevyType},
        open_rpc::OpenRpcDocument,
    },
    BrpError, BrpResult,
};

#[cfg(all(feature = "http", not(target_family = "wasm")))]
use {crate::schemas::open_rpc::ServerObject, bevy_utils::default};

/// The method path for a `world.get_components` request.
pub const BRP_GET_COMPONENTS_METHOD: &str = "world.get_components";

/// The method path for a `world.query` request.
pub const BRP_QUERY_METHOD: &str = "world.query";

/// The method path for a `world.spawn_entity` request.
pub const BRP_SPAWN_ENTITY_METHOD: &str = "world.spawn_entity";

/// The method path for a `world.insert_components` request.
pub const BRP_INSERT_COMPONENTS_METHOD: &str = "world.insert_components";

/// The method path for a `world.remove_components` request.
pub const BRP_REMOVE_COMPONENTS_METHOD: &str = "world.remove_components";

/// The method path for a `world.despawn_entity` request.
pub const BRP_DESPAWN_COMPONENTS_METHOD: &str = "world.despawn_entity";

/// The method path for a `world.reparent_entities` request.
pub const BRP_REPARENT_ENTITIES_METHOD: &str = "world.reparent_entities";

/// The method path for a `world.list_components` request.
pub const BRP_LIST_COMPONENTS_METHOD: &str = "world.list_components";

/// The method path for a `world.mutate_components` request.
pub const BRP_MUTATE_COMPONENTS_METHOD: &str = "world.mutate_components";

/// The method path for a `world.get_components+watch` request.
pub const BRP_GET_COMPONENTS_AND_WATCH_METHOD: &str = "world.get_components+watch";

/// The method path for a `world.list_components+watch` request.
pub const BRP_LIST_COMPONENTS_AND_WATCH_METHOD: &str = "world.list_components+watch";

/// The method path for a `world.get_resources` request.
pub const BRP_GET_RESOURCE_METHOD: &str = "world.get_resources";

/// The method path for a `world.insert_resources` request.
pub const BRP_INSERT_RESOURCE_METHOD: &str = "world.insert_resources";

/// The method path for a `world.remove_resources` request.
pub const BRP_REMOVE_RESOURCE_METHOD: &str = "world.remove_resources";

/// The method path for a `world.mutate_resources` request.
pub const BRP_MUTATE_RESOURCE_METHOD: &str = "world.mutate_resources";

/// The method path for a `world.list_resources` request.
pub const BRP_LIST_RESOURCES_METHOD: &str = "world.list_resources";

/// The method path for a `registry.schema` request.
pub const BRP_REGISTRY_SCHEMA_METHOD: &str = "registry.schema";

/// The method path for a `rpc.discover` request.
pub const RPC_DISCOVER_METHOD: &str = "rpc.discover";

/// `world.get_components`: Retrieves one or more components from the entity with the given
/// ID.
///
/// The server responds with a [`BrpGetComponentsResponse`].
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct BrpGetComponentsParams {
    /// The ID of the entity from which components are to be requested.
    pub entity: Entity,

    /// The [full paths] of the component types that are to be requested
    /// from the entity.
    ///
    /// Note that these strings must consist of the *full* type paths: e.g.
    /// `bevy_transform::components::transform::Transform`, not just
    /// `Transform`.
    ///
    /// [full paths]: bevy_reflect::TypePath::type_path
    pub components: Vec<String>,

    /// An optional flag to fail when encountering an invalid component rather
    /// than skipping it. Defaults to false.
    #[serde(default)]
    pub strict: bool,
}

/// `world.get_resources`: Retrieves the value of a given resource.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct BrpGetResourcesParams {
    /// The [full path] of the resource type being requested.
    ///
    /// [full path]: bevy_reflect::TypePath::type_path
    pub resource: String,
}

/// `world.query`: Performs a query over components in the ECS, returning entities
/// and component values that match.
///
/// The server responds with a [`BrpQueryResponse`].
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct BrpQueryParams {
    /// The components to select.
    pub data: BrpQuery,

    /// An optional filter that specifies which entities to include or
    /// exclude from the results.
    #[serde(default)]
    pub filter: BrpQueryFilter,

    /// An optional flag to fail when encountering an invalid component rather
    /// than skipping it. Defaults to false.
    #[serde(default)]
    pub strict: bool,
}

/// `world.spawn_entity`: Creates a new entity with the given components and responds
/// with its ID.
///
/// The server responds with a [`BrpSpawnEntityResponse`].
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct BrpSpawnEntityParams {
    /// A map from each component's full path to its serialized value.
    ///
    /// These components will be added to the entity.
    ///
    /// Note that the keys of the map must be the [full type paths]: e.g.
    /// `bevy_transform::components::transform::Transform`, not just
    /// `Transform`.
    ///
    /// [full type paths]: bevy_reflect::TypePath::type_path
    pub components: HashMap<String, Value>,
}

/// `world.despawn_entity`: Given an ID, despawns the entity with that ID.
///
/// The server responds with an okay.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct BrpDespawnEntityParams {
    /// The ID of the entity to despawn.
    pub entity: Entity,
}

/// `world.remove_components`: Deletes one or more components from an entity.
///
/// The server responds with a null.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct BrpRemoveComponentsParams {
    /// The ID of the entity from which components are to be removed.
    pub entity: Entity,

    /// The full paths of the component types that are to be removed from
    /// the entity.
    ///
    /// Note that these strings must consist of the [full type paths]: e.g.
    /// `bevy_transform::components::transform::Transform`, not just
    /// `Transform`.
    ///
    /// [full type paths]: bevy_reflect::TypePath::type_path
    pub components: Vec<String>,
}

/// `world.remove_resources`: Removes the given resource from the world.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct BrpRemoveResourcesParams {
    /// The [full path] of the resource type to remove.
    ///
    /// [full path]: bevy_reflect::TypePath::type_path
    pub resource: String,
}

/// `world.insert_components`: Adds one or more components to an entity.
///
/// The server responds with a null.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct BrpInsertComponentsParams {
    /// The ID of the entity that components are to be added to.
    pub entity: Entity,

    /// A map from each component's full path to its serialized value.
    ///
    /// These components will be added to the entity.
    ///
    /// Note that the keys of the map must be the [full type paths]: e.g.
    /// `bevy_transform::components::transform::Transform`, not just
    /// `Transform`.
    ///
    /// [full type paths]: bevy_reflect::TypePath::type_path
    pub components: HashMap<String, Value>,
}

/// `world.insert_resources`: Inserts a resource into the world with a given
/// value.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct BrpInsertResourcesParams {
    /// The [full path] of the resource type to insert.
    ///
    /// [full path]: bevy_reflect::TypePath::type_path
    pub resource: String,

    /// The serialized value of the resource to be inserted.
    pub value: Value,
}

/// `world.reparent_entities`: Assign a new parent to one or more entities.
///
/// The server responds with a null.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct BrpReparentEntitiesParams {
    /// The IDs of the entities that are to become the new children of the
    /// `parent`.
    pub entities: Vec<Entity>,

    /// The IDs of the entity that will become the new parent of the
    /// `entities`.
    ///
    /// If this is `None`, then the entities are removed from all parents.
    #[serde(default)]
    pub parent: Option<Entity>,
}

/// `world.list_components`: Returns a list of all type names of registered components in the
/// system (no params provided), or those on an entity (params provided).
///
/// The server responds with a [`BrpListComponentsResponse`]
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct BrpListComponentsParams {
    /// The entity to query.
    pub entity: Entity,
}

/// `world.mutate_components`:
///
/// The server responds with a null.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct BrpMutateComponentsParams {
    /// The entity of the component to mutate.
    pub entity: Entity,

    /// The [full path] of the component to mutate.
    ///
    /// [full path]: bevy_reflect::TypePath::type_path
    pub component: String,

    /// The [path] of the field within the component.
    ///
    /// [path]: bevy_reflect::GetPath
    pub path: String,

    /// The value to insert at `path`.
    pub value: Value,
}

/// `world.mutate_resources`:
///
/// The server responds with a null.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct BrpMutateResourcesParams {
    /// The [full path] of the resource to mutate.
    ///
    /// [full path]: bevy_reflect::TypePath::type_path
    pub resource: String,

    /// The [path] of the field within the resource.
    ///
    /// [path]: bevy_reflect::GetPath
    pub path: String,

    /// The value to insert at `path`.
    pub value: Value,
}

/// Describes the data that is to be fetched in a query.
#[derive(Debug, Serialize, Deserialize, Clone, Default, PartialEq)]
pub struct BrpQuery {
    /// The [full path] of the type name of each component that is to be
    /// fetched.
    ///
    /// [full path]: bevy_reflect::TypePath::type_path
    #[serde(default)]
    pub components: Vec<String>,

    /// The [full path] of the type name of each component that is to be
    /// optionally fetched.
    ///
    /// [full path]: bevy_reflect::TypePath::type_path
    #[serde(default)]
    pub option: ComponentSelector,

    /// The [full path] of the type name of each component that is to be checked
    /// for presence.
    ///
    /// [full path]: bevy_reflect::TypePath::type_path
    #[serde(default)]
    pub has: Vec<String>,
}

/// Additional constraints that can be placed on a query to include or exclude
/// certain entities.
#[derive(Debug, Serialize, Deserialize, Clone, Default, PartialEq)]
pub struct BrpQueryFilter {
    /// The [full path] of the type name of each component that must not be
    /// present on the entity for it to be included in the results.
    ///
    /// [full path]: bevy_reflect::TypePath::type_path
    #[serde(default)]
    pub without: Vec<String>,

    /// The [full path] of the type name of each component that must be present
    /// on the entity for it to be included in the results.
    ///
    /// [full path]: bevy_reflect::TypePath::type_path
    #[serde(default)]
    pub with: Vec<String>,
}

/// Constraints that can be placed on a query to include or exclude
/// certain definitions.
#[derive(Debug, Serialize, Deserialize, Clone, Default, PartialEq)]
pub struct BrpJsonSchemaQueryFilter {
    /// The crate name of the type name of each component that must not be
    /// present on the entity for it to be included in the results.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub without_crates: Vec<String>,

    /// The crate name of the type name of each component that must be present
    /// on the entity for it to be included in the results.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub with_crates: Vec<String>,

    /// Constrain resource by type
    #[serde(default)]
    pub type_limit: JsonSchemaTypeLimit,
}

/// Additional [`BrpJsonSchemaQueryFilter`] constraints that can be placed on a query to include or exclude
/// certain definitions.
#[derive(Debug, Serialize, Deserialize, Clone, Default, PartialEq)]
pub struct JsonSchemaTypeLimit {
    /// Schema cannot have specified reflect types
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub without: Vec<String>,

    /// Schema needs to have specified reflect types
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub with: Vec<String>,
}

/// A response from the world to the client that specifies a single entity.
///
/// This is sent in response to `world.spawn_entity`.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct BrpSpawnEntityResponse {
    /// The ID of the entity in question.
    pub entity: Entity,
}

/// The response to a `world.get_components` request.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(untagged)]
pub enum BrpGetComponentsResponse {
    /// The non-strict response that reports errors separately without failing the entire request.
    Lenient {
        /// A map of successful components with their values.
        components: HashMap<String, Value>,
        /// A map of unsuccessful components with their errors.
        errors: HashMap<String, Value>,
    },
    /// The strict response that will fail if any components are not present or aren't
    /// reflect-able.
    Strict(HashMap<String, Value>),
}

/// The response to a `world.get_resources` request.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct BrpGetResourcesResponse {
    /// The value of the requested resource.
    pub value: Value,
}

/// A single response from a `world.get_components+watch` request.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(untagged)]
pub enum BrpGetComponentsWatchingResponse {
    /// The non-strict response that reports errors separately without failing the entire request.
    Lenient {
        /// A map of successful components with their values that were added or changes in the last
        /// tick.
        components: HashMap<String, Value>,
        /// An array of components that were been removed in the last tick.
        removed: Vec<String>,
        /// A map of unsuccessful components with their errors.
        errors: HashMap<String, Value>,
    },
    /// The strict response that will fail if any components are not present or aren't
    /// reflect-able.
    Strict {
        /// A map of successful components with their values that were added or changes in the last
        /// tick.
        components: HashMap<String, Value>,
        /// An array of components that were been removed in the last tick.
        removed: Vec<String>,
    },
}

/// The response to a `world.list_components` request.
pub type BrpListComponentsResponse = Vec<String>;

/// The response to a `world.list_resources` request.
pub type BrpListResourcesResponse = Vec<String>;

/// A single response from a `world.list_components+watch` request.
#[derive(Debug, Default, Serialize, Deserialize, Clone, PartialEq)]
pub struct BrpListComponentsWatchingResponse {
    added: Vec<String>,
    removed: Vec<String>,
}

/// The response to a `world.query` request.
pub type BrpQueryResponse = Vec<BrpQueryRow>;

/// One query match result: a single entity paired with the requested components.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct BrpQueryRow {
    /// The ID of the entity that matched.
    pub entity: Entity,

    /// The serialized values of the requested components.
    pub components: HashMap<String, Value>,

    /// The boolean-only containment query results.
    #[serde(skip_serializing_if = "HashMap::is_empty", default)]
    pub has: HashMap<String, Value>,
}

/// A helper function used to parse a `serde_json::Value`.
fn parse<T: for<'de> Deserialize<'de>>(value: Value) -> Result<T, BrpError> {
    serde_json::from_value(value).map_err(|err| BrpError {
        code: error_codes::INVALID_PARAMS,
        message: err.to_string(),
        data: None,
    })
}

/// A helper function used to parse a `serde_json::Value` wrapped in an `Option`.
fn parse_some<T: for<'de> Deserialize<'de>>(value: Option<Value>) -> Result<T, BrpError> {
    match value {
        Some(value) => parse(value),
        None => Err(BrpError {
            code: error_codes::INVALID_PARAMS,
            message: String::from("Params not provided"),
            data: None,
        }),
    }
}

/// Handles a `world.get_components` request coming from a client.
pub fn process_remote_get_components_request(
    In(params): In<Option<Value>>,
    world: &World,
) -> BrpResult {
    let BrpGetComponentsParams {
        entity,
        components,
        strict,
    } = parse_some(params)?;

    let app_type_registry = world.resource::<AppTypeRegistry>();
    let type_registry = app_type_registry.read();
    let entity_ref = get_entity(world, entity)?;

    let response =
        reflect_components_to_response(components, strict, entity, entity_ref, &type_registry)?;
    serde_json::to_value(response).map_err(BrpError::internal)
}

/// Handles a `world.get_resources` request coming from a client.
pub fn process_remote_get_resources_request(
    In(params): In<Option<Value>>,
    world: &World,
) -> BrpResult {
    let BrpGetResourcesParams {
        resource: resource_path,
    } = parse_some(params)?;

    let app_type_registry = world.resource::<AppTypeRegistry>();
    let type_registry = app_type_registry.read();
    let reflect_resource =
        get_reflect_resource(&type_registry, &resource_path).map_err(BrpError::resource_error)?;

    let Ok(reflected) = reflect_resource.reflect(world) else {
        return Err(BrpError::resource_not_present(&resource_path));
    };

    // Use the `ReflectSerializer` to serialize the value of the resource;
    // this produces a map with a single item.
    let reflect_serializer = ReflectSerializer::new(reflected.as_partial_reflect(), &type_registry);
    let Value::Object(serialized_object) =
        serde_json::to_value(&reflect_serializer).map_err(BrpError::resource_error)?
    else {
        return Err(BrpError {
            code: error_codes::RESOURCE_ERROR,
            message: format!("Resource `{resource_path}` could not be serialized"),
            data: None,
        });
    };

    // Get the single value out of the map.
    let value = serialized_object.into_values().next().ok_or_else(|| {
        BrpError::internal(anyhow!("Unexpected format of serialized resource value"))
    })?;
    let response = BrpGetResourcesResponse { value };
    serde_json::to_value(response).map_err(BrpError::internal)
}

/// Handles a `world.get_components+watch` request coming from a client.
pub fn process_remote_get_components_watching_request(
    In(params): In<Option<Value>>,
    world: &World,
    mut removal_cursors: Local<HashMap<ComponentId, EventCursor<RemovedComponentEntity>>>,
) -> BrpResult<Option<Value>> {
    let BrpGetComponentsParams {
        entity,
        components,
        strict,
    } = parse_some(params)?;

    let app_type_registry = world.resource::<AppTypeRegistry>();
    let type_registry = app_type_registry.read();
    let entity_ref = get_entity(world, entity)?;

    let mut changed = Vec::new();
    let mut removed = Vec::new();
    let mut errors = <HashMap<_, _>>::default();

    'component_loop: for component_path in components {
        let Ok(type_registration) =
            get_component_type_registration(&type_registry, &component_path)
        else {
            let err =
                BrpError::component_error(format!("Unknown component type: `{component_path}`"));
            if strict {
                return Err(err);
            }
            errors.insert(
                component_path,
                serde_json::to_value(err).map_err(BrpError::internal)?,
            );
            continue;
        };
        let Some(component_id) = world.components().get_valid_id(type_registration.type_id())
        else {
            let err = BrpError::component_error(format!("Unknown component: `{component_path}`"));
            if strict {
                return Err(err);
            }
            errors.insert(
                component_path,
                serde_json::to_value(err).map_err(BrpError::internal)?,
            );
            continue;
        };

        if let Some(ticks) = entity_ref.get_change_ticks_by_id(component_id)
            && ticks.is_changed(world.last_change_tick(), world.read_change_tick())
        {
            changed.push(component_path);
            continue;
        };

        let Some(events) = world.removed_components().get(component_id) else {
            continue;
        };
        let cursor = removal_cursors
            .entry(component_id)
            .or_insert_with(|| events.get_cursor());
        for event in cursor.read(events) {
            if Entity::from(event.clone()) == entity {
                removed.push(component_path);
                continue 'component_loop;
            }
        }
    }

    if changed.is_empty() && removed.is_empty() {
        return Ok(None);
    }

    let response =
        reflect_components_to_response(changed, strict, entity, entity_ref, &type_registry)?;

    let response = match response {
        BrpGetComponentsResponse::Lenient {
            components,
            errors: mut errs,
        } => BrpGetComponentsWatchingResponse::Lenient {
            components,
            removed,
            errors: {
                errs.extend(errors);
                errs
            },
        },
        BrpGetComponentsResponse::Strict(components) => BrpGetComponentsWatchingResponse::Strict {
            components,
            removed,
        },
    };

    Ok(Some(
        serde_json::to_value(response).map_err(BrpError::internal)?,
    ))
}

/// Reflect a list of components on an entity into a [`BrpGetComponentsResponse`].
fn reflect_components_to_response(
    components: Vec<String>,
    strict: bool,
    entity: Entity,
    entity_ref: EntityRef,
    type_registry: &TypeRegistry,
) -> BrpResult<BrpGetComponentsResponse> {
    let mut response = if strict {
        BrpGetComponentsResponse::Strict(Default::default())
    } else {
        BrpGetComponentsResponse::Lenient {
            components: Default::default(),
            errors: Default::default(),
        }
    };

    for component_path in components {
        match reflect_component(&component_path, entity, entity_ref, type_registry) {
            Ok(serialized_object) => match response {
                BrpGetComponentsResponse::Strict(ref mut components)
                | BrpGetComponentsResponse::Lenient {
                    ref mut components, ..
                } => {
                    components.extend(serialized_object.into_iter());
                }
            },
            Err(err) => match response {
                BrpGetComponentsResponse::Strict(_) => return Err(err),
                BrpGetComponentsResponse::Lenient { ref mut errors, .. } => {
                    let err_value = serde_json::to_value(err).map_err(BrpError::internal)?;
                    errors.insert(component_path, err_value);
                }
            },
        }
    }

    Ok(response)
}

/// Reflect a single component on an entity with the given component path.
fn reflect_component(
    component_path: &str,
    entity: Entity,
    entity_ref: EntityRef,
    type_registry: &TypeRegistry,
) -> BrpResult<Map<String, Value>> {
    let reflect_component =
        get_reflect_component(type_registry, component_path).map_err(BrpError::component_error)?;

    // Retrieve the reflected value for the given specified component on the given entity.
    let Some(reflected) = reflect_component.reflect(entity_ref) else {
        return Err(BrpError::component_not_present(component_path, entity));
    };

    // Each component value serializes to a map with a single entry.
    let reflect_serializer = ReflectSerializer::new(reflected.as_partial_reflect(), type_registry);
    let Value::Object(serialized_object) =
        serde_json::to_value(&reflect_serializer).map_err(BrpError::component_error)?
    else {
        return Err(BrpError {
            code: error_codes::COMPONENT_ERROR,
            message: format!("Component `{component_path}` could not be serialized"),
            data: None,
        });
    };

    Ok(serialized_object)
}

/// A selector for components in a query.
///
/// This can either be a list of component paths or an "all" selector that
/// indicates that all components should be selected.
/// The "all" selector is useful when you want to retrieve all components
/// present on an entity without specifying each one individually.
/// The paths in the `Paths` variant must be the [full type paths]: e.g.
/// `bevy_transform::components::transform::Transform`, not just
/// `Transform`.
///
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ComponentSelector {
    /// An "all" selector that indicates all components should be selected.
    All,
    /// A list of component paths to select as optional components.
    #[serde(untagged)]
    Paths(Vec<String>),
}

impl Default for ComponentSelector {
    fn default() -> Self {
        Self::Paths(Vec::default())
    }
}

/// Handles a `world.query` request coming from a client.
pub fn process_remote_query_request(In(params): In<Option<Value>>, world: &mut World) -> BrpResult {
    let BrpQueryParams {
        data: BrpQuery {
            components,
            option,
            has,
        },
        filter,
        strict,
    } = match params {
        Some(params) => parse_some(Some(params))?,
        None => BrpQueryParams {
            data: BrpQuery {
                components: Vec::new(),
                option: ComponentSelector::default(),
                has: Vec::new(),
            },
            filter: BrpQueryFilter::default(),
            strict: false,
        },
    };

    let app_type_registry = world.resource::<AppTypeRegistry>().clone();
    let type_registry = app_type_registry.read();

    // Required components: must be present
    let (required, unregistered_in_required) =
        get_component_ids(&type_registry, world, components.clone(), strict)
            .map_err(BrpError::component_error)?;

    // Optional components: Option<&T> or all reflectable if "all"
    let (optional, _) = match &option {
        ComponentSelector::Paths(paths) => {
            get_component_ids(&type_registry, world, paths.clone(), strict)
                .map_err(BrpError::component_error)?
        }
        ComponentSelector::All => (Vec::new(), Vec::new()),
    };

    // Has components: presence check
    let (has_ids, unregistered_in_has) =
        get_component_ids(&type_registry, world, has, strict).map_err(BrpError::component_error)?;

    // Filters
    let (without, _) = get_component_ids(&type_registry, world, filter.without.clone(), strict)
        .map_err(BrpError::component_error)?;
    let (with, unregistered_in_with) =
        get_component_ids(&type_registry, world, filter.with.clone(), strict)
            .map_err(BrpError::component_error)?;

    // When "strict" is false:
    // - Unregistered components in "option" and "without" are ignored.
    // - Unregistered components in "has" are considered absent from the entity.
    // - Unregistered components in "components" and "with" result in an empty
    // response since they specify hard requirements.
    // If strict, fail if any required or with components are unregistered
    if !unregistered_in_required.is_empty() || !unregistered_in_with.is_empty() {
        return serde_json::to_value(BrpQueryResponse::default()).map_err(BrpError::internal);
    }

    let mut query = QueryBuilder::<FilteredEntityRef>::new(world);
    for (_, component) in &required {
        query.ref_id(*component);
    }
    for (_, option) in &optional {
        query.optional(|query| {
            query.ref_id(*option);
        });
    }
    for (_, has) in &has_ids {
        query.optional(|query| {
            query.ref_id(*has);
        });
    }
    for (_, without) in without {
        query.without_id(without);
    }
    for (_, with) in with {
        query.with_id(with);
    }

    // Prepare has reflect info
    let has_paths_and_reflect_components: Vec<(&str, &ReflectComponent)> = has_ids
        .iter()
        .map(|(type_id, _)| reflect_component_from_id(*type_id, &type_registry))
        .collect::<AnyhowResult<Vec<(&str, &ReflectComponent)>>>()
        .map_err(BrpError::component_error)?;

    let mut response = BrpQueryResponse::default();
    let mut query = query.build();

    for row in query.iter(world) {
        let entity_id = row.id();
        let entity_ref = world.get_entity(entity_id).expect("Entity should exist");

        // Required components
        let mut components_map = serialize_components(
            entity_ref,
            &type_registry,
            required
                .iter()
                .map(|(type_id, component_id)| (*type_id, Some(*component_id))),
        );

        // Optional components
        match &option {
            ComponentSelector::All => {
                // Add all reflectable components present on the entity (as Option<&T>)
                let all_optionals =
                    entity_ref
                        .archetype()
                        .components()
                        .filter_map(|component_id| {
                            let info = world.components().get_info(component_id)?;
                            let type_id = info.type_id()?;
                            // Skip required components (already included)
                            if required.iter().any(|(_, cid)| cid == &component_id) {
                                return None;
                            }
                            Some((type_id, Some(component_id)))
                        });
                components_map.extend(serialize_components(
                    entity_ref,
                    &type_registry,
                    all_optionals,
                ));
            }
            ComponentSelector::Paths(_) => {
                // Add only the requested optional components (as Option<&T>)
                let optionals = optional.iter().filter(|(_, component_id)| {
                    // Skip required components (already included)
                    !required.iter().any(|(_, cid)| cid == component_id)
                });
                components_map.extend(serialize_components(
                    entity_ref,
                    &type_registry,
                    optionals
                        .clone()
                        .map(|(type_id, component_id)| (*type_id, Some(*component_id))),
                ));
            }
        }

        // The map of boolean-valued component presences:
        let has_map = build_has_map(
            row,
            has_paths_and_reflect_components.iter().copied(),
            &unregistered_in_has,
        );

        let query_row = BrpQueryRow {
            entity: row.id(),
            components: components_map,
            has: has_map,
        };

        response.push(query_row);
    }

    serde_json::to_value(response).map_err(BrpError::internal)
}

/// Serializes the specified components for an entity.
/// The iterator yields ([`TypeId`], Option<[`ComponentId`]>).
fn serialize_components(
    entity_ref: EntityRef,
    type_registry: &TypeRegistry,
    components: impl Iterator<Item = (TypeId, Option<ComponentId>)>,
) -> HashMap<String, Value> {
    let mut components_map = HashMap::new();
    for (type_id, component_id_opt) in components {
        let Some(type_registration) = type_registry.get(type_id) else {
            continue;
        };
        if let Some(reflect_component) = type_registration.data::<ReflectComponent>() {
            // If a component_id is provided, check if the entity has it
            if let Some(component_id) = component_id_opt
                && !entity_ref.contains_id(component_id)
            {
                continue;
            }
            if let Some(reflected) = reflect_component.reflect(entity_ref) {
                let reflect_serializer =
                    ReflectSerializer::new(reflected.as_partial_reflect(), type_registry);
                if let Ok(Value::Object(obj)) = serde_json::to_value(&reflect_serializer) {
                    components_map.extend(obj);
                } else {
                    warn_once!(
                        "Failed to serialize component `{}` for entity {:?}",
                        type_registration.type_info().type_path(),
                        entity_ref.id()
                    );
                }
            }
        }
    }
    components_map
}

/// Handles a `world.spawn_entity` request coming from a client.
pub fn process_remote_spawn_entity_request(
    In(params): In<Option<Value>>,
    world: &mut World,
) -> BrpResult {
    let BrpSpawnEntityParams { components } = parse_some(params)?;

    let app_type_registry = world.resource::<AppTypeRegistry>().clone();
    let type_registry = app_type_registry.read();

    let reflect_components =
        deserialize_components(&type_registry, components).map_err(BrpError::component_error)?;

    let entity = world.spawn_empty();
    let entity_id = entity.id();
    insert_reflected_components(&type_registry, entity, reflect_components)
        .map_err(BrpError::component_error)?;

    let response = BrpSpawnEntityResponse { entity: entity_id };
    serde_json::to_value(response).map_err(BrpError::internal)
}

/// Handles a `rpc.discover` request coming from a client.
pub fn process_remote_list_methods_request(
    In(_params): In<Option<Value>>,
    world: &mut World,
) -> BrpResult {
    let remote_methods = world.resource::<crate::RemoteMethods>();

    #[cfg(all(feature = "http", not(target_family = "wasm")))]
    let servers = match (
        world.get_resource::<crate::http::HostAddress>(),
        world.get_resource::<crate::http::HostPort>(),
    ) {
        (Some(url), Some(port)) => Some(vec![ServerObject {
            name: "Server".to_owned(),
            url: format!("{}:{}", url.0, port.0),
            ..default()
        }]),
        (Some(url), None) => Some(vec![ServerObject {
            name: "Server".to_owned(),
            url: url.0.to_string(),
            ..default()
        }]),
        _ => None,
    };

    #[cfg(any(not(feature = "http"), target_family = "wasm"))]
    let servers = None;

    let doc = OpenRpcDocument {
        info: Default::default(),
        methods: remote_methods.into(),
        openrpc: "1.3.2".to_owned(),
        servers,
    };

    serde_json::to_value(doc).map_err(BrpError::internal)
}

/// Handles a `world.insert_components` request (insert components) coming from a client.
pub fn process_remote_insert_components_request(
    In(params): In<Option<Value>>,
    world: &mut World,
) -> BrpResult {
    let BrpInsertComponentsParams { entity, components } = parse_some(params)?;

    let app_type_registry = world.resource::<AppTypeRegistry>().clone();
    let type_registry = app_type_registry.read();

    let reflect_components =
        deserialize_components(&type_registry, components).map_err(BrpError::component_error)?;

    insert_reflected_components(
        &type_registry,
        get_entity_mut(world, entity)?,
        reflect_components,
    )
    .map_err(BrpError::component_error)?;

    Ok(Value::Null)
}

/// Handles a `world.insert_resources` request coming from a client.
pub fn process_remote_insert_resources_request(
    In(params): In<Option<Value>>,
    world: &mut World,
) -> BrpResult {
    let BrpInsertResourcesParams {
        resource: resource_path,
        value,
    } = parse_some(params)?;

    let app_type_registry = world.resource::<AppTypeRegistry>().clone();
    let type_registry = app_type_registry.read();

    let reflected_resource = deserialize_resource(&type_registry, &resource_path, value)
        .map_err(BrpError::resource_error)?;

    let reflect_resource =
        get_reflect_resource(&type_registry, &resource_path).map_err(BrpError::resource_error)?;
    reflect_resource.insert(world, &*reflected_resource, &type_registry);

    Ok(Value::Null)
}

/// Handles a `world.mutate_components` request coming from a client.
///
/// This method allows you to mutate a single field inside an Entity's
/// component.
pub fn process_remote_mutate_components_request(
    In(params): In<Option<Value>>,
    world: &mut World,
) -> BrpResult {
    let BrpMutateComponentsParams {
        entity,
        component,
        path,
        value,
    } = parse_some(params)?;
    let app_type_registry = world.resource::<AppTypeRegistry>().clone();
    let type_registry = app_type_registry.read();

    // Get the fully-qualified type names of the component to be mutated.
    let component_type: &TypeRegistration = type_registry
        .get_with_type_path(&component)
        .ok_or_else(|| {
            BrpError::component_error(anyhow!("Unknown component type: `{}`", component))
        })?;

    // Get the reflected representation of the component.
    let mut reflected = component_type
        .data::<ReflectComponent>()
        .ok_or_else(|| {
            BrpError::component_error(anyhow!("Component `{}` isn't registered", component))
        })?
        .reflect_mut(world.entity_mut(entity))
        .ok_or_else(|| {
            BrpError::component_error(anyhow!("Cannot reflect component `{}`", component))
        })?;

    // Get the type of the field in the component that is to be
    // mutated.
    let value_type: &TypeRegistration = type_registry
        .get_with_type_path(
            reflected
                .reflect_path(path.as_str())
                .map_err(BrpError::component_error)?
                .reflect_type_path(),
        )
        .ok_or_else(|| {
            BrpError::component_error(anyhow!("Unknown component field type: `{}`", component))
        })?;

    // Get the reflected representation of the value to be inserted
    // into the component.
    let value: Box<dyn PartialReflect> = TypedReflectDeserializer::new(value_type, &type_registry)
        .deserialize(&value)
        .map_err(BrpError::component_error)?;

    // Apply the mutation.
    reflected
        .reflect_path_mut(path.as_str())
        .map_err(BrpError::component_error)?
        .try_apply(value.as_ref())
        .map_err(BrpError::component_error)?;

    Ok(Value::Null)
}

/// Handles a `world.mutate_resources` request coming from a client.
pub fn process_remote_mutate_resources_request(
    In(params): In<Option<Value>>,
    world: &mut World,
) -> BrpResult {
    let BrpMutateResourcesParams {
        resource: resource_path,
        path: field_path,
        value,
    } = parse_some(params)?;

    let app_type_registry = world.resource::<AppTypeRegistry>().clone();
    let type_registry = app_type_registry.read();

    // Get the `ReflectResource` for the given resource path.
    let reflect_resource =
        get_reflect_resource(&type_registry, &resource_path).map_err(BrpError::resource_error)?;

    // Get the actual resource value from the world as a `dyn Reflect`.
    let mut reflected_resource = reflect_resource
        .reflect_mut(world)
        .map_err(|_| BrpError::resource_not_present(&resource_path))?;

    // Get the type registration for the field with the given path.
    let value_registration = type_registry
        .get_with_type_path(
            reflected_resource
                .reflect_path(field_path.as_str())
                .map_err(BrpError::resource_error)?
                .reflect_type_path(),
        )
        .ok_or_else(|| {
            BrpError::resource_error(anyhow!("Unknown resource field type: `{}`", resource_path))
        })?;

    // Use the field's type registration to deserialize the given value.
    let deserialized_value: Box<dyn PartialReflect> =
        TypedReflectDeserializer::new(value_registration, &type_registry)
            .deserialize(&value)
            .map_err(BrpError::resource_error)?;

    // Apply the value to the resource.
    reflected_resource
        .reflect_path_mut(field_path.as_str())
        .map_err(BrpError::resource_error)?
        .try_apply(&*deserialized_value)
        .map_err(BrpError::resource_error)?;

    Ok(Value::Null)
}

/// Handles a `world.remove_components` request (remove components) coming from a client.
pub fn process_remote_remove_components_request(
    In(params): In<Option<Value>>,
    world: &mut World,
) -> BrpResult {
    let BrpRemoveComponentsParams { entity, components } = parse_some(params)?;

    let app_type_registry = world.resource::<AppTypeRegistry>().clone();
    let type_registry = app_type_registry.read();

    let component_ids = get_component_ids(&type_registry, world, components, true)
        .and_then(|(registered, unregistered)| {
            if unregistered.is_empty() {
                Ok(registered)
            } else {
                Err(anyhow!("Unregistered component types: {:?}", unregistered))
            }
        })
        .map_err(BrpError::component_error)?;

    // Remove the components.
    let mut entity_world_mut = get_entity_mut(world, entity)?;
    for (_, component_id) in component_ids.iter() {
        entity_world_mut.remove_by_id(*component_id);
    }

    Ok(Value::Null)
}

/// Handles a `world.remove_resources` request coming from a client.
pub fn process_remote_remove_resources_request(
    In(params): In<Option<Value>>,
    world: &mut World,
) -> BrpResult {
    let BrpRemoveResourcesParams {
        resource: resource_path,
    } = parse_some(params)?;

    let app_type_registry = world.resource::<AppTypeRegistry>().clone();
    let type_registry = app_type_registry.read();

    let reflect_resource =
        get_reflect_resource(&type_registry, &resource_path).map_err(BrpError::resource_error)?;
    reflect_resource.remove(world);

    Ok(Value::Null)
}

/// Handles a `world.despawn_entity` (despawn entity) request coming from a client.
pub fn process_remote_despawn_entity_request(
    In(params): In<Option<Value>>,
    world: &mut World,
) -> BrpResult {
    let BrpDespawnEntityParams { entity } = parse_some(params)?;

    get_entity_mut(world, entity)?.despawn();

    Ok(Value::Null)
}

/// Handles a `world.reparent_entities` request coming from a client.
pub fn process_remote_reparent_entities_request(
    In(params): In<Option<Value>>,
    world: &mut World,
) -> BrpResult {
    let BrpReparentEntitiesParams {
        entities,
        parent: maybe_parent,
    } = parse_some(params)?;

    // If `Some`, reparent the entities.
    if let Some(parent) = maybe_parent {
        let mut parent_commands =
            get_entity_mut(world, parent).map_err(|_| BrpError::entity_not_found(parent))?;
        for entity in entities {
            if entity == parent {
                return Err(BrpError::self_reparent(entity));
            }
            parent_commands.add_child(entity);
        }
    }
    // If `None`, remove the entities' parents.
    else {
        for entity in entities {
            get_entity_mut(world, entity)?.remove::<ChildOf>();
        }
    }

    Ok(Value::Null)
}

/// Handles a `world.list_components` request (list all components) coming from a client.
pub fn process_remote_list_components_request(
    In(params): In<Option<Value>>,
    world: &World,
) -> BrpResult {
    let app_type_registry = world.resource::<AppTypeRegistry>();
    let type_registry = app_type_registry.read();

    let mut response = BrpListComponentsResponse::default();

    // If `Some`, return all components of the provided entity.
    if let Some(BrpListComponentsParams { entity }) = params.map(parse).transpose()? {
        let entity = get_entity(world, entity)?;
        for component_id in entity.archetype().components() {
            let Some(component_info) = world.components().get_info(component_id) else {
                continue;
            };
            response.push(component_info.name().to_string());
        }
    }
    // If `None`, list all registered components.
    else {
        for registered_type in type_registry.iter() {
            if registered_type.data::<ReflectComponent>().is_some() {
                response.push(registered_type.type_info().type_path().to_owned());
            }
        }
    }

    // Sort both for cleanliness and to reduce the risk that clients start
    // accidentally depending on the order.
    response.sort();

    serde_json::to_value(response).map_err(BrpError::internal)
}

/// Handles a `world.list_resources` request coming from a client.
pub fn process_remote_list_resources_request(
    In(_params): In<Option<Value>>,
    world: &World,
) -> BrpResult {
    let mut response = BrpListResourcesResponse::default();

    let app_type_registry = world.resource::<AppTypeRegistry>();
    let type_registry = app_type_registry.read();

    for registered_type in type_registry.iter() {
        if registered_type.data::<ReflectResource>().is_some() {
            response.push(registered_type.type_info().type_path().to_owned());
        }
    }

    response.sort();

    serde_json::to_value(response).map_err(BrpError::internal)
}

/// Handles a `world.list_components+watch` request coming from a client.
pub fn process_remote_list_components_watching_request(
    In(params): In<Option<Value>>,
    world: &World,
    mut removal_cursors: Local<HashMap<ComponentId, EventCursor<RemovedComponentEntity>>>,
) -> BrpResult<Option<Value>> {
    let BrpListComponentsParams { entity } = parse_some(params)?;
    let entity_ref = get_entity(world, entity)?;
    let mut response = BrpListComponentsWatchingResponse::default();

    for component_id in entity_ref.archetype().components() {
        let ticks = entity_ref
            .get_change_ticks_by_id(component_id)
            .ok_or(BrpError::internal("Failed to get ticks"))?;

        if ticks.is_added(world.last_change_tick(), world.read_change_tick()) {
            let Some(component_info) = world.components().get_info(component_id) else {
                continue;
            };
            response.added.push(component_info.name().to_string());
        }
    }

    for (component_id, events) in world.removed_components().iter() {
        let cursor = removal_cursors
            .entry(*component_id)
            .or_insert_with(|| events.get_cursor());
        for event in cursor.read(events) {
            if Entity::from(event.clone()) == entity {
                let Some(component_info) = world.components().get_info(*component_id) else {
                    continue;
                };
                response.removed.push(component_info.name().to_string());
            }
        }
    }

    if response.added.is_empty() && response.removed.is_empty() {
        Ok(None)
    } else {
        Ok(Some(
            serde_json::to_value(response).map_err(BrpError::internal)?,
        ))
    }
}

/// Handles a `registry.schema` request (list all registry types in form of schema) coming from a client.
pub fn export_registry_types(In(params): In<Option<Value>>, world: &World) -> BrpResult {
    let filter: BrpJsonSchemaQueryFilter = match params {
        None => Default::default(),
        Some(params) => parse(params)?,
    };

    let extra_info = world.resource::<crate::schemas::SchemaTypesMetadata>();
    let types = world.resource::<AppTypeRegistry>();
    let types = types.read();
    let schemas = types
        .iter()
        .filter_map(|type_reg| {
            let path_table = type_reg.type_info().type_path_table();
            if let Some(crate_name) = &path_table.crate_name() {
                if !filter.with_crates.is_empty()
                    && !filter.with_crates.iter().any(|c| crate_name.eq(c))
                {
                    return None;
                }
                if !filter.without_crates.is_empty()
                    && filter.without_crates.iter().any(|c| crate_name.eq(c))
                {
                    return None;
                }
            }
            let (id, schema) = export_type(type_reg, extra_info);

            if !filter.type_limit.with.is_empty()
                && !filter
                    .type_limit
                    .with
                    .iter()
                    .any(|c| schema.reflect_types.iter().any(|cc| c.eq(cc)))
            {
                return None;
            }
            if !filter.type_limit.without.is_empty()
                && filter
                    .type_limit
                    .without
                    .iter()
                    .any(|c| schema.reflect_types.iter().any(|cc| c.eq(cc)))
            {
                return None;
            }
            Some((id.to_string(), schema))
        })
        .collect::<HashMap<String, JsonSchemaBevyType>>();

    serde_json::to_value(schemas).map_err(BrpError::internal)
}

/// Immutably retrieves an entity from the [`World`], returning an error if the
/// entity isn't present.
fn get_entity(world: &World, entity: Entity) -> Result<EntityRef<'_>, BrpError> {
    world
        .get_entity(entity)
        .map_err(|_| BrpError::entity_not_found(entity))
}

/// Mutably retrieves an entity from the [`World`], returning an error if the
/// entity isn't present.
fn get_entity_mut(world: &mut World, entity: Entity) -> Result<EntityWorldMut<'_>, BrpError> {
    world
        .get_entity_mut(entity)
        .map_err(|_| BrpError::entity_not_found(entity))
}

/// Given components full path, returns a tuple that contains
/// - A list of corresponding [`TypeId`] and [`ComponentId`] for registered components.
/// - A list of unregistered component paths.
///
/// Note that the supplied path names must be *full* path names: e.g.
/// `bevy_transform::components::transform::Transform` instead of `Transform`.
fn get_component_ids(
    type_registry: &TypeRegistry,
    world: &World,
    component_paths: Vec<String>,
    strict: bool,
) -> AnyhowResult<(Vec<(TypeId, ComponentId)>, Vec<String>)> {
    let mut component_ids = vec![];
    let mut unregistered_components = vec![];

    for component_path in component_paths {
        let maybe_component_tuple = get_component_type_registration(type_registry, &component_path)
            .ok()
            .and_then(|type_registration| {
                let type_id = type_registration.type_id();
                world
                    .components()
                    .get_valid_id(type_id)
                    .map(|component_id| (type_id, component_id))
            });
        if let Some((type_id, component_id)) = maybe_component_tuple {
            component_ids.push((type_id, component_id));
        } else if strict {
            return Err(anyhow!(
                "Component `{}` isn't registered or used in the world",
                component_path
            ));
        } else {
            unregistered_components.push(component_path);
        }
    }

    Ok((component_ids, unregistered_components))
}

/// Given an entity (`entity_ref`),
/// a list of reflected component information (`paths_and_reflect_components`)
/// and a list of unregistered components,
/// return a map which associates each component to a boolean value indicating
/// whether or not that component is present on the entity.
/// Unregistered components are considered absent from the entity.
fn build_has_map<'a>(
    entity_ref: FilteredEntityRef,
    paths_and_reflect_components: impl Iterator<Item = (&'a str, &'a ReflectComponent)>,
    unregistered_components: &[String],
) -> HashMap<String, Value> {
    let mut has_map = <HashMap<_, _>>::default();

    for (type_path, reflect_component) in paths_and_reflect_components {
        let has = reflect_component.contains(entity_ref);
        has_map.insert(type_path.to_owned(), Value::Bool(has));
    }
    unregistered_components.iter().for_each(|component| {
        has_map.insert(component.to_owned(), Value::Bool(false));
    });

    has_map
}

/// Given a component ID, return the associated [type path] and `ReflectComponent` if possible.
///
/// The `ReflectComponent` part is the meat of this; the type path is only used for error messages.
///
/// [type path]: bevy_reflect::TypePath::type_path
fn reflect_component_from_id(
    component_type_id: TypeId,
    type_registry: &TypeRegistry,
) -> AnyhowResult<(&str, &ReflectComponent)> {
    let Some(type_registration) = type_registry.get(component_type_id) else {
        return Err(anyhow!(
            "Component `{:?}` isn't registered",
            component_type_id
        ));
    };

    let type_path = type_registration.type_info().type_path();

    let Some(reflect_component) = type_registration.data::<ReflectComponent>() else {
        return Err(anyhow!("Component `{}` isn't reflectable", type_path));
    };

    Ok((type_path, reflect_component))
}

/// Given a collection of component paths and their associated serialized values (`components`),
/// return the associated collection of deserialized reflected values.
fn deserialize_components(
    type_registry: &TypeRegistry,
    components: HashMap<String, Value>,
) -> AnyhowResult<Vec<Box<dyn PartialReflect>>> {
    let mut reflect_components = vec![];

    for (component_path, component) in components {
        let Some(component_type) = type_registry.get_with_type_path(&component_path) else {
            return Err(anyhow!("Unknown component type: `{}`", component_path));
        };
        let reflected: Box<dyn PartialReflect> =
            TypedReflectDeserializer::new(component_type, type_registry)
                .deserialize(&component)
                .map_err(|err| anyhow!("{component_path} is invalid: {err}"))?;
        reflect_components.push(reflected);
    }

    Ok(reflect_components)
}

/// Given a resource path and an associated serialized value (`value`), return the
/// deserialized value.
fn deserialize_resource(
    type_registry: &TypeRegistry,
    resource_path: &str,
    value: Value,
) -> AnyhowResult<Box<dyn PartialReflect>> {
    let Some(resource_type) = type_registry.get_with_type_path(resource_path) else {
        return Err(anyhow!("Unknown resource type: `{}`", resource_path));
    };
    let reflected: Box<dyn PartialReflect> =
        TypedReflectDeserializer::new(resource_type, type_registry)
            .deserialize(&value)
            .map_err(|err| anyhow!("{resource_path} is invalid: {err}"))?;
    Ok(reflected)
}

/// Given a collection `reflect_components` of reflected component values, insert them into
/// the given entity (`entity_world_mut`).
fn insert_reflected_components(
    type_registry: &TypeRegistry,
    mut entity_world_mut: EntityWorldMut,
    reflect_components: Vec<Box<dyn PartialReflect>>,
) -> AnyhowResult<()> {
    for reflected in reflect_components {
        let reflect_component =
            get_reflect_component(type_registry, reflected.reflect_type_path())?;
        reflect_component.insert(&mut entity_world_mut, &*reflected, type_registry);
    }

    Ok(())
}

/// Given a component's type path, return the associated [`ReflectComponent`] from the given
/// `type_registry` if possible.
fn get_reflect_component<'r>(
    type_registry: &'r TypeRegistry,
    component_path: &str,
) -> AnyhowResult<&'r ReflectComponent> {
    let component_registration = get_component_type_registration(type_registry, component_path)?;

    component_registration
        .data::<ReflectComponent>()
        .ok_or_else(|| anyhow!("Component `{}` isn't reflectable", component_path))
}

/// Given a component's type path, return the associated [`TypeRegistration`] from the given
/// `type_registry` if possible.
fn get_component_type_registration<'r>(
    type_registry: &'r TypeRegistry,
    component_path: &str,
) -> AnyhowResult<&'r TypeRegistration> {
    type_registry
        .get_with_type_path(component_path)
        .ok_or_else(|| anyhow!("Unknown component type: `{}`", component_path))
}

/// Given a resource's type path, return the associated [`ReflectResource`] from the given
/// `type_registry` if possible.
fn get_reflect_resource<'r>(
    type_registry: &'r TypeRegistry,
    resource_path: &str,
) -> AnyhowResult<&'r ReflectResource> {
    let resource_registration = get_resource_type_registration(type_registry, resource_path)?;

    resource_registration
        .data::<ReflectResource>()
        .ok_or_else(|| anyhow!("Resource `{}` isn't reflectable", resource_path))
}

/// Given a resource's type path, return the associated [`TypeRegistration`] from the given
/// `type_registry` if possible.
fn get_resource_type_registration<'r>(
    type_registry: &'r TypeRegistry,
    resource_path: &str,
) -> AnyhowResult<&'r TypeRegistration> {
    type_registry
        .get_with_type_path(resource_path)
        .ok_or_else(|| anyhow!("Unknown resource type: `{}`", resource_path))
}

#[cfg(test)]
mod tests {
    /// A generic function that tests serialization and deserialization of any type
    /// implementing Serialize and Deserialize traits.
    fn test_serialize_deserialize<T>(value: T)
    where
        T: Serialize + for<'a> Deserialize<'a> + PartialEq + core::fmt::Debug,
    {
        // Serialize the value to JSON string
        let serialized = serde_json::to_string(&value).expect("Failed to serialize");

        // Deserialize the JSON string back into the original type
        let deserialized: T = serde_json::from_str(&serialized).expect("Failed to deserialize");

        // Assert that the deserialized value is the same as the original
        assert_eq!(
            &value, &deserialized,
            "Deserialized value does not match original"
        );
    }

    use super::*;

    #[test]
    fn serialization_tests() {
        test_serialize_deserialize(BrpQueryRow {
            components: Default::default(),
            entity: Entity::from_raw_u32(0).unwrap(),
            has: Default::default(),
        });
        test_serialize_deserialize(BrpListComponentsWatchingResponse::default());
        test_serialize_deserialize(BrpQuery::default());
        test_serialize_deserialize(BrpJsonSchemaQueryFilter::default());
        test_serialize_deserialize(BrpJsonSchemaQueryFilter {
            type_limit: JsonSchemaTypeLimit {
                with: vec!["Resource".to_owned()],
                ..Default::default()
            },
            ..Default::default()
        });
        test_serialize_deserialize(BrpListComponentsParams {
            entity: Entity::from_raw_u32(0).unwrap(),
        });
    }
}

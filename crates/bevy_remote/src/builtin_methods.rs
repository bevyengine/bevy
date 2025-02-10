//! Built-in verbs for the Bevy Remote Protocol.

use core::any::TypeId;

use anyhow::{anyhow, Result as AnyhowResult};
use bevy_ecs::{
    component::ComponentId,
    entity::Entity,
    event::EventCursor,
    hierarchy::ChildOf,
    query::QueryBuilder,
    reflect::{AppTypeRegistry, ReflectComponent, ReflectResource},
    removal_detection::RemovedComponentEntity,
    system::{In, Local},
    world::{EntityRef, EntityWorldMut, FilteredEntityRef, World},
};
use bevy_platform_support::collections::HashMap;
use bevy_reflect::{
    prelude::ReflectDefault,
    serde::{ReflectSerializer, TypedReflectDeserializer},
    GetPath as _, NamedField, OpaqueInfo, PartialReflect, ReflectDeserialize, ReflectSerialize,
    TypeInfo, TypeRegistration, TypeRegistry, VariantInfo,
};
use serde::{de::DeserializeSeed as _, Deserialize, Serialize};
use serde_json::{json, Map, Value};

use crate::{error_codes, BrpError, BrpResult};

/// The method path for a `bevy/get` request.
pub const BRP_GET_METHOD: &str = "bevy/get";

/// The method path for a `bevy/query` request.
pub const BRP_QUERY_METHOD: &str = "bevy/query";

/// The method path for a `bevy/spawn` request.
pub const BRP_SPAWN_METHOD: &str = "bevy/spawn";

/// The method path for a `bevy/insert` request.
pub const BRP_INSERT_METHOD: &str = "bevy/insert";

/// The method path for a `bevy/remove` request.
pub const BRP_REMOVE_METHOD: &str = "bevy/remove";

/// The method path for a `bevy/destroy` request.
pub const BRP_DESTROY_METHOD: &str = "bevy/destroy";

/// The method path for a `bevy/reparent` request.
pub const BRP_REPARENT_METHOD: &str = "bevy/reparent";

/// The method path for a `bevy/list` request.
pub const BRP_LIST_METHOD: &str = "bevy/list";

/// The method path for a `bevy/mutate_component` request.
pub const BRP_MUTATE_COMPONENT_METHOD: &str = "bevy/mutate_component";

/// The method path for a `bevy/get+watch` request.
pub const BRP_GET_AND_WATCH_METHOD: &str = "bevy/get+watch";

/// The method path for a `bevy/list+watch` request.
pub const BRP_LIST_AND_WATCH_METHOD: &str = "bevy/list+watch";

/// The method path for a `bevy/registry/schema` request.
pub const BRP_REGISTRY_SCHEMA_METHOD: &str = "bevy/registry/schema";

/// `bevy/get`: Retrieves one or more components from the entity with the given
/// ID.
///
/// The server responds with a [`BrpGetResponse`].
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct BrpGetParams {
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

/// `bevy/query`: Performs a query over components in the ECS, returning entities
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

/// `bevy/spawn`: Creates a new entity with the given components and responds
/// with its ID.
///
/// The server responds with a [`BrpSpawnResponse`].
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct BrpSpawnParams {
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

/// `bevy/destroy`: Given an ID, despawns the entity with that ID.
///
/// The server responds with an okay.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct BrpDestroyParams {
    /// The ID of the entity to despawn.
    pub entity: Entity,
}

/// `bevy/remove`: Deletes one or more components from an entity.
///
/// The server responds with a null.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct BrpRemoveParams {
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

/// `bevy/insert`: Adds one or more components to an entity.
///
/// The server responds with a null.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct BrpInsertParams {
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

/// `bevy/reparent`: Assign a new parent to one or more entities.
///
/// The server responds with a null.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct BrpReparentParams {
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

/// `bevy/list`: Returns a list of all type names of registered components in the
/// system (no params provided), or those on an entity (params provided).
///
/// The server responds with a [`BrpListResponse`]
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct BrpListParams {
    /// The entity to query.
    pub entity: Entity,
}

/// `bevy/mutate_component`:
///
/// The server responds with a null.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct BrpMutateParams {
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
    pub option: Vec<String>,

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
/// This is sent in response to `bevy/spawn`.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct BrpSpawnResponse {
    /// The ID of the entity in question.
    pub entity: Entity,
}

/// The response to a `bevy/get` request.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(untagged)]
pub enum BrpGetResponse {
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

/// A single response from a `bevy/get+watch` request.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(untagged)]
pub enum BrpGetWatchingResponse {
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

/// The response to a `bevy/list` request.
pub type BrpListResponse = Vec<String>;

/// A single response from a `bevy/list+watch` request.
#[derive(Debug, Default, Serialize, Deserialize, Clone, PartialEq)]
pub struct BrpListWatchingResponse {
    added: Vec<String>,
    removed: Vec<String>,
}

/// The response to a `bevy/query` request.
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

/// Handles a `bevy/get` request coming from a client.
pub fn process_remote_get_request(In(params): In<Option<Value>>, world: &World) -> BrpResult {
    let BrpGetParams {
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

/// Handles a `bevy/get+watch` request coming from a client.
pub fn process_remote_get_watching_request(
    In(params): In<Option<Value>>,
    world: &World,
    mut removal_cursors: Local<HashMap<ComponentId, EventCursor<RemovedComponentEntity>>>,
) -> BrpResult<Option<Value>> {
    let BrpGetParams {
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
        let Some(component_id) = world.components().get_id(type_registration.type_id()) else {
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

        if let Some(ticks) = entity_ref.get_change_ticks_by_id(component_id) {
            if ticks.is_changed(world.last_change_tick(), world.read_change_tick()) {
                changed.push(component_path);
                continue;
            }
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
        BrpGetResponse::Lenient {
            components,
            errors: mut errs,
        } => BrpGetWatchingResponse::Lenient {
            components,
            removed,
            errors: {
                errs.extend(errors);
                errs
            },
        },
        BrpGetResponse::Strict(components) => BrpGetWatchingResponse::Strict {
            components,
            removed,
        },
    };

    Ok(Some(
        serde_json::to_value(response).map_err(BrpError::internal)?,
    ))
}

/// Reflect a list of components on an entity into a [`BrpGetResponse`].
fn reflect_components_to_response(
    components: Vec<String>,
    strict: bool,
    entity: Entity,
    entity_ref: EntityRef,
    type_registry: &TypeRegistry,
) -> BrpResult<BrpGetResponse> {
    let mut response = if strict {
        BrpGetResponse::Strict(Default::default())
    } else {
        BrpGetResponse::Lenient {
            components: Default::default(),
            errors: Default::default(),
        }
    };

    for component_path in components {
        match reflect_component(&component_path, entity, entity_ref, type_registry) {
            Ok(serialized_object) => match response {
                BrpGetResponse::Strict(ref mut components)
                | BrpGetResponse::Lenient {
                    ref mut components, ..
                } => {
                    components.extend(serialized_object.into_iter());
                }
            },
            Err(err) => match response {
                BrpGetResponse::Strict(_) => return Err(err),
                BrpGetResponse::Lenient { ref mut errors, .. } => {
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
        serde_json::to_value(&reflect_serializer).map_err(|err| BrpError {
            code: error_codes::COMPONENT_ERROR,
            message: err.to_string(),
            data: None,
        })?
    else {
        return Err(BrpError {
            code: error_codes::COMPONENT_ERROR,
            message: format!("Component `{}` could not be serialized", component_path),
            data: None,
        });
    };

    Ok(serialized_object)
}

/// Handles a `bevy/query` request coming from a client.
pub fn process_remote_query_request(In(params): In<Option<Value>>, world: &mut World) -> BrpResult {
    let BrpQueryParams {
        data: BrpQuery {
            components,
            option,
            has,
        },
        filter: BrpQueryFilter { without, with },
        strict,
    } = parse_some(params)?;

    let app_type_registry = world.resource::<AppTypeRegistry>().clone();
    let type_registry = app_type_registry.read();

    let components = get_component_ids(&type_registry, world, components, strict)
        .map_err(BrpError::component_error)?;
    let option = get_component_ids(&type_registry, world, option, strict)
        .map_err(BrpError::component_error)?;
    let has =
        get_component_ids(&type_registry, world, has, strict).map_err(BrpError::component_error)?;
    let without = get_component_ids(&type_registry, world, without, strict)
        .map_err(BrpError::component_error)?;
    let with = get_component_ids(&type_registry, world, with, strict)
        .map_err(BrpError::component_error)?;

    let mut query = QueryBuilder::<FilteredEntityRef>::new(world);
    for (_, component) in &components {
        query.ref_id(*component);
    }
    for (_, option) in &option {
        query.optional(|query| {
            query.ref_id(*option);
        });
    }
    for (_, has) in &has {
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

    // At this point, we can safely unify `components` and `option`, since we only retrieved
    // entities that actually have all the `components` already.
    //
    // We also will just collect the `ReflectComponent` values from the type registry all
    // at once so that we can reuse them between components.
    let paths_and_reflect_components: Vec<(&str, &ReflectComponent)> = components
        .into_iter()
        .chain(option)
        .map(|(type_id, _)| reflect_component_from_id(type_id, &type_registry))
        .collect::<AnyhowResult<Vec<(&str, &ReflectComponent)>>>()
        .map_err(BrpError::component_error)?;

    // ... and the analogous construction for `has`:
    let has_paths_and_reflect_components: Vec<(&str, &ReflectComponent)> = has
        .into_iter()
        .map(|(type_id, _)| reflect_component_from_id(type_id, &type_registry))
        .collect::<AnyhowResult<Vec<(&str, &ReflectComponent)>>>()
        .map_err(BrpError::component_error)?;

    let mut response = BrpQueryResponse::default();
    let mut query = query.build();
    for row in query.iter(world) {
        // The map of component values:
        let components_map = build_components_map(
            row.clone(),
            paths_and_reflect_components.iter().copied(),
            &type_registry,
        )
        .map_err(BrpError::component_error)?;

        // The map of boolean-valued component presences:
        let has_map = build_has_map(
            row.clone(),
            has_paths_and_reflect_components.iter().copied(),
        );
        response.push(BrpQueryRow {
            entity: row.id(),
            components: components_map,
            has: has_map,
        });
    }

    serde_json::to_value(response).map_err(BrpError::internal)
}

/// Handles a `bevy/spawn` request coming from a client.
pub fn process_remote_spawn_request(In(params): In<Option<Value>>, world: &mut World) -> BrpResult {
    let BrpSpawnParams { components } = parse_some(params)?;

    let app_type_registry = world.resource::<AppTypeRegistry>().clone();
    let type_registry = app_type_registry.read();

    let reflect_components =
        deserialize_components(&type_registry, components).map_err(BrpError::component_error)?;

    let entity = world.spawn_empty();
    let entity_id = entity.id();
    insert_reflected_components(&type_registry, entity, reflect_components)
        .map_err(BrpError::component_error)?;

    let response = BrpSpawnResponse { entity: entity_id };
    serde_json::to_value(response).map_err(BrpError::internal)
}

/// Handles a `bevy/insert` request (insert components) coming from a client.
pub fn process_remote_insert_request(
    In(params): In<Option<Value>>,
    world: &mut World,
) -> BrpResult {
    let BrpInsertParams { entity, components } = parse_some(params)?;

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

/// Handles a `bevy/mutate_component` request coming from a client.
///
/// This method allows you to mutate a single field inside an Entity's
/// component.
pub fn process_remote_mutate_component_request(
    In(params): In<Option<Value>>,
    world: &mut World,
) -> BrpResult {
    let BrpMutateParams {
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
            BrpError::component_error(anyhow!("Component `{}` isn't registered.", component))
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

/// Handles a `bevy/remove` request (remove components) coming from a client.
pub fn process_remote_remove_request(
    In(params): In<Option<Value>>,
    world: &mut World,
) -> BrpResult {
    let BrpRemoveParams { entity, components } = parse_some(params)?;

    let app_type_registry = world.resource::<AppTypeRegistry>().clone();
    let type_registry = app_type_registry.read();

    let component_ids = get_component_ids(&type_registry, world, components, true)
        .map_err(BrpError::component_error)?;

    // Remove the components.
    let mut entity_world_mut = get_entity_mut(world, entity)?;
    for (_, component_id) in component_ids {
        entity_world_mut.remove_by_id(component_id);
    }

    Ok(Value::Null)
}

/// Handles a `bevy/destroy` (despawn entity) request coming from a client.
pub fn process_remote_destroy_request(
    In(params): In<Option<Value>>,
    world: &mut World,
) -> BrpResult {
    let BrpDestroyParams { entity } = parse_some(params)?;

    get_entity_mut(world, entity)?.despawn();

    Ok(Value::Null)
}

/// Handles a `bevy/reparent` request coming from a client.
pub fn process_remote_reparent_request(
    In(params): In<Option<Value>>,
    world: &mut World,
) -> BrpResult {
    let BrpReparentParams {
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

/// Handles a `bevy/list` request (list all components) coming from a client.
pub fn process_remote_list_request(In(params): In<Option<Value>>, world: &World) -> BrpResult {
    let app_type_registry = world.resource::<AppTypeRegistry>();
    let type_registry = app_type_registry.read();

    let mut response = BrpListResponse::default();

    // If `Some`, return all components of the provided entity.
    if let Some(BrpListParams { entity }) = params.map(parse).transpose()? {
        let entity = get_entity(world, entity)?;
        for component_id in entity.archetype().components() {
            let Some(component_info) = world.components().get_info(component_id) else {
                continue;
            };
            response.push(component_info.name().to_owned());
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

/// Handles a `bevy/list` request (list all components) coming from a client.
pub fn process_remote_list_watching_request(
    In(params): In<Option<Value>>,
    world: &World,
    mut removal_cursors: Local<HashMap<ComponentId, EventCursor<RemovedComponentEntity>>>,
) -> BrpResult<Option<Value>> {
    let BrpListParams { entity } = parse_some(params)?;
    let entity_ref = get_entity(world, entity)?;
    let mut response = BrpListWatchingResponse::default();

    for component_id in entity_ref.archetype().components() {
        let ticks = entity_ref
            .get_change_ticks_by_id(component_id)
            .ok_or(BrpError::internal("Failed to get ticks"))?;

        if ticks.is_added(world.last_change_tick(), world.read_change_tick()) {
            let Some(component_info) = world.components().get_info(component_id) else {
                continue;
            };
            response.added.push(component_info.name().to_owned());
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
                response.removed.push(component_info.name().to_owned());
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

/// Handles a `bevy/registry/schema` request (list all registry types in form of schema) coming from a client.
pub fn export_registry_types(In(params): In<Option<Value>>, world: &World) -> BrpResult {
    let filter: BrpJsonSchemaQueryFilter = match params {
        None => Default::default(),
        Some(params) => parse(params)?,
    };

    let types = world.resource::<AppTypeRegistry>();
    let types = types.read();
    let schemas = types
        .iter()
        .map(export_type)
        .filter(|(_, schema)| {
            if let Some(crate_name) = &schema.crate_name {
                if !filter.with_crates.is_empty()
                    && !filter.with_crates.iter().any(|c| crate_name.eq(c))
                {
                    return false;
                }
                if !filter.without_crates.is_empty()
                    && filter.without_crates.iter().any(|c| crate_name.eq(c))
                {
                    return false;
                }
            }
            if !filter.type_limit.with.is_empty()
                && !filter
                    .type_limit
                    .with
                    .iter()
                    .any(|c| schema.reflect_types.iter().any(|cc| c.eq(cc)))
            {
                return false;
            }
            if !filter.type_limit.without.is_empty()
                && filter
                    .type_limit
                    .without
                    .iter()
                    .any(|c| schema.reflect_types.iter().any(|cc| c.eq(cc)))
            {
                return false;
            }

            true
        })
        .collect::<HashMap<String, JsonSchemaBevyType>>();

    serde_json::to_value(schemas).map_err(BrpError::internal)
}

/// Exports schema info for a given type
fn export_type(reg: &TypeRegistration) -> (String, JsonSchemaBevyType) {
    let t = reg.type_info();
    let binding = t.type_path_table();

    let short_path = binding.short_path();
    let type_path = binding.path();
    let mut typed_schema = JsonSchemaBevyType {
        reflect_types: get_registered_reflect_types(reg),
        short_path: short_path.to_owned(),
        type_path: type_path.to_owned(),
        crate_name: binding.crate_name().map(str::to_owned),
        module_path: binding.module_path().map(str::to_owned),
        ..Default::default()
    };
    match t {
        TypeInfo::Struct(info) => {
            typed_schema.properties = info
                .iter()
                .map(|field| (field.name().to_owned(), field.ty().ref_type()))
                .collect::<HashMap<_, _>>();
            typed_schema.required = info
                .iter()
                .filter(|field| !field.type_path().starts_with("core::option::Option"))
                .map(|f| f.name().to_owned())
                .collect::<Vec<_>>();
            typed_schema.additional_properties = Some(false);
            typed_schema.schema_type = SchemaType::Object;
            typed_schema.kind = SchemaKind::Struct;
        }
        TypeInfo::Enum(info) => {
            typed_schema.kind = SchemaKind::Enum;

            let simple = info
                .iter()
                .all(|variant| matches!(variant, VariantInfo::Unit(_)));
            if simple {
                typed_schema.schema_type = SchemaType::String;
                typed_schema.one_of = info
                    .iter()
                    .map(|variant| match variant {
                        VariantInfo::Unit(v) => v.name().into(),
                        _ => unreachable!(),
                    })
                    .collect::<Vec<_>>();
            } else {
                typed_schema.schema_type = SchemaType::Object;
                typed_schema.one_of = info
                .iter()
                .map(|variant| match variant {
                    VariantInfo::Struct(v) => json!({
                        "type": "object",
                        "kind": "Struct",
                        "typePath": format!("{}::{}", type_path, v.name()),
                        "shortPath": v.name(),
                        "properties": v
                            .iter()
                            .map(|field| (field.name().to_owned(), field.ref_type()))
                            .collect::<Map<_, _>>(),
                        "additionalProperties": false,
                        "required": v
                            .iter()
                            .filter(|field| !field.type_path().starts_with("core::option::Option"))
                            .map(NamedField::name)
                            .collect::<Vec<_>>(),
                    }),
                    VariantInfo::Tuple(v) => json!({
                        "type": "array",
                        "kind": "Tuple",
                        "typePath": format!("{}::{}", type_path, v.name()),
                        "shortPath": v.name(),
                        "prefixItems": v
                            .iter()
                            .map(SchemaJsonReference::ref_type)
                            .collect::<Vec<_>>(),
                        "items": false,
                    }),
                    VariantInfo::Unit(v) => json!({
                        "typePath": format!("{}::{}", type_path, v.name()),
                        "shortPath": v.name(),
                    }),
                })
                .collect::<Vec<_>>();
            }
        }
        TypeInfo::TupleStruct(info) => {
            typed_schema.schema_type = SchemaType::Array;
            typed_schema.kind = SchemaKind::TupleStruct;
            typed_schema.prefix_items = info
                .iter()
                .map(SchemaJsonReference::ref_type)
                .collect::<Vec<_>>();
            typed_schema.items = Some(false.into());
        }
        TypeInfo::List(info) => {
            typed_schema.schema_type = SchemaType::Array;
            typed_schema.kind = SchemaKind::List;
            typed_schema.items = info.item_ty().ref_type().into();
        }
        TypeInfo::Array(info) => {
            typed_schema.schema_type = SchemaType::Array;
            typed_schema.kind = SchemaKind::Array;
            typed_schema.items = info.item_ty().ref_type().into();
        }
        TypeInfo::Map(info) => {
            typed_schema.schema_type = SchemaType::Object;
            typed_schema.kind = SchemaKind::Map;
            typed_schema.key_type = info.key_ty().ref_type().into();
            typed_schema.value_type = info.value_ty().ref_type().into();
        }
        TypeInfo::Tuple(info) => {
            typed_schema.schema_type = SchemaType::Array;
            typed_schema.kind = SchemaKind::Tuple;
            typed_schema.prefix_items = info
                .iter()
                .map(SchemaJsonReference::ref_type)
                .collect::<Vec<_>>();
            typed_schema.items = Some(false.into());
        }
        TypeInfo::Set(info) => {
            typed_schema.schema_type = SchemaType::Set;
            typed_schema.kind = SchemaKind::Set;
            typed_schema.items = info.value_ty().ref_type().into();
        }
        TypeInfo::Opaque(info) => {
            typed_schema.schema_type = info.map_json_type();
            typed_schema.kind = SchemaKind::Value;
        }
    };

    (t.type_path().to_owned(), typed_schema)
}

fn get_registered_reflect_types(reg: &TypeRegistration) -> Vec<String> {
    // Vec could be moved to allow registering more types by game maker.
    let registered_reflect_types: [(TypeId, &str); 5] = [
        { (TypeId::of::<ReflectComponent>(), "Component") },
        { (TypeId::of::<ReflectResource>(), "Resource") },
        { (TypeId::of::<ReflectDefault>(), "Default") },
        { (TypeId::of::<ReflectSerialize>(), "Serialize") },
        { (TypeId::of::<ReflectDeserialize>(), "Deserialize") },
    ];
    let mut result = Vec::new();
    for (id, name) in registered_reflect_types {
        if reg.data_by_id(id).is_some() {
            result.push(name.to_owned());
        }
    }
    result
}

/// JSON Schema type for Bevy Registry Types
/// It tries to follow this standard: <https://json-schema.org/specification>
///
/// To take the full advantage from info provided by Bevy registry it provides extra fields
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct JsonSchemaBevyType {
    /// Bevy specific field, short path of the type.
    pub short_path: String,
    /// Bevy specific field, full path of the type.
    pub type_path: String,
    /// Bevy specific field, path of the module that type is part of.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub module_path: Option<String>,
    /// Bevy specific field, name of the crate that type is part of.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub crate_name: Option<String>,
    /// Bevy specific field, names of the types that type reflects.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub reflect_types: Vec<String>,
    /// Bevy specific field, [`TypeInfo`] type mapping.
    pub kind: SchemaKind,
    /// Bevy specific field, provided when [`SchemaKind`] `kind` field is equal to [`SchemaKind::Map`].
    ///
    /// It contains type info of key of the Map.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub key_type: Option<Value>,
    /// Bevy specific field, provided when [`SchemaKind`] `kind` field is equal to [`SchemaKind::Map`].
    ///
    /// It contains type info of value of the Map.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub value_type: Option<Value>,
    /// The type keyword is fundamental to JSON Schema. It specifies the data type for a schema.
    #[serde(rename = "type")]
    pub schema_type: SchemaType,
    /// The behavior of this keyword depends on the presence and annotation results of "properties"
    /// and "patternProperties" within the same schema object.
    /// Validation with "additionalProperties" applies only to the child
    /// values of instance names that do not appear in the annotation results of either "properties" or "patternProperties".
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub additional_properties: Option<bool>,
    /// Validation succeeds if, for each name that appears in both the instance and as a name
    /// within this keyword's value, the child instance for that name successfully validates
    /// against the corresponding schema.
    #[serde(skip_serializing_if = "HashMap::is_empty", default)]
    pub properties: HashMap<String, Value>,
    /// An object instance is valid against this keyword if every item in the array is the name of a property in the instance.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub required: Vec<String>,
    /// An instance validates successfully against this keyword if it validates successfully against exactly one schema defined by this keyword's value.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub one_of: Vec<Value>,
    /// Validation succeeds if each element of the instance validates against the schema at the same position, if any. This keyword does not constrain the length of the array. If the array is longer than this keyword's value, this keyword validates only the prefix of matching length.
    ///
    /// This keyword produces an annotation value which is the largest index to which this keyword
    /// applied a subschema. The value MAY be a boolean true if a subschema was applied to every
    /// index of the instance, such as is produced by the "items" keyword.
    /// This annotation affects the behavior of "items" and "unevaluatedItems".
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub prefix_items: Vec<Value>,
    /// This keyword applies its subschema to all instance elements at indexes greater
    /// than the length of the "prefixItems" array in the same schema object,
    /// as reported by the annotation result of that "prefixItems" keyword.
    /// If no such annotation result exists, "items" applies its subschema to all
    /// instance array elements.
    ///
    /// If the "items" subschema is applied to any positions within the instance array,
    /// it produces an annotation result of boolean true, indicating that all remaining
    /// array elements have been evaluated against this keyword's subschema.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub items: Option<Value>,
}

/// Kind of json schema, maps [`TypeInfo`] type
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Default)]
pub enum SchemaKind {
    /// Struct
    #[default]
    Struct,
    /// Enum type
    Enum,
    /// A key-value map
    Map,
    /// Array
    Array,
    /// List
    List,
    /// Fixed size collection of items
    Tuple,
    /// Fixed size collection of items with named fields
    TupleStruct,
    /// Set of unique values
    Set,
    /// Single value, eg. primitive types
    Value,
}

/// Type of json schema
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Default)]
#[serde(rename_all = "lowercase")]
pub enum SchemaType {
    /// Represents a string value.
    String,
    /// Represents a floating-point number.
    Float,

    /// Represents an unsigned integer.
    Uint,

    /// Represents a signed integer.
    Int,

    /// Represents an object with key-value pairs.
    Object,

    /// Represents an array of values.
    Array,

    /// Represents a boolean value (true or false).
    Boolean,

    /// Represents a set of unique values.
    Set,

    /// Represents a null value.
    #[default]
    Null,
}

/// Helper trait for generating json schema reference
trait SchemaJsonReference {
    /// Reference to another type in schema.
    /// The value `$ref` is a URI-reference that is resolved against the schema.
    fn ref_type(self) -> Value;
}

/// Helper trait for mapping bevy type path into json schema type
trait SchemaJsonType {
    /// Bevy Reflect type path
    fn get_type_path(&self) -> &'static str;

    /// JSON Schema type keyword from Bevy reflect type path into
    fn map_json_type(&self) -> SchemaType {
        match self.get_type_path() {
            "bool" => SchemaType::Boolean,
            "u8" | "u16" | "u32" | "u64" | "u128" | "usize" => SchemaType::Uint,
            "i8" | "i16" | "i32" | "i64" | "i128" | "isize" => SchemaType::Int,
            "f32" | "f64" => SchemaType::Float,
            "char" | "str" | "alloc::string::String" => SchemaType::String,
            _ => SchemaType::Object,
        }
    }
}

impl SchemaJsonType for OpaqueInfo {
    fn get_type_path(&self) -> &'static str {
        self.type_path()
    }
}

impl SchemaJsonReference for &bevy_reflect::Type {
    fn ref_type(self) -> Value {
        let path = self.path();
        json!({"type": json!({ "$ref": format!("#/$defs/{path}") })})
    }
}

impl SchemaJsonReference for &bevy_reflect::UnnamedField {
    fn ref_type(self) -> Value {
        let path = self.type_path();
        json!({"type": json!({ "$ref": format!("#/$defs/{path}") })})
    }
}

impl SchemaJsonReference for &NamedField {
    fn ref_type(self) -> Value {
        let type_path = self.type_path();
        json!({"type": json!({ "$ref": format!("#/$defs/{type_path}") }), "typePath": self.name()})
    }
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

/// Returns the [`TypeId`] and [`ComponentId`] of the components with the given
/// full path names.
///
/// Note that the supplied path names must be *full* path names: e.g.
/// `bevy_transform::components::transform::Transform` instead of `Transform`.
fn get_component_ids(
    type_registry: &TypeRegistry,
    world: &World,
    component_paths: Vec<String>,
    strict: bool,
) -> AnyhowResult<Vec<(TypeId, ComponentId)>> {
    let mut component_ids = vec![];

    for component_path in component_paths {
        let type_id = get_component_type_registration(type_registry, &component_path)?.type_id();
        let Some(component_id) = world.components().get_id(type_id) else {
            if strict {
                return Err(anyhow!(
                    "Component `{}` isn't used in the world",
                    component_path
                ));
            }
            continue;
        };

        component_ids.push((type_id, component_id));
    }

    Ok(component_ids)
}

/// Given an entity (`entity_ref`) and a list of reflected component information
/// (`paths_and_reflect_components`), return a map which associates each component to
/// its serialized value from the entity.
///
/// This is intended to be used on an entity which has already been filtered; components
/// where the value is not present on an entity are simply skipped.
fn build_components_map<'a>(
    entity_ref: FilteredEntityRef,
    paths_and_reflect_components: impl Iterator<Item = (&'a str, &'a ReflectComponent)>,
    type_registry: &TypeRegistry,
) -> AnyhowResult<HashMap<String, Value>> {
    let mut serialized_components_map = <HashMap<_, _>>::default();

    for (type_path, reflect_component) in paths_and_reflect_components {
        let Some(reflected) = reflect_component.reflect(entity_ref.clone()) else {
            continue;
        };

        let reflect_serializer =
            ReflectSerializer::new(reflected.as_partial_reflect(), type_registry);
        let Value::Object(serialized_object) = serde_json::to_value(&reflect_serializer)? else {
            return Err(anyhow!("Component `{}` could not be serialized", type_path));
        };

        serialized_components_map.extend(serialized_object.into_iter());
    }

    Ok(serialized_components_map)
}

/// Given an entity (`entity_ref`) and list of reflected component information
/// (`paths_and_reflect_components`), return a map which associates each component to
/// a boolean value indicating whether or not that component is present on the entity.
fn build_has_map<'a>(
    entity_ref: FilteredEntityRef,
    paths_and_reflect_components: impl Iterator<Item = (&'a str, &'a ReflectComponent)>,
) -> HashMap<String, Value> {
    let mut has_map = <HashMap<_, _>>::default();

    for (type_path, reflect_component) in paths_and_reflect_components {
        let has = reflect_component.contains(entity_ref.clone());
        has_map.insert(type_path.to_owned(), Value::Bool(has));
    }

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
    use bevy_ecs::{component::Component, resource::Resource};
    use bevy_reflect::Reflect;

    #[test]
    fn serialization_tests() {
        test_serialize_deserialize(BrpQueryRow {
            components: Default::default(),
            entity: Entity::from_raw(0),
            has: Default::default(),
        });
        test_serialize_deserialize(BrpListWatchingResponse::default());
        test_serialize_deserialize(BrpQuery::default());
        test_serialize_deserialize(BrpJsonSchemaQueryFilter::default());
        test_serialize_deserialize(BrpJsonSchemaQueryFilter {
            type_limit: JsonSchemaTypeLimit {
                with: vec!["Resource".to_owned()],
                ..Default::default()
            },
            ..Default::default()
        });
        test_serialize_deserialize(BrpListParams {
            entity: Entity::from_raw(0),
        });
    }

    #[test]
    fn reflect_export_struct() {
        #[derive(Reflect, Resource, Default, Deserialize, Serialize)]
        #[reflect(Resource, Default, Serialize, Deserialize)]
        struct Foo {
            a: f32,
            b: Option<f32>,
        }

        let atr = AppTypeRegistry::default();
        {
            let mut register = atr.write();
            register.register::<Foo>();
        }
        let type_registry = atr.read();
        let foo_registration = type_registry
            .get(TypeId::of::<Foo>())
            .expect("SHOULD BE REGISTERED")
            .clone();
        let (_, schema) = export_type(&foo_registration);
        println!("{}", &serde_json::to_string_pretty(&schema).unwrap());

        assert!(
            !schema.reflect_types.contains(&"Component".to_owned()),
            "Should not be a component"
        );
        assert!(
            schema.reflect_types.contains(&"Resource".to_owned()),
            "Should be a resource"
        );
        let _ = schema.properties.get("a").expect("Missing `a` field");
        let _ = schema.properties.get("b").expect("Missing `b` field");
        assert!(
            schema.required.contains(&"a".to_owned()),
            "Field a should be required"
        );
        assert!(
            !schema.required.contains(&"b".to_owned()),
            "Field b should not be required"
        );
    }

    #[test]
    fn reflect_export_enum() {
        #[derive(Reflect, Component, Default, Deserialize, Serialize)]
        #[reflect(Component, Default, Serialize, Deserialize)]
        enum EnumComponent {
            ValueOne(i32),
            ValueTwo {
                test: i32,
            },
            #[default]
            NoValue,
        }

        let atr = AppTypeRegistry::default();
        {
            let mut register = atr.write();
            register.register::<EnumComponent>();
        }
        let type_registry = atr.read();
        let foo_registration = type_registry
            .get(TypeId::of::<EnumComponent>())
            .expect("SHOULD BE REGISTERED")
            .clone();
        let (_, schema) = export_type(&foo_registration);
        assert!(
            schema.reflect_types.contains(&"Component".to_owned()),
            "Should be a component"
        );
        assert!(
            !schema.reflect_types.contains(&"Resource".to_owned()),
            "Should not be a resource"
        );
        assert!(schema.properties.is_empty(), "Should not have any field");
        assert!(schema.one_of.len() == 3, "Should have 3 possible schemas");
    }

    #[test]
    fn reflect_export_struct_without_reflect_types() {
        #[derive(Reflect, Component, Default, Deserialize, Serialize)]
        enum EnumComponent {
            ValueOne(i32),
            ValueTwo {
                test: i32,
            },
            #[default]
            NoValue,
        }

        let atr = AppTypeRegistry::default();
        {
            let mut register = atr.write();
            register.register::<EnumComponent>();
        }
        let type_registry = atr.read();
        let foo_registration = type_registry
            .get(TypeId::of::<EnumComponent>())
            .expect("SHOULD BE REGISTERED")
            .clone();
        let (_, schema) = export_type(&foo_registration);
        assert!(
            !schema.reflect_types.contains(&"Component".to_owned()),
            "Should not be a component"
        );
        assert!(
            !schema.reflect_types.contains(&"Resource".to_owned()),
            "Should not be a resource"
        );
        assert!(schema.properties.is_empty(), "Should not have any field");
        assert!(schema.one_of.len() == 3, "Should have 3 possible schemas");
    }

    #[test]
    fn reflect_export_tuple_struct() {
        #[derive(Reflect, Component, Default, Deserialize, Serialize)]
        #[reflect(Component, Default, Serialize, Deserialize)]
        struct TupleStructType(usize, i32);

        let atr = AppTypeRegistry::default();
        {
            let mut register = atr.write();
            register.register::<TupleStructType>();
        }
        let type_registry = atr.read();
        let foo_registration = type_registry
            .get(TypeId::of::<TupleStructType>())
            .expect("SHOULD BE REGISTERED")
            .clone();
        let (_, schema) = export_type(&foo_registration);
        println!("{}", &serde_json::to_string_pretty(&schema).unwrap());
        assert!(
            schema.reflect_types.contains(&"Component".to_owned()),
            "Should be a component"
        );
        assert!(
            !schema.reflect_types.contains(&"Resource".to_owned()),
            "Should not be a resource"
        );
        assert!(schema.properties.is_empty(), "Should not have any field");
        assert!(schema.prefix_items.len() == 2, "Should have 2 prefix items");
    }

    #[test]
    fn reflect_export_serialization_check() {
        #[derive(Reflect, Resource, Default, Deserialize, Serialize)]
        #[reflect(Resource, Default)]
        struct Foo {
            a: f32,
        }

        let atr = AppTypeRegistry::default();
        {
            let mut register = atr.write();
            register.register::<Foo>();
        }
        let type_registry = atr.read();
        let foo_registration = type_registry
            .get(TypeId::of::<Foo>())
            .expect("SHOULD BE REGISTERED")
            .clone();
        let (_, schema) = export_type(&foo_registration);
        let schema_as_value = serde_json::to_value(&schema).expect("Should serialize");
        let value = json!({
          "shortPath": "Foo",
          "typePath": "bevy_remote::builtin_methods::tests::Foo",
          "modulePath": "bevy_remote::builtin_methods::tests",
          "crateName": "bevy_remote",
          "reflectTypes": [
            "Resource",
            "Default",
          ],
          "kind": "Struct",
          "type": "object",
          "additionalProperties": false,
          "properties": {
            "a": {
              "type": {
                "$ref": "#/$defs/f32"
              }
            },
          },
          "required": [
            "a"
          ]
        });
        assert_eq!(schema_as_value, value);
    }
}

//! Built-in verbs for the Bevy Remote Protocol.

use core::any::TypeId;

use anyhow::{anyhow, Result as AnyhowResult};
use bevy_ecs::{
    component::ComponentId,
    entity::Entity,
    event::EventCursor,
    query::QueryBuilder,
    reflect::{AppTypeRegistry, ReflectComponent},
    removal_detection::RemovedComponentEntity,
    system::{In, Local},
    world::{EntityRef, EntityWorldMut, FilteredEntityRef, World},
};
use bevy_hierarchy::BuildChildren as _;
use bevy_reflect::{
    serde::{ReflectSerializer, TypedReflectDeserializer},
    PartialReflect, TypeRegistration, TypeRegistry,
};
use bevy_utils::HashMap;
use serde::{de::DeserializeSeed as _, Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::{error_codes, BrpError, BrpResult, RemoteWatchingSystemParams};

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

/// The method path for a `bevy/get+watch` request.
pub const BRP_GET_AND_WATCH_METHOD: &str = "bevy/get+watch";

/// The method path for a `bevy/list+watch` request.
pub const BRP_LIST_AND_WATCH_METHOD: &str = "bevy/list+watch";

/// `bevy/get`: Retrieves one or more components from the entity with the given
/// ID.
///
/// The server responds with a [`BrpGetResponse`].
#[derive(Debug, Serialize, Deserialize, Clone)]
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
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BrpQueryParams {
    /// The components to select.
    pub data: BrpQuery,

    /// An optional filter that specifies which entities to include or
    /// exclude from the results.
    #[serde(default)]
    pub filter: BrpQueryFilter,
}

/// `bevy/spawn`: Creates a new entity with the given components and responds
/// with its ID.
///
/// The server responds with a [`BrpSpawnResponse`].
#[derive(Debug, Serialize, Deserialize, Clone)]
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
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BrpDestroyParams {
    /// The ID of the entity to despawn.
    pub entity: Entity,
}

/// `bevy/remove`: Deletes one or more components from an entity.
///
/// The server responds with a null.
#[derive(Debug, Serialize, Deserialize, Clone)]
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
#[derive(Debug, Serialize, Deserialize, Clone)]
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
#[derive(Debug, Serialize, Deserialize, Clone)]
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
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BrpListParams {
    /// The entity to query.
    pub entity: Entity,
}

/// Describes the data that is to be fetched in a query.
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
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
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
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

/// A response from the world to the client that specifies a single entity.
///
/// This is sent in response to `bevy/spawn`.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BrpSpawnResponse {
    /// The ID of the entity in question.
    pub entity: Entity,
}

/// The response to a `bevy/get` request.
#[derive(Debug, Serialize, Deserialize, Clone)]
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
#[derive(Debug, Serialize, Deserialize, Clone)]
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
#[derive(Debug, Default, Serialize, Deserialize, Clone)]
pub struct BrpListWatchingResponse {
    added: Vec<String>,
    removed: Vec<String>,
}

/// The response to a `bevy/query` request.
pub type BrpQueryResponse = Vec<BrpQueryRow>;

/// One query match result: a single entity paired with the requested components.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BrpQueryRow {
    /// The ID of the entity that matched.
    pub entity: Entity,

    /// The serialized values of the requested components.
    pub components: HashMap<String, Value>,

    /// The boolean-only containment query results.
    #[serde(skip_serializing_if = "HashMap::is_empty")]
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
    In((_, params)): In<RemoteWatchingSystemParams>,
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
    let mut errors = HashMap::new();

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
    } = parse_some(params)?;

    let app_type_registry = world.resource::<AppTypeRegistry>().clone();
    let type_registry = app_type_registry.read();

    let components =
        get_component_ids(&type_registry, world, components).map_err(BrpError::component_error)?;
    let option =
        get_component_ids(&type_registry, world, option).map_err(BrpError::component_error)?;
    let has = get_component_ids(&type_registry, world, has).map_err(BrpError::component_error)?;
    let without =
        get_component_ids(&type_registry, world, without).map_err(BrpError::component_error)?;
    let with = get_component_ids(&type_registry, world, with).map_err(BrpError::component_error)?;

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

/// Handles a `bevy/remove` request (remove components) coming from a client.
pub fn process_remote_remove_request(
    In(params): In<Option<Value>>,
    world: &mut World,
) -> BrpResult {
    let BrpRemoveParams { entity, components } = parse_some(params)?;

    let app_type_registry = world.resource::<AppTypeRegistry>().clone();
    let type_registry = app_type_registry.read();

    let component_ids =
        get_component_ids(&type_registry, world, components).map_err(BrpError::component_error)?;

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
            get_entity_mut(world, entity)?.remove_parent();
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
    In((_, params)): In<RemoteWatchingSystemParams>,
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
) -> AnyhowResult<Vec<(TypeId, ComponentId)>> {
    let mut component_ids = vec![];

    for component_path in component_paths {
        let type_id = get_component_type_registration(type_registry, &component_path)?.type_id();
        let Some(component_id) = world.components().get_id(type_id) else {
            return Err(anyhow!(
                "Component `{}` isn't used in the world",
                component_path
            ));
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
    let mut serialized_components_map = HashMap::new();

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
    let mut has_map = HashMap::new();

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
                .unwrap();
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

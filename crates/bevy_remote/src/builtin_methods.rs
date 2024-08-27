//! Built-in verbs for the Bevy Remote Protocol.

use std::any::TypeId;

use anyhow::{anyhow, Result as AnyhowResult};
use bevy_ecs::{
    component::ComponentId,
    entity::Entity,
    query::QueryBuilder,
    reflect::{AppTypeRegistry, ReflectComponent},
    system::In,
    world::{EntityRef, EntityWorldMut, FilteredEntityRef, World},
};
use bevy_hierarchy::BuildChildren as _;
use bevy_reflect::{
    serde::{ReflectSerializer, TypedReflectDeserializer},
    Reflect, TypeRegistration, TypeRegistry,
};
use bevy_utils::HashMap;
use serde::de::DeserializeSeed as _;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{error_codes, BrpError, BrpResult};

/// `bevy/get`: Retrieves one or more components from the entity with the given
/// ID.
///
/// The server responds with a [`BrpGetResponse`].
#[derive(Serialize, Deserialize, Clone)]
pub struct BrpGetParams {
    /// The ID of the entity from which components are to be requested.
    pub entity: Entity,

    /// The *full paths* of the component types that are to be requested
    /// from the entity.
    ///
    /// Note that these strings must consist of the *full* type paths: e.g.
    /// `bevy_transform::components::transform::Transform`, not just
    /// `Transform`.
    pub components: Vec<String>,
}

/// `bevy/query`: Performs a query over components in the ECS, returning entities
/// and component values that match.
///
/// The server responds with a [`BrpQueryResponse`].
#[derive(Serialize, Deserialize, Clone)]
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
/// The server responds with a [`BrpEntityResponse`].
#[derive(Serialize, Deserialize, Clone)]
pub struct BrpSpawnParams {
    /// A map from each component's *full path* to its serialized value.
    ///
    /// These components will be added to the entity.
    ///
    /// Note that the keys of the map must be the *full* type paths: e.g.
    /// `bevy_transform::components::transform::Transform`, not just
    /// `Transform`.
    pub components: HashMap<String, Value>,
}

/// `bevy/destroy`: Given an ID, despawns the entity with that ID.
///
/// The server responds with an okay.
#[derive(Serialize, Deserialize, Clone)]
pub struct BrpDestroyParams {
    /// The ID of the entity to despawn.
    pub entity: Entity,
}

/// `bevy/remove`: Deletes one or more components from an entity.
///
/// The server responds with a null.
#[derive(Serialize, Deserialize, Clone)]
pub struct BrpRemoveParams {
    /// The ID of the entity from which components are to be removed.
    pub entity: Entity,

    /// The *full paths* of the component types that are to be removed from
    /// the entity.
    ///
    /// Note that these strings must consist of the *full* type paths: e.g.
    /// `bevy_transform::components::transform::Transform`, not just
    /// `Transform`.
    pub components: Vec<String>,
}

/// `bevy/insert`: Adds one or more components to an entity.
///
/// The server responds with a null.
#[derive(Serialize, Deserialize, Clone)]
pub struct BrpInsertParams {
    /// The ID of the entity that components are to be added to.
    pub entity: Entity,

    /// A map from each component's *full path* to its serialized value.
    ///
    /// These components will be added to the entity.
    ///
    /// Note that the keys of the map must be the *full* type paths: e.g.
    /// `bevy_transform::components::transform::Transform`, not just
    /// `Transform`.
    pub components: HashMap<String, Value>,
}

/// `bevy/reparent`: Changes the parent of an entity.
///
/// The server responds with a null.
#[derive(Serialize, Deserialize, Clone)]
pub struct BrpReparentParams {
    /// The IDs of the entities that are to become the new children of the
    /// `parent`.
    pub entities: Vec<Entity>,

    /// The IDs of the entity that will become the new parent of the
    /// `entities`.
    ///
    /// If this is `None`, then the entities are removed from all parents.
    pub parent: Option<Entity>,
}

/// `bevy/list`: Returns a list of all type names of registered components in the
/// system (no params provided), or those on an entity (params provided).
///
/// The server responds with a [`BrpListResponse`]
#[derive(Serialize, Deserialize, Clone)]
pub struct BrpListParams {
    /// The entity to query.
    pub entity: Entity,
}

/// Describes the data that is to be fetched in a query.
#[derive(Serialize, Deserialize, Clone, Default)]
pub struct BrpQuery {
    /// The *full path* of the type name of each component that is to be
    /// fetched.
    #[serde(default)]
    pub components: Vec<String>,

    /// The *full path* of the type name of each component that is to be
    /// optionally fetched.
    #[serde(default)]
    pub option: Vec<String>,

    /// The *full path* of the type name of each component that is to be checked
    /// for presence.
    #[serde(default)]
    pub has: Vec<String>,
}

/// Additional constraints that can be placed on a query to include or exclude
/// certain entities.
#[derive(Serialize, Deserialize, Clone, Default)]
pub struct BrpQueryFilter {
    /// The *full path* of the type name of each component that may not be
    /// present on the entity for it to be included in the results.
    #[serde(default)]
    pub without: Vec<String>,

    /// The *full path* of the type name of each component that must be present
    /// on the entity for it to be included in the results.
    #[serde(default)]
    pub with: Vec<String>,
}

/// A response from the world to the client that specifies a single entity.
///
/// This is sent in response to `bevy/spawn`.
#[derive(Serialize, Deserialize, Clone)]
pub struct BrpSpawnResponse {
    /// The ID of the entity in question.
    pub entity: Entity,
}

/// The response to a `bevy/get` request.
pub type BrpGetResponse = HashMap<String, Value>;

/// The response to a `bevy/list` request.
pub type BrpListResponse = Vec<String>;

/// The response to a `bevy/query` request.
pub type BrpQueryResponse = Vec<BrpQueryRow>;

/// One query match result: a single entity paired with the requested components.
#[derive(Serialize, Deserialize, Clone)]
pub struct BrpQueryRow {
    /// The ID of the entity that matched.
    pub entity: Entity,

    /// The serialized values of the requested components.
    pub components: HashMap<String, Value>,
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
    let BrpGetParams { entity, components } = parse_some(params)?;

    let app_type_registry = world.resource::<AppTypeRegistry>();
    let type_registry = app_type_registry.read();
    let entity_ref = get_entity(world, entity)?;

    let mut serialized_components_map = HashMap::new();

    for component_path in components {
        let reflect_component = get_reflect_component(&type_registry, &component_path)
            .map_err(BrpError::component_error)?;

        // Retrieve the reflected value for the given specified component on the given entity.
        let Some(reflected) = reflect_component.reflect(entity_ref) else {
            return Err(BrpError::component_not_present(&component_path, entity));
        };

        // Each component value serializes to a map with a single entry.
        let reflect_serializer = ReflectSerializer::new(reflected, &type_registry);
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

        serialized_components_map.extend(serialized_object.into_iter());
    }

    serde_json::to_value(serialized_components_map).map_err(BrpError::internal)
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
    for (_, option) in option {
        query.optional(|query| {
            query.ref_id(option);
        });
    }
    for (_, has) in has {
        query.optional(|query| {
            query.ref_id(has);
        });
    }
    for (_, without) in without {
        query.without_id(without);
    }
    for (_, with) in with {
        query.with_id(with);
    }

    let mut rows = vec![];
    let mut query = query.build();
    for row in query.iter(world) {
        let components_map = serialize_components(
            row.clone(),
            components.iter().map(|(type_id, _)| *type_id),
            &type_registry,
        )
        .map_err(BrpError::component_error)?;
        rows.push(BrpQueryRow {
            entity: row.id(),
            components: components_map,
        });
    }

    serde_json::to_value(rows).map_err(BrpError::internal)
}

/// Handles a `bevy/spawn` request coming from a client.
pub fn process_remote_spawn_request(In(params): In<Option<Value>>, world: &mut World) -> BrpResult {
    let BrpSpawnParams { components } = parse_some(params)?;

    let app_type_registry = world.resource::<AppTypeRegistry>().clone();
    let type_registry = app_type_registry.read();

    let reflect_components =
        deserialize_components(&type_registry, components).map_err(BrpError::component_error)?;
    insert_reflected_components(&type_registry, world.spawn_empty(), reflect_components)
        .map_err(BrpError::component_error)?;

    Ok(Value::Null)
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

    let mut result = vec![];

    // If `Some`, return all components of the provided entity.
    if let Some(BrpListParams { entity }) = params.map(parse).transpose()? {
        let entity = get_entity(world, entity)?;
        for component_id in entity.archetype().components() {
            let Some(component_info) = world.components().get_info(component_id) else {
                continue;
            };
            result.push(component_info.name().to_owned());
        }
    }
    // If `None`, list all registered components.
    else {
        for registered_type in type_registry.iter() {
            if registered_type.data::<ReflectComponent>().is_some() {
                result.push(registered_type.type_info().type_path().to_owned());
            }
        }
    }

    // Sort both for cleanliness and to reduce the risk that clients start
    // accidentally depending on the order.
    result.sort();

    serde_json::to_value(result).map_err(BrpError::internal)
}

/// Immutably retrieves an entity from the [`World`], returning an error if the
/// entity isn't present.
fn get_entity(world: &World, entity: Entity) -> Result<EntityRef<'_>, BrpError> {
    world
        .get_entity(entity)
        .ok_or_else(|| BrpError::entity_not_found(entity))
}

/// Mutably retrieves an entity from the [`World`], returning an error if the
/// entity isn't present.
fn get_entity_mut(world: &mut World, entity: Entity) -> Result<EntityWorldMut<'_>, BrpError> {
    world
        .get_entity_mut(entity)
        .ok_or_else(|| BrpError::entity_not_found(entity))
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

/// Given an entity (`entity_ref`) and a list of component type IDs (`component_type_ids`),
/// return a map which associates each component to its serialized value from the entity.
fn serialize_components(
    entity_ref: FilteredEntityRef,
    component_type_ids: impl Iterator<Item = TypeId>,
    type_registry: &TypeRegistry,
) -> AnyhowResult<HashMap<String, Value>> {
    let mut serialized_components_map = HashMap::new();

    for component_type_id in component_type_ids {
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

        let Some(reflected) = reflect_component.reflect(entity_ref.clone()) else {
            return Err(anyhow!(
                "Entity {:?} has no component `{}`",
                entity_ref.id(),
                type_path
            ));
        };

        let reflect_serializer = ReflectSerializer::new(reflected, type_registry);
        let Value::Object(serialized_object) = serde_json::to_value(&reflect_serializer)? else {
            return Err(anyhow!("Component `{}` could not be serialized", type_path));
        };

        serialized_components_map.extend(serialized_object.into_iter());
    }

    Ok(serialized_components_map)
}

/// Given a collection of component paths and their associated serialized values (`components`),
/// return the associated collection of deserialized reflected values.
fn deserialize_components(
    type_registry: &TypeRegistry,
    components: HashMap<String, Value>,
) -> AnyhowResult<Vec<Box<dyn Reflect>>> {
    let mut reflect_components = vec![];

    for (component_path, component) in components {
        let Some(component_type) = type_registry.get_with_type_path(&component_path) else {
            return Err(anyhow!("Unknown component type: `{}`", component_path));
        };
        let reflected: Box<dyn Reflect> =
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
    reflect_components: Vec<Box<dyn Reflect>>,
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

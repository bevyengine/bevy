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
use bevy_hierarchy::BuildWorldChildren as _;
use bevy_reflect::{
    serde::{ReflectSerializer, TypedReflectDeserializer},
    Reflect, TypeRegistration, TypeRegistry,
};
use bevy_utils::{prelude::default, HashMap};
use serde::de::DeserializeSeed as _;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// `GET`: Retrieves one or more components from the entity with the given
/// ID.
///
/// The server responds with a `BrpResponse::Get`.
#[derive(Serialize, Deserialize, Clone)]
pub struct BrpGetRequest {
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

/// `QUERY`: Performs a query over components in the ECS, returning entities
/// and component values that match.
///
/// The server responds with a `BrpResponse::Query`.
#[derive(Serialize, Deserialize, Clone)]
pub struct BrpQueryRequest {
    /// The components to select.
    pub data: BrpQuery,

    /// An optional filter that specifies which entities to include or
    /// exclude from the results.
    #[serde(default)]
    pub filter: BrpQueryFilter,
}

/// `SPAWN`: Creates a new entity with the given components and responds
/// with its ID.
///
/// The server responds with a `BrpResponse::Entity`.
#[derive(Serialize, Deserialize, Clone)]
pub struct BrpSpawnRequest {
    /// A map from each component's *full path* to its serialized value.
    ///
    /// These components will be added to the entity.
    ///
    /// Note that the keys of the map must be the *full* type paths: e.g.
    /// `bevy_transform::components::transform::Transform`, not just
    /// `Transform`.
    pub components: HashMap<String, Value>,
}

/// `DESTROY`: Given an ID, despawns the entity with that ID.
///
/// The server responds with a `BrpResponse::Ok`.
#[derive(Serialize, Deserialize, Clone)]
pub struct BrpDestroyRequest {
    /// The ID of the entity to despawn.
    pub entity: Entity,
}

/// `REMOVE`: Deletes one or more components from an entity.
///
/// The server responds with a `BrpResponse::Ok`.
#[derive(Serialize, Deserialize, Clone)]
pub struct BrpRemoveRequest {
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

/// `INSERT`: Adds one or more components to an entity.
///
/// The server responds with a `BrpResponse::Ok`.
#[derive(Serialize, Deserialize, Clone)]
pub struct BrpInsertRequest {
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

/// `REPARENT`: Changes the parent of an entity.
///
/// The server responds with a `BrpResponse::Ok`.
#[derive(Serialize, Deserialize, Clone)]
pub struct BrpReparentRequest {
    /// The IDs of the entities that are to become the new children of the
    /// `parent`.
    pub entities: Vec<Entity>,

    /// The IDs of the entity that will become the new parent of the
    /// `entities`.
    ///
    /// If this is `None`, then the entities are removed from all parents.
    pub parent: Option<Entity>,
}

/// `LIST`: Returns a list of all type names of registered components in the
/// system, or those on an entity.
#[derive(Serialize, Deserialize, Clone)]
pub struct BrpListRequest {
    /// The entity to query.
    ///
    /// If not specified, this request returns the names of all registered
    /// components.
    pub entity: Option<Entity>,
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
/// This is sent in response to `SPAWN`.
#[derive(Serialize, Deserialize, Clone)]
pub struct BrpEntityResponse {
    /// The ID of the entity in question.
    pub entity: Entity,
}

/// The response to a `GET` request.
#[derive(Serialize, Deserialize, Clone)]
struct BrpGetResponse {
    /// The ID of the entity for which components were requested.
    entity: Entity,

    /// The values of the requested components.
    components: HashMap<String, Value>,
}

/// The response to a `LIST` request.
#[derive(Serialize, Deserialize, Clone)]
struct BrpListResponse {
    /// The ID of the entity for which component names were requested, if
    /// present.
    ///
    /// If this is `None`, then `components` contains the name of all
    /// reflectable components known to the system.
    entity: Option<Entity>,

    /// The full type names of the registered components.
    components: Vec<String>,
}

/// The response to a `QUERY` request.
#[derive(Serialize, Deserialize, Clone)]
struct BrpQueryResponse {
    /// All results of the query: the entities and the requested components.
    rows: Vec<BrpQueryRow>,
}

/// One query match result: a single entity paired with the requested components.
#[derive(Serialize, Deserialize, Clone)]
pub struct BrpQueryRow {
    /// The ID of the entity that matched.
    pub entity: Entity,

    /// The serialized values of the requested components.
    #[serde(flatten)]
    pub components: HashMap<String, Value>,
}

/// Handles a `GET` request coming from a client.
pub fn process_remote_get_request(
    In(request): In<Value>,
    world: &mut World,
) -> AnyhowResult<Value> {
    let BrpGetRequest { entity, components } = serde_json::from_value(request)?;

    let app_type_registry = world.resource::<AppTypeRegistry>();
    let type_registry = app_type_registry.read();
    let Some(entity_ref) = world.get_entity(entity) else {
        return Err(anyhow!("Entity {:?} didn't exist", entity));
    };

    let mut serialized_components_map = HashMap::new();

    for component_path in components {
        let reflect_component = get_reflect_component(&type_registry, &component_path)?;

        let Some(reflected) = reflect_component.reflect(entity_ref) else {
            return Err(anyhow!(
                "Entity {:?} has no component `{}`",
                entity,
                component_path
            ));
        };

        let reflect_serializer = ReflectSerializer::new(reflected, &type_registry);
        let Value::Object(serialized_object) = serde_json::to_value(&reflect_serializer)? else {
            return Err(anyhow!(
                "Component didn't serialize into a JSON object: `{}`",
                component_path
            ));
        };

        serialized_components_map.extend(serialized_object.into_iter());
    }

    Ok(serde_json::to_value(BrpGetResponse {
        entity,
        components: serialized_components_map,
    })?)
}

/// Handles a `QUERY` request coming from a client.
pub fn process_remote_query_request(
    In(request): In<Value>,
    world: &mut World,
) -> AnyhowResult<Value> {
    let BrpQueryRequest {
        data: BrpQuery {
            components,
            option,
            has,
        },
        filter: BrpQueryFilter { without, with },
    } = serde_json::from_value(request)?;

    let app_type_registry = world.resource::<AppTypeRegistry>().clone();
    let type_registry = app_type_registry.read();

    let components = get_component_ids(&type_registry, world, components)?;
    let option = get_component_ids(&type_registry, world, option)?;
    let has = get_component_ids(&type_registry, world, has)?;
    let without = get_component_ids(&type_registry, world, without)?;
    let with = get_component_ids(&type_registry, world, with)?;

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
        )?;
        rows.push(BrpQueryRow {
            entity: row.id(),
            components: components_map,
        });
    }

    Ok(serde_json::to_value(BrpQueryResponse { rows })?)
}

/// Handles a `SPAWN` request coming from a client.
pub fn process_remote_spawn_request(
    In(request): In<Value>,
    world: &mut World,
) -> AnyhowResult<Value> {
    let BrpSpawnRequest { components } = serde_json::from_value(request)?;

    let app_type_registry = world.resource::<AppTypeRegistry>().clone();
    let type_registry = app_type_registry.read();

    let reflect_components = deserialize_components(&type_registry, components)?;
    insert_reflected_components(&type_registry, world.spawn_empty(), reflect_components)?;

    Ok(Value::Object(default()))
}

/// Handles an `INSERT` request (insert components) coming from a client.
pub fn process_remote_insert_request(
    In(request): In<Value>,
    world: &mut World,
) -> AnyhowResult<Value> {
    let BrpInsertRequest { entity, components } = serde_json::from_value(request)?;

    let app_type_registry = world.resource::<AppTypeRegistry>().clone();
    let type_registry = app_type_registry.read();

    let reflect_components = deserialize_components(&type_registry, components)?;

    insert_reflected_components(
        &type_registry,
        get_entity_mut(world, entity)?,
        reflect_components,
    )?;

    Ok(Value::Object(default()))
}

/// Handles a `REMOVE` request (remove components) coming from a client.
pub fn process_remote_remove_request(
    In(request): In<Value>,
    world: &mut World,
) -> AnyhowResult<Value> {
    let BrpRemoveRequest { entity, components } = serde_json::from_value(request)?;

    let app_type_registry = world.resource::<AppTypeRegistry>().clone();
    let type_registry = app_type_registry.read();

    let component_ids = get_component_ids(&type_registry, world, components)?;

    // Remove the components.
    let mut entity_world_mut = get_entity_mut(world, entity)?;
    for (_, component_id) in component_ids {
        entity_world_mut.remove_by_id(component_id);
    }

    Ok(Value::Object(default()))
}

/// Handles a `DESTROY` (despawn entity) request coming from a client.
pub fn process_remote_destroy_request(
    In(request): In<Value>,
    world: &mut World,
) -> AnyhowResult<Value> {
    let BrpDestroyRequest { entity } = serde_json::from_value(request)?;

    get_entity_mut(world, entity)?.despawn();

    Ok(Value::Object(default()))
}

/// Handles a `REPARENT` request coming from a client.
pub fn process_remote_reparent_request(
    In(request): In<Value>,
    world: &mut World,
) -> AnyhowResult<Value> {
    let BrpReparentRequest {
        entities,
        parent: maybe_parent,
    } = serde_json::from_value(request)?;

    match maybe_parent {
        // If `Some`, reparent the entities.
        Some(parent) => {
            let mut parent_commands = get_entity_mut(world, parent)?;
            for entity in entities {
                if entity == parent {
                    return Err(anyhow!("Can't parent an object to itself"));
                }
                parent_commands.add_child(entity);
            }
        }

        // If `None`, remove the entities from their parents.
        None => {
            for entity in entities {
                get_entity_mut(world, entity)?.remove_parent();
            }
        }
    }

    Ok(Value::Object(default()))
}

/// Handles a `LIST` request (list all components) coming from a client.
pub fn process_remote_list_request(
    In(request): In<Value>,
    world: &mut World,
) -> AnyhowResult<Value> {
    let BrpListRequest { entity } = serde_json::from_value(request)?;

    let app_type_registry = world.resource::<AppTypeRegistry>();
    let type_registry = app_type_registry.read();

    let mut result = vec![];

    match entity {
        Some(entity) => {
            let entity = get_entity(world, entity)?;
            for component_id in entity.archetype().components() {
                let Some(component_info) = world.components().get_info(component_id) else {
                    continue;
                };
                result.push(component_info.name().to_owned());
            }
        }

        None => {
            for registered_type in type_registry.iter() {
                if registered_type.data::<ReflectComponent>().is_some() {
                    result.push(registered_type.type_info().type_path().to_owned());
                }
            }
        }
    }

    // Sort both for cleanliness and to reduce the risk that clients start
    // accidentally start depending on the order.
    result.sort();

    Ok(serde_json::to_value(BrpListResponse {
        entity,
        components: result,
    })?)
}

/// Immutably retrieves an entity from the [`World`], returning an error if the
/// entity isn't present.
fn get_entity(world: &World, entity: Entity) -> AnyhowResult<EntityRef<'_>> {
    world
        .get_entity(entity)
        .ok_or_else(|| anyhow!("Entity {:?} not found", entity))
}

/// Mutably retrieves an entity from the [`World`], returning an error if the
/// entity isn't present.
fn get_entity_mut(world: &mut World, entity: Entity) -> AnyhowResult<EntityWorldMut<'_>> {
    world
        .get_entity_mut(entity)
        .ok_or_else(|| anyhow!("Entity {:?} not found", entity))
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
            return Err(anyhow!(
                "Component didn't serialize into a JSON object: `{}`",
                type_path
            ));
        };

        serialized_components_map.extend(serialized_object.into_iter());
    }

    Ok(serialized_components_map)
}

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

fn get_reflect_component<'a>(
    type_registry: &'a TypeRegistry,
    component_path: &str,
) -> AnyhowResult<&'a ReflectComponent> {
    let component_registration = get_component_type_registration(type_registry, component_path)?;
    let Some(reflect_component) = component_registration.data::<ReflectComponent>() else {
        return Err(anyhow!(
            "Type isn't a reflectable component: `{}`",
            component_path
        ));
    };

    Ok(reflect_component)
}

fn get_component_type_registration<'r>(
    type_registry: &'r TypeRegistry,
    component_path: &str,
) -> AnyhowResult<&'r TypeRegistration> {
    match type_registry.get_with_type_path(component_path) {
        Some(component_registration) => Ok(component_registration),
        None => Err(anyhow!("Unknown component type: `{}`", component_path)),
    }
}

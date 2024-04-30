//! The Bevy Remote Protocol, an HTTP- and JSON-based protocol that allows for
//! remote control of a Bevy app.
//!
//! Adding the [`RemotePlugin`] to your [`App`] causes Bevy to accept
//! connections over HTTP (by default, on port 15702) while your app is running.
//! These *remote clients* can inspect and alter the state of the
//! entity-component system. Clients are expected to `POST` JSON requests to the
//! root URL; see the `client` example for a trivial example of use.
//!
//! A typical client request might look like this:
//!
//! ```json
//! {
//!     "request": "GET",
//!     "id": 0,
//!     "params": {
//!         "data": {
//!             "entity": 4294967298,
//!             "components": [
//!                 "bevy_transform::components::transform::Transform"
//!             ]
//!         }
//!     }
//! }
//! ```
//!
//! And a response might look like this:
//!
//! ```json
//! {
//!     "components": {
//!         "bevy_transform::components::transform::Transform": {
//!             "rotation": { "x": 0.0, "y": 0.0, "z": 0.0, "w": 1.0 },
//!             "scale": { "x": 1.0, "y": 1.0, "z": 1.0 },
//!             "translation": { "x": 0.0, "y": 0.5, "z": 0.0 }
//!         }
//!     },
//!     "entity": 4294967298,
//!     "id": 0,
//!     "status": "OK"
//! }
//! ```
use std::{
    any::TypeId,
    net::SocketAddr,
    sync::{Arc, Mutex},
    thread,
};

use anyhow::{anyhow, Result as AnyhowResult};
use bevy_app::prelude::*;
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{
    component::ComponentId,
    entity::Entity,
    query::QueryBuilder,
    reflect::{AppTypeRegistry, ReflectComponent},
    system::{Commands, Res, Resource},
    world::{EntityRef, EntityWorldMut, FilteredEntityRef, World},
};
use bevy_hierarchy::BuildWorldChildren as _;
use bevy_reflect::{
    serde::{ReflectSerializer, TypedReflectDeserializer},
    Reflect, TypeRegistration, TypeRegistry,
};
use bevy_utils::{tracing::error, HashMap};
use http_body_util::{BodyExt as _, Full};
use hyper::{
    body::{Bytes, Incoming},
    service, Request, Response,
};
use hyper_util::{
    rt::{TokioExecutor, TokioIo},
    server::conn::auto::Builder,
};
use serde::{de::DeserializeSeed as _, Deserialize, Serialize};
use serde_json::{value, Map, Value};
use tokio::{
    net::TcpListener,
    sync::{
        broadcast::{self, Receiver as BroadcastReceiver, Sender as BroadcastSender},
        oneshot::{self, Sender as OneshotSender},
    },
    task,
};

/// The default port that Bevy will listen on.
///
/// This value was chosen randomly.
pub const DEFAULT_PORT: u16 = 15702;

const CHANNEL_SIZE: usize = 16;

/// Add this plugin to your [`App`] to allow remote connections to inspect and modify entities.
///
/// By default, this is [`DEFAULT_PORT`]: 15702.
pub struct RemotePlugin {
    /// The port that Bevy will listen on.
    pub port: u16,
}

/// A resource containing the port number that Bevy will listen on.
#[derive(Resource, Reflect)]
pub struct RemotePort(pub u16);

/// A single request from a Bevy Remote Protocol client to the server,
/// serialized in JSON.
///
/// The JSON payload is expected to look like this:
///
/// ```json
/// {
///     "request": "GET",
///     "id": 0,
///     "params": {
///         "data": {
///             "entity": 4294967298,
///             "components": [
///                 "bevy_transform::components::transform::Transform"
///             ]
///         }
///     }
/// }
/// ```
#[derive(Serialize, Deserialize, Clone)]
#[serde(tag = "request", content = "params")]
pub enum BrpRequest {
    /// `GET`: Retrieves one or more components from the entity with the given
    /// ID.
    ///
    /// The server responds with a `BrpResponse::Get`.
    #[serde(rename = "GET")]
    Get {
        /// The ID of the entity from which components are to be requested.
        entity: Entity,

        /// The *full paths* of the component types that are to be requested
        /// from the entity.
        ///
        /// Note that these strings must consist of the *full* type paths: e.g.
        /// `bevy_transform::components::transform::Transform`, not just
        /// `Transform`.
        components: Vec<String>,
    },

    /// `QUERY`: Performs a query over components in the ECS, returning entities
    /// and component values that match.
    ///
    /// The server responds with a `BrpResponse::Query`.
    #[serde(rename = "QUERY")]
    Query {
        /// The components to select.
        data: BrpQuery,

        /// An optional filter that specifies which entities to include or
        /// exclude from the results.
        #[serde(default)]
        filter: BrpQueryFilter,
    },

    /// `SPAWN`: Creates a new entity with the given components and responds
    /// with its ID.
    ///
    /// The server responds with a `BrpResponse::Entity`.
    #[serde(rename = "SPAWN")]
    Spawn {
        /// A map from each component's *full path* to its serialized value.
        ///
        /// These components will be added to the entity.
        ///
        /// Note that the keys of the map must be the *full* type paths: e.g.
        /// `bevy_transform::components::transform::Transform`, not just
        /// `Transform`.
        components: HashMap<String, Value>,
    },

    /// `DESTROY`: Given an ID, despawns the entity with that ID.
    ///
    /// The server responds with a `BrpResponse::Ok`.
    #[serde(rename = "DESTROY")]
    Destroy {
        /// The ID of the entity to despawn.
        entity: Entity,
    },

    /// `REMOVE`: Deletes one or more components from an entity.
    ///
    /// The server responds with a `BrpResponse::Ok`.
    #[serde(rename = "REMOVE")]
    Remove {
        /// The ID of the entity from which components are to be removed.
        entity: Entity,

        /// The *full paths* of the component types that are to be removed from
        /// the entity.
        ///
        /// Note that these strings must consist of the *full* type paths: e.g.
        /// `bevy_transform::components::transform::Transform`, not just
        /// `Transform`.
        components: Vec<String>,
    },

    /// `INSERT`: Adds one or more components to an entity.
    ///
    /// The server responds with a `BrpResponse::Ok`.
    #[serde(rename = "INSERT")]
    Insert {
        /// The ID of the entity that components are to be added to.
        entity: Entity,

        /// A map from each component's *full path* to its serialized value.
        ///
        /// These components will be added to the entity.
        ///
        /// Note that the keys of the map must be the *full* type paths: e.g.
        /// `bevy_transform::components::transform::Transform`, not just
        /// `Transform`.
        components: HashMap<String, Value>,
    },

    /// `REPARENT`: Changes the parent of an entity.
    ///
    /// The server responds with a `BrpResponse::Ok`.
    #[serde(rename = "REPARENT")]
    Reparent {
        /// The IDs of the entities that are to become the new children of the
        /// `parent`.
        entities: Vec<Entity>,

        /// The IDs of the entity that will become the new parent of the
        /// `entities`.
        parent: Entity,
    },

    /// `LIST`: Returns a list of all type names of registered components in the
    /// system, or those on an entity.
    #[serde(rename = "LIST")]
    List {
        /// The entity to query.
        ///
        /// If not specified, this request returns the names of all registered
        /// components.
        entity: Option<Entity>,
    },
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

/// A response from the world to the client.
#[derive(Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum BrpResponse {
    /// Acknowledgment with no further information.
    ///
    /// This is sent in response to with `DESTROY`, `REMOVE`, `INSERT`, and `REPARENT` messages.
    Ok,

    /// Specifies a single entity.
    ///
    /// This is sent in response to `SPAWN`.
    Entity {
        /// The ID of the entity in question.
        entity: Entity,
    },

    /// The response to a `GET` request.
    Get {
        /// The ID of the entity for which components were requested.
        entity: Entity,
        /// The values of the requested components.
        components: HashMap<String, Value>,
    },

    /// The response to a `LIST` request.
    List {
        /// The ID of the entity for which component names were requested, if
        /// present.
        ///
        /// If this is `None`, then `components` contains the name of all
        /// reflectable components known to the system.
        entity: Option<Entity>,

        /// The full type names of the registered components.
        components: Vec<String>,
    },

    /// The response to a `QUERY` request.
    Query {
        /// All results of the query: the entities and the requested components.
        rows: Vec<BrpQueryRow>,
    },
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

/// A message from the Bevy Remote Protocol server thread to the main world.
///
/// This is placed in the [`BrpMailbox`].
#[derive(Clone)]
pub struct BrpMessage {
    /// The deserialized request from the client.
    request: BrpRequest,

    /// The channel on which the response is to be sent.
    ///
    /// The value sent here is serialized and sent back to the client.
    sender: Arc<Mutex<Option<OneshotSender<AnyhowResult<BrpResponse>>>>>,
}

/// A resource that receives messages sent by Bevy Remote Protocol clients.
///
/// Every frame, the [`process_remote_requests`] system drains this mailbox, and
/// processes the messages within.
#[derive(Resource, Deref, DerefMut)]
pub struct BrpMailbox(BroadcastReceiver<BrpMessage>);

impl Default for RemotePlugin {
    fn default() -> Self {
        RemotePlugin { port: DEFAULT_PORT }
    }
}

impl Plugin for RemotePlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(RemotePort(self.port))
            .add_systems(Startup, start_server)
            .add_systems(Update, process_remote_requests);
    }
}

/// A system that starts up the Bevy Remote Protocol server.
fn start_server(mut commands: Commands, remote_port: Res<RemotePort>) {
    // Create the channel and the mailbox.
    let (request_sender, request_receiver) = broadcast::channel(CHANNEL_SIZE);
    commands.insert_resource(BrpMailbox(request_receiver));

    let port = remote_port.0;
    thread::spawn(move || server_main(port, request_sender));
}

/// A system that receives requests placed in the [`BrpMailbox`] and processes
/// them.
///
/// This needs exclusive access to the [`World`] because clients can manipulate
/// anything in the ECS.
fn process_remote_requests(world: &mut World) {
    if !world.contains_resource::<BrpMailbox>() {
        return;
    }

    let app_type_registry = world.resource::<AppTypeRegistry>().clone();
    while let Ok(message) = world.resource_mut::<BrpMailbox>().try_recv() {
        let Ok(mut sender) = message.sender.lock() else {
            continue;
        };
        let Some(sender) = sender.take() else {
            continue;
        };
        let _ = sender.send(process_remote_request(
            world,
            message.request,
            &app_type_registry,
        ));
    }
}

/// Processes a single request coming from a client.
fn process_remote_request(
    world: &mut World,
    request: BrpRequest,
    app_type_registry: &AppTypeRegistry,
) -> AnyhowResult<BrpResponse> {
    match request {
        BrpRequest::Get { entity, components } => {
            process_remote_get_request(entity, components, world, app_type_registry)
        }

        BrpRequest::Query { data, filter } => {
            process_remote_query_request(data, filter, world, app_type_registry)
        }

        BrpRequest::Spawn { components } => {
            let type_registry = app_type_registry.read();
            let reflect_components = deserialize_components(&type_registry, components)?;
            insert_reflected_components(&type_registry, world.spawn_empty(), reflect_components)?;
            Ok(BrpResponse::Ok)
        }

        BrpRequest::Insert { entity, components } => {
            process_remote_insert_request(entity, components, world, app_type_registry)
        }

        BrpRequest::Remove { entity, components } => {
            process_remote_remove_request(entity, components, world, app_type_registry)
        }

        BrpRequest::Destroy { entity } => {
            get_entity_mut(world, entity)?.despawn();
            Ok(BrpResponse::Ok)
        }

        BrpRequest::Reparent { entities, parent } => {
            process_remote_reparent_request(entities, parent, world)
        }

        BrpRequest::List { entity } => {
            process_remote_list_request(entity, world, app_type_registry)
        }
    }
}

/// Handles a `GET` request coming from a client.
fn process_remote_get_request(
    entity: Entity,
    components: Vec<String>,
    world: &mut World,
    app_type_registry: &AppTypeRegistry,
) -> AnyhowResult<BrpResponse> {
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

    Ok(BrpResponse::Get {
        entity,
        components: serialized_components_map,
    })
}

/// Handles a `QUERY` request coming from a client.
fn process_remote_query_request(
    data: BrpQuery,
    filter: BrpQueryFilter,
    world: &mut World,
    app_type_registry: &AppTypeRegistry,
) -> AnyhowResult<BrpResponse> {
    let BrpQuery {
        components,
        option,
        has,
    } = data;
    let BrpQueryFilter { without, with } = filter;

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

    Ok(BrpResponse::Query { rows })
}

/// Handles an `INSERT` request (insert components) coming from a client.
fn process_remote_insert_request(
    entity: Entity,
    components: HashMap<String, Value>,
    world: &mut World,
    app_type_registry: &AppTypeRegistry,
) -> AnyhowResult<BrpResponse> {
    let type_registry = app_type_registry.read();
    let reflect_components = deserialize_components(&type_registry, components)?;

    insert_reflected_components(
        &type_registry,
        get_entity_mut(world, entity)?,
        reflect_components,
    )?;

    Ok(BrpResponse::Ok)
}

/// Handles a `REMOVE` request (remove components) coming from a client.
fn process_remote_remove_request(
    entity: Entity,
    components: Vec<String>,
    world: &mut World,
    app_type_registry: &AppTypeRegistry,
) -> AnyhowResult<BrpResponse> {
    let type_registry = app_type_registry.read();
    let component_ids = get_component_ids(&type_registry, world, components)?;

    // Remove the components.
    let mut entity_world_mut = get_entity_mut(world, entity)?;
    for (_, component_id) in component_ids {
        entity_world_mut.remove_by_id(component_id);
    }

    Ok(BrpResponse::Ok)
}

/// Handles a `REPARENT` request coming from a client.
fn process_remote_reparent_request(
    entities: Vec<Entity>,
    parent: Entity,
    world: &mut World,
) -> AnyhowResult<BrpResponse> {
    let mut parent_commands = get_entity_mut(world, parent)?;
    for entity in entities {
        if entity == parent {
            return Err(anyhow!("Can't parent an object to itself"));
        }
        parent_commands.add_child(entity);
    }

    Ok(BrpResponse::Ok)
}

/// Handles a `LIST` request (list all components) coming from a client.
fn process_remote_list_request(
    entity: Option<Entity>,
    world: &World,
    app_type_registry: &AppTypeRegistry,
) -> AnyhowResult<BrpResponse> {
    let mut result = vec![];
    let type_registry = app_type_registry.read();

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

    Ok(BrpResponse::List {
        entity,
        components: result,
    })
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

/// The Bevy Remote Protocol server main loop.
#[tokio::main]
async fn server_main(port: u16, sender: BroadcastSender<BrpMessage>) -> AnyhowResult<()> {
    let addr = SocketAddr::from(([127, 0, 0, 1], port));

    let listener = TcpListener::bind(addr).await?;

    loop {
        let (stream, _) = listener.accept().await?;

        let io = TokioIo::new(stream);
        let sender = sender.clone();

        task::spawn(async move {
            if let Err(err) = Builder::new(TokioExecutor::new())
                .serve_connection(
                    io,
                    service::service_fn(|request| process_request(request, sender.clone())),
                )
                .await
            {
                error!("Tokio error: {:?}", err);
            }
        });
    }
}

/// A helper function for the Bevy Remote Protocol server that handles a single
/// request coming from a client.
async fn process_request(
    request: Request<Incoming>,
    sender: BroadcastSender<BrpMessage>,
) -> AnyhowResult<Response<Full<Bytes>>> {
    let request_bytes = request.into_body().collect().await?.to_bytes();
    let value: Value = serde_json::from_slice(&request_bytes)?;

    let Value::Object(mut object_value) = value else {
        return Err(anyhow!("JSON value wasn't an object"));
    };
    let Some(id_value) = object_value.remove("id") else {
        return Err(anyhow!("JSON value had no `id` field"));
    };

    let mut value = match process_request_body(object_value, &sender).await {
        Ok(mut value) => {
            value.insert("status".to_owned(), "OK".into());
            value
        }
        Err(err) => {
            let mut response = Map::new();
            response.insert("status".to_owned(), "ERROR".into());
            response.insert("message".to_owned(), err.to_string().into());
            response
        }
    };

    // Echo the same `id` value back to the client.
    value.insert("id".to_owned(), id_value);

    // Serialize and return the JSON as a response.
    let string = serde_json::to_string(&value)?;
    Ok(Response::new(Full::new(Bytes::from(
        string.as_bytes().to_owned(),
    ))))
}

/// A helper function for the Bevy Remote Protocol server that parses a single
/// request coming from a client and places it in the [`BrpMailbox`].
async fn process_request_body(
    request: Map<String, Value>,
    sender: &BroadcastSender<BrpMessage>,
) -> AnyhowResult<Map<String, Value>> {
    let request = value::from_value(Value::Object(request))?;
    let (response_sender, response_receiver) = oneshot::channel();

    let _ = sender.send(BrpMessage {
        request,
        sender: Arc::new(Mutex::new(Some(response_sender))),
    });

    let response = response_receiver.await??;
    match value::to_value(response)? {
        Value::Object(map) => Ok(map),
        _ => Err(anyhow!("Response wasn't an object")),
    }
}

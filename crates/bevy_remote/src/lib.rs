//! An implementation of the Bevy Remote Protocol, to allow for remote control of a Bevy app.
//!
//! Adding the [`RemotePlugin`] to your [`App`] will setup everything needed without
//! starting any transports. To start accepting remote connections you will need to
//! add a second plugin like the [`RemoteHttpPlugin`](http::RemoteHttpPlugin) to enable communication
//! over HTTP. These *remote clients* can inspect and alter the state of the
//! entity-component system.
//!
//! The Bevy Remote Protocol is based on the JSON-RPC 2.0 protocol.
//!
//! ## Request objects
//!
//! A typical client request might look like this:
//!
//! ```json
//! {
//!     "method": "bevy/get",
//!     "id": 0,
//!     "params": {
//!         "entity": 4294967298,
//!         "components": [
//!             "bevy_transform::components::transform::Transform"
//!         ]
//!     }
//! }
//! ```
//!
//! The `id` and `method` fields are required. The `params` field may be omitted
//! for certain methods:
//!
//! * `id` is arbitrary JSON data. The server completely ignores its contents,
//!   and the client may use it for any purpose. It will be copied via
//!   serialization and deserialization (so object property order, etc. can't be
//!   relied upon to be identical) and sent back to the client as part of the
//!   response.
//!
//! * `method` is a string that specifies one of the possible [`BrpRequest`]
//!   variants: `bevy/query`, `bevy/get`, `bevy/insert`, etc. It's case-sensitive.
//!
//! * `params` is parameter data specific to the request.
//!
//! For more information, see the documentation for [`BrpRequest`].
//! [`BrpRequest`] is serialized to JSON via `serde`, so [the `serde`
//! documentation] may be useful to clarify the correspondence between the Rust
//! structure and the JSON format.
//!
//! ## Response objects
//!
//! A response from the server to the client might look like this:
//!
//! ```json
//! {
//!     "jsonrpc": "2.0",
//!     "id": 0,
//!     "result": {
//!         "bevy_transform::components::transform::Transform": {
//!             "rotation": { "x": 0.0, "y": 0.0, "z": 0.0, "w": 1.0 },
//!             "scale": { "x": 1.0, "y": 1.0, "z": 1.0 },
//!             "translation": { "x": 0.0, "y": 0.5, "z": 0.0 }
//!         }
//!     }
//! }
//! ```
//!
//! The `id` field will always be present. The `result` field will be present if the
//! request was successful. Otherwise, an `error` field will replace it.
//!
//! * `id` is the arbitrary JSON data that was sent as part of the request. It
//!   will be identical to the `id` data sent during the request, modulo
//!   serialization and deserialization. If there's an error reading the `id` field,
//!   it will be `null`.
//!
//! * `result` will be present if the request succeeded and will contain the response
//!   specific to the request.
//!
//! * `error` will be present if the request failed and will contain an error object
//!   with more information about the cause of failure.
//!
//! ## Error objects
//!
//! An error object might look like this:
//!
//! ```json
//! {
//!     "code": -32602,
//!     "message": "Missing \"entity\" field"
//! }
//! ```
//!
//! The `code` and `message` fields will always be present. There may also be a `data` field.
//!
//! * `code` is an integer representing the kind of an error that happened. Error codes documented
//!   in the [`error_codes`] module.
//!
//! * `message` is a short, one-sentence human-readable description of the error.
//!
//! * `data` is an optional field of arbitrary type containing additional information about the error.
//!
//! ## Built-in methods
//!
//! The Bevy Remote Protocol includes a number of built-in methods for accessing and modifying data
//! in the ECS. Each of these methods uses the `bevy/` prefix, which is a namespace reserved for
//! BRP built-in methods.
//!
//! ### bevy/get
//!
//! Retrieve the values of one or more components from an entity.
//!
//! `params`:
//! - `entity`: The ID of the entity whose components will be fetched.
//! - `components`: An array of [fully-qualified type names] of components to fetch.
//! - `strict` (optional): A flag to enable strict mode which will fail if any one of the
//!   components is not present or can not be reflected. Defaults to false.
//!
//! If `strict` is false:
//!
//! `result`:
//! - `components`: A map associating each type name to its value on the requested entity.
//! - `errors`: A map associating each type name with an error if it was not on the entity
//!   or could not be reflected.
//!
//! If `strict` is true:
//!
//! `result`: A map associating each type name to its value on the requested entity.
//!
//! ### bevy/query
//!
//! Perform a query over components in the ECS, returning all matching entities and their associated
//! component values.
//!
//! All of the arrays that comprise this request are optional, and when they are not provided, they
//! will be treated as if they were empty.
//!
//! `params`:
//! - `data`:
//!   - `components` (optional): An array of [fully-qualified type names] of components to fetch.
//!   - `option` (optional): An array of fully-qualified type names of components to fetch optionally.
//!   - `has` (optional): An array of fully-qualified type names of components whose presence will be
//!      reported as boolean values.
//! - `filter` (optional):
//!   - `with` (optional): An array of fully-qualified type names of components that must be present
//!     on entities in order for them to be included in results.
//!   - `without` (optional): An array of fully-qualified type names of components that must *not* be
//!     present on entities in order for them to be included in results.
//!
//! `result`: An array, each of which is an object containing:
//! - `entity`: The ID of a query-matching entity.
//! - `components`: A map associating each type name from `components`/`option` to its value on the matching
//!   entity if the component is present.
//! - `has`: A map associating each type name from `has` to a boolean value indicating whether or not the
//!   entity has that component. If `has` was empty or omitted, this key will be omitted in the response.
//!
//! ### bevy/spawn
//!
//! Create a new entity with the provided components and return the resulting entity ID.
//!
//! `params`:
//! - `components`: A map associating each component's [fully-qualified type name] with its value.
//!
//! `result`:
//! - `entity`: The ID of the newly spawned entity.
//!
//! ### bevy/destroy
//!
//! Despawn the entity with the given ID.
//!
//! `params`:
//! - `entity`: The ID of the entity to be despawned.
//!
//! `result`: null.
//!
//! ### bevy/remove
//!
//! Delete one or more components from an entity.
//!
//! `params`:
//! - `entity`: The ID of the entity whose components should be removed.
//! - `components`: An array of [fully-qualified type names] of components to be removed.
//!
//! `result`: null.
//!
//! ### bevy/insert
//!
//! Insert one or more components into an entity.
//!
//! `params`:
//! - `entity`: The ID of the entity to insert components into.
//! - `components`: A map associating each component's fully-qualified type name with its value.
//!
//! `result`: null.
//!
//! ### bevy/reparent
//!
//! Assign a new parent to one or more entities.
//!
//! `params`:
//! - `entities`: An array of entity IDs of entities that will be made children of the `parent`.
//! - `parent` (optional): The entity ID of the parent to which the child entities will be assigned.
//!   If excluded, the given entities will be removed from their parents.
//!
//! `result`: null.
//!
//! ### bevy/list
//!
//! List all registered components or all components present on an entity.
//!
//! When `params` is not provided, this lists all registered components. If `params` is provided,
//! this lists only those components present on the provided entity.
//!
//! `params` (optional):
//! - `entity`: The ID of the entity whose components will be listed.
//!
//! `result`: An array of fully-qualified type names of components.
//!
//! ## Custom methods
//!
//! In addition to the provided methods, the Bevy Remote Protocol can be extended to include custom
//! methods. This is primarily done during the initialization of [`RemotePlugin`], although the
//! methods may also be extended at runtime using the [`RemoteMethods`] resource.
//!
//! ### Example
//! ```ignore
//! fn main() {
//!     App::new()
//!         .add_plugins(DefaultPlugins)
//!         .add_plugins(
//!             // `default` adds all of the built-in methods, while `with_method` extends them
//!             RemotePlugin::default()
//!                 .with_method("super_user/cool_method", path::to::my::cool::handler)
//!                 // ... more methods can be added by chaining `with_method`
//!         )
//!         .add_systems(
//!             // ... standard application setup
//!         )
//!         .run();
//! }
//! ```
//!
//! The handler is expected to be a system-convertible function which takes optional JSON parameters
//! as input and returns a [`BrpResult`]. This means that it should have a type signature which looks
//! something like this:
//! ```
//! # use serde_json::Value;
//! # use bevy_ecs::prelude::{In, World};
//! # use bevy_remote::BrpResult;
//! fn handler(In(params): In<Option<Value>>, world: &mut World) -> BrpResult {
//!     todo!()
//! }
//! ```
//!
//! Arbitrary system parameters can be used in conjunction with the optional `Value` input. The
//! handler system will always run with exclusive `World` access.
//!
//! [the `serde` documentation]: https://serde.rs/
//! [fully-qualified type names]: bevy_reflect::TypePath::type_path
//! [fully-qualified type name]: bevy_reflect::TypePath::type_path

use async_channel::{Receiver, Sender};
use bevy_app::prelude::*;
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{
    entity::Entity,
    system::{Commands, In, IntoSystem, Resource, System, SystemId},
    world::World,
};
use bevy_utils::{prelude::default, HashMap};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::RwLock;

pub mod builtin_methods;
pub mod http;

const CHANNEL_SIZE: usize = 16;

/// Add this plugin to your [`App`] to allow remote connections to inspect and modify entities.
///
/// This the main plugin for `bevy_remote`. See the [crate-level documentation] for details on
/// the available protocols and its default methods.
///
/// [crate-level documentation]: crate
pub struct RemotePlugin {
    /// The verbs that the server will recognize and respond to.
    methods: RwLock<
        Vec<(
            String,
            Box<dyn System<In = In<Option<Value>>, Out = BrpResult>>,
        )>,
    >,
}

impl RemotePlugin {
    /// Create a [`RemotePlugin`] with the default address and port but without
    /// any associated methods.
    fn empty() -> Self {
        Self {
            methods: RwLock::new(vec![]),
        }
    }

    /// Add a remote method to the plugin using the given `name` and `handler`.
    #[must_use]
    pub fn with_method<M>(
        mut self,
        name: impl Into<String>,
        handler: impl IntoSystem<In<Option<Value>>, BrpResult, M>,
    ) -> Self {
        self.methods
            .get_mut()
            .unwrap()
            .push((name.into(), Box::new(IntoSystem::into_system(handler))));
        self
    }
}

impl Default for RemotePlugin {
    fn default() -> Self {
        Self::empty()
            .with_method(
                builtin_methods::BRP_GET_METHOD,
                builtin_methods::process_remote_get_request,
            )
            .with_method(
                builtin_methods::BRP_QUERY_METHOD,
                builtin_methods::process_remote_query_request,
            )
            .with_method(
                builtin_methods::BRP_SPAWN_METHOD,
                builtin_methods::process_remote_spawn_request,
            )
            .with_method(
                builtin_methods::BRP_INSERT_METHOD,
                builtin_methods::process_remote_insert_request,
            )
            .with_method(
                builtin_methods::BRP_REMOVE_METHOD,
                builtin_methods::process_remote_remove_request,
            )
            .with_method(
                builtin_methods::BRP_DESTROY_METHOD,
                builtin_methods::process_remote_destroy_request,
            )
            .with_method(
                builtin_methods::BRP_REPARENT_METHOD,
                builtin_methods::process_remote_reparent_request,
            )
            .with_method(
                builtin_methods::BRP_LIST_METHOD,
                builtin_methods::process_remote_list_request,
            )
    }
}

impl Plugin for RemotePlugin {
    fn build(&self, app: &mut App) {
        let mut remote_methods = RemoteMethods::new();
        let plugin_methods = &mut *self.methods.write().unwrap();
        for (name, system) in plugin_methods.drain(..) {
            remote_methods.insert(
                name,
                app.main_mut().world_mut().register_boxed_system(system),
            );
        }

        app.insert_resource(remote_methods)
            .add_systems(PreStartup, setup_mailbox_channel)
            .add_systems(Update, process_remote_requests);
    }
}

/// The type of a function that implements a remote method (`bevy/get`, `bevy/query`, etc.)
///
/// The first parameter is the JSON value of the `params`. Typically, an
/// implementation will deserialize these as the first thing they do.
///
/// The returned JSON value will be returned as the response. Bevy will
/// automatically populate the `id` field before sending.
pub type RemoteMethod = SystemId<In<Option<Value>>, BrpResult>;

/// Holds all implementations of methods known to the server.
///
/// Custom methods can be added to this list using [`RemoteMethods::insert`].
#[derive(Debug, Resource, Default)]
pub struct RemoteMethods(HashMap<String, RemoteMethod>);

impl RemoteMethods {
    /// Creates a new [`RemoteMethods`] resource with no methods registered in it.
    pub fn new() -> Self {
        default()
    }

    /// Adds a new method, replacing any existing method with that name.
    ///
    /// If there was an existing method with that name, returns its handler.
    pub fn insert(
        &mut self,
        method_name: impl Into<String>,
        handler: RemoteMethod,
    ) -> Option<RemoteMethod> {
        self.0.insert(method_name.into(), handler)
    }
}

/// A single request from a Bevy Remote Protocol client to the server,
/// serialized in JSON.
///
/// The JSON payload is expected to look like this:
///
/// ```json
/// {
///     "jsonrpc": "2.0",
///     "method": "bevy/get",
///     "id": 0,
///     "params": {
///         "entity": 4294967298,
///         "components": [
///             "bevy_transform::components::transform::Transform"
///         ]
///     }
/// }
/// ```
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BrpRequest {
    /// This field is mandatory and must be set to `"2.0"` for the request to be accepted.
    pub jsonrpc: String,

    /// The action to be performed.
    pub method: String,

    /// Arbitrary data that will be returned verbatim to the client as part of
    /// the response.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<Value>,

    /// The parameters, specific to each method.
    ///
    /// These are passed as the first argument to the method handler.
    /// Sometimes params can be omitted.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
}

/// A response according to BRP.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BrpResponse {
    /// This field is mandatory and must be set to `"2.0"`.
    pub jsonrpc: &'static str,

    /// The id of the original request.
    pub id: Option<Value>,

    /// The actual response payload.
    #[serde(flatten)]
    pub payload: BrpPayload,
}

impl BrpResponse {
    /// Generates a [`BrpResponse`] from an id and a `Result`.
    #[must_use]
    pub fn new(id: Option<Value>, result: BrpResult) -> Self {
        Self {
            jsonrpc: "2.0",
            id,
            payload: BrpPayload::from(result),
        }
    }
}

/// A result/error payload present in every response.
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum BrpPayload {
    /// `Ok` variant
    Result(Value),
    /// `Err` variant
    Error(BrpError),
}

impl From<BrpResult> for BrpPayload {
    fn from(value: BrpResult) -> Self {
        match value {
            Ok(v) => Self::Result(v),
            Err(err) => Self::Error(err),
        }
    }
}

/// An error a request might return.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BrpError {
    /// Defines the general type of the error.
    pub code: i16,
    /// Short, human-readable description of the error.
    pub message: String,
    /// Optional additional error data.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

impl BrpError {
    /// Entity wasn't found.
    #[must_use]
    pub fn entity_not_found(entity: Entity) -> Self {
        Self {
            code: error_codes::ENTITY_NOT_FOUND,
            message: format!("Entity {entity} not found"),
            data: None,
        }
    }

    /// Component wasn't found in an entity.
    #[must_use]
    pub fn component_not_present(component: &str, entity: Entity) -> Self {
        Self {
            code: error_codes::COMPONENT_NOT_PRESENT,
            message: format!("Component `{component}` not present in Entity {entity}"),
            data: None,
        }
    }

    /// An arbitrary component error. Possibly related to reflection.
    #[must_use]
    pub fn component_error<E: ToString>(error: E) -> Self {
        Self {
            code: error_codes::COMPONENT_ERROR,
            message: error.to_string(),
            data: None,
        }
    }

    /// An arbitrary internal error.
    #[must_use]
    pub fn internal<E: ToString>(error: E) -> Self {
        Self {
            code: error_codes::INTERNAL_ERROR,
            message: error.to_string(),
            data: None,
        }
    }

    /// Attempt to reparent an entity to itself.
    #[must_use]
    pub fn self_reparent(entity: Entity) -> Self {
        Self {
            code: error_codes::SELF_REPARENT,
            message: format!("Cannot reparent Entity {entity} to itself"),
            data: None,
        }
    }
}

/// Error codes used by BRP.
pub mod error_codes {
    // JSON-RPC errors
    // Note that the range -32728 to -32000 (inclusive) is reserved by the JSON-RPC specification.

    /// Invalid JSON.
    pub const PARSE_ERROR: i16 = -32700;

    /// JSON sent is not a valid request object.
    pub const INVALID_REQUEST: i16 = -32600;

    /// The method does not exist / is not available.
    pub const METHOD_NOT_FOUND: i16 = -32601;

    /// Invalid method parameter(s).
    pub const INVALID_PARAMS: i16 = -32602;

    /// Internal error.
    pub const INTERNAL_ERROR: i16 = -32603;

    // Bevy errors (i.e. application errors)

    /// Entity not found.
    pub const ENTITY_NOT_FOUND: i16 = -23401;

    /// Could not reflect or find component.
    pub const COMPONENT_ERROR: i16 = -23402;

    /// Could not find component in entity.
    pub const COMPONENT_NOT_PRESENT: i16 = -23403;

    /// Cannot reparent an entity to itself.
    pub const SELF_REPARENT: i16 = -23404;
}

/// The result of a request.
pub type BrpResult = Result<Value, BrpError>;

/// The requests may occur on their own or in batches.
/// Actual parsing is deferred for the sake of proper
/// error reporting.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum BrpBatch {
    /// Multiple requests with deferred parsing.
    Batch(Vec<Value>),
    /// A single request with deferred parsing.
    Single(Value),
}

/// A message from the Bevy Remote Protocol server thread to the main world.
///
/// This is placed in the [`BrpReceiver`].
#[derive(Debug, Clone)]
pub struct BrpMessage {
    /// The request method.
    pub method: String,

    /// The request params.
    pub params: Option<Value>,

    /// The channel on which the response is to be sent.
    ///
    /// The value sent here is serialized and sent back to the client.
    pub sender: Sender<BrpResult>,
}

/// A resource holding the matching sender for the [`BrpReceiver`]'s receiver.
#[derive(Debug, Resource, Deref, DerefMut)]
pub struct BrpSender(Sender<BrpMessage>);

/// A resource that receives messages sent by Bevy Remote Protocol clients.
///
/// Every frame, the `process_remote_requests` system drains this mailbox and
/// processes the messages within.
#[derive(Debug, Resource, Deref, DerefMut)]
pub struct BrpReceiver(Receiver<BrpMessage>);

fn setup_mailbox_channel(mut commands: Commands) {
    // Create the channel and the mailbox.
    let (request_sender, request_receiver) = async_channel::bounded(CHANNEL_SIZE);
    commands.insert_resource(BrpSender(request_sender));
    commands.insert_resource(BrpReceiver(request_receiver));
}

/// A system that receives requests placed in the [`BrpReceiver`] and processes
/// them, using the [`RemoteMethods`] resource to map each request to its handler.
///
/// This needs exclusive access to the [`World`] because clients can manipulate
/// anything in the ECS.
fn process_remote_requests(world: &mut World) {
    if !world.contains_resource::<BrpReceiver>() {
        return;
    }

    while let Ok(message) = world.resource_mut::<BrpReceiver>().try_recv() {
        // Fetch the handler for the method. If there's no such handler
        // registered, return an error.
        let methods = world.resource::<RemoteMethods>();

        let Some(handler) = methods.0.get(&message.method) else {
            let _ = message.sender.force_send(Err(BrpError {
                code: error_codes::METHOD_NOT_FOUND,
                message: format!("Method `{}` not found", message.method),
                data: None,
            }));
            continue;
        };

        // Execute the handler, and send the result back to the client.
        let result = match world.run_system_with_input(*handler, message.params) {
            Ok(result) => result,
            Err(error) => {
                let _ = message.sender.force_send(Err(BrpError {
                    code: error_codes::INTERNAL_ERROR,
                    message: format!("Failed to run method handler: {error}"),
                    data: None,
                }));
                continue;
            }
        };

        let _ = message.sender.force_send(result);
    }
}

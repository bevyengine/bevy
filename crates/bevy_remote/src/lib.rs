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
//!     "method": "world.get_components",
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
//!   variants: `world.query`, `world.get_components`, `world.insert_components`, etc. It's case-sensitive.
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
//! in the ECS.
//!
//! ### `world.get_components`
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
//! ### `world.query`
//!
//! Perform a query over components in the ECS, returning all matching entities and their associated
//! component values.
//!
//! All of the arrays that comprise this request are optional, and when they are not provided, they
//! will be treated as if they were empty.
//!
//! `params`:
//! - `data`:
//!   - `components` (optional): An array of [fully-qualified type names] of components to fetch,
//!     see _below_ example for a query to list all the type names in **your** project.
//!   - `option` (optional): An array of fully-qualified type names of components to fetch optionally.
//!     to fetch all reflectable components, you can pass in the string `"all"`.
//!   - `has` (optional): An array of fully-qualified type names of components whose presence will be
//!     reported as boolean values.
//! - `filter` (optional):
//!   - `with` (optional): An array of fully-qualified type names of components that must be present
//!     on entities in order for them to be included in results.
//!   - `without` (optional): An array of fully-qualified type names of components that must *not* be
//!     present on entities in order for them to be included in results.
//! - `strict` (optional): A flag to enable strict mode which will fail if any one of the components
//!   is not present or can not be reflected. Defaults to false.
//!
//! `result`: An array, each of which is an object containing:
//! - `entity`: The ID of a query-matching entity.
//! - `components`: A map associating each type name from `components`/`option` to its value on the matching
//!   entity if the component is present.
//! - `has`: A map associating each type name from `has` to a boolean value indicating whether or not the
//!   entity has that component. If `has` was empty or omitted, this key will be omitted in the response.
//!
//! ### Example
//! To use the query API and retrieve Transform data for all entities that have a Transform
//! use this query:
//!
//! ```json
//! {
//!     "jsonrpc": "2.0",
//!     "method": "bevy/query",
//!     "id": 0,
//!     "params": {
//!         "data": {
//!             "components": ["bevy_transform::components::transform::Transform"]
//!             "option": [],
//!             "has": []
//!        },
//!        "filter": {
//!           "with": [],
//!           "without": []
//!         },
//!         "strict": false
//!     }
//! }
//! ```
//!
//!
//! To query all entities and all of their Reflectable components (and retrieve their values), you can pass in "all" for the option field:
//! ```json
//! {
//!     "jsonrpc": "2.0",
//!     "method": "bevy/query",
//!     "id": 0,
//!     "params": {
//!         "data": {
//!             "components": []
//!             "option": "all",
//!             "has": []
//!        },
//!        "filter": {
//!            "with": [],
//!           "without": []
//!         },
//!         "strict": false
//!     }
//! }
//! ```
//!
//! This should return you something like the below (in a larger list):
//! ```json
//! {
//!      "components": {
//!        "bevy_camera::Camera3d": {
//!          "depth_load_op": {
//!            "Clear": 0.0
//!          },
//!          "depth_texture_usages": 16,
//!          "screen_space_specular_transmission_quality": "Medium",
//!          "screen_space_specular_transmission_steps": 1
//!        },
//!        "bevy_core_pipeline::tonemapping::DebandDither": "Enabled",
//!        "bevy_core_pipeline::tonemapping::Tonemapping": "TonyMcMapface",
//!        "bevy_light::cluster::ClusterConfig": {
//!          "FixedZ": {
//!         "dynamic_resizing": true,
//!            "total": 4096,
//!            "z_config": {
//!              "far_z_mode": "MaxClusterableObjectRange",
//!              "first_slice_depth": 5.0
//!            },
//!            "z_slices": 24
//!          }
//!        },
//!        "bevy_camera::Camera": {
//!          "clear_color": "Default",
//!          "is_active": true,
//!          "msaa_writeback": true,
//!          "order": 0,
//!          "sub_camera_view": null,
//!          "target": {
//!            "Window": "Primary"
//!          },
//!       "viewport": null
//!        },
//!        "bevy_camera::Projection": {
//!          "Perspective": {
//!            "aspect_ratio": 1.7777777910232544,
//!            "far": 1000.0,
//!            "fov": 0.7853981852531433,
//!            "near": 0.10000000149011612
//!          }
//!        },
//!        "bevy_camera::primitives::Frustum": {},
//!     "bevy_render::sync_world::RenderEntity": 4294967291,
//!        "bevy_render::sync_world::SyncToRenderWorld": {},
//!        "bevy_render::view::Msaa": "Sample4",
//!        "bevy_camera::visibility::InheritedVisibility": true,
//!        "bevy_camera::visibility::ViewVisibility": false,
//!        "bevy_camera::visibility::Visibility": "Inherited",
//!        "bevy_camera::visibility::VisibleEntities": {},
//!        "bevy_transform::components::global_transform::GlobalTransform": [
//!          0.9635179042816162,
//!          -3.725290298461914e-9,
//!          0.26764383912086487,
//!          0.11616238951683044,
//!          0.9009039402008056,
//!          -0.4181846082210541,
//!          -0.24112138152122495,
//!          0.4340185225009918,
//!          0.8680371046066284,
//!          -2.5,
//!          4.5,
//!          9.0
//!        ],
//!        "bevy_transform::components::transform::Transform": {
//!       "rotation": [
//!            -0.22055435180664065,
//!            -0.13167093694210052,
//!            -0.03006339818239212,
//!            0.9659786224365234
//!          ],
//!          "scale": [
//!            1.0,
//!            1.0,
//!            1.0
//!       ],
//!          "translation": [
//!            -2.5,
//!          4.5,
//!            9.0
//!          ]
//!        },
//!        "bevy_transform::components::transform::TransformTreeChanged": null
//!      },
//!      "entity": 4294967261
//!},
//! ```
//!
//! ### `world.spawn_entity`
//!
//! Create a new entity with the provided components and return the resulting entity ID.
//!
//! `params`:
//! - `components`: A map associating each component's [fully-qualified type name] with its value.
//!
//! `result`:
//! - `entity`: The ID of the newly spawned entity.
//!
//! ### `world.despawn_entity`
//!
//! Despawn the entity with the given ID.
//!
//! `params`:
//! - `entity`: The ID of the entity to be despawned.
//!
//! `result`: null.
//!
//! ### `world.remove_components`
//!
//! Delete one or more components from an entity.
//!
//! `params`:
//! - `entity`: The ID of the entity whose components should be removed.
//! - `components`: An array of [fully-qualified type names] of components to be removed.
//!
//! `result`: null.
//!
//! ### `world.insert_components`
//!
//! Insert one or more components into an entity.
//!
//! `params`:
//! - `entity`: The ID of the entity to insert components into.
//! - `components`: A map associating each component's fully-qualified type name with its value.
//!
//! `result`: null.
//!
//! ### `world.mutate_components`
//!
//! Mutate a field in a component.
//!
//! `params`:
//! - `entity`: The ID of the entity with the component to mutate.
//! - `component`: The component's [fully-qualified type name].
//! - `path`: The path of the field within the component. See
//!   [`GetPath`](bevy_reflect::GetPath#syntax) for more information on formatting this string.
//! - `value`: The value to insert at `path`.
//!
//! `result`: null.
//!
//! ### `world.reparent_entities`
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
//! ### `world.list_components`
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
//! ### `world.get_components+watch`
//!
//! Watch the values of one or more components from an entity.
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
//! - `components`: A map of components added or changed in the last tick associating each type
//!   name to its value on the requested entity.
//! - `removed`: An array of fully-qualified type names of components removed from the entity
//!   in the last tick.
//! - `errors`: A map associating each type name with an error if it was not on the entity
//!   or could not be reflected.
//!
//! If `strict` is true:
//!
//! `result`:
//! - `components`: A map of components added or changed in the last tick associating each type
//!   name to its value on the requested entity.
//! - `removed`: An array of fully-qualified type names of components removed from the entity
//!   in the last tick.
//!
//! ### `world.list_components+watch`
//!
//! Watch all components present on an entity.
//!
//! When `params` is not provided, this lists all registered components. If `params` is provided,
//! this lists only those components present on the provided entity.
//!
//! `params`:
//! - `entity`: The ID of the entity whose components will be listed.
//!
//! `result`:
//! - `added`: An array of fully-qualified type names of components added to the entity in the
//!   last tick.
//! - `removed`: An array of fully-qualified type names of components removed from the entity
//!   in the last tick.
//!
//! ### `world.get_resources`
//!
//! Extract the value of a given resource from the world.
//!
//! `params`:
//! - `resource`: The [fully-qualified type name] of the resource to get.
//!
//! `result`:
//! - `value`: The value of the resource in the world.
//!
//! ### `world.insert_resources`
//!
//! Insert the given resource into the world with the given value.
//!
//! `params`:
//! - `resource`: The [fully-qualified type name] of the resource to insert.
//! - `value`: The value of the resource to be inserted.
//!
//! `result`: null.
//!
//! ### `world.remove_resources`
//!
//! Remove the given resource from the world.
//!
//! `params`
//! - `resource`: The [fully-qualified type name] of the resource to remove.
//!
//! `result`: null.
//!
//! ### `world.mutate_resources`
//!
//! Mutate a field in a resource.
//!
//! `params`:
//! - `resource`: The [fully-qualified type name] of the resource to mutate.
//! - `path`: The path of the field within the resource. See
//!   [`GetPath`](bevy_reflect::GetPath#syntax) for more information on formatting this string.
//! - `value`: The value to be inserted at `path`.
//!
//! `result`: null.
//!
//! ### `world.list_resources`
//!
//! List all reflectable registered resource types. This method has no parameters.
//!
//! `result`: An array of [fully-qualified type names] of registered resource types.
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

extern crate alloc;

use async_channel::{Receiver, Sender};
use bevy_app::{prelude::*, MainScheduleOrder};
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{
    entity::Entity,
    resource::Resource,
    schedule::{IntoScheduleConfigs, ScheduleLabel, SystemSet},
    system::{Commands, In, IntoSystem, ResMut, System, SystemId},
    world::World,
};
use bevy_platform::collections::HashMap;
use bevy_utils::prelude::default;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::RwLock;

pub mod builtin_methods;
#[cfg(feature = "http")]
pub mod http;
pub mod schemas;

const CHANNEL_SIZE: usize = 16;

/// Add this plugin to your [`App`] to allow remote connections to inspect and modify entities.
///
/// This the main plugin for `bevy_remote`. See the [crate-level documentation] for details on
/// the available protocols and its default methods.
///
/// [crate-level documentation]: crate
pub struct RemotePlugin {
    /// The verbs that the server will recognize and respond to.
    methods: RwLock<Vec<(String, RemoteMethodHandler)>>,
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
        self.methods.get_mut().unwrap().push((
            name.into(),
            RemoteMethodHandler::Instant(Box::new(IntoSystem::into_system(handler))),
        ));
        self
    }

    /// Add a remote method with a watching handler to the plugin using the given `name`.
    #[must_use]
    pub fn with_watching_method<M>(
        mut self,
        name: impl Into<String>,
        handler: impl IntoSystem<In<Option<Value>>, BrpResult<Option<Value>>, M>,
    ) -> Self {
        self.methods.get_mut().unwrap().push((
            name.into(),
            RemoteMethodHandler::Watching(Box::new(IntoSystem::into_system(handler))),
        ));
        self
    }
}

impl Default for RemotePlugin {
    fn default() -> Self {
        Self::empty()
            .with_method(
                builtin_methods::BRP_GET_COMPONENTS_METHOD,
                builtin_methods::process_remote_get_components_request,
            )
            .with_method(
                builtin_methods::BRP_QUERY_METHOD,
                builtin_methods::process_remote_query_request,
            )
            .with_method(
                builtin_methods::BRP_SPAWN_ENTITY_METHOD,
                builtin_methods::process_remote_spawn_entity_request,
            )
            .with_method(
                builtin_methods::BRP_INSERT_COMPONENTS_METHOD,
                builtin_methods::process_remote_insert_components_request,
            )
            .with_method(
                builtin_methods::BRP_REMOVE_COMPONENTS_METHOD,
                builtin_methods::process_remote_remove_components_request,
            )
            .with_method(
                builtin_methods::BRP_DESPAWN_COMPONENTS_METHOD,
                builtin_methods::process_remote_despawn_entity_request,
            )
            .with_method(
                builtin_methods::BRP_REPARENT_ENTITIES_METHOD,
                builtin_methods::process_remote_reparent_entities_request,
            )
            .with_method(
                builtin_methods::BRP_LIST_COMPONENTS_METHOD,
                builtin_methods::process_remote_list_components_request,
            )
            .with_method(
                builtin_methods::BRP_MUTATE_COMPONENTS_METHOD,
                builtin_methods::process_remote_mutate_components_request,
            )
            .with_method(
                builtin_methods::RPC_DISCOVER_METHOD,
                builtin_methods::process_remote_list_methods_request,
            )
            .with_watching_method(
                builtin_methods::BRP_GET_COMPONENTS_AND_WATCH_METHOD,
                builtin_methods::process_remote_get_components_watching_request,
            )
            .with_watching_method(
                builtin_methods::BRP_LIST_COMPONENTS_AND_WATCH_METHOD,
                builtin_methods::process_remote_list_components_watching_request,
            )
            .with_method(
                builtin_methods::BRP_GET_RESOURCE_METHOD,
                builtin_methods::process_remote_get_resources_request,
            )
            .with_method(
                builtin_methods::BRP_INSERT_RESOURCE_METHOD,
                builtin_methods::process_remote_insert_resources_request,
            )
            .with_method(
                builtin_methods::BRP_REMOVE_RESOURCE_METHOD,
                builtin_methods::process_remote_remove_resources_request,
            )
            .with_method(
                builtin_methods::BRP_MUTATE_RESOURCE_METHOD,
                builtin_methods::process_remote_mutate_resources_request,
            )
            .with_method(
                builtin_methods::BRP_LIST_RESOURCES_METHOD,
                builtin_methods::process_remote_list_resources_request,
            )
            .with_method(
                builtin_methods::BRP_REGISTRY_SCHEMA_METHOD,
                builtin_methods::export_registry_types,
            )
    }
}

impl Plugin for RemotePlugin {
    fn build(&self, app: &mut App) {
        let mut remote_methods = RemoteMethods::new();

        let plugin_methods = &mut *self.methods.write().unwrap();
        for (name, handler) in plugin_methods.drain(..) {
            remote_methods.insert(
                name,
                match handler {
                    RemoteMethodHandler::Instant(system) => RemoteMethodSystemId::Instant(
                        app.main_mut().world_mut().register_boxed_system(system),
                    ),
                    RemoteMethodHandler::Watching(system) => RemoteMethodSystemId::Watching(
                        app.main_mut().world_mut().register_boxed_system(system),
                    ),
                },
            );
        }

        app.init_schedule(RemoteLast)
            .world_mut()
            .resource_mut::<MainScheduleOrder>()
            .insert_after(Last, RemoteLast);

        app.insert_resource(remote_methods)
            .init_resource::<schemas::SchemaTypesMetadata>()
            .init_resource::<RemoteWatchingRequests>()
            .add_systems(PreStartup, setup_mailbox_channel)
            .configure_sets(
                RemoteLast,
                (RemoteSystems::ProcessRequests, RemoteSystems::Cleanup).chain(),
            )
            .add_systems(
                RemoteLast,
                (
                    (process_remote_requests, process_ongoing_watching_requests)
                        .chain()
                        .in_set(RemoteSystems::ProcessRequests),
                    remove_closed_watching_requests.in_set(RemoteSystems::Cleanup),
                ),
            );
    }
}

/// Schedule that contains all systems to process Bevy Remote Protocol requests
#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash, Default)]
pub struct RemoteLast;

/// The systems sets of the [`RemoteLast`] schedule.
///
/// These can be useful for ordering.
#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
pub enum RemoteSystems {
    /// Processing of remote requests.
    ProcessRequests,
    /// Cleanup (remove closed watchers etc)
    Cleanup,
}

/// Deprecated alias for [`RemoteSystems`].
#[deprecated(since = "0.17.0", note = "Renamed to `RemoteSystems`.")]
pub type RemoteSet = RemoteSystems;

/// A type to hold the allowed types of systems to be used as method handlers.
#[derive(Debug)]
pub enum RemoteMethodHandler {
    /// A handler that only runs once and returns one response.
    Instant(Box<dyn System<In = In<Option<Value>>, Out = BrpResult>>),
    /// A handler that watches for changes and response when a change is detected.
    Watching(Box<dyn System<In = In<Option<Value>>, Out = BrpResult<Option<Value>>>>),
}

/// The [`SystemId`] of a function that implements a remote instant method (`world.get_components`, `world.query`, etc.)
///
/// The first parameter is the JSON value of the `params`. Typically, an
/// implementation will deserialize these as the first thing they do.
///
/// The returned JSON value will be returned as the response. Bevy will
/// automatically populate the `id` field before sending.
pub type RemoteInstantMethodSystemId = SystemId<In<Option<Value>>, BrpResult>;

/// The [`SystemId`] of a function that implements a remote watching method (`world.get_components+watch`, `world.list_components+watch`, etc.)
///
/// The first parameter is the JSON value of the `params`. Typically, an
/// implementation will deserialize these as the first thing they do.
///
/// The optional returned JSON value will be sent as a response. If no
/// changes were detected this should be [`None`]. Re-running of this
/// handler is done in the [`RemotePlugin`].
pub type RemoteWatchingMethodSystemId = SystemId<In<Option<Value>>, BrpResult<Option<Value>>>;

/// The [`SystemId`] of a function that can be used as a remote method.
#[derive(Debug, Clone, Copy)]
pub enum RemoteMethodSystemId {
    /// A handler that only runs once and returns one response.
    Instant(RemoteInstantMethodSystemId),
    /// A handler that watches for changes and response when a change is detected.
    Watching(RemoteWatchingMethodSystemId),
}

/// Holds all implementations of methods known to the server.
///
/// Custom methods can be added to this list using [`RemoteMethods::insert`].
#[derive(Debug, Resource, Default)]
pub struct RemoteMethods(HashMap<String, RemoteMethodSystemId>);

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
        handler: RemoteMethodSystemId,
    ) -> Option<RemoteMethodSystemId> {
        self.0.insert(method_name.into(), handler)
    }

    /// Get a [`RemoteMethodSystemId`] with its method name.
    pub fn get(&self, method: &str) -> Option<&RemoteMethodSystemId> {
        self.0.get(method)
    }

    /// Get a [`Vec<String>`] with method names.
    pub fn methods(&self) -> Vec<String> {
        self.0.keys().cloned().collect()
    }
}

/// Holds the [`BrpMessage`]'s of all ongoing watching requests along with their handlers.
#[derive(Debug, Resource, Default)]
pub struct RemoteWatchingRequests(Vec<(BrpMessage, RemoteWatchingMethodSystemId)>);

/// A single request from a Bevy Remote Protocol client to the server,
/// serialized in JSON.
///
/// The JSON payload is expected to look like this:
///
/// ```json
/// {
///     "jsonrpc": "2.0",
///     "method": "world.get_components",
///     "id": 0,
///     "params": {
///         "entity": 4294967298,
///         "components": [
///             "bevy_transform::components::transform::Transform"
///         ]
///     }
/// }
/// ```
/// Or, to list all the fully-qualified type paths in **your** project, pass Null to the
/// `params`.
/// ```json
/// {
///    "jsonrpc": "2.0",
///    "method": "world.list_components",
///    "id": 0,
///    "params": null
///}
///```
///
/// In Rust:
/// ```ignore
///    let req = BrpRequest {
///         jsonrpc: "2.0".to_string(),
///         method: BRP_LIST_METHOD.to_string(), // All the methods have consts
///         id: Some(ureq::json!(0)),
///         params: None,
///     };
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

    /// Resource was not present in the world.
    #[must_use]
    pub fn resource_not_present(resource: &str) -> Self {
        Self {
            code: error_codes::RESOURCE_NOT_PRESENT,
            message: format!("Resource `{resource}` not present in the world"),
            data: None,
        }
    }

    /// An arbitrary resource error. Possibly related to reflection.
    #[must_use]
    pub fn resource_error<E: ToString>(error: E) -> Self {
        Self {
            code: error_codes::RESOURCE_ERROR,
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

    /// Could not reflect or find resource.
    pub const RESOURCE_ERROR: i16 = -23501;

    /// Could not find resource in the world.
    pub const RESOURCE_NOT_PRESENT: i16 = -23502;
}

/// The result of a request.
pub type BrpResult<T = Value> = Result<T, BrpError>;

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
        let Some(&handler) = world.resource::<RemoteMethods>().get(&message.method) else {
            let _ = message.sender.force_send(Err(BrpError {
                code: error_codes::METHOD_NOT_FOUND,
                message: format!("Method `{}` not found", message.method),
                data: None,
            }));
            return;
        };

        match handler {
            RemoteMethodSystemId::Instant(id) => {
                let result = match world.run_system_with(id, message.params) {
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
            RemoteMethodSystemId::Watching(id) => {
                world
                    .resource_mut::<RemoteWatchingRequests>()
                    .0
                    .push((message, id));
            }
        }
    }
}

/// A system that checks all ongoing watching requests for changes that should be sent
/// and handles it if so.
fn process_ongoing_watching_requests(world: &mut World) {
    world.resource_scope::<RemoteWatchingRequests, ()>(|world, requests| {
        for (message, system_id) in requests.0.iter() {
            let handler_result = process_single_ongoing_watching_request(world, message, system_id);
            let sender_result = match handler_result {
                Ok(Some(value)) => message.sender.try_send(Ok(value)),
                Err(err) => message.sender.try_send(Err(err)),
                Ok(None) => continue,
            };

            if sender_result.is_err() {
                // The [`remove_closed_watching_requests`] system will clean this up.
                message.sender.close();
            }
        }
    });
}

fn process_single_ongoing_watching_request(
    world: &mut World,
    message: &BrpMessage,
    system_id: &RemoteWatchingMethodSystemId,
) -> BrpResult<Option<Value>> {
    world
        .run_system_with(*system_id, message.params.clone())
        .map_err(|error| BrpError {
            code: error_codes::INTERNAL_ERROR,
            message: format!("Failed to run method handler: {error}"),
            data: None,
        })?
}

fn remove_closed_watching_requests(mut requests: ResMut<RemoteWatchingRequests>) {
    for i in (0..requests.0.len()).rev() {
        let Some((message, _)) = requests.0.get(i) else {
            unreachable!()
        };

        if message.sender.is_closed() {
            requests.0.swap_remove(i);
        }
    }
}

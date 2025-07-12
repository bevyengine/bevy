//! Remote command handling module.
use std::{any::TypeId, borrow::Cow};

use bevy_app::{App, PreStartup};
use bevy_ecs::{
    system::{Command, Commands, In, IntoSystem, ResMut},
    world::World,
};
use serde::{de::DeserializeOwned, Serialize};
use serde_json::Value;

use crate::{BrpError, BrpResult, CommandTypeInfo, RemoteMethodHandler, RemoteMethods};

/// Remote command handling module.
pub struct RpcCommand {
    /// The path of the command.
    pub path: Cow<'static, str>,
    /// command input
    pub input: Option<Value>,
}

impl RpcCommand {
    /// Create a new RPC command with the given path.
    pub fn new(path: impl Into<Cow<'static, str>>) -> RpcCommand {
        RpcCommand {
            path: path.into(),
            input: None,
        }
    }

    /// Set the input for the RPC command.
    pub fn with_input(&mut self, input: Value) -> &mut Self {
        self.input = Some(input);
        self
    }
}

impl Command for RpcCommand {
    fn apply(self, world: &mut World) {
        let Some(remote_id) = world
            .get_resource::<RemoteMethods>()
            .and_then(|e| e.get(&self.path))
        else {
            return;
        };
        match remote_id {
            crate::RemoteMethodSystemId::Instant(system_id, ..) => {
                let output = world.run_system_with(*system_id, self.input);
                if let Ok(Ok(value)) = output {
                    bevy_log::info!("{}", serde_json::to_string_pretty(&value).expect(""));
                }
            }
            crate::RemoteMethodSystemId::Watching(system_id, ..) => {
                let _ = world.run_system_with(*system_id, self.input);
            }
        }
    }
}

/// Parses the input parameters for the command.
fn parse_input<T: Serialize + DeserializeOwned>(
    params: Option<Value>,
) -> Result<Option<T>, BrpError> {
    let command_input = match params {
        Some(json_value) => {
            match serde_json::from_value::<T>(json_value).map_err(BrpError::invalid_input) {
                Ok(v) => Some(v),
                Err(e) => return Err(e),
            }
        }
        None => None,
    };
    Ok(command_input)
}

/// Helper trait for creating RPC commands.
pub trait RemoteCommand: bevy_reflect::GetTypeRegistration + Sized {
    /// Type of the input parameter for the command.
    type ParameterType: Serialize + DeserializeOwned + bevy_reflect::GetTypeRegistration;
    /// Type of the response for the command.
    type ResponseType: Serialize + DeserializeOwned + bevy_reflect::GetTypeRegistration;
    /// Path of the command.
    const RPC_PATH: &str;

    /// Returns the input parameter for the command.
    fn input_or_err(input: Option<Self::ParameterType>) -> Result<Self::ParameterType, BrpError> {
        input.ok_or(BrpError::missing_input())
    }

    /// Builds the command with the given input.
    fn to_command(input: Option<Self::ParameterType>) -> RpcCommand {
        RpcCommand {
            path: Self::RPC_PATH.into(),
            input: serde_json::to_value(input).ok(),
        }
    }

    /// Builds the command with no input.
    fn no_input() -> RpcCommand {
        RpcCommand {
            path: Self::RPC_PATH.into(),
            input: None,
        }
    }
}

/// Returns the type information for the command.
pub(crate) fn get_command_type_info<T: RemoteCommand>() -> CommandTypeInfo {
    CommandTypeInfo {
        command_type: T::get_type_registration().type_id(),
        arg_type: TypeId::of::<T::ParameterType>(),
        response_type: TypeId::of::<T::ResponseType>(),
    }
}
/// Trait for remote commands that execute instantly and return a response.
pub trait RemoteCommandInstant: RemoteCommand {
    /// Returns the method handler for this instant remote command.
    fn get_method_handler() -> RemoteMethodHandler {
        RemoteMethodHandler::Instant(
            Box::new(IntoSystem::into_system(command_system::<Self>)),
            Some(get_command_type_info::<Self>()),
        )
    }
    /// Implementation of the command method that processes input and returns a response.
    fn method_impl(
        input: Option<Self::ParameterType>,
        world: &mut World,
    ) -> Result<Self::ResponseType, BrpError>;
}

fn command_system<T: RemoteCommandInstant>(
    In(params): In<Option<Value>>,
    world: &mut World,
) -> BrpResult {
    let command_input = parse_input::<T::ParameterType>(params)?;

    match T::method_impl(command_input, world) {
        Ok(v) => match serde_json::to_value(v) {
            Ok(value) => Ok(value),
            Err(e) => Err(BrpError::internal(e)),
        },
        Err(e) => Err(e),
    }
}
/// Trait for remote commands that execute continuously and may return optional responses.
pub trait RemoteCommandWatching: RemoteCommand {
    /// Returns the method handler for this watching remote command.
    fn get_method_handler() -> RemoteMethodHandler {
        RemoteMethodHandler::Watching(
            Box::new(IntoSystem::into_system(watching_command_system::<Self>)),
            Some(get_command_type_info::<Self>()),
        )
    }
    /// Implementation of the command method that processes input and returns an optional response.
    fn method_impl(
        input: Option<Self::ParameterType>,
        world: &mut World,
    ) -> Result<Option<Self::ResponseType>, BrpError>;
}

fn watching_command_system<T: RemoteCommandWatching>(
    In(params): In<Option<Value>>,
    world: &mut World,
) -> BrpResult<Option<Value>> {
    let command_input = parse_input::<T::ParameterType>(params)?;
    let command_output = T::method_impl(command_input, world)?;
    match command_output {
        Some(v) => {
            let value = serde_json::to_value(v).map_err(BrpError::internal)?;
            Ok(Some(value))
        }
        None => Ok(None),
    }
}
fn add_remote_command<T: RemoteCommandInstant>(
    mut methods: ResMut<RemoteMethods>,
    mut commands: Commands,
) {
    let system_id = commands.register_system(command_system::<T>);
    methods.add_method::<T>(system_id);
}

fn add_remote_watching_command<T: RemoteCommandWatching>(
    mut methods: ResMut<RemoteMethods>,
    mut commands: Commands,
) {
    let system_id = commands.register_system(watching_command_system::<T>);
    methods.add_watching_method::<T>(system_id);
}
/// Extension trait for adding remote command methods to the Bevy App.
pub trait RemoteCommandAppExt {
    /// Registers a remote method.
    fn add_remote_method<T: RemoteCommandInstant>(&mut self) -> &mut Self;
    /// Registers a remote method that can return multiple values.
    fn add_remote_watching_method<T: RemoteCommandWatching>(&mut self) -> &mut Self;
    /// Registers the types associated with a remote command for reflection.
    fn register_method_types<T: RemoteCommand>(&mut self) -> &mut Self;

    /// Registers a remote method that can return value once.
    fn register_untyped_method<M>(
        &mut self,
        name: impl Into<String>,
        handler: impl IntoSystem<In<Option<Value>>, BrpResult, M>,
    ) -> &mut Self;
    /// Registers a remote method that can return values multiple times.
    fn register_untyped_watching_method<M>(
        &mut self,
        name: impl Into<String>,
        handler: impl IntoSystem<In<Option<Value>>, BrpResult<Option<Value>>, M>,
    ) -> &mut Self;
}

impl RemoteCommandAppExt for App {
    fn add_remote_method<T: RemoteCommandInstant>(&mut self) -> &mut Self {
        self.register_method_types::<T>()
            .add_systems(PreStartup, add_remote_command::<T>)
    }
    fn add_remote_watching_method<T: RemoteCommandWatching>(&mut self) -> &mut Self {
        self.register_method_types::<T>()
            .add_systems(PreStartup, add_remote_watching_command::<T>)
    }

    fn register_method_types<T: RemoteCommand>(&mut self) -> &mut Self {
        self.register_type::<T>()
            .register_type::<T::ParameterType>()
            .register_type::<T::ResponseType>()
    }

    fn register_untyped_method<M>(
        &mut self,
        name: impl Into<String>,
        handler: impl IntoSystem<In<Option<Value>>, BrpResult, M>,
    ) -> &mut Self {
        let remote_handler = crate::RemoteMethodSystemId::Instant(
            self.main_mut()
                .world_mut()
                .register_boxed_system(Box::new(IntoSystem::into_system(handler))),
            None,
        );
        let name = name.into();
        self.main_mut()
            .world_mut()
            .get_resource_mut::<RemoteMethods>()
            .unwrap()
            .insert(name, remote_handler);
        self
    }

    fn register_untyped_watching_method<M>(
        &mut self,
        name: impl Into<String>,
        handler: impl IntoSystem<In<Option<Value>>, BrpResult<Option<Value>>, M>,
    ) -> &mut Self {
        let remote_handler = crate::RemoteMethodSystemId::Watching(
            self.main_mut()
                .world_mut()
                .register_boxed_system(Box::new(IntoSystem::into_system(handler))),
            None,
        );
        let name = name.into();
        self.main_mut()
            .world_mut()
            .get_resource_mut::<RemoteMethods>()
            .unwrap()
            .insert(name, remote_handler);
        self
    }
}

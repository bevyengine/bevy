use std::sync::Arc;

use bevy_ecs::{system::Commands, world::Command};
use bevy_log::error;
use bevy_reflect::{FromReflect, FromType, Reflect, Typed};

/// DevCommands are commands which speed up the development process
/// and are not intended to be used in production.
/// It can be used to enable or disable dev tools,
/// enter god mode, fly camera, pause game, change resources, spawn entities, etc.
pub trait DevCommand : Command + FromReflect + Reflect + Typed {
    /// The metadata of the dev command
    fn metadata() -> DevCommandMetadata {
        DevCommandMetadata {
            self_to_commands: Arc::new(|reflected_self, commands| {
                let Some(typed_self) = <Self as FromReflect>::from_reflect(reflected_self) else {
                    error!("Can not construct self from reflect");
                    return;
                };
                commands.add(typed_self);
            })
        }
    }
}



/// Metadata of the dev command
/// Contains method to add reflected clone of self to Commands struct
#[derive(Clone)]
pub struct DevCommandMetadata {
    /// Method to add reflected clone of self to Commands
    pub self_to_commands: Arc<dyn Fn(&dyn Reflect, &mut Commands) + Send + Sync>
}

/// Auto register dev command metadata in TypeRegistry
/// Must use #[reflect(DevCommand)] for auto registration
#[derive(Clone)]
pub struct ReflectDevCommand {
    /// Metadata
    pub metadata: DevCommandMetadata
}

impl<T: DevCommand> FromType<T> for ReflectDevCommand {
    fn from_type() -> Self {
        ReflectDevCommand {
            metadata: T::metadata()
        }
    }
}
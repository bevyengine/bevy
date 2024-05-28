use std::sync::Arc;

use bevy_ecs::{system::Commands, world::{Command, CommandQueue, FromWorld}};
use bevy_reflect::{reflect_trait, FromReflect, FromType, GetTypeRegistration, Reflect, TypeData};

pub trait DevCommand : Command + FromReflect + Reflect {
    fn metadata() -> DevCommandMetadata {
        DevCommandMetadata {
            self_to_commands: Arc::new(|reflected_self, commands| {
                commands.add(<Self as FromReflect>::from_reflect(reflected_self).unwrap());
            })
        }
    }
}



#[derive(Clone)]
pub struct ReflectDevCommand {
    pub metadata: DevCommandMetadata
}

impl<T: DevCommand> FromType<T> for ReflectDevCommand {
    fn from_type() -> Self {
        ReflectDevCommand {
            metadata: T::metadata()
        }
    }
}

#[derive(Clone)]
pub struct DevCommandMetadata {
    pub self_to_commands: Arc<dyn Fn(&dyn Reflect, &mut Commands) + Send + Sync>
}
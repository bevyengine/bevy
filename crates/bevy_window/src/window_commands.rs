use bevy_ecs::{
    entity::Entity,
    event::Events,
    prelude::World,
    system::{Command, Commands},
};
use bevy_math::{DVec2, IVec2, Vec2};

use crate::{
    window, CreateWindow, CursorIcon, PresentMode, RawWindowHandleWrapper, Window,
    WindowDescriptor, WindowMode, WindowResizeConstraints,
};

// TODO: Docs
pub trait WindowCommandsExtension<'w, 's> {
    // TODO: Docs
    fn window<'a>(&'a mut self, entity: Entity) -> WindowCommands<'w, 's, 'a>;
    // TODO: Docs
    fn spawn_window<'a>(&'a mut self, descriptor: WindowDescriptor) -> WindowCommands<'w, 's, 'a>;
}

impl<'w, 's> WindowCommandsExtension<'w, 's> for Commands<'w, 's> {
    // TODO: Docs
    /// Gives you windowcommands for an entity
    fn window<'a>(&'a mut self, entity: Entity) -> WindowCommands<'w, 's, 'a> {
        assert!(
            self.has_entity(entity),
            "Attempting to create an WindowCommands for entity {:?}, which doesn't exist.",
            entity
        );

        WindowCommands {
            entity,
            commands: self,
        }
    }

    // TODO: Docs
    /// Spawns and entity, then gives you window-commands for that entity
    fn spawn_window<'a>(&'a mut self, descriptor: WindowDescriptor) -> WindowCommands<'w, 's, 'a> {
        let entity = self.spawn().id();

        self.add(CreateWindowCommand { entity, descriptor });

        WindowCommands {
            entity,
            commands: self,
        }
    }
}

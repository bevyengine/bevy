use bevy_asset::Handle;
use bevy_ecs::{Command, Commands, Entity, Resources, World};
use bevy_transform::hierarchy::ChildBuilder;

use crate::{Scene, SceneSpawner};

pub struct SpawnScene {
    scene_handle: Handle<Scene>,
}

impl Command for SpawnScene {
    fn write(self: Box<Self>, _world: &mut World, resources: &mut Resources) {
        let mut spawner = resources.get_mut::<SceneSpawner>().unwrap();
        spawner.spawn(self.scene_handle);
    }
}

pub trait SpawnSceneCommands {
    fn spawn_scene(&mut self, scene: Handle<Scene>) -> &mut Self;
}

impl SpawnSceneCommands for Commands {
    fn spawn_scene(&mut self, scene_handle: Handle<Scene>) -> &mut Self {
        self.add_command(SpawnScene { scene_handle })
    }
}

pub struct SpawnSceneAsChild {
    scene_handle: Handle<Scene>,
    parent: Entity,
}

impl Command for SpawnSceneAsChild {
    fn write(self: Box<Self>, _world: &mut World, resources: &mut Resources) {
        let mut spawner = resources.get_mut::<SceneSpawner>().unwrap();
        spawner.spawn_as_child(self.scene_handle, self.parent);
    }
}

pub trait SpawnSceneAsChildCommands {
    fn spawn_scene(&mut self, scene: Handle<Scene>) -> &mut Self;
}

impl<'a> SpawnSceneAsChildCommands for ChildBuilder<'a> {
    fn spawn_scene(&mut self, scene_handle: Handle<Scene>) -> &mut Self {
        self.add_command(SpawnSceneAsChild {
            scene_handle,
            parent: self.parent_entity(),
        });
        self
    }
}

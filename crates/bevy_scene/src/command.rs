use bevy_asset::Handle;
use bevy_ecs::{
    entity::Entity,
    system::{Command, Commands},
    world::World,
};
use bevy_transform::hierarchy::ChildBuilder;

use crate::{Scene, SceneSpawner};

pub struct SpawnScene {
    scene_handle: Handle<Scene>,
}

impl Command for SpawnScene {
    fn write(self, world: &mut World) {
        let mut spawner = world.get_resource_mut::<SceneSpawner>().unwrap();
        spawner.spawn(self.scene_handle);
    }
}

pub trait SpawnSceneCommands {
    fn spawn_scene(&mut self, scene: Handle<Scene>);
}

impl<'w, 's> SpawnSceneCommands for Commands<'w, 's> {
    fn spawn_scene(&mut self, scene_handle: Handle<Scene>) {
        self.add(SpawnScene { scene_handle });
    }
}

pub struct SpawnSceneAsChild {
    scene_handle: Handle<Scene>,
    parent: Entity,
}

impl Command for SpawnSceneAsChild {
    fn write(self, world: &mut World) {
        let mut spawner = world.get_resource_mut::<SceneSpawner>().unwrap();
        spawner.spawn_as_child(self.scene_handle, self.parent);
    }
}

pub trait SpawnSceneAsChildCommands {
    fn spawn_scene(&mut self, scene: Handle<Scene>) -> &mut Self;
}

impl<'w, 's, 'a> SpawnSceneAsChildCommands for ChildBuilder<'w, 's, 'a> {
    fn spawn_scene(&mut self, scene_handle: Handle<Scene>) -> &mut Self {
        self.add_command(SpawnSceneAsChild {
            scene_handle,
            parent: self.parent_entity(),
        });
        self
    }
}

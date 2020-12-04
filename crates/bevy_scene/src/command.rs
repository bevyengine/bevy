use bevy_asset::Handle;
use bevy_ecs::{Command, Commands, Resources, World};

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

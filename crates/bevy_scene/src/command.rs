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
    fn write(self: Box<Self>, world: &mut World) {
        let mut spawner = world.get_resource_mut::<SceneSpawner>().unwrap();
        spawner.spawn(self.scene_handle);
    }
}

// Written in "one line" to insert it into the module overview.
#[doc = "[`Commands`] extension: <ul>\
<li><code>.[spawn_scene](SpawnSceneCommands::spawn_scene)(scene: [Handle]<[Scene]>)</code></li>\
</ul>"]
pub trait SpawnSceneCommands {
    /// Adds a new [`SpawnScene`] instance to this command list.
    fn spawn_scene(&mut self, scene: Handle<Scene>);
}

impl<'a> SpawnSceneCommands for Commands<'a> {
    fn spawn_scene(&mut self, scene_handle: Handle<Scene>) {
        self.add(SpawnScene { scene_handle });
    }
}

pub struct SpawnSceneAsChild {
    scene_handle: Handle<Scene>,
    parent: Entity,
}

impl Command for SpawnSceneAsChild {
    fn write(self: Box<Self>, world: &mut World) {
        let mut spawner = world.get_resource_mut::<SceneSpawner>().unwrap();
        spawner.spawn_as_child(self.scene_handle, self.parent);
    }
}

// Written in "one line" to insert it into the module overview.
#[doc = "[`ChildBuilder`] extension: <ul>\
<li><code>.[spawn_scene](SpawnSceneAsChildCommands::spawn_scene)(scene: [Handle]<[Scene]>)</code></li>\
</ul>"]
pub trait SpawnSceneAsChildCommands {
    /// Adds a new [`SpawnSceneAsChild`] instance to this command list.
    fn spawn_scene(&mut self, scene: Handle<Scene>) -> &mut Self;
}

impl<'a, 'b> SpawnSceneAsChildCommands for ChildBuilder<'a, 'b> {
    fn spawn_scene(&mut self, scene_handle: Handle<Scene>) -> &mut Self {
        self.add_command(SpawnSceneAsChild {
            scene_handle,
            parent: self.parent_entity(),
        });
        self
    }
}

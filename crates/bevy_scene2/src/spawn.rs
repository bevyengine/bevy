use crate::{ResolvedScene, Scene, ScenePatch, ScenePatchInstance};
use bevy_asset::{AssetEvent, AssetId, AssetServer, Assets};
use bevy_ecs::{event::EventCursor, prelude::*};
use bevy_platform::collections::HashMap;

pub trait SpawnScene {
    fn spawn_scene<S: Scene>(&mut self, scene: S) -> EntityWorldMut;
}

impl SpawnScene for World {
    fn spawn_scene<S: Scene>(&mut self, scene: S) -> EntityWorldMut {
        let assets = self.resource::<AssetServer>();
        let patch = ScenePatch::load(assets, scene);
        let handle = assets.add(patch);
        self.spawn(ScenePatchInstance(handle))
    }
}

pub trait CommandsSpawnScene {
    fn spawn_scene<S: Scene>(&mut self, scene: S) -> EntityCommands;
}

impl<'w, 's> CommandsSpawnScene for Commands<'w, 's> {
    fn spawn_scene<S: Scene>(&mut self, scene: S) -> EntityCommands {
        let mut entity_commands = self.spawn_empty();
        let id = entity_commands.id();
        entity_commands.commands().queue(move |world: &mut World| {
            let assets = world.resource::<AssetServer>();
            let patch = ScenePatch::load(assets, scene);
            let handle = assets.add(patch);
            if let Ok(mut entity) = world.get_entity_mut(id) {
                entity.insert(ScenePatchInstance(handle));
            }
        });
        entity_commands
    }
}

pub fn resolve_scene_patches(
    mut events: EventReader<AssetEvent<ScenePatch>>,
    assets: Res<AssetServer>,
    mut patches: ResMut<Assets<ScenePatch>>,
) {
    for event in events.read() {
        match *event {
            // TODO: handle modified?
            AssetEvent::LoadedWithDependencies { id } => {
                let mut scene = ResolvedScene::default();
                // TODO: real error handling
                let patch = patches.get(id).unwrap();
                patch.patch.patch(&assets, &patches, &mut scene);
                let patch = patches.get_mut(id).unwrap();
                patch.resolved = Some(scene)
            }
            _ => {}
        }
    }
}

#[derive(Resource, Default)]
pub struct QueuedScenes {
    waiting_entities: HashMap<AssetId<ScenePatch>, Vec<Entity>>,
}

pub fn spawn_queued(
    world: &mut World,
    handles: &mut QueryState<(Entity, &ScenePatchInstance), Added<ScenePatchInstance>>,
    mut reader: Local<EventCursor<AssetEvent<ScenePatch>>>,
) {
    world.resource_scope(|world, mut patches: Mut<Assets<ScenePatch>>| {
        world.resource_scope(|world, mut queued: Mut<QueuedScenes>| {
            world.resource_scope(|world, events: Mut<Events<AssetEvent<ScenePatch>>>| {
                for (entity, id) in handles
                    .iter(world)
                    .map(|(e, h)| (e, h.id()))
                    .collect::<Vec<_>>()
                {
                    if let Some(scene) = patches.get_mut(id).and_then(|p| p.resolved.as_mut()) {
                        let mut entity_mut = world.get_entity_mut(entity).unwrap();
                        scene.spawn(&mut entity_mut).unwrap();
                    } else {
                        let entities = queued.waiting_entities.entry(id).or_default();
                        entities.push(entity);
                    }
                }

                for event in reader.read(&events) {
                    if let AssetEvent::LoadedWithDependencies { id } = event {
                        let Some(scene) = patches.get_mut(*id).and_then(|p| p.resolved.as_mut())
                        else {
                            continue;
                        };

                        let Some(entities) = queued.waiting_entities.remove(id) else {
                            continue;
                        };

                        for entity in entities {
                            let mut entity_mut = world.get_entity_mut(entity).unwrap();
                            scene.spawn(&mut entity_mut).unwrap();
                        }
                    }
                }
            });
        });
    });
}

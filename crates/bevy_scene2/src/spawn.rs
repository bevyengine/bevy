use crate::{ResolvedScene, Scene, ScenePatch, ScenePatchInstance};
use bevy_asset::{AssetEvent, AssetId, AssetServer, Assets};
use bevy_ecs::{message::MessageCursor, prelude::*};
use bevy_platform::collections::HashMap;

pub trait SpawnScene {
    fn spawn_scene<S: Scene>(&mut self, scene: S) -> EntityWorldMut<'_>;
}

impl SpawnScene for World {
    fn spawn_scene<S: Scene>(&mut self, scene: S) -> EntityWorldMut<'_> {
        let assets = self.resource::<AssetServer>();
        let patch = ScenePatch::load(assets, scene);
        let handle = assets.add(patch);
        self.spawn(ScenePatchInstance(handle))
    }
}

pub trait CommandsSpawnScene {
    fn spawn_scene<S: Scene>(&mut self, scene: S) -> EntityCommands<'_>;
}

impl<'w, 's> CommandsSpawnScene for Commands<'w, 's> {
    fn spawn_scene<S: Scene>(&mut self, scene: S) -> EntityCommands<'_> {
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
    mut events: MessageReader<AssetEvent<ScenePatch>>,
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

#[derive(Resource, Default)]
pub struct NewScenes {
    entities: Vec<Entity>,
}

pub fn on_add_scene_patch_instance(
    add: On<Add, ScenePatchInstance>,
    mut new_scenes: ResMut<NewScenes>,
) {
    new_scenes.entities.push(add.entity);
}

pub fn spawn_queued(
    world: &mut World,
    handles: &mut QueryState<&ScenePatchInstance>,
    mut reader: Local<MessageCursor<AssetEvent<ScenePatch>>>,
) {
    world.resource_scope(|world, mut patches: Mut<Assets<ScenePatch>>| {
        world.resource_scope(|world, mut queued: Mut<QueuedScenes>| {
            world.resource_scope(|world, events: Mut<Messages<AssetEvent<ScenePatch>>>| {
                loop {
                    let mut new_scenes = world.resource_mut::<NewScenes>();
                    if new_scenes.entities.is_empty() {
                        break;
                    }
                    for entity in core::mem::take(&mut new_scenes.entities) {
                        if let Ok(id) = handles.get(world, entity).map(|h| h.id()) {
                            if let Some(scene) =
                                patches.get_mut(id).and_then(|p| p.resolved.as_mut())
                            {
                                let mut entity_mut = world.get_entity_mut(entity).unwrap();
                                scene.spawn(&mut entity_mut).unwrap();
                            } else {
                                let entities = queued.waiting_entities.entry(id).or_default();
                                entities.push(entity);
                            }
                        }
                    }
                }

                for event in reader.read(&events) {
                    if let AssetEvent::LoadedWithDependencies { id } = event
                        && let Some(scene) = patches.get_mut(*id).and_then(|p| p.resolved.as_mut())
                        && let Some(entities) = queued.waiting_entities.remove(id)
                    {
                        for entity in entities {
                            if let Ok(mut entity_mut) = world.get_entity_mut(entity) {
                                scene.spawn(&mut entity_mut).unwrap();
                            }
                        }
                    }
                }
            });
        });
    });
}

use crate::{DynamicScene, Scene};
use bevy_asset::{AssetEvent, AssetId, Assets, Handle};
use bevy_ecs::{
    entity::{Entity, EntityHashMap},
    event::{Event, EventCursor, Events},
    reflect::AppTypeRegistry,
    system::Resource,
    world::{Command, Mut, World},
};
use bevy_hierarchy::{AddChild, BuildChildren, DespawnRecursiveExt, Parent};
use bevy_utils::{HashMap, HashSet};
use derive_more::derive::{Display, Error};
use uuid::Uuid;

/// Triggered on a scene's parent entity when [`crate::SceneInstance`] becomes ready to use.
///
/// See also [`Trigger`], [`SceneSpawner::instance_is_ready`].
///
/// [`Trigger`]: bevy_ecs::observer::Trigger
#[derive(Clone, Copy, Debug, Eq, PartialEq, Event)]
pub struct SceneInstanceReady {
    /// Instance which has been spawned.
    pub instance_id: InstanceId,
}

/// Information about a scene instance.
#[derive(Debug)]
pub struct InstanceInfo {
    /// Mapping of entities from the scene world to the instance world.
    pub entity_map: EntityHashMap<Entity>,
}

/// Unique id identifying a scene instance.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct InstanceId(Uuid);

impl InstanceId {
    fn new() -> Self {
        InstanceId(Uuid::new_v4())
    }
}

/// Handles spawning and despawning scenes in the world, either synchronously or batched through the [`scene_spawner_system`].
///
/// Synchronous methods: (Scene operations will take effect immediately)
/// - [`spawn_dynamic_sync`](Self::spawn_dynamic_sync)
/// - [`spawn_sync`](Self::spawn_sync)
/// - [`despawn_sync`](Self::despawn_sync)
/// - [`despawn_instance_sync`](Self::despawn_instance_sync)
/// - [`update_spawned_scenes`](Self::update_spawned_scenes)
/// - [`spawn_queued_scenes`](Self::spawn_queued_scenes)
/// - [`despawn_queued_scenes`](Self::despawn_queued_scenes)
/// - [`despawn_queued_instances`](Self::despawn_queued_instances)
///
/// Deferred methods: (Scene operations will be processed when the [`scene_spawner_system`] is run)
/// - [`spawn_dynamic`](Self::spawn_dynamic)
/// - [`spawn_dynamic_as_child`](Self::spawn_dynamic_as_child)
/// - [`spawn`](Self::spawn)
/// - [`spawn_as_child`](Self::spawn_as_child)
/// - [`despawn`](Self::despawn)
/// - [`despawn_instance`](Self::despawn_instance)
#[derive(Default, Resource)]
pub struct SceneSpawner {
    pub(crate) spawned_dynamic_scenes: HashMap<AssetId<DynamicScene>, HashSet<InstanceId>>,
    pub(crate) spawned_instances: HashMap<InstanceId, InstanceInfo>,
    scene_asset_event_reader: EventCursor<AssetEvent<DynamicScene>>,
    dynamic_scenes_to_spawn: Vec<(Handle<DynamicScene>, InstanceId, Option<Entity>)>,
    scenes_to_spawn: Vec<(Handle<Scene>, InstanceId, Option<Entity>)>,
    scenes_to_despawn: Vec<AssetId<DynamicScene>>,
    instances_to_despawn: Vec<InstanceId>,
    scenes_with_parent: Vec<(InstanceId, Entity)>,
}

/// Errors that can occur when spawning a scene.
#[derive(Error, Display, Debug)]
pub enum SceneSpawnError {
    /// Scene contains an unregistered component type.
    #[display("scene contains the unregistered component `{type_path}`. consider adding `#[reflect(Component)]` to your type")]
    UnregisteredComponent {
        /// Type of the unregistered component.
        type_path: String,
    },
    /// Scene contains an unregistered resource type.
    #[display("scene contains the unregistered resource `{type_path}`. consider adding `#[reflect(Resource)]` to your type")]
    UnregisteredResource {
        /// Type of the unregistered resource.
        type_path: String,
    },
    /// Scene contains an unregistered type.
    #[display(
        "scene contains the unregistered type `{std_type_name}`. \
        consider reflecting it with `#[derive(Reflect)]` \
        and registering the type using `app.register_type::<T>()`"
    )]
    UnregisteredType {
        /// The [type name](std::any::type_name) for the unregistered type.
        std_type_name: String,
    },
    /// Scene contains an unregistered type which has a `TypePath`.
    #[display(
        "scene contains the reflected type `{type_path}` but it was not found in the type registry. \
        consider registering the type using `app.register_type::<T>()``"
    )]
    UnregisteredButReflectedType {
        /// The unregistered type.
        type_path: String,
    },
    /// Scene contains a proxy without a represented type.
    #[display("scene contains dynamic type `{type_path}` without a represented type. consider changing this using `set_represented_type`.")]
    NoRepresentedType {
        /// The dynamic instance type.
        type_path: String,
    },
    /// Dynamic scene with the given id does not exist.
    #[display("scene does not exist")]
    NonExistentScene {
        /// Id of the non-existent dynamic scene.
        id: AssetId<DynamicScene>,
    },
    /// Scene with the given id does not exist.
    #[display("scene does not exist")]
    NonExistentRealScene {
        /// Id of the non-existent scene.
        id: AssetId<Scene>,
    },
}

impl SceneSpawner {
    /// Schedule the spawn of a new instance of the provided dynamic scene.
    pub fn spawn_dynamic(&mut self, id: impl Into<Handle<DynamicScene>>) -> InstanceId {
        let instance_id = InstanceId::new();
        self.dynamic_scenes_to_spawn
            .push((id.into(), instance_id, None));
        instance_id
    }

    /// Schedule the spawn of a new instance of the provided dynamic scene as a child of `parent`.
    pub fn spawn_dynamic_as_child(
        &mut self,
        id: impl Into<Handle<DynamicScene>>,
        parent: Entity,
    ) -> InstanceId {
        let instance_id = InstanceId::new();
        self.dynamic_scenes_to_spawn
            .push((id.into(), instance_id, Some(parent)));
        self.scenes_with_parent.push((instance_id, parent));
        instance_id
    }

    /// Schedule the spawn of a new instance of the provided scene.
    pub fn spawn(&mut self, id: impl Into<Handle<Scene>>) -> InstanceId {
        let instance_id = InstanceId::new();
        self.scenes_to_spawn.push((id.into(), instance_id, None));
        instance_id
    }

    /// Schedule the spawn of a new instance of the provided scene as a child of `parent`.
    pub fn spawn_as_child(&mut self, id: impl Into<Handle<Scene>>, parent: Entity) -> InstanceId {
        let instance_id = InstanceId::new();
        self.scenes_to_spawn
            .push((id.into(), instance_id, Some(parent)));
        self.scenes_with_parent.push((instance_id, parent));
        instance_id
    }

    /// Schedule the despawn of all instances of the provided dynamic scene.
    pub fn despawn(&mut self, id: impl Into<AssetId<DynamicScene>>) {
        self.scenes_to_despawn.push(id.into());
    }

    /// Schedule the despawn of a scene instance, removing all its entities from the world.
    pub fn despawn_instance(&mut self, instance_id: InstanceId) {
        self.instances_to_despawn.push(instance_id);
    }

    /// Immediately despawns all instances of a dynamic scene.
    pub fn despawn_sync(
        &mut self,
        world: &mut World,
        id: impl Into<AssetId<DynamicScene>>,
    ) -> Result<(), SceneSpawnError> {
        if let Some(instance_ids) = self.spawned_dynamic_scenes.remove(&id.into()) {
            for instance_id in instance_ids {
                self.despawn_instance_sync(world, &instance_id);
            }
        }
        Ok(())
    }

    /// Immediately despawns a scene instance, removing all its entities from the world.
    pub fn despawn_instance_sync(&mut self, world: &mut World, instance_id: &InstanceId) {
        if let Some(instance) = self.spawned_instances.remove(instance_id) {
            for &entity in instance.entity_map.values() {
                if let Ok(mut entity_mut) = world.get_entity_mut(entity) {
                    entity_mut.remove_parent();
                    entity_mut.despawn_recursive();
                };
            }
        }
    }

    /// Immediately spawns a new instance of the provided dynamic scene.
    pub fn spawn_dynamic_sync(
        &mut self,
        world: &mut World,
        id: impl Into<AssetId<DynamicScene>>,
    ) -> Result<InstanceId, SceneSpawnError> {
        let mut entity_map = EntityHashMap::default();
        let id = id.into();
        Self::spawn_dynamic_internal(world, id, &mut entity_map)?;
        let instance_id = InstanceId::new();
        self.spawned_instances
            .insert(instance_id, InstanceInfo { entity_map });
        let spawned = self.spawned_dynamic_scenes.entry(id).or_default();
        spawned.insert(instance_id);
        Ok(instance_id)
    }

    fn spawn_dynamic_internal(
        world: &mut World,
        id: AssetId<DynamicScene>,
        entity_map: &mut EntityHashMap<Entity>,
    ) -> Result<(), SceneSpawnError> {
        world.resource_scope(|world, scenes: Mut<Assets<DynamicScene>>| {
            let scene = scenes
                .get(id)
                .ok_or(SceneSpawnError::NonExistentScene { id })?;

            scene.write_to_world(world, entity_map)
        })
    }

    /// Immediately spawns a new instance of the provided scene.
    pub fn spawn_sync(
        &mut self,
        world: &mut World,
        id: impl Into<AssetId<Scene>>,
    ) -> Result<InstanceId, SceneSpawnError> {
        let mut entity_map = EntityHashMap::default();
        let id = id.into();
        Self::spawn_sync_internal(world, id, &mut entity_map)?;
        let instance_id = InstanceId::new();
        self.spawned_instances
            .insert(instance_id, InstanceInfo { entity_map });
        Ok(instance_id)
    }

    fn spawn_sync_internal(
        world: &mut World,
        id: AssetId<Scene>,
        entity_map: &mut EntityHashMap<Entity>,
    ) -> Result<(), SceneSpawnError> {
        world.resource_scope(|world, scenes: Mut<Assets<Scene>>| {
            let scene = scenes
                .get(id)
                .ok_or(SceneSpawnError::NonExistentRealScene { id })?;

            scene.write_to_world_with(
                world,
                entity_map,
                &world.resource::<AppTypeRegistry>().clone(),
            )
        })
    }

    /// Iterate through all instances of the provided scenes and update those immediately.
    ///
    /// Useful for updating already spawned scene instances after their corresponding scene has been modified.
    pub fn update_spawned_scenes(
        &mut self,
        world: &mut World,
        scene_ids: &[AssetId<DynamicScene>],
    ) -> Result<(), SceneSpawnError> {
        for id in scene_ids {
            if let Some(spawned_instances) = self.spawned_dynamic_scenes.get(id) {
                for instance_id in spawned_instances {
                    if let Some(instance_info) = self.spawned_instances.get_mut(instance_id) {
                        Self::spawn_dynamic_internal(world, *id, &mut instance_info.entity_map)?;
                    }
                }
            }
        }
        Ok(())
    }

    /// Immediately despawns all scenes scheduled for despawn by despawning their instances.
    pub fn despawn_queued_scenes(&mut self, world: &mut World) -> Result<(), SceneSpawnError> {
        let scenes_to_despawn = core::mem::take(&mut self.scenes_to_despawn);

        for scene_handle in scenes_to_despawn {
            self.despawn_sync(world, scene_handle)?;
        }
        Ok(())
    }

    /// Immediately despawns all scene instances scheduled for despawn.
    pub fn despawn_queued_instances(&mut self, world: &mut World) {
        let instances_to_despawn = core::mem::take(&mut self.instances_to_despawn);

        for instance_id in instances_to_despawn {
            self.despawn_instance_sync(world, &instance_id);
        }
    }

    /// Immediately spawns all scenes scheduled for spawn.
    pub fn spawn_queued_scenes(&mut self, world: &mut World) -> Result<(), SceneSpawnError> {
        let scenes_to_spawn = core::mem::take(&mut self.dynamic_scenes_to_spawn);

        for (handle, instance_id, parent) in scenes_to_spawn {
            let mut entity_map = EntityHashMap::default();

            match Self::spawn_dynamic_internal(world, handle.id(), &mut entity_map) {
                Ok(_) => {
                    self.spawned_instances
                        .insert(instance_id, InstanceInfo { entity_map });
                    let spawned = self
                        .spawned_dynamic_scenes
                        .entry(handle.id())
                        .or_insert_with(HashSet::new);
                    spawned.insert(instance_id);

                    // Scenes with parents need more setup before they are ready.
                    // See `set_scene_instance_parent_sync()`.
                    if parent.is_none() {
                        // Defer via commands otherwise SceneSpawner is not available in the observer.
                        world.commands().trigger(SceneInstanceReady { instance_id });
                    }
                }
                Err(SceneSpawnError::NonExistentScene { .. }) => {
                    self.dynamic_scenes_to_spawn
                        .push((handle, instance_id, parent));
                }
                Err(err) => return Err(err),
            }
        }

        let scenes_to_spawn = core::mem::take(&mut self.scenes_to_spawn);

        for (scene_handle, instance_id, parent) in scenes_to_spawn {
            let mut entity_map = EntityHashMap::default();

            match Self::spawn_sync_internal(world, scene_handle.id(), &mut entity_map) {
                Ok(_) => {
                    self.spawned_instances
                        .insert(instance_id, InstanceInfo { entity_map });

                    // Scenes with parents need more setup before they are ready.
                    // See `set_scene_instance_parent_sync()`.
                    if parent.is_none() {
                        // Defer via commands otherwise SceneSpawner is not available in the observer.
                        world.commands().trigger(SceneInstanceReady { instance_id });
                    }
                }
                Err(SceneSpawnError::NonExistentRealScene { .. }) => {
                    self.scenes_to_spawn
                        .push((scene_handle, instance_id, parent));
                }
                Err(err) => return Err(err),
            }
        }

        Ok(())
    }

    pub(crate) fn set_scene_instance_parent_sync(&mut self, world: &mut World) {
        let scenes_with_parent = core::mem::take(&mut self.scenes_with_parent);

        for (instance_id, parent) in scenes_with_parent {
            if let Some(instance) = self.spawned_instances.get(&instance_id) {
                for &entity in instance.entity_map.values() {
                    // Add the `Parent` component to the scene root, and update the `Children` component of
                    // the scene parent
                    if !world
                        .get_entity(entity)
                        // This will filter only the scene root entity, as all other from the
                        // scene have a parent
                        .map(|entity| entity.contains::<Parent>())
                        // Default is true so that it won't run on an entity that wouldn't exist anymore
                        // this case shouldn't happen anyway
                        .unwrap_or(true)
                    {
                        AddChild {
                            parent,
                            child: entity,
                        }
                        .apply(world);
                    }
                }

                // Defer via commands otherwise SceneSpawner is not available in the observer.
                world
                    .commands()
                    .trigger_targets(SceneInstanceReady { instance_id }, parent);
            } else {
                self.scenes_with_parent.push((instance_id, parent));
            }
        }
    }

    /// Check that an scene instance spawned previously is ready to use
    pub fn instance_is_ready(&self, instance_id: InstanceId) -> bool {
        self.spawned_instances.contains_key(&instance_id)
    }

    /// Get an iterator over the entities in an instance, once it's spawned.
    ///
    /// Before the scene is spawned, the iterator will be empty. Use [`Self::instance_is_ready`]
    /// to check if the instance is ready.
    pub fn iter_instance_entities(
        &'_ self,
        instance_id: InstanceId,
    ) -> impl Iterator<Item = Entity> + '_ {
        self.spawned_instances
            .get(&instance_id)
            .map(|instance| instance.entity_map.values())
            .into_iter()
            .flatten()
            .copied()
    }
}

/// System that handles scheduled scene instance spawning and despawning through a [`SceneSpawner`].
pub fn scene_spawner_system(world: &mut World) {
    world.resource_scope(|world, mut scene_spawner: Mut<SceneSpawner>| {
        // remove any loading instances where parent is deleted
        let mut dead_instances = HashSet::default();
        scene_spawner
            .scenes_with_parent
            .retain(|(instance, parent)| {
                let retain = world.get_entity(*parent).is_ok();

                if !retain {
                    dead_instances.insert(*instance);
                }

                retain
            });
        scene_spawner
            .dynamic_scenes_to_spawn
            .retain(|(_, instance, _)| !dead_instances.contains(instance));
        scene_spawner
            .scenes_to_spawn
            .retain(|(_, instance, _)| !dead_instances.contains(instance));

        let scene_asset_events = world.resource::<Events<AssetEvent<DynamicScene>>>();

        let mut updated_spawned_scenes = Vec::new();
        let scene_spawner = &mut *scene_spawner;
        for event in scene_spawner
            .scene_asset_event_reader
            .read(scene_asset_events)
        {
            if let AssetEvent::Modified { id } = event {
                if scene_spawner.spawned_dynamic_scenes.contains_key(id) {
                    updated_spawned_scenes.push(*id);
                }
            }
        }

        scene_spawner.despawn_queued_scenes(world).unwrap();
        scene_spawner.despawn_queued_instances(world);
        scene_spawner
            .spawn_queued_scenes(world)
            .unwrap_or_else(|err| panic!("{}", err));
        scene_spawner
            .update_spawned_scenes(world, &updated_spawned_scenes)
            .unwrap();
        scene_spawner.set_scene_instance_parent_sync(world);
    });
}

#[cfg(test)]
mod tests {
    use bevy_app::App;
    use bevy_asset::{AssetPlugin, AssetServer, Handle};
    use bevy_ecs::{
        component::Component,
        observer::Trigger,
        prelude::ReflectComponent,
        query::With,
        system::{Commands, Query, Res, ResMut, RunSystemOnce},
    };
    use bevy_reflect::Reflect;

    use crate::{DynamicSceneBuilder, DynamicSceneRoot, ScenePlugin};

    use super::*;

    #[derive(Reflect, Component, Debug, PartialEq, Eq, Clone, Copy, Default)]
    #[reflect(Component)]
    struct A(usize);

    #[test]
    fn clone_dynamic_entities() {
        let mut world = World::default();

        // setup
        let atr = AppTypeRegistry::default();
        atr.write().register::<A>();
        world.insert_resource(atr);
        world.insert_resource(Assets::<DynamicScene>::default());

        // start test
        world.spawn(A(42));

        assert_eq!(world.query::<&A>().iter(&world).len(), 1);

        // clone only existing entity
        let mut scene_spawner = SceneSpawner::default();
        let entity = world.query_filtered::<Entity, With<A>>().single(&world);
        let scene = DynamicSceneBuilder::from_world(&world)
            .extract_entity(entity)
            .build();

        let scene_id = world.resource_mut::<Assets<DynamicScene>>().add(scene);
        let instance_id = scene_spawner
            .spawn_dynamic_sync(&mut world, &scene_id)
            .unwrap();

        // verify we spawned exactly one new entity with our expected component
        assert_eq!(world.query::<&A>().iter(&world).len(), 2);

        // verify that we can get this newly-spawned entity by the instance ID
        let new_entity = scene_spawner
            .iter_instance_entities(instance_id)
            .next()
            .unwrap();

        // verify this is not the original entity
        assert_ne!(entity, new_entity);

        // verify this new entity contains the same data as the original entity
        let [old_a, new_a] = world
            .query::<&A>()
            .get_many(&world, [entity, new_entity])
            .unwrap();
        assert_eq!(old_a, new_a);
    }

    #[derive(Component, Reflect, Default)]
    #[reflect(Component)]
    struct ComponentA;

    #[derive(Resource, Default)]
    struct TriggerCount(u32);

    fn setup() -> App {
        let mut app = App::new();
        app.add_plugins((AssetPlugin::default(), ScenePlugin));
        app.init_resource::<TriggerCount>();

        app.register_type::<ComponentA>();
        app.world_mut().spawn(ComponentA);
        app.world_mut().spawn(ComponentA);

        app
    }

    fn build_scene(app: &mut App) -> Handle<Scene> {
        app.world_mut()
            .run_system_once(
                |world: &World,
                 type_registry: Res<'_, AppTypeRegistry>,
                 asset_server: Res<'_, AssetServer>| {
                    asset_server.add(
                        Scene::from_dynamic_scene(&DynamicScene::from_world(world), &type_registry)
                            .unwrap(),
                    )
                },
            )
            .expect("Failed to run scene builder system.")
    }

    fn build_dynamic_scene(app: &mut App) -> Handle<DynamicScene> {
        app.world_mut()
            .run_system_once(|world: &World, asset_server: Res<'_, AssetServer>| {
                asset_server.add(DynamicScene::from_world(world))
            })
            .expect("Failed to run dynamic scene builder system.")
    }

    fn observe_trigger(app: &mut App, scene_id: InstanceId, scene_entity: Entity) {
        // Add observer
        app.world_mut().add_observer(
            move |trigger: Trigger<SceneInstanceReady>,
                  scene_spawner: Res<SceneSpawner>,
                  mut trigger_count: ResMut<TriggerCount>| {
                assert_eq!(
                    trigger.event().instance_id,
                    scene_id,
                    "`SceneInstanceReady` contains the wrong `InstanceId`"
                );
                assert_eq!(
                    trigger.entity(),
                    scene_entity,
                    "`SceneInstanceReady` triggered on the wrong parent entity"
                );
                assert!(
                    scene_spawner.instance_is_ready(trigger.event().instance_id),
                    "`InstanceId` is not ready"
                );
                trigger_count.0 += 1;
            },
        );

        // Check observer is triggered once.
        app.update();
        app.world_mut()
            .run_system_once(|trigger_count: Res<TriggerCount>| {
                assert_eq!(
                    trigger_count.0, 1,
                    "wrong number of `SceneInstanceReady` triggers"
                );
            })
            .unwrap();
    }

    #[test]
    fn observe_scene() {
        let mut app = setup();

        // Build scene.
        let scene = build_scene(&mut app);

        // Spawn scene.
        let scene_id = app
            .world_mut()
            .run_system_once(move |mut scene_spawner: ResMut<'_, SceneSpawner>| {
                scene_spawner.spawn(scene.clone())
            })
            .unwrap();

        // Check trigger.
        observe_trigger(&mut app, scene_id, Entity::PLACEHOLDER);
    }

    #[test]
    fn observe_dynamic_scene() {
        let mut app = setup();

        // Build scene.
        let scene = build_dynamic_scene(&mut app);

        // Spawn scene.
        let scene_id = app
            .world_mut()
            .run_system_once(move |mut scene_spawner: ResMut<'_, SceneSpawner>| {
                scene_spawner.spawn_dynamic(scene.clone())
            })
            .unwrap();

        // Check trigger.
        observe_trigger(&mut app, scene_id, Entity::PLACEHOLDER);
    }

    #[test]
    fn observe_scene_as_child() {
        let mut app = setup();

        // Build scene.
        let scene = build_scene(&mut app);

        // Spawn scene as child.
        let (scene_id, scene_entity) = app
            .world_mut()
            .run_system_once(
                move |mut commands: Commands<'_, '_>,
                      mut scene_spawner: ResMut<'_, SceneSpawner>| {
                    let entity = commands.spawn_empty().id();
                    let id = scene_spawner.spawn_as_child(scene.clone(), entity);
                    (id, entity)
                },
            )
            .unwrap();

        // Check trigger.
        observe_trigger(&mut app, scene_id, scene_entity);
    }

    #[test]
    fn observe_dynamic_scene_as_child() {
        let mut app = setup();

        // Build scene.
        let scene = build_dynamic_scene(&mut app);

        // Spawn scene as child.
        let (scene_id, scene_entity) = app
            .world_mut()
            .run_system_once(
                move |mut commands: Commands<'_, '_>,
                      mut scene_spawner: ResMut<'_, SceneSpawner>| {
                    let entity = commands.spawn_empty().id();
                    let id = scene_spawner.spawn_dynamic_as_child(scene.clone(), entity);
                    (id, entity)
                },
            )
            .unwrap();

        // Check trigger.
        observe_trigger(&mut app, scene_id, scene_entity);
    }

    #[test]
    fn despawn_scene() {
        let mut app = App::new();
        app.add_plugins((AssetPlugin::default(), ScenePlugin));
        app.register_type::<ComponentA>();

        let asset_server = app.world().resource::<AssetServer>();

        // Build scene.
        let scene = asset_server.add(DynamicScene::default());
        let count = 10;

        // Checks the number of scene instances stored in `SceneSpawner`.
        let check = |world: &mut World, expected_count: usize| {
            let scene_spawner = world.resource::<SceneSpawner>();
            assert_eq!(
                scene_spawner.spawned_dynamic_scenes[&scene.id()].len(),
                expected_count
            );
            assert_eq!(scene_spawner.spawned_instances.len(), expected_count);
        };

        // Spawn scene.
        for _ in 0..count {
            app.world_mut()
                .spawn((ComponentA, DynamicSceneRoot(scene.clone())));
        }

        app.update();
        check(app.world_mut(), count);

        // Despawn scene.
        app.world_mut()
            .run_system_once(
                |mut commands: Commands, query: Query<Entity, With<ComponentA>>| {
                    for entity in query.iter() {
                        commands.entity(entity).despawn_recursive();
                    }
                },
            )
            .unwrap();

        app.update();
        check(app.world_mut(), 0);
    }
}

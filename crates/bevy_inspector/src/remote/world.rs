//! A [`World`] mirroring the entities of the remote app.
//!
//! Remote entities are spawned at their remote ids, so the entity tree reads the mirror like any
//! other world. Only their [`Name`] and [`ChildOf`] are inserted. The other components the tree
//! labels entities with are recorded on the entity's [`RemoteComponents`].

use alloc::{string::String, vec::Vec};
use core::cmp::Ordering;

use bevy_ecs::{
    component::{Component, ComponentId},
    entity::{Entity, SpawnError},
    hierarchy::{ChildOf, Children},
    name::Name,
    query::With,
    reflect::{AppTypeRegistry, ReflectComponent},
    resource::Resource,
    world::{FromWorld, World},
};
use bevy_log::warn_once;
use bevy_platform::collections::HashMap;

/// The most generations [`spawn_at_remote_id`] steps through for one entity in one call.
const MAX_GENERATION_STEPS: u32 = 4096;

/// The [`World`] mirroring the remote app.
///
/// It holds a clone of the local [`AppTypeRegistry`], sharing its types, and the remote entities
/// the inspector shows, each with a [`RemoteComponents`] record.
#[derive(Resource)]
pub struct RemoteWorld {
    world: World,
    reserved: u32,
    types: HashMap<String, Option<ComponentId>>,
}

impl FromWorld for RemoteWorld {
    fn from_world(world: &mut World) -> Self {
        Self::new(
            world
                .get_resource::<AppTypeRegistry>()
                .cloned()
                .unwrap_or_default(),
        )
    }
}

/// Why a remote entity could not be spawned in the [`RemoteWorld`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SpawnRemoteError {
    /// The index is taken by an entity of the remote world itself, such as a resource.
    Taken,
    /// The index has a later generation than the remote entity, so the remote app restarted.
    Behind,
    /// The generation is more than [`MAX_GENERATION_STEPS`] ahead. Later calls catch up further.
    Ahead,
}

/// The components a poll reported for one remote entity.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub(crate) struct TreeComponents {
    /// The value of the entity's [`Name`].
    pub(crate) name: Option<String>,
    /// The entity's [`ChildOf`] target.
    pub(crate) parent: Option<Entity>,
    /// The sorted type paths of the label-defining components read with a value.
    pub(crate) labels: Vec<String>,
    /// The sorted type paths of the label-defining components `has` reported present, or `None`
    /// if the poll did not ask for `has`.
    pub(crate) detected: Option<Vec<String>>,
}

impl RemoteWorld {
    /// An empty remote world sharing the types of `registry`.
    pub(crate) fn new(registry: AppTypeRegistry) -> Self {
        let mut world = World::new();
        world.insert_resource(registry);
        world.register_component::<Name>();
        world.register_component::<ChildOf>();
        world.register_component::<Children>();
        world.register_component::<RemoteComponents>();
        Self {
            world,
            reserved: 0,
            types: HashMap::default(),
        }
    }

    /// The mirrored world.
    pub fn world(&self) -> &World {
        &self.world
    }

    /// Replaces the world with an empty one.
    pub(crate) fn reset(&mut self, registry: AppTypeRegistry) {
        *self = Self::new(registry);
    }

    /// Whether `remote` is mirrored.
    pub(crate) fn contains(&self, remote: Entity) -> bool {
        self.world.get::<RemoteComponents>(remote).is_some()
    }

    /// The mirrored entities.
    pub(crate) fn entities(&mut self) -> Vec<Entity> {
        self.world
            .query_filtered::<Entity, With<RemoteComponents>>()
            .iter(&self.world)
            .collect()
    }

    /// Spawns `remote` at its remote id. Returns whether it was not mirrored yet.
    pub(crate) fn spawn(&mut self, remote: Entity) -> Result<bool, SpawnRemoteError> {
        if self.contains(remote) {
            return Ok(false);
        }
        spawn_at_remote_id(&mut self.world, &mut self.reserved, remote)?;
        self.world
            .entity_mut(remote)
            .insert(RemoteComponents::default());
        Ok(true)
    }

    /// Despawns `remote` without freeing its index, detaching its children first so that they
    /// are not despawned with it.
    pub(crate) fn despawn(&mut self, remote: Entity) {
        let Ok(mut entity) = self.world.get_entity_mut(remote) else {
            return;
        };
        entity.remove::<Children>();
        entity.despawn_no_free();
    }

    /// Writes the components a poll reported for `remote`. Returns whether anything changed.
    ///
    /// A [`ChildOf`] whose target is not mirrored is left out until the target is, so that the
    /// entity shows as a root meanwhile. The detected label types are kept until a poll that asked
    /// for `has` reports them again.
    pub(crate) fn write(&mut self, remote: Entity, components: TreeComponents) -> bool {
        let parent = components
            .parent
            .filter(|parent| *parent != remote && self.contains(*parent));
        let world = &mut self.world;
        let Ok(mut entity) = world.get_entity_mut(remote) else {
            return false;
        };
        let mut changed = false;
        if entity.get::<Name>().map(Name::as_str) != components.name.as_deref() {
            match components.name {
                Some(name) => entity.insert(Name::new(name)),
                None => entity.remove::<Name>(),
            };
            changed = true;
        }
        if entity.get::<ChildOf>().map(ChildOf::parent) != parent {
            match parent {
                Some(parent) => entity.insert(ChildOf(parent)),
                None => entity.remove::<ChildOf>(),
            };
            changed = true;
        }

        let Some(record) = entity.get::<RemoteComponents>() else {
            return changed;
        };
        let detected = components
            .detected
            .filter(|detected| *detected != record.detected);
        if record.labels == components.labels && detected.is_none() {
            return changed;
        }
        let detected = detected.unwrap_or_else(|| record.detected.clone());
        let registry = world.resource::<AppTypeRegistry>().clone();
        let registry = registry.read();
        let mut ids = Vec::new();
        for type_path in components.labels.iter().chain(&detected) {
            let id = *self.types.entry(type_path.clone()).or_insert_with(|| {
                Some(
                    registry
                        .get_with_type_path(type_path)?
                        .data::<ReflectComponent>()?
                        .register_component(world),
                )
            });
            ids.extend(id);
        }
        world.entity_mut(remote).insert(RemoteComponents {
            labels: components.labels,
            detected,
            ids,
        });
        true
    }
}

/// Spawns an empty entity at exactly `remote`, an id the remote app allocated.
///
/// This is the only place that works around the entity allocator of the remote world:
/// - Every index up to `remote.index()` is allocated and leaked first, so that the allocator never
///   hands one out later, which would make that spawn panic. `reserved` counts them.
/// - An index starts at the first generation, so it is stepped up to `remote.generation()` by
///   spawning and despawning it without freeing, which bumps the generation by one each time. At
///   most [`MAX_GENERATION_STEPS`] steps are taken per call.
fn spawn_at_remote_id(
    world: &mut World,
    reserved: &mut u32,
    remote: Entity,
) -> Result<(), SpawnRemoteError> {
    let index = remote.index_u32();
    while *reserved <= index {
        for entity in world.entity_allocator().alloc_many(index + 1 - *reserved) {
            *reserved = (*reserved).max(entity.index_u32() + 1);
        }
    }
    for _ in 0..MAX_GENERATION_STEPS {
        match world.entities().check_can_spawn_at(remote) {
            Ok(()) => {
                world
                    .spawn_empty_at(remote)
                    .map_err(|_| SpawnRemoteError::Taken)?;
                return Ok(());
            }
            Err(SpawnError::AlreadySpawned) => return Err(SpawnRemoteError::Taken),
            Err(SpawnError::Invalid(error)) => {
                if error.current_generation.cmp_approx(&remote.generation()) != Ordering::Less {
                    return Err(SpawnRemoteError::Behind);
                }
                let current =
                    Entity::from_index_and_generation(remote.index(), error.current_generation);
                world
                    .spawn_empty_at(current)
                    .map_err(|_| SpawnRemoteError::Taken)?
                    .despawn_no_free();
            }
        }
    }
    warn_once!(
        "the remote inspector is still catching up with the generation of {remote}, which can \
         take several polls"
    );
    Err(SpawnRemoteError::Ahead)
}

/// The label-defining components the remote app reported for one mirrored entity.
///
/// Only [`Name`] and [`ChildOf`] are inserted into the remote world, so the entity tree labels
/// remote entities with the components recorded here.
#[derive(Component, Debug, Default)]
pub struct RemoteComponents {
    labels: Vec<String>,
    detected: Vec<String>,
    ids: Vec<ComponentId>,
}

impl RemoteComponents {
    /// The ids in the remote world of the label-defining components the remote app reported that
    /// are registered locally.
    pub fn reported(&self) -> impl Iterator<Item = ComponentId> + '_ {
        self.ids.iter().copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy_reflect::{Reflect, TypePath, TypeRegistryArc};

    #[derive(Component, Reflect)]
    #[reflect(Component)]
    struct Labelled;

    fn registry() -> AppTypeRegistry {
        let registry = AppTypeRegistry(TypeRegistryArc::default());
        registry.write().register::<Labelled>();
        registry
    }

    fn remote(index: u32) -> Entity {
        Entity::from_raw_u32(index).unwrap()
    }

    fn at_generation(index: u32, generation: u32) -> Entity {
        let entity = remote(index);
        Entity::from_index_and_generation(
            entity.index(),
            entity.generation().after_versions(generation),
        )
    }

    fn spawned(entity: Entity) -> RemoteWorld {
        let mut remote_world = RemoteWorld::new(registry());
        remote_world.spawn(entity).unwrap();
        remote_world
    }

    fn child_of(parent: Entity) -> TreeComponents {
        TreeComponents {
            parent: Some(parent),
            ..TreeComponents::default()
        }
    }

    #[test]
    fn spawn_at_rejects_a_later_generation_in_a_fresh_world() {
        let entity = at_generation(5, 3);
        let mut world = World::new();
        assert!(matches!(
            world.spawn_at(entity, ()),
            Err(SpawnError::Invalid(_))
        ));

        let mut remote_world = RemoteWorld::new(registry());
        assert_eq!(remote_world.spawn(entity), Ok(true));
        assert!(remote_world.world.get_entity(entity).is_ok());
        assert_eq!(remote_world.spawn(entity), Ok(false));
    }

    #[test]
    fn hand_placed_ids_are_never_allocated_again() {
        let mut remote_world = RemoteWorld::new(registry());
        let placed = [remote(20), at_generation(40, 2)];
        for entity in placed {
            remote_world.spawn(entity).unwrap();
        }
        remote_world.despawn(placed[0]);
        for _ in 0..100 {
            let entity = remote_world.world.spawn_empty().id();
            assert!(placed.iter().all(|placed| placed.index() != entity.index()));
        }
        assert!(remote_world.world.get_entity(placed[1]).is_ok());
    }

    #[test]
    fn a_respawn_at_the_same_index_catches_up() {
        let mut remote_world = spawned(remote(4));
        remote_world.despawn(remote(4));
        assert_eq!(remote_world.spawn(at_generation(4, 2)), Ok(true));
        assert!(remote_world.world.get_entity(remote(4)).is_err());
    }

    #[test]
    fn an_earlier_generation_means_the_remote_app_restarted() {
        let mut remote_world = spawned(at_generation(4, 2));
        assert_eq!(
            remote_world.spawn(at_generation(4, 1)),
            Err(SpawnRemoteError::Behind)
        );
        remote_world.despawn(at_generation(4, 2));
        assert_eq!(remote_world.spawn(remote(4)), Err(SpawnRemoteError::Behind));
    }

    #[test]
    fn ids_taken_by_the_remote_world_itself_are_skipped() {
        let mut remote_world = RemoteWorld::new(registry());
        assert_eq!(remote_world.spawn(remote(0)), Err(SpawnRemoteError::Taken));
        assert!(!remote_world.contains(remote(0)));
    }

    #[test]
    fn distant_generations_are_caught_up_over_several_calls() {
        let mut remote_world = RemoteWorld::new(registry());
        let entity = at_generation(3, MAX_GENERATION_STEPS + 10);
        assert_eq!(remote_world.spawn(entity), Err(SpawnRemoteError::Ahead));
        assert_eq!(remote_world.spawn(entity), Ok(true));
    }

    #[test]
    fn despawning_a_parent_keeps_its_children() {
        let parent = remote(10);
        let child = remote(11);
        let mut remote_world = spawned(parent);
        remote_world.spawn(child).unwrap();
        remote_world.write(child, child_of(parent));
        remote_world.despawn(parent);
        assert!(remote_world.contains(child));
        assert!(remote_world.world.get::<ChildOf>(child).is_none());
    }

    #[test]
    fn a_child_of_a_missing_parent_waits_until_it_appears() {
        let parent = remote(20);
        let child = remote(21);
        let mut remote_world = spawned(child);
        assert!(!remote_world.write(child, child_of(parent)));
        assert!(remote_world.world.get::<ChildOf>(child).is_none());

        remote_world.spawn(parent).unwrap();
        assert!(remote_world.write(child, child_of(parent)));
        assert_eq!(
            remote_world
                .world
                .get::<ChildOf>(child)
                .map(ChildOf::parent),
            Some(parent)
        );
        assert!(!remote_world.write(child, child_of(parent)));
    }

    #[test]
    fn detected_labels_are_kept_until_has_is_asked_again() {
        let entity = remote(3);
        let mut remote_world = spawned(entity);
        let id = remote_world.world.register_component::<Labelled>();
        let reported = |remote_world: &RemoteWorld| -> Vec<ComponentId> {
            remote_world
                .world
                .get::<RemoteComponents>(entity)
                .unwrap()
                .reported()
                .collect()
        };

        let detected = TreeComponents {
            detected: Some(alloc::vec![Labelled::type_path().into()]),
            ..TreeComponents::default()
        };
        assert!(remote_world.write(entity, detected.clone()));
        assert_eq!(reported(&remote_world), [id]);
        assert!(!remote_world.write(entity, TreeComponents::default()));
        assert_eq!(reported(&remote_world), [id]);
        assert!(!remote_world.write(entity, detected));

        let unknown = TreeComponents {
            detected: Some(alloc::vec!["demo::Unknown".into()]),
            ..TreeComponents::default()
        };
        assert!(remote_world.write(entity, unknown));
        assert!(reported(&remote_world).is_empty());
        assert!(remote_world.world.get::<Labelled>(entity).is_none());
    }
}

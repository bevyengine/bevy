//! This module holds utilities for reference-counting of entities, similar to [`Arc`]. This enables
//! automatic cleanup of entities that can be referenced in multiple places.

use core::{
    fmt::{Debug, Formatter},
    ops::Deref,
};

use bevy_platform::sync::{Arc, Weak};
use concurrent_queue::ConcurrentQueue;

use crate::{entity::Entity, system::Commands};

/// A reference count for an entity.
///
/// This "handle" also stores some optional data, allowing users to customize any shared data
/// between all references to the entity.
///
/// Once all [`EntityRc`] instances have been dropped, the entity will be queued for destruction.
/// This means it is possible for the entity to still exist, while its [`EntityRc`] has been
/// dropped.
///
/// The reverse is also true: a held [`EntityRc`] does not guarantee that the entity still exists.
/// It can still be explicitly despawned, so users should try to be resilient to this.
///
/// This type has similar semantics to [`Arc`].
#[derive(Debug)]
pub struct EntityRc<T: Send + Sync + 'static = ()>(Arc<EntityRcInner<T>>);

impl<T: Send + Sync + 'static> Clone for EntityRc<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T: Send + Sync + 'static> EntityRc<T> {
    /// Creates a new [`EntityWeak`] referring to the same entity (and reference count).
    pub fn downgrade(this: &Self) -> EntityWeak<T> {
        EntityWeak {
            entity: this.0.entity,
            weak: Arc::downgrade(&this.0),
        }
    }

    /// Returns the entity this reference count refers to.
    pub fn entity(&self) -> Entity {
        self.0.entity
    }
}

impl<T: Send + Sync + 'static> Deref for EntityRc<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0.payload
    }
}

/// A "non-owning" reference to a reference-counted entity.
///
/// Holding this handle does not guarantee that the entity will not be cleaned up. This handle
/// allows "upgrading" to an [`EntityRc`], if the reference count is still positive, which **will**
/// avoid clean ups.
///
/// This type has similar semantics to [`Weak`].
#[derive(Debug)]
pub struct EntityWeak<T: Send + Sync + 'static = ()> {
    /// The entity being referenced.
    ///
    /// This allows the entity to be referenced even if the reference count has expired. This is
    /// generally useful for cleanup operations.
    entity: Entity,
    /// The underlying weak reference.
    weak: Weak<EntityRcInner<T>>,
}

impl<T: Send + Sync + 'static> Clone for EntityWeak<T> {
    fn clone(&self) -> Self {
        Self {
            entity: self.entity,
            weak: self.weak.clone(),
        }
    }
}

impl<T: Send + Sync + 'static> EntityWeak<T> {
    /// Attempts to upgrade the weak reference into an [`EntityRc`], which can keep the entity alive
    /// if successful.
    ///
    /// Returns [`None`] if all [`EntityRc`]s were previously dropped. This does not necessarily
    /// mean that the entity has been despawned yet.
    pub fn upgrade(&self) -> Option<EntityRc<T>> {
        self.weak.upgrade().map(EntityRc)
    }

    /// Returns the entity this weak reference count refers to.
    ///
    /// The entity may or may not have been despawned (since the [`EntityRc`]s may have all been
    /// dropped). In order to guarantee the entity remains alive, use [`Self::upgrade`] first. This
    /// accessor exists to support cleanup operations.
    pub fn entity(&self) -> Entity {
        self.entity
    }
}

/// Data stored inside the shared data for [`EntityRc`].
struct EntityRcInner<T: Send + Sync + 'static> {
    /// The concurrent queue to notify when dropping this type.
    drop_notifier: Arc<ConcurrentQueue<Entity>>,
    /// The entity this reference count refers to.
    entity: Entity,
    /// The data that is shared with all reference counts for easy access.
    payload: T,
}

// Manual impl of Debug to avoid debugging the drop_notifier.
impl<T: Send + Sync + 'static + Debug> Debug for EntityRcInner<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("EntityRcInner")
            .field("entity", &self.entity)
            .field("payload", &self.payload)
            .finish()
    }
}

impl<T: Send + Sync + 'static> Drop for EntityRcInner<T> {
    fn drop(&mut self) {
        // Try to push the entity. If the notifier is closed for some reason, that's ok.
        let _ = self.drop_notifier.push(self.entity);
    }
}

/// Allows creating [`EntityRc`] and handles syncing them with the world.
///
/// Note: this can produce [`EntityRc`] containing any "payload", since the payload is not
/// accessible during despawn time. This is because it's possible for the entity to be despawned
/// explicitly even though an [`EntityRc`] is still held - callers should be resilient to this.
pub struct EntityRcSource {
    /// The concurrent queue used for communicating drop events of [`EntityRcInner`]s.
    // Note: this could be a channel, but `bevy_ecs` already depends on `concurrent_queue`, so use
    // it as a simple channel.
    drop_notifier: Arc<ConcurrentQueue<Entity>>,
}

impl Default for EntityRcSource {
    fn default() -> Self {
        Self::new()
    }
}

impl EntityRcSource {
    /// Creates a new source of [`EntityRc`]s.
    ///
    /// Generally, only one [`EntityRcSource`] is needed, but having separate ones allows clean up
    /// operations to occur at different times or different rates.
    pub fn new() -> Self {
        Self {
            drop_notifier: Arc::new(ConcurrentQueue::unbounded()),
        }
    }

    /// Creates a new [`EntityRc`] for `entity`, storing the given `payload` in that [`EntityRc`].
    ///
    /// It is up to the caller to ensure that the provided `entity` does not already have an
    /// [`EntityRc`] associated with it. Providing an `entity` which already has an [`EntityRc`]
    /// will result in two reference counts tracking the same entity and both attempting to despawn
    /// the entity (and more importantly, for a held [`EntityRc`] to have its entity despawned
    /// anyway).
    ///
    /// Providing an `entity` allows this method to be compatible with regular entity allocation
    /// ([`EntityAllocator`](crate::entity::EntityAllocator)), remote entity allocation
    /// ([`RemoteAllocator`](crate::entity::RemoteAllocator)), or even taking an existing entity and
    /// making it reference counted.
    pub fn create_rc<T: Send + Sync + 'static>(&self, entity: Entity, payload: T) -> EntityRc<T> {
        EntityRc(Arc::new(EntityRcInner {
            drop_notifier: self.drop_notifier.clone(),
            entity,
            payload,
        }))
    }

    /// Handles any dropped [`EntityRc`]s and despawns the corresponding entities.
    ///
    /// This must be called regularly in order for reference-counted entities to actually be cleaned
    /// up.
    ///
    /// Note: if you have exclusive world access (`&mut World`), you can use
    /// [`World::commands`](crate::world::World::commands) to get an instance of [`Commands`].
    pub fn handle_dropped_rcs(&self, commands: &mut Commands) {
        for entity in self.drop_notifier.try_iter() {
            let Ok(mut entity) = commands.get_entity(entity) else {
                // We intended to despawn the entity - and the entity is despawned. Someone did our
                // work for us!
                continue;
            };
            // Also only try to despawn here - if the entity is despawned when this is run, it's not
            // a problem.
            entity.try_despawn();
        }
    }
}

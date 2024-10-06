use core::marker::PhantomData;

use bevy_ecs::{
    component::Component,
    entity::{Entity, VisitEntities},
    world::{Command, EntityWorldMut, World},
};
use bevy_utils::tracing::debug;

/// Despawns all [entities](Entity) found using [`VisitEntities`] on the
/// [`Component`] `C` for the provided [`Entity`], including said [`Entity`].
/// If an [`Entity`] cannot be found, a warning will be emitted.
///
/// The target [`Entity`] can be excluded from being despawned using
/// [`with_inclusion`](`DespawnRecursive::with_inclusion`).
///
/// Warnings can be disabled using [`with_warn`](`DespawnRecursive::with_warn`).
///
/// Note that the [`Component`] `C` is _removed_ from the [`Entity`] even if it isn't despawned.
///
/// # Examples
///
/// ```rust
/// # use bevy_hierarchy::{DespawnRecursive, Children, Parent};
/// # use bevy_ecs::world::{Command, World};
/// #
/// # let mut world = World::new();
/// # let parent = world.spawn_empty().id();
/// # let child = world.spawn(Parent::new(parent)).id();
/// #
/// # let mut commands = world.commands();
/// #
/// # let command = {
/// // Despawn all Children from a parent
/// DespawnRecursive::<Children>::new(parent)
/// # };
/// #
/// # commands.queue(command);
/// # world.flush();
/// #
/// # assert!(world.get_entity(child).is_none());
/// # assert!(world.get_entity(parent).is_none());
/// ```
#[derive(Debug)]
pub struct DespawnRecursive<C> {
    /// Target entity
    entity: Entity,
    /// Whether or not this command should output a warning if the entity does not exist
    warn: bool,
    /// Whether this command will despawn the provided entity (`inclusive`) or just
    /// its descendants (`exclusive`).
    inclusive: bool,
    /// Marker for the
    _phantom: PhantomData<fn(C)>,
}

impl<C> DespawnRecursive<C> {
    /// Create a new [`DespawnRecursive`] [`Command`].
    pub const fn new(entity: Entity) -> Self {
        Self {
            entity,
            warn: true,
            inclusive: true,
            _phantom: PhantomData,
        }
    }

    /// Control whether this [`Command`] should emit a warning when attempting to despawn
    /// a nonexistent [`Entity`].
    pub const fn with_warn(mut self, warn: bool) -> Self {
        self.warn = warn;
        self
    }

    /// Control whether this [`Command`] should also despawn the target [`Entity`] (`true`)
    /// or on its descendants (`false`).
    pub const fn with_inclusion(mut self, inclusive: bool) -> Self {
        self.inclusive = inclusive;
        self
    }
}

impl<C: Component + VisitEntities> Command for DespawnRecursive<C> {
    fn apply(self, world: &mut World) {
        #[cfg(feature = "trace")]
        let _span = bevy_utils::tracing::info_span!(
            "command",
            name = "DespawnRecursive",
            entity = bevy_utils::tracing::field::debug(self.entity),
            warn = bevy_utils::tracing::field::debug(self.warn)
        )
        .entered();

        let f = if self.warn {
            despawn::<true>
        } else {
            despawn::<false>
        };

        if self.inclusive {
            visit_recursive_depth_first::<true, C>(world, self.entity, f);
        } else {
            visit_recursive_depth_first::<false, C>(world, self.entity, f);
        }
    }
}

fn visit_recursive_depth_first<const INCLUSIVE: bool, C: Component + VisitEntities>(
    world: &mut World,
    entity: Entity,
    f: fn(&mut World, Entity),
) {
    if let Some(component) = world
        .get_entity_mut(entity)
        .as_mut()
        .and_then(EntityWorldMut::take::<C>)
    {
        component.visit_entities(|e| {
            visit_recursive_depth_first::<true, C>(world, e, f);
        });
    }

    if INCLUSIVE {
        f(world, entity);
    }
}

fn despawn<const WARN: bool>(world: &mut World, entity: Entity) {
    let succeeded = if WARN {
        world.despawn(entity)
    } else {
        world.try_despawn(entity)
    };

    if !succeeded {
        debug!("Failed to despawn entity {:?}", entity);
    }
}

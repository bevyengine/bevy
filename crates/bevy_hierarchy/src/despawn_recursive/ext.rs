use bevy_ecs::{
    component::Component,
    entity::VisitEntities,
    system::EntityCommands,
    world::{Command, EntityWorldMut},
};

use crate::{Children, DespawnRecursive};

/// Trait that holds functions for despawning recursively down the a hierarchy.
pub trait DespawnRecursiveExt: Sized {
    /// Despawns the provided [`Entity`]((bevy_ecs::entity::Entity)) alongside all descendants.
    fn despawn_recursive(self) {
        self.despawn_recursive_with_option::<Children>(true);
    }

    /// Despawns all descendants of the given [`Entity`](bevy_ecs::entity::Entity).
    fn despawn_descendants(&mut self) -> &mut Self {
        self.despawn_descendants_with_option::<Children>(true)
    }

    /// Similar to [`despawn_recursive`](`DespawnRecursiveExt::despawn_recursive`) but does not emit warnings
    fn try_despawn_recursive(self) {
        self.despawn_recursive_with_option::<Children>(false);
    }

    /// Similar to [`despawn_descendants`](`DespawnRecursiveExt::despawn_descendants`) but does not emit warnings
    fn try_despawn_descendants(&mut self) -> &mut Self {
        self.despawn_descendants_with_option::<Children>(false)
    }

    /// Despawns the provided [`Entity`](bevy_ecs::entity::Entity) alongside all descendants as related via `C`.
    /// Optionally warns when attempting to despawn a nonexistent [`Entity`](bevy_ecs::entity::Entity).
    fn despawn_recursive_with_option<C: Component + VisitEntities>(self, warn: bool);

    /// Despawns from [`Entity`](bevy_ecs::entity::Entity) all descendants as related via `C`.
    /// Optionally warns when attempting to despawn a nonexistent [`Entity`](bevy_ecs::entity::Entity).
    fn despawn_descendants_with_option<C: Component + VisitEntities>(
        &mut self,
        warn: bool,
    ) -> &mut Self;
}

impl DespawnRecursiveExt for EntityCommands<'_> {
    fn despawn_recursive_with_option<C: Component + VisitEntities>(mut self, warn: bool) {
        let entity = self.id();
        self.commands().queue(
            DespawnRecursive::<C>::new(entity)
                .with_warn(warn)
                .with_inclusion(true),
        );
    }

    fn despawn_descendants_with_option<C: Component + VisitEntities>(
        &mut self,
        warn: bool,
    ) -> &mut Self {
        let entity = self.id();
        self.commands().queue(
            DespawnRecursive::<C>::new(entity)
                .with_warn(warn)
                .with_inclusion(false),
        );
        self
    }
}

impl<'w> DespawnRecursiveExt for EntityWorldMut<'w> {
    fn despawn_recursive_with_option<C: Component + VisitEntities>(self, warn: bool) {
        DespawnRecursive::<C>::new(self.id())
            .with_warn(warn)
            .with_inclusion(true)
            .apply(self.into_world_mut());
    }

    fn despawn_descendants_with_option<C: Component + VisitEntities>(
        &mut self,
        warn: bool,
    ) -> &mut Self {
        let entity = self.id();

        self.world_scope(|world| {
            DespawnRecursive::<C>::new(entity)
                .with_warn(warn)
                .with_inclusion(false)
                .apply(world);
        });
        self
    }
}

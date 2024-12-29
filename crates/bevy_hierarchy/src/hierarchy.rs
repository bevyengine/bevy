use crate::{
    components::{Children, Parent},
    BuildChildren,
};
use bevy_ecs::{
    component::ComponentCloneHandler,
    entity::{ComponentCloneCtx, Entity, EntityCloneBuilder},
    system::EntityCommands,
    world::{Command, DeferredWorld, EntityWorldMut, World},
};
use log::debug;

/// Despawns the given entity and all its children recursively
#[derive(Debug)]
pub struct DespawnRecursive {
    /// Target entity
    pub entity: Entity,
    /// Whether or not this command should output a warning if the entity does not exist
    pub warn: bool,
}

/// Despawns the given entity's children recursively
#[derive(Debug)]
pub struct DespawnChildrenRecursive {
    /// Target entity
    pub entity: Entity,
    /// Whether or not this command should output a warning if the entity does not exist
    pub warn: bool,
}

/// Function for despawning an entity and all its children
pub fn despawn_with_children_recursive(world: &mut World, entity: Entity, warn: bool) {
    // first, make the entity's own parent forget about it
    if let Some(parent) = world.get::<Parent>(entity).map(|parent| parent.0) {
        if let Some(mut children) = world.get_mut::<Children>(parent) {
            children.0.retain(|c| *c != entity);
        }
    }

    // then despawn the entity and all of its children
    despawn_with_children_recursive_inner(world, entity, warn);
}

// Should only be called by `despawn_with_children_recursive` and `try_despawn_with_children_recursive`!
fn despawn_with_children_recursive_inner(world: &mut World, entity: Entity, warn: bool) {
    if let Some(mut children) = world.get_mut::<Children>(entity) {
        for e in core::mem::take(&mut children.0) {
            despawn_with_children_recursive_inner(world, e, warn);
        }
    }

    if warn {
        if !world.despawn(entity) {
            debug!("Failed to despawn entity {}", entity);
        }
    } else if !world.try_despawn(entity) {
        debug!("Failed to despawn entity {}", entity);
    }
}

fn despawn_children_recursive(world: &mut World, entity: Entity, warn: bool) {
    if let Some(children) = world.entity_mut(entity).take::<Children>() {
        for e in children.0 {
            despawn_with_children_recursive_inner(world, e, warn);
        }
    }
}

impl Command for DespawnRecursive {
    fn apply(self, world: &mut World) {
        #[cfg(feature = "trace")]
        let _span = tracing::info_span!(
            "command",
            name = "DespawnRecursive",
            entity = tracing::field::debug(self.entity),
            warn = tracing::field::debug(self.warn)
        )
        .entered();
        despawn_with_children_recursive(world, self.entity, self.warn);
    }
}

impl Command for DespawnChildrenRecursive {
    fn apply(self, world: &mut World) {
        #[cfg(feature = "trace")]
        let _span = tracing::info_span!(
            "command",
            name = "DespawnChildrenRecursive",
            entity = tracing::field::debug(self.entity),
            warn = tracing::field::debug(self.warn)
        )
        .entered();

        despawn_children_recursive(world, self.entity, self.warn);
    }
}

/// Trait that holds functions for despawning recursively down the transform hierarchy
pub trait DespawnRecursiveExt {
    /// Despawns the provided entity alongside all descendants.
    fn despawn_recursive(self);

    /// Despawns all descendants of the given entity.
    fn despawn_descendants(&mut self) -> &mut Self;

    /// Similar to [`Self::despawn_recursive`] but does not emit warnings
    fn try_despawn_recursive(self);

    /// Similar to [`Self::despawn_descendants`] but does not emit warnings
    fn try_despawn_descendants(&mut self) -> &mut Self;
}

impl DespawnRecursiveExt for EntityCommands<'_> {
    /// Despawns the provided entity and its children.
    /// This will emit warnings for any entity that does not exist.
    fn despawn_recursive(mut self) {
        let entity = self.id();
        self.commands()
            .queue(DespawnRecursive { entity, warn: true });
    }

    fn despawn_descendants(&mut self) -> &mut Self {
        let entity = self.id();
        self.commands()
            .queue(DespawnChildrenRecursive { entity, warn: true });
        self
    }

    /// Despawns the provided entity and its children.
    /// This will never emit warnings.
    fn try_despawn_recursive(mut self) {
        let entity = self.id();
        self.commands().queue(DespawnRecursive {
            entity,
            warn: false,
        });
    }

    fn try_despawn_descendants(&mut self) -> &mut Self {
        let entity = self.id();
        self.commands().queue(DespawnChildrenRecursive {
            entity,
            warn: false,
        });
        self
    }
}

fn despawn_recursive_inner(world: EntityWorldMut, warn: bool) {
    let entity = world.id();

    #[cfg(feature = "trace")]
    let _span = tracing::info_span!(
        "despawn_recursive",
        entity = tracing::field::debug(entity),
        warn = tracing::field::debug(warn)
    )
    .entered();

    despawn_with_children_recursive(world.into_world_mut(), entity, warn);
}

fn despawn_descendants_inner<'v, 'w>(
    world: &'v mut EntityWorldMut<'w>,
    warn: bool,
) -> &'v mut EntityWorldMut<'w> {
    let entity = world.id();

    #[cfg(feature = "trace")]
    let _span = tracing::info_span!(
        "despawn_descendants",
        entity = tracing::field::debug(entity),
        warn = tracing::field::debug(warn)
    )
    .entered();

    world.world_scope(|world| {
        despawn_children_recursive(world, entity, warn);
    });
    world
}

impl<'w> DespawnRecursiveExt for EntityWorldMut<'w> {
    /// Despawns the provided entity and its children.
    /// This will emit warnings for any entity that does not exist.
    fn despawn_recursive(self) {
        despawn_recursive_inner(self, true);
    }

    fn despawn_descendants(&mut self) -> &mut Self {
        despawn_descendants_inner(self, true)
    }

    /// Despawns the provided entity and its children.
    /// This will not emit warnings.
    fn try_despawn_recursive(self) {
        despawn_recursive_inner(self, false);
    }

    fn try_despawn_descendants(&mut self) -> &mut Self {
        despawn_descendants_inner(self, false)
    }
}

/// Trait that holds functions for cloning entities recursively down the hierarchy
pub trait CloneEntityHierarchyExt {
    /// Sets the option to recursively clone entities.
    /// When set to true all children will be cloned with the same options as the parent.
    fn recursive(&mut self, recursive: bool) -> &mut Self;
    /// Sets the option to add cloned entity as a child to the parent entity.
    fn as_child(&mut self, as_child: bool) -> &mut Self;
}

impl CloneEntityHierarchyExt for EntityCloneBuilder<'_> {
    fn recursive(&mut self, recursive: bool) -> &mut Self {
        if recursive {
            self.override_component_clone_handler::<Children>(
                ComponentCloneHandler::custom_handler(component_clone_children),
            )
        } else {
            self.remove_component_clone_handler_override::<Children>()
        }
    }
    fn as_child(&mut self, as_child: bool) -> &mut Self {
        if as_child {
            self.override_component_clone_handler::<Parent>(ComponentCloneHandler::custom_handler(
                component_clone_parent,
            ))
        } else {
            self.remove_component_clone_handler_override::<Parent>()
        }
    }
}

/// Clone handler for the [`Children`] component. Allows to clone the entity recursively.
fn component_clone_children(world: &mut DeferredWorld, ctx: &mut ComponentCloneCtx) {
    let children = ctx
        .read_source_component::<Children>()
        .expect("Source entity must have Children component")
        .iter();
    let parent = ctx.target();
    for child in children {
        let child_clone = world.commands().spawn_empty().id();
        let mut clone_entity = ctx
            .entity_cloner()
            .with_source_and_target(*child, child_clone);
        world.commands().queue(move |world: &mut World| {
            clone_entity.clone_entity(world);
            world.entity_mut(child_clone).set_parent(parent);
        });
    }
}

/// Clone handler for the [`Parent`] component. Allows to add clone as a child to the parent entity.
fn component_clone_parent(world: &mut DeferredWorld, ctx: &mut ComponentCloneCtx) {
    let parent = ctx
        .read_source_component::<Parent>()
        .map(|p| p.0)
        .expect("Source entity must have Parent component");
    world.commands().entity(ctx.target()).set_parent(parent);
}

#[cfg(test)]
mod tests {
    use bevy_ecs::{
        component::Component,
        system::Commands,
        world::{CommandQueue, World},
    };

    use super::DespawnRecursiveExt;
    use crate::{
        child_builder::{BuildChildren, ChildBuild},
        components::Children,
        CloneEntityHierarchyExt,
    };

    #[derive(Component, Clone, Copy, PartialEq, Eq, Ord, PartialOrd, Debug)]
    struct Idx(u32);

    #[derive(Component, Clone, PartialEq, Eq, Ord, PartialOrd, Debug)]
    struct N(String);

    #[test]
    fn despawn_recursive() {
        let mut world = World::default();
        let mut queue = CommandQueue::default();
        let grandparent_entity;
        {
            let mut commands = Commands::new(&mut queue, &world);

            commands
                .spawn((N("Another parent".to_owned()), Idx(0)))
                .with_children(|parent| {
                    parent.spawn((N("Another child".to_owned()), Idx(1)));
                });

            // Create a grandparent entity which will _not_ be deleted
            grandparent_entity = commands.spawn((N("Grandparent".to_owned()), Idx(2))).id();
            commands.entity(grandparent_entity).with_children(|parent| {
                // Add a child to the grandparent (the "parent"), which will get deleted
                parent
                    .spawn((N("Parent, to be deleted".to_owned()), Idx(3)))
                    // All descendants of the "parent" should also be deleted.
                    .with_children(|parent| {
                        parent
                            .spawn((N("First Child, to be deleted".to_owned()), Idx(4)))
                            .with_children(|parent| {
                                // child
                                parent.spawn((
                                    N("First grand child, to be deleted".to_owned()),
                                    Idx(5),
                                ));
                            });
                        parent.spawn((N("Second child, to be deleted".to_owned()), Idx(6)));
                    });
            });

            commands.spawn((N("An innocent bystander".to_owned()), Idx(7)));
        }
        queue.apply(&mut world);

        let parent_entity = world.get::<Children>(grandparent_entity).unwrap()[0];

        {
            let mut commands = Commands::new(&mut queue, &world);
            commands.entity(parent_entity).despawn_recursive();
            // despawning the same entity twice should not panic
            commands.entity(parent_entity).despawn_recursive();
        }
        queue.apply(&mut world);

        let mut results = world
            .query::<(&N, &Idx)>()
            .iter(&world)
            .map(|(a, b)| (a.clone(), *b))
            .collect::<Vec<_>>();
        results.sort_unstable_by_key(|(_, index)| *index);

        {
            let children = world.get::<Children>(grandparent_entity).unwrap();
            assert!(
                !children.iter().any(|&i| i == parent_entity),
                "grandparent should no longer know about its child which has been removed"
            );
        }

        assert_eq!(
            results,
            vec![
                (N("Another parent".to_owned()), Idx(0)),
                (N("Another child".to_owned()), Idx(1)),
                (N("Grandparent".to_owned()), Idx(2)),
                (N("An innocent bystander".to_owned()), Idx(7))
            ]
        );
    }

    #[test]
    fn despawn_descendants() {
        let mut world = World::default();
        let mut queue = CommandQueue::default();
        let mut commands = Commands::new(&mut queue, &world);

        let parent = commands.spawn_empty().id();
        let child = commands.spawn_empty().id();

        commands
            .entity(parent)
            .add_child(child)
            .despawn_descendants();

        queue.apply(&mut world);

        // The parent's Children component should be removed.
        assert!(world.entity(parent).get::<Children>().is_none());
        // The child should be despawned.
        assert!(world.get_entity(child).is_err());
    }

    #[test]
    fn spawn_children_after_despawn_descendants() {
        let mut world = World::default();
        let mut queue = CommandQueue::default();
        let mut commands = Commands::new(&mut queue, &world);

        let parent = commands.spawn_empty().id();
        let child = commands.spawn_empty().id();

        commands
            .entity(parent)
            .add_child(child)
            .despawn_descendants()
            .with_children(|parent| {
                parent.spawn_empty();
                parent.spawn_empty();
            });

        queue.apply(&mut world);

        // The parent's Children component should still have two children.
        let children = world.entity(parent).get::<Children>();
        assert!(children.is_some());
        assert_eq!(children.unwrap().len(), 2_usize);
        // The original child should be despawned.
        assert!(world.get_entity(child).is_err());
    }

    #[test]
    fn clone_entity_recursive() {
        #[derive(Component, PartialEq, Eq, Clone)]
        struct Component1 {
            field: usize,
        }

        let parent_component = Component1 { field: 10 };
        let child1_component = Component1 { field: 20 };
        let child1_1_component = Component1 { field: 30 };
        let child2_component = Component1 { field: 21 };
        let child2_1_component = Component1 { field: 31 };

        let mut world = World::default();

        let mut queue = CommandQueue::default();
        let e_clone = {
            let mut commands = Commands::new(&mut queue, &world);
            let e = commands
                .spawn(parent_component.clone())
                .with_children(|children| {
                    children
                        .spawn(child1_component.clone())
                        .with_children(|children| {
                            children.spawn(child1_1_component.clone());
                        });
                    children
                        .spawn(child2_component.clone())
                        .with_children(|children| {
                            children.spawn(child2_1_component.clone());
                        });
                })
                .id();
            let e_clone = commands
                .entity(e)
                .clone_and_spawn_with(|builder| {
                    builder.recursive(true);
                })
                .id();
            e_clone
        };
        queue.apply(&mut world);

        assert!(world
            .get::<Component1>(e_clone)
            .is_some_and(|c| *c == parent_component));

        let children = world.get::<Children>(e_clone).unwrap();
        for (child, (component1, component2)) in children.iter().zip([
            (child1_component, child1_1_component),
            (child2_component, child2_1_component),
        ]) {
            assert!(world
                .get::<Component1>(*child)
                .is_some_and(|c| *c == component1));
            for child2 in world.get::<Children>(*child).unwrap().iter() {
                assert!(world
                    .get::<Component1>(*child2)
                    .is_some_and(|c| *c == component2));
            }
        }
    }

    #[test]
    fn clone_entity_as_child() {
        let mut world = World::default();
        let mut queue = CommandQueue::default();
        let mut commands = Commands::new(&mut queue, &world);

        let child = commands.spawn_empty().id();
        let parent = commands.spawn_empty().add_child(child).id();

        let child_clone = commands
            .entity(child)
            .clone_and_spawn_with(|builder| {
                builder.as_child(true);
            })
            .id();

        queue.apply(&mut world);

        assert!(world
            .entity(parent)
            .get::<Children>()
            .is_some_and(|c| c.contains(&child_clone)));
    }
}

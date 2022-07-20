use smallvec::SmallVec;

use bevy_ecs::{
    bundle::Bundle,
    entity::Entity,
    system::{Command, Commands, EntityCommands},
    world::{EntityMut, World},
};

use crate::prelude::{Children, Parent, PreviousParent};

/// Command that adds a child to an entity
#[derive(Debug)]
pub struct AddChild {
    /// Parent entity to add the child to
    pub parent: Entity,
    /// Child entity to add
    pub child: Entity,
}

impl Command for AddChild {
    fn write(self, world: &mut World) {
        world
            .entity_mut(self.child)
            // FIXME: don't erase the previous parent (see #1545)
            .insert_bundle((Parent(self.parent), PreviousParent(self.parent)));
        if let Some(mut children) = world.get_mut::<Children>(self.parent) {
            children.0.push(self.child);
        } else {
            world
                .entity_mut(self.parent)
                .insert(Children(smallvec::smallvec![self.child]));
        }
    }
}

/// Command that inserts a child at a given index of a parent's children, shifting following children back
#[derive(Debug)]
pub struct InsertChildren {
    parent: Entity,
    children: SmallVec<[Entity; 8]>,
    index: usize,
}

impl Command for InsertChildren {
    fn write(self, world: &mut World) {
        for child in self.children.iter() {
            world
                .entity_mut(*child)
                // FIXME: don't erase the previous parent (see #1545)
                .insert_bundle((Parent(self.parent), PreviousParent(self.parent)));
        }
        {
            if let Some(mut children) = world.get_mut::<Children>(self.parent) {
                children.0.insert_from_slice(self.index, &self.children);
            } else {
                world
                    .entity_mut(self.parent)
                    .insert(Children(self.children));
            }
        }
    }
}

/// Command that pushes children to the end of the entity's children
#[derive(Debug)]
pub struct PushChildren {
    parent: Entity,
    children: SmallVec<[Entity; 8]>,
}

impl Command for PushChildren {
    fn write(self, world: &mut World) {
        for child in self.children.iter() {
            world
                .entity_mut(*child)
                // FIXME: don't erase the previous parent (see #1545)
                .insert_bundle((Parent(self.parent), PreviousParent(self.parent)));
        }
        {
            let mut added = false;
            if let Some(mut children) = world.get_mut::<Children>(self.parent) {
                children.0.extend(self.children.iter().cloned());
                added = true;
            }

            // NOTE: ideally this is just an else statement, but currently that _incorrectly_ fails
            // borrow-checking
            if !added {
                world
                    .entity_mut(self.parent)
                    .insert(Children(self.children));
            }
        }
    }
}

/// Command that removes children from an entity, and removes that child's parent and inserts it into the previous parent component
pub struct RemoveChildren {
    parent: Entity,
    children: SmallVec<[Entity; 8]>,
}

fn remove_children(parent: Entity, children: &[Entity], world: &mut World) {
    for child in children.iter() {
        let mut child = world.entity_mut(*child);
        let mut remove_parent = false;
        if let Some(child_parent) = child.get_mut::<Parent>() {
            if child_parent.0 == parent {
                remove_parent = true;
            }
        }
        if remove_parent {
            if let Some(parent) = child.remove::<Parent>() {
                child.insert(PreviousParent(parent.0));
            }
        }
    }
    // Remove the children from the parents.
    if let Some(mut parent_children) = world.get_mut::<Children>(parent) {
        parent_children
            .0
            .retain(|parent_child| !children.contains(parent_child));
    }
}

impl Command for RemoveChildren {
    fn write(self, world: &mut World) {
        // Remove any matching Parent components from the children
        remove_children(self.parent, &self.children, world);
    }
}

/// Struct for building children onto an entity
pub struct ChildBuilder<'w, 's, 'a> {
    commands: &'a mut Commands<'w, 's>,
    push_children: PushChildren,
}

impl<'w, 's, 'a> ChildBuilder<'w, 's, 'a> {
    /// Spawns an entity with the given bundle and inserts it into the children defined by the [`ChildBuilder`]
    pub fn spawn_bundle(&mut self, bundle: impl Bundle) -> EntityCommands<'w, 's, '_> {
        let e = self.commands.spawn_bundle(bundle);
        self.push_children.children.push(e.id());
        e
    }

    /// Spawns an [`Entity`] with no components and inserts it into the children defined by the [`ChildBuilder`] which adds the [`Parent`] component to it.
    pub fn spawn(&mut self) -> EntityCommands<'w, 's, '_> {
        let e = self.commands.spawn();
        self.push_children.children.push(e.id());
        e
    }

    /// Returns the parent entity of this [`ChildBuilder`]
    pub fn parent_entity(&self) -> Entity {
        self.push_children.parent
    }

    /// Adds a command to this [`ChildBuilder`]
    pub fn add_command<C: Command + 'static>(&mut self, command: C) -> &mut Self {
        self.commands.add(command);
        self
    }
}

/// Trait defining how to build children
pub trait BuildChildren {
    /// Creates a [`ChildBuilder`] with the given children built in the given closure
    ///
    /// Compared to [`add_children`][BuildChildren::add_children], this method returns self
    /// to allow chaining.
    fn with_children(&mut self, f: impl FnOnce(&mut ChildBuilder)) -> &mut Self;
    /// Creates a [`ChildBuilder`] with the given children built in the given closure
    ///
    /// Compared to [`with_children`][BuildChildren::with_children], this method returns the
    /// the value returned from the closure, but doesn't allow chaining.
    ///
    /// ## Example
    ///
    /// ```no_run
    /// # use bevy_ecs::prelude::*;
    /// # use bevy_hierarchy::*;
    /// #
    /// # #[derive(Component)]
    /// # struct SomethingElse;
    /// #
    /// # #[derive(Component)]
    /// # struct MoreStuff;
    /// #
    /// # fn foo(mut commands: Commands) {
    ///     let mut parent_commands = commands.spawn();
    ///     let child_id = parent_commands.add_children(|parent| {
    ///         parent.spawn().id()
    ///     });
    ///
    ///     parent_commands.insert(SomethingElse);
    ///     commands.entity(child_id).with_children(|parent| {
    ///         parent.spawn().insert(MoreStuff);
    ///     });
    /// # }
    /// ```
    fn add_children<T>(&mut self, f: impl FnOnce(&mut ChildBuilder) -> T) -> T;
    /// Pushes children to the back of the builder's children
    fn push_children(&mut self, children: &[Entity]) -> &mut Self;
    /// Inserts children at the given index
    fn insert_children(&mut self, index: usize, children: &[Entity]) -> &mut Self;
    /// Removes the given children
    fn remove_children(&mut self, children: &[Entity]) -> &mut Self;
    /// Adds a single child
    fn add_child(&mut self, child: Entity) -> &mut Self;
}

impl<'w, 's, 'a> BuildChildren for EntityCommands<'w, 's, 'a> {
    fn with_children(&mut self, spawn_children: impl FnOnce(&mut ChildBuilder)) -> &mut Self {
        self.add_children(spawn_children);
        self
    }

    fn add_children<T>(&mut self, spawn_children: impl FnOnce(&mut ChildBuilder) -> T) -> T {
        let parent = self.id();
        let mut builder = ChildBuilder {
            commands: self.commands(),
            push_children: PushChildren {
                children: SmallVec::default(),
                parent,
            },
        };

        let result = spawn_children(&mut builder);
        let children = builder.push_children;
        self.commands().add(children);

        result
    }

    fn push_children(&mut self, children: &[Entity]) -> &mut Self {
        let parent = self.id();
        self.commands().add(PushChildren {
            children: SmallVec::from(children),
            parent,
        });
        self
    }

    fn insert_children(&mut self, index: usize, children: &[Entity]) -> &mut Self {
        let parent = self.id();
        self.commands().add(InsertChildren {
            children: SmallVec::from(children),
            index,
            parent,
        });
        self
    }

    fn remove_children(&mut self, children: &[Entity]) -> &mut Self {
        let parent = self.id();
        self.commands().add(RemoveChildren {
            children: SmallVec::from(children),
            parent,
        });
        self
    }

    fn add_child(&mut self, child: Entity) -> &mut Self {
        let parent = self.id();
        self.commands().add(AddChild { child, parent });
        self
    }
}

/// Struct for adding children to an entity directly through the [`World`] for use in exclusive systems
#[derive(Debug)]
pub struct WorldChildBuilder<'w> {
    world: &'w mut World,
    current_entity: Option<Entity>,
    parent_entities: Vec<Entity>,
}

impl<'w> WorldChildBuilder<'w> {
    /// Spawns an entity with the given bundle and inserts it into the children defined by the [`WorldChildBuilder`]
    pub fn spawn_bundle(&mut self, bundle: impl Bundle + Send + Sync + 'static) -> EntityMut<'_> {
        let parent_entity = self.parent_entity();
        let entity = self
            .world
            .spawn()
            .insert_bundle(bundle)
            .insert_bundle((Parent(parent_entity), PreviousParent(parent_entity)))
            .id();
        self.current_entity = Some(entity);
        if let Some(mut parent) = self.world.get_entity_mut(parent_entity) {
            if let Some(mut children) = parent.get_mut::<Children>() {
                children.0.push(entity);
            } else {
                parent.insert(Children(smallvec::smallvec![entity]));
            }
        }
        self.world.entity_mut(entity)
    }

    /// Spawns an [`Entity`] with no components and inserts it into the children defined by the [`WorldChildBuilder`] which adds the [`Parent`] component to it.
    pub fn spawn(&mut self) -> EntityMut<'_> {
        let parent_entity = self.parent_entity();
        let entity = self
            .world
            .spawn()
            .insert_bundle((Parent(parent_entity), PreviousParent(parent_entity)))
            .id();
        self.current_entity = Some(entity);
        if let Some(mut parent) = self.world.get_entity_mut(parent_entity) {
            if let Some(mut children) = parent.get_mut::<Children>() {
                children.0.push(entity);
            } else {
                parent.insert(Children(smallvec::smallvec![entity]));
            }
        }
        self.world.entity_mut(entity)
    }

    /// Returns the parent entity of this [`WorldChildBuilder`]
    pub fn parent_entity(&self) -> Entity {
        self.parent_entities
            .last()
            .cloned()
            .expect("There should always be a parent at this point.")
    }
}

/// Trait that defines adding children to an entity directly through the [`World`]
pub trait BuildWorldChildren {
    /// Creates a [`WorldChildBuilder`] with the given children built in the given closure
    fn with_children(&mut self, spawn_children: impl FnOnce(&mut WorldChildBuilder)) -> &mut Self;
    /// Pushes children to the back of the builder's children
    fn push_children(&mut self, children: &[Entity]) -> &mut Self;
    /// Inserts children at the given index
    fn insert_children(&mut self, index: usize, children: &[Entity]) -> &mut Self;
    /// Removes the given children
    fn remove_children(&mut self, children: &[Entity]) -> &mut Self;
}

impl<'w> BuildWorldChildren for EntityMut<'w> {
    fn with_children(&mut self, spawn_children: impl FnOnce(&mut WorldChildBuilder)) -> &mut Self {
        {
            let entity = self.id();
            let mut builder = WorldChildBuilder {
                current_entity: None,
                parent_entities: vec![entity],
                // SAFETY: self.update_location() is called below. It is impossible to make EntityMut
                // function calls on `self` within the scope defined here
                world: unsafe { self.world_mut() },
            };

            spawn_children(&mut builder);
        }
        self.update_location();
        self
    }

    fn push_children(&mut self, children: &[Entity]) -> &mut Self {
        let parent = self.id();
        {
            // SAFETY: parent entity is not modified and its location is updated manually
            let world = unsafe { self.world_mut() };
            for child in children.iter() {
                world
                    .entity_mut(*child)
                    // FIXME: don't erase the previous parent (see #1545)
                    .insert_bundle((Parent(parent), PreviousParent(parent)));
            }
            // Inserting a bundle in the children entities may change the parent entity's location if they were of the same archetype
            self.update_location();
        }
        if let Some(mut children_component) = self.get_mut::<Children>() {
            children_component.0.extend(children.iter().cloned());
        } else {
            self.insert(Children::with(children));
        }
        self
    }

    fn insert_children(&mut self, index: usize, children: &[Entity]) -> &mut Self {
        let parent = self.id();
        {
            // SAFETY: parent entity is not modified and its location is updated manually
            let world = unsafe { self.world_mut() };
            for child in children.iter() {
                world
                    .entity_mut(*child)
                    // FIXME: don't erase the previous parent (see #1545)
                    .insert_bundle((Parent(parent), PreviousParent(parent)));
            }
            // Inserting a bundle in the children entities may change the parent entity's location if they were of the same archetype
            self.update_location();
        }

        if let Some(mut children_component) = self.get_mut::<Children>() {
            children_component.0.insert_from_slice(index, children);
        } else {
            self.insert(Children::with(children));
        }
        self
    }

    fn remove_children(&mut self, children: &[Entity]) -> &mut Self {
        let parent = self.id();
        // SAFETY: This doesn't change the parent's location
        let world = unsafe { self.world_mut() };
        for child in children.iter() {
            let mut child = world.entity_mut(*child);
            let mut remove_parent = false;
            if let Some(child_parent) = child.get_mut::<Parent>() {
                if child_parent.0 == parent {
                    remove_parent = true;
                }
            }
            if remove_parent {
                if let Some(parent) = child.remove::<Parent>() {
                    child.insert(PreviousParent(parent.0));
                }
            }
        }
        // Remove the children from the parents.
        if let Some(mut parent_children) = world.get_mut::<Children>(parent) {
            parent_children
                .0
                .retain(|parent_child| !children.contains(parent_child));
        }
        self
    }
}

impl<'w> BuildWorldChildren for WorldChildBuilder<'w> {
    fn with_children(
        &mut self,
        spawn_children: impl FnOnce(&mut WorldChildBuilder<'w>),
    ) -> &mut Self {
        let current_entity = self
            .current_entity
            .expect("Cannot add children without a parent. Try creating an entity first.");
        self.parent_entities.push(current_entity);
        self.current_entity = None;

        spawn_children(self);

        self.current_entity = self.parent_entities.pop();
        self
    }

    fn push_children(&mut self, children: &[Entity]) -> &mut Self {
        let parent = self
            .current_entity
            .expect("Cannot add children without a parent. Try creating an entity first.");
        for child in children.iter() {
            self.world
                .entity_mut(*child)
                // FIXME: don't erase the previous parent (see #1545)
                .insert_bundle((Parent(parent), PreviousParent(parent)));
        }
        if let Some(mut children_component) = self.world.get_mut::<Children>(parent) {
            children_component.0.extend(children.iter().cloned());
        } else {
            self.world
                .entity_mut(parent)
                .insert(Children::with(children));
        }
        self
    }

    fn insert_children(&mut self, index: usize, children: &[Entity]) -> &mut Self {
        let parent = self
            .current_entity
            .expect("Cannot add children without a parent. Try creating an entity first.");

        for child in children.iter() {
            self.world
                .entity_mut(*child)
                // FIXME: don't erase the previous parent (see #1545)
                .insert_bundle((Parent(parent), PreviousParent(parent)));
        }
        if let Some(mut children_component) = self.world.get_mut::<Children>(parent) {
            children_component.0.insert_from_slice(index, children);
        } else {
            self.world
                .entity_mut(parent)
                .insert(Children::with(children));
        }
        self
    }

    fn remove_children(&mut self, children: &[Entity]) -> &mut Self {
        let parent = self
            .current_entity
            .expect("Cannot remove children without a parent. Try creating an entity first.");

        remove_children(parent, children, self.world);
        self
    }
}

#[cfg(test)]
mod tests {
    use smallvec::{smallvec, SmallVec};

    use bevy_ecs::{
        component::Component,
        entity::Entity,
        system::{CommandQueue, Commands},
        world::World,
    };

    use crate::prelude::{Children, Parent, PreviousParent};

    use super::{BuildChildren, BuildWorldChildren};

    #[derive(Component)]
    struct C(u32);

    #[test]
    fn build_children() {
        let mut world = World::default();
        let mut queue = CommandQueue::default();
        let mut commands = Commands::new(&mut queue, &world);

        let parent = commands.spawn().insert(C(1)).id();
        let children = commands.entity(parent).add_children(|parent| {
            [
                parent.spawn().insert(C(2)).id(),
                parent.spawn().insert(C(3)).id(),
                parent.spawn().insert(C(4)).id(),
            ]
        });

        queue.apply(&mut world);
        assert_eq!(
            world.get::<Children>(parent).unwrap().0.as_slice(),
            children.as_slice(),
        );
        assert_eq!(*world.get::<Parent>(children[0]).unwrap(), Parent(parent));
        assert_eq!(*world.get::<Parent>(children[1]).unwrap(), Parent(parent));

        assert_eq!(
            *world.get::<PreviousParent>(children[0]).unwrap(),
            PreviousParent(parent)
        );
        assert_eq!(
            *world.get::<PreviousParent>(children[1]).unwrap(),
            PreviousParent(parent)
        );
    }

    #[test]
    fn push_and_insert_and_remove_children_commands() {
        let mut world = World::default();

        let entities = world
            .spawn_batch(vec![(C(1),), (C(2),), (C(3),), (C(4),), (C(5),)])
            .collect::<Vec<Entity>>();

        let mut queue = CommandQueue::default();
        {
            let mut commands = Commands::new(&mut queue, &world);
            commands.entity(entities[0]).push_children(&entities[1..3]);
        }
        queue.apply(&mut world);

        let parent = entities[0];
        let child1 = entities[1];
        let child2 = entities[2];
        let child3 = entities[3];
        let child4 = entities[4];

        let expected_children: SmallVec<[Entity; 8]> = smallvec![child1, child2];
        assert_eq!(
            world.get::<Children>(parent).unwrap().0.clone(),
            expected_children
        );
        assert_eq!(*world.get::<Parent>(child1).unwrap(), Parent(parent));
        assert_eq!(*world.get::<Parent>(child2).unwrap(), Parent(parent));

        assert_eq!(
            *world.get::<PreviousParent>(child1).unwrap(),
            PreviousParent(parent)
        );
        assert_eq!(
            *world.get::<PreviousParent>(child2).unwrap(),
            PreviousParent(parent)
        );

        {
            let mut commands = Commands::new(&mut queue, &world);
            commands.entity(parent).insert_children(1, &entities[3..]);
        }
        queue.apply(&mut world);

        let expected_children: SmallVec<[Entity; 8]> = smallvec![child1, child3, child4, child2];
        assert_eq!(
            world.get::<Children>(parent).unwrap().0.clone(),
            expected_children
        );
        assert_eq!(*world.get::<Parent>(child3).unwrap(), Parent(parent));
        assert_eq!(*world.get::<Parent>(child4).unwrap(), Parent(parent));
        assert_eq!(
            *world.get::<PreviousParent>(child3).unwrap(),
            PreviousParent(parent)
        );
        assert_eq!(
            *world.get::<PreviousParent>(child4).unwrap(),
            PreviousParent(parent)
        );

        let remove_children = [child1, child4];
        {
            let mut commands = Commands::new(&mut queue, &world);
            commands.entity(parent).remove_children(&remove_children);
        }
        queue.apply(&mut world);

        let expected_children: SmallVec<[Entity; 8]> = smallvec![child3, child2];
        assert_eq!(
            world.get::<Children>(parent).unwrap().0.clone(),
            expected_children
        );
        assert!(world.get::<Parent>(child1).is_none());
        assert!(world.get::<Parent>(child4).is_none());
        assert_eq!(
            *world.get::<PreviousParent>(child1).unwrap(),
            PreviousParent(parent)
        );
        assert_eq!(
            *world.get::<PreviousParent>(child4).unwrap(),
            PreviousParent(parent)
        );
    }

    #[test]
    fn push_and_insert_and_remove_children_world() {
        let mut world = World::default();

        let entities = world
            .spawn_batch(vec![(C(1),), (C(2),), (C(3),), (C(4),), (C(5),)])
            .collect::<Vec<Entity>>();

        world.entity_mut(entities[0]).push_children(&entities[1..3]);

        let parent = entities[0];
        let child1 = entities[1];
        let child2 = entities[2];
        let child3 = entities[3];
        let child4 = entities[4];

        let expected_children: SmallVec<[Entity; 8]> = smallvec![child1, child2];
        assert_eq!(
            world.get::<Children>(parent).unwrap().0.clone(),
            expected_children
        );
        assert_eq!(*world.get::<Parent>(child1).unwrap(), Parent(parent));
        assert_eq!(*world.get::<Parent>(child2).unwrap(), Parent(parent));

        assert_eq!(
            *world.get::<PreviousParent>(child1).unwrap(),
            PreviousParent(parent)
        );
        assert_eq!(
            *world.get::<PreviousParent>(child2).unwrap(),
            PreviousParent(parent)
        );

        world.entity_mut(parent).insert_children(1, &entities[3..]);
        let expected_children: SmallVec<[Entity; 8]> = smallvec![child1, child3, child4, child2];
        assert_eq!(
            world.get::<Children>(parent).unwrap().0.clone(),
            expected_children
        );
        assert_eq!(*world.get::<Parent>(child3).unwrap(), Parent(parent));
        assert_eq!(*world.get::<Parent>(child4).unwrap(), Parent(parent));
        assert_eq!(
            *world.get::<PreviousParent>(child3).unwrap(),
            PreviousParent(parent)
        );
        assert_eq!(
            *world.get::<PreviousParent>(child4).unwrap(),
            PreviousParent(parent)
        );

        let remove_children = [child1, child4];
        world.entity_mut(parent).remove_children(&remove_children);
        let expected_children: SmallVec<[Entity; 8]> = smallvec![child3, child2];
        assert_eq!(
            world.get::<Children>(parent).unwrap().0.clone(),
            expected_children
        );
        assert!(world.get::<Parent>(child1).is_none());
        assert!(world.get::<Parent>(child4).is_none());
        assert_eq!(
            *world.get::<PreviousParent>(child1).unwrap(),
            PreviousParent(parent)
        );
        assert_eq!(
            *world.get::<PreviousParent>(child4).unwrap(),
            PreviousParent(parent)
        );
    }

    #[test]
    fn regression_push_children_same_archetype() {
        let mut world = World::new();
        let child = world.spawn().id();
        world.spawn().push_children(&[child]);
    }
}

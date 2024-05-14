use crate::{Children, HierarchyEvent, Parent};
use bevy_ecs::{
    bundle::Bundle,
    entity::Entity,
    prelude::Events,
    system::{Commands, EntityCommands},
    world::{Command, EntityWorldMut, World},
};
use smallvec::{smallvec, SmallVec};

// Do not use `world.send_event_batch` as it prints error message when the Events are not available in the world,
// even though it's a valid use case to execute commands on a world without events. Loading a GLTF file for example
fn push_events(world: &mut World, events: impl IntoIterator<Item = HierarchyEvent>) {
    if let Some(mut moved) = world.get_resource_mut::<Events<HierarchyEvent>>() {
        moved.extend(events);
    }
}

/// Adds `child` to `parent`'s [`Children`], without checking if it is already present there.
///
/// This might cause unexpected results when removing duplicate children.
fn push_child_unchecked(world: &mut World, parent: Entity, child: Entity) {
    let mut parent = world.entity_mut(parent);
    if let Some(mut children) = parent.get_mut::<Children>() {
        children.0.push(child);
    } else {
        parent.insert(Children(smallvec![child]));
    }
}

/// Sets [`Parent`] of the `child` to `new_parent`. Inserts [`Parent`] if `child` doesn't have one.
fn update_parent(world: &mut World, child: Entity, new_parent: Entity) -> Option<Entity> {
    let mut child = world.entity_mut(child);
    if let Some(mut parent) = child.get_mut::<Parent>() {
        let previous = parent.0;
        *parent = Parent(new_parent);
        Some(previous)
    } else {
        child.insert(Parent(new_parent));
        None
    }
}

/// Remove child from the parent's [`Children`] component.
///
/// Removes the [`Children`] component from the parent if it's empty.
fn remove_from_children(world: &mut World, parent: Entity, child: Entity) {
    let Some(mut parent) = world.get_entity_mut(parent) else {
        return;
    };
    let Some(mut children) = parent.get_mut::<Children>() else {
        return;
    };
    children.0.retain(|x| *x != child);
    if children.is_empty() {
        parent.remove::<Children>();
    }
}

/// Update the [`Parent`] component of the `child`.
/// Removes the `child` from the previous parent's [`Children`].
///
/// Does not update the new parents [`Children`] component.
///
/// Does nothing if `child` was already a child of `parent`.
///
/// Sends [`HierarchyEvent`]'s.
fn update_old_parent(world: &mut World, child: Entity, parent: Entity) {
    let previous = update_parent(world, child, parent);
    if let Some(previous_parent) = previous {
        // Do nothing if the child was already parented to this entity.
        if previous_parent == parent {
            return;
        }
        remove_from_children(world, previous_parent, child);

        push_events(
            world,
            [HierarchyEvent::ChildMoved {
                child,
                previous_parent,
                new_parent: parent,
            }],
        );
    } else {
        push_events(world, [HierarchyEvent::ChildAdded { child, parent }]);
    }
}

/// Update the [`Parent`] components of the `children`.
/// Removes the `children` from their previous parent's [`Children`].
///
/// Does not update the new parents [`Children`] component.
///
/// Does nothing for a child if it was already a child of `parent`.
///
/// Sends [`HierarchyEvent`]'s.
fn update_old_parents(world: &mut World, parent: Entity, children: &[Entity]) {
    let mut events: SmallVec<[HierarchyEvent; 8]> = SmallVec::with_capacity(children.len());
    for &child in children {
        if let Some(previous) = update_parent(world, child, parent) {
            // Do nothing if the entity already has the correct parent.
            if parent == previous {
                continue;
            }

            remove_from_children(world, previous, child);
            events.push(HierarchyEvent::ChildMoved {
                child,
                previous_parent: previous,
                new_parent: parent,
            });
        } else {
            events.push(HierarchyEvent::ChildAdded { child, parent });
        }
    }
    push_events(world, events);
}

/// Removes entities in `children` from `parent`'s [`Children`], removing the component if it ends up empty.
/// Also removes [`Parent`] component from `children`.
fn remove_children(parent: Entity, children: &[Entity], world: &mut World) {
    let mut events: SmallVec<[HierarchyEvent; 8]> = SmallVec::new();
    if let Some(parent_children) = world.get::<Children>(parent) {
        for &child in children {
            if parent_children.contains(&child) {
                events.push(HierarchyEvent::ChildRemoved { child, parent });
            }
        }
    } else {
        return;
    }
    for event in &events {
        if let &HierarchyEvent::ChildRemoved { child, .. } = event {
            world.entity_mut(child).remove::<Parent>();
        }
    }
    push_events(world, events);

    let mut parent = world.entity_mut(parent);
    if let Some(mut parent_children) = parent.get_mut::<Children>() {
        parent_children
            .0
            .retain(|parent_child| !children.contains(parent_child));

        if parent_children.is_empty() {
            parent.remove::<Children>();
        }
    }
}

/// Removes all children from `parent` by removing its [`Children`] component, as well as removing
/// [`Parent`] component from its children.
fn clear_children(parent: Entity, world: &mut World) {
    if let Some(children) = world.entity_mut(parent).take::<Children>() {
        for &child in &children.0 {
            world.entity_mut(child).remove::<Parent>();
        }
    }
}

/// Command that adds a child to an entity.
#[derive(Debug)]
pub struct PushChild {
    /// Parent entity to add the child to.
    pub parent: Entity,
    /// Child entity to add.
    pub child: Entity,
}

impl Command for PushChild {
    fn apply(self, world: &mut World) {
        world.entity_mut(self.parent).add_child(self.child);
    }
}

/// Command that inserts a child at a given index of a parent's children, shifting following children back.
#[derive(Debug)]
pub struct InsertChildren {
    parent: Entity,
    children: SmallVec<[Entity; 8]>,
    index: usize,
}

impl Command for InsertChildren {
    fn apply(self, world: &mut World) {
        world
            .entity_mut(self.parent)
            .insert_children(self.index, &self.children);
    }
}

/// Command that pushes children to the end of the entity's [`Children`].
#[derive(Debug)]
pub struct PushChildren {
    parent: Entity,
    children: SmallVec<[Entity; 8]>,
}

impl Command for PushChildren {
    fn apply(self, world: &mut World) {
        world.entity_mut(self.parent).push_children(&self.children);
    }
}

/// Command that removes children from an entity, and removes these children's parent.
pub struct RemoveChildren {
    parent: Entity,
    children: SmallVec<[Entity; 8]>,
}

impl Command for RemoveChildren {
    fn apply(self, world: &mut World) {
        remove_children(self.parent, &self.children, world);
    }
}

/// Command that clears all children from an entity and removes [`Parent`] component from those
/// children.
pub struct ClearChildren {
    parent: Entity,
}

impl Command for ClearChildren {
    fn apply(self, world: &mut World) {
        clear_children(self.parent, world);
    }
}

/// Command that clear all children from an entity, replacing them with the given children.
pub struct ReplaceChildren {
    parent: Entity,
    children: SmallVec<[Entity; 8]>,
}

impl Command for ReplaceChildren {
    fn apply(self, world: &mut World) {
        clear_children(self.parent, world);
        world.entity_mut(self.parent).push_children(&self.children);
    }
}

/// Command that removes the parent of an entity, and removes that entity from the parent's [`Children`].
pub struct RemoveParent {
    /// `Entity` whose parent must be removed.
    pub child: Entity,
}

impl Command for RemoveParent {
    fn apply(self, world: &mut World) {
        world.entity_mut(self.child).remove_parent();
    }
}

/// Struct for building children entities and adding them to a parent entity.
///
/// # Example
///
/// This example creates three entities, a parent and two children.
///
/// ```
/// # use bevy_ecs::bundle::Bundle;
/// # use bevy_ecs::system::Commands;
/// # use bevy_hierarchy::BuildChildren;
/// # #[derive(Bundle)]
/// # struct MyBundle {}
/// # #[derive(Bundle)]
/// # struct MyChildBundle {}
/// #
/// # fn test(mut commands: Commands) {
/// commands.spawn(MyBundle {}).with_children(|child_builder| {
///     child_builder.spawn(MyChildBundle {});
///     child_builder.spawn(MyChildBundle {});
/// });
/// # }
/// ```
pub struct ChildBuilder<'a> {
    commands: Commands<'a, 'a>,
    push_children: PushChildren,
}

impl ChildBuilder<'_> {
    /// Spawns an entity with the given bundle and inserts it into the parent entity's [`Children`].
    /// Also adds [`Parent`] component to the created entity.
    pub fn spawn(&mut self, bundle: impl Bundle) -> EntityCommands {
        let e = self.commands.spawn(bundle);
        self.push_children.children.push(e.id());
        e
    }

    /// Spawns an [`Entity`] with no components and inserts it into the parent entity's [`Children`].
    /// Also adds [`Parent`] component to the created entity.
    pub fn spawn_empty(&mut self) -> EntityCommands {
        let e = self.commands.spawn_empty();
        self.push_children.children.push(e.id());
        e
    }

    /// Returns the parent entity of this [`ChildBuilder`].
    pub fn parent_entity(&self) -> Entity {
        self.push_children.parent
    }

    /// Adds a command to be executed, like [`Commands::add`].
    pub fn add_command<C: Command>(&mut self, command: C) -> &mut Self {
        self.commands.add(command);
        self
    }
}

/// Trait for removing, adding and replacing children and parents of an entity.
pub trait BuildChildren {
    /// Takes a closure which builds children for this entity using [`ChildBuilder`].
    fn with_children(&mut self, f: impl FnOnce(&mut ChildBuilder)) -> &mut Self;
    /// Pushes children to the back of the builder's children. For any entities that are
    /// already a child of this one, this method does nothing.
    ///
    /// If the children were previously children of another parent, that parent's [`Children`] component
    /// will have those children removed from its list. Removing all children from a parent causes its
    /// [`Children`] component to be removed from the entity.
    ///
    /// # Panics
    ///
    /// Panics if any of the children are the same as the parent.
    fn push_children(&mut self, children: &[Entity]) -> &mut Self;
    /// Inserts children at the given index.
    ///
    /// If the children were previously children of another parent, that parent's [`Children`] component
    /// will have those children removed from its list. Removing all children from a parent causes its
    /// [`Children`] component to be removed from the entity.
    ///
    /// # Panics
    ///
    /// Panics if any of the children are the same as the parent.
    fn insert_children(&mut self, index: usize, children: &[Entity]) -> &mut Self;
    /// Removes the given children
    ///
    /// Removing all children from a parent causes its [`Children`] component to be removed from the entity.
    fn remove_children(&mut self, children: &[Entity]) -> &mut Self;
    /// Adds a single child.
    ///
    /// If the children were previously children of another parent, that parent's [`Children`] component
    /// will have those children removed from its list. Removing all children from a parent causes its
    /// [`Children`] component to be removed from the entity.
    ///
    /// # Panics
    ///
    /// Panics if the child is the same as the parent.
    fn add_child(&mut self, child: Entity) -> &mut Self;
    /// Removes all children from this entity. The [`Children`] component will be removed if it exists, otherwise this does nothing.
    fn clear_children(&mut self) -> &mut Self;
    /// Removes all current children from this entity, replacing them with the specified list of entities.
    ///
    /// The removed children will have their [`Parent`] component removed.
    ///
    /// # Panics
    ///
    /// Panics if any of the children are the same as the parent.
    fn replace_children(&mut self, children: &[Entity]) -> &mut Self;
    /// Sets the parent of this entity.
    ///
    /// If this entity already had a parent, the parent's [`Children`] component will have this
    /// child removed from its list. Removing all children from a parent causes its [`Children`]
    /// component to be removed from the entity.
    ///
    /// # Panics
    ///
    /// Panics if the parent is the same as the child.
    fn set_parent(&mut self, parent: Entity) -> &mut Self;
    /// Removes the [`Parent`] of this entity.
    ///
    /// Also removes this entity from its parent's [`Children`] component. Removing all children from a parent causes
    /// its [`Children`] component to be removed from the entity.
    fn remove_parent(&mut self) -> &mut Self;
}

impl BuildChildren for EntityCommands<'_> {
    fn with_children(&mut self, spawn_children: impl FnOnce(&mut ChildBuilder)) -> &mut Self {
        let parent = self.id();
        let mut builder = ChildBuilder {
            commands: self.commands(),
            push_children: PushChildren {
                children: SmallVec::default(),
                parent,
            },
        };

        spawn_children(&mut builder);
        let children = builder.push_children;
        if children.children.contains(&parent) {
            panic!("Entity cannot be a child of itself.");
        }
        self.commands().add(children);
        self
    }

    fn push_children(&mut self, children: &[Entity]) -> &mut Self {
        let parent = self.id();
        if children.contains(&parent) {
            panic!("Cannot push entity as a child of itself.");
        }
        self.commands().add(PushChildren {
            children: SmallVec::from(children),
            parent,
        });
        self
    }

    fn insert_children(&mut self, index: usize, children: &[Entity]) -> &mut Self {
        let parent = self.id();
        if children.contains(&parent) {
            panic!("Cannot insert entity as a child of itself.");
        }
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
        if child == parent {
            panic!("Cannot add entity as a child of itself.");
        }
        self.commands().add(PushChild { child, parent });
        self
    }

    fn clear_children(&mut self) -> &mut Self {
        let parent = self.id();
        self.commands().add(ClearChildren { parent });
        self
    }

    fn replace_children(&mut self, children: &[Entity]) -> &mut Self {
        let parent = self.id();
        if children.contains(&parent) {
            panic!("Cannot replace entity as a child of itself.");
        }
        self.commands().add(ReplaceChildren {
            children: SmallVec::from(children),
            parent,
        });
        self
    }

    fn set_parent(&mut self, parent: Entity) -> &mut Self {
        let child = self.id();
        if child == parent {
            panic!("Cannot set parent to itself");
        }
        self.commands().add(PushChild { child, parent });
        self
    }

    fn remove_parent(&mut self) -> &mut Self {
        let child = self.id();
        self.commands().add(RemoveParent { child });
        self
    }
}

/// Struct for adding children to an entity directly through the [`World`] for use in exclusive systems.
#[derive(Debug)]
pub struct WorldChildBuilder<'w> {
    world: &'w mut World,
    parent: Entity,
}

impl<'w> WorldChildBuilder<'w> {
    /// Spawns an entity with the given bundle and inserts it into the parent entity's [`Children`].
    /// Also adds [`Parent`] component to the created entity.
    pub fn spawn(&mut self, bundle: impl Bundle) -> EntityWorldMut<'_> {
        let entity = self.world.spawn((bundle, Parent(self.parent))).id();
        push_child_unchecked(self.world, self.parent, entity);
        push_events(
            self.world,
            [HierarchyEvent::ChildAdded {
                child: entity,
                parent: self.parent,
            }],
        );
        self.world.entity_mut(entity)
    }

    /// Spawns an [`Entity`] with no components and inserts it into the parent entity's [`Children`].
    /// Also adds [`Parent`] component to the created entity.
    pub fn spawn_empty(&mut self) -> EntityWorldMut<'_> {
        let entity = self.world.spawn(Parent(self.parent)).id();
        push_child_unchecked(self.world, self.parent, entity);
        push_events(
            self.world,
            [HierarchyEvent::ChildAdded {
                child: entity,
                parent: self.parent,
            }],
        );
        self.world.entity_mut(entity)
    }

    /// Returns the parent entity of this [`WorldChildBuilder`].
    pub fn parent_entity(&self) -> Entity {
        self.parent
    }
}

/// Trait that defines adding, changing and children and parents of an entity directly through the [`World`].
pub trait BuildWorldChildren {
    /// Takes a closure which builds children for this entity using [`WorldChildBuilder`].
    fn with_children(&mut self, spawn_children: impl FnOnce(&mut WorldChildBuilder)) -> &mut Self;

    /// Adds a single child.
    ///
    /// If the children were previously children of another parent, that parent's [`Children`] component
    /// will have those children removed from its list. Removing all children from a parent causes its
    /// [`Children`] component to be removed from the entity.
    ///
    /// # Panics
    ///
    /// Panics if the child is the same as the parent.
    fn add_child(&mut self, child: Entity) -> &mut Self;

    /// Pushes children to the back of the builder's children. For any entities that are
    /// already a child of this one, this method does nothing.
    ///
    /// If the children were previously children of another parent, that parent's [`Children`] component
    /// will have those children removed from its list. Removing all children from a parent causes its
    /// [`Children`] component to be removed from the entity.
    ///
    /// # Panics
    ///
    /// Panics if any of the children are the same as the parent.
    fn push_children(&mut self, children: &[Entity]) -> &mut Self;
    /// Inserts children at the given index.
    ///
    /// If the children were previously children of another parent, that parent's [`Children`] component
    /// will have those children removed from its list. Removing all children from a parent causes its
    /// [`Children`] component to be removed from the entity.
    ///
    /// # Panics
    ///
    /// Panics if any of the children are the same as the parent.
    fn insert_children(&mut self, index: usize, children: &[Entity]) -> &mut Self;
    /// Removes the given children
    ///
    /// Removing all children from a parent causes its [`Children`] component to be removed from the entity.
    fn remove_children(&mut self, children: &[Entity]) -> &mut Self;

    /// Sets the parent of this entity.
    ///
    /// If this entity already had a parent, the parent's [`Children`] component will have this
    /// child removed from its list. Removing all children from a parent causes its [`Children`]
    /// component to be removed from the entity.
    ///
    /// # Panics
    ///
    /// Panics if the parent is the same as the child.
    fn set_parent(&mut self, parent: Entity) -> &mut Self;

    /// Removes the [`Parent`] of this entity.
    ///
    /// Also removes this entity from its parent's [`Children`] component. Removing all children from a parent causes
    /// its [`Children`] component to be removed from the entity.
    fn remove_parent(&mut self) -> &mut Self;
    /// Removes all children from this entity. The [`Children`] component will be removed if it exists, otherwise this does nothing.
    fn clear_children(&mut self) -> &mut Self;
    /// Removes all current children from this entity, replacing them with the specified list of entities.
    ///
    /// The removed children will have their [`Parent`] component removed.
    ///
    /// # Panics
    ///
    /// Panics if any of the children are the same as the parent.
    fn replace_children(&mut self, children: &[Entity]) -> &mut Self;
}

impl<'w> BuildWorldChildren for EntityWorldMut<'w> {
    fn with_children(&mut self, spawn_children: impl FnOnce(&mut WorldChildBuilder)) -> &mut Self {
        let parent = self.id();
        self.world_scope(|world| {
            spawn_children(&mut WorldChildBuilder { world, parent });
        });
        self
    }

    fn add_child(&mut self, child: Entity) -> &mut Self {
        let parent = self.id();
        if child == parent {
            panic!("Cannot add entity as a child of itself.");
        }
        self.world_scope(|world| {
            update_old_parent(world, child, parent);
        });
        if let Some(mut children_component) = self.get_mut::<Children>() {
            children_component.0.retain(|value| child != *value);
            children_component.0.push(child);
        } else {
            self.insert(Children::from_entities(&[child]));
        }
        self
    }

    fn push_children(&mut self, children: &[Entity]) -> &mut Self {
        let parent = self.id();
        if children.contains(&parent) {
            panic!("Cannot push entity as a child of itself.");
        }
        self.world_scope(|world| {
            update_old_parents(world, parent, children);
        });
        if let Some(mut children_component) = self.get_mut::<Children>() {
            children_component
                .0
                .retain(|value| !children.contains(value));
            children_component.0.extend(children.iter().cloned());
        } else {
            self.insert(Children::from_entities(children));
        }
        self
    }

    fn insert_children(&mut self, index: usize, children: &[Entity]) -> &mut Self {
        let parent = self.id();
        if children.contains(&parent) {
            panic!("Cannot insert entity as a child of itself.");
        }
        self.world_scope(|world| {
            update_old_parents(world, parent, children);
        });
        if let Some(mut children_component) = self.get_mut::<Children>() {
            children_component
                .0
                .retain(|value| !children.contains(value));
            children_component.0.insert_from_slice(index, children);
        } else {
            self.insert(Children::from_entities(children));
        }
        self
    }

    fn remove_children(&mut self, children: &[Entity]) -> &mut Self {
        let parent = self.id();
        self.world_scope(|world| {
            remove_children(parent, children, world);
        });
        self
    }

    fn set_parent(&mut self, parent: Entity) -> &mut Self {
        let child = self.id();
        self.world_scope(|world| {
            world.entity_mut(parent).add_child(child);
        });
        self
    }

    fn remove_parent(&mut self) -> &mut Self {
        let child = self.id();
        if let Some(parent) = self.take::<Parent>().map(|p| p.get()) {
            self.world_scope(|world| {
                remove_from_children(world, parent, child);
                push_events(world, [HierarchyEvent::ChildRemoved { child, parent }]);
            });
        }
        self
    }

    fn clear_children(&mut self) -> &mut Self {
        let parent = self.id();
        self.world_scope(|world| {
            clear_children(parent, world);
        });
        self
    }

    fn replace_children(&mut self, children: &[Entity]) -> &mut Self {
        self.clear_children().push_children(children)
    }
}

#[cfg(test)]
mod tests {
    use super::{BuildChildren, BuildWorldChildren};
    use crate::{
        components::{Children, Parent},
        HierarchyEvent::{self, ChildAdded, ChildMoved, ChildRemoved},
    };
    use smallvec::{smallvec, SmallVec};

    use bevy_ecs::{
        component::Component,
        entity::Entity,
        event::Events,
        system::Commands,
        world::{CommandQueue, World},
    };

    /// Assert the (non)existence and state of the child's [`Parent`] component.
    fn assert_parent(world: &World, child: Entity, parent: Option<Entity>) {
        assert_eq!(world.get::<Parent>(child).map(|p| p.get()), parent);
    }

    /// Assert the (non)existence and state of the parent's [`Children`] component.
    fn assert_children(world: &World, parent: Entity, children: Option<&[Entity]>) {
        assert_eq!(world.get::<Children>(parent).map(|c| &**c), children);
    }

    /// Used to omit a number of events that are not relevant to a particular test.
    fn omit_events(world: &mut World, number: usize) {
        let mut events_resource = world.resource_mut::<Events<HierarchyEvent>>();
        let mut events: Vec<_> = events_resource.drain().collect();
        events_resource.extend(events.drain(number..));
    }

    fn assert_events(world: &mut World, expected_events: &[HierarchyEvent]) {
        let events: Vec<_> = world
            .resource_mut::<Events<HierarchyEvent>>()
            .drain()
            .collect();
        assert_eq!(events, expected_events);
    }

    #[test]
    fn add_child() {
        let world = &mut World::new();
        world.insert_resource(Events::<HierarchyEvent>::default());

        let [a, b, c, d] = std::array::from_fn(|_| world.spawn_empty().id());

        world.entity_mut(a).add_child(b);

        assert_parent(world, b, Some(a));
        assert_children(world, a, Some(&[b]));
        assert_events(
            world,
            &[ChildAdded {
                child: b,
                parent: a,
            }],
        );

        world.entity_mut(a).add_child(c);

        assert_children(world, a, Some(&[b, c]));
        assert_parent(world, c, Some(a));
        assert_events(
            world,
            &[ChildAdded {
                child: c,
                parent: a,
            }],
        );
        // Children component should be removed when it's empty.
        world.entity_mut(d).add_child(b).add_child(c);
        assert_children(world, a, None);
    }

    #[test]
    fn set_parent() {
        let world = &mut World::new();
        world.insert_resource(Events::<HierarchyEvent>::default());

        let [a, b, c] = std::array::from_fn(|_| world.spawn_empty().id());

        world.entity_mut(a).set_parent(b);

        assert_parent(world, a, Some(b));
        assert_children(world, b, Some(&[a]));
        assert_events(
            world,
            &[ChildAdded {
                child: a,
                parent: b,
            }],
        );

        world.entity_mut(a).set_parent(c);

        assert_parent(world, a, Some(c));
        assert_children(world, b, None);
        assert_children(world, c, Some(&[a]));
        assert_events(
            world,
            &[ChildMoved {
                child: a,
                previous_parent: b,
                new_parent: c,
            }],
        );
    }

    // regression test for https://github.com/bevyengine/bevy/pull/8346
    #[test]
    fn set_parent_of_orphan() {
        let world = &mut World::new();

        let [a, b, c] = std::array::from_fn(|_| world.spawn_empty().id());
        world.entity_mut(a).set_parent(b);
        assert_parent(world, a, Some(b));
        assert_children(world, b, Some(&[a]));

        world.entity_mut(b).despawn();
        world.entity_mut(a).set_parent(c);

        assert_parent(world, a, Some(c));
        assert_children(world, c, Some(&[a]));
    }

    #[test]
    fn remove_parent() {
        let world = &mut World::new();
        world.insert_resource(Events::<HierarchyEvent>::default());

        let [a, b, c] = std::array::from_fn(|_| world.spawn_empty().id());

        world.entity_mut(a).push_children(&[b, c]);
        world.entity_mut(b).remove_parent();

        assert_parent(world, b, None);
        assert_parent(world, c, Some(a));
        assert_children(world, a, Some(&[c]));
        omit_events(world, 2); // Omit ChildAdded events.
        assert_events(
            world,
            &[ChildRemoved {
                child: b,
                parent: a,
            }],
        );

        world.entity_mut(c).remove_parent();
        assert_parent(world, c, None);
        assert_children(world, a, None);
        assert_events(
            world,
            &[ChildRemoved {
                child: c,
                parent: a,
            }],
        );
    }

    #[allow(dead_code)]
    #[derive(Component)]
    struct C(u32);

    #[test]
    fn build_children() {
        let mut world = World::default();
        let mut queue = CommandQueue::default();
        let mut commands = Commands::new(&mut queue, &world);

        let parent = commands.spawn(C(1)).id();
        let mut children = Vec::new();
        commands.entity(parent).with_children(|parent| {
            children.extend([
                parent.spawn(C(2)).id(),
                parent.spawn(C(3)).id(),
                parent.spawn(C(4)).id(),
            ]);
        });

        queue.apply(&mut world);
        assert_eq!(
            world.get::<Children>(parent).unwrap().0.as_slice(),
            children.as_slice(),
        );
        assert_eq!(*world.get::<Parent>(children[0]).unwrap(), Parent(parent));
        assert_eq!(*world.get::<Parent>(children[1]).unwrap(), Parent(parent));

        assert_eq!(*world.get::<Parent>(children[0]).unwrap(), Parent(parent));
        assert_eq!(*world.get::<Parent>(children[1]).unwrap(), Parent(parent));
    }

    #[test]
    fn push_and_insert_and_remove_children_commands() {
        let mut world = World::default();
        let entities = world
            .spawn_batch(vec![C(1), C(2), C(3), C(4), C(5)])
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

        assert_eq!(*world.get::<Parent>(child1).unwrap(), Parent(parent));
        assert_eq!(*world.get::<Parent>(child2).unwrap(), Parent(parent));

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
    }

    #[test]
    fn push_and_clear_children_commands() {
        let mut world = World::default();
        let entities = world
            .spawn_batch(vec![C(1), C(2), C(3), C(4), C(5)])
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

        let expected_children: SmallVec<[Entity; 8]> = smallvec![child1, child2];
        assert_eq!(
            world.get::<Children>(parent).unwrap().0.clone(),
            expected_children
        );
        assert_eq!(*world.get::<Parent>(child1).unwrap(), Parent(parent));
        assert_eq!(*world.get::<Parent>(child2).unwrap(), Parent(parent));

        {
            let mut commands = Commands::new(&mut queue, &world);
            commands.entity(parent).clear_children();
        }
        queue.apply(&mut world);

        assert!(world.get::<Children>(parent).is_none());

        assert!(world.get::<Parent>(child1).is_none());
        assert!(world.get::<Parent>(child2).is_none());
    }

    #[test]
    fn push_and_replace_children_commands() {
        let mut world = World::default();
        let entities = world
            .spawn_batch(vec![C(1), C(2), C(3), C(4), C(5)])
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
        let child4 = entities[4];

        let expected_children: SmallVec<[Entity; 8]> = smallvec![child1, child2];
        assert_eq!(
            world.get::<Children>(parent).unwrap().0.clone(),
            expected_children
        );
        assert_eq!(*world.get::<Parent>(child1).unwrap(), Parent(parent));
        assert_eq!(*world.get::<Parent>(child2).unwrap(), Parent(parent));

        let replace_children = [child1, child4];
        {
            let mut commands = Commands::new(&mut queue, &world);
            commands.entity(parent).replace_children(&replace_children);
        }
        queue.apply(&mut world);

        let expected_children: SmallVec<[Entity; 8]> = smallvec![child1, child4];
        assert_eq!(
            world.get::<Children>(parent).unwrap().0.clone(),
            expected_children
        );
        assert_eq!(*world.get::<Parent>(child1).unwrap(), Parent(parent));
        assert_eq!(*world.get::<Parent>(child4).unwrap(), Parent(parent));
        assert!(world.get::<Parent>(child2).is_none());
    }

    #[test]
    fn push_and_insert_and_remove_children_world() {
        let mut world = World::default();
        let entities = world
            .spawn_batch(vec![C(1), C(2), C(3), C(4), C(5)])
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

        world.entity_mut(parent).insert_children(1, &entities[3..]);
        let expected_children: SmallVec<[Entity; 8]> = smallvec![child1, child3, child4, child2];
        assert_eq!(
            world.get::<Children>(parent).unwrap().0.clone(),
            expected_children
        );
        assert_eq!(*world.get::<Parent>(child3).unwrap(), Parent(parent));
        assert_eq!(*world.get::<Parent>(child4).unwrap(), Parent(parent));

        let remove_children = [child1, child4];
        world.entity_mut(parent).remove_children(&remove_children);
        let expected_children: SmallVec<[Entity; 8]> = smallvec![child3, child2];
        assert_eq!(
            world.get::<Children>(parent).unwrap().0.clone(),
            expected_children
        );
        assert!(world.get::<Parent>(child1).is_none());
        assert!(world.get::<Parent>(child4).is_none());
    }

    #[test]
    fn push_and_insert_and_clear_children_world() {
        let mut world = World::default();
        let entities = world
            .spawn_batch(vec![C(1), C(2), C(3)])
            .collect::<Vec<Entity>>();

        world.entity_mut(entities[0]).push_children(&entities[1..3]);

        let parent = entities[0];
        let child1 = entities[1];
        let child2 = entities[2];

        let expected_children: SmallVec<[Entity; 8]> = smallvec![child1, child2];
        assert_eq!(
            world.get::<Children>(parent).unwrap().0.clone(),
            expected_children
        );
        assert_eq!(*world.get::<Parent>(child1).unwrap(), Parent(parent));
        assert_eq!(*world.get::<Parent>(child2).unwrap(), Parent(parent));

        world.entity_mut(parent).clear_children();
        assert!(world.get::<Children>(parent).is_none());
        assert!(world.get::<Parent>(child1).is_none());
        assert!(world.get::<Parent>(child2).is_none());
    }

    #[test]
    fn push_and_replace_children_world() {
        let mut world = World::default();
        let entities = world
            .spawn_batch(vec![C(1), C(2), C(3), C(4), C(5)])
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

        world.entity_mut(parent).replace_children(&entities[2..]);
        let expected_children: SmallVec<[Entity; 8]> = smallvec![child2, child3, child4];
        assert_eq!(
            world.get::<Children>(parent).unwrap().0.clone(),
            expected_children
        );
        assert!(world.get::<Parent>(child1).is_none());
        assert_eq!(*world.get::<Parent>(child2).unwrap(), Parent(parent));
        assert_eq!(*world.get::<Parent>(child3).unwrap(), Parent(parent));
        assert_eq!(*world.get::<Parent>(child4).unwrap(), Parent(parent));
    }

    /// Tests what happens when all children are removed from a parent using world functions
    #[test]
    fn children_removed_when_empty_world() {
        let mut world = World::default();
        let entities = world
            .spawn_batch(vec![C(1), C(2), C(3)])
            .collect::<Vec<Entity>>();

        let parent1 = entities[0];
        let parent2 = entities[1];
        let child = entities[2];

        // push child into parent1
        world.entity_mut(parent1).push_children(&[child]);
        assert_eq!(
            world.get::<Children>(parent1).unwrap().0.as_slice(),
            &[child]
        );

        // move only child from parent1 with `push_children`
        world.entity_mut(parent2).push_children(&[child]);
        assert!(world.get::<Children>(parent1).is_none());

        // move only child from parent2 with `insert_children`
        world.entity_mut(parent1).insert_children(0, &[child]);
        assert!(world.get::<Children>(parent2).is_none());

        // remove only child from parent1 with `remove_children`
        world.entity_mut(parent1).remove_children(&[child]);
        assert!(world.get::<Children>(parent1).is_none());
    }

    /// Tests what happens when all children are removed form a parent using commands
    #[test]
    fn children_removed_when_empty_commands() {
        let mut world = World::default();
        let entities = world
            .spawn_batch(vec![C(1), C(2), C(3)])
            .collect::<Vec<Entity>>();

        let parent1 = entities[0];
        let parent2 = entities[1];
        let child = entities[2];

        let mut queue = CommandQueue::default();

        // push child into parent1
        {
            let mut commands = Commands::new(&mut queue, &world);
            commands.entity(parent1).push_children(&[child]);
            queue.apply(&mut world);
        }
        assert_eq!(
            world.get::<Children>(parent1).unwrap().0.as_slice(),
            &[child]
        );

        // move only child from parent1 with `push_children`
        {
            let mut commands = Commands::new(&mut queue, &world);
            commands.entity(parent2).push_children(&[child]);
            queue.apply(&mut world);
        }
        assert!(world.get::<Children>(parent1).is_none());

        // move only child from parent2 with `insert_children`
        {
            let mut commands = Commands::new(&mut queue, &world);
            commands.entity(parent1).insert_children(0, &[child]);
            queue.apply(&mut world);
        }
        assert!(world.get::<Children>(parent2).is_none());

        // move only child from parent1 with `add_child`
        {
            let mut commands = Commands::new(&mut queue, &world);
            commands.entity(parent2).add_child(child);
            queue.apply(&mut world);
        }
        assert!(world.get::<Children>(parent1).is_none());

        // remove only child from parent2 with `remove_children`
        {
            let mut commands = Commands::new(&mut queue, &world);
            commands.entity(parent2).remove_children(&[child]);
            queue.apply(&mut world);
        }
        assert!(world.get::<Children>(parent2).is_none());
    }

    #[test]
    fn regression_push_children_same_archetype() {
        let mut world = World::new();
        let child = world.spawn_empty().id();
        world.spawn_empty().push_children(&[child]);
    }

    #[test]
    fn push_children_idempotent() {
        let mut world = World::new();
        let child = world.spawn_empty().id();
        let parent = world
            .spawn_empty()
            .push_children(&[child])
            .push_children(&[child])
            .id();

        let mut query = world.query::<&Children>();
        let children = query.get(&world, parent).unwrap();
        assert_eq!(**children, [child]);
    }
}

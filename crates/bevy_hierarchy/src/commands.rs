use bevy_ecs::{
    prelude::{Bundle, Entity, Events},
    system::{Command, Commands, EntityCommands},
    world::{EntityMut, World},
};
use smallvec::SmallVec;

use crate::{Children, HierarchyEvent, Parent};

/// Trait defining how to modify the hierarchy.
pub trait HierarchyCommands {
    /// Add the `child` to this entity, at the end of the list.
    ///
    /// If the `child` already had a parent it will be removed from that parent.
    ///
    /// If the `child` already belonged to this entity, it will still be moved to the end of the list.
    fn add_child(&mut self, child: Entity) -> &mut Self;

    /// Add the `children` to this entity, at the end of the list.
    ///
    /// If the `children` already had parents they will be removed from them.
    ///
    /// If the `children` already belonged to this entity, they will still be moved to the end of the list.
    fn add_children(&mut self, children: &[Entity]) -> &mut Self;

    /// Add the `child` to this entity, inserted at at the given `index`.
    ///
    /// If the `child` already had a parent it will be removed from that parent.
    ///
    /// If the `child` already belonged to this entity, it will still be moved to the `index`.
    ///
    fn insert_child(&mut self, index: usize, child: Entity) -> &mut Self;

    /// Add the `children` to this entity, inserted at the given `index`.
    ///
    /// If the `children` already had parents they will be removed from them.
    ///
    /// If the `children` already belonged to this entity, they will still be moved to the `index`.
    fn insert_children(&mut self, index: usize, children: &[Entity]) -> &mut Self;

    /// Remove the `child` from this entity.
    fn remove_child(&mut self, child: Entity) -> &mut Self;

    /// Remove the `children` from this entity.
    fn remove_children(&mut self, children: &[Entity]) -> &mut Self;

    /// Remove all children from this entity.
    fn clear_children(&mut self) -> &mut Self;

    /// Set the `parent` of this entity. This entity will be added to the end of the `parent`'s list of children.
    ///
    /// If this entity already had a parent it will be removed from it.
    fn set_parent(&mut self, parent: Entity) -> &mut Self;

    /// Remove the parent from this entity.
    fn remove_parent(&mut self) -> &mut Self;
}

impl<'w> HierarchyCommands for EntityMut<'w> {
    fn add_child(&mut self, child: Entity) -> &mut Self {
        let parent = self.id();
        {
            // SAFETY: parent entity is not modified and its location is updated manually
            let world = unsafe { self.world_mut() };
            set_parent(world, child, parent);
            // Inserting a bundle in the children entities may change the parent entity's location if they were of the same archetype
            self.update_location();
        }
        if let Some(mut children_component) = self.get_mut::<Children>() {
            children_component.0.retain(|value| child != *value);
            children_component.0.push(child);
        } else {
            self.insert(Children::from_entity(child));
        }
        self
    }

    fn add_children(&mut self, children: &[Entity]) -> &mut Self {
        if children.is_empty() {
            return self;
        }
        let parent = self.id();
        {
            // SAFETY: parent entity is not modified and its location is updated manually
            let world = unsafe { self.world_mut() };
            update_parent_components(world, parent, children);

            // Inserting a bundle in the children entities may change the parent entity's location if they were of the same archetype
            self.update_location();
        }
        if let Some(mut children_component) = self.get_mut::<Children>() {
            children_component
                .0
                .retain(|value| !children.contains(value));
            children_component.0.extend_from_slice(children);
        } else {
            self.insert(Children::from_entities(children));
        }
        self
    }

    fn insert_child(&mut self, index: usize, child: Entity) -> &mut Self {
        let parent = self.id();
        {
            // SAFETY: parent entity is not modified and its location is updated manually
            let world = unsafe { self.world_mut() };
            set_parent(world, child, parent);
            // Inserting a bundle in the children entities may change the parent entity's location if they were of the same archetype
            self.update_location();
        }

        if let Some(mut children_component) = self.get_mut::<Children>() {
            children_component.0.retain(|value| child != *value);
            if index < children_component.0.len() {
                // FIXME The retain above could change the position of elements, so the index is no longer accurate.
                children_component.0.insert(index, child);
            } else {
                children_component.0.push(child);
            }
        } else {
            self.insert(Children::from_entity(child));
        }
        self
    }

    fn insert_children(&mut self, index: usize, children: &[Entity]) -> &mut Self {
        if children.is_empty() {
            return self;
        }
        let parent = self.id();
        {
            // SAFETY: parent entity is not modified and its location is updated manually
            let world = unsafe { self.world_mut() };
            update_parent_components(world, parent, children);
            // Inserting a bundle in the children entities may change the parent entity's location if they were of the same archetype
            self.update_location();
        }

        if let Some(mut children_component) = self.get_mut::<Children>() {
            children_component
                .0
                .retain(|value| !children.contains(value));
            if index < children_component.0.len() {
                // FIXME The retain above could change the position of elements, so the index is no longer accurate.
                children_component.0.insert_from_slice(index, children);
            } else {
                children_component.0.extend_from_slice(children);
            }
        } else {
            self.insert(Children::from_entities(children));
        }
        self
    }

    fn remove_child(&mut self, child: Entity) -> &mut Self {
        if let Some(mut children) = self.get_mut::<Children>() {
            if let Some(i) = children.iter().position(|e| *e == child) {
                children.0.remove(i);
                if children.is_empty() {
                    self.remove::<Children>();
                }
                let parent = self.id();
                // SAFETY: This doesn't change the parent's location
                let world = unsafe { self.world_mut() };
                world.entity_mut(child).remove::<Parent>();
                push_event(world, HierarchyEvent::ChildRemoved { child, parent });
            }
        }
        self
    }

    fn remove_children(&mut self, children: &[Entity]) -> &mut Self {
        let parent = self.id();
        // SAFETY: This doesn't change the parent's location
        let world = unsafe { self.world_mut() };
        remove_children(world, parent, children);
        self
    }

    fn clear_children(&mut self) -> &mut Self {
        let parent = self.id();
        if let Some(children) = self.remove::<Children>() {
            // SAFETY: This doesn't change the parent's location
            let world = unsafe { self.world_mut() };
            for child in children.0 {
                world.entity_mut(child).remove::<Parent>();
                push_event(world, HierarchyEvent::ChildRemoved { child, parent });
            }
        }
        self
    }

    fn set_parent(&mut self, parent: Entity) -> &mut Self {
        let child = self.id();
        // SAFETY: Not
        let world = unsafe { self.world_mut() };
        world.entity_mut(parent).add_child(child);
        self.update_location();
        self
    }

    fn remove_parent(&mut self) -> &mut Self {
        let child = self.id();
        if let Some(parent) = self.remove::<Parent>().map(|p| p.get()) {
            // SAFETY: child entity is not modified and its location is updated manually
            let world = unsafe { self.world_mut() };
            remove_child(world, parent, child);
            push_event(world, HierarchyEvent::ChildRemoved { child, parent });
            // Inserting a bundle in the children entities may change the parent entity's location if they were of the same archetype
            self.update_location();
        }
        self
    }
}

/// Extension trait for [`EntityMut`].
pub trait HierachyEntityMutExt {
    /// Provides a [`WorldChildBuilder`] in the given closure for spawning children.
    ///
    /// ## Example
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// # use bevy_hierarchy::*;
    /// # #[derive(Component)] struct SomethingElse;
    /// # #[derive(Component)] struct MoreStuff;
    /// # let mut world = World::new();
    /// world.spawn(SomethingElse).with_children(|parent| {
    ///     parent.spawn(MoreStuff);
    /// });
    /// ```
    fn with_children(&mut self, f: impl FnOnce(&mut WorldChildBuilder)) -> &mut Self;
}

impl<'w> HierachyEntityMutExt for EntityMut<'w> {
    fn with_children(&mut self, spawn_children: impl FnOnce(&mut WorldChildBuilder)) -> &mut Self {
        {
            let entity = self.id();
            let mut builder = WorldChildBuilder {
                parent: entity,
                // SAFETY: self.update_location() is called below. It is impossible to make EntityMut
                // function calls on `self` within the scope defined here
                world: unsafe { self.world_mut() },
            };

            spawn_children(&mut builder);
        }
        self.update_location();
        self
    }
}

/// Struct for adding children to an entity directly through the [`World`] for use in exclusive systems
#[derive(Debug)]
pub struct WorldChildBuilder<'w> {
    world: &'w mut World,
    parent: Entity,
}

impl<'w> WorldChildBuilder<'w> {
    /// Spawns an entity with the given bundle and inserts it into the children defined by the [`WorldChildBuilder`]
    pub fn spawn(&mut self, bundle: impl Bundle) -> EntityMut<'_> {
        let child = self.world.spawn((bundle, Parent(self.parent))).id();
        add_child_unchecked(self.world, self.parent, child);
        if let Some(mut added) = self.world.get_resource_mut::<Events<HierarchyEvent>>() {
            added.send(HierarchyEvent::ChildAdded {
                child,
                parent: self.parent,
            });
        }
        self.world.entity_mut(child)
    }

    #[deprecated(
        since = "0.9.0",
        note = "Use `spawn` instead, which now accepts bundles, components, and tuples of bundles and components."
    )]
    /// Spawns an entity with the given bundle and inserts it into the children defined by the [`WorldChildBuilder`]
    pub fn spawn_bundle(&mut self, bundle: impl Bundle) -> EntityMut<'_> {
        self.spawn(bundle)
    }

    /// Spawns an [`Entity`] with no components and inserts it into the children defined by the [`WorldChildBuilder`] which adds the [`Parent`] component to it.
    pub fn spawn_empty(&mut self) -> EntityMut<'_> {
        let child = self.world.spawn(Parent(self.parent)).id();
        add_child_unchecked(self.world, self.parent, child);
        if let Some(mut added) = self.world.get_resource_mut::<Events<HierarchyEvent>>() {
            added.send(HierarchyEvent::ChildAdded {
                child,
                parent: self.parent,
            });
        }
        self.world.entity_mut(child)
    }

    /// Returns the parent entity of this [`WorldChildBuilder`].
    pub fn parent_id(&self) -> Entity {
        self.parent
    }
}

/// Struct for building children onto an entity
pub struct ChildBuilder<'w, 's, 'a> {
    commands: &'a mut Commands<'w, 's>,
    add_children: AddChildren,
}

impl<'w, 's, 'a> ChildBuilder<'w, 's, 'a> {
    /// Spawns an entity with the given bundle and inserts it into the children defined by the [`ChildBuilder`]
    #[deprecated(
        since = "0.9.0",
        note = "Use `spawn` instead, which now accepts bundles, components, and tuples of bundles and components."
    )]
    pub fn spawn_bundle(&mut self, bundle: impl Bundle) -> EntityCommands<'w, 's, '_> {
        self.spawn(bundle)
    }

    /// Spawns an entity with the given bundle and inserts it into the children defined by the [`ChildBuilder`]
    pub fn spawn(&mut self, bundle: impl Bundle) -> EntityCommands<'w, 's, '_> {
        let e = self.commands.spawn(bundle);
        self.add_children.children.push(e.id());
        e
    }

    /// Spawns an [`Entity`] with no components and inserts it into the children defined by the [`ChildBuilder`] which adds the [`Parent`] component to it.
    pub fn spawn_empty(&mut self) -> EntityCommands<'w, 's, '_> {
        let e = self.commands.spawn_empty();
        self.add_children.children.push(e.id());
        e
    }

    /// Returns the parent entity of this [`ChildBuilder`]
    pub fn parent_id(&self) -> Entity {
        self.add_children.parent
    }

    /// Returns the underlying [`Commands`].
    pub fn commands(&mut self) -> &mut Commands<'w, 's> {
        self.commands
    }
}

/// Extension trait for [`EntityCommands`].
pub trait HierachyEntityCommandsExt {
    /// Provides a [`ChildBuilder`] in the given closure for spawning children.
    ///
    /// ## Example
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// # use bevy_hierarchy::*;
    /// # #[derive(Component)] struct SomethingElse;
    /// # #[derive(Component)] struct MoreStuff;
    /// #
    /// # fn foo(mut commands: Commands) {
    /// let mut parent_commands = commands
    ///     .spawn(SomethingElse)
    ///     .with_children(|parent| {
    ///         parent.spawn(MoreStuff);
    ///     });
    /// # }
    /// ```
    fn with_children(&mut self, spawn_children: impl FnOnce(&mut ChildBuilder)) -> &mut Self;
}

impl<'w, 's, 'a> HierachyEntityCommandsExt for EntityCommands<'w, 's, 'a> {
    fn with_children(&mut self, spawn_children: impl FnOnce(&mut ChildBuilder)) -> &mut Self {
        let parent = self.id();
        let mut builder = ChildBuilder {
            commands: self.commands(),
            add_children: AddChildren {
                children: SmallVec::default(),
                parent,
            },
        };

        spawn_children(&mut builder);
        let children = builder.add_children;
        self.commands().add(children);
        self
    }
}

impl<'w, 's, 'a> HierarchyCommands for EntityCommands<'w, 's, 'a> {
    fn add_child(&mut self, child: Entity) -> &mut Self {
        let parent = self.id();
        self.commands().add(AddChild { child, parent });
        self
    }

    fn add_children(&mut self, children: &[Entity]) -> &mut Self {
        let parent = self.id();
        self.commands().add(AddChildren {
            children: children.into(),
            parent,
        });
        self
    }

    fn insert_child(&mut self, index: usize, child: Entity) -> &mut Self {
        let parent = self.id();
        self.commands().add(InsertChildren {
            children: smallvec::smallvec![child],
            index,
            parent,
        });
        self
    }

    fn insert_children(&mut self, index: usize, children: &[Entity]) -> &mut Self {
        let parent = self.id();
        self.commands().add(InsertChildren {
            children: children.into(),
            index,
            parent,
        });
        self
    }

    fn remove_child(&mut self, child: Entity) -> &mut Self {
        let parent = self.id();
        self.commands().add(RemoveChild { child, parent });
        self
    }

    fn remove_children(&mut self, children: &[Entity]) -> &mut Self {
        let parent = self.id();
        self.commands().add(RemoveChildren {
            children: children.into(),
            parent,
        });
        self
    }

    fn clear_children(&mut self) -> &mut Self {
        let parent = self.id();
        self.commands().add(ClearChildren { parent });
        self
    }

    fn set_parent(&mut self, parent: Entity) -> &mut Self {
        let child = self.id();
        self.commands().add(AddChild { parent, child });
        self
    }

    fn remove_parent(&mut self) -> &mut Self {
        let child = self.id();
        self.commands().add(RemoveParent { child });
        self
    }
}

fn push_events(world: &mut World, events: SmallVec<[HierarchyEvent; 8]>) {
    if let Some(mut events_resource) = world.get_resource_mut::<Events<_>>() {
        events_resource.extend(events);
    }
}

fn push_event(world: &mut World, event: HierarchyEvent) {
    if let Some(mut events_resource) = world.get_resource_mut::<Events<_>>() {
        events_resource.send(event);
    }
}

/// Update the [`Parent`] component of the child.
///
/// Returns the previous parent if it had one.
/// The previous parent could be the same entity as the new parent.
fn update_parent_component(world: &mut World, child: Entity, parent: Entity) -> Option<Entity> {
    let mut child = world.entity_mut(child);
    if let Some(mut parent_component) = child.get_mut::<Parent>() {
        let previous = parent_component.0;
        parent_component.0 = parent;
        Some(previous)
    } else {
        child.insert(Parent(parent));
        None
    }
}

/// Add child to parent's [`Children`] component without checking if the child is already parented.
fn add_child_unchecked(world: &mut World, parent: Entity, child: Entity) {
    let mut parent = world.entity_mut(parent);
    if let Some(mut children) = parent.get_mut::<Children>() {
        children.0.push(child);
    } else {
        parent.insert(Children::from_entity(child));
    }
}

/// Update the [`Parent`] components of the children.
///
/// Sends [`HierarchyEvent`]'s.
fn update_parent_components(world: &mut World, parent: Entity, children: &[Entity]) {
    let mut events: SmallVec<[_; 8]> = SmallVec::with_capacity(children.len());
    for &child in children {
        if let Some(previous_parent) = update_parent_component(world, child, parent) {
            // Do nothing if the child was already parented to this entity.
            if parent == previous_parent {
                continue;
            }
            remove_child(world, previous_parent, child);
            events.push(HierarchyEvent::ChildMoved {
                child,
                previous_parent,
                new_parent: parent,
            });
        } else {
            events.push(HierarchyEvent::ChildAdded { child, parent });
        }
    }
    push_events(world, events);
}

/// Remove child from the parent's [`Children`] component.
///
/// Removes the [`Children`] component from the parent if it's empty.
fn remove_child(world: &mut World, parent: Entity, child: Entity) {
    let mut parent = world.entity_mut(parent);
    if let Some(mut children) = parent.get_mut::<Children>() {
        children.0.retain(|x| *x != child);
        if children.is_empty() {
            parent.remove::<Children>();
        }
    }
}

/// Remove children from the parent's [`Children`] component and remove their [`Parent`] component.
///
/// Sends [`HierarchyEvent`]'s.
fn remove_children(world: &mut World, parent: Entity, children: &[Entity]) {
    let mut events: SmallVec<[_; 8]> = SmallVec::new();
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

/// Change the parent of `child` to `parent`.
/// Does not update the new parents [`Children`] component.
///
/// Removes the `child` from the previous parent's [`Children`].
///
/// Does nothing if `child` was already a child of `parent`.
///
/// Sends [`HierarchyEvent`]'s.
fn set_parent(world: &mut World, child: Entity, parent: Entity) {
    let previous = update_parent_component(world, child, parent);
    if let Some(previous_parent) = previous {
        // Do nothing if the child was already parented to this entity.
        if previous_parent == parent {
            return;
        }
        remove_child(world, previous_parent, child);
        push_event(
            world,
            HierarchyEvent::ChildMoved {
                child,
                previous_parent,
                new_parent: parent,
            },
        );
    } else {
        push_event(world, HierarchyEvent::ChildAdded { child, parent });
    }
}

/// Command that adds a child to an entity.
#[derive(Debug)]
pub struct AddChild {
    /// Parent entity to add the child to.
    pub parent: Entity,
    /// Child entity to add.
    pub child: Entity,
}

impl Command for AddChild {
    fn write(self, world: &mut World) {
        world.entity_mut(self.parent).add_child(self.child);
    }
}

/// Command that appends children to the end of the entity's children.
#[derive(Debug)]
pub struct AddChildren {
    /// Parent entity to add the children to.
    pub parent: Entity,
    /// Child entities to add.
    pub children: SmallVec<[Entity; 8]>,
}

impl Command for AddChildren {
    fn write(self, world: &mut World) {
        world.entity_mut(self.parent).add_children(&self.children);
    }
}

/// Command that inserts a child at a given index of a parent's children, shifting following children back
#[derive(Debug)]
pub struct InsertChild {
    /// Parent entity to add the child to.
    pub parent: Entity,
    /// The index to insert at.
    pub index: usize,
    /// Child entity to add.
    pub child: Entity,
}

impl Command for InsertChild {
    fn write(self, world: &mut World) {
        world
            .entity_mut(self.parent)
            .insert_child(self.index, self.child);
    }
}

/// Command that inserts a child at a given index of a parent's children, shifting following children back
#[derive(Debug)]
pub struct InsertChildren {
    /// The parent entity to add the children to.
    pub parent: Entity,
    /// The index to insert at.
    pub index: usize,
    /// The child entities to add.
    pub children: SmallVec<[Entity; 8]>,
}

impl Command for InsertChildren {
    fn write(self, world: &mut World) {
        world
            .entity_mut(self.parent)
            .insert_children(self.index, &self.children);
    }
}

/// Command that removes a child from an entity, and removes that child's [`Parent`] component.
#[derive(Debug)]
pub struct RemoveChild {
    /// The parent entity to remove the child from.
    pub parent: Entity,
    /// The child entity to remove.
    pub child: Entity,
}

impl Command for RemoveChild {
    fn write(self, world: &mut World) {
        world.entity_mut(self.parent).remove_child(self.child);
    }
}

/// Command that removes children from an entity, and removes that child's [`Parent`] component.
pub struct RemoveChildren {
    /// The parent entity to remove the children from.
    pub parent: Entity,
    /// The child entities to remove.
    pub children: SmallVec<[Entity; 8]>,
}

impl Command for RemoveChildren {
    fn write(self, world: &mut World) {
        world
            .entity_mut(self.parent)
            .remove_children(&self.children);
    }
}

/// Command that removes all children from an entity, and removes that child's [`Parent`] component.
pub struct ClearChildren {
    /// The parent entity to remove the children from.
    pub parent: Entity,
}

impl Command for ClearChildren {
    fn write(self, world: &mut World) {
        world.entity_mut(self.parent).clear_children();
    }
}

/// Command that removes the parent of an entity, and removes that entity from the parent's [`Children`].
pub struct RemoveParent {
    /// The child entity to remove the parent from.
    pub child: Entity,
}

impl Command for RemoveParent {
    fn write(self, world: &mut World) {
        world.entity_mut(self.child).remove_parent();
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        Children, HierachyEntityCommandsExt, HierarchyCommands,
        HierarchyEvent::{self, ChildAdded, ChildMoved, ChildRemoved},
        Parent,
    };

    use bevy_ecs::{
        entity::Entity,
        prelude::Events,
        schedule::{Schedule, Stage, StageLabel, SystemStage},
        system::{CommandQueue, Commands, IntoSystem},
        world::World,
    };

    fn assert_children(world: &mut World, parent: Entity, children: Option<&[Entity]>) {
        assert_eq!(world.get::<Children>(parent).map(|c| &**c), children);
    }

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

    fn run_system<Param>(world: &mut World, system: impl IntoSystem<(), (), Param>) {
        #[derive(StageLabel)]
        struct UpdateStage;

        let mut schedule = Schedule::default();
        let mut update = SystemStage::parallel();
        update.add_system(system);
        schedule.add_stage(UpdateStage, update);
        schedule.run(world);
    }

    #[test]
    fn add_child() {
        let world = &mut World::new();
        world.insert_resource(Events::<HierarchyEvent>::default());

        let [a, b, c] = std::array::from_fn(|_| world.spawn_empty().id());

        world.entity_mut(a).add_child(b);

        assert_eq!(Some(&Parent(a)), world.get::<Parent>(b));
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
        assert_eq!(Some(&Parent(a)), world.get::<Parent>(c));
        assert_events(
            world,
            &[ChildAdded {
                child: c,
                parent: a,
            }],
        );
    }

    #[test]
    fn add_children() {
        let world = &mut World::new();
        world.insert_resource(Events::<HierarchyEvent>::default());

        let [a, b, c, d, e] = std::array::from_fn(|_| world.spawn_empty().id());

        world.entity_mut(a).add_children(&[]);
        assert_children(world, a, None);

        world.entity_mut(a).add_children(&[b, c]);

        assert_children(world, a, Some(&[b, c]));
        assert_eq!(world.get::<Parent>(b), Some(&Parent(a)));
        assert_eq!(world.get::<Parent>(c), Some(&Parent(a)));
        assert_events(
            world,
            &[
                ChildAdded {
                    child: b,
                    parent: a,
                },
                ChildAdded {
                    child: c,
                    parent: a,
                },
            ],
        );

        world.entity_mut(d).add_children(&[b, e, c]);

        assert_children(world, d, Some(&[b, e, c]));
        assert_eq!(world.get::<Parent>(b), Some(&Parent(d)));
        assert_eq!(world.get::<Parent>(e), Some(&Parent(d)));
        assert_eq!(world.get::<Parent>(c), Some(&Parent(d)));
        assert_children(world, a, None);
        assert_events(
            world,
            &[
                ChildMoved {
                    child: b,
                    previous_parent: a,
                    new_parent: d,
                },
                ChildAdded {
                    child: e,
                    parent: d,
                },
                ChildMoved {
                    child: c,
                    previous_parent: a,
                    new_parent: d,
                },
            ],
        );
    }

    #[test]
    fn insert_child() {
        let world = &mut World::new();
        world.insert_resource(Events::<HierarchyEvent>::default());

        let [a, b, c, d] = std::array::from_fn(|_| world.spawn_empty().id());

        world.entity_mut(a).insert_child(5, b);

        assert_children(world, a, Some(&[b]));
        assert_eq!(world.get::<Parent>(b), Some(&Parent(a)));
        assert_events(
            world,
            &[ChildAdded {
                child: b,
                parent: a,
            }],
        );

        world.entity_mut(a).insert_child(0, c);

        assert_children(world, a, Some(&[c, b]));
        assert_eq!(world.get::<Parent>(c), Some(&Parent(a)));
        assert_eq!(world.get::<Parent>(b), Some(&Parent(a)));
        assert_events(
            world,
            &[ChildAdded {
                child: c,
                parent: a,
            }],
        );

        world.entity_mut(d).insert_child(0, b).insert_child(0, c);
        assert_children(world, a, None);
    }

    #[test]
    fn insert_children() {
        let world = &mut World::new();
        world.insert_resource(Events::<HierarchyEvent>::default());

        let [a, b, c, d, e] = std::array::from_fn(|_| world.spawn_empty().id());

        world.entity_mut(a).insert_children(0, &[]);
        assert_children(world, a, None);

        world.entity_mut(a).insert_children(5, &[b, c]);

        assert_children(world, a, Some(&[b, c]));
        assert_eq!(world.get::<Parent>(b), Some(&Parent(a)));
        assert_eq!(world.get::<Parent>(c), Some(&Parent(a)));
        assert_events(
            world,
            &[
                ChildAdded {
                    child: b,
                    parent: a,
                },
                ChildAdded {
                    child: c,
                    parent: a,
                },
            ],
        );

        world.entity_mut(a).insert_children(1, &[d]);

        assert_children(world, a, Some(&[b, d, c]));
        assert_eq!(world.get::<Parent>(b), Some(&Parent(a)));
        assert_eq!(world.get::<Parent>(d), Some(&Parent(a)));
        assert_eq!(world.get::<Parent>(c), Some(&Parent(a)));
        assert_events(
            world,
            &[ChildAdded {
                child: d,
                parent: a,
            }],
        );

        world.entity_mut(e).insert_children(0, &[b, c, d]);
        assert_children(world, a, None);
    }

    #[test]
    fn remove_child() {
        let world = &mut World::new();
        world.insert_resource(Events::<HierarchyEvent>::default());

        let [a, b, c, e] = std::array::from_fn(|_| world.spawn_empty().id());

        world.entity_mut(a).add_children(&[b, c]).remove_child(b);

        assert_children(world, a, Some(&[c]));
        assert_eq!(world.get::<Parent>(c), Some(&Parent(a)));
        assert_eq!(world.get::<Parent>(b), None);
        omit_events(world, 2); // Omit ChildAdded events.
        assert_events(
            world,
            &[ChildRemoved {
                child: b,
                parent: a,
            }],
        );

        world.entity_mut(e).remove_child(c);
        assert_eq!(world.get::<Parent>(c), Some(&Parent(a)));

        world.entity_mut(a).remove_child(c);
        assert_children(world, a, None);
    }

    #[test]
    fn remove_children() {
        let world = &mut World::new();
        world.insert_resource(Events::<HierarchyEvent>::default());

        let [a, b, c, d, e] = std::array::from_fn(|_| world.spawn_empty().id());

        world
            .entity_mut(a)
            .add_children(&[b, c, d])
            .remove_children(&[b, d]);

        assert_children(world, a, Some(&[c]));
        assert_eq!(world.get::<Parent>(c), Some(&Parent(a)));
        assert_eq!(world.get::<Parent>(b), None);
        assert_eq!(world.get::<Parent>(d), None);
        omit_events(world, 3); // Omit ChildAdded events.
        assert_events(
            world,
            &[
                ChildRemoved {
                    child: b,
                    parent: a,
                },
                ChildRemoved {
                    child: d,
                    parent: a,
                },
            ],
        );

        world.entity_mut(e).remove_children(&[c]);
        assert_eq!(world.get::<Parent>(c), Some(&Parent(a)));

        world.entity_mut(a).remove_children(&[c]);
        assert_children(world, a, None);
    }

    #[test]
    fn clear_children() {
        let world = &mut World::new();
        world.insert_resource(Events::<HierarchyEvent>::default());

        let [a, b, c] = std::array::from_fn(|_| world.spawn_empty().id());

        world.entity_mut(a).add_children(&[b, c]).clear_children();

        assert_children(world, a, None);
        assert_eq!(world.get::<Parent>(b), None);
        assert_eq!(world.get::<Parent>(c), None);
        omit_events(world, 2); // Omit ChildAdded events.
        assert_events(
            world,
            &[
                ChildRemoved {
                    child: b,
                    parent: a,
                },
                ChildRemoved {
                    child: c,
                    parent: a,
                },
            ],
        );
    }

    #[test]
    fn set_parent() {
        let world = &mut World::new();
        world.insert_resource(Events::<HierarchyEvent>::default());

        let [a, b, c] = std::array::from_fn(|_| world.spawn_empty().id());

        world.entity_mut(a).set_parent(b);

        assert_eq!(world.get::<Parent>(a), Some(&Parent(b)));
        assert_children(world, b, Some(&[a]));
        assert_events(
            world,
            &[ChildAdded {
                child: a,
                parent: b,
            }],
        );

        world.entity_mut(a).set_parent(c);

        assert_eq!(world.get::<Parent>(a), Some(&Parent(c)),);
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

    #[test]
    fn remove_parent() {
        let world = &mut World::new();
        world.insert_resource(Events::<HierarchyEvent>::default());

        let [a, b, c] = std::array::from_fn(|_| world.spawn_empty().id());

        world.entity_mut(a).add_children(&[b, c]);
        world.entity_mut(b).remove_parent();

        assert_eq!(world.get::<Parent>(b), None,);
        assert_eq!(world.get::<Parent>(c), Some(&Parent(a)),);
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
        assert_eq!(world.get::<Parent>(c), None);
        assert_children(world, a, None);
        assert_events(
            world,
            &[ChildRemoved {
                child: c,
                parent: a,
            }],
        );
    }

    #[test]
    fn with_children() {
        let mut world = World::default();
        let mut queue = CommandQueue::default();
        let mut commands = Commands::new(&mut queue, &world);

        let mut children = Vec::new();
        let parent = commands
            .spawn_empty()
            .with_children(|parent| {
                children.extend([
                    parent.spawn_empty().id(),
                    parent.spawn_empty().id(),
                    parent.spawn_empty().id(),
                ]);
            })
            .id();

        queue.apply(&mut world);
        assert_children(&mut world, parent, Some(&children));
        for child in children {
            assert_eq!(world.get::<Parent>(child), Some(&Parent(parent)));
        }
    }

    #[test]
    fn push_and_insert_and_remove_children() {
        let world = &mut World::new();
        let [parent, child1, child2, child3, child4] =
            std::array::from_fn(|_| world.spawn_empty().id());

        run_system(world, move |mut commands: Commands| {
            commands.entity(parent).add_children(&[child1, child2]);
        });
        assert_children(world, parent, Some(&[child1, child2]));
        assert_eq!(world.get::<Parent>(child1), Some(&Parent(parent)));
        assert_eq!(world.get::<Parent>(child2), Some(&Parent(parent)));

        run_system(world, move |mut commands: Commands| {
            commands
                .entity(parent)
                .insert_children(1, &[child3, child4]);
        });
        assert_children(world, parent, Some(&[child1, child3, child4, child2]));
        assert_eq!(world.get::<Parent>(child3), Some(&Parent(parent)));
        assert_eq!(world.get::<Parent>(child4), Some(&Parent(parent)));

        run_system(world, move |mut commands: Commands| {
            commands.entity(parent).remove_children(&[child1, child4]);
        });
        assert_children(world, parent, Some(&[child3, child2]));
        assert_eq!(world.get::<Parent>(child1), None);
        assert_eq!(world.get::<Parent>(child4), None);
    }

    /// Tests what happens when all children are removed from a parent
    #[test]
    fn children_removed_when_empty() {
        let world = &mut World::new();
        let [parent1, parent2, child] = std::array::from_fn(|_| world.spawn_empty().id());

        // push child into parent1 with `add_child`
        run_system(world, move |mut commands: Commands| {
            commands.entity(parent1).add_child(child);
        });
        assert_children(world, parent1, Some(&[child]));

        // move only child from parent1 with `add_children`
        run_system(world, move |mut commands: Commands| {
            commands.entity(parent2).add_children(&[child]);
        });
        assert_children(world, parent1, None);

        // move only child from parent2 with `insert_child`
        run_system(world, move |mut commands: Commands| {
            commands.entity(parent1).insert_child(0, child);
        });
        assert_children(world, parent2, None);

        // move only child from parent1 with `add_child`
        run_system(world, move |mut commands: Commands| {
            commands.entity(parent2).add_child(child);
        });
        assert_children(world, parent1, None);

        // remove only child from parent2 with `remove_child`
        run_system(world, move |mut commands: Commands| {
            commands.entity(parent2).remove_child(child);
        });
        assert_children(world, parent2, None);
    }
}

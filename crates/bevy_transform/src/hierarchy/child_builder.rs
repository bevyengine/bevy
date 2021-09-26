use crate::prelude::{Children, Parent, PreviousParent};
use bevy_ecs::{
    bundle::Bundle,
    entity::Entity,
    system::{Command, Commands, EntityCommands},
    world::{EntityMut, World},
};
use smallvec::SmallVec;

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

#[derive(Debug)]
pub struct PushChildren {
    parent: Entity,
    children: SmallVec<[Entity; 8]>,
}

pub struct ChildBuilder<'w, 's, 'a> {
    commands: &'a mut Commands<'w, 's>,
    push_children: PushChildren,
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

impl<'w, 's, 'a> ChildBuilder<'w, 's, 'a> {
    pub fn spawn_bundle(&mut self, bundle: impl Bundle) -> EntityCommands<'w, 's, '_> {
        let e = self.commands.spawn_bundle(bundle);
        self.push_children.children.push(e.id());
        e
    }

    pub fn spawn(&mut self) -> EntityCommands<'w, 's, '_> {
        let e = self.commands.spawn();
        self.push_children.children.push(e.id());
        e
    }

    pub fn parent_entity(&self) -> Entity {
        self.push_children.parent
    }

    pub fn add_command<C: Command + 'static>(&mut self, command: C) -> &mut Self {
        self.commands.add(command);
        self
    }
}

pub trait BuildChildren {
    fn with_children(&mut self, f: impl FnOnce(&mut ChildBuilder)) -> &mut Self;
    fn push_children(&mut self, children: &[Entity]) -> &mut Self;
    fn insert_children(&mut self, index: usize, children: &[Entity]) -> &mut Self;
}

impl<'w, 's, 'a> BuildChildren for EntityCommands<'w, 's, 'a> {
    fn with_children(&mut self, spawn_children: impl FnOnce(&mut ChildBuilder)) -> &mut Self {
        let parent = self.id();
        let push_children = {
            let mut builder = ChildBuilder {
                commands: self.commands(),
                push_children: PushChildren {
                    children: SmallVec::default(),
                    parent,
                },
            };
            spawn_children(&mut builder);
            builder.push_children
        };

        self.commands().add(push_children);
        self
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
}

#[derive(Debug)]
pub struct WorldChildBuilder<'w> {
    world: &'w mut World,
    current_entity: Option<Entity>,
    parent_entities: Vec<Entity>,
}

impl<'w> WorldChildBuilder<'w> {
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

    pub fn parent_entity(&self) -> Entity {
        self.parent_entities
            .last()
            .cloned()
            .expect("There should always be a parent at this point.")
    }
}

pub trait BuildWorldChildren {
    fn with_children(&mut self, spawn_children: impl FnOnce(&mut WorldChildBuilder)) -> &mut Self;
    fn push_children(&mut self, children: &[Entity]) -> &mut Self;
    fn insert_children(&mut self, index: usize, children: &[Entity]) -> &mut Self;
}

impl<'w> BuildWorldChildren for EntityMut<'w> {
    fn with_children(&mut self, spawn_children: impl FnOnce(&mut WorldChildBuilder)) -> &mut Self {
        {
            let entity = self.id();
            let mut builder = WorldChildBuilder {
                current_entity: None,
                parent_entities: vec![entity],
                // SAFE: self.update_location() is called below. It is impossible to make EntityMut
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
            // SAFE: parent entity is not modified and its location is updated manually
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
            // SAFE: parent entity is not modified and its location is updated manually
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
}

#[cfg(test)]
mod tests {
    use super::{BuildChildren, BuildWorldChildren};
    use crate::prelude::{Children, Parent, PreviousParent};
    use bevy_ecs::{
        entity::Entity,
        system::{CommandQueue, Commands},
        world::World,
    };
    use smallvec::{smallvec, SmallVec};

    #[test]
    fn build_children() {
        let mut world = World::default();
        let mut queue = CommandQueue::default();
        let mut commands = Commands::new(&mut queue, &world);

        let mut children = Vec::new();
        let parent = commands.spawn().insert(1).id();
        commands.entity(parent).with_children(|parent| {
            children.push(parent.spawn().insert(2).id());
            children.push(parent.spawn().insert(3).id());
            children.push(parent.spawn().insert(4).id());
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
    fn push_and_insert_children_commands() {
        let mut world = World::default();

        let entities = world
            .spawn_batch(vec![(1,), (2,), (3,), (4,), (5,)])
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
    }

    #[test]
    fn push_and_insert_children_world() {
        let mut world = World::default();

        let entities = world
            .spawn_batch(vec![(1,), (2,), (3,), (4,), (5,)])
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
    }

    #[test]
    fn regression_push_children_same_archetype() {
        let mut world = World::new();
        let child = world.spawn().id();
        world.spawn().push_children(&[child]);
    }
}

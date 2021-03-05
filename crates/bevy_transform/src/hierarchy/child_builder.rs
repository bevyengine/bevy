use crate::prelude::{Children, Parent, PreviousParent};
use bevy_ecs::{
    bundle::Bundle,
    component::Component,
    entity::Entity,
    system::{Command, Commands},
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
    fn write(self: Box<Self>, world: &mut World) {
        for child in self.children.iter() {
            world
                .entity_mut(*child)
                .insert_bundle((Parent(self.parent), PreviousParent(self.parent)));
        }
        {
            let mut added = false;
            if let Some(mut children) = world.get_mut::<Children>(self.parent) {
                children.0.insert_from_slice(self.index, &self.children);
                added = true;
            }

            // NOTE: ideally this is just an else statement, but currently that _incorrectly_ fails borrow-checking
            if !added {
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

pub struct ChildBuilder<'a, 'b> {
    commands: &'a mut Commands<'b>,
    push_children: PushChildren,
}

impl Command for PushChildren {
    fn write(self: Box<Self>, world: &mut World) {
        for child in self.children.iter() {
            world
                .entity_mut(*child)
                .insert_bundle((Parent(self.parent), PreviousParent(self.parent)));
        }
        {
            let mut added = false;
            if let Some(mut children) = world.get_mut::<Children>(self.parent) {
                children.0.extend(self.children.iter().cloned());
                added = true;
            }

            // NOTE: ideally this is just an else statement, but currently that _incorrectly_ fails borrow-checking
            if !added {
                world
                    .entity_mut(self.parent)
                    .insert(Children(self.children));
            }
        }
    }
}

impl<'a, 'b> ChildBuilder<'a, 'b> {
    pub fn spawn(&mut self, bundle: impl Bundle) -> &mut Self {
        self.commands.spawn(bundle);
        self.push_children
            .children
            .push(self.commands.current_entity().unwrap());
        self
    }

    pub fn current_entity(&self) -> Option<Entity> {
        self.commands.current_entity()
    }

    pub fn parent_entity(&self) -> Entity {
        self.push_children.parent
    }

    pub fn with_bundle(&mut self, bundle: impl Bundle) -> &mut Self {
        self.commands.with_bundle(bundle);
        self
    }

    pub fn with(&mut self, component: impl Component) -> &mut Self {
        self.commands.with(component);
        self
    }

    pub fn for_current_entity(&mut self, func: impl FnOnce(Entity)) -> &mut Self {
        let current_entity = self
            .commands
            .current_entity()
            .expect("The 'current entity' is not set. You should spawn an entity first.");
        func(current_entity);
        self
    }

    pub fn add_command<C: Command + 'static>(&mut self, command: C) -> &mut Self {
        self.commands.add_command(command);
        self
    }
}

pub trait BuildChildren {
    fn with_children(&mut self, f: impl FnOnce(&mut ChildBuilder)) -> &mut Self;
    fn push_children(&mut self, parent: Entity, children: &[Entity]) -> &mut Self;
    fn insert_children(&mut self, parent: Entity, index: usize, children: &[Entity]) -> &mut Self;
}

impl<'a> BuildChildren for Commands<'a> {
    fn with_children(&mut self, parent: impl FnOnce(&mut ChildBuilder)) -> &mut Self {
        let current_entity = self.current_entity().expect("Cannot add children because the 'current entity' is not set. You should spawn an entity first.");
        self.clear_current_entity();
        let push_children = {
            let mut builder = ChildBuilder {
                commands: self,
                push_children: PushChildren {
                    children: SmallVec::default(),
                    parent: current_entity,
                },
            };
            parent(&mut builder);
            builder.push_children
        };

        self.set_current_entity(current_entity);
        self.add_command(push_children);
        self
    }

    fn push_children(&mut self, parent: Entity, children: &[Entity]) -> &mut Self {
        self.add_command(PushChildren {
            children: SmallVec::from(children),
            parent,
        });
        self
    }

    fn insert_children(&mut self, parent: Entity, index: usize, children: &[Entity]) -> &mut Self {
        self.add_command(InsertChildren {
            children: SmallVec::from(children),
            index,
            parent,
        });
        self
    }
}

impl<'a, 'b> BuildChildren for ChildBuilder<'a, 'b> {
    fn with_children(&mut self, spawn_children: impl FnOnce(&mut ChildBuilder)) -> &mut Self {
        let current_entity = self.commands.current_entity().expect("Cannot add children because the 'current entity' is not set. You should spawn an entity first.");
        self.commands.clear_current_entity();
        let push_children = {
            let mut builder = ChildBuilder {
                commands: self.commands,
                push_children: PushChildren {
                    children: SmallVec::default(),
                    parent: current_entity,
                },
            };

            spawn_children(&mut builder);
            builder.push_children
        };

        self.commands.set_current_entity(current_entity);
        self.commands.add_command(push_children);
        self
    }

    fn push_children(&mut self, parent: Entity, children: &[Entity]) -> &mut Self {
        self.commands.add_command(PushChildren {
            children: SmallVec::from(children),
            parent,
        });
        self
    }

    fn insert_children(&mut self, parent: Entity, index: usize, children: &[Entity]) -> &mut Self {
        self.commands.add_command(InsertChildren {
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
    pub fn spawn(&mut self, bundle: impl Bundle + Send + Sync + 'static) -> &mut Self {
        let parent_entity = self
            .parent_entities
            .last()
            .cloned()
            .expect("There should always be a parent at this point.");
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
        self
    }

    pub fn with_bundle(&mut self, bundle: impl Bundle + Send + Sync + 'static) -> &mut Self {
        self.world
            .entity_mut(self.current_entity.unwrap())
            .insert_bundle(bundle);
        self
    }

    pub fn with(&mut self, component: impl Component) -> &mut Self {
        self.world
            .entity_mut(self.current_entity.unwrap())
            .insert(component);
        self
    }

    pub fn current_entity(&self) -> Option<Entity> {
        self.current_entity
    }

    pub fn for_current_entity(&mut self, func: impl FnOnce(Entity)) -> &mut Self {
        let current_entity = self
            .current_entity()
            .expect("The 'current entity' is not set. You should spawn an entity first.");
        func(current_entity);
        self
    }
}

pub trait BuildWorldChildren {
    fn with_children(&mut self, spawn_children: impl FnOnce(&mut WorldChildBuilder)) -> &mut Self;
}

impl<'w> BuildWorldChildren for EntityMut<'w> {
    fn with_children(&mut self, spawn_children: impl FnOnce(&mut WorldChildBuilder)) -> &mut Self {
        {
            let entity = self.id();
            let mut builder = WorldChildBuilder {
                current_entity: None,
                parent_entities: vec![entity],
                // SAFE: self.update_location() is called below. It is impossible to make EntityMut function calls on `self`
                // within the scope defined here
                world: unsafe { self.world_mut() },
            };

            spawn_children(&mut builder);
        }
        self.update_location();
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
}

#[cfg(test)]
mod tests {
    use super::BuildChildren;
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

        let mut parent = None;
        let mut child1 = None;
        let mut child2 = None;
        let mut child3 = None;

        commands
            .spawn((1,))
            .for_current_entity(|e| parent = Some(e))
            .with_children(|parent| {
                parent
                    .spawn((2,))
                    .for_current_entity(|e| child1 = Some(e))
                    .spawn((3,))
                    .for_current_entity(|e| child2 = Some(e))
                    .spawn((4,));

                child3 = parent.current_entity();
            });

        queue.apply(&mut world);
        let parent = parent.expect("parent should exist");
        let child1 = child1.expect("child1 should exist");
        let child2 = child2.expect("child2 should exist");
        let child3 = child3.expect("child3 should exist");
        let expected_children: SmallVec<[Entity; 8]> = smallvec![child1, child2, child3];

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
    }

    #[test]
    fn push_and_insert_children() {
        let mut world = World::default();

        let entities = world
            .spawn_batch(vec![(1,), (2,), (3,), (4,), (5,)])
            .collect::<Vec<Entity>>();

        let mut queue = CommandQueue::default();
        {
            let mut commands = Commands::new(&mut queue, &world);
            commands.push_children(entities[0], &entities[1..3]);
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
            commands.insert_children(parent, 1, &entities[3..]);
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
}

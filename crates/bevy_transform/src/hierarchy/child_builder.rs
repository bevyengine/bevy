use crate::prelude::{Children, Parent, PreviousParent};
use bevy_ecs::{Command, Commands, Component, DynamicBundle, Entity, Resources, World};
use smallvec::SmallVec;

#[derive(Debug)]
pub struct InsertChildren {
    parent: Entity,
    children: SmallVec<[Entity; 8]>,
    index: usize,
}

impl Command for InsertChildren {
    fn write(self: Box<Self>, world: &mut World, _resources: &mut Resources) {
        for child in self.children.iter() {
            world
                .insert(*child, (Parent(self.parent), PreviousParent(self.parent)))
                .unwrap();
        }
        {
            let mut added = false;
            if let Ok(mut children) = world.get_mut::<Children>(self.parent) {
                children.0.insert_from_slice(self.index, &self.children);
                added = true;
            }

            // NOTE: ideally this is just an else statement, but currently that _incorrectly_ fails borrow-checking
            if !added {
                world
                    .insert_one(self.parent, Children(self.children))
                    .unwrap();
            }
        }
    }
}

#[derive(Debug)]
pub struct PushChildren {
    parent: Entity,
    children: SmallVec<[Entity; 8]>,
}

pub struct ChildBuilder<'a> {
    commands: &'a mut Commands,
    push_children: PushChildren,
}

impl Command for PushChildren {
    fn write(self: Box<Self>, world: &mut World, _resources: &mut Resources) {
        for child in self.children.iter() {
            world
                .insert(*child, (Parent(self.parent), PreviousParent(self.parent)))
                .unwrap();
        }
        {
            let mut added = false;
            if let Ok(mut children) = world.get_mut::<Children>(self.parent) {
                children.0.extend(self.children.iter().cloned());
                added = true;
            }

            // NOTE: ideally this is just an else statement, but currently that _incorrectly_ fails borrow-checking
            if !added {
                world
                    .insert_one(self.parent, Children(self.children))
                    .unwrap();
            }
        }
    }
}

impl<'a> ChildBuilder<'a> {
    pub fn spawn(&mut self, components: impl DynamicBundle + Send + Sync + 'static) -> &mut Self {
        self.commands.spawn(components);
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

    pub fn with_bundle(
        &mut self,
        components: impl DynamicBundle + Send + Sync + 'static,
    ) -> &mut Self {
        self.commands.with_bundle(components);
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

impl BuildChildren for Commands {
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

impl<'a> BuildChildren for ChildBuilder<'a> {
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

#[cfg(test)]
mod tests {
    use super::BuildChildren;
    use crate::prelude::{Children, Parent, PreviousParent};
    use bevy_ecs::{Commands, Entity, Resources, World};
    use smallvec::{smallvec, SmallVec};

    #[test]
    fn build_children() {
        let mut world = World::default();
        let mut resources = Resources::default();
        let mut commands = Commands::default();
        commands.set_entity_reserver(world.get_entity_reserver());

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

        commands.apply(&mut world, &mut resources);
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
        let mut resources = Resources::default();
        let mut commands = Commands::default();
        let entities = world
            .spawn_batch(vec![(1,), (2,), (3,), (4,), (5,)])
            .collect::<Vec<Entity>>();

        commands.push_children(entities[0], &entities[1..3]);
        commands.apply(&mut world, &mut resources);

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

        commands.insert_children(parent, 1, &entities[3..]);
        commands.apply(&mut world, &mut resources);

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

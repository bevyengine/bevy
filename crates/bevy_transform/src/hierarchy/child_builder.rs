use crate::prelude::{Children, Parent, PreviousParent};
use bevy_ecs::{Command, Commands, Component, DynamicBundle, Entity, Resources, World};
use bevy_utils::HashSet;
use smallvec::SmallVec;

#[derive(Debug)]
pub struct InsertChildren {
    parent: Entity,
    children: SmallVec<[Entity; 8]>,
    index: usize,
}

impl Command for InsertChildren {
    fn write(self: Box<Self>, world: &mut World, _resources: &mut Resources) {
        let mut childset = HashSet::default();

        let mut new_children = if let Ok(children) = world.get::<Children>(self.parent) {
            childset.extend(children.iter().copied());
            children.0.clone()
        } else {
            Default::default()
        };
        /*
        old_children
        new_children
        self.parent
        self.index
        self.children*/

        for (offset, child) in self.children.iter().enumerate() {
            if !childset.contains(child) {
                if self.index >= new_children.len() {
                    new_children.push(*child);
                } else {
                    new_children.insert(self.index + offset, *child);
                }
            }

            if let Ok(Parent(old_parent)) = world.get::<Parent>(*child) {
                let old_parent = *old_parent;

                // clean old parent of children references
                if let Ok(mut children) = world.get_mut::<Children>(old_parent) {
                    let vec = children.iter().copied().filter(|c| c != child).collect();
                    children.0 = vec;
                }

                world
                    .insert(*child, (Parent(self.parent), PreviousParent(old_parent)))
                    .unwrap();
            } else {
                world
                    .insert(*child, (Parent(self.parent), PreviousParent(self.parent)))
                    .unwrap();
            }
        }

        world
            .insert_one(self.parent, Children(new_children))
            .unwrap();
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
        let mut childset = HashSet::default();

        let mut new_children = if let Ok(children) = world.get::<Children>(self.parent) {
            childset.extend(children.iter().copied());
            children.0.clone()
        } else {
            Default::default()
        };

        for child in self.children.iter() {
            if !childset.contains(child) {
                new_children.push(*child);
            }

            if let Ok(Parent(old_parent)) = world.get::<Parent>(*child) {
                let old_parent = *old_parent;

                // clean old parent of children references
                if let Ok(mut children) = world.get_mut::<Children>(old_parent) {
                    let vec = children
                        .iter()
                        .filter_map(|c| if c != child { Some(*c) } else { None })
                        .collect();
                    children.0 = vec;
                }

                world
                    .insert(*child, (Parent(self.parent), PreviousParent(old_parent)))
                    .unwrap();
            } else {
                world
                    .insert(*child, (Parent(self.parent), PreviousParent(self.parent)))
                    .unwrap();
            }
        }

        world
            .insert_one(self.parent, Children(self.children))
            .unwrap();
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

    fn setup() -> (World, Resources, Commands, Vec<Entity>, Entity) {
        let mut world = World::default();
        let resources = Resources::default();
        let commands = Commands::default();
        let entities = world
            .spawn_batch(vec![(0,), (1,), (2,), (3,), (4,)])
            .collect::<Vec<Entity>>();
        let parent = entities[0];
        (world, resources, commands, entities, parent)
    }

    // push_children_adds_parent_component
    // push_children_adds_previous_parent_component
    // push_children_adds_children_component
    // push_children_keeps_children_unique
    // push_children_updates_previous_parent

    // insert_children_adds_parent_component
    // insert_children_adds_previous_parent_component
    // insert_children_adds_children_component
    // insert_children_keeps_children_unique
    // insert_children_updates_previous_parent
    // insert_children_keeps_children_order
    // insert_children_out_of_bounds_pushes_to_end

    #[test]
    fn push_children_adds_parent_component() {
        let (mut world, mut resources, mut commands, child, parent) = setup();
        commands.push_children(parent, &child[1..=2]);
        commands.apply(&mut world, &mut resources);
        assert_eq!(world.get::<Parent>(child[2]).unwrap(), &Parent(parent));
    }

    #[test]
    fn push_children_adds_previous_parent_component() {
        let (mut world, mut resources, mut commands, child, parent) = setup();
        commands.push_children(parent, &child[1..=2]);
        commands.apply(&mut world, &mut resources);
        assert_eq!(
            world.get::<PreviousParent>(child[2]).unwrap(),
            &PreviousParent(parent)
        );
    }

    #[test]
    fn push_children_adds_children_component() {
        let (mut world, mut resources, mut commands, child, parent) = setup();
        commands.push_children(parent, &child[1..=2]);
        commands.apply(&mut world, &mut resources);
        assert_eq!(
            world.get::<Children>(parent).unwrap(),
            &Children::with(&child[1..=2])
        );
    }

    #[test]
    fn push_children_keeps_children_unique() {
        let (mut world, mut resources, mut commands, child, parent) = setup();
        commands.push_children(parent, &child[1..=2]);
        commands.apply(&mut world, &mut resources);
        commands.push_children(parent, &child[1..=2]);
        commands.apply(&mut world, &mut resources);
        assert_eq!(
            world.get::<Children>(parent).unwrap(),
            &Children::with(&child[1..=2])
        );
    }

    #[test]
    fn push_children_updates_previous_parent() {
        let (mut world, mut resources, mut commands, entities, parent1) = setup();
        let parent2 = entities[4];
        let child = entities[1];
        commands.push_children(parent1, &[child]);
        commands.apply(&mut world, &mut resources);
        commands.push_children(parent2, &[child]);
        commands.apply(&mut world, &mut resources);
        assert_eq!(world.get::<Parent>(child).unwrap(), &Parent(parent2));
        assert_eq!(
            world.get::<PreviousParent>(child).unwrap(),
            &PreviousParent(parent1)
        );
    }

    #[test]
    fn insert_children_adds_parent_component() {
        let (mut world, mut resources, mut commands, child, parent) = setup();
        commands.insert_children(parent, 0, &child[1..=2]);
        commands.apply(&mut world, &mut resources);
        assert_eq!(world.get::<Parent>(child[2]).unwrap(), &Parent(parent));
    }

    #[test]
    fn insert_children_adds_previous_parent_component() {
        let (mut world, mut resources, mut commands, child, parent) = setup();
        commands.insert_children(parent, 0, &child[1..=2]);
        commands.apply(&mut world, &mut resources);
        assert_eq!(
            world.get::<PreviousParent>(child[2]).unwrap(),
            &PreviousParent(parent)
        );
    }

    #[test]
    fn insert_children_adds_children_component() {
        let (mut world, mut resources, mut commands, child, parent) = setup();
        commands.insert_children(parent, 0, &child[1..=2]);
        commands.apply(&mut world, &mut resources);
        assert_eq!(
            world.get::<Children>(parent).unwrap(),
            &Children::with(&child[1..=2])
        );
    }

    #[test]
    fn insert_children_keeps_children_unique() {
        let (mut world, mut resources, mut commands, child, parent) = setup();
        commands.insert_children(parent, 0, &child[1..=2]);
        commands.apply(&mut world, &mut resources);
        commands.insert_children(parent, 1, &child[1..=2]);
        commands.apply(&mut world, &mut resources);
        assert_eq!(
            world.get::<Children>(parent).unwrap(),
            &Children::with(&child[1..=2])
        );
    }

    #[test]
    fn insert_children_updates_previous_parent() {
        let (mut world, mut resources, mut commands, entities, parent1) = setup();
        let parent2 = entities[4];
        let child = entities[1];
        commands.insert_children(parent1, 0, &[child]);
        commands.apply(&mut world, &mut resources);
        commands.insert_children(parent2, 0, &[child]);
        commands.apply(&mut world, &mut resources);
        assert_eq!(world.get::<Parent>(child).unwrap(), &Parent(parent2));
        assert_eq!(
            world.get::<PreviousParent>(child).unwrap(),
            &PreviousParent(parent1)
        );
    }

    #[test]
    fn insert_children_keeps_children_order() {
        let (mut world, mut resources, mut commands, child, parent) = setup();
        commands.insert_children(parent, 0, &child[1..=2]);
        commands.apply(&mut world, &mut resources);
        assert_eq!(
            world.get::<Children>(parent).unwrap(),
            &Children::with(&[child[1], child[2]])
        );
        commands.insert_children(parent, 1, &child[3..=4]);
        commands.apply(&mut world, &mut resources);
        assert_eq!(
            world.get::<Children>(parent).unwrap(),
            &Children::with(&[child[1], child[3], child[4], child[2]])
        );
    }

    #[test]
    fn insert_children_out_of_bounds_pushes_to_end() {
        let (mut world, mut resources, mut commands, child, parent) = setup();
        commands.insert_children(parent, 0, &child[1..=2]);
        commands.apply(&mut world, &mut resources);
        commands.insert_children(parent, 999, &child[3..=4]);
        commands.apply(&mut world, &mut resources);
        assert_eq!(
            world.get::<Children>(parent).unwrap(),
            &Children::with(&[child[1], child[2], child[3], child[4]])
        );
    }
}

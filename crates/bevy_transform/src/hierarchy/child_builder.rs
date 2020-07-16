use crate::prelude::{LocalTransform, Parent};
use bevy_ecs::{Commands, CommandsInternal, Component, DynamicBundle, Entity};

pub struct ChildBuilder<'a> {
    commands: &'a mut CommandsInternal,
    parent_entities: Vec<Entity>,
}

impl<'a> ChildBuilder<'a> {
    pub fn spawn(&mut self, components: impl DynamicBundle + Send + Sync + 'static) -> &mut Self {
        self.spawn_as_entity(Entity::new(), components)
    }

    pub fn spawn_as_entity(
        &mut self,
        entity: Entity,
        components: impl DynamicBundle + Send + Sync + 'static,
    ) -> &mut Self {
        let parent_entity = self
            .parent_entities
            .last()
            .cloned()
            .expect("There should always be a parent at this point.");
        self.commands
            .spawn_as_entity(entity, components)
            .with_bundle((Parent(parent_entity), LocalTransform::default()));
        self
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
}

pub trait BuildChildren {
    fn with_children(&mut self, spawn_children: impl FnMut(&mut ChildBuilder)) -> &mut Self;
}

impl BuildChildren for Commands {
    fn with_children(&mut self, mut spawn_children: impl FnMut(&mut ChildBuilder)) -> &mut Self {
        {
            let mut commands = self.commands.lock().unwrap();
            let current_entity = commands.current_entity.expect("Cannot add children because the 'current entity' is not set. You should spawn an entity first.");
            let mut builder = ChildBuilder {
                commands: &mut commands,
                parent_entities: vec![current_entity],
            };

            spawn_children(&mut builder);
        }
        self
    }
}

impl<'a> BuildChildren for ChildBuilder<'a> {
    fn with_children(&mut self, mut spawn_children: impl FnMut(&mut ChildBuilder)) -> &mut Self {
        let current_entity = self
            .commands
            .current_entity
            .expect("Cannot add children without a parent. Try creating an entity first.");
        self.parent_entities.push(current_entity);
        self.commands.current_entity = None;

        spawn_children(self);

        self.commands.current_entity = self.parent_entities.pop();
        self
    }
}

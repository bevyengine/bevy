use crate::prelude::{Children, Parent, PreviousParent};
use bevy_ecs::{Component, DynamicBundle, Entity, WorldBuilder};

#[derive(Debug)]
pub struct WorldChildBuilder<'a, 'b> {
    world_builder: &'b mut WorldBuilder<'a>,
    parent_entities: Vec<Entity>,
}

impl<'a, 'b> WorldChildBuilder<'a, 'b> {
    pub fn spawn(&mut self, components: impl DynamicBundle + Send + Sync + 'static) -> &mut Self {
        let parent_entity = self
            .parent_entities
            .last()
            .cloned()
            .expect("There should always be a parent at this point.");
        self.world_builder
            .spawn(components)
            .with_bundle((Parent(parent_entity), PreviousParent(Some(parent_entity))));
        let entity = self.world_builder.current_entity.unwrap();
        {
            let world = &mut self.world_builder.world;
            let mut added = false;
            if let Ok(mut children) = world.get_mut::<Children>(parent_entity) {
                children.push(entity);
                added = true;
            }

            // NOTE: ideally this is just an else statement, but currently that _incorrectly_ fails borrow-checking
            if !added {
                world
                    .insert_one(parent_entity, Children(smallvec::smallvec![entity]))
                    .unwrap();
            }
        }
        self
    }

    pub fn with_bundle(
        &mut self,
        components: impl DynamicBundle + Send + Sync + 'static,
    ) -> &mut Self {
        self.world_builder.with_bundle(components);
        self
    }

    pub fn with(&mut self, component: impl Component) -> &mut Self {
        self.world_builder.with(component);
        self
    }
}

pub trait BuildWorldChildren {
    fn with_children(&mut self, spawn_children: impl FnOnce(&mut WorldChildBuilder)) -> &mut Self;
}

impl<'a> BuildWorldChildren for WorldBuilder<'a> {
    fn with_children(&mut self, spawn_children: impl FnOnce(&mut WorldChildBuilder)) -> &mut Self {
        {
            let current_entity = self.current_entity.expect("Cannot add children because the 'current entity' is not set. You should spawn an entity first.");
            let mut builder = WorldChildBuilder {
                world_builder: self,
                parent_entities: vec![current_entity],
            };

            spawn_children(&mut builder);
        }
        self
    }
}

impl<'a, 'b> BuildWorldChildren for WorldChildBuilder<'a, 'b> {
    fn with_children(&mut self, spawn_children: impl FnOnce(&mut WorldChildBuilder)) -> &mut Self {
        let current_entity = self
            .world_builder
            .current_entity
            .expect("Cannot add children without a parent. Try creating an entity first.");
        self.parent_entities.push(current_entity);
        self.world_builder.current_entity = None;

        spawn_children(self);

        self.world_builder.current_entity = self.parent_entities.pop();
        self
    }
}

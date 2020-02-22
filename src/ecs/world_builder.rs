use crate::ecs::EntityArchetype;
use legion::{
    filter::{ChunksetFilterData, Filter},
    prelude::*,
    world::{IntoComponentSource, TagLayout, TagSet},
};

pub trait WorldBuilderSource {
    fn build(&mut self) -> WorldBuilder;
}

impl WorldBuilderSource for World {
    fn build(&mut self) -> WorldBuilder {
        WorldBuilder {
            world: self,
            current_entity: None,
        }
    }
}

pub struct WorldBuilder<'a> {
    world: &'a mut World,
    current_entity: Option<Entity>,
}

impl<'a> WorldBuilder<'a> {
    pub fn build_entity(mut self) -> Self {
        let entity = *self.world.insert((), vec![()]).first().unwrap();
        self.current_entity = Some(entity);
        self
    }
    pub fn build(self) {}

    // note: this is slow and does a full entity copy
    pub fn add<T>(self, component: T) -> Self
    where
        T: legion::storage::Component,
    {
        let _ = self
            .world
            .add_component(*self.current_entity.as_ref().unwrap(), component);
        self
    }

    pub fn tag<T>(self, tag: T) -> Self
    where
        T: legion::storage::Tag,
    {
        let _ = self
            .world
            .add_tag(*self.current_entity.as_ref().unwrap(), tag);
        self
    }

    pub fn add_entities<T, C>(self, tags: T, components: C) -> Self
    where
        T: TagSet + TagLayout + for<'b> Filter<ChunksetFilterData<'b>>,
        C: IntoComponentSource,
    {
        self.world.insert(tags, components);
        self
    }

    pub fn add_archetype(mut self, entity_archetype: impl EntityArchetype) -> Self {
        self.current_entity = Some(entity_archetype.insert(self.world));
        self
    }
}

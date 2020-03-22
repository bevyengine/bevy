use crate::ecs::EntityArchetype;
use bevy_transform::components::{LocalToParent, Parent};
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
            parent_entity: None,
        }
    }
}

pub struct WorldBuilder<'a> {
    world: &'a mut World,
    current_entity: Option<Entity>,
    parent_entity: Option<Entity>,
}

impl<'a> WorldBuilder<'a> {
    pub fn build_entity(mut self) -> Self {
        let entity = *self.world.insert((), vec![()]).first().unwrap();
        self.current_entity = Some(entity);
        self.add_parent_to_current_entity();
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

    pub fn add_entity(mut self, entity_archetype: impl EntityArchetype) -> Self {
        let current_entity = entity_archetype.insert(self.world);
        self.current_entity = Some(current_entity);
        self.add_parent_to_current_entity();
        self
    }

    pub fn add_children(mut self, build_children: impl Fn(WorldBuilder) -> WorldBuilder) -> Self {
        self.parent_entity = self.current_entity;
        self.current_entity = None;

        self = build_children(self);

        self.current_entity = self.parent_entity;
        self.parent_entity = None;
        self
    }

    fn add_parent_to_current_entity(&mut self) {
        let current_entity = self.current_entity.unwrap();
        if let Some(parent_entity) = self.parent_entity {
            let _ = self
                .world
                .add_component(current_entity, Parent(parent_entity));
            let _ = self
                .world
                .add_component(current_entity, LocalToParent::identity());
        }
    }
}

pub trait CommandBufferBuilderSource {
    fn build(&mut self) -> CommandBufferBuilder;
}

impl CommandBufferBuilderSource for CommandBuffer {
    fn build(&mut self) -> CommandBufferBuilder {
        CommandBufferBuilder {
            command_buffer: self,
            current_entity: None,
            parent_entity: None,
        }
    }
}

pub struct CommandBufferBuilder<'a> {
    command_buffer: &'a mut CommandBuffer,
    current_entity: Option<Entity>,
    parent_entity: Option<Entity>,
}

impl<'a> CommandBufferBuilder<'a> {
    pub fn build_entity(mut self) -> Self {
        let entity = *self.command_buffer.insert((), vec![()]).first().unwrap();
        self.current_entity = Some(entity);
        self.add_parent_to_current_entity();
        self
    }
    pub fn build(self) {}

    // note: this is slow and does a full entity copy
    pub fn add<T>(self, component: T) -> Self
    where
        T: legion::storage::Component,
    {
        let _ = self
            .command_buffer
            .add_component(*self.current_entity.as_ref().unwrap(), component);
        self
    }

    pub fn tag<T>(self, tag: T) -> Self
    where
        T: legion::storage::Tag,
    {
        let _ = self
            .command_buffer
            .add_tag(*self.current_entity.as_ref().unwrap(), tag);
        self
    }

    pub fn add_entities<T, C>(self, tags: T, components: C) -> Self
    where
        T: TagSet + TagLayout + for<'b> Filter<ChunksetFilterData<'b>> + 'static,
        C: IntoComponentSource + 'static,
    {
        self.command_buffer.insert(tags, components);
        self
    }

    pub fn add_entity(mut self, entity_archetype: impl EntityArchetype) -> Self {
        let current_entity = entity_archetype.insert_command_buffer(self.command_buffer);
        self.current_entity = Some(current_entity);
        self.add_parent_to_current_entity();
        self
    }

    pub fn add_children(mut self, build_children: impl Fn(CommandBufferBuilder) -> CommandBufferBuilder) -> Self {
        self.parent_entity = self.current_entity;
        self.current_entity = None;

        self = build_children(self);

        self.current_entity = self.parent_entity;
        self.parent_entity = None;
        self
    }

    fn add_parent_to_current_entity(&mut self) {
        let current_entity = self.current_entity.unwrap();
        if let Some(parent_entity) = self.parent_entity {
            let _ = self
                .command_buffer
                .add_component(current_entity, Parent(parent_entity));
            let _ = self
                .command_buffer
                .add_component(current_entity, LocalToParent::identity());
        }
    }
}

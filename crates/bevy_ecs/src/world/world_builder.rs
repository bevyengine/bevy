use bevy_hecs::{Bundle, Component, DynamicBundle, Entity, World};

/// Converts a reference to `Self` to a [WorldBuilder]
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

/// Modify a [World] using the builder pattern
#[derive(Debug)]
pub struct WorldBuilder<'a> {
    pub world: &'a mut World,
    pub current_entity: Option<Entity>,
}

impl<'a> WorldBuilder<'a> {
    pub fn entity(&mut self) -> &mut Self {
        self.current_entity = Some(self.world.reserve_entity());
        self
    }

    pub fn set_entity(&mut self, entity: Entity) -> &mut Self {
        self.current_entity = Some(entity);
        self
    }

    pub fn with<T>(&mut self, component: T) -> &mut Self
    where
        T: Component,
    {
        self.world
            .insert_one(self.current_entity.expect("Cannot add component because the 'current entity' is not set. You should spawn an entity first."), component)
            .unwrap();
        self
    }

    pub fn with_bundle(&mut self, components: impl DynamicBundle) -> &mut Self {
        self.world
            .insert(self.current_entity.expect("Cannot add component because the 'current entity' is not set. You should spawn an entity first."), components)
            .unwrap();
        self
    }

    pub fn spawn_batch<I>(&mut self, components_iter: I) -> &mut Self
    where
        I: IntoIterator,
        I::Item: Bundle,
    {
        self.world.spawn_batch(components_iter);
        self
    }

    pub fn spawn(&mut self, components: impl DynamicBundle) -> &mut Self {
        self.current_entity = Some(self.world.spawn(components));
        self
    }
}

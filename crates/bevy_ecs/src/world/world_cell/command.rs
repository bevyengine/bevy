use crate::{
    component::Component,
    prelude::{Entity, World},
    system::{Command, Despawn, Remove},
    world::{WorldCell, WorldCellState},
};
use std::{marker::PhantomData, sync::RwLock};

#[derive(Debug)]
pub struct CellInsert<T> {
    pub entity: Entity,
    // this could be a RefCell, because we will use it from single thread anyway.
    // Unfortunately, Command trait requires Sync.
    pub component: RwLock<T>,
}

impl<T> Command for CellInsert<T>
where
    T: Component,
{
    fn write(self, world: &mut World) {
        world
            .entity_mut(self.entity)
            .insert::<T>(self.component.into_inner().unwrap());
    }
}

/// A list of commands that will be run to modify an [`Entity`] inside `WorldCell`.
pub struct CellEntityCommands<'a> {
    entity: Entity,
    state: &'a WorldCellState,
}

impl<'a> CellEntityCommands<'a> {
    /// Retrieves the current entity's unique [`Entity`] id.
    #[inline]
    pub fn id(&self) -> Entity {
        self.entity
    }

    // /// Adds a [`Bundle`] of components to the current entity.
    // pub fn insert_bundle(&mut self, bundle: impl Bundle) -> &mut Self {
    //     self.state.command_queue.borrow_mut().push(InsertBundle {
    //         entity: self.entity,
    //         bundle,
    //     });
    //     self
    // }

    /// Adds a single [`Component`] to the current entity.
    ///
    /// `Self::insert` can be chained with [`WorldCell::spawn`].
    ///
    /// See [`Commands::insert`] for analogous method in [`Commands`].
    pub fn insert(&mut self, component: impl Component) -> &mut Self {
        self.state.command_queue.push(CellInsert {
            entity: self.entity,
            component: RwLock::new(component),
        });
        self
    }

    // /// See [`EntityMut::remove_bundle`](crate::world::EntityMut::remove_bundle).
    // pub fn remove_bundle<T>(&mut self) -> &mut Self
    // where
    //     T: Bundle,
    // {
    //     self.state
    //         .command_queue
    //         .borrow_mut()
    //         .push(RemoveBundle::<T> {
    //             entity: self.entity,
    //             phantom: PhantomData,
    //         });
    //     self
    // }

    /// See [`EntityMut::remove`](crate::world::EntityMut::remove).
    pub fn remove<T>(&mut self) -> &mut Self
    where
        T: Component,
    {
        self.state.command_queue.push(Remove::<T> {
            entity: self.entity,
            phantom: PhantomData,
        });
        self
    }

    /// Despawns only the specified entity, not including its children.
    pub fn despawn(&mut self) {
        self.state.command_queue.push(Despawn {
            entity: self.entity,
        })
    }
}

impl<'w> WorldCell<'w> {
    pub fn entity(&self, entity: Entity) -> CellEntityCommands<'_> {
        CellEntityCommands {
            entity,
            state: &self.state,
        }
    }

    /// A WorldCell session "barrier". Applies world commands issued thus far, optimizing future query accesses.
    pub fn maintain(&mut self) {
        // Clear working set when the WorldCell session ends.
        for entry in self.state.query_cache_working_set.get_mut().drain(..) {
            entry.in_working_set.set(false);
        }
        self.state.command_queue.apply(self.world);
    }
}

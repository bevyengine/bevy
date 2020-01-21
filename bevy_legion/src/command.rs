use crate::{
    borrow::AtomicRefCell,
    cons::{ConsAppend, ConsFlatten},
    entity::{Entity, EntityAllocator},
    filter::{ChunksetFilterData, Filter},
    storage::{Component, ComponentTypeId, Tag, TagTypeId},
    world::{ComponentSource, ComponentTupleSet, IntoComponentSource, TagLayout, TagSet, World},
};
use derivative::Derivative;
use smallvec::SmallVec;
use std::{collections::VecDeque, iter::FromIterator, marker::PhantomData, sync::Arc};

/// This trait can be used to implement custom world writer types that can be directly
/// inserted into the command buffer, for more custom and complex world operations. This is analogous
/// to the `CommandBuffer::exec_mut` function type, but does not perform explicit any/any archetype
/// access.
pub trait WorldWritable {
    /// Destructs the writer and performs the write operations on the world.
    fn write(self: Arc<Self>, world: &mut World);

    /// Returns the list of `ComponentTypeId` which are written by this command buffer. This is leveraged
    /// to allow parralel command buffer flushing.
    fn write_components(&self) -> Vec<ComponentTypeId>;

    /// Returns the list of `TagTypeId` which are written by this command buffer. This is leveraged
    /// to allow parralel command buffer flushing.
    fn write_tags(&self) -> Vec<TagTypeId>;
}

#[derive(Derivative)]
#[derivative(Debug(bound = ""))]
struct InsertBufferedCommand<T, C> {
    write_components: Vec<ComponentTypeId>,
    write_tags: Vec<TagTypeId>,

    #[derivative(Debug = "ignore")]
    tags: T,
    #[derivative(Debug = "ignore")]
    components: C,

    entities: Vec<Entity>,
}
impl<T, C> WorldWritable for InsertBufferedCommand<T, C>
where
    T: TagSet + TagLayout + for<'a> Filter<ChunksetFilterData<'a>>,
    C: ComponentSource,
{
    fn write(self: Arc<Self>, world: &mut World) {
        let consumed = Arc::try_unwrap(self).unwrap();

        world.insert_buffered(&consumed.entities, consumed.tags, consumed.components);
    }

    fn write_components(&self) -> Vec<ComponentTypeId> { self.write_components.clone() }
    fn write_tags(&self) -> Vec<TagTypeId> { self.write_tags.clone() }
}

#[derive(Derivative)]
#[derivative(Debug(bound = ""))]
struct InsertCommand<T, C> {
    write_components: Vec<ComponentTypeId>,
    write_tags: Vec<TagTypeId>,

    #[derivative(Debug = "ignore")]
    tags: T,
    #[derivative(Debug = "ignore")]
    components: C,
}
impl<T, C> WorldWritable for InsertCommand<T, C>
where
    T: TagSet + TagLayout + for<'a> Filter<ChunksetFilterData<'a>>,
    C: IntoComponentSource,
{
    fn write(self: Arc<Self>, world: &mut World) {
        let consumed = Arc::try_unwrap(self).unwrap();
        world.insert(consumed.tags, consumed.components);
    }

    fn write_components(&self) -> Vec<ComponentTypeId> { self.write_components.clone() }
    fn write_tags(&self) -> Vec<TagTypeId> { self.write_tags.clone() }
}

#[derive(Derivative)]
#[derivative(Debug(bound = ""))]
struct DeleteEntityCommand(Entity);
impl WorldWritable for DeleteEntityCommand {
    fn write(self: Arc<Self>, world: &mut World) { world.delete(self.0); }

    fn write_components(&self) -> Vec<ComponentTypeId> { Vec::with_capacity(0) }
    fn write_tags(&self) -> Vec<TagTypeId> { Vec::with_capacity(0) }
}

#[derive(Derivative)]
#[derivative(Debug(bound = ""))]
struct AddTagCommand<T> {
    entity: Entity,
    #[derivative(Debug = "ignore")]
    tag: T,
}
impl<T> WorldWritable for AddTagCommand<T>
where
    T: Tag,
{
    fn write(self: Arc<Self>, world: &mut World) {
        let consumed = Arc::try_unwrap(self).unwrap();
        world.add_tag(consumed.entity, consumed.tag)
    }

    fn write_components(&self) -> Vec<ComponentTypeId> { Vec::with_capacity(0) }
    fn write_tags(&self) -> Vec<TagTypeId> { vec![TagTypeId::of::<T>()] }
}

#[derive(Derivative)]
#[derivative(Debug(bound = ""))]
struct RemoveTagCommand<T> {
    entity: Entity,
    _marker: PhantomData<T>,
}
impl<T> WorldWritable for RemoveTagCommand<T>
where
    T: Tag,
{
    fn write(self: Arc<Self>, world: &mut World) { world.remove_tag::<T>(self.entity) }

    fn write_components(&self) -> Vec<ComponentTypeId> { Vec::with_capacity(0) }
    fn write_tags(&self) -> Vec<TagTypeId> { vec![TagTypeId::of::<T>()] }
}

#[derive(Derivative)]
#[derivative(Debug(bound = ""))]
struct AddComponentCommand<C> {
    #[derivative(Debug = "ignore")]
    entity: Entity,
    #[derivative(Debug = "ignore")]
    component: C,
}
impl<C> WorldWritable for AddComponentCommand<C>
where
    C: Component,
{
    fn write(self: Arc<Self>, world: &mut World) {
        let consumed = Arc::try_unwrap(self).unwrap();
        world
            .add_component::<C>(consumed.entity, consumed.component)
            .unwrap();
    }

    fn write_components(&self) -> Vec<ComponentTypeId> { vec![ComponentTypeId::of::<C>()] }
    fn write_tags(&self) -> Vec<TagTypeId> { Vec::with_capacity(0) }
}

#[derive(Derivative)]
#[derivative(Debug(bound = ""))]
struct RemoveComponentCommand<C> {
    entity: Entity,
    _marker: PhantomData<C>,
}
impl<C> WorldWritable for RemoveComponentCommand<C>
where
    C: Component,
{
    fn write(self: Arc<Self>, world: &mut World) { world.remove_component::<C>(self.entity) }

    fn write_components(&self) -> Vec<ComponentTypeId> { vec![ComponentTypeId::of::<C>()] }
    fn write_tags(&self) -> Vec<TagTypeId> { Vec::with_capacity(0) }
}

#[allow(clippy::enum_variant_names)]
enum EntityCommand {
    WriteWorld(Arc<dyn WorldWritable>),
    ExecWorld(Arc<dyn Fn(&World)>),
    ExecMutWorld(Arc<dyn Fn(&mut World)>),
}

/// A builder type which can be retrieved from the command buffer. This is the ideal use case for
/// inserted complex entities with multiple components and tags from a command buffer. Although
/// `add_component` will perform a new move operation on every addition, this allows the construction
/// of a single `insert` command for an entity, but without using the actual `insert` command
/// provided by the `CommandBuffer`
///
/// # Examples
///
/// Inserting an entity using the `EntityBuilder`:
///
/// ```
/// # use legion::prelude::*;
/// # #[derive(Copy, Clone, Debug, PartialEq)]
/// # struct Position(f32);
/// # #[derive(Copy, Clone, Debug, PartialEq)]
/// # struct Rotation(f32);
/// # let universe = Universe::new();
/// # let mut world = universe.create_world();
/// let mut command_buffer = CommandBuffer::from_world(&mut world);
/// command_buffer.build_entity().unwrap()
///     .with_component(Position(123.0))
///     .with_component(Rotation(456.0)).build(&mut command_buffer);
/// command_buffer.write(&mut world);
/// ```
pub struct EntityBuilder<TS = (), CS = ()> {
    entity: Entity,
    tags: TS,
    components: CS,
}
impl<TS, CS> EntityBuilder<TS, CS>
where
    TS: 'static + Send + ConsFlatten,
    CS: 'static + Send + ConsFlatten,
{
    /// Adds a component to this builder, returning a new builder type containing that component type
    /// and its data.
    pub fn with_component<C: Component>(
        self,
        component: C,
    ) -> EntityBuilder<TS, <CS as ConsAppend<C>>::Output>
    where
        CS: ConsAppend<C>,
        <CS as ConsAppend<C>>::Output: ConsFlatten,
    {
        EntityBuilder {
            components: ConsAppend::append(self.components, component),
            entity: self.entity,
            tags: self.tags,
        }
    }

    /// Adds a tag to this builder, returning a new builder type containing that component type
    /// and its data.
    pub fn with_tag<T: Tag>(self, tag: T) -> EntityBuilder<<TS as ConsAppend<T>>::Output, CS>
    where
        TS: ConsAppend<T>,
        <TS as ConsAppend<T>>::Output: ConsFlatten,
    {
        EntityBuilder {
            tags: ConsAppend::append(self.tags, tag),
            entity: self.entity,
            components: self.components,
        }
    }

    /// Finalizes this builder type and submits it to the `CommandBuffer` as a `WorldWritable` trait
    /// object.
    pub fn build(self, buffer: &mut CommandBuffer)
    where
        <TS as ConsFlatten>::Output: TagSet + TagLayout + for<'a> Filter<ChunksetFilterData<'a>>,
        ComponentTupleSet<
            <CS as ConsFlatten>::Output,
            std::iter::Once<<CS as ConsFlatten>::Output>,
        >: ComponentSource,
    {
        buffer
            .commands
            .get_mut()
            .push_front(EntityCommand::WriteWorld(Arc::new(InsertBufferedCommand {
                write_components: Vec::default(),
                write_tags: Vec::default(),
                tags: self.tags.flatten(),
                components: IntoComponentSource::into(std::iter::once(self.components.flatten())),
                entities: vec![self.entity],
            })));
    }
}

/// Errors returned by the `CommandBuffer`
#[derive(Debug)]
pub enum CommandError {
    /// The command buffers entity cache has been exhausted. This is defaulted to 64 at `World::DEFAULT_COMMAND_BUFFER_SIZE`.
    /// This upper limit can be changed via `SystemBuilder::with_command_buffer_size` for specific systems,
    /// or globally via `World::set_command_buffer_size`.
    EntityBlockFull,
}
impl std::fmt::Display for CommandError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { write!(f, "CommandError") }
}

impl std::error::Error for CommandError {
    fn cause(&self) -> Option<&dyn std::error::Error> { None }
}

/// A command buffer used to queue mutable changes to the world from a system. This buffer is automatically
/// flushed and refreshed at the beginning of every frame by `Schedule`. If `Schedule` is not used,
/// then the user needs to manually flush it by performing `CommandBuffer::write`.
///
/// This buffer operates as follows:
///     - All commands are queued as trait object of type `WorldWritable`, to be executed when `CommandBuffer:write` is called.
///     - Entities are allocated at the time of `CommandBuffer:write` occuring, being directly allocated from the world
///       and cached internally in the system. This upper cache size can be changed via `SystemBuilder::with_command_buffer_size`
///       for specific systems, or globally via `World::set_command_buffer_size`. In the event the cached entity count is exceeded,
///       the cache will be refilled on demand from the world `EntityAllocator`.
///  
/// This behavior exists because `EntityAllocator` is a shared lock within the world, so in order to reduce lock contention with many
/// systems running and adding entities, the `CommandBuffer` will cache the configured number of entities - reducing contention.
///
/// # Examples
///
/// Inserting an entity using the `CommandBuffer`:
///
/// ```
/// # use legion::prelude::*;
/// # #[derive(Copy, Clone, Debug, PartialEq)]
/// # struct Position(f32);
/// # #[derive(Copy, Clone, Debug, PartialEq)]
/// # struct Rotation(f32);
/// # let universe = Universe::new();
/// # let mut world = universe.create_world();
/// let mut command_buffer = CommandBuffer::from_world(&mut world);
/// let entity = command_buffer.create_entity().unwrap();
///
/// command_buffer.add_component(entity, Position(123.0));
/// command_buffer.delete(entity);
///
/// command_buffer.write(&mut world);
/// ```
#[derive(Default)]
pub struct CommandBuffer {
    commands: AtomicRefCell<VecDeque<EntityCommand>>,
    entity_allocator: Option<Arc<EntityAllocator>>,
    pub(crate) custom_capacity: Option<usize>,
    pub(crate) free_list: SmallVec<[Entity; 64]>,
    pub(crate) used_list: SmallVec<[Entity; 64]>,
}
// This is safe because only 1 system in 1 execution is only ever accessing a command buffer
// and we garuntee the write operations of a command buffer occur in a safe manner
unsafe impl Send for CommandBuffer {}
unsafe impl Sync for CommandBuffer {}

impl CommandBuffer {
    /// Creates a `CommandBuffer` with a custom capacity of cached Entity's to be collected every frame.
    /// Allocating a command buffer in this manner will overwrite `World::set_command_buffer_size` and
    /// this system will always allocate the custom provide capacity of entities every frame.
    ///
    /// # Notes
    /// This function does not perform any actual entity preallocation. `ComamandBuffer:resize` or `CommandBuffer:write`
    /// must be called before using the command buffer for the first time to make entities available.
    pub fn with_capacity(capacity: usize) -> Self {
        // Pull  free entities from the world.

        Self {
            custom_capacity: Some(capacity),
            free_list: SmallVec::with_capacity(capacity),
            commands: Default::default(),
            used_list: SmallVec::with_capacity(capacity),
            entity_allocator: None,
        }
    }

    /// Creates a `CommandBuffer` with a custom capacity of cached Entity's to be collected every frame.
    /// Allocating a command buffer in this manner will overwrite `World::set_command_buffer_size` and
    /// this system will always allocate the custom provide capacity of entities every frame.
    ///
    /// This constructor will preallocate the first round of entities needed from the world.
    pub fn from_world_with_capacity(world: &mut World, capacity: usize) -> Self {
        // Pull  free entities from the world.

        let free_list =
            SmallVec::from_iter((0..capacity).map(|_| world.entity_allocator.create_entity()));

        Self {
            free_list,
            custom_capacity: Some(capacity),
            commands: Default::default(),
            used_list: SmallVec::with_capacity(capacity),
            entity_allocator: Some(world.entity_allocator.clone()),
        }
    }

    /// Creates a `CommandBuffer` with a custom capacity of cached Entity's to be collected every frame.
    /// Allocating a command buffer in this manner will use the default `World::set_command_buffer_size`
    /// value.
    ///
    /// This constructor will preallocate the first round of entities needed from the world.
    pub fn from_world(world: &mut World) -> Self {
        // Pull  free entities from the world.

        let free_list = SmallVec::from_iter(
            (0..world.command_buffer_size()).map(|_| world.entity_allocator.create_entity()),
        );

        Self {
            free_list,
            custom_capacity: None,
            commands: Default::default(),
            used_list: SmallVec::with_capacity(world.command_buffer_size()),
            entity_allocator: Some(world.entity_allocator.clone()),
        }
    }

    /// Changes the cached capacity of this `CommandBuffer` to the specified capacity. This includes shrinking
    /// and growing the allocated entities, and possibly returning them to the entity allocator in the
    /// case of a shrink.
    ///
    /// This function does *NOT* set the `CommandBuffer::custom_capacity` override.
    #[allow(clippy::comparison_chain)]
    pub fn resize(&mut self, capacity: usize) {
        let allocator = &self.entity_allocator;
        let free_list = &mut self.free_list;

        if let Some(allocator) = allocator.as_ref() {
            if free_list.len() < capacity {
                (free_list.len()..capacity).for_each(|_| free_list.push(allocator.create_entity()));
            } else if free_list.len() > capacity {
                // Free the entities
                (free_list.len() - capacity..capacity).for_each(|_| {
                    allocator.delete_entity(free_list.pop().unwrap());
                });
            }
        } else {
            panic!("Entity allocator not assigned to command buffer")
        }
    }

    /// Flushes this command buffer, draining all stored commands and writing them to the world.
    ///
    /// Command flushes are performed in a FIFO manner, allowing for reliable, linear commands being
    /// executed in the order they were provided.
    ///
    /// This function also calls `CommandBuffer:resize`, performing any appropriate entity preallocation,
    /// refilling the entity cache of any consumed entities.
    pub fn write(&mut self, world: &mut World) {
        tracing::trace!("Draining command buffer");

        if self.entity_allocator.is_none() {
            self.entity_allocator = Some(world.entity_allocator.clone());
        }

        let empty = Vec::from_iter((0..self.used_list.len()).map(|_| ()));
        world.insert_buffered(
            self.used_list.as_slice(),
            (),
            IntoComponentSource::into(empty),
        );
        self.used_list.clear();

        while let Some(command) = self.commands.get_mut().pop_back() {
            match command {
                EntityCommand::WriteWorld(ptr) => ptr.write(world),
                EntityCommand::ExecMutWorld(closure) => closure(world),
                EntityCommand::ExecWorld(closure) => closure(world),
            }
        }

        // Refill our entity buffer from the world
        if let Some(custom_capacity) = self.custom_capacity {
            self.resize(custom_capacity);
        } else {
            self.resize(world.command_buffer_size());
        }
    }

    /// Consumed an internally cached entity, returning an `EntityBuilder` using that entity.
    pub fn build_entity(&mut self) -> Result<EntityBuilder<(), ()>, CommandError> {
        let entity = self.create_entity()?;

        Ok(EntityBuilder {
            entity,
            tags: (),
            components: (),
        })
    }

    /// Consumed an internally cached entity, or returns `CommandError`
    pub fn create_entity(&mut self) -> Result<Entity, CommandError> {
        if self.free_list.is_empty() {
            self.resize(
                self.custom_capacity
                    .unwrap_or(World::DEFAULT_COMMAND_BUFFER_SIZE),
            );
        }
        let entity = self.free_list.pop().ok_or(CommandError::EntityBlockFull)?;
        self.used_list.push(entity);

        Ok(entity)
    }

    /// Executes an arbitrary closure against the mutable world, allowing for queued exclusive
    /// access to the world.
    pub fn exec_mut<F>(&self, f: F)
    where
        F: 'static + Fn(&mut World),
    {
        self.commands
            .get_mut()
            .push_front(EntityCommand::ExecMutWorld(Arc::new(f)));
    }

    /// Inserts an arbitrary implementor of the `WorldWritable` trait into the command queue.
    /// This can be leveraged for creating custom `WorldWritable` trait implementors, and is used
    /// internally for the default writers.
    pub fn insert_writer<W>(&self, writer: W)
    where
        W: 'static + WorldWritable,
    {
        self.commands
            .get_mut()
            .push_front(EntityCommand::WriteWorld(Arc::new(writer)));
    }

    /// Queues an *unbuffered* insertion into the world. This command follows the same syntax as
    /// the normal `World::insert`, except for one caviate - entities are NOT returned by this
    /// function, meaning that the internal entity cache and limits of this `CommandBuffer` are not
    /// applicable to this function.
    ///
    /// This function can be considered a "fire and forget" entity creation method which is not bound
    /// by the standard command buffer size limits of the other entity insertion functions. This allows
    /// for mass insertion of entities, exceeding the command buffer sizes, to occur in scenarios that
    /// the entities do not need to be retrieved.
    pub fn insert_unbuffered<T, C>(&mut self, tags: T, components: C)
    where
        T: 'static + TagSet + TagLayout + for<'a> Filter<ChunksetFilterData<'a>>,
        C: 'static + IntoComponentSource,
    {
        self.commands
            .get_mut()
            .push_front(EntityCommand::WriteWorld(Arc::new(InsertCommand {
                write_components: Vec::default(),
                write_tags: Vec::default(),
                tags,
                components,
            })));
    }

    /// Queues an insertion into the world. This command follows the same syntax as
    /// the normal `World::insert`, returning the entities created for this command.
    pub fn insert<T, C>(&mut self, tags: T, components: C) -> Result<Vec<Entity>, CommandError>
    where
        T: 'static + TagSet + TagLayout + for<'a> Filter<ChunksetFilterData<'a>>,
        C: 'static + IntoComponentSource,
    {
        let components = components.into();
        if components.len() > self.free_list.len() {
            return Err(CommandError::EntityBlockFull);
        }

        let mut entities = Vec::with_capacity(components.len());
        for _ in 0..components.len() {
            entities.push(self.free_list.pop().ok_or(CommandError::EntityBlockFull)?);
        }

        self.commands
            .get_mut()
            .push_front(EntityCommand::WriteWorld(Arc::new(InsertBufferedCommand {
                write_components: Vec::default(),
                write_tags: Vec::default(),
                tags,
                components,
                entities: entities.clone(),
            })));

        Ok(entities)
    }

    /// Queues the deletion of an entity in the command buffer. This writer calls `World::delete`
    pub fn delete(&self, entity: Entity) {
        self.commands
            .get_mut()
            .push_front(EntityCommand::WriteWorld(Arc::new(DeleteEntityCommand(
                entity,
            ))));
    }

    /// Queues the addition of a component from an entity in the command buffer.
    /// This writer calls `World::add_component`
    pub fn add_component<C: Component>(&self, entity: Entity, component: C) {
        self.commands
            .get_mut()
            .push_front(EntityCommand::WriteWorld(Arc::new(AddComponentCommand {
                entity,
                component,
            })));
    }

    /// Queues the removal of a component from an entity in the command buffer.
    /// This writer calls `World::remove_component`
    pub fn remove_component<C: Component>(&self, entity: Entity) {
        self.commands
            .get_mut()
            .push_front(EntityCommand::WriteWorld(Arc::new(
                RemoveComponentCommand {
                    entity,
                    _marker: PhantomData::<C>::default(),
                },
            )));
    }

    /// Queues the addition of a tag from an entity in the command buffer.
    /// This writer calls `World::add_tag`
    pub fn add_tag<T: Tag>(&self, entity: Entity, tag: T) {
        self.commands
            .get_mut()
            .push_front(EntityCommand::WriteWorld(Arc::new(AddTagCommand {
                entity,
                tag,
            })));
    }

    /// Queues the removal of a tag from an entity in the command buffer.
    /// This writer calls `World::remove_tag`
    pub fn remove_tag<T: Tag>(&self, entity: Entity) {
        self.commands
            .get_mut()
            .push_front(EntityCommand::WriteWorld(Arc::new(RemoveTagCommand {
                entity,
                _marker: PhantomData::<T>::default(),
            })));
    }

    /// Returns the current number of commands already queued in this `CommandBuffer` instance.
    #[inline]
    pub fn len(&self) -> usize { self.commands.get().len() }

    /// Returns true if this `CommandBuffer` is currently empty and contains no writers.
    #[inline]
    pub fn is_empty(&self) -> bool { self.commands.get().len() == 0 }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prelude::*;

    #[derive(Clone, Copy, Debug, PartialEq)]
    struct Pos(f32, f32, f32);
    #[derive(Clone, Copy, Debug, PartialEq)]
    struct Vel(f32, f32, f32);
    #[derive(Default)]
    struct TestResource(pub i32);

    #[test]
    fn create_entity_test() -> Result<(), CommandError> {
        let _ = tracing_subscriber::fmt::try_init();

        let universe = Universe::new();
        let mut world = universe.create_world();

        let components = vec![
            (Pos(1., 2., 3.), Vel(0.1, 0.2, 0.3)),
            (Pos(4., 5., 6.), Vel(0.4, 0.5, 0.6)),
        ];
        let components_len = components.len();

        //world.entity_allocator.get_block()
        let mut command = CommandBuffer::from_world(&mut world);
        let entity1 = command.create_entity()?;
        let entity2 = command.create_entity()?;

        command.add_component(entity1, Pos(1., 2., 3.));
        command.add_component(entity2, Pos(4., 5., 6.));

        command.write(&mut world);

        let query = Read::<Pos>::query();

        let mut count = 0;
        for _ in query.iter_entities(&mut world) {
            count += 1;
        }

        assert_eq!(components_len, count);

        Ok(())
    }

    #[test]
    fn simple_write_test() -> Result<(), CommandError> {
        let _ = tracing_subscriber::fmt::try_init();

        let universe = Universe::new();
        let mut world = universe.create_world();

        let components = vec![
            (Pos(1., 2., 3.), Vel(0.1, 0.2, 0.3)),
            (Pos(4., 5., 6.), Vel(0.4, 0.5, 0.6)),
        ];
        let components_len = components.len();

        //world.entity_allocator.get_block()
        let mut command = CommandBuffer::from_world(&mut world);
        let _ = command.insert((), components)?;

        // Assert writing checks
        // TODO:
        //assert_eq!(
        //    vec![ComponentTypeId::of::<Pos>(), ComponentTypeId::of::<Vel>()],
        //    command.write_components()
        //);

        command.write(&mut world);

        let query = Read::<Pos>::query();

        let mut count = 0;
        for _ in query.iter_entities_mut(&mut world) {
            count += 1;
        }

        assert_eq!(components_len, count);

        Ok(())
    }
}

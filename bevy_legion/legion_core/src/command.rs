use crate::{
    borrow::AtomicRefCell,
    cons::{ConsAppend, ConsFlatten},
    entity::{Entity, EntityAllocator},
    filter::{ChunksetFilterData, Filter},
    storage::{Component, ComponentTypeId, Tag, TagTypeId},
    world::{
        ComponentSource, ComponentTupleSet, IntoComponentSource, PreallocComponentSource,
        TagLayout, TagSet, World, WorldId,
    },
};
use derivative::Derivative;
use smallvec::SmallVec;
use std::ops::Range;
use std::{collections::VecDeque, iter::FromIterator, marker::PhantomData, sync::Arc};
use tracing::{span, Level};

/// This trait can be used to implement custom world writer types that can be directly
/// inserted into the command buffer, for more custom and complex world operations. This is analogous
/// to the `CommandBuffer::exec_mut` function type, but does not perform explicit any/any archetype
/// access.
pub trait WorldWritable {
    /// Destructs the writer and performs the write operations on the world.
    fn write(self: Arc<Self>, world: &mut World, cmd: &CommandBuffer);

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

    entities: Range<usize>,
}
impl<T, C> WorldWritable for InsertBufferedCommand<T, C>
where
    T: TagSet + TagLayout + for<'a> Filter<ChunksetFilterData<'a>>,
    C: ComponentSource,
{
    fn write(self: Arc<Self>, world: &mut World, cmd: &CommandBuffer) {
        let consumed = Arc::try_unwrap(self).unwrap();

        world.insert(
            consumed.tags,
            PreallocComponentSource::new(
                cmd.pending_insertion[consumed.entities].iter().copied(),
                consumed.components,
            ),
        );
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
    fn write(self: Arc<Self>, world: &mut World, _: &CommandBuffer) {
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
    fn write(self: Arc<Self>, world: &mut World, _: &CommandBuffer) { world.delete(self.0); }

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
    fn write(self: Arc<Self>, world: &mut World, _: &CommandBuffer) {
        let consumed = Arc::try_unwrap(self).unwrap();
        if let Err(err) = world.add_tag(consumed.entity, consumed.tag) {
            tracing::error!(error = %err, "error adding tag");
        }
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
    fn write(self: Arc<Self>, world: &mut World, _: &CommandBuffer) {
        if let Err(err) = world.remove_tag::<T>(self.entity) {
            tracing::error!(error = %err, "error removing tag");
        }
    }

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
    fn write(self: Arc<Self>, world: &mut World, _: &CommandBuffer) {
        let consumed = Arc::try_unwrap(self).unwrap();
        if let Err(err) = world.add_component::<C>(consumed.entity, consumed.component) {
            tracing::error!(error = %err, "error adding component");
        }
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
    fn write(self: Arc<Self>, world: &mut World, _: &CommandBuffer) {
        if let Err(err) = world.remove_component::<C>(self.entity) {
            tracing::error!(error = %err, "error removing component");
        }
    }

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
/// # use legion_core::prelude::*;
/// # #[derive(Copy, Clone, Debug, PartialEq)]
/// # struct Position(f32);
/// # #[derive(Copy, Clone, Debug, PartialEq)]
/// # struct Rotation(f32);
/// # let universe = Universe::new();
/// # let mut world = universe.create_world();
/// let mut command_buffer = CommandBuffer::new(&world);
/// command_buffer.start_entity()
///     .with_component(Position(123.0))
///     .with_component(Rotation(456.0))
///     .build();
/// command_buffer.write(&mut world);
/// ```
pub struct EntityBuilder<'a, TS = (), CS = ()> {
    cmd: &'a mut CommandBuffer,
    tags: TS,
    components: CS,
}

impl<'a, TS, CS> EntityBuilder<'a, TS, CS>
where
    TS: 'static + Send + ConsFlatten,
    CS: 'static + Send + ConsFlatten,
{
    /// Adds a component to this builder, returning a new builder type containing that component type
    /// and its data.
    pub fn with_component<C: Component>(
        self,
        component: C,
    ) -> EntityBuilder<'a, TS, <CS as ConsAppend<C>>::Output>
    where
        CS: ConsAppend<C>,
        <CS as ConsAppend<C>>::Output: ConsFlatten,
    {
        EntityBuilder {
            cmd: self.cmd,
            components: ConsAppend::append(self.components, component),
            tags: self.tags,
        }
    }

    /// Adds a tag to this builder, returning a new builder type containing that component type
    /// and its data.
    pub fn with_tag<T: Tag>(self, tag: T) -> EntityBuilder<'a, <TS as ConsAppend<T>>::Output, CS>
    where
        TS: ConsAppend<T>,
        <TS as ConsAppend<T>>::Output: ConsFlatten,
    {
        EntityBuilder {
            cmd: self.cmd,
            tags: ConsAppend::append(self.tags, tag),
            components: self.components,
        }
    }

    /// Finalizes this builder type and submits it to the `CommandBuffer`.
    pub fn build(self) -> Entity
    where
        <TS as ConsFlatten>::Output: TagSet + TagLayout + for<'b> Filter<ChunksetFilterData<'b>>,
        ComponentTupleSet<
            <CS as ConsFlatten>::Output,
            std::iter::Once<<CS as ConsFlatten>::Output>,
        >: ComponentSource,
    {
        self.cmd.insert(
            self.tags.flatten(),
            std::iter::once(self.components.flatten()),
        )[0]
    }
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
/// # use legion_core::prelude::*;
/// # #[derive(Copy, Clone, Debug, PartialEq)]
/// # struct Position(f32);
/// # #[derive(Copy, Clone, Debug, PartialEq)]
/// # struct Rotation(f32);
/// # let universe = Universe::new();
/// # let mut world = universe.create_world();
/// let mut command_buffer = CommandBuffer::new(&world);
/// let entity = command_buffer.start_entity().build();
///
/// command_buffer.add_component(entity, Position(123.0));
/// command_buffer.delete(entity);
///
/// command_buffer.write(&mut world);
/// ```
pub struct CommandBuffer {
    world_id: WorldId,
    commands: AtomicRefCell<VecDeque<EntityCommand>>,
    entity_allocator: Arc<EntityAllocator>,
    preallocated_capacity: usize,
    free_list: SmallVec<[Entity; 64]>,
    pending_insertion: SmallVec<[Entity; 64]>,
}

// This is safe because only 1 system in 1 execution is only ever accessing a command buffer
// and we gaurantee the write operations of a command buffer occur in a safe manner
unsafe impl Send for CommandBuffer {}
unsafe impl Sync for CommandBuffer {}

impl CommandBuffer {
    /// Creates a `CommandBuffer` with a custom capacity of cached Entity's to be collected every frame.
    /// Allocating a command buffer in this manner will override `World::set_command_buffer_size` and
    /// this system will always allocate the custom provide capacity of entities every frame.
    ///
    /// This constructor will preallocate the first round of entities needed from the world.
    pub fn new_with_capacity(world: &World, capacity: usize) -> Self {
        // Pull  free entities from the world.

        let free_list =
            SmallVec::from_iter((0..capacity).map(|_| world.entity_allocator.create_entity()));

        Self {
            world_id: world.id(),
            free_list,
            preallocated_capacity: capacity,
            commands: Default::default(),
            pending_insertion: SmallVec::new(),
            entity_allocator: world.entity_allocator.clone(),
        }
    }

    /// Creates a `CommandBuffer` with a custom capacity of cached Entity's to be collected every frame.
    /// Allocating a command buffer in this manner will use the default `World::set_command_buffer_size`
    /// value.
    ///
    /// This constructor will preallocate the first round of entities needed from the world.
    pub fn new(world: &World) -> Self {
        let free_list = SmallVec::from_iter(
            (0..world.command_buffer_size()).map(|_| world.entity_allocator.create_entity()),
        );

        Self {
            world_id: world.id(),
            free_list,
            preallocated_capacity: world.command_buffer_size(),
            commands: Default::default(),
            pending_insertion: SmallVec::new(),
            entity_allocator: world.entity_allocator.clone(),
        }
    }

    /// Gets the ID of the world this command buffer belongs to.
    pub fn world(&self) -> WorldId { self.world_id }

    /// Changes the cached capacity of this `CommandBuffer` to the specified capacity. This includes shrinking
    /// and growing the allocated entities, and possibly returning them to the entity allocator in the
    /// case of a shrink.
    ///
    /// This function does *NOT* set the `CommandBuffer::custom_capacity` override.
    #[allow(clippy::comparison_chain)]
    fn resize(&mut self) {
        let allocator = &self.entity_allocator;
        let free_list = &mut self.free_list;
        let capacity = self.preallocated_capacity;

        if free_list.len() < capacity {
            for entity in allocator.create_entities().take(capacity - free_list.len()) {
                free_list.push(entity);
            }
        } else if free_list.len() > capacity {
            // Free the entities
            (free_list.len() - capacity..capacity).for_each(|_| {
                allocator.delete_entity(free_list.pop().unwrap());
            });
        }
    }

    /// Flushes this command buffer, draining all stored commands and writing them to the world.
    ///
    /// Command flushes are performed in a FIFO manner, allowing for reliable, linear commands being
    /// executed in the order they were provided.
    pub fn write(&mut self, world: &mut World) {
        let span = span!(Level::TRACE, "Draining command buffer");
        let _guard = span.enter();

        if self.world_id != world.id() {
            panic!("command buffers may only write into their parent world");
        }

        while let Some(command) = self.commands.get_mut().pop_back() {
            match command {
                EntityCommand::WriteWorld(ptr) => ptr.write(world, self),
                EntityCommand::ExecMutWorld(closure) => closure(world),
                EntityCommand::ExecWorld(closure) => closure(world),
            }
        }
        self.pending_insertion.clear();

        // Refill our entity buffer from the world
        self.resize();
    }

    /// Creates an entity builder for constructing a new entity.
    pub fn start_entity(&mut self) -> EntityBuilder<(), ()> {
        EntityBuilder {
            cmd: self,
            tags: (),
            components: (),
        }
    }

    /// Allocates a new entity.
    fn allocate_entity(&mut self) -> Entity {
        if self.free_list.is_empty() {
            self.resize();
        }
        let entity = self
            .free_list
            .pop()
            .unwrap_or_else(|| self.entity_allocator.create_entity());
        self.pending_insertion.push(entity);
        entity
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
    fn insert_writer<W>(&self, writer: W)
    where
        W: 'static + WorldWritable,
    {
        self.commands
            .get_mut()
            .push_front(EntityCommand::WriteWorld(Arc::new(writer)));
    }

    /// Queues an insertion into the world. This command follows the same syntax as
    /// the normal `World::insert`, returning the entities created for this command.
    pub fn insert<T, C>(&mut self, tags: T, components: C) -> &[Entity]
    where
        T: 'static + TagSet + TagLayout + for<'a> Filter<ChunksetFilterData<'a>>,
        C: 'static + IntoComponentSource,
    {
        let components = components.into();
        let start = self.pending_insertion.len();
        let count = components.len();

        self.pending_insertion.reserve(count);
        for _ in 0..count {
            self.allocate_entity();
        }

        let range = start..self.pending_insertion.len();

        self.commands
            .get_mut()
            .push_front(EntityCommand::WriteWorld(Arc::new(InsertBufferedCommand {
                write_components: Vec::default(),
                write_tags: Vec::default(),
                tags,
                components,
                entities: range.clone(),
            })));

        &self.pending_insertion[range]
    }

    /// Queues the deletion of an entity in the command buffer. This writer calls `World::delete`
    pub fn delete(&self, entity: Entity) { self.insert_writer(DeleteEntityCommand(entity)); }

    /// Queues the addition of a component from an entity in the command buffer.
    /// This writer calls `World::add_component`
    pub fn add_component<C: Component>(&self, entity: Entity, component: C) {
        self.insert_writer(AddComponentCommand { entity, component });
    }

    /// Queues the removal of a component from an entity in the command buffer.
    /// This writer calls `World::remove_component`
    pub fn remove_component<C: Component>(&self, entity: Entity) {
        self.insert_writer(RemoveComponentCommand {
            entity,
            _marker: PhantomData::<C>::default(),
        });
    }

    /// Queues the addition of a tag from an entity in the command buffer.
    /// This writer calls `World::add_tag`
    pub fn add_tag<T: Tag>(&self, entity: Entity, tag: T) {
        self.insert_writer(AddTagCommand { entity, tag });
    }

    /// Queues the removal of a tag from an entity in the command buffer.
    /// This writer calls `World::remove_tag`
    pub fn remove_tag<T: Tag>(&self, entity: Entity) {
        self.insert_writer(RemoveTagCommand {
            entity,
            _marker: PhantomData::<T>::default(),
        });
    }

    /// Returns the current number of commands already queued in this `CommandBuffer` instance.
    #[inline]
    pub fn len(&self) -> usize { self.commands.get().len() }

    /// Returns true if this `CommandBuffer` is currently empty and contains no writers.
    #[inline]
    pub fn is_empty(&self) -> bool { self.commands.get().len() == 0 }
}

impl Drop for CommandBuffer {
    fn drop(&mut self) {
        while let Some(entity) = self.free_list.pop() {
            self.entity_allocator.delete_entity(entity);
        }

        while let Some(entity) = self.pending_insertion.pop() {
            self.entity_allocator.delete_entity(entity);
        }
    }
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
    fn create_entity_test() {
        let _ = tracing_subscriber::fmt::try_init();

        let universe = Universe::new();
        let mut world = universe.create_world();

        let components = vec![
            (Pos(1., 2., 3.), Vel(0.1, 0.2, 0.3)),
            (Pos(4., 5., 6.), Vel(0.4, 0.5, 0.6)),
        ];
        let components_len = components.len();

        //world.entity_allocator.get_block()
        let mut command = CommandBuffer::new(&world);
        let entity1 = command.start_entity().build();
        let entity2 = command.start_entity().build();

        command.add_component(entity1, Pos(1., 2., 3.));
        command.add_component(entity2, Pos(4., 5., 6.));

        command.write(&mut world);

        let query = Read::<Pos>::query();

        let mut count = 0;
        for _ in query.iter_entities(&world) {
            count += 1;
        }

        assert_eq!(components_len, count);
    }

    #[test]
    fn simple_write_test() {
        let _ = tracing_subscriber::fmt::try_init();

        let universe = Universe::new();
        let mut world = universe.create_world();

        let components = vec![
            (Pos(1., 2., 3.), Vel(0.1, 0.2, 0.3)),
            (Pos(4., 5., 6.), Vel(0.4, 0.5, 0.6)),
        ];
        let components_len = components.len();

        //world.entity_allocator.get_block()
        let mut command = CommandBuffer::new(&world);
        let _ = command.insert((), components);

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
    }
}

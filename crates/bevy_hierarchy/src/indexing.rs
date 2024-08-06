use std::{hash::Hash, marker::PhantomData};

use bevy_app::{App, Plugin, Update};

use bevy_ecs::{
    component::{Component, Tick},
    prelude::{Changed, Entity, Query, Ref, RemovedComponents, ResMut},
    query::ReadOnlyWorldQuery,
    system::{Resource, SystemChangeTick, SystemParam},
};

use bevy_utils::{default, EntityHashMap, EntityHashSet, HashMap};

/// Describes how to transform an `Input` into an `Index` suitable for an [`Index`].
pub trait Indexer {
    /// The input to index against.
    type Input;

    /// A type suitable for indexing the `Input`
    type Index;

    /// Generate an `Index` from the provided `Input`
    fn index(input: &Self::Input) -> Self::Index;
}

/// A basic [`Indexer`] which directly uses `T`'s value.
pub struct SimpleIndexer<T>(PhantomData<T>);

impl<T> Indexer for SimpleIndexer<T>
where
    T: Clone,
{
    type Input = T;

    type Index = T;

    fn index(input: &Self::Input) -> Self::Index {
        input.clone()
    }
}

/// Stored data required for an [`Index`].
#[derive(Resource)]
struct IndexBacking<T, F = (), I = SimpleIndexer<T>>
where
    I: Indexer,
{
    forward: HashMap<I::Index, EntityHashSet<Entity>>,
    reverse: EntityHashMap<Entity, I::Index>,
    last_this_run: Option<Tick>,
    _phantom: PhantomData<fn(T, F, I)>,
    /// Used to return an empty `impl Iterator` from `get` on the `None` branch
    empty: EntityHashSet<Entity>,
}

impl<T, F, I> Default for IndexBacking<T, F, I>
where
    I: Indexer,
{
    fn default() -> Self {
        Self {
            forward: default(),
            reverse: default(),
            last_this_run: default(),
            _phantom: PhantomData,
            empty: default(),
        }
    }
}

impl<T, F, I> IndexBacking<T, F, I>
where
    I: Indexer<Input = T>,
    I::Index: Hash + Clone + Eq,
{
    fn update(&mut self, entity: Entity, value: Option<&T>) -> Option<I::Index> {
        let value = value.map(|value| I::index(value));

        if self.reverse.get(&entity) == value.as_ref() {
            // Return early since the value is already up-to-date
            return None;
        }

        let old = if let Some(ref value) = value {
            self.reverse.insert(entity, value.clone())
        } else {
            self.reverse.remove(&entity)
        };

        if let Some(ref old) = old {
            if let Some(set) = self.forward.get_mut(old) {
                set.remove(&entity);

                if set.is_empty() {
                    self.forward.remove(old);
                }
            }
        }

        if let Some(value) = value {
            self.forward.entry(value).or_default().insert(entity);
        };

        old
    }

    fn insert(&mut self, entity: Entity, value: &T) -> Option<I::Index> {
        self.update(entity, Some(value))
    }

    fn remove_by_entity(&mut self, entity: Entity) -> Option<I::Index> {
        self.update(entity, None)
    }

    fn get(&self, value: &T) -> impl Iterator<Item = Entity> + '_ {
        self.get_by_index(&I::index(value))
    }

    fn get_by_index(&self, index: &I::Index) -> impl Iterator<Item = Entity> + '_ {
        self.forward
            .get(index)
            .unwrap_or(&self.empty)
            .iter()
            .copied()
    }

    fn iter(
        &mut self,
    ) -> impl Iterator<Item = (&I::Index, impl Iterator<Item = Entity> + '_)> + '_ {
        self.forward
            .iter()
            .map(|(index, entities)| (index, entities.iter().copied()))
    }
}

/// Allows for lookup of an [`Entity`] based on the [`Component`] `T`'s value.
/// `F` allows this [`Index`] to only target a subset of all [entities](`Entity`) using a
/// [`ReadOnlyWorldQuery`].
/// `I` controls how the [`Component`] `T` will be used to create an indexable value using the [`Indexer`] trait.
#[derive(SystemParam)]
pub struct Index<'w, 's, T, F = (), I = SimpleIndexer<T>>
where
    T: Component,
    I: Indexer + 'static,
    F: ReadOnlyWorldQuery + 'static,
    I::Index: Send + Sync + 'static,
{
    changed: Query<'w, 's, (Entity, Ref<'static, T>), (Changed<T>, F)>,
    removed: RemovedComponents<'w, 's, T>,
    index: ResMut<'w, IndexBacking<T, F, I>>,
    change_tick: SystemChangeTick,
}

impl<'w, 's, T, F, I> Index<'w, 's, T, F, I>
where
    T: Component,
    I: Indexer<Input = T> + 'static,
    F: ReadOnlyWorldQuery + 'static,
    I::Index: Hash + Clone + Eq + Send + Sync + 'static,
{
    fn update_index_internal(&mut self) {
        let this_run = self.change_tick.this_run();

        // Remove old entires
        for entity in self.removed.read() {
            self.index.remove_by_entity(entity);
        }

        // Update new and existing entries
        for (entity, component) in self.changed.iter() {
            self.index.insert(entity, component.as_ref());
        }

        self.index.last_this_run = Some(this_run);
    }

    /// System to keep [`Index`] coarsely updated every frame
    fn update_index(mut index: Index<T, F, I>) {
        index.update_index_internal();
    }

    fn ensure_updated(&mut self) {
        let this_run = self.change_tick.this_run();

        if self.index.last_this_run != Some(this_run) {
            self.update_index_internal();
        }
    }

    /// Get all [entities](`Entity`) with a [`Component`] of `value`.
    pub fn get(&mut self, value: &T) -> impl Iterator<Item = Entity> + '_ {
        self.ensure_updated();

        self.index.get(value)
    }

    /// Get all [entities](`Entity`) with an `index`.
    pub fn get_by_index(&mut self, index: &I::Index) -> impl Iterator<Item = Entity> + '_ {
        self.ensure_updated();

        self.index.get_by_index(index)
    }

    /// Iterate over [entities](`Entity`) grouped by their [Index](`Indexer::Index`)
    pub fn iter(
        &mut self,
    ) -> impl Iterator<Item = (&I::Index, impl Iterator<Item = Entity> + '_)> + '_ {
        self.ensure_updated();

        self.index.iter()
    }
}

/// Starts indexing the [`Component`] `T`. This provides access to the [`Index`] system parameter.
pub struct IndexPlugin<T, F = (), I = SimpleIndexer<T>>(PhantomData<fn(T, F, I)>);

impl<T, F, I> Default for IndexPlugin<T, F, I> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

impl<T, I, F> Plugin for IndexPlugin<T, F, I>
where
    T: Component,
    I: Indexer<Input = T> + 'static,
    F: ReadOnlyWorldQuery + 'static,
    I::Index: Hash + Clone + Eq + Send + Sync + 'static,
{
    fn build(&self, app: &mut App) {
        app.init_resource::<IndexBacking<T, F, I>>()
            .add_systems(Update, Index::<T, F, I>::update_index);
    }
}

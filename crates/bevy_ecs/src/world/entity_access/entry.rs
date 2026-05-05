use crate::{
    component::{Component, Mutable},
    world::{EntityWorldMut, Mut},
};

use core::marker::PhantomData;

/// A view into a single entity and component in a world, which may either be vacant or occupied.
///
/// This `enum` can only be constructed from the [`entry`] method on [`EntityWorldMut`].
///
/// [`entry`]: EntityWorldMut::entry
pub enum ComponentEntry<'w, 'a, T: Component> {
    /// An occupied entry.
    Occupied(OccupiedComponentEntry<'w, 'a, T>),
    /// A vacant entry.
    Vacant(VacantComponentEntry<'w, 'a, T>),
}

impl<'w, 'a, T: Component<Mutability = Mutable>> ComponentEntry<'w, 'a, T> {
    /// Provides in-place mutable access to an occupied entry.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// #[derive(Component, Default, Clone, Copy, Debug, PartialEq)]
    /// struct Comp(u32);
    ///
    /// # let mut world = World::new();
    /// let mut entity = world.spawn(Comp(0));
    ///
    /// entity.entry::<Comp>().and_modify(|mut c| c.0 += 1);
    /// assert_eq!(world.query::<&Comp>().single(&world).unwrap().0, 1);
    /// ```
    #[inline]
    pub fn and_modify<F: FnOnce(Mut<'_, T>)>(self, f: F) -> Self {
        match self {
            ComponentEntry::Occupied(mut entry) => {
                f(entry.get_mut());
                ComponentEntry::Occupied(entry)
            }
            ComponentEntry::Vacant(entry) => ComponentEntry::Vacant(entry),
        }
    }
}

impl<'w, 'a, T: Component> ComponentEntry<'w, 'a, T> {
    /// Replaces the component of the entry, and returns an [`OccupiedComponentEntry`].
    ///
    /// # Examples
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// #[derive(Component, Default, Clone, Copy, Debug, PartialEq)]
    /// struct Comp(u32);
    ///
    /// # let mut world = World::new();
    /// let mut entity = world.spawn_empty();
    ///
    /// let entry = entity.entry().insert_entry(Comp(4));
    /// assert_eq!(entry.get(), &Comp(4));
    ///
    /// let entry = entity.entry().insert_entry(Comp(2));
    /// assert_eq!(entry.get(), &Comp(2));
    /// ```
    #[inline]
    pub fn insert_entry(self, component: T) -> OccupiedComponentEntry<'w, 'a, T> {
        match self {
            ComponentEntry::Occupied(mut entry) => {
                entry.insert(component);
                entry
            }
            ComponentEntry::Vacant(entry) => entry.insert(component),
        }
    }

    /// Ensures the entry has this component by inserting the given default if empty, and
    /// returns a mutable reference to this component in the entry.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// #[derive(Component, Default, Clone, Copy, Debug, PartialEq)]
    /// struct Comp(u32);
    ///
    /// # let mut world = World::new();
    /// let mut entity = world.spawn_empty();
    ///
    /// entity.entry().or_insert(Comp(4));
    /// # let entity_id = entity.id();
    /// assert_eq!(world.query::<&Comp>().single(&world).unwrap().0, 4);
    ///
    /// # let mut entity = world.get_entity_mut(entity_id).unwrap();
    /// entity.entry().or_insert(Comp(15)).into_mut().0 *= 2;
    /// assert_eq!(world.query::<&Comp>().single(&world).unwrap().0, 8);
    /// ```
    #[inline]
    pub fn or_insert(self, default: T) -> OccupiedComponentEntry<'w, 'a, T> {
        match self {
            ComponentEntry::Occupied(entry) => entry,
            ComponentEntry::Vacant(entry) => entry.insert(default),
        }
    }

    /// Ensures the entry has this component by inserting the result of the default function if
    /// empty, and returns a mutable reference to this component in the entry.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// #[derive(Component, Default, Clone, Copy, Debug, PartialEq)]
    /// struct Comp(u32);
    ///
    /// # let mut world = World::new();
    /// let mut entity = world.spawn_empty();
    ///
    /// entity.entry().or_insert_with(|| Comp(4));
    /// assert_eq!(world.query::<&Comp>().single(&world).unwrap().0, 4);
    /// ```
    #[inline]
    pub fn or_insert_with<F: FnOnce() -> T>(self, default: F) -> OccupiedComponentEntry<'w, 'a, T> {
        match self {
            ComponentEntry::Occupied(entry) => entry,
            ComponentEntry::Vacant(entry) => entry.insert(default()),
        }
    }
}

impl<'w, 'a, T: Component + Default> ComponentEntry<'w, 'a, T> {
    /// Ensures the entry has this component by inserting the default value if empty, and
    /// returns a mutable reference to this component in the entry.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// #[derive(Component, Default, Clone, Copy, Debug, PartialEq)]
    /// struct Comp(u32);
    ///
    /// # let mut world = World::new();
    /// let mut entity = world.spawn_empty();
    ///
    /// entity.entry::<Comp>().or_default();
    /// assert_eq!(world.query::<&Comp>().single(&world).unwrap().0, 0);
    /// ```
    #[inline]
    pub fn or_default(self) -> OccupiedComponentEntry<'w, 'a, T> {
        match self {
            ComponentEntry::Occupied(entry) => entry,
            ComponentEntry::Vacant(entry) => entry.insert(Default::default()),
        }
    }
}

/// A view into an occupied entry in a [`EntityWorldMut`]. It is part of the [`OccupiedComponentEntry`] enum.
///
/// The contained entity must have the component type parameter if we have this struct.
pub struct OccupiedComponentEntry<'w, 'a, T: Component> {
    pub(crate) entity_world: &'a mut EntityWorldMut<'w>,
    pub(crate) _marker: PhantomData<T>,
}

impl<'w, 'a, T: Component> OccupiedComponentEntry<'w, 'a, T> {
    /// Gets a reference to the component in the entry.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bevy_ecs::{prelude::*, world::ComponentEntry};
    /// #[derive(Component, Default, Clone, Copy, Debug, PartialEq)]
    /// struct Comp(u32);
    ///
    /// # let mut world = World::new();
    /// let mut entity = world.spawn(Comp(5));
    ///
    /// if let ComponentEntry::Occupied(o) = entity.entry::<Comp>() {
    ///     assert_eq!(o.get().0, 5);
    /// }
    /// ```
    #[inline]
    pub fn get(&self) -> &T {
        // This shouldn't panic because if we have an OccupiedComponentEntry the component must exist.
        self.entity_world.get::<T>().unwrap()
    }

    /// Replaces the component of the entry.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bevy_ecs::{prelude::*, world::ComponentEntry};
    /// #[derive(Component, Default, Clone, Copy, Debug, PartialEq)]
    /// struct Comp(u32);
    ///
    /// # let mut world = World::new();
    /// let mut entity = world.spawn(Comp(5));
    ///
    /// if let ComponentEntry::Occupied(mut o) = entity.entry::<Comp>() {
    ///     o.insert(Comp(10));
    /// }
    ///
    /// assert_eq!(world.query::<&Comp>().single(&world).unwrap().0, 10);
    /// ```
    #[inline]
    pub fn insert(&mut self, component: T) {
        self.entity_world.insert(component);
    }

    /// Removes the component from the entry and returns it.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bevy_ecs::{prelude::*, world::ComponentEntry};
    /// #[derive(Component, Default, Clone, Copy, Debug, PartialEq)]
    /// struct Comp(u32);
    ///
    /// # let mut world = World::new();
    /// let mut entity = world.spawn(Comp(5));
    ///
    /// if let ComponentEntry::Occupied(o) = entity.entry::<Comp>() {
    ///     assert_eq!(o.take(), Comp(5));
    /// }
    ///
    /// assert_eq!(world.query::<&Comp>().iter(&world).len(), 0);
    /// ```
    #[inline]
    pub fn take(self) -> T {
        // This shouldn't panic because if we have an OccupiedComponentEntry the component must exist.
        self.entity_world.take().unwrap()
    }
}

impl<'w, 'a, T: Component<Mutability = Mutable>> OccupiedComponentEntry<'w, 'a, T> {
    /// Gets a mutable reference to the component in the entry.
    ///
    /// If you need a reference to the [`OccupiedComponentEntry`] which may outlive the destruction of
    /// the [`OccupiedComponentEntry`] value, see [`into_mut`].
    ///
    /// [`into_mut`]: Self::into_mut
    ///
    /// # Examples
    ///
    /// ```
    /// # use bevy_ecs::{prelude::*, world::ComponentEntry};
    /// #[derive(Component, Default, Clone, Copy, Debug, PartialEq)]
    /// struct Comp(u32);
    ///
    /// # let mut world = World::new();
    /// let mut entity = world.spawn(Comp(5));
    ///
    /// if let ComponentEntry::Occupied(mut o) = entity.entry::<Comp>() {
    ///     o.get_mut().0 += 10;
    ///     assert_eq!(o.get().0, 15);
    ///
    ///     // We can use the same Entry multiple times.
    ///     o.get_mut().0 += 2
    /// }
    ///
    /// assert_eq!(world.query::<&Comp>().single(&world).unwrap().0, 17);
    /// ```
    #[inline]
    pub fn get_mut(&mut self) -> Mut<'_, T> {
        // This shouldn't panic because if we have an OccupiedComponentEntry the component must exist.
        self.entity_world.get_mut::<T>().unwrap()
    }

    /// Converts the [`OccupiedComponentEntry`] into a mutable reference to the value in the entry with
    /// a lifetime bound to the `EntityWorldMut`.
    ///
    /// If you need multiple references to the [`OccupiedComponentEntry`], see [`get_mut`].
    ///
    /// [`get_mut`]: Self::get_mut
    ///
    /// # Examples
    ///
    /// ```
    /// # use bevy_ecs::{prelude::*, world::ComponentEntry};
    /// #[derive(Component, Default, Clone, Copy, Debug, PartialEq)]
    /// struct Comp(u32);
    ///
    /// # let mut world = World::new();
    /// let mut entity = world.spawn(Comp(5));
    ///
    /// if let ComponentEntry::Occupied(o) = entity.entry::<Comp>() {
    ///     o.into_mut().0 += 10;
    /// }
    ///
    /// assert_eq!(world.query::<&Comp>().single(&world).unwrap().0, 15);
    /// ```
    #[inline]
    pub fn into_mut(self) -> Mut<'a, T> {
        // This shouldn't panic because if we have an OccupiedComponentEntry the component must exist.
        self.entity_world.get_mut().unwrap()
    }
}

/// A view into a vacant entry in a [`EntityWorldMut`]. It is part of the [`ComponentEntry`] enum.
pub struct VacantComponentEntry<'w, 'a, T: Component> {
    pub(crate) entity_world: &'a mut EntityWorldMut<'w>,
    pub(crate) _marker: PhantomData<T>,
}

impl<'w, 'a, T: Component> VacantComponentEntry<'w, 'a, T> {
    /// Inserts the component into the [`VacantComponentEntry`] and returns an [`OccupiedComponentEntry`].
    ///
    /// # Examples
    ///
    /// ```
    /// # use bevy_ecs::{prelude::*, world::ComponentEntry};
    /// #[derive(Component, Default, Clone, Copy, Debug, PartialEq)]
    /// struct Comp(u32);
    ///
    /// # let mut world = World::new();
    /// let mut entity = world.spawn_empty();
    ///
    /// if let ComponentEntry::Vacant(v) = entity.entry::<Comp>() {
    ///     v.insert(Comp(10));
    /// }
    ///
    /// assert_eq!(world.query::<&Comp>().single(&world).unwrap().0, 10);
    /// ```
    #[inline]
    pub fn insert(self, component: T) -> OccupiedComponentEntry<'w, 'a, T> {
        self.entity_world.insert(component);
        OccupiedComponentEntry {
            entity_world: self.entity_world,
            _marker: PhantomData,
        }
    }
}

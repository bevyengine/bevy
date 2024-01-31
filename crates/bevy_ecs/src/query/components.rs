use crate::{
    change_detection::{Mut, Ref},
    component::Component,
    query::{Added, Always, And, Changed, Has, Never, Or, QueryFilter, With, Without, WorldQuery},
    query::{BooleanQuery, QueryData, ReadOnlyQueryData},
    world::{EntityMut, EntityRef, EntityWorldMut},
};
use bevy_utils::all_tuples;
use std::any::TypeId;

/// A helper trait that can be used to generate
/// queries and filters based on multiple components.
///
/// This trait is implemented on [`Component`]s and
/// tuples of of [`ComponentGroup`]s.
///
/// # Example
///
/// ```
/// # use std::marker::PhantomData;
/// # use std::any::type_name;
/// # use bevy_ecs::component::Component;
/// # use bevy_ecs::prelude::Query;
/// # use bevy_ecs::query::{Is, ComponentGroup};
/// # #[derive(Component, Debug, Hash, Eq, PartialEq, Clone, Copy)]
/// # struct Transform(usize);
/// # #[derive(Component, Debug, Eq, PartialEq, Clone, Copy)]
/// # struct RigidBody(usize);
///
/// // Construct type aliases
/// type UsePhysics = <(Transform, RigidBody) as ComponentGroup>::Has;
/// type WithPhysics = <(Transform, RigidBody) as ComponentGroup>::With;
/// type WithoutPhysics = <(Transform, RigidBody) as ComponentGroup>::Without;
///
/// // As generics
/// pub fn count_system<T: ComponentGroup>(query: Query<(T::Has, Is<T::Changed>), WithPhysics>) {
///     let mut total = 0;
///     let mut num_changed = 0;
///     for (has, changed) in query.iter() {
///         if has {
///             total += 1;
///         }
///         if changed {
///             num_changed += 1;
///         }
///     }
///     println!("For physics object {}, {} out of {} changed.",
///         type_name::<T>(),
///         num_changed,
///         total
///     )
/// }
/// ```
///
/// # Safety
///
/// `is_disjoint` and `is_nested` must return the correct value.
pub unsafe trait ComponentGroup: 'static
where
    for<'t> Self::Has: WorldQuery<Item<'t> = bool>,
    for<'t> Self::HasAny: WorldQuery<Item<'t> = bool>,
{
    /// Generate a read only query based on this type.
    type ReadQuery: ReadOnlyQueryData;
    /// Generate a mutable query based on this type.
    type WriteQuery: QueryData;
    /// A reference type with lifetime based on this type.
    type Ref<'w>;
    /// A reference change detection type like [`Ref`] based on this type.
    type RefTicks<'w>;
    /// A mutable change detection type like [`Mut`] based on this type.
    type RefMut<'w>;
    /// Generate a [`With`] query based on this type.
    ///
    /// Returns true when all components are present.
    type With: QueryFilter;
    /// Generate a [`Without`] query based on this type.
    ///
    /// Returns true when any component is missing.
    type Without: QueryFilter + BooleanQuery;
    /// Generate a [`Added`] query based on this type.
    ///
    /// Returns true when some component is added,
    /// **only if** all components are present.
    type Added: QueryFilter + BooleanQuery;
    /// Generate a [`Changed`] query based on this type.
    ///
    /// Returns true when some component is changed,
    /// **only if** all components are present.
    type Changed: QueryFilter + BooleanQuery;
    /// Generate a [`Has`] query based on this type.
    ///
    /// Returns true when all components are present.
    type Has: ReadOnlyQueryData + BooleanQuery;
    /// Generate a [`Has`] query based on this type.
    ///
    /// Returns true when any component is present.
    type HasAny: ReadOnlyQueryData + BooleanQuery;

    /// Check if all items are unique,
    /// if so, allow fetching a mutable reference from an [`EntityMut`].
    fn is_disjoint() -> bool;

    /// There is no good way to inspect the insides of tuple to
    /// assert uniqueness at compile time,
    /// so we disallow nested tuples in mutable queries for now.
    #[doc(hidden)]
    fn is_nested() -> bool {
        true
    }

    /// Obtain mutable references to components in this tuple from an [`EntityRef`].
    fn from_entity_ref(entity: EntityRef) -> Option<Self::Ref<'_>>;

    /// Obtain mutable references to components in this tuple from an [`EntityRef`].
    fn from_entity_ref_ticks(entity: EntityRef) -> Option<Self::RefTicks<'_>>;

    /// Obtain mutable references to components using `get_unchecked`.
    ///
    /// # Safety
    ///
    /// `Self` must be a disjoint tuple.
    unsafe fn from_entity_mut_unchecked<'w>(entity: &EntityMut<'w>) -> Option<Self::RefMut<'w>>;

    /// Obtain mutable references to components in this tuple from an [`EntityMut`].
    /// This is only allowed on disjoint non-nested tuples.
    ///
    /// # Panics
    ///
    /// If T is nested or not disjoint.
    fn from_entity_mut(entity: EntityMut) -> Option<Self::RefMut<'_>> {
        if Self::is_disjoint() {
            // Safety: Safe since no duplicate access if tuple is disjoint.
            unsafe { Self::from_entity_mut_unchecked(&entity) }
        } else {
            panic!("The type system cannot verify the components are disjoint, only disjoint non-nested tuples are supported.")
        }
    }
}

// Safety: Safe since `()` is disjoint.
unsafe impl ComponentGroup for () {
    type ReadQuery = ();
    type WriteQuery = ();
    type Ref<'w> = ();
    type RefTicks<'w> = ();
    type RefMut<'w> = ();
    type With = Always<true>;
    type Without = Never;
    type Has = Always<true>;
    type HasAny = Always<true>;
    type Added = Always<false>;
    type Changed = Always<false>;

    fn is_nested() -> bool {
        false
    }

    fn is_disjoint() -> bool {
        true
    }

    fn from_entity_ref(_: EntityRef) -> Option<Self::Ref<'_>> {
        Some(())
    }

    fn from_entity_ref_ticks(_: EntityRef) -> Option<Self::RefTicks<'_>> {
        Some(())
    }

    unsafe fn from_entity_mut_unchecked<'w>(_: &EntityMut<'w>) -> Option<Self::RefMut<'w>> {
        Some(())
    }
}

// Safety: Safe since a single component is always disjoint.
unsafe impl<T> ComponentGroup for T
where
    T: Component,
{
    type ReadQuery = &'static T;
    type WriteQuery = &'static mut T;
    type Ref<'w> = &'w T;
    type RefTicks<'w> = Ref<'w, T>;
    type RefMut<'w> = Mut<'w, T>;
    type With = With<T>;
    type Without = Without<T>;
    type Has = Has<T>;
    type HasAny = Has<T>;
    type Added = Added<T>;
    type Changed = Changed<T>;

    fn is_disjoint() -> bool {
        true
    }

    fn is_nested() -> bool {
        false
    }

    fn from_entity_ref(entity: EntityRef) -> Option<Self::Ref<'_>> {
        entity.get::<Self>()
    }

    fn from_entity_ref_ticks(entity: EntityRef) -> Option<Self::RefTicks<'_>> {
        entity.get_ref::<Self>()
    }

    unsafe fn from_entity_mut_unchecked<'w>(entity: &EntityMut<'w>) -> Option<Self::RefMut<'w>> {
        entity.get_unchecked()
    }
}

macro_rules! comparisons {
    ($first: ident) => {true};
    ($first: ident $(,$rest: ident)*) => {
        $(TypeId::of::<$first>() != TypeId::of::<$rest>())&&* && comparisons!($($rest),*)
    }
}

macro_rules! impl_component_tuple {
    ($($t: ident),*) => {
        // Safety: Safe since disjointness is properly checked.
        unsafe impl<$($t: ComponentGroup),*> ComponentGroup for ($($t,)*) where
                $($t::Has: BooleanQuery,)*
                $($t::HasAny: BooleanQuery,)*
                $($t::Added: BooleanQuery,)*
                $($t::Changed: BooleanQuery,)* {
            type ReadQuery = ($($t::ReadQuery,)*);
            type WriteQuery = ($($t::WriteQuery,)*);
            type Ref<'w> = ($($t::Ref<'w>,)*);
            type RefTicks<'w> = ($($t::RefTicks<'w>,)*);
            type RefMut<'w> = ($($t::RefMut<'w>,)*);
            type With = ($($t::With,)*);
            type Without = Or<($($t::Without,)*)>;
            type Added = Or<($($t::Added,)*)>;
            type Changed = Or<($($t::Changed,)*)>;

            type Has = And<($($t::Has,)*)>;
            type HasAny = Or<($($t::Has,)*)>;

            fn is_disjoint() -> bool {
                $(!$t::is_nested() &&)*
                comparisons!($($t),*)
            }

            fn from_entity_ref(entity: EntityRef) -> Option<Self::Ref<'_>> {
                Some(($($t::from_entity_ref(entity)?,)*))
            }

            fn from_entity_ref_ticks(entity: EntityRef) -> Option<Self::RefTicks<'_>> {
                Some(($($t::from_entity_ref_ticks(entity)?,)*))
            }

            unsafe fn from_entity_mut_unchecked<'w>(entity: &EntityMut<'w>) -> Option<Self::RefMut<'w>> {
                Some(($($t::from_entity_mut_unchecked(entity)?,)*))
            }
        }
    };
}

all_tuples!(impl_component_tuple, 1, 15, T);

impl EntityRef<'_> {
    /// Gets access to multiple components on the current entity.
    /// Returns `None` if the entity does not have all of the components in `T`.
    #[inline]
    pub fn get_many<T: ComponentGroup>(&self) -> Option<T::Ref<'_>> {
        T::from_entity_ref(*self)
    }

    /// Gets access to multiple components on the current entity.
    /// including change detection information as a [`Ref`].
    ///
    /// Returns `None` if the entity does not have all of the components in `T`.
    #[inline]
    pub fn get_many_ref<T: ComponentGroup>(&self) -> Option<T::RefTicks<'_>> {
        T::from_entity_ref_ticks(*self)
    }
}

impl EntityMut<'_> {
    /// Gets access to multiple components on the current entity.
    /// Returns `None` if the entity does not have all of the components in `T`.
    #[inline]
    pub fn get_many<T: ComponentGroup>(&self) -> Option<T::Ref<'_>> {
        T::from_entity_ref(self.as_readonly())
    }

    /// Gets access to multiple components on the current entity.
    /// including change detection information as a [`Ref`].
    ///
    /// Returns `None` if the entity does not have all of the components in `T`.
    #[inline]
    pub fn get_many_ref<T: ComponentGroup>(&self) -> Option<T::RefTicks<'_>> {
        T::from_entity_ref_ticks(self.as_readonly())
    }

    /// Gets mutable access to the component of type `T` for the current entity.
    /// Returns `None` if the entity does not have all of the components in `T`.
    ///
    /// # Panics
    ///
    /// If T is nested or not disjoint.
    #[inline]
    pub fn get_many_mut<T: ComponentGroup>(&mut self) -> Option<T::RefMut<'_>> {
        T::from_entity_mut(self.reborrow())
    }
}

impl EntityWorldMut<'_> {
    /// Gets access to multiple components on the current entity.
    /// Returns `None` if the entity does not have all of the components in `T`.
    #[inline]
    pub fn get_many<T: ComponentGroup>(&self) -> Option<T::Ref<'_>> {
        T::from_entity_ref(EntityRef::from(self))
    }

    /// Gets access to multiple components on the current entity.
    /// including change detection information as a [`Ref`].
    ///
    /// Returns `None` if the entity does not have all of the components in `T`.
    #[inline]
    pub fn get_many_ref<T: ComponentGroup>(&self) -> Option<T::RefTicks<'_>> {
        T::from_entity_ref_ticks(EntityRef::from(self))
    }

    /// Gets mutable access to the component of type `T` for the current entity.
    /// Returns `None` if the entity does not have all of the components in `T`.
    ///
    /// # Panics
    ///
    /// If T is nested or not disjoint.
    #[inline]
    pub fn get_many_mut<T: ComponentGroup>(&mut self) -> Option<T::RefMut<'_>> {
        T::from_entity_mut(EntityMut::from(self))
    }
}

#[cfg(test)]
mod tests {
    use crate::entity::Entity;
    use crate::prelude::{With, Without, World};
    use crate::query::Is;
    use crate::system::{In, RunSystemOnce};
    use crate::{self as bevy_ecs};
    use crate::{
        component::Component,
        system::{assert_is_system, Query},
    };

    use super::ComponentGroup;

    #[derive(Component, Debug, Eq, PartialEq, Clone, Copy)]
    struct A(usize);
    #[derive(Component, Debug, Eq, PartialEq, Clone, Copy)]
    struct B(usize);
    #[derive(Component, Debug, Eq, PartialEq, Clone, Copy)]
    struct C(usize);
    #[derive(Component, Debug, Eq, PartialEq, Clone, Copy, PartialOrd, Ord)]
    struct D(usize);

    type Group = (A, B, C);

    fn system_read(
        In(array): In<Vec<(usize, usize, usize)>>,
        query: Query<<Group as ComponentGroup>::ReadQuery>,
    ) {
        let mut vec: Vec<_> = query.iter().map(|(a, b, c)| (a.0, b.0, c.0)).collect();
        vec.sort();
        assert_eq!(vec, array);
    }

    fn system_write(mut query: Query<<Group as ComponentGroup>::WriteQuery>) {
        for (mut a, mut b, mut c) in query.iter_mut() {
            a.0 = 100;
            b.0 = 100;
            c.0 = 100;
        }
    }

    fn system_with(In(array): In<Vec<usize>>, query: Query<&D, <Group as ComponentGroup>::With>) {
        let mut vec: Vec<_> = query.iter().map(|D(x)| *x).collect();
        vec.sort();

        assert_eq!(vec, array);
    }

    fn system_without(
        In(array): In<Vec<usize>>,
        query: Query<&D, <Group as ComponentGroup>::Without>,
    ) {
        let mut vec: Vec<_> = query.iter().map(|D(x)| *x).collect();
        vec.sort();

        assert_eq!(vec, array);
    }

    fn system_has(
        In((total, filtered)): In<(usize, usize)>,
        query: Query<<Group as ComponentGroup>::Has>,
    ) {
        assert!(query.iter().count() == total);
        assert!(query.iter().filter(|x| *x).count() == filtered);
    }

    fn system_has_any(
        In((total, filtered)): In<(usize, usize)>,
        query: Query<<Group as ComponentGroup>::HasAny>,
    ) {
        assert!(query.iter().count() == total);
        assert!(query.iter().filter(|x| *x).count() == filtered);
    }

    // Just validating here.
    fn system_added_changed(
        In((added, changed)): In<(usize, usize)>,
        q_added: Query<&D, <Group as ComponentGroup>::Added>,
        q_changed: Query<&D, <Group as ComponentGroup>::Changed>,
    ) {
        assert!(q_added.iter().count() == added);
        assert!(q_changed.iter().count() == changed);
    }

    #[test]
    fn run_systems() {
        let mut world = World::new();
        let e1 = world.spawn((A(1), B(1), C(1), D(1))).id();
        let e2 = world.spawn((A(1), B(2), C(3), D(2))).id();
        let e3 = world.spawn((A(3), B(3), C(3), D(3))).id();
        world.spawn(D(4));
        world.spawn((A(5), D(5)));
        world.spawn((B(6), D(6)));

        world.run_system_once_with(vec![(1, 1, 1), (1, 2, 3), (3, 3, 3)], system_read);
        world.run_system_once_with((6, 3), system_has);
        world.run_system_once_with((6, 5), system_has_any);
        world.run_system_once_with(vec![1, 2, 3], system_with);
        world.run_system_once_with(vec![4, 5, 6], system_without);
        assert_is_system(system_added_changed);

        let e1 = world.entity(e1);
        let (a, b, c) = e1.get_many::<(A, B, C)>().unwrap();

        assert!(a.0 == 1);
        assert!(b.0 == 1);
        assert!(c.0 == 1);

        let e2 = world.entity(e2);
        let (d, a, b) = e2.get_many_ref::<(D, A, B)>().unwrap();

        assert!(a.0 == 1);
        assert!(b.0 == 2);
        assert!(d.0 == 2);

        let mut e3 = world.entity_mut(e3);
        let (mut b, mut c, d) = e3.get_many_mut::<(B, C, D)>().unwrap();

        assert!(b.0 == 3);
        assert!(c.0 == 3);
        assert!(d.0 == 3);

        b.0 = 1;
        c.0 = 2;

        let e3 = e3.id();
        let e3 = world.entity(e3);
        let (a, b, c, d) = e3.get_many::<(A, B, C, D)>().unwrap();

        assert!(a.0 == 3);
        assert!(b.0 == 1);
        assert!(c.0 == 2);
        assert!(d.0 == 3);

        world.run_system_once(system_write);
    }

    #[test]
    #[should_panic]
    fn not_disjoint() {
        let mut world = World::new();
        let entity = world.spawn((A(1), B(1), C(1), D(1))).id();
        world.entity_mut(entity).get_many_mut::<(A, A, B, B)>();
    }

    #[test]
    #[should_panic]
    fn nested() {
        let mut world = World::new();
        let entity = world.spawn((A(1), B(1), C(1), D(1))).id();
        world.entity_mut(entity).get_many_mut::<(A, (B, C))>();
    }

    #[test]
    fn change_detection() {
        let mut world = World::new();
        let mut added = world.query_filtered::<Entity, <Group as ComponentGroup>::Added>();
        let mut changed = world.query_filtered::<Entity, <Group as ComponentGroup>::Changed>();
        let mut is_added = world.query::<Is<<Group as ComponentGroup>::Added>>();
        let mut is_changed = world.query::<Is<<Group as ComponentGroup>::Changed>>();

        assert_eq!(added.iter(&world).count(), 0);
        assert_eq!(changed.iter(&world).count(), 0);
        assert_eq!(is_added.iter(&world).count(), 0);
        assert_eq!(is_changed.iter(&world).count(), 0);
        assert_eq!(is_added.iter(&world).filter(|x| *x).count(), 0);
        assert_eq!(is_changed.iter(&world).filter(|x| *x).count(), 0);
        world.clear_trackers();

        world.spawn((A(1), B(1), C(1), D(1)));
        world.spawn((A(1), B(2), C(3), D(2)));

        assert_eq!(added.iter(&world).count(), 2);
        assert_eq!(changed.iter(&world).count(), 2);
        assert_eq!(is_added.iter(&world).count(), 2);
        assert_eq!(is_changed.iter(&world).count(), 2);
        assert_eq!(is_added.iter(&world).filter(|x| *x).count(), 2);
        assert_eq!(is_changed.iter(&world).filter(|x| *x).count(), 2);
        world.clear_trackers();

        world.spawn((A(1), B(1), C(1)));

        assert_eq!(added.iter(&world).count(), 1);
        assert_eq!(changed.iter(&world).count(), 1);
        assert_eq!(is_added.iter(&world).count(), 3);
        assert_eq!(is_changed.iter(&world).count(), 3);
        assert_eq!(is_added.iter(&world).filter(|x| *x).count(), 1);
        assert_eq!(is_changed.iter(&world).filter(|x| *x).count(), 1);
        world.clear_trackers();

        let mut change = world.query_filtered::<<Group as ComponentGroup>::WriteQuery, With<D>>();
        change
            .iter_mut(&mut world)
            .for_each(|(mut a, _, _)| a.0 += 1);

        assert_eq!(added.iter(&world).count(), 0);
        assert_eq!(changed.iter(&world).count(), 2);
        assert_eq!(is_added.iter(&world).count(), 3);
        assert_eq!(is_changed.iter(&world).count(), 3);
        assert_eq!(is_added.iter(&world).filter(|x| *x).count(), 0);
        assert_eq!(is_changed.iter(&world).filter(|x| *x).count(), 2);
        world.clear_trackers();

        change.iter_mut(&mut world).for_each(|(_, mut b, mut c)| {
            b.0 += 1;
            c.0 += 1;
        });

        assert_eq!(added.iter(&world).count(), 0);
        assert_eq!(changed.iter(&world).count(), 2);
        assert_eq!(is_added.iter(&world).count(), 3);
        assert_eq!(is_changed.iter(&world).count(), 3);
        assert_eq!(is_added.iter(&world).filter(|x| *x).count(), 0);
        assert_eq!(is_changed.iter(&world).filter(|x| *x).count(), 2);
        world.clear_trackers();

        let mut change2 =
            world.query_filtered::<<Group as ComponentGroup>::WriteQuery, Without<D>>();

        change2.iter_mut(&mut world).for_each(|(_, mut b, mut c)| {
            b.0 += 1;
            c.0 += 1;
        });

        assert_eq!(added.iter(&world).count(), 0);
        assert_eq!(changed.iter(&world).count(), 1);
        assert_eq!(is_added.iter(&world).count(), 3);
        assert_eq!(is_changed.iter(&world).count(), 3);
        assert_eq!(is_added.iter(&world).filter(|x| *x).count(), 0);
        assert_eq!(is_changed.iter(&world).filter(|x| *x).count(), 1);
    }
}

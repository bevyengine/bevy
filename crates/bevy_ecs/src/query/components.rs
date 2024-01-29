use crate::{
    component::Component,
    query::{BooleanQuery, QueryData, ReadOnlyQueryData},
};
use bevy_utils::all_tuples;

use super::{Added, Always, And, Changed, Has, Never, Or, QueryFilter, With, Without, WorldQuery};

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
///
/// ```
pub trait ComponentGroup
where
    for<'t> Self::Has: WorldQuery<Item<'t> = bool>,
    for<'t> Self::HasAny: WorldQuery<Item<'t> = bool>,
{
    /// Generate a read only query based on this type.
    type ReadQuery: ReadOnlyQueryData;
    /// Generate a mutable query based on this type.
    type WriteQuery: QueryData;
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
}

impl ComponentGroup for () {
    type ReadQuery = ();
    type WriteQuery = ();
    type With = Always<true>;
    type Without = Never;
    type Has = Always<true>;
    type HasAny = Always<true>;
    type Added = Always<false>;
    type Changed = Always<false>;
}

impl<T> ComponentGroup for T
where
    T: Component,
{
    type ReadQuery = &'static T;
    type WriteQuery = &'static mut T;
    type With = With<T>;
    type Without = Without<T>;
    type Has = Has<T>;
    type HasAny = Has<T>;
    type Added = Added<T>;
    type Changed = Changed<T>;
}

macro_rules! impl_component_tuple {
    ($($t: ident),*) => {
        impl<$($t: ComponentGroup),*> ComponentGroup for ($($t,)*) where
                $($t::Has: BooleanQuery,)*
                $($t::HasAny: BooleanQuery,)*
                $($t::Added: BooleanQuery,)*
                $($t::Changed: BooleanQuery,)* {
            type ReadQuery = ($($t::ReadQuery,)*);
            type WriteQuery = ($($t::WriteQuery,)*);
            type With = ($($t::With,)*);
            type Without = Or<($($t::Without,)*)>;
            type Added = Or<($($t::Added,)*)>;
            type Changed = Or<($($t::Changed,)*)>;

            type Has = And<($($t::Has,)*)>;
            type HasAny = Or<($($t::Has,)*)>;
        }
    };
}

all_tuples!(impl_component_tuple, 1, 15, T);

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
        world.spawn((A(1), B(1), C(1), D(1)));
        world.spawn((A(1), B(2), C(3), D(2)));
        world.spawn((A(3), B(3), C(3), D(3)));
        world.spawn(D(4));
        world.spawn((A(5), D(5)));
        world.spawn((B(6), D(6)));

        world.run_system_once_with(vec![(1, 1, 1), (1, 2, 3), (3, 3, 3)], system_read);
        world.run_system_once(system_write);
        world.run_system_once_with((6, 3), system_has);
        world.run_system_once_with((6, 5), system_has_any);
        world.run_system_once_with(vec![1, 2, 3], system_with);
        world.run_system_once_with(vec![4, 5, 6], system_without);
        assert_is_system(system_added_changed);
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

use crate::{
    component::Component,
    query::{BooleanQuery, QueryData, ReadOnlyQueryData},
};
use bevy_utils::all_tuples;

use super::{Added, Always, And, Changed, Has, Never, Or, QueryFilter, With, Without};

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
/// # use bevy_ecs::query::ComponentGroup;
/// # use bevy_ecs::prelude::Query;
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
/// pub fn count_system<T: ComponentGroup>(query: Query<(T::Has, T::Changed), WithPhysics>) {
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
pub trait ComponentGroup {
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
    /// Returns true when some component is added
    /// and all components are present.
    ///
    /// [`IsAdded`](ComponentGroup::IsAdded) should be used outside of a filter.
    type Added: QueryFilter + BooleanQuery;
    /// Generate a [`Changed`] query based on this type.
    ///
    /// Returns true when some component is changed
    /// and all components are present.
    ///
    /// [`IsChanged`](ComponentGroup::IsChanged) should be used outside of a filter.
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

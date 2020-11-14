use crate::{archetype::Archetype, Component, QueryAccess};
use core::{any::TypeId, marker::PhantomData, ptr::NonNull};
use std::{boxed::Box, vec};

pub trait QueryFilter: Sized {
    type EntityFilter: EntityFilter;
    fn access() -> QueryAccess;
    fn get_entity_filter(archetype: &Archetype) -> Option<Self::EntityFilter>;
}

pub trait EntityFilter: Sized {
    const DANGLING: Self;

    /// # Safety
    /// This might access archetype data in an unsafe manner. In general filters should be read-only and they should only access
    /// the data they have claimed in `access()`.
    unsafe fn matches_entity(&self, _offset: usize) -> bool;
}

pub struct AnyEntityFilter;

impl EntityFilter for AnyEntityFilter {
    const DANGLING: Self = AnyEntityFilter;

    #[inline]
    unsafe fn matches_entity(&self, _offset: usize) -> bool {
        true
    }
}

pub struct Or<T>(pub T);

/// Query transformer that retrieves components of type `T` that have been mutated since the start of the frame.
/// Added components do not count as mutated.
pub struct Mutated<T>(NonNull<bool>, PhantomData<T>);

/// Query transformer that retrieves components of type `T` that have been added since the start of the frame.
pub struct Added<T>(NonNull<bool>, PhantomData<T>);

/// Query transformer that retrieves components of type `T` that have either been mutated or added since the start of the frame.
pub struct Changed<T>(NonNull<bool>, NonNull<bool>, PhantomData<T>);

impl QueryFilter for () {
    type EntityFilter = AnyEntityFilter;

    fn access() -> QueryAccess {
        QueryAccess::None
    }

    #[inline]
    fn get_entity_filter(_archetype: &Archetype) -> Option<Self::EntityFilter> {
        Some(AnyEntityFilter)
    }
}

impl<T: Component> QueryFilter for Added<T> {
    type EntityFilter = Self;

    fn access() -> QueryAccess {
        QueryAccess::read::<T>()
    }

    #[inline]
    fn get_entity_filter(archetype: &Archetype) -> Option<Self::EntityFilter> {
        archetype
            .get_type_state(TypeId::of::<T>())
            .map(|state| Added(state.added(), Default::default()))
    }
}

impl<T: Component> EntityFilter for Added<T> {
    const DANGLING: Self = Added(NonNull::dangling(), PhantomData::<T>);

    #[inline]
    unsafe fn matches_entity(&self, offset: usize) -> bool {
        *self.0.as_ptr().add(offset)
    }
}

impl<T: Component> QueryFilter for Mutated<T> {
    type EntityFilter = Self;

    fn access() -> QueryAccess {
        QueryAccess::read::<T>()
    }

    #[inline]
    fn get_entity_filter(archetype: &Archetype) -> Option<Self::EntityFilter> {
        archetype
            .get_type_state(TypeId::of::<T>())
            .map(|state| Mutated(state.mutated(), Default::default()))
    }
}

impl<T: Component> EntityFilter for Mutated<T> {
    const DANGLING: Self = Mutated(NonNull::dangling(), PhantomData::<T>);

    unsafe fn matches_entity(&self, offset: usize) -> bool {
        *self.0.as_ptr().add(offset)
    }
}

impl<T: Component> QueryFilter for Changed<T> {
    type EntityFilter = Self;

    fn access() -> QueryAccess {
        QueryAccess::read::<T>()
    }

    #[inline]
    fn get_entity_filter(archetype: &Archetype) -> Option<Self::EntityFilter> {
        archetype
            .get_type_state(TypeId::of::<T>())
            .map(|state| Changed(state.added(), state.mutated(), Default::default()))
    }
}

impl<T: Component> EntityFilter for Changed<T> {
    const DANGLING: Self = Changed(NonNull::dangling(), NonNull::dangling(), PhantomData::<T>);

    #[inline]
    unsafe fn matches_entity(&self, offset: usize) -> bool {
        *self.0.as_ptr().add(offset) || *self.1.as_ptr().add(offset)
    }
}

pub struct Without<T>(PhantomData<T>);

impl<T: Component> QueryFilter for Without<T> {
    type EntityFilter = AnyEntityFilter;

    fn access() -> QueryAccess {
        QueryAccess::without::<T>(QueryAccess::None)
    }

    #[inline]
    fn get_entity_filter(archetype: &Archetype) -> Option<Self::EntityFilter> {
        if archetype.has_type(TypeId::of::<T>()) {
            None
        } else {
            Some(AnyEntityFilter)
        }
    }
}

pub struct With<T>(PhantomData<T>);

impl<T: Component> QueryFilter for With<T> {
    type EntityFilter = AnyEntityFilter;

    fn access() -> QueryAccess {
        QueryAccess::with::<T>(QueryAccess::None)
    }

    #[inline]
    fn get_entity_filter(archetype: &Archetype) -> Option<Self::EntityFilter> {
        if archetype.has_type(TypeId::of::<T>()) {
            Some(AnyEntityFilter)
        } else {
            None
        }
    }
}

macro_rules! impl_query_filter_tuple {
    ($($filter: ident),*) => {
        #[allow(unused_variables)]
        #[allow(non_snake_case)]
        impl<$($filter: QueryFilter),*> QueryFilter for ($($filter,)*) {
            type EntityFilter = ($($filter::EntityFilter,)*);

            fn access() -> QueryAccess {
                QueryAccess::union(vec![
                    $($filter::access(),)+
                ])
            }

            fn get_entity_filter(archetype: &Archetype) -> Option<Self::EntityFilter> {
                Some(($($filter::get_entity_filter(archetype)?,)*))
            }

        }

        #[allow(unused_variables)]
        #[allow(non_snake_case)]
        impl<$($filter: EntityFilter),*> EntityFilter for ($($filter,)*) {
            const DANGLING: Self = ($($filter::DANGLING,)*);
            unsafe fn matches_entity(&self, offset: usize) -> bool {
                let ($($filter,)*) = self;
                true $(&& $filter.matches_entity(offset))*
            }
        }

        #[allow(unused_variables)]
        #[allow(non_snake_case)]
        impl<$($filter: QueryFilter),*> QueryFilter for Or<($($filter,)*)> {
            type EntityFilter = Or<($(Option<<$filter as QueryFilter>::EntityFilter>,)*)>;
            fn access() -> QueryAccess {
                QueryAccess::union(vec![
                    $(QueryAccess::Optional(Box::new($filter::access())),)+
                ])
            }

            fn get_entity_filter(archetype: &Archetype) -> Option<Self::EntityFilter> {
                let mut matches_something = false;
                $(
                    let $filter = $filter::get_entity_filter(archetype);
                    matches_something = matches_something || $filter.is_some();
                )*
                if matches_something {
                    Some(Or(($($filter,)*)))
                } else {
                    None
                }
            }

        }
        #[allow(unused_variables)]
        #[allow(non_snake_case)]
        impl<$($filter: EntityFilter),*> EntityFilter for Or<($(Option<$filter>,)*)> {
            const DANGLING: Self = Or(($(Some($filter::DANGLING),)*));
            unsafe fn matches_entity(&self, offset: usize) -> bool {
                let Or(($($filter,)*)) = self;
                false $(|| $filter.as_ref().map_or(false, |filter|filter.matches_entity(offset)))*
            }
        }
    };
}

impl_query_filter_tuple!(A);
impl_query_filter_tuple!(A, B);
impl_query_filter_tuple!(A, B, C);
impl_query_filter_tuple!(A, B, C, D);
impl_query_filter_tuple!(A, B, C, D, E);
impl_query_filter_tuple!(A, B, C, D, E, F);
impl_query_filter_tuple!(A, B, C, D, E, F, G);
impl_query_filter_tuple!(A, B, C, D, E, F, G, H);
impl_query_filter_tuple!(A, B, C, D, E, F, G, H, I);
impl_query_filter_tuple!(A, B, C, D, E, F, G, H, I, J);
impl_query_filter_tuple!(A, B, C, D, E, F, G, H, I, J, K);
impl_query_filter_tuple!(A, B, C, D, E, F, G, H, I, J, K, L);
impl_query_filter_tuple!(A, B, C, D, E, F, G, H, I, J, K, L, M);
impl_query_filter_tuple!(A, B, C, D, E, F, G, H, I, J, K, L, M, N);
impl_query_filter_tuple!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O);
impl_query_filter_tuple!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P);

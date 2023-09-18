use bevy_ptr::{Ptr, PtrMut, UnsafeCellDeref};
use bevy_utils::all_tuples;

use crate::{
    change_detection::{Mut, MutUntyped, Ticks, TicksMut},
    entity::Entity,
    prelude::{
        Added, AnyOf, Changed, Component, EntityMut, EntityRef, Has, Or, Ref, With, Without, World,
    },
    query::DebugCheckedUnwrap,
};

use super::{FetchedTerm, Term, TermAccess, TermOperator};

/// Types that can be fetched from a [`World`] using a [`TermQuery`](crate::prelude::TermQuery).
///
/// This is implemented for all the same types as [`WorldQuery`](crate::query::WorldQuery) as well
/// as additional types that for dynamic queries.
///
/// Theses additional types are [`Ptr`] and [`PtrMut`] which are equivalent to
/// &T and &mut T respectively but their component id is set at runtime.
///
/// ```
/// use bevy_ecs::prelude::*;
/// use bevy_ptr::Ptr;
///
/// #[derive(Component)]
/// struct MyComponent;
///
/// let mut world = World::new();
/// world.spawn(MyComponent);
///
/// let component_id = world.init_component::<MyComponent>();
///
/// let mut query = unsafe {
///     QueryBuilder::<(Entity, Ptr)>::new(&mut world)
///         .term_at(1)
///         .set_dynamic_by_id(component_id)
///         .build()
/// };
///
/// let (entity, component): (Entity, Ptr) = query.single(&world);
/// let component_ref: &MyComponent = unsafe { component.deref::<MyComponent>() };
/// ```
///
/// # Safety
///
/// Component access of `Self::ReadOnly` must be a subset of `Self`
/// and `Self::ReadOnly` must match exactly the same archetypes/tables as `Self`
///
/// Implementor must ensure that [`Self::from_fetch`] is safe to call on a [`FetchedTerm`]
/// resolved from the value returned by [`Self::init_term`]
pub trait QueryTerm {
    /// The item returned by this [`QueryTerm`]
    type Item<'w>;
    /// The read-only variant of this [`QueryTerm`]
    type ReadOnly: QueryTerm;

    /// Creates a new [`Term`] instance satisfying the requirements for [`Self::from_fetch`]
    fn init_term(world: &mut World) -> Term;

    /// Creates an instance of [`Self::Item`] out of a fetched [`Term`]
    ///
    /// # Safety
    ///
    /// Caller must ensure that `fetch` is consumable as the implementing type
    unsafe fn from_fetch<'w>(fetch: &FetchedTerm<'w>) -> Self::Item<'w>;
}

/// A trait representing a group of types implementing [`QueryTerm`].
///
/// This is most commonly tuples of terms or operators like [`Or`] and [`AnyOf`].
pub trait QueryTermGroup {
    /// The item returned by this [`QueryTermGroup`]
    type Item<'w>;
    /// The read-only variant of this [`QueryTermGroup`]
    type ReadOnly: QueryTermGroup;
    /// The optional variant of this [`QueryTermGroup`]
    type Optional: QueryTermGroup;

    /// Writes new [`Term`] instances to `terms`, satisfying the requirements for [`Self::from_fetches`]
    fn init_terms(world: &mut World, terms: &mut Vec<Term>);

    /// Creates an instance of [`Self::Item`] out of an iterator of fetched [`Term`]s
    ///
    /// # Safety
    ///
    /// Caller must ensure the `terms` is consumable as the implementing type
    unsafe fn from_fetches<'w: 'f, 'f>(
        terms: &mut impl Iterator<Item = &'f FetchedTerm<'w>>,
    ) -> Self::Item<'w>;
}

// Blanket implementation [`QueryTermGroup`] for [`QueryTerm`]
// Pushes a single term to the list of terms and resolves a single term from the iterator
impl<T: QueryTerm> QueryTermGroup for T {
    type Item<'w> = T::Item<'w>;
    type ReadOnly = T::ReadOnly;
    type Optional = Option<T>;

    fn init_terms(world: &mut World, terms: &mut Vec<Term>) {
        terms.push(T::init_term(world));
    }

    #[inline]
    unsafe fn from_fetches<'w: 'f, 'f>(
        terms: &mut impl Iterator<Item = &'f FetchedTerm<'w>>,
    ) -> Self::Item<'w> {
        T::from_fetch(terms.next().debug_checked_unwrap())
    }
}

// Blanket implementatinon of [`QueryTermGroup`] for all tuples of [`QueryTermGroup`]
macro_rules! impl_query_term_tuple {
    ($($term: ident),*) => {
        impl<$($term: QueryTermGroup),*> QueryTermGroup for ($($term,)*) {
            type Item<'w> = ($($term::Item<'w>,)*);
            type ReadOnly = ($($term::ReadOnly,)*);
            type Optional = ($($term::Optional,)*);

            fn init_terms(_world: &mut World, _terms: &mut Vec<Term>) {
                $(
                    $term::init_terms(_world, _terms);
                )*
            }

            #[allow(clippy::unused_unit)]
            #[inline]
            unsafe fn from_fetches<'w: 'f, 'f>(_terms: &mut impl Iterator<Item = &'f FetchedTerm<'w>>) -> Self::Item<'w> {
                ($(
                    $term::from_fetches(_terms),
                )*)
            }
        }
    };
}

all_tuples!(impl_query_term_tuple, 0, 15, T);

impl QueryTerm for Entity {
    type Item<'w> = Self;
    type ReadOnly = Self;

    fn init_term(_world: &mut World) -> Term {
        Term::entity()
    }

    #[inline]
    unsafe fn from_fetch<'w>(term: &'w FetchedTerm<'w>) -> Self::Item<'w> {
        term.entity
    }
}

impl QueryTerm for EntityRef<'_> {
    type Item<'w> = EntityRef<'w>;
    type ReadOnly = Self;

    fn init_term(_world: &mut World) -> Term {
        Term::entity().set_access(TermAccess::Read)
    }

    #[inline]
    unsafe fn from_fetch<'w>(term: &FetchedTerm<'w>) -> Self::Item<'w> {
        EntityRef::new(term.entity_cell().debug_checked_unwrap())
    }
}

impl<'r> QueryTerm for EntityMut<'r> {
    type Item<'w> = EntityMut<'w>;
    type ReadOnly = EntityRef<'r>;

    fn init_term(_world: &mut World) -> Term {
        Term::entity().set_access(TermAccess::Write)
    }

    #[inline]
    unsafe fn from_fetch<'w>(term: &FetchedTerm<'w>) -> Self::Item<'w> {
        EntityMut::new(term.entity_cell().debug_checked_unwrap())
    }
}

impl<T: Component> QueryTerm for With<T> {
    type Item<'w> = ();
    type ReadOnly = Self;

    fn init_term(world: &mut World) -> Term {
        let component = world.init_component::<T>();
        Term::with_id(component)
    }

    #[inline(always)]
    unsafe fn from_fetch<'w>(_term: &'w FetchedTerm<'w>) -> Self::Item<'w> {}
}

impl<T: Component> QueryTerm for Without<T> {
    type Item<'w> = ();
    type ReadOnly = Self;

    fn init_term(world: &mut World) -> Term {
        let component = world.init_component::<T>();
        Term::without_id(component)
    }

    #[inline]
    unsafe fn from_fetch<'w>(_term: &'w FetchedTerm<'w>) -> Self::Item<'w> {}
}

impl<T: Component> QueryTerm for Has<T> {
    type Item<'w> = bool;
    type ReadOnly = Self;

    fn init_term(world: &mut World) -> Term {
        let component = world.init_component::<T>();
        Term::with_id(component).set_operator(TermOperator::Optional)
    }

    #[inline]
    unsafe fn from_fetch<'w>(term: &'w FetchedTerm<'w>) -> Self::Item<'w> {
        term.matched
    }
}

impl<T: Component> QueryTerm for Added<T> {
    type Item<'w> = ();
    type ReadOnly = Self;

    fn init_term(world: &mut World) -> Term {
        let component = world.init_component::<T>();
        Term::added_id(component)
    }

    #[inline]
    unsafe fn from_fetch<'w>(_term: &'w FetchedTerm<'w>) -> Self::Item<'w> {}
}

impl<T: Component> QueryTerm for Changed<T> {
    type Item<'w> = ();
    type ReadOnly = Self;

    fn init_term(world: &mut World) -> Term {
        let component = world.init_component::<T>();
        Term::changed_id(component)
    }

    #[inline]
    unsafe fn from_fetch<'w>(_term: &'w FetchedTerm<'w>) -> Self::Item<'w> {}
}

impl<T: Component> QueryTerm for &T {
    type Item<'w> = &'w T;
    type ReadOnly = Self;

    fn init_term(world: &mut World) -> Term {
        let component = world.init_component::<T>();
        Term::read_id(component)
    }

    #[inline]
    unsafe fn from_fetch<'w>(term: &FetchedTerm<'w>) -> Self::Item<'w> {
        term.component_ptr().debug_checked_unwrap().deref()
    }
}

impl<T: Component> QueryTerm for Ref<'_, T> {
    type Item<'w> = Ref<'w, T>;
    type ReadOnly = Self;

    fn init_term(world: &mut World) -> Term {
        let component = world.init_component::<T>();
        Term::read_id(component).with_change_detection()
    }

    #[inline]
    unsafe fn from_fetch<'w>(term: &FetchedTerm<'w>) -> Self::Item<'w> {
        let change_detection = term.change_ticks().debug_checked_unwrap();
        Ref {
            value: term.component_ptr().debug_checked_unwrap().deref(),
            ticks: Ticks {
                added: change_detection.added.deref(),
                changed: change_detection.changed.deref(),

                last_run: change_detection.last_run,
                this_run: change_detection.this_run,
            },
        }
    }
}

impl QueryTerm for Ptr<'_> {
    type Item<'w> = Ptr<'w>;
    type ReadOnly = Self;

    fn init_term(_world: &mut World) -> Term {
        Term::read()
    }

    #[inline]
    unsafe fn from_fetch<'w>(term: &FetchedTerm<'w>) -> Self::Item<'w> {
        term.component_ptr().debug_checked_unwrap()
    }
}

impl<'r, T: Component> QueryTerm for &'r mut T {
    type Item<'w> = Mut<'w, T>;
    type ReadOnly = &'r T;

    fn init_term(world: &mut World) -> Term {
        let component = world.init_component::<T>();
        Term::write_id(component).with_change_detection()
    }

    #[inline]
    unsafe fn from_fetch<'w>(term: &FetchedTerm<'w>) -> Self::Item<'w> {
        let change_detection = term.change_ticks().debug_checked_unwrap();
        Mut {
            value: term
                .component_ptr()
                .debug_checked_unwrap()
                .assert_unique()
                .deref_mut(),
            ticks: TicksMut {
                added: change_detection.added.deref_mut(),
                changed: change_detection.changed.deref_mut(),

                last_run: change_detection.last_run,
                this_run: change_detection.this_run,
            },
        }
    }
}

impl<'r> QueryTerm for PtrMut<'r> {
    type Item<'w> = MutUntyped<'w>;
    type ReadOnly = Ptr<'r>;

    fn init_term(_world: &mut World) -> Term {
        Term::write().with_change_detection()
    }

    #[inline]
    unsafe fn from_fetch<'w>(term: &FetchedTerm<'w>) -> Self::Item<'w> {
        let change_detection = term.change_ticks().debug_checked_unwrap();
        MutUntyped {
            value: term.component_ptr().debug_checked_unwrap().assert_unique(),
            ticks: TicksMut {
                added: change_detection.added.deref_mut(),
                changed: change_detection.changed.deref_mut(),

                last_run: change_detection.last_run,
                this_run: change_detection.this_run,
            },
        }
    }
}

impl<C: QueryTerm> QueryTerm for Option<C> {
    type Item<'w> = Option<C::Item<'w>>;
    type ReadOnly = Option<C::ReadOnly>;

    fn init_term(world: &mut World) -> Term {
        C::init_term(world).set_operator(TermOperator::Optional)
    }

    #[inline]
    unsafe fn from_fetch<'w>(term: &FetchedTerm<'w>) -> Self::Item<'w> {
        if term.matched {
            Some(C::from_fetch(term))
        } else {
            None
        }
    }
}

impl<Q: QueryTermGroup> QueryTermGroup for Or<Q> {
    type Item<'w> = ();
    type ReadOnly = Self;
    type Optional = ();

    fn init_terms(world: &mut World, terms: &mut Vec<Term>) {
        let mut sub_terms = Vec::new();
        Q::init_terms(world, &mut sub_terms);
        terms.push(Term::or_terms(sub_terms));
    }

    #[inline]
    unsafe fn from_fetches<'w: 'f, 'f>(
        terms: &mut impl Iterator<Item = &'f FetchedTerm<'w>>,
    ) -> Self::Item<'w> {
        terms.next();
    }
}

impl<Q: QueryTermGroup> QueryTermGroup for AnyOf<Q> {
    type Item<'w> = <Q::Optional as QueryTermGroup>::Item<'w>;
    type ReadOnly = Self;
    type Optional = ();

    fn init_terms(world: &mut World, terms: &mut Vec<Term>) {
        let mut sub_terms = Vec::new();
        Q::Optional::init_terms(world, &mut sub_terms);
        terms.push(Term::any_of_terms(sub_terms));
    }

    #[inline]
    unsafe fn from_fetches<'w: 'f, 'f>(
        terms: &mut impl Iterator<Item = &'f FetchedTerm<'w>>,
    ) -> Self::Item<'w> {
        let term = terms.next().debug_checked_unwrap();
        Q::Optional::from_fetches(&mut term.sub_terms().debug_checked_unwrap().iter())
    }
}

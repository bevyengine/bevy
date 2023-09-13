use bevy_ptr::{Ptr, PtrMut, UnsafeCellDeref};

use crate::{
    change_detection::{Mut, MutUntyped, Ticks, TicksMut},
    prelude::{Added, Changed, Component, Has, Ref, With, Without, World},
    query::DebugCheckedUnwrap,
};

use super::{FetchedTerm, QueryTerm, Term};

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
        Term::with_id(component).set_optional()
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
        C::init_term(world).set_optional()
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
